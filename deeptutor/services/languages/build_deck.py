"""Build a starter deck from openly licensed sources (contract C10).

    python -m deeptutor.services.languages.build_deck --lang es --size 50 [--offline DIR]

Sources (downloaded into ``data/languages/sources/`` unless ``--offline`` names a
folder that already holds them, plain or ``.bz2``):

- Tatoeba per-language exports: target-language sentences, English sentences
  and the links between them. Licence CC BY 2.0 FR (tatoeba.org).
- FrequencyWords by Hermit Dave (word counts from OpenSubtitles), used as
  metadata only to rank words. Licence CC BY-SA 4.0 (share-alike; acceptable
  for this private corpus, see docs/tutor/PLAN.md).

The deck takes the ``size`` most frequent words that occur in a short sentence
(3-10 words) with an English translation. Each word gets a production card (English plus the
sentence with the word blanked; type the word) and a recognition card (the
sentence; give its meaning, self-graded). Every row records its source and
licence. The CSV lands in ``data/languages/<lang>/`` and is imported through the
practice import pipeline, so re-running is idempotent.
"""

from __future__ import annotations

import argparse
import bz2
from collections.abc import Iterator
import csv
from pathlib import Path
import shutil
import sys
import urllib.request

from deeptutor.runtime.home import get_runtime_data_root
from deeptutor.services.practice.importing import MAX_ACCEPTED, MAX_ROWS, preview
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session import get_sqlite_session_store

from .text import LANGUAGES, normalize, tokenize

TATOEBA_CODES = {"es": "spa", "fr": "fra", "de": "deu", "ja": "jpn", "zh": "cmn"}
TATOEBA_URL = "https://downloads.tatoeba.org/exports/per_language/{code}/{name}"
# FrequencyWords has no 2018 Japanese list; 2016 is the latest.
FREQUENCY_PATHS = {
    "es": "2018/es/es_50k.txt",
    "fr": "2018/fr/fr_50k.txt",
    "de": "2018/de/de_50k.txt",
    "ja": "2016/ja/ja_50k.txt",
    "zh": "2018/zh_cn/zh_cn_50k.txt",
}
FREQUENCY_URL = "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/{path}"
LICENCE = (
    "Sentences: CC BY 2.0 FR, Tatoeba contributors (tatoeba.org). "
    "Word ranking: CC BY-SA 4.0, FrequencyWords by Hermit Dave (OpenSubtitles counts)."
)
# A short example sentence that is still a full sentence, not a one-word reply.
MIN_WORDS = 3
MAX_WORDS = 10
MAX_CHARS = 100
BLANK = "___"
HEADER = [
    "question",
    "question_type",
    "correct_answer",
    "accepted_answers",
    "explanation",
    "tags",
    "source",
    "licence",
]


def data_root() -> Path:
    return get_runtime_data_root() / "languages"


def sources(lang: str) -> dict[str, str]:
    """File name -> download URL."""
    code, frequency = TATOEBA_CODES[lang], FREQUENCY_PATHS[lang]
    return {
        f"{code}_sentences.tsv.bz2": TATOEBA_URL.format(
            code=code, name=f"{code}_sentences.tsv.bz2"
        ),
        "eng_sentences.tsv.bz2": TATOEBA_URL.format(code="eng", name="eng_sentences.tsv.bz2"),
        f"{code}-eng_links.tsv.bz2": TATOEBA_URL.format(
            code=code, name=f"{code}-eng_links.tsv.bz2"
        ),
        Path(frequency).name: FREQUENCY_URL.format(path=frequency),
    }


def _download(url: str, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(path.name + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "deeptutor-build-deck"})
    # Fixed https URLs from ``sources``.
    with urllib.request.urlopen(request, timeout=120) as response, partial.open("wb") as out:  # nosec B310
        shutil.copyfileobj(response, out)
    partial.replace(path)


def fetch(lang: str, offline: Path | None) -> dict[str, Path]:
    found = {}
    for name, url in sources(lang).items():
        if offline is not None:
            plain = name.removesuffix(".bz2")
            path = next((p for p in (offline / name, offline / plain) if p.is_file()), None)
            if path is None:
                raise FileNotFoundError(f"{offline} holds neither {name} nor {plain}")
        else:
            path = data_root() / "sources" / name
            if not path.is_file():
                print(f"Downloading {url}", file=sys.stderr)
                _download(url, path)
        found[name] = path
    return found


def _rows(path: Path) -> Iterator[list[str]]:
    opener = bz2.open if path.suffix == ".bz2" else open
    with opener(path, "rt", encoding="utf-8") as handle:
        for line in handle:
            yield line.rstrip("\r\n").split("\t")


def _usable(text: str) -> bool:
    # "|" separates accepted answers; a quote would need CSV escaping the
    # importer's dialect sniffing cannot always detect.
    return len(text) <= MAX_CHARS and not any(ch in text for ch in '|"\n')


def build_rows(lang: str, files: dict[str, Path], size: int) -> list[list[str]]:
    code = TATOEBA_CODES[lang]
    links = [
        (int(row[0]), int(row[1]))
        for row in _rows(files[f"{code}-eng_links.tsv.bz2"])
        if len(row) >= 2 and row[0].isdigit() and row[1].isdigit()
    ]
    linked = {sid for pair in links for sid in pair}
    target = {
        int(row[0]): row[2]
        for row in _rows(files[f"{code}_sentences.tsv.bz2"])
        if len(row) >= 3 and row[0].isdigit() and int(row[0]) in linked and _usable(row[2])
    }
    english = {
        int(row[0]): row[2]
        for row in _rows(files["eng_sentences.tsv.bz2"])
        if len(row) >= 3 and row[0].isdigit() and int(row[0]) in linked and _usable(row[2])
    }
    translations: dict[int, list[int]] = {}
    for a, b in links:  # the export may list a pair in either direction
        for sid, eid in ((a, b), (b, a)):
            if sid in target and eid in english:
                translations.setdefault(sid, []).append(eid)

    # The shortest sentence per word form (ties: fewer characters, lower id).
    best: dict[str, tuple[tuple[int, int, int], int]] = {}
    for sid in sorted(translations):
        _, tokens = tokenize(lang, target[sid])
        if not MIN_WORDS <= len(tokens) <= MAX_WORDS:
            continue
        rank = (len(tokens), len(target[sid]), sid)
        for form in {token.forms[0] for token in tokens}:
            if form not in best or rank < best[form][0]:
                best[form] = (rank, sid)

    frequency = files[Path(FREQUENCY_PATHS[lang]).name]
    rows: list[list[str]] = []
    seen: set[str] = set()
    for rank, row in enumerate(_rows(frequency), 1):
        form = normalize(lang, row[0].split(" ")[0])
        if not form or form in seen or form not in best:
            continue
        seen.add(form)
        rows.extend(_cards(lang, code, form, rank, best[form][1], target, english, translations))
        if len(seen) == size:
            break
    return rows


def _cards(lang, code, form, rank, sid, target, english, translations) -> list[list[str]]:
    sentence, tokens = tokenize(lang, target[sid])
    hits = [token for token in tokens if token.forms[0] == form]
    blanked, end = [], 0
    for token in hits:
        blanked += [sentence[end : token.start], BLANK]
        end = token.end
    blanked.append(sentence[end:])
    eids = sorted(translations[sid])
    meaning = english[eids[0]]
    others = "|".join(english[eid] for eid in eids[1 : MAX_ACCEPTED + 1])
    source = (
        f"Tatoeba #{sid} ({code}) with #{eids[0]} (eng), https://tatoeba.org/en/sentences/show/{sid}; "
        f"frequency rank {rank} in FrequencyWords {FREQUENCY_PATHS[lang]}"
    )
    explanation = f"{sentence}\n{meaning}\nSource: {source}\nLicence: {LICENCE}"
    tags = f"lang:{lang},card:"
    return [
        [
            f"{meaning}\n{''.join(blanked)}",
            "fill_blank",
            hits[0].surface,
            "",
            explanation,
            tags + "production",
            source,
            LICENCE,
        ],
        [
            sentence,
            "short_answer",
            meaning,
            others,
            explanation,
            tags + "recognition",
            source,
            LICENCE,
        ],
    ]


def write_csv(lang: str, size: int, rows: list[list[str]]) -> list[Path]:
    """One CSV, or several parts when the deck exceeds one import (500 rows)."""
    folder = data_root() / lang
    folder.mkdir(parents=True, exist_ok=True)
    chunks = [rows[i : i + MAX_ROWS] for i in range(0, len(rows), MAX_ROWS)]
    paths = []
    for number, chunk in enumerate(chunks, 1):
        suffix = "" if len(chunks) == 1 else f"-part{number}"
        path = folder / f"starter-{size}{suffix}.csv"
        with path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.writer(handle, quoting=csv.QUOTE_ALL)
            writer.writerow(HEADER)
            writer.writerows(chunk)
        paths.append(path)
    return paths


def import_csv(paths: list[Path]) -> dict:
    repo = PracticeStore(get_sqlite_session_store().db_path)
    totals = {"created": 0, "duplicates": 0}
    for path in paths:
        parsed = preview(path.read_bytes(), path.name)
        if parsed["errors"]:
            raise ValueError(f"{path.name} did not import cleanly: {parsed['errors'][:3]}")
        result = repo.commit_import(repo.stage_import(path.name, "bank", parsed["questions"]))
        for key in totals:
            totals[key] += result[key]
    return totals


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m deeptutor.services.languages.build_deck", description=__doc__.split("\n")[0]
    )
    parser.add_argument("--lang", required=True, choices=LANGUAGES)
    parser.add_argument("--size", required=True, type=int, help="number of words (2 cards each)")
    parser.add_argument("--offline", type=Path, help="read the source files from this folder")
    args = parser.parse_args(argv)
    if args.size < 1:
        parser.error("--size must be at least 1")
    files = fetch(args.lang, args.offline)
    rows = build_rows(args.lang, files, args.size)
    if not rows:
        print("No frequent word has a short example sentence in these sources.", file=sys.stderr)
        return 1
    paths = write_csv(args.lang, args.size, rows)
    result = import_csv(paths)
    print(f"Words: {len(rows) // 2} of {args.size} requested; cards: {len(rows)}")
    for path in paths:
        print(f"Deck: {path}")
    print(f"Imported: {result['created']} new, {result['duplicates']} already present")
    for name, path in files.items():
        print(f"Source: {name} ({path.stat().st_size:,} bytes)")
    print(f"Licence: {LICENCE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

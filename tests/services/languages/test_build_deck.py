"""Starter decks from Tatoeba pairs and a frequency list, offline with fixture files."""

import asyncio
import bz2
import csv
import urllib.request

import pytest

from deeptutor.services.languages import build_deck
from deeptutor.services.practice.answers import grade
from deeptutor.services.session.sqlite_store import SQLiteSessionStore

SPA = [
    (1, "El gato come."),
    (2, "El gato come pescado, pan; y queso todos los días en la cocina de mi casa."),  # long
    (3, "¿Dónde está el gato?"),
    (5, 'Él dijo "hola".'),  # quotes are skipped
    (6, "Mi casa es tu casa."),
    (7, "No tengo traducción."),
    (8, "Casa."),  # too short to be an example
]
ENG = [
    (101, "The cat eats."),
    (102, "The cat eats fish."),
    (103, "Where is the cat?"),
    (113, "Where's the cat?"),
    (105, "He said hi."),
    (106, "My house is your house."),
    (108, "House."),
]
# The export may list a pair in either direction.
LINKS = [(1, 101), (2, 102), (103, 3), (3, 113), (5, 105), (6, 106), (8, 108)]
FREQUENCY = ["de 900", "el 800", "casa 700", "está 600", "gato 500", "zzz 400", "es 300"]


def write_fixtures(folder, compress=False):
    def put(name, lines):
        data = "".join(line + "\n" for line in lines)
        if compress:
            (folder / name).write_bytes(bz2.compress(data.encode()))
        else:
            (folder / name.removesuffix(".bz2")).write_text(data, encoding="utf-8")

    put("spa_sentences.tsv.bz2", [f"{i}\tspa\t{t}" for i, t in SPA])
    put("eng_sentences.tsv.bz2", [f"{i}\teng\t{t}" for i, t in ENG])
    put("spa-eng_links.tsv.bz2", [f"{a}\t{b}" for a, b in LINKS])
    (folder / "es_50k.txt").write_text("\n".join(FREQUENCY) + "\n", encoding="utf-8")


@pytest.fixture
def deck(tmp_path, monkeypatch):
    sources = tmp_path / "sources"
    sources.mkdir()
    store = SQLiteSessionStore(tmp_path / "deck.db")
    monkeypatch.setattr(build_deck, "data_root", lambda: tmp_path / "data" / "languages")
    monkeypatch.setattr(build_deck, "get_sqlite_session_store", lambda: store)

    def no_network(*args, **kwargs):
        raise AssertionError("offline builds must not download")

    monkeypatch.setattr(urllib.request, "urlopen", no_network)
    return tmp_path, sources, store


def rows(path):
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def test_offline_build_selects_frequent_words_with_short_sentences(deck, capsys):
    tmp_path, sources, store = deck
    write_fixtures(sources)
    assert build_deck.main(["--lang", "es", "--size", "3", "--offline", str(sources)]) == 0

    path = tmp_path / "data" / "languages" / "es" / "starter-3.csv"
    written = rows(path)
    # "de" only occurs in the long sentence; "el", "casa", "está" are the top three with one.
    production = [row for row in written if row["tags"] == "lang:es,card:production"]
    assert [row["correct_answer"] for row in production] == ["El", "casa", "está"]
    assert len(written) == 6
    el, casa, esta = production
    assert el["question"] == "The cat eats.\n___ gato come."
    assert casa["question"] == "My house is your house.\nMi ___ es tu ___."  # every occurrence
    assert esta["question"] == "Where is the cat?\n¿Dónde ___ el gato?"
    recognition = {row["question"]: row for row in written if row["tags"].endswith("recognition")}
    where = recognition["¿Dónde está el gato?"]
    assert (where["question_type"], where["correct_answer"]) == (
        "short_answer",
        "Where is the cat?",
    )
    assert where["accepted_answers"] == "Where's the cat?"
    for row in written:
        assert "CC BY 2.0 FR" in row["licence"] and "CC BY-SA 4.0" in row["licence"]
        assert row["source"].startswith("Tatoeba #")
        assert "Licence: " in row["explanation"] and row["source"] in row["explanation"]

    out = capsys.readouterr().out
    assert "Imported: 6 new, 0 already present" in out
    items = asyncio.run(store.list_notebook_entries(limit=50))["items"]
    assert len(items) == 6
    card = next(item for item in items if item["correct_answer"] == "está")
    names = {c["name"] for c in card["categories"]}
    assert names == {"lang:es", "card:production"}
    assert grade(card, "esta") == (False, "accent")
    assert grade(card, "ESTÁ") == (True, None)
    assert "tatoeba.org" in card["explanation"]

    # Re-running imports nothing new.
    assert build_deck.main(["--lang", "es", "--size", "3", "--offline", str(sources)]) == 0
    assert "Imported: 0 new, 6 already present" in capsys.readouterr().out


def test_offline_build_reads_bz2_and_reports_a_short_deck(deck, capsys):
    tmp_path, sources, _ = deck
    write_fixtures(sources, compress=True)
    assert build_deck.main(["--lang", "es", "--size", "50", "--offline", str(sources)]) == 0
    assert "Words: 5 of 50 requested; cards: 10" in capsys.readouterr().out
    assert (tmp_path / "data" / "languages" / "es" / "starter-50.csv").is_file()


def test_missing_offline_source_is_named(deck):
    _, sources, _ = deck
    with pytest.raises(FileNotFoundError, match="spa_sentences"):
        build_deck.main(["--lang", "es", "--size", "3", "--offline", str(sources)])


def test_large_decks_split_into_importable_parts(deck, monkeypatch):
    tmp_path, _, _ = deck
    monkeypatch.setattr(build_deck, "MAX_ROWS", 4)
    fake = [
        [f"q{i}", "fill_blank", "a", "", "e", "lang:es,card:production", "s", "l"]
        for i in range(10)
    ]
    paths = build_deck.write_csv("es", 5, fake)
    assert [p.name for p in paths] == [
        "starter-5-part1.csv",
        "starter-5-part2.csv",
        "starter-5-part3.csv",
    ]
    assert [len(rows(p)) for p in paths] == [4, 4, 2]


def test_sources_are_the_documented_open_exports():
    urls = build_deck.sources("ja")
    assert urls["jpn_sentences.tsv.bz2"].startswith(
        "https://downloads.tatoeba.org/exports/per_language/jpn/"
    )
    assert urls["eng_sentences.tsv.bz2"].endswith("/eng/eng_sentences.tsv.bz2")
    assert urls["jpn-eng_links.tsv.bz2"].endswith("/jpn/jpn-eng_links.tsv.bz2")
    assert urls["ja_50k.txt"].endswith("FrequencyWords/master/content/2016/ja/ja_50k.txt")

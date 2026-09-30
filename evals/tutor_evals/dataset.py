"""Dataset JSONL reading, validation and writing. Format: evals/datasets/README.md."""

from __future__ import annotations

import json
import os
from pathlib import Path

SPLITS = ("train", "test")
KINDS = ("synthetic", "human")


class DatasetError(ValueError):
    """The dataset file does not follow the documented format."""


def norm_path(path: str) -> str:
    """Corpus paths compare equal across Windows and POSIX separators."""
    path = path.replace("\\", "/")
    return path[2:] if path.startswith("./") else path


def _check_gold(entry: object, where: str) -> None:
    if not isinstance(entry, dict):
        raise DatasetError(f"{where}: gold entry must be an object")
    path = entry.get("path")
    if not isinstance(path, str) or not path:
        raise DatasetError(f"{where}: gold 'path' is required and must be a non-empty string")
    for key in ("chunk_id", "doc_id"):
        value = entry.get(key)
        if value is not None and (not isinstance(value, str) or not value):
            raise DatasetError(f"{where}: gold '{key}' must be a non-empty string when present")
    page = entry.get("page")
    if page is not None and (isinstance(page, bool) or not isinstance(page, int) or page < 0):
        raise DatasetError(f"{where}: gold 'page' must be a non-negative integer when present")
    if entry.get("chunk_id") is None and page is None:
        raise DatasetError(
            f"{where}: gold entry needs a chunk_id or a page, otherwise no hit can ever match it"
        )


def validate_row(row: object, where: str, *, require_split: bool) -> None:
    if not isinstance(row, dict):
        raise DatasetError(f"{where}: each line must be a JSON object")
    for key in ("qid", "question", "domain"):
        value = row.get(key)
        if not isinstance(value, str) or not value.strip():
            raise DatasetError(f"{where}: '{key}' is required and must be a non-empty string")
    gold = row.get("gold")
    if not isinstance(gold, list) or not gold:
        raise DatasetError(f"{where}: 'gold' must be a non-empty list")
    for i, entry in enumerate(gold):
        _check_gold(entry, f"{where} gold[{i}]")
    if row.get("kind") not in KINDS:
        raise DatasetError(f"{where}: 'kind' must be one of {KINDS}, got {row.get('kind')!r}")
    split = row.get("split")
    if split is None:
        if require_split:
            raise DatasetError(
                f"{where}: 'split' is not assigned; run `python -m tutor_evals split` first"
            )
    elif split not in SPLITS:
        raise DatasetError(f"{where}: 'split' must be one of {SPLITS} or null, got {split!r}")
    source = row.get("source_chunk_id")
    if source is not None and not isinstance(source, str):
        raise DatasetError(f"{where}: 'source_chunk_id' must be a string when present")


def load_dataset(path: str | Path, *, require_split: bool = False) -> list[dict]:
    """Read and validate a dataset. Unknown fields (e.g. a human's "hard": true) are kept."""
    path = Path(path)
    rows: list[dict] = []
    seen: set[str] = set()
    with path.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            if not line.strip():
                continue
            where = f"{path.name}:{lineno}"
            try:
                row = json.loads(line)
            except ValueError as exc:
                raise DatasetError(f"{where}: invalid JSON ({exc})") from exc
            validate_row(row, where, require_split=require_split)
            if row["qid"] in seen:
                raise DatasetError(f"{where}: duplicate qid {row['qid']!r}")
            seen.add(row["qid"])
            rows.append(row)
    return rows


def dump_row(row: dict) -> str:
    return json.dumps(row, ensure_ascii=False)


def write_dataset(path: str | Path, rows: list[dict]) -> None:
    """Atomically rewrite a dataset (LF line endings, UTF-8)."""
    path = Path(path)
    tmp = path.with_name(path.name + ".tmp")
    with tmp.open("w", encoding="utf-8", newline="\n") as handle:
        for row in rows:
            handle.write(dump_row(row) + "\n")
    os.replace(tmp, path)

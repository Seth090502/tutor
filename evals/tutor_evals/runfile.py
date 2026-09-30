"""Run files (contract C2) and their per-question sidecars.

evals/runs/<run_id>.json holds exactly the C2 fields. The per-question scores that the
paired bootstrap in `compare` needs are not part of C2, so they live next to it in
<run_id>.questions.jsonl. The extension differs on purpose: the desktop app reads every
evals/runs/*.json as a C2 run, and a sidecar must not be one of them.
"""

from __future__ import annotations

from collections.abc import Sequence
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import time

from .metrics import METRICS
from .stats import bootstrap_ci

DECISIONS = ("baseline", "keep", "revert", "pending")
RUN_KEYS = (
    "run_id",
    "created_at",
    "eval",
    "dataset",
    "change",
    "config",
    "splits",
    "per_domain",
    "decision",
    "cost_usd",
)


class RunFileError(ValueError):
    """A run file or its sidecar is missing or does not follow contract C2."""


def slugify(text: str, max_len: int = 40) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")[:max_len].strip("-")
    return slug or "run"


def make_run_id(change: str, now: float | None = None) -> str:
    stamp = datetime.fromtimestamp(time.time() if now is None else now, timezone.utc)
    return f"{stamp:%Y%m%dT%H%M%SZ}-{slugify(change)}"


def summarize(records: Sequence[dict]) -> dict:
    """Mean and bootstrap 95% CI of every metric over per-question records."""
    n = len(records)
    if n == 0:
        return {"n": 0, "metrics": {}, "ci95": {}}
    columns = {m: [r[m] for r in records] for m in METRICS}
    cis = bootstrap_ci(columns)
    return {
        "n": n,
        "metrics": {m: sum(col) / n for m, col in columns.items()},
        "ci95": {m: [lo, hi] for m, (lo, hi) in cis.items()},
    }


def sidecar_path(run_path: Path) -> Path:
    return run_path.with_name(run_path.stem + ".questions.jsonl")


def _write_atomic(path: Path, text: str) -> None:
    if path.exists():
        raise FileExistsError(f"{path} already exists")
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8", newline="\n")
    os.replace(tmp, path)


def write_run(run: dict, records: Sequence[dict], runs_dir: Path) -> Path:
    """Write the sidecar, then the C2 file (whose appearance marks the run complete)."""
    runs_dir.mkdir(parents=True, exist_ok=True)
    path = runs_dir / f"{run['run_id']}.json"
    lines = "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in records)
    _write_atomic(sidecar_path(path), lines)
    _write_atomic(path, json.dumps(run, indent=2, ensure_ascii=False) + "\n")
    return path


def rewrite_run(path: Path, run: dict) -> None:
    """Replace an existing run file (used to record a decision)."""
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(
        json.dumps(run, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n"
    )
    os.replace(tmp, path)


def _check_cell(cell: object, where: str, *, ci: bool) -> None:
    if not isinstance(cell, dict) or not isinstance(cell.get("n"), int):
        raise RunFileError(f"{where}: needs an integer 'n'")
    if not isinstance(cell.get("metrics"), dict):
        raise RunFileError(f"{where}: needs a 'metrics' object")
    if ci and not isinstance(cell.get("ci95"), dict):
        raise RunFileError(f"{where}: needs a 'ci95' object")


def validate_run(run: object, where: str = "run") -> None:
    if not isinstance(run, dict):
        raise RunFileError(f"{where}: must be a JSON object")
    missing = [k for k in RUN_KEYS if k not in run]
    if missing:
        raise RunFileError(f"{where}: missing C2 fields {missing}")
    if run["decision"] not in DECISIONS:
        raise RunFileError(f"{where}: decision must be one of {DECISIONS}")
    splits = run["splits"]
    if not isinstance(splits, dict) or set(splits) != {"train", "test"}:
        raise RunFileError(f"{where}: 'splits' must have exactly train and test")
    for name, cell in splits.items():
        _check_cell(cell, f"{where} splits.{name}", ci=True)
    if not isinstance(run["per_domain"], dict):
        raise RunFileError(f"{where}: 'per_domain' must be an object")
    for domain, by_split in run["per_domain"].items():
        for name in ("train", "test"):
            _check_cell(by_split.get(name), f"{where} per_domain.{domain}.{name}", ci=False)


def resolve_run_path(ref: str, runs_dir: Path) -> Path:
    """A run is given as a file path or as a run_id inside runs_dir."""
    direct = Path(ref)
    if direct.is_file():
        return direct
    by_id = runs_dir / (ref if ref.endswith(".json") else ref + ".json")
    if by_id.is_file():
        return by_id
    raise RunFileError(f"run not found: {ref!r} (looked for that path and {by_id})")


def load_run(path: Path) -> dict:
    try:
        run = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as exc:
        raise RunFileError(f"{path}: invalid JSON ({exc})") from exc
    validate_run(run, path.name)
    return run


def load_sidecar(run_path: Path) -> dict[str, dict]:
    """qid -> per-question record."""
    path = sidecar_path(run_path)
    if not path.is_file():
        raise RunFileError(
            f"{path.name} is missing: compare needs the per-question scores written by `run`"
        )
    records: dict[str, dict] = {}
    with path.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            if line.strip():
                try:
                    record = json.loads(line)
                    records[record["qid"]] = record
                except (ValueError, KeyError) as exc:
                    raise RunFileError(f"{path.name}:{lineno}: bad record ({exc!r})") from exc
    return records

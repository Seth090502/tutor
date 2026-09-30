"""Locations inside the repo. Run files store repo-relative dataset paths (contract C2)."""

from __future__ import annotations

from pathlib import Path

EVALS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = EVALS_DIR.parent
DATASETS_DIR = EVALS_DIR / "datasets"
RUNS_DIR = EVALS_DIR / "runs"
DEFAULT_DATASET = DATASETS_DIR / "retrieval-v1.jsonl"


def repo_relative(path: str | Path) -> str:
    """Posix path relative to the repo root, or the absolute posix path if outside it."""
    resolved = Path(path).resolve()
    try:
        return resolved.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()

"""Run a retrieval eval and write a C2 run file.

Without an arm, questions go straight to rag-engine ``POST /search``. With an arm (CORPUS.md
Stage 0 step 8), they go through DeepTutor ``POST /api/library/search`` (contract C11) with
that arm's ``library.*`` settings, set through ``/api/library/settings`` for the run and
restored afterwards:

* A: raw BM25 (all library steps off);
* B: A's settings; the difference is the rag-engine index (boosted title and heading fields,
  maths tokenizer), so run A and B against the engine build under test;
* C: plus query rewrites fused by RRF;
* D: plus the listwise rerank;
* E: plus agentic search.

Library search returns the evidence pack in reading order (strongest first and last), so
hits are scored in ``ranks.final`` order. The pack holds at most 12 hits.
"""

from __future__ import annotations

from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import time
from typing import NamedTuple

from .dataset import SPLITS, load_dataset
from .httpjson import backend_url, engine_url, post_json, request_json
from .metrics import METRICS, score_questions
from .paths import RUNS_DIR, repo_relative
from .runfile import make_run_id, summarize, write_run

MODES = ("hybrid", "bm25", "vector")
RESERVED_CONFIG = ("mode", "k", "mode_used", "arm", "library")
_OFF = {"rewrite": False, "rerank": False, "agentic": False}
ARMS: dict[str, dict[str, bool]] = {
    "A": _OFF,
    "B": _OFF,
    "C": {**_OFF, "rewrite": True},
    "D": {**_OFF, "rewrite": True, "rerank": True},
    "E": {"rewrite": True, "rerank": True, "agentic": True},
}
LIBRARY_MAX_K = 12


class RunResult(NamedTuple):
    path: Path
    run: dict
    # Full summaries including ci95 (C2 per_domain cells carry none), for the console report.
    split_cells: dict[str, dict]
    domain_cells: dict[str, dict[str, dict]]


def search(url: str, question: str, *, mode: str, k: int, timeout: float = 60.0) -> dict:
    response = post_json(
        f"{url}/search", {"query": question, "k": k, "mode": mode}, timeout=timeout
    )
    if not isinstance(response.get("hits"), list):
        raise ValueError(f"{url}/search response has no 'hits' list: {str(response)[:200]}")
    return response


def library_search(url: str, question: str, *, k: int, timeout: float = 120.0) -> dict:
    """DeepTutor library search, with the pack in rank order as ``hits``."""
    response = post_json(f"{url}/api/library/search", {"query": question, "k": k}, timeout=timeout)
    pack = response.get("pack")
    if not isinstance(pack, list):
        raise ValueError(
            f"{url}/api/library/search response has no 'pack' list: {str(response)[:200]}"
        )
    hits = sorted(pack, key=lambda h: (h.get("ranks") or {}).get("final", len(pack) + 1))
    return {"mode_used": response.get("mode_used"), "hits": hits}


def run_retrieval(
    dataset_path: str | Path,
    *,
    change: str,
    mode: str = "hybrid",
    k: int = 10,
    url: str | None = None,
    arm: str | None = None,
    runs_dir: Path = RUNS_DIR,
    baseline: bool = False,
    extra_config: dict[str, str] | None = None,
    workers: int = 4,
    now: float | None = None,
) -> RunResult:
    if mode not in MODES:
        raise ValueError(f"mode must be one of {MODES}")
    if k < 10:
        raise ValueError("k must be at least 10 (recall@10, mrr@10 and ndcg@10 need the top 10)")
    if not change.strip():
        raise ValueError("--change is required: one sentence naming the single change under test")
    extra = dict(extra_config or {})
    clash = [key for key in extra if key in RESERVED_CONFIG]
    if clash:
        raise ValueError(f"--config cannot set reserved keys {clash}")
    if arm is not None:
        if arm not in ARMS:
            raise ValueError(f"arm must be one of {sorted(ARMS)}")
        if mode != "hybrid":
            raise ValueError("library search always asks rag-engine for hybrid; use --mode hybrid")
        if k > LIBRARY_MAX_K:
            raise ValueError(f"library search packs at most {LIBRARY_MAX_K} hits; use --k 10-12")

    rows = load_dataset(dataset_path, require_split=True)
    if not rows:
        raise ValueError(f"{dataset_path} has no questions")

    if arm is None:
        base_url = engine_url(url)
        with ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
            responses = list(
                pool.map(lambda r: search(base_url, r["question"], mode=mode, k=k), rows)
            )
    else:
        base_url = backend_url(url)
        settings_url = f"{base_url}/api/library/settings"
        previous = request_json("GET", settings_url)
        request_json("PUT", settings_url, ARMS[arm])
        try:
            with ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
                responses = list(
                    pool.map(lambda r: library_search(base_url, r["question"], k=k), rows)
                )
        finally:
            request_json("PUT", settings_url, {f: bool(previous.get(f)) for f in ARMS["E"]})
        extra = {**extra, "arm": arm, "library": dict(ARMS[arm])}

    scored = score_questions(
        (r["qid"], r["gold"], resp["hits"]) for r, resp in zip(rows, responses)
    )
    records = []
    modes_used: set[str] = set()
    for row, resp in zip(rows, responses):
        metrics, first_rank = scored[row["qid"]]
        if resp.get("mode_used"):
            modes_used.add(str(resp["mode_used"]))
        records.append(
            {
                "qid": row["qid"],
                "split": row["split"],
                "domain": row["domain"],
                "first_rank": first_rank,
                "retrieved": [h.get("chunk_id") for h in resp["hits"]],
                **metrics,
            }
        )

    by_split: dict[str, list[dict]] = {s: [] for s in SPLITS}
    by_domain: dict[str, dict[str, list[dict]]] = defaultdict(lambda: {s: [] for s in SPLITS})
    for record in records:
        by_split[record["split"]].append(record)
        by_domain[record["domain"]][record["split"]].append(record)
    split_cells = {s: summarize(by_split[s]) for s in SPLITS}
    domain_cells = {
        d: {s: summarize(cells[s]) for s in SPLITS} for d, cells in sorted(by_domain.items())
    }

    created = time.time() if now is None else now
    run = {
        "run_id": make_run_id(change, created),
        "created_at": created,
        "eval": "retrieval",
        "dataset": repo_relative(dataset_path),
        "change": change,
        "config": {"mode": mode, "k": k, "mode_used": sorted(modes_used), **extra},
        "splits": split_cells,
        # C2 has no ci95 slot in per_domain cells; the console report prints them.
        "per_domain": {
            d: {s: {"n": c["n"], "metrics": c["metrics"]} for s, c in cells.items()}
            for d, cells in domain_cells.items()
        },
        "decision": "baseline" if baseline else "pending",
        # Scoring makes no LLM calls; engine-side embedding spend and the library pipeline's
        # model calls (arms C-E) are not attributed.
        "cost_usd": 0.0,
    }
    path = write_run(run, records, runs_dir)
    return RunResult(path, run, split_cells, domain_cells)


def _fmt(cell: dict, metric: str) -> str:
    lo, hi = cell["ci95"][metric]
    return f"{cell['metrics'][metric]:.3f} [{lo:.3f}, {hi:.3f}]"


def format_report(result: RunResult) -> str:
    run = result.run
    lines = [
        f"run      {run['run_id']}",
        f"change   {run['change']}",
        f"config   {run['config']}",
        f"file     {result.path}",
        "",
    ]
    requested = run["config"]["mode"]
    used = run["config"]["mode_used"]
    if used and used != [requested]:
        lines += [f"WARNING: requested mode {requested!r} but the engine reported {used}", ""]
    header = f"{'':<22}{'n':>5}  " + "  ".join(f"{m:<22}" for m in METRICS)
    lines.append(header)

    def row(label: str, cell: dict) -> str:
        if cell["n"] == 0:
            return f"{label:<22}{0:>5}"
        return f"{label:<22}{cell['n']:>5}  " + "  ".join(f"{_fmt(cell, m):<22}" for m in METRICS)

    for split in SPLITS:
        lines.append(row(split, result.split_cells[split]))
    lines.append("")
    lines.append("per domain (95% bootstrap CI over questions; small n means wide CI)")
    for domain, cells in result.domain_cells.items():
        for split in SPLITS:
            lines.append(row(f"{domain}/{split}", cells[split]))
    return "\n".join(lines)

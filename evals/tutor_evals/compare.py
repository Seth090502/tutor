"""Paired bootstrap comparison of two retrieval runs, and the keep/revert/noise verdict.

Verdict (the owner's hillclimbing rule, applied to the chosen metric):
  keep    train AND test both improve and both lower CI bounds on the delta are above 0
  revert  either split gets worse (delta below 0)
  noise   anything else: no split is worse, but the gain is not clear of the noise floor
Only "keep" keeps a change. The test split gates the change; it must never be used to
choose what to try next.
"""

from __future__ import annotations

from pathlib import Path

from .dataset import SPLITS
from .metrics import METRICS
from .paths import RUNS_DIR
from .runfile import (
    RunFileError,
    load_run,
    load_sidecar,
    resolve_run_path,
    rewrite_run,
    summarize,
)
from .stats import bootstrap_ci

EPS = 1e-12  # float dust is not a regression
VERDICT_TO_DECISION = {"keep": "keep", "revert": "revert", "noise": "revert"}  # noise is not kept


def decide(deltas: dict[str, tuple[float, float, float]]) -> str:
    """deltas: split -> (delta, ci_lo, ci_hi)."""
    if any(delta < -EPS for delta, _, _ in deltas.values()):
        return "revert"
    if all(delta > EPS and lo > 0 for delta, lo, _ in deltas.values()):
        return "keep"
    return "noise"


def compare_runs(
    base_ref: str, new_ref: str, *, metric: str = "ndcg@10", runs_dir: Path = RUNS_DIR
) -> dict:
    if metric not in METRICS:
        raise RunFileError(f"metric must be one of {METRICS}")
    base_path = resolve_run_path(base_ref, runs_dir)
    new_path = resolve_run_path(new_ref, runs_dir)
    base, new = load_run(base_path), load_run(new_path)
    if base["dataset"] != new["dataset"]:
        raise RunFileError(
            f"runs use different datasets ({base['dataset']} vs {new['dataset']}); "
            "a paired comparison needs the same dataset"
        )
    base_q, new_q = load_sidecar(base_path), load_sidecar(new_path)
    if set(base_q) != set(new_q):
        only_base, only_new = len(set(base_q) - set(new_q)), len(set(new_q) - set(base_q))
        raise RunFileError(
            f"runs cover different questions ({only_base} only in baseline, {only_new} only in new); "
            "rerun both on the same dataset file"
        )
    moved = [q for q in base_q if base_q[q]["split"] != new_q[q]["split"]]
    if moved:
        raise RunFileError(
            f"{len(moved)} questions changed split between the runs (e.g. {moved[0]})"
        )

    splits: dict[str, dict] = {}
    for split in SPLITS:
        qids = sorted(q for q, r in new_q.items() if r["split"] == split)
        if not qids:
            raise RunFileError(f"split {split!r} is empty: cannot apply the keep rule without it")
        base_vals = [base_q[q][metric] for q in qids]
        new_vals = [new_q[q][metric] for q in qids]
        diffs = [n - b for n, b in zip(new_vals, base_vals)]
        delta_lo, delta_hi = bootstrap_ci({"delta": diffs})["delta"]
        base_cell = summarize([base_q[q] for q in qids])
        new_cell = summarize([new_q[q] for q in qids])
        splits[split] = {
            "n": len(qids),
            "base": base_cell["metrics"][metric],
            "base_ci95": base_cell["ci95"][metric],
            "new": new_cell["metrics"][metric],
            "new_ci95": new_cell["ci95"][metric],
            "delta": sum(diffs) / len(diffs),
            "delta_ci95": [delta_lo, delta_hi],
        }
    verdict = decide(
        {s: (c["delta"], c["delta_ci95"][0], c["delta_ci95"][1]) for s, c in splits.items()}
    )
    return {
        "metric": metric,
        "baseline": base["run_id"],
        "new": new["run_id"],
        "new_path": new_path,
        "new_decision": new["decision"],
        "splits": splits,
        "verdict": verdict,
    }


def write_decision(result: dict) -> str:
    """Record the verdict as the new run's `decision`. Returns the decision written."""
    if result["new_decision"] == "baseline":
        raise RunFileError(f"{result['new']} is a baseline run; refusing to overwrite its decision")
    path = result["new_path"]
    run = load_run(path)
    run["decision"] = VERDICT_TO_DECISION[result["verdict"]]
    rewrite_run(path, run)
    return run["decision"]


def format_comparison(result: dict) -> str:
    metric = result["metric"]
    lines = [
        f"metric   {metric}",
        f"baseline {result['baseline']}",
        f"new      {result['new']}",
        "",
        f"{'split':<7}{'n':>5}  {'baseline':<24}{'new':<24}{'delta (paired 95% CI)':<30}",
    ]
    for split, c in result["splits"].items():
        b, n, d = c["base_ci95"], c["new_ci95"], c["delta_ci95"]
        base_txt = f"{c['base']:.3f} [{b[0]:.3f}, {b[1]:.3f}]"
        new_txt = f"{c['new']:.3f} [{n[0]:.3f}, {n[1]:.3f}]"
        delta_txt = f"{c['delta']:+.3f} [{d[0]:+.3f}, {d[1]:+.3f}]"
        lines.append(f"{split:<7}{c['n']:>5}  {base_txt:<24}{new_txt:<24}{delta_txt}")
    verdict = result["verdict"]
    meaning = {
        "keep": "train and test both improve, both CI lower bounds above 0",
        "revert": "at least one split got worse",
        "noise": "no split got worse, but the gain is not clear of the noise floor: do not keep",
    }[verdict]
    lines += ["", f"verdict: {verdict.upper()} ({meaning})"]
    return "\n".join(lines)

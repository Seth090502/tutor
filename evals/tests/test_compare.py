import json
import random

import pytest
from tutor_evals.cli import main
from tutor_evals.compare import compare_runs, decide, format_comparison, write_decision
from tutor_evals.metrics import METRICS
from tutor_evals.runfile import RunFileError, load_run, summarize, write_run


def make_run(runs_dir, run_id, values, *, decision="pending", dataset="evals/datasets/d.jsonl"):
    """values: qid -> (split, ndcg@10). Other metrics are fixed at 0.5."""
    records = [
        {
            "qid": qid,
            "split": split,
            "domain": "math",
            "first_rank": 1,
            "retrieved": [],
            **dict.fromkeys(METRICS, 0.5),
            "ndcg@10": ndcg,
        }
        for qid, (split, ndcg) in values.items()
    ]
    cells = {s: summarize([r for r in records if r["split"] == s]) for s in ("train", "test")}
    run = {
        "run_id": run_id,
        "created_at": 1790000000.0,
        "eval": "retrieval",
        "dataset": dataset,
        "change": run_id,
        "config": {"mode": "bm25", "k": 10},
        "splits": cells,
        "per_domain": {
            "math": {s: {"n": c["n"], "metrics": c["metrics"]} for s, c in cells.items()}
        },
        "decision": decision,
        "cost_usd": 0.0,
    }
    return write_run(run, records, runs_dir)


def paired(rng_seed, train_diffs, test_diffs):
    """Baseline values plus per-question deltas for each split -> (base, new) dicts."""
    rng = random.Random(rng_seed)
    base, new = {}, {}
    for split, diffs in (("train", train_diffs), ("test", test_diffs)):
        for i, d in enumerate(diffs):
            b = rng.uniform(0.3, 0.6)
            base[f"{split}{i}"] = (split, b)
            new[f"{split}{i}"] = (split, b + d)
    return base, new


def jitter(n, lo, hi, seed):
    rng = random.Random(seed)
    return [rng.uniform(lo, hi) for _ in range(n)]


def alternating(n_pos, n_neg):
    return [0.1] * n_pos + [-0.1] * n_neg


def verdict_for(tmp_path, train_diffs, test_diffs):
    base, new = paired(1, train_diffs, test_diffs)
    make_run(tmp_path, "base", base, decision="baseline")
    make_run(tmp_path, "new", new)
    return compare_runs("base", "new", metric="ndcg@10", runs_dir=tmp_path)


def test_keep_when_both_splits_clearly_improve(tmp_path):
    result = verdict_for(tmp_path, jitter(60, 0.02, 0.08, 1), jitter(60, 0.02, 0.08, 2))
    assert result["verdict"] == "keep"
    for split in ("train", "test"):
        assert result["splits"][split]["delta_ci95"][0] > 0
    assert "verdict: KEEP" in format_comparison(result)


def test_revert_when_test_gets_worse_even_if_train_improves(tmp_path):
    result = verdict_for(tmp_path, jitter(60, 0.02, 0.08, 1), jitter(60, -0.08, -0.02, 2))
    assert result["verdict"] == "revert"
    assert result["splits"]["train"]["delta_ci95"][0] > 0 > result["splits"]["test"]["delta"]


def test_revert_when_train_gets_worse(tmp_path):
    result = verdict_for(tmp_path, jitter(60, -0.08, -0.02, 1), jitter(60, 0.02, 0.08, 2))
    assert result["verdict"] == "revert"


def test_noise_when_gains_are_inside_the_noise_floor(tmp_path):
    # 26 questions +0.1 and 24 questions -0.1: mean delta +0.004, CI straddles 0 in both splits
    result = verdict_for(tmp_path, alternating(26, 24), alternating(26, 24))
    assert result["verdict"] == "noise"
    for split in ("train", "test"):
        c = result["splits"][split]
        assert c["delta"] > 0 and c["delta_ci95"][0] < 0 < c["delta_ci95"][1]


def test_noise_when_only_one_split_clears_the_noise_floor(tmp_path):
    result = verdict_for(tmp_path, jitter(60, 0.02, 0.08, 1), alternating(26, 24))
    assert result["verdict"] == "noise"
    assert result["splits"]["train"]["delta_ci95"][0] > 0


def test_identical_runs_are_noise(tmp_path):
    base, _ = paired(1, [0.0] * 30, [0.0] * 30)
    make_run(tmp_path, "a", base, decision="baseline")
    make_run(tmp_path, "b", base)
    assert compare_runs("a", "b", runs_dir=tmp_path)["verdict"] == "noise"


def test_decide_rule_directly():
    assert decide({"train": (0.05, 0.01, 0.09), "test": (0.04, 0.001, 0.08)}) == "keep"
    assert decide({"train": (0.05, 0.01, 0.09), "test": (0.04, -0.001, 0.08)}) == "noise"
    assert decide({"train": (0.05, 0.01, 0.09), "test": (-0.01, -0.05, 0.03)}) == "revert"
    assert decide({"train": (0.0, 0.0, 0.0), "test": (0.0, 0.0, 0.0)}) == "noise"


def test_write_decision_maps_verdicts_onto_c2_decisions(tmp_path):
    for i, (diffs, expected) in enumerate(
        (
            ((jitter(60, 0.02, 0.08, 1), jitter(60, 0.02, 0.08, 2)), "keep"),
            ((jitter(60, 0.02, 0.08, 1), jitter(60, -0.08, -0.02, 2)), "revert"),
            # Noise is not kept, so it is recorded as a revert.
            ((alternating(26, 24), alternating(26, 24)), "revert"),
        )
    ):
        d = tmp_path / str(i)
        result = verdict_for(d, *diffs)
        assert write_decision(result) == expected
        assert load_run(d / "new.json")["decision"] == expected  # still a valid C2 file
        assert load_run(d / "base.json")["decision"] == "baseline"


def test_write_decision_refuses_to_overwrite_a_baseline(tmp_path):
    base, new = paired(1, jitter(40, 0.02, 0.08, 1), jitter(40, 0.02, 0.08, 2))
    make_run(tmp_path, "a", base, decision="baseline")
    make_run(tmp_path, "b", new, decision="baseline")
    with pytest.raises(RunFileError, match="baseline"):
        write_decision(compare_runs("a", "b", runs_dir=tmp_path))


def test_rejects_runs_that_are_not_paired(tmp_path):
    base, new = paired(1, [0.05] * 20, [0.05] * 20)
    make_run(tmp_path, "a", base, decision="baseline")
    make_run(tmp_path, "other-dataset", new, dataset="evals/datasets/other.jsonl")
    with pytest.raises(RunFileError, match="different datasets"):
        compare_runs("a", "other-dataset", runs_dir=tmp_path)
    dropped = dict(list(new.items())[:-1])
    make_run(tmp_path, "fewer", dropped)
    with pytest.raises(RunFileError, match="different questions"):
        compare_runs("a", "fewer", runs_dir=tmp_path)


def test_cli_compare_writes_decision(tmp_path, capsys):
    base, new = paired(1, jitter(60, 0.02, 0.08, 1), jitter(60, 0.02, 0.08, 2))
    make_run(tmp_path, "base", base, decision="baseline")
    path = make_run(tmp_path, "new", new)
    code = main(["compare", "base", "new", "--runs-dir", str(tmp_path), "--write-decision"])
    out = capsys.readouterr().out
    assert code == 0 and "verdict: KEEP" in out and "decision=keep" in out
    assert json.loads(path.read_text(encoding="utf-8"))["decision"] == "keep"

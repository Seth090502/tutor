import json
import math
import re

from fakes import hit
import pytest
from tutor_evals.cli import main
from tutor_evals.httpjson import HttpError
from tutor_evals.run_retrieval import format_report, run_retrieval
from tutor_evals.runfile import sidecar_path


def gold_chunk(chunk_id):
    return [{"chunk_id": chunk_id, "path": "corpus/math/books/x.pdf"}]


def q(qid, question, gold, domain, split):
    return {
        "qid": qid,
        "question": question,
        "gold": gold,
        "domain": domain,
        "split": split,
        "kind": "synthetic",
    }


def noise(n, start=0):
    return [hit(f"n{i}", doc_id=f"nd{i}", path=f"n{i}.pdf") for i in range(start, start + n)]


# question text -> hits the stub engine returns
ANSWERS = {
    "t1": [hit("c1")] + noise(9),  # rank 1
    "t2": noise(2) + [hit("c2")] + noise(7, 2),  # rank 3
    "t3": noise(7) + [hit("c3")] + noise(2, 7),  # rank 8
    "t4": noise(10),  # not retrieved
    "e1": noise(1)
    + [hit("other", doc_id="docE", path="corpus/math/books/e.pdf", page=4, end=5)]
    + noise(8, 1),
    "e2": [hit("cz", doc_id="dz", path="corpus/chemistry/books/z.pdf", page=7, domain="chemistry")]
    + noise(9),
    "e3": noise(10),
}

ROWS = [
    q("t1", "t1", gold_chunk("c1"), "math", "train"),
    q("t2", "t2", gold_chunk("c2"), "math", "train"),
    q("t3", "t3", gold_chunk("c3"), "chemistry", "train"),
    q("t4", "t4", gold_chunk("c4"), "chemistry", "train"),
    q(
        "e1",
        "e1",
        [{"doc_id": "docE", "path": "corpus/math/books/e.pdf", "page": 5}],
        "math",
        "test",
    ),
    q("e2", "e2", [{"path": "corpus\\chemistry\\books\\z.pdf", "page": 7}], "chemistry", "test"),
    q("e3", "e3", gold_chunk("c5"), "math", "test"),
]


def engine(payload):
    return 200, {"mode_used": "bm25", "hits": ANSWERS[payload["query"]]}


@pytest.fixture
def dataset(tmp_path):
    path = tmp_path / "ds.jsonl"
    path.write_text("\n".join(json.dumps(r) for r in ROWS) + "\n", encoding="utf-8")
    return path


def test_run_scores_and_writes_c2_file(fake_server, dataset, tmp_path):
    server = fake_server({"/search": engine})
    runs = tmp_path / "runs"
    result = run_retrieval(
        dataset,
        change="BM25 only: baseline",
        mode="bm25",
        k=10,
        url=server.url,
        runs_dir=runs,
        baseline=True,
        now=1790000000.0,
        extra_config={"reranker": "none"},
    )

    # the engine was asked exactly what the config says
    assert len(server.requests) == 7
    assert {(r["path"], json.dumps(r["payload"], sort_keys=True)) for r in server.requests} >= {
        ("/search", json.dumps({"k": 10, "mode": "bm25", "query": "t1"}, sort_keys=True))
    }

    run = json.loads(result.path.read_text(encoding="utf-8"))
    assert (
        run["run_id"] == "20260921T141320Z-bm25-only-baseline"
    )  # UTC stamp of now + slug of change
    assert result.path.name == run["run_id"] + ".json"

    train, test = run["splits"]["train"], run["splits"]["test"]
    assert (train["n"], test["n"]) == (4, 3)
    # train: ranks 1, 3, 8, miss
    assert train["metrics"]["recall@5"] == pytest.approx(2 / 4)
    assert train["metrics"]["recall@10"] == pytest.approx(3 / 4)
    assert train["metrics"]["mrr@10"] == pytest.approx((1 + 1 / 3 + 1 / 8) / 4)
    assert train["metrics"]["ndcg@10"] == pytest.approx(
        (1 + 1 / math.log2(4) + 1 / math.log2(9)) / 4
    )
    # test: doc_id+page match at rank 2, path+page match at rank 1, miss
    assert test["metrics"]["recall@5"] == pytest.approx(2 / 3)
    assert test["metrics"]["recall@10"] == pytest.approx(2 / 3)
    assert test["metrics"]["mrr@10"] == pytest.approx((1 / 2 + 1) / 3)
    assert test["metrics"]["ndcg@10"] == pytest.approx((1 / math.log2(3) + 1) / 3)

    pd = run["per_domain"]
    assert pd["math"]["train"]["n"] == 2 and pd["math"]["test"]["n"] == 2
    assert pd["chemistry"]["train"]["n"] == 2 and pd["chemistry"]["test"]["n"] == 1
    assert pd["math"]["train"]["metrics"]["recall@5"] == 1.0
    assert pd["chemistry"]["train"]["metrics"]["recall@10"] == pytest.approx(0.5)

    assert run["config"] == {"mode": "bm25", "k": 10, "mode_used": ["bm25"], "reranker": "none"}
    assert run["decision"] == "baseline" and run["cost_usd"] == 0.0
    assert run["created_at"] == 1790000000.0 and run["change"] == "BM25 only: baseline"

    # the sidecar that compare needs: one record per question, C2 file unaffected by it
    lines = [
        json.loads(x) for x in sidecar_path(result.path).read_text(encoding="utf-8").splitlines()
    ]
    assert [r["qid"] for r in lines] == [r["qid"] for r in ROWS]
    assert lines[2]["first_rank"] == 8 and lines[3]["first_rank"] is None
    assert lines[0]["retrieved"][0] == "c1"
    assert [p.name for p in runs.glob("*.json")] == [result.path.name]  # sidecar is not *.json


def test_c2_schema(fake_server, dataset, tmp_path):
    """Checked field by field against docs/tutor/CONTRACTS.md C2, independent of the code's validator."""
    server = fake_server({"/search": engine})
    result = run_retrieval(
        dataset, change="schema check", mode="bm25", url=server.url, runs_dir=tmp_path / "r"
    )
    run = json.loads(result.path.read_text(encoding="utf-8"))
    metric_keys = {"recall@5", "recall@10", "mrr@10", "ndcg@10"}

    assert set(run) == {
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
    }
    assert re.fullmatch(r"\d{8}T\d{6}Z-[a-z0-9-]+", run["run_id"])
    assert isinstance(run["created_at"], float)
    assert run["eval"] == "retrieval"
    assert isinstance(run["dataset"], str) and "\\" not in run["dataset"]
    assert isinstance(run["change"], str)
    assert run["config"]["mode"] == "bm25" and run["config"]["k"] == 10
    assert (
        run["decision"] in {"baseline", "keep", "revert", "pending"}
        and run["decision"] == "pending"
    )
    assert isinstance(run["cost_usd"], float)

    assert set(run["splits"]) == {"train", "test"}
    for cell in run["splits"].values():
        assert set(cell) == {"n", "metrics", "ci95"} and isinstance(cell["n"], int)
        assert set(cell["metrics"]) == metric_keys and set(cell["ci95"]) == metric_keys
        for name, (lo, hi) in cell["ci95"].items():
            assert 0.0 <= lo <= hi <= 1.0
            assert lo - 1e-9 <= cell["metrics"][name] <= hi + 1e-9
    for by_split in run["per_domain"].values():
        assert set(by_split) == {"train", "test"}
        for cell in by_split.values():
            assert set(cell) == {"n", "metrics"}
            assert set(cell["metrics"]) in (metric_keys, set())


def test_empty_split_follows_c2_example_shape(fake_server, tmp_path):
    rows = [r for r in ROWS if r["split"] == "train"]
    path = tmp_path / "train_only.jsonl"
    path.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")
    server = fake_server({"/search": engine})
    run = run_retrieval(path, change="x", mode="bm25", url=server.url, runs_dir=tmp_path / "r").run
    assert run["splits"]["test"] == {"n": 0, "metrics": {}, "ci95": {}}


def test_bootstrap_is_reproducible(fake_server, dataset, tmp_path):
    server = fake_server({"/search": engine})
    a = run_retrieval(dataset, change="a", mode="bm25", url=server.url, runs_dir=tmp_path / "r")
    b = run_retrieval(dataset, change="b", mode="bm25", url=server.url, runs_dir=tmp_path / "r")
    assert a.run["splits"] == b.run["splits"]


def test_warns_when_engine_ran_a_different_mode(fake_server, dataset, tmp_path):
    server = fake_server({"/search": engine})  # stub always reports bm25
    result = run_retrieval(
        dataset, change="hybrid please", mode="hybrid", url=server.url, runs_dir=tmp_path / "r"
    )
    assert result.run["config"]["mode_used"] == ["bm25"]
    assert "WARNING: requested mode 'hybrid' but the engine reported ['bm25']" in format_report(
        result
    )


def test_engine_error_aborts_the_run_instead_of_scoring_zero(fake_server, dataset, tmp_path):
    server = fake_server({"/search": lambda payload: (400, {"error": "bad query"})})
    with pytest.raises(HttpError, match="bad query"):
        run_retrieval(dataset, change="x", mode="bm25", url=server.url, runs_dir=tmp_path / "r")
    assert not (tmp_path / "r").exists()


def test_rejects_unassigned_split_and_small_k(dataset, tmp_path):
    rows = [dict(r, split=None) for r in ROWS]
    bad = tmp_path / "unsplit.jsonl"
    bad.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")
    with pytest.raises(ValueError, match="split"):
        run_retrieval(bad, change="x", runs_dir=tmp_path / "r")
    with pytest.raises(ValueError, match="at least 10"):
        run_retrieval(dataset, change="x", k=5, runs_dir=tmp_path / "r")


def test_cli_run_end_to_end(fake_server, dataset, tmp_path, capsys):
    server = fake_server({"/search": engine})
    code = main(
        [
            "run",
            "--dataset",
            str(dataset),
            "--change",
            "cli smoke",
            "--mode",
            "bm25",
            "--url",
            server.url,
            "--runs-dir",
            str(tmp_path / "r"),
            "--baseline",
        ]
    )
    out = capsys.readouterr().out
    assert code == 0
    assert "recall@10" in out and "math/train" in out and "chemistry/test" in out
    assert len(list((tmp_path / "r").glob("*.json"))) == 1

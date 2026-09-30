import json
import random

import pytest
from tutor_evals.dataset import DatasetError, load_dataset
from tutor_evals.split import apply_split, document_groups


def row(qid, doc, domain="math", extra_gold=(), **gold_extra):
    gold = [
        {
            "chunk_id": f"{qid}-chunk",
            "doc_id": doc,
            "path": f"corpus/{domain}/books/{doc}.pdf",
            **gold_extra,
        }
    ]
    gold += list(extra_gold)
    return {
        "qid": qid,
        "question": "What is the thing in question?",
        "gold": gold,
        "domain": domain,
        "split": None,
        "kind": "synthetic",
    }


def many_rows(n_docs=60, per_doc=4):
    return [
        row(f"q{d}-{i}", f"doc{d}", domain="math" if d % 2 else "chemistry")
        for d in range(n_docs)
        for i in range(per_doc)
    ]


def splits(rows, **kw):
    apply_split(rows, **kw)
    return {r["qid"]: r["split"] for r in rows}


def test_deterministic_for_same_seed_and_order_independent():
    a = splits(many_rows(), seed=3)
    assert a == splits(many_rows(), seed=3)
    shuffled = many_rows()
    random.Random(1).shuffle(shuffled)
    assert a == splits(shuffled, seed=3)


def test_seed_changes_assignment():
    assert splits(many_rows(), seed=1) != splits(many_rows(), seed=2)


def test_no_document_crosses_splits():
    rows = many_rows()
    apply_split(rows, seed=0)
    by_doc = {}
    for r in rows:
        by_doc.setdefault(r["gold"][0]["doc_id"], set()).add(r["split"])
    assert all(len(s) == 1 for s in by_doc.values())
    assert {"train", "test"} == {r["split"] for r in rows}


def test_default_ratio_is_about_70_30_by_document():
    rows = many_rows(n_docs=200, per_doc=1)
    apply_split(rows)
    frac = sum(r["split"] == "train" for r in rows) / len(rows)
    assert 0.6 <= frac <= 0.8


def test_existing_documents_keep_their_split_when_the_dataset_grows():
    small = splits(many_rows(40), seed=5)
    grown = splits(many_rows(80), seed=5)
    assert all(grown[qid] == split for qid, split in small.items())


def test_cross_document_question_ties_its_documents_together():
    rows = [row(f"q{d}", f"doc{d}") for d in range(40)]
    bridge = row(
        "bridge",
        "doc3",
        extra_gold=[{"chunk_id": "bc", "doc_id": "doc7", "path": "corpus/math/books/doc7.pdf"}],
    )
    rows.append(bridge)
    apply_split(rows, seed=2)
    by_qid = {r["qid"]: r["split"] for r in rows}
    assert by_qid["q3"] == by_qid["q7"] == by_qid["bridge"]


def test_same_document_named_by_doc_id_or_path_is_one_group():
    by_id = row("a", "doc1")
    by_path_only = {
        "qid": "b",
        "question": "x y z",
        "domain": "math",
        "split": None,
        "kind": "human",
        "gold": [{"path": "corpus\\math\\books\\doc1.pdf", "page": 2}],
    }
    # ties doc:doc1 to the path via a gold entry that has both
    groups = document_groups([by_id, by_path_only])
    assert groups["a"] == groups["b"]


def test_changed_assignments_are_reported():
    rows = many_rows(40)
    apply_split(rows, seed=1)
    summary = apply_split(rows, seed=2)
    assert summary["changed"] > 0 and any("changed split" in w for w in summary["warnings"])


def test_bad_train_frac_rejected():
    with pytest.raises(ValueError):
        apply_split(many_rows(4), train_frac=1.0)


def test_dataset_validation(tmp_path):
    path = tmp_path / "d.jsonl"

    def write(*rows):
        path.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")

    write(row("q1", "d1"))
    assert load_dataset(path)[0]["qid"] == "q1"
    with pytest.raises(DatasetError, match="split.*not assigned"):
        load_dataset(path, require_split=True)
    write(row("q1", "d1"), row("q1", "d2"))
    with pytest.raises(DatasetError, match="duplicate qid"):
        load_dataset(path)
    bad = row("q2", "d1")
    bad["gold"] = [{"path": "only/a/path.pdf"}]  # no chunk_id and no page: can never match
    write(bad)
    with pytest.raises(DatasetError, match="chunk_id or a page"):
        load_dataset(path)

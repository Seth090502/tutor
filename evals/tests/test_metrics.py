"""Metrics against hand-computed cases. log2 values: 1/log2(3)=0.63093, 1/log2(5)=0.43068."""

import math

from fakes import hit
import pytest
from tutor_evals.metrics import METRICS, credit_hits, hit_matches, score_questions


def score(gold, hits):
    ((metrics, first_rank),) = score_questions([("q", gold, hits)]).values()
    return metrics, first_rank


def filler(n, start=0):
    return [hit(f"x{i}", doc_id=f"other{i}", path=f"other{i}.pdf") for i in range(start, start + n)]


G1 = [{"chunk_id": "c1", "path": "p.pdf"}]


def test_found_at_rank_1():
    m, rank = score(G1, [hit("c1")] + filler(9))
    assert m == {"recall@5": 1.0, "recall@10": 1.0, "mrr@10": 1.0, "ndcg@10": 1.0}
    assert rank == 1


def test_found_at_rank_3():
    m, rank = score(G1, filler(2) + [hit("c1")] + filler(7, 2))
    assert m["recall@5"] == 1.0 and m["recall@10"] == 1.0
    assert m["mrr@10"] == pytest.approx(1 / 3)
    assert m["ndcg@10"] == pytest.approx(1 / math.log2(4))  # 0.5
    assert rank == 3


def test_found_at_rank_8_is_outside_recall_5():
    m, _ = score(G1, filler(7) + [hit("c1")] + filler(2, 7))
    assert m["recall@5"] == 0.0 and m["recall@10"] == 1.0
    assert m["mrr@10"] == pytest.approx(1 / 8)
    assert m["ndcg@10"] == pytest.approx(1 / math.log2(9))


def test_found_beyond_cutoff_scores_zero_but_reports_rank():
    m, rank = score(G1, filler(11) + [hit("c1")])
    assert all(m[name] == 0.0 for name in METRICS)
    assert rank == 12


def test_nothing_relevant_and_no_hits():
    assert all(v == 0.0 for v in score(G1, filler(10))[0].values())
    m, rank = score(G1, [])
    assert all(v == 0.0 for v in m.values()) and rank is None


def test_two_gold_both_found():
    gold = [{"chunk_id": "a", "path": "p"}, {"chunk_id": "b", "path": "p"}]
    m, _ = score(gold, [hit("x"), hit("a"), hit("y"), hit("b")])
    dcg = 1 / math.log2(3) + 1 / math.log2(5)
    idcg = 1 + 1 / math.log2(3)
    assert m["recall@5"] == 1.0 and m["recall@10"] == 1.0
    assert m["mrr@10"] == 0.5
    assert m["ndcg@10"] == pytest.approx(dcg / idcg)  # 0.65092


def test_two_gold_one_found():
    gold = [{"chunk_id": "a", "path": "p"}, {"chunk_id": "b", "path": "p"}]
    m, _ = score(gold, [hit("a")] + filler(9))
    assert m["recall@5"] == 0.5 and m["recall@10"] == 0.5 and m["mrr@10"] == 1.0
    assert m["ndcg@10"] == pytest.approx(1 / (1 + 1 / math.log2(3)))  # 0.61314


def test_second_hit_on_same_gold_entry_is_not_relevant_again():
    gold = [{"chunk_id": "c1", "doc_id": "d", "path": "p.pdf", "page": 4}]
    by_chunk_then_page = [
        hit("c1", doc_id="d", path="p.pdf", page=4),
        hit("c2", doc_id="d", path="p.pdf", page=4),
    ]
    assert credit_hits(gold, by_chunk_then_page) == [0, None]
    m, rank = score(gold, by_chunk_then_page)
    assert m["ndcg@10"] == 1.0 and m["recall@10"] == 1.0 and rank == 1
    # a neighbouring chunk on the gold page ranks first: it gets the credit, c1 adds nothing
    m, rank = score(gold, [by_chunk_then_page[1], by_chunk_then_page[0]])
    assert m["mrr@10"] == 1.0 and m["ndcg@10"] == 1.0 and rank == 1


@pytest.mark.parametrize(
    ("gold", "h", "expected"),
    [
        (
            {"chunk_id": "c1", "path": "a.pdf"},
            hit("c1", doc_id="zzz", path="b.pdf", page=None),
            True,
        ),
        ({"chunk_id": "c1", "path": "a.pdf"}, hit("c2"), False),
        # doc_id + page, page inside the hit's span
        (
            {"doc_id": "d", "path": "a.pdf", "page": 5},
            hit("c2", doc_id="d", path="other.pdf", page=4, end=6),
            True,
        ),
        # doc_id matches but page is outside the span
        (
            {"doc_id": "d", "path": "a.pdf", "page": 7},
            hit("c2", doc_id="d", path="other.pdf", page=4, end=6),
            False,
        ),
        # path + page, Windows separators in gold
        (
            {"path": "corpus\\math\\x.pdf", "page": 3},
            hit("c2", doc_id="new", path="corpus/math/x.pdf", page=3),
            True,
        ),
        # path without page never matches on its own
        ({"chunk_id": "c9", "path": "a.pdf"}, hit("c2", doc_id="d", path="a.pdf", page=3), False),
        # hit without page info cannot match by page
        ({"path": "a.pdf", "page": 3}, {"chunk_id": "c2", "doc_id": "d", "path": "a.pdf"}, False),
        # different document, same page
        (
            {"doc_id": "d", "path": "a.pdf", "page": 3},
            hit("c2", doc_id="e", path="b.pdf", page=3),
            False,
        ),
    ],
)
def test_hit_matching_rules(gold, h, expected):
    assert hit_matches(gold, h) is expected

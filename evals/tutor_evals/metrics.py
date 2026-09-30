"""Hit matching and retrieval metrics (recall@5, recall@10, mrr@10, ndcg@10).

Matching (see evals/datasets/README.md): a returned hit matches a gold entry if ANY of
  1. gold.chunk_id == hit.chunk_id
  2. gold.doc_id == hit.doc_id and gold.page lies in [hit.page_start, hit.page_end]
  3. gold.path == hit.path (separator-normalised) and the same page test
Rules 2 and 3 need a gold page and a hit page span; page-less hits only match by chunk_id.

Each gold entry is credited at most once, by the highest-ranked hit that matches it. Any
further hit that matches only already-credited entries counts as non-relevant, so every
metric stays in [0, 1]. The resulting binary relevance is scored by ir_measures
(pytrec_eval backend): recall is credited gold entries / gold entries, nDCG uses gains of
1 with log2(rank + 1) discounts.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence

from .dataset import norm_path

METRICS = ("recall@5", "recall@10", "mrr@10", "ndcg@10")


def hit_matches(gold: dict, hit: dict) -> bool:
    chunk_id = gold.get("chunk_id")
    if chunk_id and hit.get("chunk_id") == chunk_id:
        return True
    page = gold.get("page")
    start = hit.get("page_start")
    if page is None or start is None:
        return False
    end = hit.get("page_end")
    if not start <= page <= (start if end is None else end):
        return False
    if gold.get("doc_id") and hit.get("doc_id") == gold["doc_id"]:
        return True
    hit_path = hit.get("path")
    return isinstance(hit_path, str) and norm_path(gold["path"]) == norm_path(hit_path)


def credit_hits(gold: Sequence[dict], hits: Sequence[dict]) -> list[int | None]:
    """For each hit in rank order: the index of the gold entry it credits, or None."""
    credited: set[int] = set()
    out: list[int | None] = []
    for hit in hits:
        found = next(
            (i for i, g in enumerate(gold) if i not in credited and hit_matches(g, hit)), None
        )
        if found is not None:
            credited.add(found)
        out.append(found)
    return out


def score_questions(
    items: Iterable[tuple[str, Sequence[dict], Sequence[dict]]],
) -> dict[str, tuple[dict[str, float], int | None]]:
    """Score (qid, gold, ranked_hits) items.

    Returns qid -> (metrics, first_rank) where first_rank is the 1-based rank of the first
    relevant hit among the returned hits, or None.
    """
    try:
        import ir_measures
        from ir_measures import Qrel, ScoredDoc
    except ImportError as exc:
        raise RuntimeError(
            "ir_measures is required: pip install -r evals/requirements.txt"
        ) from exc

    # Object API on purpose: ir_measures 0.4.3's parse_measure() fails on Python 3.14.
    measures = {
        "recall@5": ir_measures.R @ 5,
        "recall@10": ir_measures.R @ 10,
        "mrr@10": ir_measures.RR @ 10,
        "ndcg@10": ir_measures.nDCG @ 10,
    }
    by_measure = {str(m): name for name, m in measures.items()}

    qrels: list[Qrel] = []
    run: list[ScoredDoc] = []
    first_ranks: dict[str, int | None] = {}
    has_hits: set[str] = set()
    for qid, gold, hits in items:
        qrels.extend(Qrel(qid, f"g{i}", 1) for i in range(len(gold)))
        credits = credit_hits(gold, hits)
        first_ranks[qid] = next((r for r, c in enumerate(credits, 1) if c is not None), None)
        for rank, credit in enumerate(credits, 1):
            doc_id = f"g{credit}" if credit is not None else f"x{rank}"
            run.append(ScoredDoc(qid, doc_id, -float(rank)))
        if credits:
            has_hits.add(qid)

    scores: dict[str, dict[str, float]] = {qid: {} for qid in first_ranks}
    for result in ir_measures.iter_calc(list(measures.values()), qrels, run):
        scores[result.query_id][by_measure[str(result.measure)]] = float(result.value)

    out: dict[str, tuple[dict[str, float], int | None]] = {}
    for qid, metrics in scores.items():
        if qid in has_hits:
            missing = [m for m in METRICS if m not in metrics]
            if missing:
                raise RuntimeError(f"ir_measures returned no {missing} for question {qid}")
        else:  # no hits returned: pytrec_eval omits the query; every metric is 0
            metrics = dict.fromkeys(METRICS, 0.0)
        out[qid] = ({m: metrics[m] for m in METRICS}, first_ranks[qid])
    return out

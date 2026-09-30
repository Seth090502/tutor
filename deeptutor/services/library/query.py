"""Query pipeline pieces (CORPUS.md Stage 0 steps 1-2): rewrites, RRF, rerank, agentic search.

Every model call goes through an ``Ask`` callable ``(system, prompt) -> reply``, so the
service decides which model runs them and tests pass a fake. A failed or unreadable
model reply never fails the search: the step is skipped, the raw query's results
stand, and the failure is logged.
"""

from __future__ import annotations

from collections.abc import Awaitable, Callable, Sequence
from dataclasses import dataclass
import logging
from typing import Any

from deeptutor.utils.json_parser import parse_json_response

logger = logging.getLogger(__name__)

Ask = Callable[[str, str], Awaitable[str]]

RRF_K = 60
MIN_REWRITES = 3
MAX_REWRITES = 5
REWRITE_KINDS = ("vocabulary", "notation", "passage", "prerequisite")
# Flatness: see is_flat.
FLAT_TOP_N = 5
FLAT_MARGIN = 0.1
RERANK_CANDIDATES = 40
RERANK_SNIPPET_CHARS = 300
MAX_AGENTIC_SEARCHES = 4
MAX_AGENTIC_STEPS = 6
AGENTIC_VIEW = 10
AGENTIC_SNIPPET_CHARS = 200
OPEN_CHUNK_CHARS = 2000

# ---------------------------------------------------------------------------
# Rewrites
# ---------------------------------------------------------------------------

REWRITE_SYSTEM = (
    "You turn a learner's question into search queries for a keyword (BM25) index of "
    "textbooks, papers and notes. Reply with JSON only."
)
REWRITE_PROMPT = """Question: {query}

Write {low} to {high} search queries as JSON:
{{"queries": [{{"kind": "vocabulary" | "notation" | "passage" | "prerequisite", "query": "..."}}]}}

- vocabulary: the words a textbook would use for this: technical terms and synonyms.
- notation: the symbols or formula as printed in a book, in plain Unicode (∫, ∑, ≤, x², H₂O), not LaTeX.
- passage: two or three sentences that a textbook passage answering the question would contain.
- prerequisite: the concept a learner must know first, named as a textbook would name it.

Write at least one vocabulary, one passage and one prerequisite query. Add a notation query
when the question involves symbols, formulas or units. Do not answer the question."""


def parse_rewrites(reply: str, query: str) -> list[str]:
    """Up to 5 distinct rewritten queries from the model's JSON, raw query excluded.

    Accepts ``{"queries": [{"kind", "query"} | str, ...]}`` or a bare list; anything
    else yields no rewrites.
    """
    data = parse_json_response(reply or "", logger_instance=logger, fallback=None)
    items = data.get("queries") if isinstance(data, dict) else data
    if not isinstance(items, list):
        return []
    seen = {" ".join(query.split()).casefold()}
    out: list[str] = []
    for item in items:
        text = item.get("query") if isinstance(item, dict) else item
        if not isinstance(text, str):
            continue
        text = " ".join(text.split())
        key = text.casefold()
        if not text or key in seen:
            continue
        seen.add(key)
        out.append(text)
        if len(out) == MAX_REWRITES:
            break
    return out


async def rewrite_queries(ask: Ask, query: str) -> list[str]:
    """The queries to run: the raw query first, then the model's rewrites."""
    try:
        reply = await ask(
            REWRITE_SYSTEM,
            REWRITE_PROMPT.format(query=query, low=MIN_REWRITES, high=MAX_REWRITES),
        )
    except Exception:
        logger.warning("Library query rewrite failed; searching the raw query only", exc_info=True)
        return [query]
    rewrites = parse_rewrites(reply, query)
    if not rewrites:
        logger.warning("Library query rewrite returned no usable queries: %.200r", reply)
    return [query, *rewrites]


# ---------------------------------------------------------------------------
# Fusion
# ---------------------------------------------------------------------------


@dataclass
class Fused:
    hit: dict[str, Any]
    score: float


def rrf(lists: Sequence[Sequence[dict[str, Any]]], k: int = RRF_K) -> list[Fused]:
    """Reciprocal rank fusion: score = sum over lists of 1 / (k + rank), rank from 1.

    Hits are keyed by ``chunk_id``; the first list that returned a chunk supplies its
    fields. Ties keep first-seen order (the raw query's list comes first).
    """
    fused: dict[str, Fused] = {}
    for hits in lists:
        for rank, hit in enumerate(hits, start=1):
            chunk_id = str(hit.get("chunk_id") or "")
            if not chunk_id:
                continue
            entry = fused.get(chunk_id)
            if entry is None:
                fused[chunk_id] = entry = Fused(hit, 0.0)
            entry.score += 1.0 / (k + rank)
    return sorted(fused.values(), key=lambda f: -f.score)


def is_flat(scores: Sequence[float], lists: int, k: int = RRF_K) -> bool:
    """Whether the top fused scores are too close to trust the fused order.

    With ``lists`` queries fused, a chunk ranked first by every query scores
    ``S_max = lists / (k + 1)``. The top is flat when there are at least two hits and
    the first fused score beats the ``FLAT_TOP_N``-th (or the last, if fewer) by less
    than ``FLAT_MARGIN * S_max``: no chunk won clearly more of the queries' votes.

    A single list can never clear the margin (its rank 1 and rank 5 differ by
    ``1/61 - 1/65``, about ``0.06 * S_max``), so without rewrites every result with
    two or more hits is flat: one query's ranks carry no agreement signal.
    """
    if len(scores) < 2 or lists < 1:
        return False
    s_max = lists / (k + 1)
    nth = scores[min(FLAT_TOP_N, len(scores)) - 1]
    return scores[0] - nth < FLAT_MARGIN * s_max


# ---------------------------------------------------------------------------
# Listwise rerank
# ---------------------------------------------------------------------------

RERANK_SYSTEM = (
    "You rank library passages by how well they help answer a learner's question. "
    "Reply with JSON only."
)
RERANK_PROMPT = """Question: {query}

Passages:
{passages}

Order the passage numbers from most to least useful for answering the question, as JSON:
{{"order": [3, 1, 2]}}
Leave out passages that do not help."""


def _describe(index: int, hit: dict[str, Any], chars: int) -> str:
    headings = " > ".join(str(h) for h in hit.get("heading_path") or [])
    label = str(hit.get("title") or hit.get("path") or "Untitled")
    if headings:
        label += f" / {headings}"
    text = " ".join(str(hit.get("text") or "").split())[:chars]
    return f"[{index}] {label}\n{text}"


def parse_order(reply: str, count: int) -> list[int]:
    """Distinct 1-based passage numbers within ``1..count`` from ``{"order": [...]}``."""
    data = parse_json_response(reply or "", logger_instance=logger, fallback=None)
    order = data.get("order") if isinstance(data, dict) else data
    if not isinstance(order, list):
        return []
    out: list[int] = []
    for value in order:
        try:
            number = int(value)
        except (TypeError, ValueError):
            continue
        if 1 <= number <= count and number not in out:
            out.append(number)
    return out


async def rerank(ask: Ask, query: str, fused: list[Fused]) -> list[Fused] | None:
    """The top ``RERANK_CANDIDATES`` reordered by one listwise call, or None on failure.

    Passages the model leaves out follow the ranked ones in fused order, and the rest
    of the fused list follows unchanged.
    """
    head, tail = fused[:RERANK_CANDIDATES], fused[RERANK_CANDIDATES:]
    passages = "\n\n".join(
        _describe(i, f.hit, RERANK_SNIPPET_CHARS) for i, f in enumerate(head, start=1)
    )
    try:
        reply = await ask(RERANK_SYSTEM, RERANK_PROMPT.format(query=query, passages=passages))
    except Exception:
        logger.warning("Library rerank failed; keeping the fused order", exc_info=True)
        return None
    order = parse_order(reply, len(head))
    if not order:
        logger.warning("Library rerank returned no usable order: %.200r", reply)
        return None
    ranked = [head[n - 1] for n in order]
    rest = [f for i, f in enumerate(head, start=1) if i not in order]
    return [*ranked, *rest, *tail]


# ---------------------------------------------------------------------------
# Agentic search
# ---------------------------------------------------------------------------

AGENTIC_SYSTEM = (
    "You search a learner's library to gather the passages needed to answer their "
    "question. You see the best results so far. Reply with one JSON action only."
)
AGENTIC_PROMPT = """Question: {query}

Queries run so far: {queries}

Best results so far:
{results}
{opened}
Choose one action, as JSON:
{{"action": "search", "query": "..."}}  to run another search ({left} left),
{{"action": "open_chunk", "chunk_id": "..."}}  to read one result in full,
{{"action": "done"}}  when the results cover the question."""


def parse_action(reply: str) -> dict[str, str]:
    """``{"action": "search"|"open_chunk"|"done", ...}``; anything unreadable is done."""
    data = parse_json_response(reply or "", logger_instance=logger, fallback=None)
    if not isinstance(data, dict):
        return {"action": "done"}
    action = str(data.get("action") or "").strip()
    if action == "search" and isinstance(data.get("query"), str) and data["query"].strip():
        return {"action": "search", "query": " ".join(data["query"].split())}
    if action == "open_chunk" and isinstance(data.get("chunk_id"), str):
        return {"action": "open_chunk", "chunk_id": data["chunk_id"].strip()}
    return {"action": "done"}


Search = Callable[[str], Awaitable[list[dict[str, Any]] | None]]
OpenChunk = Callable[[str], Awaitable[str]]


async def agentic_search(
    ask: Ask,
    query: str,
    queries: list[str],
    lists: list[list[dict[str, Any]]],
    search: Search,
    open_chunk: OpenChunk,
) -> None:
    """Let the model run up to ``MAX_AGENTIC_SEARCHES`` more searches; extends in place.

    ``search`` returns a result list already filtered by the leak guard (None when
    that search failed); ``open_chunk`` returns the text the model may read.
    """
    searches = 0
    opened: list[str] = []
    for _ in range(MAX_AGENTIC_STEPS):
        if searches >= MAX_AGENTIC_SEARCHES:
            return
        top = rrf(lists)[:AGENTIC_VIEW]
        results = "\n\n".join(
            f"{_describe(i, f.hit, AGENTIC_SNIPPET_CHARS)}\n(chunk_id: {f.hit.get('chunk_id')})"
            for i, f in enumerate(top, start=1)
        )
        prompt = AGENTIC_PROMPT.format(
            query=query,
            queries="; ".join(queries),
            results=results or "(none)",
            opened="".join(f"\n{text}\n" for text in opened),
            left=MAX_AGENTIC_SEARCHES - searches,
        )
        try:
            action = parse_action(await ask(AGENTIC_SYSTEM, prompt))
        except Exception:
            logger.warning("Library agentic search step failed; stopping", exc_info=True)
            return
        if action["action"] == "search":
            searches += 1
            if action["query"].casefold() in {q.casefold() for q in queries}:
                continue
            queries.append(action["query"])
            hits = await search(action["query"])
            if hits is not None:
                lists.append(hits)
        elif action["action"] == "open_chunk":
            opened.append(await open_chunk(action["chunk_id"]))
        else:
            return


__all__ = [
    "FLAT_MARGIN",
    "FLAT_TOP_N",
    "MAX_AGENTIC_SEARCHES",
    "RRF_K",
    "Ask",
    "Fused",
    "agentic_search",
    "is_flat",
    "parse_action",
    "parse_order",
    "parse_rewrites",
    "rerank",
    "rewrite_queries",
    "rrf",
]

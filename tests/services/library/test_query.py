"""Query pipeline pieces (CORPUS.md Stage 0 steps 1-2): rewrites, RRF, flatness, rerank, agentic."""

from __future__ import annotations

import pytest

from deeptutor.services.library.query import (
    FLAT_MARGIN,
    MAX_AGENTIC_SEARCHES,
    RRF_K,
    Fused,
    agentic_search,
    is_flat,
    parse_action,
    parse_order,
    parse_rewrites,
    rerank,
    rewrite_queries,
    rrf,
)

from .fake_rag_engine import FakeLLM


def hit(chunk_id: str, **extra) -> dict:
    return {"chunk_id": chunk_id, "doc_id": chunk_id.split(":")[0], "text": chunk_id, **extra}


# ---------------------------------------------------------------------------
# Rewrites
# ---------------------------------------------------------------------------


def test_parse_rewrites_reads_kinds_and_drops_the_raw_query_and_duplicates() -> None:
    reply = """```json
{"queries": [
  {"kind": "vocabulary", "query": "limit of a function  epsilon delta"},
  {"kind": "notation", "query": "lim x→a f(x) = L"},
  {"kind": "passage", "query": "We say f(x) approaches L as x approaches a."},
  {"kind": "prerequisite", "query": "Limits"},
  {"kind": "vocabulary", "query": "LIMIT OF A FUNCTION epsilon delta"},
  {"kind": "vocabulary", "query": ""}
]}
```"""

    assert parse_rewrites(reply, "limits") == [
        "limit of a function epsilon delta",
        "lim x→a f(x) = L",
        "We say f(x) approaches L as x approaches a.",
    ]


def test_parse_rewrites_keeps_at_most_five() -> None:
    reply = '{"queries": ["a", "b", "c", "d", "e", "f", "g"]}'

    assert parse_rewrites(reply, "q") == ["a", "b", "c", "d", "e"]


@pytest.mark.parametrize(
    "reply",
    ["not json at all", "", '{"queries": "limits"}', '{"other": []}', "[1, 2, null]"],
)
def test_parse_rewrites_yields_nothing_for_unusable_replies(reply: str) -> None:
    assert parse_rewrites(reply, "q") == []


@pytest.mark.asyncio
async def test_rewrite_queries_puts_the_raw_query_first() -> None:
    llm = FakeLLM(rewrite='{"queries": [{"kind": "vocabulary", "query": "derivative"}]}')

    assert await rewrite_queries(llm, "slope of a curve") == ["slope of a curve", "derivative"]
    assert "slope of a curve" in llm.calls[0][1]


@pytest.mark.asyncio
@pytest.mark.parametrize("reply", [RuntimeError("no LLM configured"), "garbage"])
async def test_rewrite_queries_falls_back_to_the_raw_query(reply) -> None:
    assert await rewrite_queries(FakeLLM(rewrite=reply), "slope") == ["slope"]


# ---------------------------------------------------------------------------
# RRF and flatness
# ---------------------------------------------------------------------------


def test_rrf_sums_reciprocal_ranks_with_k_60() -> None:
    fused = rrf([[hit("a:0"), hit("b:0")], [hit("b:0"), hit("c:0")]])

    scores = {f.hit["chunk_id"]: f.score for f in fused}
    assert RRF_K == 60
    assert scores["b:0"] == pytest.approx(1 / 62 + 1 / 61)
    assert scores["a:0"] == pytest.approx(1 / 61)
    assert scores["c:0"] == pytest.approx(1 / 62)
    assert [f.hit["chunk_id"] for f in fused] == ["b:0", "a:0", "c:0"]


def test_rrf_keeps_the_first_lists_fields_and_first_seen_order_on_ties() -> None:
    fused = rrf([[hit("a:0", text="raw")], [hit("b:0"), hit("a:0", text="rewrite")], [hit("b:0")]])
    by_id = {f.hit["chunk_id"]: f for f in fused}

    assert by_id["a:0"].hit["text"] == "raw"
    tie = rrf([[hit("x:0")], [hit("y:0")]])
    assert [f.hit["chunk_id"] for f in tie] == ["x:0", "y:0"]


def test_flatness_uses_the_gap_to_the_fifth_score_against_s_max() -> None:
    lists = 5
    s_max = lists / 61
    top = s_max
    assert is_flat([top, top, top, top, top - FLAT_MARGIN * s_max * 0.99], lists)
    assert not is_flat([top, top, top, top, top - FLAT_MARGIN * s_max * 1.01], lists)


def test_flatness_uses_the_last_score_when_fewer_than_five() -> None:
    s_max = 3 / 61
    assert not is_flat([s_max, s_max * 0.5], 3)
    assert is_flat([s_max, s_max * 0.95], 3)


def test_a_single_query_list_is_always_flat_and_one_hit_never_is() -> None:
    single = [f.score for f in rrf([[hit(f"d:{i}") for i in range(10)]])]

    assert is_flat(single, 1)
    assert is_flat(single[:2], 1)
    assert not is_flat(single[:1], 1)
    assert not is_flat([], 3)


def test_consensus_across_rewrites_is_not_flat() -> None:
    # Every query ranks d:0 first; the others are scattered.
    lists = [[hit("d:0"), hit(f"x{i}:0"), hit(f"y{i}:0")] for i in range(4)]
    fused = rrf(lists)

    assert fused[0].hit["chunk_id"] == "d:0"
    assert not is_flat([f.score for f in fused], len(lists))


# ---------------------------------------------------------------------------
# Rerank
# ---------------------------------------------------------------------------


def test_parse_order_keeps_valid_distinct_numbers() -> None:
    assert parse_order('{"order": [3, "1", 3, 9, 0, "x", 2]}', 3) == [3, 1, 2]
    assert parse_order("[2, 1]", 2) == [2, 1]
    assert parse_order("nonsense", 3) == []


@pytest.mark.asyncio
async def test_rerank_orders_the_head_and_appends_what_the_model_left_out() -> None:
    fused = [Fused(hit(f"d:{i}"), 1.0 - i / 100) for i in range(45)]
    llm = FakeLLM(rerank='{"order": [3, 1]}')

    out = await rerank(llm, "q", fused)

    ids = [f.hit["chunk_id"] for f in out]
    assert ids[:3] == ["d:2", "d:0", "d:1"]
    assert ids[3:40] == [f"d:{i}" for i in range(3, 40)]
    assert ids[40:] == [f"d:{i}" for i in range(40, 45)]
    assert "[40]" in llm.calls[0][1] and "[41]" not in llm.calls[0][1]


@pytest.mark.asyncio
@pytest.mark.parametrize("reply", [RuntimeError("down"), '{"order": []}'])
async def test_rerank_failure_returns_none(reply) -> None:
    assert await rerank(FakeLLM(rerank=reply), "q", [Fused(hit("a:0"), 1.0)]) is None


# ---------------------------------------------------------------------------
# Agentic search
# ---------------------------------------------------------------------------


def test_parse_action() -> None:
    assert parse_action('{"action": "search", "query": " more  terms "}') == {
        "action": "search",
        "query": "more terms",
    }
    assert parse_action('{"action": "open_chunk", "chunk_id": "d:3"}') == {
        "action": "open_chunk",
        "chunk_id": "d:3",
    }
    assert parse_action('{"action": "search", "query": ""}') == {"action": "done"}
    assert parse_action("I think we are done") == {"action": "done"}


@pytest.mark.asyncio
async def test_agentic_search_is_capped_at_four_searches() -> None:
    llm = FakeLLM(agentic=[f'{{"action": "search", "query": "q{i}"}}' for i in range(10)])
    searched: list[str] = []

    async def search(query: str):
        searched.append(query)
        return [hit(f"{query}:0")]

    async def open_chunk(chunk_id: str) -> str:
        raise AssertionError("not opened")

    queries, lists = ["raw"], [[hit("raw:0")]]
    await agentic_search(llm, "raw", queries, lists, search, open_chunk)

    assert MAX_AGENTIC_SEARCHES == 4
    assert searched == ["q0", "q1", "q2", "q3"]
    assert queries == ["raw", "q0", "q1", "q2", "q3"]
    assert len(lists) == 5


@pytest.mark.asyncio
async def test_agentic_search_opens_chunks_and_stops_when_done() -> None:
    llm = FakeLLM(
        agentic=[
            '{"action": "open_chunk", "chunk_id": "raw:0"}',
            '{"action": "search", "query": "RAW"}',
            '{"action": "done"}',
        ]
    )

    async def search(query: str):
        raise AssertionError("a repeated query is not run again")

    async def open_chunk(chunk_id: str) -> str:
        return f"[{chunk_id}] full text of the chunk"

    queries, lists = ["raw"], [[hit("raw:0")]]
    await agentic_search(llm, "raw", queries, lists, search, open_chunk)

    assert llm.count("agentic") == 3
    assert "full text of the chunk" in llm.calls[1][1]
    assert queries == ["raw"]

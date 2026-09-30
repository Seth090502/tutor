"""Evidence packer (CORPUS.md Stage 0 step 3): quota, section expansion, budget, ordering."""

from __future__ import annotations

import asyncio

import httpx
import pytest

from deeptutor.services.library.engine import RagEngine
from deeptutor.services.library.leak import LeakGuard
from deeptutor.services.library.packer import (
    MAX_TOKENS,
    MIN_TOKENS,
    SOURCE_QUOTA,
    build_pack,
    edge_order,
    estimate_tokens,
    quota_order,
)

from .fake_rag_engine import FakeRagEngine


@pytest.fixture(autouse=True)
def _engine_env(monkeypatch):
    monkeypatch.delenv("RAG_ENGINE_URL", raising=False)
    monkeypatch.delenv("RAG_ENGINE_TOKEN", raising=False)


def hit(doc: str, ordinal: int, text: str = "t", headings: list[str] | None = None) -> dict:
    return {
        "chunk_id": f"{doc}:{ordinal}",
        "doc_id": doc,
        "title": doc,
        "text": text,
        "heading_path": headings or [],
        "ranks": {"bm25": 1, "vector": None},
    }


async def no_pages(doc_id: str, start: int, limit: int):
    raise AssertionError("no section fetch expected")


def ids(pack: list[dict]) -> list[str]:
    return [item["chunk_id"] for item in pack]


def test_edge_order_puts_the_strongest_first_and_last() -> None:
    assert edge_order([1, 2, 3, 4, 5, 6]) == [1, 3, 5, 6, 4, 2]
    assert edge_order([1, 2, 3, 4, 5]) == [1, 3, 5, 4, 2]
    assert edge_order([1]) == [1]
    assert edge_order([]) == []


def test_quota_caps_each_document_then_tops_up_in_rank_order() -> None:
    ranked = [hit("a", i) for i in range(5)] + [hit("b", 0), hit("b", 1)]

    ordered = quota_order(ranked, k=6)

    assert SOURCE_QUOTA == 3
    # a:0-2 and b:0-1 are within quota; a:3 tops up to 6; a:4 follows.
    assert ids(ordered) == ["a:0", "a:1", "a:2", "a:3", "b:0", "b:1", "a:4"]
    assert ids(quota_order(ranked, k=4)) == ["a:0", "a:1", "a:2", "b:0", "b:1", "a:3", "a:4"]


@pytest.mark.asyncio
async def test_pack_keeps_k_hits_in_edge_order_and_records_the_final_rank() -> None:
    ranked = [hit(f"d{i}", 0, "x" * 2000) for i in range(10)]

    pack = await build_pack(ranked, 6, no_pages, LeakGuard())

    assert ids(pack) == ["d0:0", "d2:0", "d4:0", "d5:0", "d3:0", "d1:0"]
    assert [item["ranks"]["final"] for item in pack] == [1, 3, 5, 6, 4, 2]
    assert pack[0]["ranks"]["bm25"] == 1
    assert pack[0]["section"] == ["[c:d0:0] " + "x" * 2000]


@pytest.mark.asyncio
async def test_pack_clamps_k_to_four_through_twelve() -> None:
    ranked = [hit(f"d{i}", 0, "x" * 4000) for i in range(20)]

    assert len(await build_pack(ranked, 1, no_pages, LeakGuard())) == 4
    # 12 hits of 1000 tokens would pass 8k, so the budget stops at 7.
    assert len(await build_pack(ranked, 50, no_pages, LeakGuard())) == 7


@pytest.mark.asyncio
async def test_pack_grows_past_k_until_the_token_floor_up_to_twelve() -> None:
    short = [hit(f"d{i}", 0, "x" * 400) for i in range(20)]  # about 100 tokens each

    pack = await build_pack(short, 4, no_pages, LeakGuard())

    assert len(pack) == 12
    assert sum(estimate_tokens(s) for item in pack for s in item["section"]) < MIN_TOKENS


@pytest.mark.asyncio
async def test_pack_stays_within_the_token_budget() -> None:
    ranked = [hit(f"d{i}", 0, "x" * 12000) for i in range(6)]  # about 3000 tokens each

    pack = await build_pack(ranked, 8, no_pages, LeakGuard())

    total = sum(estimate_tokens(s) for item in pack for s in item["section"])
    assert len(pack) == 2
    assert total <= MAX_TOKENS


def section_engine() -> FakeRagEngine:
    engine = FakeRagEngine()
    engine.add(
        "book",
        [
            ("Chapter intro", ["Ch 1"]),
            ("Definition of a limit", ["Ch 1", "1.1 Limits"]),
            ("Limit laws for sums", ["Ch 1", "1.1 Limits"]),
            ("Squeeze theorem statement", ["Ch 1", "1.1 Limits"]),
            ("One-sided limits", ["Ch 1", "1.1 Limits"]),
            ("Continuity defined", ["Ch 1", "1.2 Continuity"]),
        ],
        title="Calculus",
    )
    return engine


def engine_hit(engine: FakeRagEngine, doc: str, ordinal: int) -> dict:
    return engine.hit(engine.docs[doc][ordinal], doc, 1.0, 1)


@pytest.mark.asyncio
async def test_hits_expand_to_neighbours_under_the_same_heading() -> None:
    fake = section_engine()
    client = RagEngine(transport=fake.transport())

    pack = await build_pack([engine_hit(fake, "book", 2)], 4, client.chunks, LeakGuard())

    assert pack[0]["section"] == [
        "[c:book:1] Definition of a limit",
        "[c:book:2] Limit laws for sums",
        "[c:book:3] Squeeze theorem statement",
        "[c:book:4] One-sided limits",
    ]
    request = fake.requests[0]
    assert request.url.path == "/documents/book/chunks"
    assert dict(request.url.params) == {"from": "0", "limit": "5"}


@pytest.mark.asyncio
async def test_a_hit_inside_an_earlier_section_is_not_repeated() -> None:
    fake = section_engine()
    client = RagEngine(transport=fake.transport())
    ranked = [engine_hit(fake, "book", 2), engine_hit(fake, "book", 3), engine_hit(fake, "book", 5)]

    pack = await build_pack(ranked, 4, client.chunks, LeakGuard())

    assert sorted(ids(pack)) == ["book:2", "book:5"]


@pytest.mark.asyncio
async def test_expansion_stops_at_a_withheld_neighbour_and_counts_it() -> None:
    fake = section_engine()
    client = RagEngine(transport=fake.transport())
    guard = LeakGuard.for_answers(["squeeze theorem"])

    pack = await build_pack([engine_hit(fake, "book", 2)], 4, client.chunks, guard)

    assert pack[0]["section"] == [
        "[c:book:1] Definition of a limit",
        "[c:book:2] Limit laws for sums",
    ]
    assert guard.withheld == 1


@pytest.mark.asyncio
async def test_a_hit_without_headings_or_a_failed_fetch_stays_alone() -> None:
    fake = section_engine()
    fake.down = True
    client = RagEngine(transport=fake.transport())

    pack = await build_pack(
        [engine_hit(fake, "book", 2), hit("note", 0, "loose note")], 4, client.chunks, LeakGuard()
    )

    assert [item["section"] for item in pack] == [
        ["[c:book:2] Limit laws for sums"],
        ["[c:note:0] loose note"],
    ]
    assert len(fake.requests) == 1  # only the hit with a heading path asked for its section


@pytest.mark.asyncio
async def test_an_oversized_section_is_cut_to_the_hit() -> None:
    fake = FakeRagEngine()
    big = "y" * 20000
    fake.add("book", [(big, ["S"]), ("small hit", ["S"]), (big, ["S"])])
    client = RagEngine(transport=fake.transport())

    pack = await build_pack([engine_hit(fake, "book", 1)], 4, client.chunks, LeakGuard())

    assert pack[0]["section"] == ["[c:book:1] small hit"]


def test_engine_chunk_rejects_a_different_chunk_id_shape() -> None:
    fake = section_engine()
    client = RagEngine(transport=fake.transport())

    assert asyncio.run(client.chunk("book:3"))["text"] == "Squeeze theorem statement"
    assert asyncio.run(client.chunk("book:99")) is None
    assert asyncio.run(client.chunk("nodoc:1")) is None
    assert asyncio.run(client.chunk("no-ordinal")) is None


def test_engine_sends_the_bearer_token(monkeypatch) -> None:
    monkeypatch.setenv("RAG_ENGINE_TOKEN", "secret-token")
    seen: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        seen.append(request)
        return httpx.Response(200, json={"chunks": [], "next": None})

    asyncio.run(RagEngine(transport=httpx.MockTransport(handler)).chunks("d", 0, 1))

    assert seen[0].headers["Authorization"] == "Bearer secret-token"

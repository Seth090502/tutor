"""Citation checks at level L0 (C11 /api/library/verify) and the chat post-check text edit."""

from __future__ import annotations

import pytest

from deeptutor.services.library.citations import checked_answer, unverified_note
from deeptutor.services.library.normalize import normalize_quote
from deeptutor.services.library.verify import verify_citations

CHUNKS = {
    "book:3": {
        "chunk_id": "book:3",
        "text": "The “derivative” of f at a—if it exists—is the limit of the differ­ence quotient.",
        "page_start": 12,
        "page_end": 13,
    },
    "note:0": {"chunk_id": "note:0", "text": "A note without pages.", "page_start": None},
}


async def lookup(chunk_id: str):
    return CHUNKS.get(chunk_id)


def test_normalize_quote_handles_whitespace_quotes_dashes_and_soft_hyphens() -> None:
    assert normalize_quote("  ‘a’  “b”\n\tc–d—e−f soft­hyphen ") == "'a' \"b\" c-d-e-f softhyphen"


@pytest.mark.asyncio
async def test_verify_reasons() -> None:
    results = await verify_citations(
        [
            {"chunk_id": "book:3", "quote": 'The "derivative" of f at a-if it exists-is the limit'},
            {"chunk_id": "book:3", "quote": "difference   quotient", "page": 13},
            {"chunk_id": "book:3", "quote": "the limit", "page": 12},
            {"chunk_id": "book:3", "quote": "the integral"},
            {"chunk_id": "book:3", "quote": "THE LIMIT"},
            {"chunk_id": "book:3", "quote": "   "},
            {"chunk_id": "book:3", "quote": "the limit", "page": 14},
            {"chunk_id": "book:3", "quote": "the limit", "page": 11},
            {"chunk_id": "note:0", "quote": "A note", "page": 1},
            {"chunk_id": "note:0", "quote": "A note"},
            {"chunk_id": "book:99", "quote": "the limit"},
            {"chunk_id": "", "quote": "the limit"},
        ],
        lookup,
    )

    assert [r["reason"] for r in results] == [
        None,
        None,
        None,
        "not_substring",
        "not_substring",
        "not_substring",
        "page_out_of_range",
        "page_out_of_range",
        "page_out_of_range",
        None,
        "no_chunk",
        "no_chunk",
    ]
    assert [r["ok"] for r in results] == [r["reason"] is None for r in results]


@pytest.mark.asyncio
async def test_verify_looks_each_chunk_up_once() -> None:
    seen: list[str] = []

    async def counting(chunk_id: str):
        seen.append(chunk_id)
        return CHUNKS.get(chunk_id)

    await verify_citations([{"chunk_id": "book:3", "quote": "limit"}] * 3, counting)

    assert seen == ["book:3"]


# ---------------------------------------------------------------------------
# Chat post-check
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_answer_without_citations_or_with_only_valid_ones_is_kept() -> None:
    assert await checked_answer("No citations here.", lookup) is None
    assert (
        await checked_answer(
            "It is a limit [c:book:3] “the limit of the difference quotient”.", lookup
        )
        is None
    )


@pytest.mark.asyncio
async def test_failed_citations_are_removed_and_the_answer_says_so() -> None:
    text = (
        'The derivative is a limit [c:book:3] "the limit of the difference quotient". '
        'It was invented in 1492 [c:book:3] "invented in 1492". '
        'See also [c:missing:1] "anything" and [c:note:0].'
    )

    out = await checked_answer(text, lookup)

    assert out == (
        'The derivative is a limit [c:book:3] "the limit of the difference quotient". '
        "It was invented in 1492. See also and.\n\n" + unverified_note(3)
    )
    assert "I cannot verify this" in out


@pytest.mark.asyncio
async def test_an_unreachable_library_removes_every_citation() -> None:
    async def down(chunk_id: str):
        raise ConnectionError("rag-engine is not running")

    out = await checked_answer('Fact [c:book:3] "the limit".', down)

    assert out == "Fact.\n\n" + unverified_note(1)
    assert unverified_note(1).endswith("removed it.")

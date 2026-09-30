"""Citation check level L0 (CORPUS.md Stage 0 step 5, C11 ``/api/library/verify``).

A citation passes when its chunk exists, its quote is a substring of the chunk's text
after :func:`normalize_quote` (whitespace, quotes, dashes, soft hyphens; case kept),
and its page, when given, lies in the chunk's page range. An empty quote cites
nothing and fails as ``not_substring``.
"""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from typing import Any

from .normalize import normalize_quote

ChunkLookup = Callable[[str], Awaitable[dict[str, Any] | None]]


def check_citation(chunk: dict[str, Any] | None, quote: str, page: int | None) -> dict[str, Any]:
    if chunk is None:
        return {"ok": False, "reason": "no_chunk"}
    needle = normalize_quote(quote)
    if not needle or needle not in normalize_quote(str(chunk.get("text") or "")):
        return {"ok": False, "reason": "not_substring"}
    if page is not None:
        start, end = chunk.get("page_start"), chunk.get("page_end")
        if start is None or not start <= page <= (end if end is not None else start):
            return {"ok": False, "reason": "page_out_of_range"}
    return {"ok": True, "reason": None}


async def verify_citations(
    citations: list[dict[str, Any]], lookup: ChunkLookup
) -> list[dict[str, Any]]:
    """One result per citation, in order. ``lookup`` errors propagate (nothing verified)."""
    chunks: dict[str, dict[str, Any] | None] = {}
    results = []
    for citation in citations:
        chunk_id = str(citation.get("chunk_id") or "")
        if chunk_id not in chunks:
            chunks[chunk_id] = await lookup(chunk_id) if chunk_id else None
        results.append(
            check_citation(chunks[chunk_id], str(citation.get("quote") or ""), citation.get("page"))
        )
    return results


__all__ = ["check_citation", "verify_citations"]

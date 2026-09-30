"""Evidence packer (CORPUS.md Stage 0 step 3, C11 ``pack``).

From the final ranking it picks 4-12 hits, expands each to its section, and orders the
pack strongest first and last:

* **Quota:** at most ``SOURCE_QUOTA`` hits per document on the first pass; when that
  leaves fewer than ``k`` hits, the overflow fills in, in rank order (a one-book
  library still gets a full pack).
* **Section:** a hit expands to the neighbouring chunks (up to ``SECTION_NEIGHBOURS``
  each side, via ``GET /documents/{id}/chunks``) that share its heading path, stopping
  at a heading change or a chunk the leak guard withholds. A hit with no heading path
  has no known section and stays alone. ``section`` lists the chunk texts in reading
  order, the hit's own included, each prefixed with its ``[c:<chunk_id>]`` tag so every
  quoted sentence can be cited to the chunk it came from.
* **Budget:** tokens are estimated at 4 characters each over the section texts. Hits
  are added in rank order until the pack holds ``k`` hits and at least ``MIN_TOKENS``,
  or 12 hits, or the next hit would pass ``MAX_TOKENS``. A section that would pass the
  budget is cut to the hit alone. ``k`` is clamped to 4-12.
* **Order:** rank 1 first, rank 2 last, rank 3 second, rank 4 second to last, and so
  on, so the weakest hits sit in the middle. ``ranks["final"]`` keeps each hit's rank.
* A hit already inside an earlier hit's section is not repeated.

The question goes last and titles and heading paths go first when a consumer renders
the pack into a prompt; each hit carries ``title`` and ``heading_path`` for that.
"""

from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable
import logging
from typing import Any

from .engine import split_chunk_id
from .leak import LeakGuard

logger = logging.getLogger(__name__)

MIN_HITS = 4
MAX_HITS = 12
MIN_TOKENS = 3000
MAX_TOKENS = 8000
SOURCE_QUOTA = 3
SECTION_NEIGHBOURS = 2
CHARS_PER_TOKEN = 4

ChunkPage = Callable[[str, int, int], Awaitable[list[dict[str, Any]] | None]]


def estimate_tokens(text: str) -> int:
    return -(-len(text) // CHARS_PER_TOKEN)


def tagged(chunk: dict[str, Any]) -> str:
    return f"[c:{chunk.get('chunk_id')}] {chunk.get('text') or ''}"


def quota_order(
    hits: list[dict[str, Any]], k: int, quota: int = SOURCE_QUOTA
) -> list[dict[str, Any]]:
    """Hits within the per-document quota first, topped up to ``k`` from the overflow;
    both in rank order. The remaining overflow follows."""
    counts: dict[str, int] = {}
    within: list[bool] = []
    for hit in hits:
        doc = str(hit.get("doc_id") or hit.get("path") or "")
        counts[doc] = counts.get(doc, 0) + 1
        within.append(counts[doc] <= quota)
    need = max(0, k - sum(within))
    first: list[dict[str, Any]] = []
    rest: list[dict[str, Any]] = []
    for hit, ok in zip(hits, within):
        if ok or need:
            need -= not ok
            first.append(hit)
        else:
            rest.append(hit)
    return first + rest


def edge_order(items: list[Any]) -> list[Any]:
    """Strongest first and last: ranks 1, 3, 5, ... then ..., 6, 4, 2."""
    return items[0::2] + items[1::2][::-1]


def section_of(
    hit: dict[str, Any], page: list[dict[str, Any]], guard: LeakGuard
) -> list[dict[str, Any]]:
    """The run of chunks around ``hit`` in ``page`` that share its heading path."""
    headings = hit.get("heading_path") or []
    at = next((i for i, c in enumerate(page) if c.get("chunk_id") == hit.get("chunk_id")), None)
    if not headings or at is None:
        return [hit]

    def same(chunk: dict[str, Any]) -> bool:
        return (chunk.get("heading_path") or []) == headings and guard.allow(chunk)

    start = at
    while start > 0 and same(page[start - 1]):
        start -= 1
    end = at + 1
    while end < len(page) and same(page[end]):
        end += 1
    return [*page[start:at], hit, *page[at + 1 : end]]


async def fetch_section(
    hit: dict[str, Any], chunk_page: ChunkPage, guard: LeakGuard
) -> list[dict[str, Any]]:
    parts = split_chunk_id(str(hit.get("chunk_id") or ""))
    if parts is None or not hit.get("heading_path"):
        return [hit]
    doc_id, ordinal = parts
    start = max(0, ordinal - SECTION_NEIGHBOURS)
    try:
        page = await chunk_page(doc_id, start, ordinal - start + SECTION_NEIGHBOURS + 1)
    except Exception:
        # Expansion adds context; without it the hit still stands on its own.
        logger.warning("Section expansion failed for %s", hit.get("chunk_id"), exc_info=True)
        return [hit]
    return section_of(hit, page or [], guard)


async def build_pack(
    ranked: list[dict[str, Any]],
    k: int,
    chunk_page: ChunkPage,
    guard: LeakGuard,
) -> list[dict[str, Any]]:
    """The pack for ``ranked`` hits (already filtered by the leak guard)."""
    k = max(MIN_HITS, min(MAX_HITS, k))
    candidates = quota_order(ranked, k)
    chosen: list[dict[str, Any]] = []
    included: set[str] = set()
    tokens = 0

    def full() -> bool:
        return len(chosen) >= MAX_HITS or (len(chosen) >= k and tokens >= MIN_TOKENS)

    # Sections are fetched a batch at a time: hits inside an earlier hit's section are
    # skipped, so the first MAX_HITS candidates may not fill the pack.
    for at in range(0, len(candidates), MAX_HITS):
        batch = candidates[at : at + MAX_HITS]
        sections = await asyncio.gather(*(fetch_section(h, chunk_page, guard) for h in batch))
        for hit, section in zip(batch, sections):
            if full():
                return edge_order(chosen)
            if hit.get("chunk_id") in included:
                continue
            section = [c for c in section if c is hit or c.get("chunk_id") not in included]
            cost = sum(estimate_tokens(tagged(c)) for c in section)
            if tokens + cost > MAX_TOKENS:
                section = [hit]
                cost = estimate_tokens(tagged(hit))
                if tokens + cost > MAX_TOKENS:
                    return edge_order(chosen)
            tokens += cost
            included.update(str(c.get("chunk_id")) for c in section)
            chosen.append(
                {
                    **hit,
                    "ranks": {**(hit.get("ranks") or {}), "final": len(chosen) + 1},
                    "section": [tagged(c) for c in section],
                }
            )
        if full():
            break
    return edge_order(chosen)


__all__ = [
    "MAX_HITS",
    "MAX_TOKENS",
    "MIN_HITS",
    "MIN_TOKENS",
    "SOURCE_QUOTA",
    "build_pack",
    "edge_order",
    "estimate_tokens",
    "quota_order",
    "section_of",
]

"""Which chunks form a Guided Reading section (CORPUS.md Stage 1: 5-15 pages, from heading paths).

A section is the contiguous run of chunks sharing the opened chunk's top two heading-path levels.
A run longer than 15 pages is cut to a 15-page window from the opened chunk; a run shorter than 5
pages grows into its neighbours within the same chapter (top heading level). Documents without
page numbers (notes) are bounded by chunk count instead.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import Any

MIN_PAGES, MAX_PAGES = 5, 15
# About 15 pages of 400-token chunks, and within one C9 chunk page (limit 200).
MAX_CHUNKS = 60

Chunk = dict[str, Any]


def heading_key(chunk: Chunk, levels: int) -> tuple[str, ...]:
    return tuple(chunk.get("heading_path") or [])[:levels]


def page_span(chunks: Sequence[Chunk]) -> int:
    """Pages from the first to the last numbered page; 0 when no chunk has page numbers."""
    starts = [c["page_start"] for c in chunks if c.get("page_start") is not None]
    ends = [c.get("page_end") or c["page_start"] for c in chunks if c.get("page_start") is not None]
    return max(ends) - min(starts) + 1 if starts else 0


def _fits(chunks: Sequence[Chunk], lo: int, hi: int) -> bool:
    return hi - lo <= MAX_CHUNKS and page_span(chunks[lo:hi]) <= MAX_PAGES


def _grow(
    chunks: Sequence[Chunk],
    bounds: tuple[int, int],
    limits: tuple[int, int],
    enough: Callable[[int, int], bool],
) -> tuple[int, int]:
    """Extend ``bounds`` forwards, then backwards, within ``limits`` while it fits and is short."""
    (lo, hi), (floor, ceiling) = bounds, limits
    while hi < ceiling and not enough(lo, hi) and _fits(chunks, lo, hi + 1):
        hi += 1
    while lo > floor and not enough(lo, hi) and _fits(chunks, lo - 1, hi):
        lo -= 1
    return lo, hi


def select_section(chapter: Sequence[Chunk], opened: int) -> tuple[int, int]:
    """``[lo, hi)`` of the section in ``chapter`` (a run sharing the top heading) at ``opened``."""
    key = heading_key(chapter[opened], 2)
    lo, hi = opened, opened + 1
    while lo > 0 and heading_key(chapter[lo - 1], 2) == key:
        lo -= 1
    while hi < len(chapter) and heading_key(chapter[hi], 2) == key:
        hi += 1
    if not _fits(chapter, lo, hi):
        lo, hi = _grow(chapter, (opened, opened + 1), (lo, hi), lambda _lo, _hi: False)
    if 0 < page_span(chapter[lo:hi]) < MIN_PAGES:
        lo, hi = _grow(
            chapter,
            (lo, hi),
            (0, len(chapter)),
            lambda a, b: page_span(chapter[a:b]) >= MIN_PAGES,
        )
    return lo, hi

"""``POST /api/library/search`` (C11): query pipeline, leak guard and evidence packer.

Order of work:

1. Queries: the raw query, plus 3-5 model rewrites when ``library.rewrite`` is on.
2. Each query runs as one rag-engine ``/search`` (``CANDIDATES_PER_QUERY`` hits). Every
   result list passes the leak guard at once, so no later step (rerank, agentic search,
   packer) and no model ever sees a withheld chunk.
3. ``library.agentic``: the model may run up to 4 more searches or open a chunk.
4. RRF over all lists (k 60).
5. ``library.rerank``: one listwise rerank when the fused top is flat or ``depth`` is
   ``"deep"``.
6. The evidence packer.

The raw query must succeed; a failed rewrite query is logged and left out of the fusion.
"""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
import logging
from typing import Any

from .engine import RagEngine, RagEngineError
from .leak import LeakGuard
from .packer import build_pack
from .query import Ask, agentic_search, is_flat, rerank, rewrite_queries, rrf

logger = logging.getLogger(__name__)

CANDIDATES_PER_QUERY = 30
OPEN_CHUNK_CHARS = 2000
LLM_MAX_TOKENS = 1500


@dataclass
class LibraryQuery:
    query: str
    k: int = 8
    domains: list[str] = field(default_factory=list)
    source_types: list[str] = field(default_factory=list)
    depth: str = "normal"


def make_ask(model: str = "") -> Ask:
    """Model calls on DeepTutor's configured LLM; ``model`` picks a smaller model there."""

    async def ask(system: str, prompt: str) -> str:
        from deeptutor.services.llm import complete

        return await complete(
            prompt,
            system_prompt=system,
            model=model or None,
            temperature=0.0,
            max_tokens=LLM_MAX_TOKENS,
        )

    return ask


async def library_search(
    request: LibraryQuery,
    *,
    settings: dict[str, Any],
    guard: LeakGuard,
    engine: RagEngine,
    ask: Ask,
) -> dict[str, Any]:
    base: dict[str, Any] = {"k": CANDIDATES_PER_QUERY, "mode": "hybrid"}
    if request.domains:
        base["domains"] = request.domains
    if request.source_types:
        base["source_types"] = request.source_types

    queries = (
        await rewrite_queries(ask, request.query) if settings.get("rewrite") else [request.query]
    )
    responses = await asyncio.gather(
        *(engine.search({**base, "query": q}) for q in queries), return_exceptions=True
    )
    if isinstance(responses[0], BaseException):
        raise responses[0]
    mode_used = str(responses[0].get("mode_used") or "")
    ran: list[str] = []
    lists: list[list[dict[str, Any]]] = []
    for query, response in zip(queries, responses):
        if isinstance(response, BaseException):
            logger.warning("Library search for rewrite %r failed: %s", query, response)
            continue
        ran.append(query)
        lists.append([hit for hit in response["hits"] if guard.allow(hit)])

    if settings.get("agentic"):

        async def search_more(query: str) -> list[dict[str, Any]] | None:
            try:
                response = await engine.search({**base, "query": query})
            except RagEngineError as exc:
                logger.warning("Library agentic search %r failed: %s", query, exc)
                return None
            return [hit for hit in response["hits"] if guard.allow(hit)]

        async def open_chunk(chunk_id: str) -> str:
            try:
                chunk = await engine.chunk(chunk_id)
            except RagEngineError as exc:
                return f"[{chunk_id}] could not be opened: {exc}"
            if chunk is None:
                return f"[{chunk_id}] does not exist."
            if not guard.allow(chunk):
                return f"[{chunk_id}] is withheld while the learner's attempt is open."
            return f"[{chunk_id}] {str(chunk.get('text') or '')[:OPEN_CHUNK_CHARS]}"

        await agentic_search(ask, request.query, ran, lists, search_more, open_chunk)

    fused = rrf(lists)
    if (
        settings.get("rerank")
        and fused
        and (request.depth == "deep" or is_flat([f.score for f in fused], len(lists)))
    ):
        fused = await rerank(ask, request.query, fused) or fused

    pack = await build_pack([f.hit for f in fused], request.k, engine.chunks, guard)
    return {"pack": pack, "queries": ran, "mode_used": mode_used, "withheld": guard.withheld}


__all__ = ["LibraryQuery", "library_search", "make_ask"]

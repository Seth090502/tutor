"""rag-engine calls for library use, on the tutor_library adapter's client and token.

``POST /search`` goes through :class:`TutorLibraryPipeline` (its URL validation, bearer
token from ``RAG_ENGINE_TOKEN`` and error messages); ``GET /documents/{id}/chunks``
(C9) uses the same URL, token and error handling.

Chunk ids: rag-engine writes ``"<doc_id>:<ordinal>"`` (``ingest.rs`` ``chunk_id``).
C1 hits carry no ordinal, so neighbours and single chunks are located by parsing
the id; :meth:`RagEngine.chunk` checks that the chunk returned has the requested id,
so an id of another shape reads as missing instead of as the wrong chunk.
"""

from __future__ import annotations

from typing import Any
from urllib.parse import quote

import httpx

from deeptutor.services.rag.pipelines.tutor_library.pipeline import (
    TIMEOUT_SECONDS,
    TutorLibraryPipeline,
    _auth_headers,
    _error_detail,
    _rag_engine_url,
    _RagEngineError,
)

RagEngineError = _RagEngineError


def split_chunk_id(chunk_id: str) -> tuple[str, int] | None:
    """``(doc_id, ordinal)`` from a rag-engine chunk id, or None when it has another shape."""
    doc_id, sep, ordinal = str(chunk_id).rpartition(":")
    if not sep or not doc_id or not ordinal.isdigit():
        return None
    return doc_id, int(ordinal)


class RagEngine:
    """The two rag-engine reads library use needs."""

    def __init__(self, *, transport: httpx.AsyncBaseTransport | None = None) -> None:
        # Test seam: an httpx transport standing in for rag-engine.
        self._transport = transport
        self._adapter = TutorLibraryPipeline(kb_base_dir="", transport=transport)

    async def search(self, payload: dict[str, Any]) -> dict[str, Any]:
        """C1 ``POST /search``; raises :class:`RagEngineError`."""
        return await self._adapter._post_search(_rag_engine_url(), payload)

    async def chunks(self, doc_id: str, start: int, limit: int) -> list[dict[str, Any]] | None:
        """C9 chunk page in reading order, or None when the document does not exist."""
        base_url = _rag_engine_url()
        url = f"{base_url}/documents/{quote(doc_id, safe='')}/chunks"
        try:
            async with httpx.AsyncClient(
                timeout=TIMEOUT_SECONDS, transport=self._transport
            ) as client:
                response = await client.get(
                    url, params={"from": start, "limit": limit}, headers=_auth_headers()
                )
        except httpx.TimeoutException as exc:
            raise RagEngineError(
                f"Tutor library (rag-engine) did not answer within {TIMEOUT_SECONDS:g} s "
                f"at {base_url}."
            ) from exc
        except httpx.HTTPError as exc:
            raise RagEngineError(
                f"Tutor library (rag-engine) request to {base_url} failed: {exc}"
            ) from exc
        if response.status_code == 404:
            return None
        if response.status_code == 401:
            raise RagEngineError(
                f"Tutor library (rag-engine) at {base_url} rejected the request; "
                "check RAG_ENGINE_TOKEN.",
                "not_configured",
            )
        if response.status_code >= 400:
            raise RagEngineError(
                f"Tutor library (rag-engine) chunk read failed (HTTP {response.status_code}): "
                f"{_error_detail(response)}"
            )
        try:
            data = response.json()
        except ValueError as exc:
            raise RagEngineError(
                "Tutor library (rag-engine) returned a non-JSON response."
            ) from exc
        chunks = data.get("chunks") if isinstance(data, dict) else None
        if not isinstance(chunks, list) or not all(isinstance(c, dict) for c in chunks):
            raise RagEngineError("Tutor library (rag-engine) returned an unexpected chunk page.")
        return chunks

    async def chunk(self, chunk_id: str) -> dict[str, Any] | None:
        """One chunk by id, or None when it does not exist."""
        parts = split_chunk_id(chunk_id)
        if parts is None:
            return None
        page = await self.chunks(parts[0], parts[1], 1)
        if not page or page[0].get("chunk_id") != chunk_id:
            return None
        return page[0]


__all__ = ["RagEngine", "RagEngineError", "split_chunk_id"]

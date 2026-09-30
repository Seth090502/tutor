"""rag-engine's library routes (contract C9) for Guided Reading: a document and its chunks.

Uses the tutor library's engine URL and bearer token (``RAG_ENGINE_URL``, ``RAG_ENGINE_TOKEN``), so
a remote engine on the second desktop works the same way as the local one.
"""

from __future__ import annotations

from collections.abc import AsyncIterator
from typing import Any
from urllib.parse import quote

import httpx

from deeptutor.services.rag.pipelines.tutor_library.pipeline import (
    TIMEOUT_SECONDS,
    _auth_headers,
    _error_detail,
    _rag_engine_url,
    _RagEngineError,
)

# Chunks per request while scanning a document; C9 allows up to 200.
PAGE_SIZE = 200


class EngineError(Exception):
    """rag-engine could not answer; ``status`` is the HTTP status to report."""

    def __init__(self, message: str, status: int = 503) -> None:
        super().__init__(message)
        self.status = status


class LibraryEngine:
    def __init__(self, *, transport: httpx.AsyncBaseTransport | None = None) -> None:
        # Test seam: an httpx transport standing in for rag-engine.
        self._transport = transport

    async def _get(self, path: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        try:
            base_url = _rag_engine_url()
        except _RagEngineError as exc:
            raise EngineError(str(exc)) from exc
        try:
            async with httpx.AsyncClient(
                timeout=TIMEOUT_SECONDS, transport=self._transport
            ) as client:
                response = await client.get(
                    f"{base_url}{path}", params=params, headers=_auth_headers()
                )
        except httpx.HTTPError as exc:
            raise EngineError(
                f"The tutor library (rag-engine) is not reachable at {base_url}: {exc}"
            ) from exc
        if response.status_code == 404:
            raise EngineError(f"Not in the library: {_error_detail(response)}", 404)
        if response.status_code >= 400:
            raise EngineError(
                f"The tutor library (rag-engine) failed (HTTP {response.status_code}): "
                f"{_error_detail(response)}"
            )
        try:
            data = response.json()
        except ValueError as exc:
            raise EngineError(
                "The tutor library (rag-engine) returned a non-JSON response."
            ) from exc
        if not isinstance(data, dict):
            raise EngineError("The tutor library (rag-engine) returned an unexpected response.")
        return data

    async def document(self, doc_id: str) -> dict[str, Any]:
        return await self._get(f"/documents/{quote(doc_id, safe='')}")

    async def chunks(self, doc_id: str, start: int, limit: int) -> dict[str, Any]:
        data = await self._get(
            f"/documents/{quote(doc_id, safe='')}/chunks", {"from": start, "limit": limit}
        )
        chunks = data.get("chunks")
        if not isinstance(chunks, list) or not all(isinstance(c, dict) for c in chunks):
            raise EngineError("The tutor library (rag-engine) returned an unexpected chunk page.")
        return data

    async def scan(self, doc_id: str) -> AsyncIterator[dict[str, Any]]:
        """Every chunk of the document in reading order, one page at a time."""
        start: int | None = 0
        while start is not None:
            page = await self.chunks(doc_id, start, PAGE_SIZE)
            for chunk in page["chunks"]:
                yield chunk
            start = page.get("next")

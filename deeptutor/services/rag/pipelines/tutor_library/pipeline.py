"""Retrieval-only RAG pipeline over the tutor library served by rag-engine.

The owner's books, papers and notes (50-250 GB) are ingested and indexed by the
separate rag-engine service (``docs/tutor/CONTRACTS.md``, C1), not by DeepTutor.
A ``tutor_library`` KB is a pointer: :meth:`TutorLibraryPipeline.search` calls
``POST {RAG_ENGINE_URL}/search`` in hybrid mode (with ``Authorization: Bearer
{RAG_ENGINE_TOKEN}`` when that is set, contract C9) and maps the hits into the
``{title, content, source, page, chunk_id, score}`` sources the ``rag`` tool
cites. Optional ``domains`` / ``source_types`` filters come from the search call,
else from the KB entry written at connect time. Nothing is copied or indexed
locally, and deleting the KB only drops DeepTutor's pointer.
"""

from __future__ import annotations

import logging
import os
from pathlib import PurePosixPath
from typing import Any, Sequence
from urllib.parse import urlparse

import httpx

from deeptutor.runtime.home import get_runtime_data_root
from deeptutor.services.rag.provider_binding import load_kb_config_entry

logger = logging.getLogger(__name__)

PROVIDER = "tutor_library"
RAG_ENGINE_URL_ENV = "RAG_ENGINE_URL"
RAG_ENGINE_TOKEN_ENV = "RAG_ENGINE_TOKEN"
DEFAULT_RAG_ENGINE_URL = "http://127.0.0.1:8811"
TIMEOUT_SECONDS = 10.0
DEFAULT_TOP_K = 10
MAX_TOP_K = 50
SOURCE_PREVIEW_CHARS = 200
SOURCE_TYPES = ("book", "paper", "note", "unknown")
LIBRARY_MANAGED_MESSAGE = (
    "The tutor library is indexed by rag-engine, not by DeepTutor. "
    "Add books, papers and notes from the desktop app's Library screen."
)


class _RagEngineError(Exception):
    def __init__(self, message: str, error_type: str = "retrieval_error") -> None:
        super().__init__(message)
        self.error_type = error_type


def _rag_engine_url() -> str:
    """The rag-engine base URL from ``RAG_ENGINE_URL``, validated."""
    url = (os.environ.get(RAG_ENGINE_URL_ENV) or "").strip().rstrip("/") or DEFAULT_RAG_ENGINE_URL
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise _RagEngineError(
            f"{RAG_ENGINE_URL_ENV} must be an http(s) URL such as {DEFAULT_RAG_ENGINE_URL}; "
            f"got {url!r}.",
            "not_configured",
        )
    return url


def _auth_headers() -> dict[str, str]:
    """The bearer token header from ``RAG_ENGINE_TOKEN``, or none when it is unset."""
    token = (os.environ.get(RAG_ENGINE_TOKEN_ENV) or "").strip()
    return {"Authorization": f"Bearer {token}"} if token else {}


def _string_list(values: Any) -> list[str]:
    if isinstance(values, str):
        values = [values]
    if not isinstance(values, (list, tuple)):
        return []
    cleaned: list[str] = []
    for value in values:
        text = str(value).strip()
        if text and text not in cleaned:
            cleaned.append(text)
    return cleaned


def clean_filters(
    domains: Sequence[str] = (), source_types: Sequence[str] = ()
) -> tuple[list[str], list[str]]:
    """Trim and de-duplicate connect-time filters; reject unknown source types."""
    domain_list = _string_list(domains)
    type_list = _string_list(source_types)
    unknown = [value for value in type_list if value not in SOURCE_TYPES]
    if unknown:
        raise ValueError(
            f"Unknown source type(s): {', '.join(unknown)}. Use {', '.join(SOURCE_TYPES)}."
        )
    return domain_list, type_list


def _top_k(raw: Any) -> int:
    try:
        requested = int(raw) if raw else DEFAULT_TOP_K
    except (TypeError, ValueError):
        requested = DEFAULT_TOP_K
    return max(1, min(MAX_TOP_K, requested))


def _page_label(page_start: Any, page_end: Any) -> str:
    if page_start is None:
        return ""
    if page_end is None or page_end == page_start:
        return str(page_start)
    return f"{page_start}-{page_end}"


def _source(hit: dict[str, Any]) -> dict[str, Any]:
    path = str(hit.get("path") or "")
    heading_path = hit.get("heading_path")
    return {
        "title": str(hit.get("title") or "") or PurePosixPath(path).name or "Untitled",
        "content": str(hit.get("text") or "")[:SOURCE_PREVIEW_CHARS],
        "source": path,
        "page": _page_label(hit.get("page_start"), hit.get("page_end")),
        "chunk_id": str(hit.get("chunk_id") or ""),
        "score": hit.get("score"),
        "doc_id": str(hit.get("doc_id") or ""),
        "path": path,
        "domain": hit.get("domain"),
        "source_type": hit.get("source_type"),
        "page_start": hit.get("page_start"),
        "page_end": hit.get("page_end"),
        "heading_path": [str(part) for part in heading_path]
        if isinstance(heading_path, list)
        else [],
    }


def _citation(index: int, source: dict[str, Any]) -> str:
    header = f"[{index}] {source['title']}"
    if source["page"]:
        header += f", p. {source['page']}"
    if source["heading_path"]:
        header += ", " + " > ".join(source["heading_path"])
    if source["path"]:
        header += f" ({source['path']})"
    return header


def _error_detail(response: httpx.Response) -> str:
    try:
        body = response.json()
    except ValueError:
        return response.text[:300]
    if isinstance(body, dict) and body.get("error"):
        return str(body["error"])[:300]
    return response.text[:300]


class TutorLibraryPipeline:
    """Query rag-engine on behalf of a connected ``tutor_library`` KB."""

    def __init__(
        self,
        kb_base_dir: str | None = None,
        *,
        transport: httpx.AsyncBaseTransport | None = None,
        **_: Any,
    ) -> None:
        self.kb_base_dir = kb_base_dir or str(get_runtime_data_root() / "knowledge_bases")
        # Test seam: an httpx transport standing in for rag-engine.
        self._transport = transport

    async def search(self, query: str, kb_name: str, **kwargs: Any) -> dict[str, Any]:
        entry = load_kb_config_entry(self.kb_base_dir, kb_name)
        payload: dict[str, Any] = {
            "query": query,
            "k": _top_k(kwargs.get("top_k")),
            "mode": "hybrid",
        }
        for field in ("domains", "source_types"):
            values = _string_list(kwargs.get(field)) or _string_list(entry.get(field))
            if values:
                payload[field] = values

        try:
            data = await self._post_search(_rag_engine_url(), payload)
        except _RagEngineError as exc:
            logger.warning("Tutor library search failed for '%s': %s", kb_name, exc)
            return self._error(query, str(exc), exc.error_type)

        sources = [_source(hit) for hit in data["hits"]]
        context = "\n\n".join(
            f"{_citation(index, source)}\n{hit.get('text') or ''}"
            for index, (source, hit) in enumerate(zip(sources, data["hits"]), start=1)
        )
        return {
            "query": query,
            "answer": context,
            "content": context,
            "sources": sources,
            "provider": PROVIDER,
            "mode": data.get("mode_used"),
        }

    async def _post_search(self, base_url: str, payload: dict[str, Any]) -> dict[str, Any]:
        try:
            async with httpx.AsyncClient(
                timeout=TIMEOUT_SECONDS, transport=self._transport
            ) as client:
                response = await client.post(
                    f"{base_url}/search", json=payload, headers=_auth_headers()
                )
        except httpx.TimeoutException as exc:
            raise _RagEngineError(
                f"Tutor library (rag-engine) did not answer within {TIMEOUT_SECONDS:g} s "
                f"at {base_url}."
            ) from exc
        except httpx.ConnectError as exc:
            raise _RagEngineError(
                f"Tutor library (rag-engine) is not running at {base_url}. "
                "Start it from the desktop app, then try again."
            ) from exc
        except httpx.HTTPError as exc:
            raise _RagEngineError(
                f"Tutor library (rag-engine) request to {base_url} failed: {exc}"
            ) from exc

        if response.status_code == 401:
            if _auth_headers():
                reason = (
                    f"rejected {RAG_ENGINE_TOKEN_ENV}; check that it matches the engine's token"
                )
            else:
                reason = f"requires a token; set {RAG_ENGINE_TOKEN_ENV} to the engine's token"
            raise _RagEngineError(
                f"Tutor library (rag-engine) at {base_url} {reason}.", "not_configured"
            )
        if response.status_code >= 400:
            raise _RagEngineError(
                f"Tutor library (rag-engine) search failed (HTTP {response.status_code}): "
                f"{_error_detail(response)}"
            )
        try:
            data = response.json()
        except ValueError as exc:
            raise _RagEngineError(
                "Tutor library (rag-engine) returned a non-JSON response."
            ) from exc
        hits = data.get("hits") if isinstance(data, dict) else None
        if not isinstance(hits, list) or not all(isinstance(hit, dict) for hit in hits):
            raise _RagEngineError("Tutor library (rag-engine) returned an unexpected response.")
        return data

    @staticmethod
    def _error(query: str, message: str, error_type: str) -> dict[str, Any]:
        return {
            "query": query,
            "answer": message,
            "content": "",
            "sources": [],
            "provider": PROVIDER,
            "error_type": error_type,
        }

    async def initialize(self, kb_name: str, file_paths: list[str], **kwargs: Any) -> bool:
        raise RuntimeError(LIBRARY_MANAGED_MESSAGE)

    async def add_documents(self, kb_name: str, file_paths: list[str], **kwargs: Any) -> bool:
        raise RuntimeError(LIBRARY_MANAGED_MESSAGE)

    async def delete(self, kb_name: str, **kwargs: Any) -> bool:
        # The KB is only a pointer; never touch the corpus or rag-engine's index.
        return True


__all__ = ["PROVIDER", "TutorLibraryPipeline", "clean_filters"]

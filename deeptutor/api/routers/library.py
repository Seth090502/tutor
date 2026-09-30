"""Library use (contract C11): search with the query pipeline and packer, and citation checks.

Spec: ``docs/tutor/CORPUS.md`` Stage 0. The pipeline lives in
:mod:`deeptutor.services.library`; this module only validates and maps errors.
``/settings`` reads and changes the ``library.*`` settings, so the eval harness can
run each retrieval arm with its own settings.
"""

from __future__ import annotations

import asyncio
from typing import Any, Literal

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from deeptutor.services.library.engine import RagEngine, RagEngineError
from deeptutor.services.library.leak import AttemptNotFound, attempt_guard
from deeptutor.services.library.packer import MAX_HITS
from deeptutor.services.library.search import LibraryQuery, library_search, make_ask
from deeptutor.services.library.settings import load_library_settings, save_library_settings
from deeptutor.services.library.verify import verify_citations
from deeptutor.services.rag.pipelines.tutor_library.pipeline import clean_filters
from deeptutor.services.session import get_sqlite_session_store

router = APIRouter()

MAX_CITATIONS = 200


class LibrarySearchRequest(BaseModel):
    query: str = Field(min_length=1, max_length=4000)
    k: int = Field(default=8, ge=1, le=MAX_HITS)
    domains: list[str] | None = None
    source_types: list[str] | None = None
    attempt_id: str | None = None
    depth: Literal["normal", "deep"] = "normal"


class Citation(BaseModel):
    chunk_id: str
    quote: str
    page: int | None = None


class VerifyRequest(BaseModel):
    citations: list[Citation] = Field(max_length=MAX_CITATIONS)


class LibrarySettingsUpdate(BaseModel):
    rewrite: bool | None = None
    rerank: bool | None = None
    agentic: bool | None = None
    model: str | None = Field(default=None, max_length=200)


def _engine_error(exc: RagEngineError) -> HTTPException:
    return HTTPException(status_code=502, detail=str(exc))


def _engine() -> RagEngine:
    return RagEngine()


@router.post("/search")
async def search_library_route(payload: LibrarySearchRequest) -> dict[str, Any]:
    query = payload.query.strip()
    if not query:
        raise HTTPException(status_code=422, detail="query must not be blank")
    try:
        domains, source_types = clean_filters(payload.domains or (), payload.source_types or ())
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    attempt_id = (payload.attempt_id or "").strip() or None
    try:
        guard = await asyncio.to_thread(
            attempt_guard, get_sqlite_session_store().db_path, attempt_id
        )
    except AttemptNotFound as exc:
        raise HTTPException(status_code=404, detail="Attempt not found") from exc
    settings = load_library_settings()
    try:
        return await library_search(
            LibraryQuery(
                query=query,
                k=payload.k,
                domains=domains,
                source_types=source_types,
                depth=payload.depth,
            ),
            settings=settings,
            guard=guard,
            engine=_engine(),
            ask=make_ask(settings["model"]),
        )
    except RagEngineError as exc:
        raise _engine_error(exc) from exc


@router.post("/verify")
async def verify_library_citations_route(payload: VerifyRequest) -> dict[str, Any]:
    try:
        results = await verify_citations(
            [citation.model_dump() for citation in payload.citations], _engine().chunk
        )
    except RagEngineError as exc:
        raise _engine_error(exc) from exc
    return {"results": results}


@router.get("/settings")
async def get_library_settings_route() -> dict[str, Any]:
    return load_library_settings()


@router.put("/settings")
async def update_library_settings_route(payload: LibrarySettingsUpdate) -> dict[str, Any]:
    return save_library_settings(payload.model_dump(exclude_none=True))

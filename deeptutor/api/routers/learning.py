"""Study commands, sessions and the public queue (contract C12), mounted at ``/api/study``.

Not ``/api/learning``: upstream retired that prefix and guards it (test_canonical_route_surface).
"""

from __future__ import annotations

import asyncio
from typing import Any

from fastapi import APIRouter, Body, Header, Path, Query
from fastapi.responses import JSONResponse

from deeptutor.services.practice import feed as study_feed
from deeptutor.services.practice.loop import CommandError, LearningLoop
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session import get_sqlite_session_store

router = APIRouter()

_STATUS = {
    "conflict": 409,
    "invalid_transition": 409,
    "invalid_input": 422,
    "not_found": 404,
    "unauthorized": 403,
    "budget_exceeded": 402,
    "unavailable": 503,
    "unverified": 424,
}
SESSION_ID = r"^[0-9a-f]{32}$"


def _loop() -> LearningLoop:
    return LearningLoop(PracticeStore(get_sqlite_session_store().db_path))


def _error(exc: CommandError) -> JSONResponse:
    return JSONResponse({"error": exc.body()}, status_code=_STATUS.get(exc.code, 400))


@router.post("/commands")
async def command(
    body: Any = Body(...),
    x_tutor_source: str | None = Header(default=None),
):
    # The source comes from the calling code path (typed controls or the voice tools), never from a
    # model-written field, and it limits what the call may do (a voice tool cannot commit an answer).
    if x_tutor_source not in ("ui", "voice_tool"):
        return _error(CommandError("invalid_input", "X-Tutor-Source must be ui or voice_tool"))
    try:
        return await asyncio.to_thread(_loop().execute, body, x_tutor_source)
    except CommandError as exc:
        return _error(exc)


@router.get("/sessions/active")
async def active_session():
    return {"session": await asyncio.to_thread(_loop().active_session)}


@router.get("/sessions/{session_id}")
async def session(
    session_id: str = Path(pattern=SESSION_ID), after_seq: int = Query(default=0, ge=0)
):
    try:
        return await asyncio.to_thread(_loop().session, session_id, after_seq)
    except CommandError as exc:
        return _error(exc)


@router.get("/queue")
async def queue(
    limit: int = Query(default=20, ge=1, le=100),
    tag: str | None = Query(default=None, max_length=100),
):
    return {"items": await asyncio.to_thread(_loop().queue, limit, tag)}


@router.get("/feed")
async def feed(
    after_event_seq: int = Query(default=0, ge=0), limit: int = Query(default=500, ge=1, le=1000)
):
    return await asyncio.to_thread(study_feed.feed, _loop().store, after_event_seq, limit)


@router.get("/candidates")
async def candidates():
    return await asyncio.to_thread(study_feed.candidates, _loop().store)

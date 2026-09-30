"""Guided Reading routes (contract C11) under ``/api/reading``, beside the immersive reader.

A thin adapter over :mod:`deeptutor.services.reading.service`: it validates inputs and maps
errors to status codes. Sessions live in the current user's notebook database.
"""

from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException, Path
from pydantic import BaseModel, Field

from deeptutor.services.llm.exceptions import LLMError
from deeptutor.services.reading.engine import EngineError, LibraryEngine
from deeptutor.services.reading.service import (
    PreparationFailed,
    ReadingNotFound,
    ReadingService,
)
from deeptutor.services.reading.storage import ReadingConflict, ReadingStore
from deeptutor.services.session import get_sqlite_session_store

router = APIRouter()

SessionId = Annotated[str, Path(pattern=r"^[a-f0-9]{32}$")]
Id = Annotated[str, Field(min_length=1, max_length=512)]
Text = Annotated[str, Field(min_length=1, max_length=20000)]
Level = Annotated[int, Field(ge=1, le=4)]


def get_reading_service() -> ReadingService:
    return ReadingService(ReadingStore(get_sqlite_session_store().db_path), LibraryEngine())


Service = Annotated[ReadingService, Depends(get_reading_service)]


class OpenRequest(BaseModel):
    doc_id: Id
    from_chunk: Id


class Section(BaseModel):
    heading_path: list[str]
    chunk_ids: list[str]
    page_start: int | None
    page_end: int | None


class Opened(BaseModel):
    session_id: str
    section: Section


class Prequestion(BaseModel):
    id: str
    q: str


class Atom(BaseModel):
    id: str
    front: str
    back: str
    chunk_id: str


class Warrant(BaseModel):
    id: str
    step: str
    chunk_id: str


class Prepared(BaseModel):
    prequestions: list[Prequestion]
    idea_units: int
    atoms: list[Atom]
    warrants: list[Warrant]


class AnswerRequest(BaseModel):
    answer: str = Field(max_length=2000)
    confidence: Level


class Answered(BaseModel):
    correct: bool | None


class RecallRequest(BaseModel):
    text: Text


class SummaryRequest(BaseModel):
    text: Text
    judgement: Level


class MissedUnit(BaseModel):
    unit: str
    chunk_id: str


class Scored(BaseModel):
    covered: int
    missed: list[MissedUnit]


class AcceptRequest(BaseModel):
    ids: list[Annotated[str, Field(min_length=1, max_length=16)]] = Field(
        min_length=1, max_length=5
    )


class Accepted(BaseModel):
    entry_ids: list[int]


async def _run(call):
    try:
        return await call
    except ReadingNotFound as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    except ReadingConflict as exc:
        raise HTTPException(status_code=409, detail=str(exc)) from exc
    except (PreparationFailed, LLMError) as exc:
        raise HTTPException(status_code=502, detail=str(exc)) from exc
    except EngineError as exc:
        raise HTTPException(status_code=exc.status, detail=str(exc)) from exc


@router.post("/sessions", response_model=Opened)
async def open_session(payload: OpenRequest, service: Service):
    return await _run(service.open(payload.doc_id, payload.from_chunk))


@router.post("/sessions/{session_id}/prepare", response_model=Prepared)
async def prepare(session_id: SessionId, service: Service):
    return await _run(service.prepare(session_id))


@router.post("/sessions/{session_id}/prequestions/{qid}/answer", response_model=Answered)
async def answer_prequestion(
    payload: AnswerRequest,
    session_id: SessionId,
    service: Service,
    qid: str = Path(pattern=r"^q[0-9]{1,2}$"),
):
    return await _run(
        service.answer_prequestion(session_id, qid, payload.answer, payload.confidence)
    )


@router.post("/sessions/{session_id}/recall")
async def recall(payload: RecallRequest, session_id: SessionId, service: Service):
    return await _run(service.recall(session_id, payload.text))


@router.post("/sessions/{session_id}/summary", response_model=Scored)
async def summary(payload: SummaryRequest, session_id: SessionId, service: Service):
    return await _run(service.summary(session_id, payload.text, payload.judgement))


@router.post("/sessions/{session_id}/atoms/accept", response_model=Accepted)
async def accept_atoms(payload: AcceptRequest, session_id: SessionId, service: Service):
    return await _run(service.accept_atoms(session_id, payload.ids))

"""Languages (contract C10): coverage, graded texts and language cards."""

from __future__ import annotations

import asyncio
from typing import Literal

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from deeptutor.services.languages.cards import add_cards
from deeptutor.services.languages.coverage import coverage as measure_coverage
from deeptutor.services.languages.coverage import known_forms
from deeptutor.services.languages.graded import GradedTextError, graded_text
from deeptutor.services.languages.text import LanguageSupportMissing
from deeptutor.services.llm import LLMError
from deeptutor.services.session import get_sqlite_session_store

router = APIRouter()

TargetLanguage = Literal["es", "fr", "de", "ja", "zh"]
Band = Literal["A1", "A2", "B1", "B2", "C1", "C2"]


class CoverageRequest(BaseModel):
    lang: TargetLanguage
    text: str = Field(min_length=1, max_length=50000)


class CoverageResult(BaseModel):
    tokens: int
    known: int
    coverage: float
    unknown: list[str]


class GradedTextRequest(BaseModel):
    lang: TargetLanguage
    band: Band
    topic: str | None = Field(default=None, max_length=200)
    target_coverage: float = Field(default=0.97, gt=0, le=1)


class ComprehensionQuestion(BaseModel):
    q: str
    a: str


class GradedText(BaseModel):
    text: str
    sentences: list[str]
    coverage: float
    unknown: list[str]
    questions: list[ComprehensionQuestion]


class CardItem(BaseModel):
    text: str = Field(min_length=1, max_length=500)
    meaning: str = Field(min_length=1, max_length=500)
    example: str | None = Field(default=None, max_length=2000)


class CardsRequest(BaseModel):
    lang: TargetLanguage
    # Three cards per item stay inside one practice import (500 rows).
    items: list[CardItem] = Field(min_length=1, max_length=100)


class CardsResult(BaseModel):
    created: int
    entry_ids: list[int]


def _db_path():
    return get_sqlite_session_store().db_path


@router.post("/coverage", response_model=CoverageResult)
async def coverage(payload: CoverageRequest):
    def run():
        return measure_coverage(payload.lang, payload.text, known_forms(_db_path(), payload.lang))

    try:
        return await asyncio.to_thread(run)
    except LanguageSupportMissing as exc:
        raise HTTPException(503, str(exc)) from exc


@router.post("/graded-text", response_model=GradedText)
async def graded(payload: GradedTextRequest):
    try:
        return await graded_text(
            _db_path(), payload.lang, payload.band, payload.topic, payload.target_coverage
        )
    except LanguageSupportMissing as exc:
        raise HTTPException(503, str(exc)) from exc
    except (GradedTextError, LLMError) as exc:
        raise HTTPException(502, str(exc)) from exc


@router.post("/cards", response_model=CardsResult)
async def cards(payload: CardsRequest):
    items = [item.model_dump() for item in payload.items]
    try:
        return await asyncio.to_thread(add_cards, _db_path(), payload.lang, items)
    except ValueError as exc:
        raise HTTPException(422, str(exc)) from exc

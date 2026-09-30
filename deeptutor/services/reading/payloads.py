"""Prompts and strict validation for the two Guided Reading model calls.

Preparation (one call per section) returns prequestions, idea units, candidate atoms and, for a
mathematical section, proof warrants, each tied to a chunk id of the section. Scoring (two
independent passes per summary) returns the idea units a learner summary covers. A reply that does
not match its schema exactly is invalid; items citing a chunk outside the section are dropped.
"""

from __future__ import annotations

import json
import re
from typing import Annotated, Any, Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError, model_validator

Text = Annotated[str, Field(min_length=1, max_length=4000)]
MIN_PREQUESTIONS, MAX_PREQUESTIONS = 1, 3
MIN_IDEA_UNITS, MAX_IDEA_UNITS = 4, 10
MAX_ATOMS = 5
MAX_WARRANTS = 10
# Atom kinds whose cards stay provisional (CORPUS.md: proofs and essays do not feed FSRS or mastery).
PROVISIONAL_KINDS = frozenset({"proof"})


class InvalidReply(ValueError):
    """A model reply that does not match its schema."""


class _Strict(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)


class Prequestion(_Strict):
    q: Text
    options: dict[Annotated[str, Field(pattern=r"^[A-F]$")], Text] | None
    answer: Text
    explanation: str
    chunk_id: Text

    @model_validator(mode="after")
    def _answer_is_an_option(self) -> Prequestion:
        if self.options is not None and (
            not 2 <= len(self.options) <= 6 or self.answer not in self.options
        ):
            raise ValueError("a choice question needs 2-6 options and its answer letter")
        return self


class IdeaUnit(_Strict):
    text: Text
    chunk_id: Text


class Atom(_Strict):
    front: Text
    back: Text
    kind: Literal["definition", "fact", "notation", "result", "proof"]
    chunk_id: Text


class Warrant(_Strict):
    step: Text
    warrant: Text
    chunk_id: Text


class Preparation(_Strict):
    mathematical: bool
    prequestions: list[Prequestion] = Field(max_length=MAX_PREQUESTIONS)
    idea_units: list[IdeaUnit] = Field(max_length=MAX_IDEA_UNITS)
    atoms: list[Atom] = Field(max_length=MAX_ATOMS)
    warrants: list[Warrant] = Field(max_length=MAX_WARRANTS)


PREPARE_SYSTEM = (
    "You prepare a guided reading of one section of a textbook or paper for a single advanced "
    "learner. Reply with one JSON object and nothing else."
)

PREPARE_PROMPT = """The section follows. Each chunk starts with its id as [c:<id>].

{section}

Return exactly this JSON shape:
{{"mathematical": bool,
 "prequestions": [{{"q": str, "options": {{"A": str, "B": str, ...}} or null, "answer": str,
                   "explanation": str, "chunk_id": str}}],
 "idea_units": [{{"text": str, "chunk_id": str}}],
 "atoms": [{{"front": str, "back": str, "kind": "definition"|"fact"|"notation"|"result"|"proof",
            "chunk_id": str}}],
 "warrants": [{{"step": str, "warrant": str, "chunk_id": str}}]}}

Rules:
- prequestions: 1-3 questions the learner guesses before reading, about the section's central
  ideas. With options, "answer" is the correct letter (2-6 options, letters A-F); with null
  options, "answer" is a short model answer. "explanation" says why, from the text.
- idea_units: 4-10 of the section's main ideas, one sentence each, in reading order.
- atoms: at most 5 flashcards for facts, definitions, notation or named results worth
  remembering; "proof" only for a card that asks for a proof.
- mathematical: true when the section proves or derives results. Then "warrants" lists the key
  proof steps with the reason each step holds; otherwise "warrants" is [].
- Every "chunk_id" is the id of the chunk the item comes from, copied exactly from the section.
- Use only the section's content. No other keys."""

SCORE_SYSTEM = (
    "You grade whether a learner's summary covers given idea units. Reply with one JSON object "
    "and nothing else."
)

SCORE_PROMPT = """Idea units of the section, numbered:
{units}

The learner's summary:
<<<
{summary}
>>>

A unit is covered when the summary states its idea, in any words. A vague mention, a wrong
statement or a contradiction is not coverage. Return {{"covered": [unit numbers]}}, listing each
covered number once, or {{"covered": []}}."""

RETRY_NOTE = (
    "\n\nYour previous reply was not valid ({error}). Reply again with only the JSON object in "
    "the shape above."
)


def section_prompt(chunks: list[dict[str, Any]]) -> str:
    blocks = []
    for chunk in chunks:
        pages = ""
        if chunk.get("page_start") is not None:
            pages = f" (p. {chunk['page_start']}-{chunk.get('page_end') or chunk['page_start']})"
        heading = " > ".join(chunk.get("heading_path") or [])
        blocks.append(f"[c:{chunk['chunk_id']}]{pages} {heading}\n{chunk.get('text') or ''}")
    return PREPARE_PROMPT.format(section="\n\n".join(blocks))


_FENCE = re.compile(r"^```(?:json)?\s*\n(.*)\n```$", re.DOTALL)


def _parse(raw: str) -> Any:
    """The reply as a JSON object. Only a surrounding code fence is tolerated: no repair, since a
    repaired truncated reply would silently lose items."""
    text = (raw or "").strip()
    fenced = _FENCE.match(text)
    try:
        payload = json.loads(fenced.group(1) if fenced else text)
    except ValueError as exc:
        raise InvalidReply("not valid JSON") from exc
    if not isinstance(payload, dict):
        raise InvalidReply("not a JSON object")
    return payload


def parse_preparation(raw: str, section_ids: set[str]) -> Preparation:
    """The reply as a :class:`Preparation`, with items citing other chunks dropped."""
    try:
        prep = Preparation.model_validate(_parse(raw))
    except ValidationError as exc:
        raise InvalidReply(f"schema: {exc.errors()[0]['msg']}") from exc

    def inside(items: list) -> list:
        return [item for item in items if item.chunk_id in section_ids]

    prep = prep.model_copy(
        update={
            "prequestions": inside(prep.prequestions),
            "idea_units": inside(prep.idea_units),
            "atoms": inside(prep.atoms),
            "warrants": inside(prep.warrants) if prep.mathematical else [],
        }
    )
    if len(prep.prequestions) < MIN_PREQUESTIONS or len(prep.idea_units) < MIN_IDEA_UNITS:
        raise InvalidReply(
            f"needs at least {MIN_PREQUESTIONS} prequestion and {MIN_IDEA_UNITS} idea units "
            "citing chunks of the section"
        )
    return prep


def score_prompt(units: list[str], summary: str, *, reverse: bool) -> str:
    numbered = list(enumerate(units, start=1))
    if reverse:
        numbered.reverse()
    lines = "\n".join(f"{number}. {text}" for number, text in numbered)
    return SCORE_PROMPT.format(units=lines, summary=summary)


def parse_score(raw: str, unit_count: int) -> set[int]:
    payload = _parse(raw)
    covered = payload.get("covered")
    if set(payload) != {"covered"} or not isinstance(covered, list):
        raise InvalidReply('expected {"covered": [unit numbers]}')
    if not all(type(n) is int and 1 <= n <= unit_count for n in covered):
        raise InvalidReply(f"unit numbers must be integers from 1 to {unit_count}")
    if len(set(covered)) != len(covered):
        raise InvalidReply("each unit number may appear once")
    return set(covered)

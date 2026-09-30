"""Guided Reading (contract C11, CORPUS.md Stage 1): open a section, prepare it once, then
prequestions, closed-book recall, a scored summary and accepted atoms.

Order, enforced here: prepare, then prequestion guesses (before the recall), the recall, the
summary (after the recall), then atoms (after the summary). Keys, explanations, idea-unit texts and
warrant reasons stay server-side; the only idea-unit texts that leave are the missed units after
the summary.
"""

from __future__ import annotations

import asyncio
from collections.abc import Awaitable, Callable
import json
import logging
import secrets

from deeptutor.services.practice.answers import check_answer

from . import payloads
from .engine import EngineError, LibraryEngine
from .section import MAX_CHUNKS, heading_key, select_section
from .storage import ReadingConflict, ReadingStore

logger = logging.getLogger(__name__)

# (prompt, system prompt) -> reply text
LLM = Callable[[str, str], Awaitable[str]]


class ReadingNotFound(LookupError):
    """No such session, prequestion, atom, document or chunk."""


class PreparationFailed(RuntimeError):
    """The model did not return a valid reply after one retry."""


async def default_llm(prompt: str, system: str) -> str:
    """DeepTutor's configured LLM service."""
    from deeptutor.services.llm import complete

    return await complete(prompt=prompt, system_prompt=system)


def _public(prep: dict) -> dict:
    """The preparation as the learner sees it: no keys, explanations or idea-unit texts."""
    return {
        "prequestions": [{"id": q["id"], "q": q["display"]} for q in prep["prequestions"]],
        "idea_units": len(prep["idea_units"]),
        "atoms": [
            {key: atom[key] for key in ("id", "front", "back", "chunk_id")}
            for atom in prep["atoms"]
        ],
        "warrants": [
            {key: warrant[key] for key in ("id", "step", "chunk_id")}
            for warrant in prep["warrants"]
        ],
    }


def _stored(prep: payloads.Preparation) -> dict:
    """The validated preparation with ids, as kept on the server."""
    prequestions = []
    for number, q in enumerate(prep.prequestions, start=1):
        options = q.options or {}
        display = "\n".join([q.q, *(f"{key}. {options[key]}" for key in sorted(options))])
        prequestions.append({"id": f"q{number}", "display": display, **q.model_dump()})
    return {
        "mathematical": prep.mathematical,
        "prequestions": prequestions,
        "idea_units": [u.model_dump() for u in prep.idea_units],
        "atoms": [
            {
                "id": f"a{n}",
                **a.model_dump(),
                "provisional": a.kind in payloads.PROVISIONAL_KINDS,
            }
            for n, a in enumerate(prep.atoms, start=1)
        ],
        "warrants": [{"id": f"w{n}", **w.model_dump()} for n, w in enumerate(prep.warrants, 1)],
    }


class ReadingService:
    def __init__(self, store: ReadingStore, engine: LibraryEngine, llm: LLM = default_llm):
        self.store, self.engine, self.llm = store, engine, llm

    async def _session(self, session_id: str, *, prepared: bool = False) -> dict:
        try:
            session = await asyncio.to_thread(self.store.get, session_id)
        except LookupError as exc:
            raise ReadingNotFound(str(exc)) from exc
        if prepared and session["prep"] is None:
            raise ReadingConflict("Prepare the section first")
        return session

    async def _chapter_around(self, doc_id: str, chunk_id: str) -> tuple[list[dict], int]:
        """The chunks sharing the opened chunk's top heading, trimmed to the reach of a section."""
        run: list[dict] = []
        opened = None
        async for chunk in self.engine.scan(doc_id):
            if run and heading_key(chunk, 1) != heading_key(run[-1], 1):
                if opened is not None:
                    break
                run = []
            run.append(chunk)
            if chunk.get("chunk_id") == chunk_id:
                opened = len(run) - 1
            if opened is None and len(run) > MAX_CHUNKS:
                run.pop(0)  # a section never reaches further back than MAX_CHUNKS
            elif opened is not None and len(run) - opened > MAX_CHUNKS:
                break
        if opened is None:
            raise ReadingNotFound(f"Chunk {chunk_id} is not in document {doc_id}")
        return run, opened

    async def open(self, doc_id: str, from_chunk: str) -> dict:
        try:
            document = await self.engine.document(doc_id)
            chapter, opened = await self._chapter_around(doc_id, from_chunk)
        except EngineError as exc:
            if exc.status == 404:
                raise ReadingNotFound(str(exc)) from exc
            raise
        lo, hi = select_section(chapter, opened)
        chunks = chapter[lo:hi]
        pages = [c for c in chunks if c.get("page_start") is not None]
        section = {
            "heading_path": list(heading_key(chapter[opened], 2)),
            "chunk_ids": [c["chunk_id"] for c in chunks],
            "page_start": min(c["page_start"] for c in pages) if pages else None,
            "page_end": max(c.get("page_end") or c["page_start"] for c in pages) if pages else None,
        }
        session_id = secrets.token_hex(16)
        await asyncio.to_thread(
            self.store.create,
            {
                "session_id": session_id,
                "doc_id": doc_id,
                "doc_title": str(document.get("title") or doc_id),
                "heading_path": section["heading_path"],
                "chunks": [
                    {k: c.get(k) for k in ("chunk_id", "page_start", "page_end")} for c in chunks
                ],
                "ordinal_start": chunks[0]["ordinal"],
                "page_start": section["page_start"],
                "page_end": section["page_end"],
            },
        )
        return {"session_id": session_id, "section": section}

    async def _ask(self, prompt: str, system: str, parse: Callable[[str], object]):
        """One model call, retried once with the validation error when the reply is invalid."""
        reply = await self.llm(prompt, system)
        try:
            return parse(reply)
        except payloads.InvalidReply as exc:
            logger.info("Guided reading: invalid model reply (%s); retrying once", exc)
            reply = await self.llm(prompt + payloads.RETRY_NOTE.format(error=exc), system)
        try:
            return parse(reply)
        except payloads.InvalidReply as exc:
            raise PreparationFailed(f"The model's reply was invalid twice: {exc}") from exc

    async def prepare(self, session_id: str) -> dict:
        session = await self._session(session_id)
        if session["prep"] is not None:
            return _public(session["prep"])
        section_ids = [c["chunk_id"] for c in session["chunks"]]
        try:
            page = await self.engine.chunks(
                session["doc_id"], session["ordinal_start"], len(section_ids)
            )
        except EngineError as exc:
            if exc.status == 404:
                raise ReadingConflict("The document is no longer in the library") from exc
            raise
        chunks = page["chunks"]
        if [c.get("chunk_id") for c in chunks] != section_ids:
            raise ReadingConflict("The document changed since this section was opened")
        prep = await self._ask(
            payloads.section_prompt(chunks),
            payloads.PREPARE_SYSTEM,
            lambda reply: payloads.parse_preparation(reply, set(section_ids)),
        )
        await asyncio.to_thread(
            self.store.save_preparation, session_id, json.dumps(_stored(prep), ensure_ascii=False)
        )
        # A concurrent call may have stored its preparation first; that one wins.
        return _public((await self._session(session_id))["prep"])

    async def answer_prequestion(
        self, session_id: str, qid: str, answer: str, confidence: int
    ) -> dict:
        session = await self._session(session_id, prepared=True)
        question = next((q for q in session["prep"]["prequestions"] if q["id"] == qid), None)
        if question is None:
            raise ReadingNotFound(f"Prequestion {qid} not found")
        options = question["options"] or {}
        correct = check_answer(
            {
                "question_type": "single_choice" if options else "short_answer",
                "options": options,
                "correct_answer": question["answer"],
            },
            answer,
        )
        await asyncio.to_thread(
            self.store.answer_prequestion, session_id, qid, answer, confidence, correct
        )
        return {"correct": correct}

    async def recall(self, session_id: str, text: str) -> dict:
        await self._session(session_id, prepared=True)
        await asyncio.to_thread(self.store.save_recall, session_id, text)
        return {"ok": True}

    async def _score_pass(self, units: list[str], summary: str, *, reverse: bool) -> set[int]:
        return await self._ask(
            payloads.score_prompt(units, summary, reverse=reverse),
            payloads.SCORE_SYSTEM,
            lambda reply: payloads.parse_score(reply, len(units)),
        )

    async def summary(self, session_id: str, text: str, judgement: int) -> dict:
        session = await self._session(session_id, prepared=True)
        if session["recall_at"] is None:
            raise ReadingConflict("Write the closed-book recall before the summary")
        if session["summary_at"] is not None:
            raise ReadingConflict("The summary was already scored")
        units = session["prep"]["idea_units"]
        texts = [u["text"] for u in units]
        # Two independent passes (the second sees the units in reverse order); a unit counts as
        # covered only when both agree.
        passes = await asyncio.gather(
            self._score_pass(texts, text, reverse=False),
            self._score_pass(texts, text, reverse=True),
            return_exceptions=True,
        )
        for result in passes:
            if isinstance(result, BaseException):
                raise result
        first, second = passes
        covered = first & second
        disagreements = sorted(first ^ second)
        if disagreements:
            logger.info(
                "Guided reading %s: scoring passes disagree on units %s; counted as missed",
                session_id,
                disagreements,
            )
        await asyncio.to_thread(
            self.store.save_summary,
            session_id,
            text,
            judgement,
            {
                "passes": [sorted(first), sorted(second)],
                "covered": sorted(covered),
                "disagreements": disagreements,
            },
        )
        return {
            "covered": len(covered),
            "missed": [
                {"unit": unit["text"], "chunk_id": unit["chunk_id"]}
                for number, unit in enumerate(units, start=1)
                if number not in covered
            ],
        }

    async def accept_atoms(self, session_id: str, ids: list[str]) -> dict:
        session = await self._session(session_id, prepared=True)
        if session["summary_at"] is None:
            raise ReadingConflict("Atoms are accepted after the summary")
        atoms = {atom["id"]: atom for atom in session["prep"]["atoms"]}
        unknown = [atom_id for atom_id in ids if atom_id not in atoms]
        if unknown:
            raise ReadingNotFound(f"Atoms not found: {', '.join(unknown)}")
        entry_ids = await asyncio.to_thread(
            self.store.add_atom_entries, session, [atoms[atom_id] for atom_id in ids]
        )
        return {"entry_ids": entry_ids}

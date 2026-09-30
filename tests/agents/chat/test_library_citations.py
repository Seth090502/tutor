"""Chat citation post-check (CORPUS.md Stage 0 step 5): citations are verified before a plain chat
answer is shown; failed ones are removed and the answer says so."""

from __future__ import annotations

from types import SimpleNamespace

import pytest

from deeptutor.agents.chat.agentic_pipeline import AgenticChatPipeline
from deeptutor.core.context import UnifiedContext
from deeptutor.services.library import engine as engine_mod
from deeptutor.services.library.citations import unverified_note
from tests.agents.chat.test_agent_loop import (
    _answer_text,
    _contents,
    _llm_chunk,
    _Registry,
    _result,
    _run,
    _ScriptedChatClient,
)
from tests.services.library.fake_rag_engine import FakeRagEngine

GOOD = 'A derivative is a limit [c:calc:0] "the limit of the difference quotient".'
BAD = ' It was found in 1492 [c:calc:0] "found in 1492".'


@pytest.fixture(autouse=True)
def _fake_llm_config(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        "deeptutor.agents.loop.pipeline.get_llm_config",
        lambda: SimpleNamespace(
            binding="openai",
            model="gpt-test",
            api_key="k",
            base_url="u",
            api_version=None,
            extra_headers={},
            reasoning_effort=None,
        ),
    )


@pytest.fixture
def library(monkeypatch: pytest.MonkeyPatch) -> FakeRagEngine:
    monkeypatch.delenv("RAG_ENGINE_URL", raising=False)
    monkeypatch.delenv("RAG_ENGINE_TOKEN", raising=False)
    fake = FakeRagEngine()
    fake.add("calc", ["The derivative is the limit of the difference quotient."])
    real = engine_mod.RagEngine
    monkeypatch.setattr(engine_mod, "RagEngine", lambda: real(transport=fake.transport()))
    return fake


def ctx(message: str) -> UnifiedContext:
    # A runtime workspace of its own, so the turn does not depend on the machine's workspace.
    context = UnifiedContext(session_id="s", user_message=message)
    context.runtime.workspace = SimpleNamespace(output_dir=".")
    return context


def pipeline_for(
    monkeypatch: pytest.MonkeyPatch, answer: str, *, attached: bool = True
) -> AgenticChatPipeline:
    pipeline = AgenticChatPipeline(language="en")
    pipeline.registry = _Registry()
    client = _ScriptedChatClient([[_llm_chunk(content=answer, finish_reason="stop")]])
    monkeypatch.setattr(pipeline, "_compose_enabled_tools", lambda _context: [])
    monkeypatch.setattr(pipeline, "_build_openai_client", lambda: client)
    monkeypatch.setattr(pipeline, "_library_attached", lambda _context: attached)
    monkeypatch.setattr(pipeline, "_workspace_system_note", lambda _context: "")
    return pipeline


@pytest.mark.asyncio
async def test_failed_citation_is_removed_before_the_answer_is_shown(monkeypatch, library) -> None:
    pipeline = pipeline_for(monkeypatch, GOOD + BAD)

    events = await _run(pipeline, ctx("What is a derivative?"))

    corrected = GOOD + " It was found in 1492.\n\n" + unverified_note(1)
    # Buffered: the unverified quote never reached the wire, not even briefly.
    assert not any('found in 1492"' in c for c in _contents(events))
    assert _answer_text(events) == corrected
    assert _result(events).metadata["response"] == corrected
    assert [r.url.path for r in library.requests] == ["/documents/calc/chunks"]


@pytest.mark.asyncio
async def test_verified_citations_are_shown_unchanged(monkeypatch, library) -> None:
    pipeline = pipeline_for(monkeypatch, GOOD)

    events = await _run(pipeline, ctx("What is a derivative?"))

    assert _answer_text(events) == GOOD
    assert _result(events).metadata["response"] == GOOD


@pytest.mark.asyncio
async def test_citations_are_checked_even_without_the_library_attached(
    monkeypatch, library
) -> None:
    library.down = True
    pipeline = pipeline_for(monkeypatch, GOOD, attached=False)

    events = await _run(pipeline, ctx("What is a derivative?"))

    corrected = "A derivative is a limit.\n\n" + unverified_note(1)
    assert _answer_text(events) == corrected


@pytest.mark.asyncio
async def test_answers_without_citations_never_touch_the_library(monkeypatch, library) -> None:
    pipeline = pipeline_for(monkeypatch, "Plain answer.", attached=False)

    events = await _run(pipeline, ctx("hi"))

    assert _answer_text(events) == "Plain answer."
    assert library.requests == []


def test_only_plain_chat_with_the_library_attached_buffers(monkeypatch) -> None:
    pipeline = AgenticChatPipeline(language="en")
    monkeypatch.setattr(pipeline, "_library_attached", lambda _context: True)

    assert pipeline._capability_buffers_visible_output(UnifiedContext(user_message="x"))
    monkeypatch.setattr(pipeline, "_library_attached", lambda _context: False)
    assert not pipeline._capability_buffers_visible_output(UnifiedContext(user_message="x"))


def test_library_attached_reads_the_kb_type_once(monkeypatch) -> None:
    calls: list[str] = []

    def metadata(kb: str):
        calls.append(kb)
        return {"type": "tutor_library"} if kb == "Library" else {"type": "local"}

    monkeypatch.setattr("deeptutor.multi_user.knowledge_access.resolve_kb_metadata", metadata)
    pipeline = AgenticChatPipeline(language="en")
    context = UnifiedContext(user_message="x", knowledge_bases=["Notes", "Library"])

    assert pipeline._library_attached(context)
    assert pipeline._library_attached(context)
    assert calls == ["Notes", "Library"]
    assert not pipeline._library_attached(
        UnifiedContext(user_message="x", knowledge_bases=["Notes"])
    )

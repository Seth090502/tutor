"""/api/library (contract C11): search pipeline, leak guard end to end, verify, settings."""

from __future__ import annotations

import asyncio
import json

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import library as library_router
from deeptutor.services.library import settings as library_settings
from deeptutor.services.library.engine import RagEngine
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore
from tests.services.library.fake_rag_engine import FakeLLM, FakeRagEngine

REWRITES = json.dumps(
    {
        "queries": [
            {"kind": "vocabulary", "query": "organelle ATP synthesis"},
            {"kind": "passage", "query": "Most ATP is made by oxidative phosphorylation."},
            {"kind": "prerequisite", "query": "cell respiration"},
        ]
    }
)


def corpus() -> FakeRagEngine:
    engine = FakeRagEngine()
    engine.add(
        "bio",
        [
            ("Cells make ATP in several organelles.", ["Ch 4", "4.1 Energy"]),
            (
                "The mitochondrion makes most ATP by oxidative phosphorylation.",
                ["Ch 4", "4.1 Energy"],
            ),
            ("Glycolysis makes a little ATP in the cytosol.", ["Ch 4", "4.1 Energy"]),
            ("Cell respiration overview and ATP yield.", ["Ch 4", "4.2 Respiration"]),
            ("Exercise 4.3: B. ATP comes mostly from here.", ["Ch 4", "Answers to Exercises"]),
        ],
        title="Biology",
    )
    engine.add(
        "chem", [("ATP hydrolysis releases energy.", ["Ch 9", "Thermodynamics"])], title="Chemistry"
    )
    return engine


class Harness:
    def __init__(self, monkeypatch, tmp_path) -> None:
        self.engine = corpus()
        self.llm = FakeLLM(rewrite=REWRITES)
        self.store = SQLiteSessionStore(tmp_path / "practice.db")
        monkeypatch.setattr(
            library_router, "_engine", lambda: RagEngine(transport=self.engine.transport())
        )
        monkeypatch.setattr(library_router, "make_ask", lambda model: self.llm)
        monkeypatch.setattr(library_router, "get_sqlite_session_store", lambda: self.store)
        settings_path = tmp_path / "settings" / "library.json"
        monkeypatch.setattr(library_settings, "_path", lambda: settings_path)
        app = FastAPI()
        app.include_router(library_router.router, prefix="/api/library")
        self.client = TestClient(app)

    def search(self, **body):
        return self.client.post("/api/library/search", json={"query": "ATP organelle", **body})

    def settings(self, **changes):
        response = self.client.put("/api/library/settings", json=changes)
        assert response.status_code == 200, response.text
        return response.json()

    def open_attempt(self, *, check: bool = False) -> str:
        async def seed() -> int:
            await self.store.ensure_notebook_session("lesson", "Lesson")
            await self.store.upsert_notebook_entries(
                "lesson",
                [
                    {
                        "question_id": "q1",
                        "question": "Which organelle makes most of the cell's ATP?",
                        "question_type": "single_choice",
                        "options": {"A": "ribosome", "B": "mitochondrion"},
                        "correct_answer": "B",
                        "explanation": "",
                        "result": "ungraded",
                    }
                ],
            )
            return (await self.store.find_notebook_entry("lesson", "q1"))["id"]

        practice = PracticeStore(self.store.db_path)
        attempt_id = practice.open_attempt(asyncio.run(seed()))["attempt_id"]
        if check:
            practice.check_attempt(attempt_id, "B")
        return attempt_id


@pytest.fixture
def h(monkeypatch, tmp_path) -> Harness:
    monkeypatch.delenv("RAG_ENGINE_URL", raising=False)
    monkeypatch.delenv("RAG_ENGINE_TOKEN", raising=False)
    return Harness(monkeypatch, tmp_path)


def pack_ids(body: dict) -> set[str]:
    return {item["chunk_id"] for item in body["pack"]}


def test_default_settings_run_only_the_raw_query_and_no_model_calls(h: Harness) -> None:
    response = h.search()

    assert response.status_code == 200, response.text
    body = response.json()
    assert body["queries"] == ["ATP organelle"]
    assert body["mode_used"] == "bm25"
    assert body["withheld"] == 0
    assert h.llm.calls == []
    assert h.engine.search_queries() == ["ATP organelle"]
    first = h.engine.requests[0]
    assert json.loads(first.content) == {"query": "ATP organelle", "k": 30, "mode": "hybrid"}
    item = body["pack"][0]
    assert {
        "chunk_id",
        "doc_id",
        "title",
        "heading_path",
        "text",
        "ranks",
        "section",
    } <= item.keys()
    assert all(s.startswith("[c:") for s in item["section"])


def test_settings_default_off_and_round_trip(h: Harness) -> None:
    assert h.client.get("/api/library/settings").json() == {
        "rewrite": False,
        "rerank": False,
        "agentic": False,
        "model": "",
    }
    assert h.settings(rewrite=True, model=" claude-sonnet-5-5 ") == {
        "rewrite": True,
        "rerank": False,
        "agentic": False,
        "model": "claude-sonnet-5-5",
    }
    assert h.client.get("/api/library/settings").json()["rewrite"] is True


def test_rewrite_runs_every_query_and_fuses_them(h: Harness) -> None:
    h.settings(rewrite=True)

    body = h.search(domains=["math"], source_types=["book"]).json()

    assert body["queries"] == [
        "ATP organelle",
        "organelle ATP synthesis",
        "Most ATP is made by oxidative phosphorylation.",
        "cell respiration",
    ]
    assert h.engine.search_queries() == body["queries"]
    assert all(
        json.loads(r.content)["domains"] == ["math"]
        and json.loads(r.content)["source_types"] == ["book"]
        for r in h.engine.requests
        if r.url.path == "/search"
    )
    assert h.llm.count("rewrite") == 1


def test_a_failed_rewrite_query_is_left_out_but_the_raw_query_must_succeed(h: Harness) -> None:
    h.settings(rewrite=True)
    h.engine.fail_queries = {"cell respiration"}

    body = h.search().json()
    assert "cell respiration" not in body["queries"]
    assert body["queries"][0] == "ATP organelle"

    h.engine.fail_queries = {"ATP organelle"}
    response = h.search()
    assert response.status_code == 502
    assert "HTTP 500" in response.json()["detail"]


def test_rerank_runs_only_when_flat_or_deep(h: Harness) -> None:
    h.settings(rerank=True)
    h.llm.replies["rerank"] = '{"order": [2, 1]}'

    # One query list is always flat (see is_flat), so rerank runs.
    h.search()
    assert h.llm.count("rerank") == 1

    # With rewrites that all agree on one chunk the top is not flat: no rerank...
    h.settings(rewrite=True)
    h.llm.replies["rewrite"] = json.dumps(
        {"queries": ["mitochondrion", "mitochondrion ATP", "mitochondrion oxidative"]}
    )
    h.search(query="mitochondrion")
    assert h.llm.count("rerank") == 1
    # ...unless the tutor asks for depth.
    h.search(query="mitochondrion", depth="deep")
    assert h.llm.count("rerank") == 2


def test_rerank_off_never_calls_the_model_even_for_depth(h: Harness) -> None:
    h.search(depth="deep")
    assert h.llm.calls == []


def test_agentic_search_adds_searches_up_to_the_cap(h: Harness) -> None:
    h.settings(agentic=True)
    h.llm.agentic = [f'{{"action": "search", "query": "energy {i}"}}' for i in range(6)]

    body = h.search().json()

    assert body["queries"] == ["ATP organelle", "energy 0", "energy 1", "energy 2", "energy 3"]


def test_open_attempt_withholds_answer_chunks_from_every_step(h: Harness) -> None:
    h.settings(rewrite=True, rerank=True, agentic=True)
    h.llm.agentic = ['{"action": "open_chunk", "chunk_id": "bio:1"}']
    attempt_id = h.open_attempt()

    body = h.search(attempt_id=attempt_id).json()

    # bio:1 names the answer ("mitochondrion"), bio:4 sits under "Answers to Exercises".
    assert body["withheld"] == 2
    packed = pack_ids(body) | {
        s.split("]")[0][3:] for item in body["pack"] for s in item["section"]
    }
    assert "bio:1" not in packed and "bio:4" not in packed
    prompts = "\n".join(prompt for _, prompt in h.llm.calls)
    assert "mitochondrion" not in prompts.lower()
    assert "withheld while the learner's attempt is open" in prompts


def test_closed_attempt_withholds_nothing(h: Harness) -> None:
    attempt_id = h.open_attempt(check=True)

    body = h.search(attempt_id=attempt_id).json()

    assert body["withheld"] == 0
    assert "bio:1" in pack_ids(body) | {
        s.split("]")[0][3:] for item in body["pack"] for s in item["section"]
    }


def test_unknown_attempt_is_404(h: Harness) -> None:
    response = h.search(attempt_id="nope")

    assert response.status_code == 404
    assert h.engine.requests == []


@pytest.mark.parametrize(
    "body",
    [
        {"query": " "},
        {"query": "x", "k": 13},
        {"query": "x", "depth": "max"},
        {"query": "x", "source_types": ["film"]},
    ],
)
def test_search_validation(h: Harness, body: dict) -> None:
    assert h.client.post("/api/library/search", json=body).status_code == 422


def test_engine_down_is_502(h: Harness) -> None:
    h.engine.down = True

    response = h.search()

    assert response.status_code == 502
    assert "not running" in response.json()["detail"]


def test_verify_route(h: Harness) -> None:
    response = h.client.post(
        "/api/library/verify",
        json={
            "citations": [
                {"chunk_id": "bio:1", "quote": "makes most ATP", "page": 2},
                {"chunk_id": "bio:1", "quote": "makes all ATP"},
                {"chunk_id": "bio:1", "quote": "makes most ATP", "page": 9},
                {"chunk_id": "bio:77", "quote": "anything"},
            ]
        },
    )

    assert response.status_code == 200
    assert response.json() == {
        "results": [
            {"ok": True, "reason": None},
            {"ok": False, "reason": "not_substring"},
            {"ok": False, "reason": "page_out_of_range"},
            {"ok": False, "reason": "no_chunk"},
        ]
    }


def test_verify_with_the_engine_down_is_502(h: Harness) -> None:
    h.engine.down = True

    response = h.client.post(
        "/api/library/verify", json={"citations": [{"chunk_id": "bio:1", "quote": "x"}]}
    )

    assert response.status_code == 502


def test_library_settings_are_setup_rows(h: Harness) -> None:
    from deeptutor.services.config.settings_spec import setting_specs

    specs = setting_specs()
    for flag in ("rewrite", "rerank", "agentic"):
        spec = specs[f"library.{flag}"]
        assert spec.read() == "off"
        spec.write("on")
        assert spec.read() == "on"
    assert h.client.get("/api/library/settings").json()["agentic"] is True

"""Guided Reading routes (contract C11, CORPUS.md Stage 1) with a fake rag-engine and a scripted LLM."""

import json
import logging

from fastapi import FastAPI
from fastapi.testclient import TestClient
import httpx
import pytest

from deeptutor.api.routers import guided_reading
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.reading import engine as engine_module
from deeptutor.services.reading.engine import LibraryEngine
from deeptutor.services.reading.service import ReadingService
from deeptutor.services.reading.storage import ReadingStore, initialize_reading
from deeptutor.services.session.sqlite_store import SQLiteSessionStore

DOC = "doc-analysis"


def _chunk(ordinal, page, *path, pages=2):
    return {
        "chunk_id": f"k{ordinal}",
        "ordinal": ordinal,
        "text": f"Passage {ordinal} of the book.",
        "page_start": page,
        "page_end": page + pages - 1,
        "heading_path": list(path),
    }


# Chapter 1: 1.1 is pp. 1-8 (k0-k3), 1.2 is pp. 9-12 (k4-k5); chapter 2 follows.
CHUNKS = [
    *(_chunk(i, 1 + 2 * i, "1 Sets", "1.1 Basics") for i in range(4)),
    *(_chunk(4 + i, 9 + 2 * i, "1 Sets", "1.2 Maps") for i in range(2)),
    *(_chunk(6 + i, 13 + 2 * i, "2 Limits", "2.1 Sequences") for i in range(4)),
]
SECTION = ["k0", "k1", "k2", "k3"]
SECRETS = [
    "SECRET-EXPLANATION-1",
    "SECRET-OPEN-ANSWER",
    "SECRET-EXPLANATION-2",
    "SECRET-WARRANT",
    *(f"SECRET-UNIT-{i}" for i in range(1, 7)),
]


def preparation(**changes):
    payload = {
        "mathematical": True,
        "prequestions": [
            {
                "q": "Which sequences always converge in R?",
                "options": {"A": "Bounded sequences", "B": "Cauchy sequences"},
                "answer": "B",
                "explanation": "SECRET-EXPLANATION-1",
                "chunk_id": "k1",
            },
            {
                "q": "What does it mean for a set to be countable?",
                "options": None,
                "answer": "SECRET-OPEN-ANSWER",
                "explanation": "SECRET-EXPLANATION-2",
                "chunk_id": "k2",
            },
        ],
        "idea_units": [
            {"text": f"SECRET-UNIT-{i}", "chunk_id": SECTION[i % 4]} for i in range(1, 7)
        ],
        "atoms": [
            {
                "front": "Define a countable set.",
                "back": "A set in bijection with a subset of N.",
                "kind": "definition",
                "chunk_id": "k0",
            },
            {
                "front": "Prove that Q is countable.",
                "back": "List fractions p/q by p + q and skip repeats.",
                "kind": "proof",
                "chunk_id": "k3",
            },
            {
                "front": "Symbol for the natural numbers?",
                "back": "N",
                "kind": "notation",
                "chunk_id": "k1",
            },
        ],
        "warrants": [
            {"step": "Enumerate by p + q.", "warrant": "SECRET-WARRANT", "chunk_id": "k3"}
        ],
    }
    payload.update(changes)
    return payload


class ScriptedLLM:
    """Returns queued replies in call order and records every prompt.

    ``by_pass`` holds replies for one scoring pass. The passes run concurrently, so they are told
    apart by unit order: the second pass lists the idea units in reverse.
    """

    def __init__(self, *replies):
        self.replies = list(replies)
        self.by_pass = {1: [], 2: []}
        self.prompts = []

    def queue(self, *replies):
        self.replies.extend(replies)

    async def __call__(self, prompt: str, system: str) -> str:
        self.prompts.append(prompt)
        replies = self.replies
        if "SECRET-UNIT-1" in prompt and "SECRET-UNIT-6" in prompt:
            second = prompt.index("SECRET-UNIT-6") < prompt.index("SECRET-UNIT-1")
            replies = self.by_pass[2 if second else 1] or replies
        reply = replies.pop(0)
        if callable(reply):
            return await reply(prompt, system)
        return reply if isinstance(reply, str) else json.dumps(reply)


def fake_engine(requests, chunks=None, fail=None):
    chunks = CHUNKS if chunks is None else chunks

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        if fail is not None:
            raise fail
        path = request.url.path
        if path == f"/documents/{DOC}":
            return httpx.Response(
                200, json={"doc_id": DOC, "title": "Analysis I", "pages": 20, "outline": []}
            )
        if path == f"/documents/{DOC}/chunks":
            start = int(request.url.params.get("from", 0))
            limit = int(request.url.params.get("limit", 20))
            page = [c for c in chunks if c["ordinal"] >= start][: limit + 1]
            following = page.pop()["ordinal"] if len(page) > limit else None
            return httpx.Response(200, json={"chunks": page, "next": following})
        return httpx.Response(404, json={"error": "document not found"})

    return httpx.MockTransport(handler)


@pytest.fixture
def reading(tmp_path, monkeypatch):
    monkeypatch.delenv("RAG_ENGINE_URL", raising=False)
    monkeypatch.delenv("RAG_ENGINE_TOKEN", raising=False)
    store = SQLiteSessionStore(tmp_path / "reading.db")
    llm = ScriptedLLM()
    requests: list[httpx.Request] = []
    state = {"transport": fake_engine(requests)}
    app = FastAPI()
    app.include_router(guided_reading.router, prefix="/api/reading")
    app.dependency_overrides[guided_reading.get_reading_service] = lambda: ReadingService(
        ReadingStore(store.db_path), LibraryEngine(transport=state["transport"]), llm
    )

    class Harness:
        def __init__(self, client):
            self.client, self.llm, self.store, self.requests = client, llm, store, requests
            self.state = state

        def open(self, chunk="k1"):
            response = client.post(
                "/api/reading/sessions", json={"doc_id": DOC, "from_chunk": chunk}
            )
            assert response.status_code == 200, response.text
            return response.json()

        def post(self, session_id, path, body=None):
            return client.post(f"/api/reading/sessions/{session_id}/{path}", json=body)

        def prepared(self, payload=None):
            session_id = self.open()["session_id"]
            self.llm.queue(payload or preparation())
            response = self.post(session_id, "prepare")
            assert response.status_code == 200, response.text
            return session_id, response.json()

        def summarised(self):
            session_id, prep = self.prepared()
            assert self.post(session_id, "recall", {"text": "Sets, maps."}).status_code == 200
            self.llm.queue({"covered": [1, 2]}, {"covered": [1, 2]})
            response = self.post(session_id, "summary", {"text": "Countable sets.", "judgement": 2})
            assert response.status_code == 200, response.text
            return session_id, prep

    with TestClient(app) as client:
        yield Harness(client)


# Sessions and section bounds


def test_open_returns_the_section_of_the_opened_chunk(reading):
    body = reading.open("k1")
    assert set(body) == {"session_id", "section"}
    assert body["section"] == {
        "heading_path": ["1 Sets", "1.1 Basics"],
        "chunk_ids": SECTION,
        "page_start": 1,
        "page_end": 8,
    }


def test_a_short_section_grows_within_its_chapter(reading):
    # 1.2 is four pages; it grows backwards into 1.1 and never into chapter 2.
    section = reading.open("k5")["section"]
    assert section["chunk_ids"] == ["k3", "k4", "k5"]
    assert (section["page_start"], section["page_end"]) == (7, 12)
    assert section["heading_path"] == ["1 Sets", "1.2 Maps"]


def test_the_scan_follows_chunk_pages(reading, monkeypatch):
    monkeypatch.setattr(engine_module, "PAGE_SIZE", 3)
    assert reading.open("k7")["section"]["chunk_ids"] == ["k6", "k7", "k8", "k9"]
    starts = [r.url.params.get("from") for r in reading.requests if r.url.path.endswith("/chunks")]
    assert starts == ["0", "3", "6", "9"]


def test_open_rejects_unknown_documents_and_chunks(reading):
    missing_doc = reading.client.post(
        "/api/reading/sessions", json={"doc_id": "nope", "from_chunk": "k1"}
    )
    assert missing_doc.status_code == 404
    missing_chunk = reading.client.post(
        "/api/reading/sessions", json={"doc_id": DOC, "from_chunk": "k99"}
    )
    assert missing_chunk.status_code == 404


def test_an_unreachable_engine_is_a_503(reading):
    reading.state["transport"] = fake_engine([], fail=httpx.ConnectError("refused"))
    response = reading.client.post(
        "/api/reading/sessions", json={"doc_id": DOC, "from_chunk": "k1"}
    )
    assert response.status_code == 503
    assert "rag-engine" in response.json()["detail"]


def test_chunk_requests_carry_the_engine_token(reading, monkeypatch):
    monkeypatch.setenv("RAG_ENGINE_TOKEN", "tok-123")
    reading.open()
    assert reading.requests
    assert all(r.headers["authorization"] == "Bearer tok-123" for r in reading.requests)


def test_reading_tables_migrate_idempotently(reading):
    import sqlite3

    conn = sqlite3.connect(reading.store.db_path)
    try:
        initialize_reading(conn)
        initialize_reading(conn)
        names = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    finally:
        conn.close()
    assert {"reading_sessions", "reading_prequestion_answers"} <= names


# Preparation


def test_prepare_returns_questions_without_keys(reading):
    session_id = reading.open()["session_id"]
    reading.llm.queue(preparation())
    response = reading.post(session_id, "prepare")
    assert response.status_code == 200, response.text
    body = response.json()
    assert set(body) == {"prequestions", "idea_units", "atoms", "warrants"}
    assert body["idea_units"] == 6
    assert [q["id"] for q in body["prequestions"]] == ["q1", "q2"]
    assert all(set(q) == {"id", "q"} for q in body["prequestions"])
    assert "A. Bounded sequences" in body["prequestions"][0]["q"]
    assert body["atoms"][0] == {
        "id": "a1",
        "front": "Define a countable set.",
        "back": "A set in bijection with a subset of N.",
        "chunk_id": "k0",
    }
    assert body["warrants"] == [{"id": "w1", "step": "Enumerate by p + q.", "chunk_id": "k3"}]
    for secret in SECRETS:
        assert secret not in response.text
    # The section text went to the model, labelled by chunk id.
    assert "[c:k0]" in reading.llm.prompts[0] and "Passage 3 of the book." in reading.llm.prompts[0]

    # Preparing again returns the stored preparation without another model call.
    again = reading.post(session_id, "prepare")
    assert again.json() == body
    assert len(reading.llm.prompts) == 1


def test_prepare_retries_once_on_invalid_json(reading):
    session_id = reading.open()["session_id"]
    reading.llm.queue("Here are your questions: {not json", preparation())
    response = reading.post(session_id, "prepare")
    assert response.status_code == 200, response.text
    assert len(reading.llm.prompts) == 2
    assert "not valid" in reading.llm.prompts[1]


@pytest.mark.parametrize(
    "bad",
    [
        preparation(idea_units=[{"text": "one", "chunk_id": "k0"}] * 11),
        preparation(prequestions=[]),
        preparation(atoms=preparation()["atoms"] * 2),
        preparation(
            prequestions=[{**preparation()["prequestions"][0], "answer": "C"}],
        ),
        preparation(atoms=[{**preparation()["atoms"][0], "kind": "trivia"}]),
        preparation(idea_units=[{"text": "", "chunk_id": "k0"}] * 4),
        preparation(extra="field"),
        {**preparation(), "mathematical": "yes"},
    ],
)
def test_schema_violations_count_as_invalid(reading, bad):
    session_id = reading.open()["session_id"]
    reading.llm.queue(bad, preparation())
    assert reading.post(session_id, "prepare").status_code == 200
    assert len(reading.llm.prompts) == 2


def test_prepare_fails_after_two_invalid_replies_and_keeps_nothing(reading):
    session_id = reading.open()["session_id"]
    reading.llm.queue("nope", {"prequestions": "nope"})
    response = reading.post(session_id, "prepare")
    assert response.status_code == 502
    # Nothing was stored: a later call prepares afresh.
    reading.llm.queue(preparation())
    assert reading.post(session_id, "prepare").status_code == 200
    assert len(reading.llm.prompts) == 3


def test_an_llm_service_error_is_a_502(reading):
    from deeptutor.services.llm.exceptions import LLMConfigError

    async def broken(prompt, system):
        raise LLMConfigError("No LLM provider is configured")

    reading.llm.replies.append(broken)
    session_id = reading.open()["session_id"]
    response = reading.post(session_id, "prepare")
    assert response.status_code == 502
    assert "No LLM provider" in response.json()["detail"]


def test_items_citing_chunks_outside_the_section_are_dropped(reading):
    payload = preparation()
    payload["prequestions"][1]["chunk_id"] = "k7"
    payload["idea_units"][0]["chunk_id"] = "k9"
    payload["atoms"][2]["chunk_id"] = "elsewhere"
    payload["warrants"][0]["chunk_id"] = "k5"
    session_id, body = reading.prepared(payload)
    assert len(reading.llm.prompts) == 1
    assert [q["id"] for q in body["prequestions"]] == ["q1"]
    assert body["idea_units"] == 5
    assert [a["chunk_id"] for a in body["atoms"]] == ["k0", "k3"]
    assert body["warrants"] == []


def test_too_few_items_after_dropping_is_invalid(reading):
    payload = preparation()
    for unit in payload["idea_units"][:3]:
        unit["chunk_id"] = "k8"  # only three units stay, below the minimum of four
    session_id = reading.open()["session_id"]
    reading.llm.queue(payload, preparation())
    assert reading.post(session_id, "prepare").json()["idea_units"] == 6
    assert len(reading.llm.prompts) == 2


def test_warrants_are_kept_only_for_mathematical_sections(reading):
    _, body = reading.prepared(preparation(mathematical=False))
    assert body["warrants"] == []


def test_a_changed_document_is_a_conflict_at_prepare(reading):
    session_id = reading.open()["session_id"]
    renumbered = [{**c, "chunk_id": f"new-{c['ordinal']}"} for c in CHUNKS]
    reading.state["transport"] = fake_engine([], chunks=renumbered)
    response = reading.post(session_id, "prepare")
    assert response.status_code == 409
    assert reading.llm.prompts == []


def test_unknown_sessions_are_404(reading):
    assert reading.post("0" * 32, "prepare").status_code == 404
    assert reading.post("0" * 32, "recall", {"text": "x"}).status_code == 404


# Prequestions


def test_prequestion_answers_are_checked_by_the_practice_normaliser(reading):
    session_id, _ = reading.prepared()
    right = reading.post(
        session_id, "prequestions/q1/answer", {"answer": "cauchy sequences", "confidence": 3}
    )
    assert right.status_code == 200
    assert right.json() == {"correct": True}
    open_answer = reading.post(
        session_id, "prequestions/q2/answer", {"answer": "Listable", "confidence": 1}
    )
    assert open_answer.json() == {"correct": None}
    for secret in SECRETS:
        assert secret not in right.text + open_answer.text

    store = ReadingStore(reading.store.db_path)
    with store.connect() as conn:
        rows = conn.execute(
            "SELECT qid, answer, confidence, correct FROM reading_prequestion_answers ORDER BY qid"
        ).fetchall()
    assert [tuple(r) for r in rows] == [
        ("q1", "cauchy sequences", 3, 1),
        ("q2", "Listable", 1, None),
    ]


def test_a_wrong_choice_is_false(reading):
    session_id, _ = reading.prepared()
    response = reading.post(session_id, "prequestions/q1/answer", {"answer": "A", "confidence": 4})
    assert response.json() == {"correct": False}


def test_prequestion_answer_rules(reading):
    session_id = reading.open()["session_id"]
    body = {"answer": "B", "confidence": 2}
    assert reading.post(session_id, "prequestions/q1/answer", body).status_code == 409  # unprepared
    reading.llm.queue(preparation())
    reading.post(session_id, "prepare")
    assert reading.post(session_id, "prequestions/q9/answer", body).status_code == 404
    bad_confidence = {"answer": "B", "confidence": 5}
    assert reading.post(session_id, "prequestions/q1/answer", bad_confidence).status_code == 422
    assert reading.post(session_id, "prequestions/q1/answer", body).status_code == 200
    assert reading.post(session_id, "prequestions/q1/answer", body).status_code == 409  # once
    assert reading.post(session_id, "recall", {"text": "Sets."}).status_code == 200
    # Prequestions are guessed before reading, so none after the recall.
    assert reading.post(session_id, "prequestions/q2/answer", body).status_code == 409


# Recall


def test_recall_is_stored_once_after_preparation(reading):
    session_id = reading.open()["session_id"]
    assert reading.post(session_id, "recall", {"text": "Sets."}).status_code == 409
    reading.llm.queue(preparation())
    reading.post(session_id, "prepare")
    response = reading.post(session_id, "recall", {"text": "Sets and countability."})
    assert response.status_code == 200
    assert response.json() == {"ok": True}
    assert reading.post(session_id, "recall", {"text": "Again."}).status_code == 409
    assert reading.post(session_id, "recall", {"text": ""}).status_code == 422
    with ReadingStore(reading.store.db_path).connect() as conn:
        text = conn.execute(
            "SELECT recall_text FROM reading_sessions WHERE session_id=?", (session_id,)
        ).fetchone()[0]
    assert text == "Sets and countability."


# Summary scoring


def test_a_unit_counts_only_when_both_passes_agree(reading, caplog):
    session_id, _ = reading.prepared()
    reading.post(session_id, "recall", {"text": "Sets."})
    reading.llm.queue({"covered": [1, 2, 3]}, {"covered": [2, 3, 4]})
    with caplog.at_level(logging.INFO, logger="deeptutor.services.reading.service"):
        response = reading.post(
            session_id, "summary", {"text": "My summary of the section.", "judgement": 3}
        )
    assert response.status_code == 200, response.text
    body = response.json()
    assert body["covered"] == 2
    assert body["missed"] == [
        {"unit": f"SECRET-UNIT-{i}", "chunk_id": SECTION[i % 4]} for i in (1, 4, 5, 6)
    ]
    assert "disagree" in caplog.text and "[1, 4]" in caplog.text
    # Both passes saw the summary and every unit.
    scoring = reading.llm.prompts[1:]
    assert len(scoring) == 2
    assert all("My summary of the section." in p and "SECRET-UNIT-6" in p for p in scoring)
    with ReadingStore(reading.store.db_path).connect() as conn:
        row = conn.execute(
            "SELECT summary_text, summary_judgement, scoring_json FROM reading_sessions"
        ).fetchone()
    assert row[0] == "My summary of the section." and row[1] == 3
    assert json.loads(row[2])["disagreements"] == [1, 4]


def test_summary_rules(reading):
    session_id, _ = reading.prepared()
    body = {"text": "Summary.", "judgement": 2}
    assert reading.post(session_id, "summary", body).status_code == 409  # recall first
    reading.post(session_id, "recall", {"text": "Sets."})
    assert reading.post(session_id, "summary", {"text": "S", "judgement": 0}).status_code == 422
    reading.llm.queue({"covered": [1]}, {"covered": [1]})
    assert reading.post(session_id, "summary", body).status_code == 200
    assert reading.post(session_id, "summary", body).status_code == 409  # scored once


def test_an_invalid_scoring_pass_is_retried_then_fails(reading):
    session_id, _ = reading.prepared()
    reading.post(session_id, "recall", {"text": "Sets."})
    # Pass one answers badly twice (out-of-range unit, then prose); pass two is fine.
    reading.llm.by_pass[1].extend([{"covered": [9]}, "prose"])
    reading.llm.by_pass[2].append({"covered": [1]})
    response = reading.post(session_id, "summary", {"text": "Summary.", "judgement": 2})
    assert response.status_code == 502
    assert len(reading.llm.prompts) == 4  # preparation, pass one twice, pass two once
    # Nothing stored: the learner can submit again.
    reading.llm.queue({"covered": [1]}, {"covered": [1]})
    assert (
        reading.post(session_id, "summary", {"text": "Summary.", "judgement": 2}).json()["covered"]
        == 1
    )


def test_a_scoring_pass_is_retried_once(reading):
    session_id, _ = reading.prepared()
    reading.post(session_id, "recall", {"text": "Sets."})
    reading.llm.queue({"covered": [1, 1]}, {"covered": [1]}, {"covered": [1]})
    response = reading.post(session_id, "summary", {"text": "Summary.", "judgement": 2})
    assert response.status_code == 200, response.text
    assert response.json()["covered"] == 1


# Atoms


def test_accepted_atoms_become_practice_cards_sourced_to_the_chunk(reading):
    session_id, prep = reading.summarised()
    response = reading.post(session_id, "atoms/accept", {"ids": ["a1", "a2"]})
    assert response.status_code == 200, response.text
    entry_ids = response.json()["entry_ids"]
    assert len(entry_ids) == 2

    with ReadingStore(reading.store.db_path).connect() as conn:
        rows = {
            r["id"]: dict(r)
            for r in conn.execute(
                "SELECT * FROM notebook_entries WHERE id IN (?, ?)", entry_ids
            ).fetchall()
        }
        tags = {
            entry_id: {
                r[0]
                for r in conn.execute(
                    """SELECT c.name FROM notebook_categories c
                    JOIN notebook_entry_categories ec ON ec.category_id=c.id
                    WHERE ec.entry_id=?""",
                    (entry_id,),
                )
            }
            for entry_id in entry_ids
        }
    definition, proof = rows[entry_ids[0]], rows[entry_ids[1]]
    assert definition["question"] == "Define a countable set."
    assert definition["correct_answer"] == "A set in bijection with a subset of N."
    assert definition["section_id"] == "k0"
    assert definition["material_title"] == "Analysis I"
    assert "[c:k0]" in definition["explanation"] and "p. 1-2" in definition["explanation"]
    assert tags[entry_ids[0]] == {"chunk:k0"}
    # A proof atom is marked provisional (CORPUS.md: proofs do not feed FSRS or mastery).
    assert tags[entry_ids[1]] == {"chunk:k3", "provisional"}
    assert proof["section_id"] == "k3"

    # The definition is in the practice queue; the provisional proof is not (C11 amendment, C12).
    queue = PracticeStore(reading.store.db_path).queue("UTC", now=definition["created_at"] + 1)
    assert entry_ids[0] in queue and entry_ids[1] not in queue

    # Accepting again is idempotent.
    again = reading.post(session_id, "atoms/accept", {"ids": ["a2", "a1"]})
    assert again.json()["entry_ids"] == [entry_ids[1], entry_ids[0]]


def test_atom_accept_rules(reading):
    session_id, _ = reading.prepared()
    body = {"ids": ["a1"]}
    assert reading.post(session_id, "atoms/accept", body).status_code == 409  # before the summary
    reading.post(session_id, "recall", {"text": "Sets."})
    reading.llm.queue({"covered": []}, {"covered": []})
    reading.post(session_id, "summary", {"text": "Summary.", "judgement": 1})
    assert reading.post(session_id, "atoms/accept", {"ids": ["a1", "a7"]}).status_code == 404
    assert reading.post(session_id, "atoms/accept", {"ids": []}).status_code == 422
    with ReadingStore(reading.store.db_path).connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM notebook_entries").fetchone()[0] == 0

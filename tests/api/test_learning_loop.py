"""Learning commands, sessions and attempts (contract C12)."""

import asyncio
import json
import threading
import uuid

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import learning
from deeptutor.services.practice import scheduler
from deeptutor.services.practice.loop import LearningLoop
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore

HINTS = ["Count on from 2.", "2 + 1 = 3, then one more.", "3 + 1 = 4.", "2, then 3, then 4."]
SECRETS = ("Two and two make four.", "Counts the first 2 as a step.", *HINTS)


@pytest.fixture
def env(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "loop.db")
    monkeypatch.setattr(learning, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(learning.router, prefix="/learning")
    with TestClient(app) as client:
        yield store, PracticeStore(store.db_path), client


def seed(store, question_id="q1", **changes):
    async def run():
        await store.ensure_notebook_session("lesson", "Lesson")
        item = {
            "question_id": question_id,
            "question": "2 + 2?",
            "question_type": "single_choice",
            "options": {"A": "3", "B": "4"},
            "correct_answer": "B",
            "explanation": "Two and two make four.",
            "misconception": "Counts the first 2 as a step.",
            "hints": HINTS,
            "result": "ungraded",
            **changes,
        }
        await store.upsert_notebook_entries("lesson", [item])
        return (await store.find_notebook_entry("lesson", question_id))["id"]

    return asyncio.run(run())


class Learner:
    """Sends commands the way the desktop client does: fresh ids, the last version seen."""

    def __init__(self, client, source="ui"):
        self.client, self.source = client, source
        self.session_id, self.version, self.attempt_id = None, None, None

    def send(self, kind, payload=None, *, attempt=True, source=None, command_id=None, version=...):
        body = {
            "schema_version": 1,
            "command_id": command_id or str(uuid.uuid4()),
            "session_id": self.session_id,
            "attempt_id": self.attempt_id if attempt else None,
            "expected_version": self.version if version is ... else version,
            "kind": kind,
            "payload": payload or {},
        }
        response = self.client.post(
            "/learning/commands", json=body, headers={"X-Tutor-Source": source or self.source}
        )
        if response.status_code == 200:
            result = response.json()
            self.version = result["aggregate_version"]
            self.session_id = result["public_state"]["session"]["session_id"]
            if result["public_state"]["attempt"]:
                self.attempt_id = result["public_state"]["attempt"]["attempt_id"]
        return response

    def ok(self, kind, payload=None, **kw):
        response = self.send(kind, payload, **kw)
        assert response.status_code == 200, response.text
        return response.json()

    def start(self, mode="practice"):
        self.session_id, self.version = None, None
        return self.ok("start_session", {"mode": mode}, attempt=False)

    def open(self, entry_id):
        self.attempt_id = None
        return self.ok("open_item", {"entry_id": entry_id}, attempt=False)

    def answer(self, text, confidence=3):
        self.ok("commit_response", {"response": text, "input_mode": "typed"})
        self.ok("record_confidence", {"level": confidence})
        return self.ok("check")


def reviews(repo):
    with repo.connect() as conn:
        return [
            dict(row)
            for row in conn.execute(
                """SELECT request_id, entry_id, rating,
                COALESCE(json_extract(outcome_json, '$.provenance'), 'legacy') AS provenance
                FROM practice_review_events ORDER BY rowid"""
            )
        ]


def error(response):
    return response.status_code, response.json()["error"]["code"]


def test_correct_first_response_closes_releases_and_schedules_good(env):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    opened = me.open(entry)
    item = opened["public_state"]["attempt"]["item"]
    assert (item["prompt"], item["options"], item["rungs_available"]) == (
        "2 + 2?",
        {"A": "3", "B": "4"},
        4,
    )
    result = me.answer("B")
    attempt = result["public_state"]["attempt"]
    assert attempt["status"] == "closed" and attempt["closure"]["reason"] == "correct"
    assert attempt["checks"][0]["verdict"] == "correct"
    assert attempt["released"]["correct_answer"] == "B"
    assert attempt["measurement_eligibility"] == {"eligible": True, "reasons": []}
    disposition = attempt["closure"]["scheduler_disposition"]
    assert (disposition["rating"], disposition["provenance"]) == ("good", "verified")
    assert reviews(repo) == [
        {
            "request_id": f"ev:{me.attempt_id}",
            "entry_id": entry,
            "rating": "good",
            "provenance": "verified",
        }
    ]
    events = client.get(f"/learning/sessions/{me.session_id}").json()["events"]
    assert [e["seq"] for e in events] == list(range(1, len(events) + 1))
    assert [e["kind"] for e in events] == [
        "session_started",
        "item_opened",
        "response_committed",
        "confidence_recorded",
        "response_checked",
        "attempt_closed",
        "answer_released",
    ]


def test_nothing_private_leaves_before_release(env):
    store, _, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    seen = [me.open(entry)]
    seen.append(me.ok("commit_response", {"response": "A", "input_mode": "typed"}))
    seen.append(me.ok("record_confidence", {"level": 4}))
    seen.append(me.ok("check"))  # incorrect: remediating, nothing released
    assert seen[-1]["public_state"]["attempt"]["status"] == "remediating"
    seen.append(client.get(f"/learning/sessions/{me.session_id}").json())
    seen.append(client.get("/learning/queue").json())
    text = json.dumps(seen)
    for secret in SECRETS:
        assert secret not in text
    assert '"correct_answer"' not in text
    hinted = me.ok("request_assistance", {"kind": "hint"})
    assert hinted["public_state"]["attempt"]["assistance"][-1]["text"] == HINTS[0]
    assert HINTS[1] not in json.dumps(hinted)


def test_confidence_comes_after_the_sealed_response_and_before_the_check(env):
    store, _, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    assert error(me.send("record_confidence", {"level": 2})) == (409, "invalid_transition")
    assert error(me.send("check")) == (409, "invalid_transition")
    me.ok("commit_response", {"response": "B", "input_mode": "typed"})
    assert error(me.send("commit_response", {"response": "A", "input_mode": "typed"})) == (
        409,
        "invalid_transition",
    )  # sealed
    assert error(me.send("check")) == (409, "invalid_transition")
    assert error(me.send("record_confidence", {"level": None})) == (422, "invalid_input")
    assert error(me.send("record_confidence", {"level": 5})) == (422, "invalid_input")
    skipped = me.ok("record_confidence", {"level": None, "skipped_reason": "unsure what to pick"})
    assert (
        skipped["public_state"]["attempt"]["confidence"]["skipped_reason"] == "unsure what to pick"
    )
    assert me.ok("check")["public_state"]["attempt"]["first_response"]["response"] == "B"


@pytest.mark.parametrize(
    ("hints", "answer", "rating", "provenance", "eligible"),
    [
        (0, "B", "good", "verified", True),
        (1, "B", "again", "assisted", False),
        (3, "B", "again", "assisted", False),
        (0, "A", "again", "verified", True),
        (2, "A", "again", "assisted", False),
    ],
)
def test_review_policy_2_gives_no_credit_for_assisted_starts(
    env, hints, answer, rating, provenance, eligible
):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    for _ in range(hints):
        me.ok("request_assistance", {"kind": "hint"})
    me.answer(answer)
    final = me.ok("give_up") if answer == "A" else None
    attempt = final or client.get(f"/learning/sessions/{me.session_id}").json()
    attempt = attempt["public_state"]["attempt"] if final else attempt["attempts"][0]
    disposition = attempt["closure"]["scheduler_disposition"]
    assert (disposition["rating"], disposition["provenance"]) == (rating, provenance)
    assert attempt["first_response"]["assisted_before_commit"] == bool(hints)
    assert attempt["measurement_eligibility"]["eligible"] is eligible
    assert [(r["rating"], r["provenance"]) for r in reviews(repo)] == [(rating, provenance)]


def test_repairs_never_rewrite_the_first_outcome_and_the_offer_follows_two_misses(env):
    store, repo, client = env
    entry = seed(store, options={"A": "3", "B": "4", "C": "5", "D": "6"})
    me = Learner(client)
    me.start()
    me.open(entry)
    me.answer("A")
    me.ok("request_assistance", {"kind": "hint"})
    me.ok("submit_repair", {"response": "C", "input_mode": "typed"})
    missed = me.ok("submit_repair", {"response": "D", "input_mode": "typed"})
    attempt = missed["public_state"]["attempt"]
    assert attempt["status"] == "remediating"
    assert attempt["offer"] == "worked_example_then_fresh_problem"
    assert attempt["released"] is None
    fixed = me.ok("submit_repair", {"response": "B", "input_mode": "typed"})["public_state"][
        "attempt"
    ]
    assert fixed["closure"]["reason"] == "correct"
    assert [c["verdict"] for c in fixed["checks"]] == [
        "incorrect",
        "incorrect",
        "incorrect",
        "correct",
    ]
    assert fixed["first_response"]["response"] == "A"
    assert fixed["closure"]["scheduler_disposition"]["rating"] == "again"
    assert [r["rating"] for r in reviews(repo)] == ["again"]  # one review, from the first outcome


def test_dont_know_and_give_up_are_failed_retrievals_not_errors(env):
    store, repo, client = env
    first, second = seed(store), seed(store, question_id="q2")
    me = Learner(client)
    me.start()
    me.open(first)
    me.ok("commit_response", {"response": "", "input_mode": "dont_know"})
    me.ok("record_confidence", {"level": 1})
    checked = me.ok("check")["public_state"]["attempt"]
    assert checked["status"] == "remediating" and checked["checks"][0]["checker_id"] == "dont_know"
    closed = me.ok("give_up")["public_state"]["attempt"]
    assert closed["closure"]["reason"] == "revealed" and closed["released"]["correct_answer"] == "B"
    assert closed["measurement_eligibility"]["eligible"] is True  # an unaided failure is evidence
    me.open(second)
    early = me.ok("give_up")["public_state"]["attempt"]
    assert early["first_response"]["input_mode"] == "dont_know"
    assert early["closure"]["scheduler_disposition"]["rating"] == "again"
    assert [(r["entry_id"], r["rating"], r["provenance"]) for r in reviews(repo)] == [
        (first, "again", "verified"),
        (second, "again", "verified"),
    ]


def test_free_response_is_judged_by_the_learner_on_screen_and_labelled(env):
    store, repo, client = env
    entry = seed(
        store, question_type="short_answer", options={}, correct_answer="Light to sugar.", hints=[]
    )
    me = Learner(client)
    me.start()
    me.open(entry)
    waiting = me.answer("Plants store light as sugar.")["public_state"]["attempt"]
    assert waiting["status"] == "awaiting_self_assessment"
    assert waiting["checks"][0]["verdict"] == "unverifiable"
    assert waiting["released"]["correct_answer"] == "Light to sugar."
    assert error(me.send("self_assess", {"judged": "correct"}, source="voice_tool")) == (
        403,
        "unauthorized",
    )
    closed = me.ok("self_assess", {"judged": "correct"})["public_state"]["attempt"]
    assert closed["closure"]["reason"] == "self_assessed"
    assert closed["measurement_eligibility"] == {"eligible": False, "reasons": ["unverified"]}
    assert [(r["rating"], r["provenance"]) for r in reviews(repo)] == [("good", "self_report")]


def test_a_broken_key_quarantines_the_item_and_never_scores_the_learner(env):
    store, repo, client = env
    broken = seed(store, correct_answer="Z")
    me = Learner(client)
    me.start()
    me.open(broken)
    closed = me.answer("B")["public_state"]["attempt"]
    assert closed["checks"][0]["verdict"] == "invalid_item"
    assert closed["closure"]["reason"] == "invalidated" and closed["released"] is None
    assert reviews(repo) == []
    assert client.get("/learning/queue").json()["items"] == []
    assert error(me.send("open_item", {"entry_id": broken}, attempt=False)) == (
        409,
        "invalid_transition",
    )
    good = seed(store, question_id="q2")
    me.open(good)
    me.answer("A")  # remediating, Again pending
    me.ok("invalidate_item", {"reason": "The options are ambiguous"})
    assert reviews(repo) == []  # a flagged item is not scored against the learner


def test_command_ids_are_idempotent_and_versions_guard_against_stale_writes(env):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    me.ok("commit_response", {"response": "B", "input_mode": "typed"})
    me.ok("record_confidence", {"level": 3})
    command_id = str(uuid.uuid4())
    version = me.version
    first = me.ok("check", command_id=command_id)
    replay = me.send("check", command_id=command_id, version=version)  # a network retry
    assert replay.status_code == 200 and replay.json() == first
    reused = me.send("abandon", command_id=command_id, version=version)
    assert error(reused) == (422, "invalid_input")
    stale = me.send("close_session", version=version)
    assert stale.status_code == 409
    assert stale.json()["error"] == {
        "code": "conflict",
        "message": "The session changed since you last read it. Reload it and try again.",
        "retryable": True,
        "current_version": version + 1,
    }
    state = client.get(f"/learning/sessions/{me.session_id}").json()
    assert state["session"]["status"] == "active" and state["session"]["version"] == version + 1
    assert len(reviews(repo)) == 1


def test_concurrent_commands_on_one_version_let_exactly_one_through(env):
    store, _, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    version, results = me.version, []

    def hint():
        results.append(me.send("request_assistance", {"kind": "hint"}, version=version).status_code)

    threads = [threading.Thread(target=hint) for _ in range(6)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert sorted(results) == [200] + [409] * 5
    attempt = client.get(f"/learning/sessions/{me.session_id}").json()["attempts"][0]
    assert [a["rung"] for a in attempt["assistance"]] == [1]


def test_a_crash_inside_the_review_rolls_back_the_whole_command(env, monkeypatch):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    me.ok("commit_response", {"response": "B", "input_mode": "typed"})
    me.ok("record_confidence", {"level": 3})
    before = client.get(f"/learning/sessions/{me.session_id}").json()
    real = scheduler.schedule

    def crash(*args, **kwargs):
        raise RuntimeError("power cut")

    monkeypatch.setattr("deeptutor.services.practice.storage.schedule", crash)
    command_id = str(uuid.uuid4())
    with pytest.raises(RuntimeError):
        me.send("check", command_id=command_id)
    assert client.get(f"/learning/sessions/{me.session_id}").json() == before
    assert reviews(repo) == []
    monkeypatch.setattr("deeptutor.services.practice.storage.schedule", real)
    done = me.ok("check", command_id=command_id)
    assert done["public_state"]["attempt"]["closure"]["reason"] == "correct"
    assert len(reviews(repo)) == 1


def test_events_replay_to_the_stored_outcomes_and_fsrs_state(env):
    store, repo, client = env
    entries = [seed(store, question_id=f"q{i}") for i in range(4)]
    me = Learner(client)
    me.start()
    plans = [("B", 0), ("A", 0), ("B", 2), (None, 0)]
    for entry, (answer, hints) in zip(entries, plans, strict=True):
        me.open(entry)
        for _ in range(hints):
            me.ok("request_assistance", {"kind": "hint"})
        if answer is not None:
            me.answer(answer)
        if answer != "B":
            me.ok("give_up")
    me.ok("close_session", attempt=False)
    state = client.get(f"/learning/sessions/{me.session_id}").json()
    closed = {
        e["attempt_id"]: e["payload"] for e in state["events"] if e["kind"] == "attempt_closed"
    }
    by_attempt = {r["request_id"][3:]: r for r in reviews(repo)}
    assert set(closed) == set(by_attempt) == {a["attempt_id"] for a in state["attempts"]}
    for attempt_id, payload in closed.items():
        disposition = payload["scheduler_disposition"]
        assert by_attempt[attempt_id]["rating"] == disposition["rating"]
        assert by_attempt[attempt_id]["provenance"] == disposition["provenance"]
    with repo.connect() as conn:
        for entry in entries:
            history = conn.execute(
                "SELECT reviewed_at, rating FROM practice_review_events WHERE entry_id=? ORDER BY rowid",
                (entry,),
            ).fetchall()
            stored = conn.execute(
                "SELECT fsrs_stability, fsrs_difficulty, fsrs_state, fsrs_step FROM practice_review_state WHERE entry_id=?",
                (entry,),
            ).fetchone()
            assert scheduler.replay((h["reviewed_at"], h["rating"]) for h in history) == dict(
                stored
            )
        with pytest.raises(Exception, match="append-only"):
            conn.execute("UPDATE learning_events SET kind='x'")


def test_voice_tools_propose_and_the_learner_confirms_on_screen(env):
    store, _, client = env
    entry = seed(store, question_type="fill_blank", options={}, correct_answer="", hints=[])
    tool, me = Learner(client, "voice_tool"), Learner(client)
    me.start()
    me.open(entry)
    tool.session_id, tool.attempt_id, tool.version = me.session_id, me.attempt_id, me.version
    spoken = {"response": "four", "input_mode": "spoken"}
    assert error(tool.send("commit_response", spoken)) == (403, "unauthorized")
    assert error(tool.send("commit_response", {"response": "", "input_mode": "dont_know"})) == (
        403,
        "unauthorized",
    )
    proposed = tool.ok("propose_response", {"raw_transcript": "for", "input_mode": "spoken"})
    assert proposed["public_state"]["attempt"]["pending_proposal"]["raw_transcript"] == "for"
    me.version = tool.version
    assert error(me.send("commit_response", spoken)) == (422, "invalid_input")  # no confirmation
    confirmation = {"raw_transcript": "for", "confirmed_text": "four"}
    committed = me.ok("commit_response", {**spoken, "transcript_confirmation": confirmation})
    first = committed["public_state"]["attempt"]["first_response"]
    assert first["transcript_confirmation"] == confirmation
    assert committed["public_state"]["attempt"]["pending_proposal"] is None
    events = client.get(f"/learning/sessions/{me.session_id}").json()["events"]
    assert [e["source"] for e in events if e["kind"] == "response_proposed"] == ["voice_tool"]
    missing = client.post("/learning/commands", json={}, headers={})
    assert error(missing) == (422, "invalid_input")


def test_assessment_buffers_feedback_until_the_session_closes(env):
    store, repo, client = env
    first, second = seed(store), seed(store, question_id="q2")
    essay = seed(store, question_id="q3", question_type="short_answer", options={}, hints=[])
    me = Learner(client)
    me.start("assessment")
    assert error(me.send("open_item", {"entry_id": essay}, attempt=False)) == (422, "invalid_input")
    me.open(first)
    assert error(me.send("request_assistance", {"kind": "hint"})) == (409, "invalid_transition")
    submitted = me.answer("A")
    attempt = submitted["public_state"]["attempt"]
    assert attempt["status"] == "submitted"
    assert attempt["checks"][0]["verdict"] == "recorded" and attempt["released"] is None
    assert all(e["payload"].get("verdict") in (None, "recorded") for e in submitted["events"])
    me.open(second)
    me.answer("B")
    assert reviews(repo) == []
    closed = me.ok("close_session", attempt=False)
    state = client.get(f"/learning/sessions/{me.session_id}").json()
    assert closed["public_state"]["session"]["status"] == "closed"
    outcomes = {
        a["entry_id"]: (a["closure"]["reason"], a["checks"][0]["verdict"])
        for a in state["attempts"]
    }
    assert outcomes == {first: ("revealed", "incorrect"), second: ("correct", "correct")}
    assert sorted(r["rating"] for r in reviews(repo)) == ["again", "good"]
    assert error(me.send("open_item", {"entry_id": first}, attempt=False)) == (
        409,
        "invalid_transition",
    )


def test_one_live_item_per_session_and_edits_mid_attempt_conflict(env):
    store, repo, client = env
    entry, other = seed(store), seed(store, question_id="q2")
    me = Learner(client)
    me.start()
    me.open(entry)
    assert error(me.send("open_item", {"entry_id": other}, attempt=False)) == (
        409,
        "invalid_transition",
    )
    me.ok("commit_response", {"response": "B", "input_mode": "typed"})
    me.ok("record_confidence", {"level": 2})
    with repo.connect() as conn:
        conn.execute("UPDATE notebook_entries SET correct_answer='A' WHERE id=?", (entry,))
    assert error(me.send("check")) == (409, "conflict")
    abandoned = me.ok("abandon")["public_state"]["attempt"]
    assert abandoned["closure"]["reason"] == "abandoned"
    assert abandoned["closure"]["scheduler_disposition"]["eligible_for_fsrs"] is False
    assert abandoned["released"] is None  # abandoning never shows the answer
    assert reviews(repo) == []  # an unchecked response is not a wrong answer


def test_peeking_at_hints_then_abandoning_records_an_assisted_failure(env):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    me.ok("request_assistance", {"kind": "hint"})
    closed = me.ok("abandon")["public_state"]["attempt"]
    assert closed["released"] is None and closed["first_response"]["input_mode"] == "dont_know"
    assert [(r["rating"], r["provenance"]) for r in reviews(repo)] == [("again", "assisted")]
    me.open(entry)
    closed = me.answer("B")["public_state"]["attempt"]
    assert closed["measurement_eligibility"]["reasons"] == ["exposed_within_24h"]


def test_a_retry_soon_after_the_answer_is_relearning_not_evidence(env):
    store, repo, client = env
    entry = seed(store)
    me = Learner(client)
    me.start()
    me.open(entry)
    me.ok("give_up")
    again = me.open(entry)["public_state"]["attempt"]
    assert again["status"] == "open"
    closed = me.answer("B")["public_state"]["attempt"]
    assert closed["closure"]["scheduler_disposition"]["rating"] == "good"  # FSRS relearning step
    assert closed["measurement_eligibility"] == {
        "eligible": False,
        "reasons": ["exposed_within_24h"],
    }


def test_close_session_applies_known_outcomes_only(env):
    store, repo, client = env
    missed, pending = seed(store), seed(store, question_id="q2")
    me = Learner(client)
    me.start()
    me.open(missed)
    me.answer("A")  # remediating
    me.ok("abandon")
    me.open(pending)
    me.ok("commit_response", {"response": "B", "input_mode": "typed"})
    me.ok("close_session", attempt=False)
    assert [(r["entry_id"], r["rating"]) for r in reviews(repo)] == [(missed, "again")]


def test_public_queue_carries_no_keys_and_filters_by_tag(env):
    store, repo, client = env
    plain = seed(store)
    spanish = seed(store, question_id="es1", question="gato", options={}, correct_answer="cat")
    provisional = seed(store, question_id="p1")
    with repo.connect() as conn:
        for entry, tag in ((spanish, "lang:es"), (provisional, "provisional")):
            conn.execute(
                "INSERT OR IGNORE INTO notebook_categories(name, created_at) VALUES(?, 0)", (tag,)
            )
            conn.execute(
                """INSERT INTO notebook_entry_categories(entry_id, category_id)
                SELECT ?, id FROM notebook_categories WHERE name=?""",
                (entry, tag),
            )
    items = client.get("/learning/queue").json()["items"]
    assert {i["entry_id"] for i in items} == {plain, spanish}
    assert set(items[0]) == {
        "entry_id",
        "prompt",
        "question_type",
        "options",
        "lang",
        "tags",
        "due_at",
    }
    only = client.get("/learning/queue?tag=LANG:ES").json()["items"]
    assert [(i["entry_id"], i["lang"]) for i in only] == [(spanish, "es")]
    assert repo.queue("UTC") == [plain, spanish]  # the C7 queue skips provisional too


def test_loop_tables_initialize_idempotently_on_legacy_databases(tmp_path):
    path = tmp_path / "old.db"
    store = SQLiteSessionStore(path)
    repo = PracticeStore(path)
    entry = seed(store)
    with repo.connect() as conn:  # a review written before provenance existed
        conn.execute(
            "INSERT INTO practice_review_events VALUES('old_request_0001', ?, 'good', 'B', 1, '{}')",
            (entry,),
        )
    for _ in range(2):
        SQLiteSessionStore(path)
    assert [r["provenance"] for r in reviews(repo)] == ["legacy"]
    result = LearningLoop(repo).execute(
        {
            "schema_version": 1,
            "command_id": str(uuid.uuid4()),
            "kind": "start_session",
            "expected_version": None,
            "payload": {"mode": "practice"},
        },
        "ui",
    )
    assert result["aggregate_version"] == 1

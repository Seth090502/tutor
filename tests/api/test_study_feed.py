"""Read-only feed and candidates for the Rust projections (contract C13)."""

from datetime import datetime, timezone
import json
import time
import uuid

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import learning
from deeptutor.services.practice import feed as study_feed
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore
from tests.api.test_learning_loop import HINTS, Learner, seed

KEY = "zebra-quartz-91"
SECRETS = (KEY, "Two and two make four.", "Counts the first 2 as a step.", *HINTS)
ITEM = {"question_type": "short_answer", "options": {}, "correct_answer": KEY}
# A language card has a checker for typed answers; a short-answer item is self-assessed instead.
CARD = {"question_type": "translation", "options": {}, "correct_answer": KEY}


@pytest.fixture
def env(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "feed.db")
    monkeypatch.setattr(learning, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(learning.router, prefix="/learning")
    with TestClient(app) as client:
        yield store, PracticeStore(store.db_path), client


def tag(repo, entry, *names):
    with repo.connect() as conn:
        for name in names:
            conn.execute(
                "INSERT OR IGNORE INTO notebook_categories(name, created_at) VALUES(?, 0)", (name,)
            )
            conn.execute(
                """INSERT INTO notebook_entry_categories(entry_id, category_id)
                SELECT ?, id FROM notebook_categories WHERE name=?""",
                (entry, name),
            )


def card(store, repo, question_id="c1"):
    entry = seed(store, question_id=question_id, **CARD)
    tag(repo, entry, "lang:es")
    return entry


def answer_good(client, entry):
    me = Learner(client)
    me.start()
    me.open(entry)
    me.answer(KEY)
    return me


def test_store_id_is_stable_across_restarts_and_distinct_between_databases(tmp_path):
    first = tmp_path / "a.db"
    ids = []
    for _ in range(3):
        SQLiteSessionStore(first)
        ids.append(study_feed.feed(PracticeStore(first))["store_id"])
    SQLiteSessionStore(tmp_path / "b.db")
    other = study_feed.feed(PracticeStore(tmp_path / "b.db"))["store_id"]
    assert len(set(ids)) == 1 and uuid.UUID(ids[0])
    assert other != ids[0]


def test_feed_pages_return_every_event_once_in_order(env):
    store, repo, client = env
    for n in range(3):
        answer_good(client, card(store, repo, f"q{n}"))
    everything = client.get("/learning/feed").json()
    assert everything["schema_version"] == 1
    assert set(everything["events"][0]) == {
        "event_seq",
        "session_id",
        "seq",
        "attempt_id",
        "kind",
        "recorded_at",
        "source",
        "payload",
    }
    seqs = [e["event_seq"] for e in everything["events"]]
    assert len(seqs) > 7 and seqs == sorted(set(seqs))
    assert everything["next_after"] == seqs[-1]

    paged, after = [], 0
    while True:
        page = client.get(f"/learning/feed?after_event_seq={after}&limit=3").json()
        assert len(page["events"]) <= 3
        if not page["events"]:
            assert page["next_after"] == after
            break
        paged += page["events"]
        after = page["next_after"]
    assert paged == everything["events"]
    assert client.get(f"/learning/feed?after_event_seq={seqs[-1]}").json()["events"] == []


def test_feed_payloads_never_carry_keys_explanations_or_hints(env):
    store, repo, client = env
    entry = card(store, repo)
    me = Learner(client)
    me.start()
    me.open(entry)
    me.answer("wrong")
    me.ok("submit_repair", {"response": "still wrong", "input_mode": "typed"})
    me.ok("give_up")  # reveals the answer to the learner, not to the feed
    text = json.dumps(client.get("/learning/feed").json(), ensure_ascii=False)
    kinds = {e["kind"] for e in json.loads(text)["events"]}
    assert {"item_opened", "response_checked", "attempt_closed"} <= kinds
    for secret in SECRETS:
        assert secret not in text


@pytest.mark.parametrize("query", ["limit=0", "limit=1001", "after_event_seq=-1"])
def test_feed_bounds_are_422(env, query):
    assert env[2].get(f"/learning/feed?{query}").status_code == 422
    assert env[2].get("/learning/feed?limit=1000").status_code == 200
    assert env[2].get("/learning/feed?limit=1").status_code == 200


def test_candidates_exclude_quarantined_and_provisional_and_carry_no_secrets(env):
    store, repo, client = env
    keep = seed(store, question_id="keep", **ITEM)
    quarantined = seed(store, question_id="bad", **ITEM)
    provisional = seed(store, question_id="prov", **ITEM)
    tag(repo, provisional, "provisional")
    with repo.connect() as conn:
        conn.execute(
            "INSERT INTO practice_item_status VALUES(?, 'quarantined', 'ambiguous', 0)",
            (quarantined,),
        )
    body = client.get("/learning/candidates").json()
    assert body["schema_version"] == 1 and uuid.UUID(body["store_id"])
    assert isinstance(body["now"], float)
    assert [i["entry_id"] for i in body["items"]] == [keep]
    assert set(body["items"][0]) == {
        "entry_id",
        "track",
        "question_type",
        "lang",
        "skill_ids",
        "family_id",
        "tags",
        "due_at",
        "retrievability",
        "stability",
        "difficulty",
        "review_count",
        "last_review_at",
        "fsrs_state",
        "estimated_seconds",
        "is_new",
    }
    text = json.dumps(body, ensure_ascii=False)
    for secret in SECRETS:
        assert secret not in text


def test_retrievability_is_null_before_review_then_falls_with_time(env):
    store, repo, client = env
    entry = card(store, repo)
    [fresh] = study_feed.candidates(repo)["items"]
    assert (fresh["retrievability"], fresh["is_new"], fresh["review_count"]) == (None, True, 0)
    assert fresh["estimated_seconds"] is None

    answer_good(client, entry)
    now = time.time()
    [soon] = study_feed.candidates(repo, now=now)["items"]
    assert 0 < soon["retrievability"] <= 1
    assert soon["is_new"] is False and soon["review_count"] == 1
    assert soon["fsrs_state"] is not None and soon["stability"] > 0
    [later] = study_feed.candidates(repo, now=now + 30 * 86400)["items"]
    assert 0 < later["retrievability"] < soon["retrievability"]
    assert later["last_review_at"] == soon["last_review_at"]
    assert datetime.fromtimestamp(later["last_review_at"], timezone.utc).year >= 2025


def test_track_rules(env):
    store, repo, client = env
    spanish = seed(store, question_id="es", **ITEM)
    tag(repo, spanish, "lang:es", "card:production", "Spanish")
    math = seed(store, question_id="m", **ITEM)
    tag(repo, math, "skill:frac.add", "family:f1", "chunk:c1", "Math", "algebra")
    special_only = seed(store, question_id="s", **ITEM)
    tag(repo, special_only, "skill:x", "card:recognition")
    untagged = seed(store, question_id="u", **ITEM)
    by_id = {i["entry_id"]: i for i in client.get("/learning/candidates").json()["items"]}
    assert by_id[spanish]["track"] == "lang:es" and by_id[spanish]["lang"] == "es"
    assert by_id[math]["track"] == "math"  # first ordinary tag by name (binary order), lowercased
    assert by_id[math]["skill_ids"] == ["frac.add"] and by_id[math]["family_id"] == "f1"
    assert by_id[special_only]["track"] == "general"
    assert by_id[untagged]["track"] == "general" and by_id[untagged]["tags"] == []


def test_feed_carries_only_the_hint_rung_the_learner_requested(env):
    store, repo, client = env
    me = Learner(client)
    me.start()
    me.open(card(store, repo))
    me.ok("request_assistance", {"kind": "hint"})
    text = json.dumps(client.get("/learning/feed").json(), ensure_ascii=False)
    assert HINTS[1] not in text and HINTS[2] not in text and HINTS[3] not in text
    assert KEY not in text

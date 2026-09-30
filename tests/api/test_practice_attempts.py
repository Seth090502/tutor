"""Practice attempts (contract C7): server-enforced hints, confidence and the rating policy.

Review policy 2 (C12): any hint before the check is an assisted, failed retrieval (Again); Hard is
never produced. The rows below changed from the policy-1 table (rung 1 Good, rungs 2-3 Hard).
"""

import asyncio
import re
import time

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import practice, question_notebook
from deeptutor.services.practice.attempts import attempt_rating
from deeptutor.services.practice.scheduler import DAY
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore

HINTS = ["Count on from 2.", "2 + 1 = 3, then one more.", "3 + 1 = 4.", "2, then 3, then 4."]
ATTEMPT_COLUMNS = {
    "attempt_id",
    "entry_id",
    "started_at",
    "confidence",
    "hint_rung_max",
    "revealed",
    "answer",
    "correct",
    "closed_at",
    "reviewed",
}


@pytest.fixture
def bank(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "practice.db")
    for module in (practice, question_notebook):
        monkeypatch.setattr(module, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(practice.router, prefix="/practice")
    with TestClient(app) as client:
        yield store, PracticeStore(store.db_path), client


def seed(store, **changes):
    async def run():
        await store.ensure_notebook_session("lesson", "Lesson")
        item = {
            "question_id": "q1",
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
        return await store.find_notebook_entry("lesson", item["question_id"])

    return asyncio.run(run())


def start(client, entry_id):
    response = client.post(f"/practice/questions/{entry_id}/attempts")
    assert response.status_code == 200, response.text
    return response.json()["attempt_id"]


def hint(client, attempt_id):
    return client.post(f"/practice/attempts/{attempt_id}/hint")


def check(client, attempt_id, answer="B"):
    return client.post(f"/practice/attempts/{attempt_id}/check", json={"answer": answer})


def review(client, entry_id, **payload):
    body = {"request_id": "review_request_0001", "version": 0, "answer": "B", **payload}
    return client.post(f"/practice/questions/{entry_id}/review", json=body)


def attempt_row(repo, attempt_id):
    with repo.connect() as conn:
        row = conn.execute(
            "SELECT * FROM practice_attempts WHERE attempt_id=?", (attempt_id,)
        ).fetchone()
    return dict(row)


@pytest.mark.parametrize(
    ("correct", "rung", "revealed", "rating"),
    [
        (False, 0, False, "again"),
        (False, 1, False, "again"),
        (False, 3, False, "again"),
        (True, 0, False, "good"),
        (True, 1, False, "again"),
        (True, 2, False, "again"),
        (True, 3, False, "again"),
        (True, 4, False, "again"),
        (True, 5, True, "again"),
        (True, 0, True, "again"),
        (True, 2, True, "again"),
    ],
)
def test_rating_policy_table(correct, rung, revealed, rating):
    assert attempt_rating(correct, rung, revealed) == rating


def test_attempt_happy_path_checks_and_returns_the_key_after_closing(bank):
    store, repo, client = bank
    entry = seed(store)
    created = client.post(f"/practice/questions/{entry['id']}/attempts").json()
    assert re.fullmatch(r"[0-9a-f]{32}", created["attempt_id"])
    assert created["rungs_available"] == 4
    attempt = created["attempt_id"]
    opened = attempt_row(repo, attempt)
    assert set(opened) == ATTEMPT_COLUMNS
    assert (opened["entry_id"], opened["hint_rung_max"], opened["revealed"]) == (entry["id"], 0, 0)
    assert opened["closed_at"] is None and opened["confidence"] is None
    confidence = client.post(f"/practice/attempts/{attempt}/confidence", json={"level": 3})
    assert (confidence.status_code, confidence.json()) == (200, {"ok": True})
    checked = check(client, attempt)
    assert checked.status_code == 200
    assert checked.json() == {
        "correct": True,
        "rating": "good",
        "correct_answer": "B",
        "explanation": "Two and two make four.",
        "misconception": "Counts the first 2 as a step.",
    }
    closed = attempt_row(repo, attempt)
    assert (closed["confidence"], closed["answer"], closed["correct"]) == (3, "B", 1)
    assert closed["closed_at"] is not None and closed["reviewed"] == 0


def test_hint_ladder_is_sequential_and_rung_five_reveals_the_answer(bank):
    store, repo, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    for rung, text in enumerate(HINTS, 1):
        response = hint(client, attempt)
        assert response.status_code == 200
        assert response.json() == {"rung": rung, "text": text, "is_answer": False}
        assert attempt_row(repo, attempt)["revealed"] == 0
    assert hint(client, attempt).json() == {"rung": 5, "text": "B", "is_answer": True}
    row = attempt_row(repo, attempt)
    assert (row["hint_rung_max"], row["revealed"]) == (5, 1)
    assert hint(client, attempt).status_code == 409  # no rung after the answer
    assert check(client, attempt).json()["rating"] == "again"


def test_rungs_without_prewritten_text_return_null(bank):
    store, _, client = bank
    entry = seed(store, hints=["Only a nudge."])
    created = client.post(f"/practice/questions/{entry['id']}/attempts").json()
    assert created["rungs_available"] == 1
    attempt = created["attempt_id"]
    assert hint(client, attempt).json() == {"rung": 1, "text": "Only a nudge.", "is_answer": False}
    for rung in (2, 3, 4):
        assert hint(client, attempt).json() == {"rung": rung, "text": None, "is_answer": False}
    assert hint(client, attempt).json() == {"rung": 5, "text": "B", "is_answer": True}
    bare = seed(store, question_id="q2", hints=None)
    assert client.post(f"/practice/questions/{bare['id']}/attempts").json()["rungs_available"] == 0


@pytest.mark.parametrize(
    ("hints", "answer", "rating"),
    [(0, "B", "good"), (1, "B", "again"), (2, "B", "again"), (3, "B", "again"), (4, "B", "again")]
    + [(0, "A", "again"), (2, "A", "again")],
)
def test_check_gives_good_only_without_hints(bank, hints, answer, rating):
    store, _, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    for _ in range(hints):
        hint(client, attempt)
    result = check(client, attempt, answer).json()
    assert (result["correct"], result["rating"]) == (answer == "B", rating)


def test_confidence_never_changes_the_rating(bank):
    store, _, client = bank
    entry = seed(store)
    ratings = set()
    for level in (1, 2, 3, 4):
        attempt = start(client, entry["id"])
        client.post(f"/practice/attempts/{attempt}/confidence", json={"level": level})
        ratings.add(check(client, attempt).json()["rating"])
    assert ratings == {"good"}


def test_confidence_hint_and_check_are_refused_after_the_check(bank):
    store, repo, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    for level in (0, 5, "high"):
        response = client.post(f"/practice/attempts/{attempt}/confidence", json={"level": level})
        assert response.status_code == 422
    assert client.post(f"/practice/attempts/{attempt}/confidence", json={"level": 1}).is_success
    assert client.post(f"/practice/attempts/{attempt}/confidence", json={"level": 2}).is_success
    assert check(client, attempt, "A").status_code == 200
    late = client.post(f"/practice/attempts/{attempt}/confidence", json={"level": 4})
    assert late.status_code == 409
    assert hint(client, attempt).status_code == 409
    assert check(client, attempt, "B").status_code == 409
    row = attempt_row(repo, attempt)
    assert (row["confidence"], row["answer"], row["correct"], row["hint_rung_max"]) == (
        2,
        "A",
        0,
        0,
    )


def test_unknown_malformed_and_hidden_attempts(bank):
    store, repo, client = bank
    entry = seed(store)
    assert client.post("/practice/questions/999/attempts").status_code == 404
    unknown = "0" * 32
    assert hint(client, unknown).status_code == 404
    assert check(client, unknown).status_code == 404
    assert (
        client.post(f"/practice/attempts/{unknown}/confidence", json={"level": 2}).status_code
        == 404
    )
    assert hint(client, "not-an-attempt").status_code == 422
    attempt = start(client, entry["id"])
    with repo.connect() as conn:  # the question's conversation was deleted
        conn.execute("UPDATE sessions SET deleted_at=1 WHERE id='lesson'")
    assert hint(client, attempt).status_code == 404
    assert check(client, attempt).status_code == 404
    assert client.post(f"/practice/questions/{entry['id']}/attempts").status_code == 404


def test_review_via_attempt_uses_the_server_rating_and_existing_fsrs_path(bank):
    store, repo, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    hint(client, attempt)
    hint(client, attempt)
    check(client, attempt)
    response = review(client, entry["id"], attempt_id=attempt)
    assert response.status_code == 200, response.text
    outcome = response.json()
    assert (outcome["rating"], outcome["correct"], outcome["self_report"]) == ("again", True, False)
    assert outcome["provenance"] == "legacy_attempt"
    assert outcome["attempt_id"] == attempt
    assert repo.state(entry["id"])["review_count"] == 1
    with repo.connect() as conn:
        event = conn.execute("SELECT rating, answer FROM practice_review_events").fetchone()
    assert tuple(event) == ("again", "B")
    assert attempt_row(repo, attempt)["reviewed"] == 1


def test_review_via_attempt_rejects_a_client_rating_and_open_attempts(bank):
    store, repo, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    assert review(client, entry["id"], attempt_id=attempt).status_code == 409  # not checked
    check(client, attempt)
    for rating in ("good", "again"):
        assert review(client, entry["id"], attempt_id=attempt, rating=rating).status_code == 422
    assert review(client, entry["id"], attempt_id=attempt, answer="A").status_code == 422
    assert (
        review(client, entry["id"], attempt_id=attempt, judged_correct=True).status_code == 422
    )  # the check already graded it
    assert review(client, entry["id"], attempt_id="1" * 32).status_code == 404
    other = seed(store, question_id="q2")
    assert review(client, other["id"], attempt_id=attempt).status_code == 404
    assert repo.state(entry["id"])["review_count"] == 0
    assert attempt_row(repo, attempt)["reviewed"] == 0


def test_an_attempt_is_reviewed_at_most_once_and_retries_are_idempotent(bank):
    store, repo, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    check(client, attempt, "A")
    first = review(client, entry["id"], attempt_id=attempt, answer="A")
    assert first.status_code == 200 and first.json()["rating"] == "again"
    retry = review(client, entry["id"], attempt_id=attempt, answer="A")
    assert (retry.status_code, retry.json()) == (200, first.json())
    second = review(
        client,
        entry["id"],
        attempt_id=attempt,
        answer="A",
        request_id="review_request_0002",
        version=1,
    )
    assert second.status_code == 409
    other = start(client, entry["id"])
    check(client, other, "A")
    reused = review(client, entry["id"], attempt_id=other, answer="A")  # same request_id
    assert reused.status_code == 409
    assert repo.state(entry["id"])["review_count"] == 1


def test_self_reported_attempts_require_and_record_judged_correct(bank):
    store, repo, client = bank
    entry = seed(
        store,
        question_type="short_answer",
        options={},
        correct_answer="Light becomes chemical energy.",
        hints=[],
    )
    attempt = start(client, entry["id"])
    checked = check(client, attempt, "Plants store light as sugar.").json()
    assert (checked["correct"], checked["rating"]) == (None, None)
    answer = {"answer": "Plants store light as sugar."}
    assert review(client, entry["id"], attempt_id=attempt, **answer).status_code == 422
    outcome = review(client, entry["id"], attempt_id=attempt, judged_correct=True, **answer).json()
    assert (outcome["rating"], outcome["correct"]) == ("good", None)
    assert (outcome["self_report"], outcome["judged_correct"]) == (True, True)
    wrong = start(client, entry["id"])
    for _ in range(2):
        hint(client, wrong)
    check(client, wrong, "No idea")
    outcome = review(
        client,
        entry["id"],
        attempt_id=wrong,
        judged_correct=False,
        answer="No idea",
        request_id="review_request_0002",
        version=1,
    ).json()
    assert (outcome["rating"], outcome["self_report"]) == ("again", True)
    assert repo.state(entry["id"])["review_count"] == 2


def test_review_without_attempt_behaves_as_before(bank):
    store, repo, client = bank
    entry = seed(store)
    assert review(client, entry["id"]).status_code == 422  # rating still required
    assert review(client, entry["id"], rating="good", judged_correct=True).status_code == 422
    response = review(client, entry["id"], rating="good")
    assert response.status_code == 200
    assert "attempt_id" not in response.json()
    assert repo.state(entry["id"])["review_count"] == 1


def _attempt_columns(repo):
    with repo.connect() as conn:
        return [row[1] for row in conn.execute("PRAGMA table_info(practice_attempts)")]


def test_attempts_table_migrates_fresh_and_legacy_databases_idempotently(tmp_path):
    fresh = tmp_path / "fresh.db"
    for _ in range(2):
        SQLiteSessionStore(fresh)
        assert set(_attempt_columns(PracticeStore(fresh))) == ATTEMPT_COLUMNS
    legacy = tmp_path / "legacy.db"
    store = SQLiteSessionStore(legacy)
    repo = PracticeStore(legacy)
    entry = seed(store)
    attempt = repo.open_attempt(entry["id"])["attempt_id"]
    kept = attempt_row(repo, attempt)
    for _ in range(2):  # reopening keeps rows
        SQLiteSessionStore(legacy)
        assert attempt_row(repo, attempt) == kept
    with repo.connect() as conn:  # a database from before attempts existed
        conn.execute("DROP TABLE practice_attempts")
    for _ in range(2):
        SQLiteSessionStore(legacy)
        assert set(_attempt_columns(repo)) == ATTEMPT_COLUMNS
    assert repo.open_attempt(entry["id"])["rungs_available"] == 4
    asyncio.run(store.delete_notebook_entry(entry["id"]))
    with repo.connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM practice_attempts").fetchone()[0] == 0


def test_analytics_calibration_and_assistance_count_checked_attempts_in_the_window(bank):
    store, repo, client = bank
    entry = seed(store)
    plan = [  # (confidence, hints, answer)
        (4, 0, "B"),
        (4, 0, "A"),
        (4, 2, "B"),
        (1, 5, "B"),
        (2, 1, "A"),
        (None, 3, "B"),
    ]
    for level, hints, answer in plan:
        attempt = start(client, entry["id"])
        if level:
            client.post(f"/practice/attempts/{attempt}/confidence", json={"level": level})
        for _ in range(hints):
            hint(client, attempt)
        check(client, attempt, answer)
    open_attempt = start(client, entry["id"])  # not checked: not counted
    hint(client, open_attempt)
    old = start(client, entry["id"])
    client.post(f"/practice/attempts/{old}/confidence", json={"level": 3})
    hint(client, old)
    check(client, old)
    with repo.connect() as conn:  # checked before the window
        conn.execute(
            "UPDATE practice_attempts SET started_at=?, closed_at=? WHERE attempt_id=?",
            (time.time() - 40 * DAY, time.time() - 40 * DAY, old),
        )
    report = repo.analytics("UTC", 30)
    assert report["calibration"] == [
        {"level": 1, "attempts": 1, "correct": 1},
        {"level": 2, "attempts": 1, "correct": 0},
        {"level": 3, "attempts": 0, "correct": 0},
        {"level": 4, "attempts": 3, "correct": 2},
    ]
    assert report["assistance"] == {"attempts": 6, "hinted": 4, "revealed": 1}
    assert repo.analytics("UTC", 90)["assistance"]["attempts"] == 7
    assert repo.analytics("UTC", 30, session_ids=[])["assistance"]["attempts"] == 0
    api = client.get("/practice/analytics?days=30").json()
    assert (api["calibration"], api["assistance"]) == (
        report["calibration"],
        report["assistance"],
    )


def test_all_workspace_analytics_sums_calibration_and_assistance(bank, monkeypatch):
    store, _, client = bank
    entry = seed(store)
    attempt = start(client, entry["id"])
    client.post(f"/practice/attempts/{attempt}/confidence", json={"level": 2})
    hint(client, attempt)
    check(client, attempt)

    async def two_workspaces(read):
        return [*(await read()), *(await read())], []

    monkeypatch.setattr(
        "deeptutor.services.workspace.navigation.read_workspace_indexes", two_workspaces
    )
    combined = client.get("/practice/analytics?all_workspaces=true").json()
    assert combined["calibration"][1] == {"level": 2, "attempts": 2, "correct": 2}
    assert combined["assistance"] == {"attempts": 2, "hinted": 2, "revealed": 0}
    assert combined["totals"]["questions"] == 2

"""Practice must preserve source evidence while imports/reviews are retry safe."""

import asyncio
from concurrent.futures import ThreadPoolExecutor
import csv
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import sqlite3

from fastapi import FastAPI
from fastapi.testclient import TestClient
from fsrs import State
from openpyxl import Workbook
import pytest

from deeptutor.api.routers import practice, question_notebook
from deeptutor.core.assessment import ASSESSMENT_SOURCES
from deeptutor.services.practice.importing import normalize_question, preview
from deeptutor.services.practice.scheduler import DAY, FSRS_FIELDS, day_bounds, schedule
from deeptutor.services.practice.storage import PracticeStore, ReviewConflict
from deeptutor.services.session.sqlite_store import SQLiteSessionStore


@pytest.fixture
def bank(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "practice.db")
    for module in (practice, question_notebook):
        monkeypatch.setattr(module, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(question_notebook.router, prefix="/bank")
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
            "user_answer": "A",
            "is_correct": False,
            **changes,
        }
        await store.upsert_notebook_entries("lesson", [item])
        return await store.find_notebook_entry("lesson", item["question_id"])

    return asyncio.run(run())


@pytest.mark.parametrize("source", sorted(ASSESSMENT_SOURCES))
def test_every_producer_enrolls_mistakes_once(bank, source):
    store, repo, _ = bank
    entry = seed(store, source=source)
    first = repo.state(entry["id"])
    assert first["due_at"] == entry["created_at"] + DAY
    repo.review(entry["id"], "first_review", first["version"], "good", "B")
    reviewed = repo.state(entry["id"])
    seed(store, source=source)  # idempotent source re-save cannot reset the schedule
    assert repo.state(entry["id"]) == reviewed


def test_ungraded_imports_are_not_mistakes_and_review_preserves_original(bank):
    store, repo, client = bank
    entry = seed(store, result="ungraded")
    assert repo.overview("UTC")["mistakes"] == 0
    payload = {"request_id": "submission_00000001", "version": 0, "rating": "again", "answer": "A"}
    assert client.post(f"/practice/questions/{entry['id']}/review", json=payload).status_code == 200
    assert repo.overview("UTC")["mistakes"] == 1
    original = asyncio.run(store.get_notebook_entry(entry["id"]))
    for field in ("user_answer", "result", "is_correct", "correct_answer", "created_at"):
        assert original[field] == entry[field]
    assert client.get("/bank/entries?mistakes_only=true").json()["total"] == 1


def test_duplicate_review_and_two_tabs_cannot_advance_twice(bank):
    store, repo, _ = bank
    entry = seed(store)
    args = (entry["id"], "same_request", 0, "good", "B")
    with ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(lambda _: repo.review(*args), range(2)))
    assert results[0] == results[1]
    assert repo.state(entry["id"])["review_count"] == 1
    with pytest.raises(ReviewConflict):
        repo.review(entry["id"], "other_request", 0, "easy", "B")
    with pytest.raises(ReviewConflict):
        repo.review(entry["id"], "same_request", 0, "again", "A")


def test_server_rejects_incorrect_answer_with_success_rating(bank):
    store, repo, client = bank
    entry = seed(store)
    response = client.post(
        f"/practice/questions/{entry['id']}/review",
        json={"request_id": "submission_00000001", "version": 0, "rating": "good", "answer": "A"},
    )
    assert response.status_code == 422
    assert repo.state(entry["id"])["review_count"] == 0


def test_source_key_change_invalidates_open_question(bank):
    store, repo, _ = bank
    entry = seed(store)
    seed(store, correct_answer="A")
    with pytest.raises(ReviewConflict):
        repo.review(entry["id"], "stale", 0, "good", "A")


def test_timezone_due_order_resolved_and_empty_course_scope(bank):
    store, repo, client = bank
    old = seed(store)
    new = seed(store, question_id="q2")
    with repo.connect() as conn:
        conn.execute("UPDATE practice_review_state SET due_at=100 WHERE entry_id=?", (old["id"],))
        conn.execute("UPDATE practice_review_state SET due_at=200 WHERE entry_id=?", (new["id"],))
    assert repo.queue("Asia/Shanghai", now=DAY) == [old["id"], new["id"]]
    assert repo.queue("UTC", session_ids=[], now=DAY) == []
    asyncio.run(store.update_notebook_entry(old["id"], {"resolved": True}))
    assert repo.queue("UTC", now=DAY) == [new["id"]]
    assert repo.overview("UTC", now=DAY)["mistakes"] == 2
    with repo.connect() as conn:
        conn.execute("UPDATE sessions SET deleted_at=1 WHERE id='lesson'")
    assert repo.overview("UTC", now=DAY)["total"] == 0
    assert repo.queue("UTC", now=DAY) == []
    assert client.get(f"/practice/questions/{old['id']}").status_code == 404
    assert (
        client.post(f"/practice/questions/{old['id']}/check", json={"answer": "B"}).status_code
        == 404
    )


def test_first_mistake_timestamp_follows_first_failed_practice(bank):
    store, repo, _ = bank
    entry = seed(store, result="ungraded")
    repo.review(entry["id"], "correct_first", 0, "good", "B", now=100)
    repo.review(entry["id"], "wrong_later", 1, "again", "A", now=200)
    assert repo.state(entry["id"])["first_wrong_at"] == 200


def test_source_mistake_after_success_uses_the_actual_wrong_date(bank):
    store, repo, _ = bank
    entry = seed(store, result="ungraded")
    repo.review(entry["id"], "initial-success", 0, "good", "B", now=100)
    with repo.connect() as conn:
        conn.execute(
            "UPDATE notebook_entries SET result='incorrect', is_correct=0, updated_at=200 WHERE id=?",
            (entry["id"],),
        )
    assert repo.state(entry["id"])["first_wrong_at"] == 200


def test_oversized_spreadsheet_dimensions_and_csv_cells_are_rejected(bank):
    _, _, client = bank
    book = Workbook()
    book.active.cell(row=1, column=1000, value="question")
    output = io.BytesIO()
    book.save(output)
    book.close()
    for name, data in [
        ("wide.xlsx", output.getvalue()),
        ("large.csv", b"question,answer\n" + b"a" * 200000 + b",yes\n"),
    ]:
        response = client.post("/practice/import/preview", files={"file": (name, data)})
        assert response.status_code == 422


def test_import_preview_commit_retries_dedup_tags_and_no_source_overwrite(bank):
    store, repo, client = bank
    data = "题目,题型,A,B,答案,标签\n2+2=?,单选题,3,4,B,数学\n".encode()
    result = client.post(
        "/practice/import/preview", files={"file": ("questions.csv", data)}, data={"target": "bank"}
    ).json()
    assert result["valid"] == 1 and result["token"]
    assert repo.overview("UTC")["total"] == 0  # preview does not alter the bank
    commit = client.post("/practice/import/commit", json={"token": result["token"]})
    assert commit.json() == {"created": 1, "duplicates": 0, "total": 1}
    assert (
        client.post("/practice/import/commit", json={"token": result["token"]}).json()
        == commit.json()
    )
    questions = [
        normalize_question({"question": "2+2=?", "A": "3", "B": "4", "answer": "B", "tags": "复习"})
    ]
    again = repo.commit_import(repo.stage_import("again.json", "mistakes", questions))
    assert again["duplicates"] == 1
    listing = asyncio.run(store.list_notebook_entries())
    assert listing["total"] == 1
    entry = listing["items"][0]
    assert entry["result"] == "ungraded"
    assert {category["name"] for category in entry["categories"]} == {"数学", "复习"}
    assert repo.overview("UTC")["mistakes"] == 1


def test_invalid_row_blocks_entire_import_and_reports_line(bank):
    _, repo, client = bank
    result = client.post(
        "/practice/import/preview",
        files={"file": ("file.csv", b"question,answer\ngood,yes\nbad,\n")},
    ).json()
    assert result["token"] is None
    assert result["errors"][0]["row"] == 3
    assert result["valid"] == 1
    assert repo.overview("UTC")["total"] == 0


def test_template_round_trips_and_formulas_are_rejected(bank):
    _, _, client = bank
    for format in ("csv", "xlsx"):
        data = client.get(f"/practice/import/template?format={format}").content
        assert len(preview(data, f"template.{format}")["questions"]) == 4
        assert preview(data, f"template.{format}")["questions"][0]["misconception"]
        assert preview(data, f"template.{format}")["questions"][0]["hints"]
    book = Workbook()
    book.active.append(["question", "answer"])
    book.active.append(["=1+1", "2"])
    file = io.BytesIO()
    book.save(file)
    with pytest.raises(ValueError, match="formulas"):
        preview(file.getvalue(), "formulas.xlsx")


def test_multi_select_text_and_json_validation():
    row = normalize_question(
        {"题干": "Primes?", "题型": "多选题", "options": ["2", "3", "4"], "答案": "AB"}
    )
    assert row["correct_answer"] == "A,B"
    assert preview(json.dumps([row]).encode(), "q.json")["questions"][0]["question"] == "Primes?"
    with pytest.raises(ValueError, match="500"):
        preview(json.dumps([row] * 501).encode(), "many.json")


QUESTION_BANK_CSV = Path(__file__).resolve().parents[2] / "content" / "question_bank.csv"


def test_question_bank_csv_import_keeps_every_misconception(bank):
    _, _, client = bank
    with QUESTION_BANK_CSV.open(encoding="utf-8", newline="") as handle:
        expected = {row["question"]: row["misconception"] for row in csv.DictReader(handle)}
    assert len(expected) == 60 and all(expected.values())
    staged = client.post(
        "/practice/import/preview",
        files={"file": ("question_bank.csv", QUESTION_BANK_CSV.read_bytes())},
        data={"target": "bank"},
    ).json()
    assert (staged["valid"], staged["errors"]) == (60, [])
    assert staged["samples"][0]["misconception"] == expected[staged["samples"][0]["question"]]
    commit = client.post("/practice/import/commit", json={"token": staged["token"]})
    assert commit.json() == {"created": 60, "duplicates": 0, "total": 60}
    listing = client.get("/bank/entries?limit=200").json()
    assert {item["question"]: item["misconception"] for item in listing["items"]} == expected
    for item in listing["items"]:
        practice_entry = client.get(f"/practice/questions/{item['id']}").json()["entry"]
        assert practice_entry["misconception"] == expected[item["question"]]


def test_misconception_is_optional_and_not_part_of_the_import_identity():
    plain = normalize_question({"question": "Q", "answer": "A"})
    noted = normalize_question({"question": "Q", "answer": "A", "Misconception": " Thinks B "})
    assert plain["misconception"] is None
    assert noted["misconception"] == "Thinks B"
    assert noted["question_id"] == plain["question_id"]
    assert normalize_question({"题目": "Q", "答案": "A", "误区": "B"})["misconception"] == "B"


def test_question_bank_csv_hints_round_trip_and_default_to_empty(bank, tmp_path):
    _, _, client = bank
    with QUESTION_BANK_CSV.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    assert "hints" not in rows[0]

    # Without a hints column every question imports with no rungs.
    plain_store = SQLiteSessionStore(tmp_path / "plain.db")
    plain_repo = PracticeStore(plain_store.db_path)
    questions = preview(QUESTION_BANK_CSV.read_bytes(), "question_bank.csv")["questions"]
    plain_repo.commit_import(plain_repo.stage_import("question_bank.csv", "bank", questions))
    plain = asyncio.run(plain_store.list_notebook_entries(limit=200))["items"]
    assert len(plain) == 60 and all(item["hints"] == [] for item in plain)
    ids = {item["question"]: item["question_id"] for item in plain}

    # The same file with a hints column: rungs split on "|", blanks dropped,
    # and the import identity (so every existing id) is unchanged.
    rungs = ["Nudge", "Hint", "Key step", "Worked"]
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, [*rows[0], "hints"])
    writer.writeheader()
    for number, row in enumerate(rows):
        writer.writerow(
            {**row, "hints": " Nudge | Hint || Key step|Worked " if number == 0 else ""}
        )
    staged = client.post(
        "/practice/import/preview",
        files={"file": ("question_bank.csv", output.getvalue().encode())},
        data={"target": "bank"},
    ).json()
    assert (staged["valid"], staged["errors"]) == (60, [])
    commit = client.post("/practice/import/commit", json={"token": staged["token"]})
    assert commit.json()["created"] == 60
    listing = client.get("/bank/entries?limit=200").json()["items"]
    assert {item["question"]: item["question_id"] for item in listing} == ids
    hinted = {item["question"]: item["hints"] for item in listing}
    assert hinted == {row["question"]: rungs if n == 0 else [] for n, row in enumerate(rows)}
    first = next(item for item in listing if item["question"] == rows[0]["question"])
    assert client.get(f"/practice/questions/{first['id']}").json()["entry"]["hints"] == rungs


def test_hints_import_from_a_committed_file_and_json_lists():
    row = normalize_question({"question": "Q", "answer": "A", "Hints": "One|Two"})
    assert row["hints"] == ["One", "Two"]
    assert row["question_id"] == normalize_question({"question": "Q", "answer": "A"})["question_id"]
    assert normalize_question({"question": "Q", "answer": "A", "hints": ["X", " "]})["hints"] == [
        "X"
    ]
    with pytest.raises(ValueError, match="hints"):
        normalize_question({"question": "Q", "answer": "A", "hints": "1|2|3|4|5"})


def test_committed_import_stores_hints_on_the_entry(bank):
    _, repo, client = bank
    question = normalize_question({"question": "Q", "answer": "A", "hints": "Look again|Think"})
    repo.commit_import(repo.stage_import("q.json", "bank", [question]))
    (item,) = client.get("/bank/entries").json()["items"]
    assert item["hints"] == ["Look again", "Think"]
    assert client.get(f"/practice/questions/{item['id']}").json()["entry"]["hints"] == [
        "Look again",
        "Think",
    ]
    # Previews staged before this field existed lack the key.
    legacy = {key: value for key, value in question.items() if key != "hints"}
    legacy["question_id"] = "import:legacy"
    repo.commit_import(repo.stage_import("old.json", "bank", [legacy]))
    listed = {row["question_id"]: row for row in client.get("/bank/entries").json()["items"]}
    assert listed["import:legacy"]["hints"] == []


def test_notebook_api_carries_hints_and_keeps_them_across_answer_saves(bank):
    _, _, client = bank
    base = {"origin_type": "external_import", "origin_ref": "notes", "question": "1/2 + 1/3?"}
    saved = client.post(
        "/bank/entries/upsert", json={**base, "question_id": "q1", "hints": ["Common base?"]}
    ).json()
    assert saved["hints"] == ["Common base?"]
    client.post("/bank/entries/upsert", json={**base, "question_id": "q1", "user_answer": "2/5"})
    assert client.get(f"/bank/entries/{saved['id']}").json()["hints"] == ["Common base?"]
    assert client.get("/bank/entries").json()["items"][0]["hints"] == ["Common base?"]
    too_many = {**base, "question_id": "q2", "hints": ["1", "2", "3", "4", "5"]}
    assert client.post("/bank/entries/upsert", json=too_many).status_code == 422


def test_notebook_api_keeps_misconception_across_answer_saves(bank):
    _, _, client = bank
    base = {"origin_type": "external_import", "origin_ref": "notes", "question": "1/2 + 1/3?"}
    saved = client.post(
        "/bank/entries/upsert",
        json={**base, "question_id": "q1", "misconception": "Adds the denominators."},
    ).json()
    # Saving an answer without the field must not erase the stored misconception.
    client.post(
        "/bank/entries/upsert",
        json={**base, "question_id": "q1", "user_answer": "2/5", "is_correct": False},
    )
    assert client.get(f"/bank/entries/{saved['id']}").json()["misconception"] == (
        "Adds the denominators."
    )
    bare = client.post("/bank/entries/upsert", json={**base, "question_id": "q2"}).json()
    assert bare["misconception"] is None
    listed = {item["id"]: item for item in client.get("/bank/entries").json()["items"]}
    assert listed[saved["id"]]["misconception"] == "Adds the denominators."
    assert listed[bare["id"]]["misconception"] is None
    assert client.get(f"/practice/questions/{bare['id']}").json()["entry"]["misconception"] is None


def test_additive_migration_is_idempotent(bank):
    store, repo, _ = bank
    entry = seed(store)
    with repo.connect() as conn:
        for trigger in (
            "practice_capture_insert",
            "practice_capture_update",
            "practice_invalidate_question",
        ):
            conn.execute(f"DROP TRIGGER {trigger}")
        conn.execute("DROP TABLE practice_review_state")
    upgraded = SQLiteSessionStore(store.db_path)
    state = repo.state(entry["id"])
    assert state["first_wrong_at"] == entry["created_at"]
    restored = asyncio.run(upgraded.get_notebook_entry(entry["id"]))
    assert {k: v for k, v in restored.items() if k not in {"practice", "categories"}} == entry
    SQLiteSessionStore(store.db_path)
    assert repo.state(entry["id"]) == state
    asyncio.run(upgraded.delete_notebook_entry(entry["id"]))
    with repo.connect() as conn:
        assert conn.execute("SELECT COUNT(*) FROM practice_review_state").fetchone()[0] == 0


def test_import_transaction_rolls_back(bank):
    _, repo, _ = bank
    token = repo.stage_import(
        "q.json", "bank", [normalize_question({"question": "Q", "answer": "A"})]
    )
    with repo.connect() as conn:
        conn.execute(
            "CREATE TRIGGER reject_import BEFORE INSERT ON notebook_entries BEGIN SELECT RAISE(ABORT, 'unavailable'); END"
        )
    with pytest.raises(sqlite3.IntegrityError):
        repo.commit_import(token)
    assert repo.overview("UTC")["total"] == 0
    with repo.connect() as conn:
        conn.execute("DROP TRIGGER reject_import")
    assert repo.commit_import(token)["created"] == 1


def test_daily_scheduler_and_dst():
    now = datetime(2026, 3, 8, 12, tzinfo=timezone.utc).timestamp()
    start, end = day_bounds("America/New_York", now)
    assert end - start == 23 * 3600
    with pytest.raises(ValueError):
        day_bounds("not/a-zone", now)
    good = schedule({}, "good", now)
    better = schedule(good, "good", good["due_at"])
    again = schedule(better, "again", better["due_at"])
    assert good["interval_days"] < better["interval_days"]
    assert 0 < again["interval_days"] < 1
    assert again["lapses"] == 1 and again["review_count"] == 3
    assert schedule({}, "easy", now)["due_at"] > good["due_at"]
    hard = schedule(better, "hard", now)
    assert (
        schedule(hard, "good", now)["interval_days"] >= schedule(hard, "hard", now)["interval_days"]
    )


T0 = datetime(2026, 9, 1, 12, tzinfo=timezone.utc).timestamp()


def test_fsrs_new_card_starts_in_short_learning_steps():
    good = schedule({}, "good", T0)
    assert (good["fsrs_state"], good["fsrs_step"]) == (State.Learning, 1)
    assert good["fsrs_stability"] > 0 and 1 <= good["fsrs_difficulty"] <= 10
    assert good["due_at"] == pytest.approx(T0 + 10 * 60)
    assert good["interval_days"] == pytest.approx(10 / 1440)
    assert (good["review_count"], good["version"], good["streak"], good["lapses"]) == (1, 1, 1, 0)
    assert good["last_review_at"] == T0 and "ease" not in good
    again = schedule({}, "again", T0)
    assert again["due_at"] == pytest.approx(T0 + 60) and again["lapses"] == 1
    assert again["fsrs_stability"] < good["fsrs_stability"]
    easy = schedule({}, "easy", T0)
    assert (easy["fsrs_state"], easy["fsrs_step"]) == (State.Review, None)
    assert easy["interval_days"] >= 1


def test_fsrs_lapse_relearns_soon_and_lowers_stability():
    learned = schedule(schedule({}, "good", T0), "good", T0 + 600)
    assert learned["fsrs_state"] == State.Review and learned["interval_days"] >= 1
    lapse = schedule(learned, "again", learned["due_at"])
    assert (lapse["fsrs_state"], lapse["fsrs_step"]) == (State.Relearning, 0)
    assert lapse["due_at"] == pytest.approx(learned["due_at"] + 10 * 60)
    assert lapse["fsrs_stability"] < learned["fsrs_stability"]
    assert lapse["fsrs_difficulty"] > learned["fsrs_difficulty"]
    assert (lapse["lapses"], lapse["streak"], lapse["review_count"]) == (1, 0, 3)
    relearned = schedule(lapse, "good", lapse["due_at"])
    assert relearned["fsrs_state"] == State.Review and relearned["interval_days"] >= 1
    assert relearned["lapses"] == 1


def test_unknown_rating_is_rejected():
    with pytest.raises(ValueError, match="Unknown review rating"):
        schedule({}, "later", T0)


def test_fsrs_state_round_trips_through_the_store(bank):
    store, repo, _ = bank
    entry = seed(store)
    repo.review(entry["id"], "round-trip-first", 0, "again", "A", now=T0)
    saved = repo.state(entry["id"])
    first = schedule({"version": 0}, "again", T0)
    assert {key: saved[key] for key in first} == first
    assert saved["ease"] == 2.5  # the legacy SM-2 column is no longer written
    second = repo.review(entry["id"], "round-trip-second", 1, "good", "B", now=T0 + 120)
    expected = schedule(saved, "good", T0 + 120)
    assert {key: second[key] for key in expected} == expected
    assert second["fsrs_stability"] != schedule({}, "good", T0 + 120)["fsrs_stability"]


def test_legacy_row_rebuilds_fsrs_state_from_review_history(bank):
    store, repo, _ = bank
    kept, legacy = seed(store), seed(store, question_id="q2")
    history = [(T0, "again", "A"), (T0 + 600, "good", "B"), (T0 + 3 * DAY, "good", "B")]
    for entry in (kept, legacy):
        for version, (at, rating, answer) in enumerate(history):
            repo.review(
                entry["id"], f"history-{entry['id']}-{version}", version, rating, answer, now=at
            )
    with repo.connect() as conn:  # a row scheduled before FSRS has no memory state
        conn.execute(
            "UPDATE practice_review_state SET fsrs_stability=NULL, fsrs_difficulty=NULL,"
            " fsrs_state=NULL, fsrs_step=NULL WHERE entry_id=?",
            (legacy["id"],),
        )
    final = [
        repo.review(entry["id"], f"final-{entry['id']}", 3, "hard", "B", now=T0 + 10 * DAY)
        for entry in (kept, legacy)
    ]
    kept_fsrs = {key: final[0][key] for key in FSRS_FIELDS}
    assert final[1]["fsrs_state"] == State.Review  # a new card would still be learning
    assert {key: final[1][key] for key in FSRS_FIELDS} == kept_fsrs
    assert {key: repo.state(legacy["id"])[key] for key in FSRS_FIELDS} == kept_fsrs


def test_fsrs_columns_migrate_existing_databases_idempotently(bank):
    store, repo, _ = bank
    entry = seed(store)

    def fsrs_columns():
        with repo.connect() as conn:
            rows = conn.execute("PRAGMA table_info(practice_review_state)").fetchall()
        return {row[1] for row in rows} & set(FSRS_FIELDS)

    assert fsrs_columns() == set(FSRS_FIELDS)  # fresh database
    with repo.connect() as conn:  # restore the schema from before FSRS
        for name in FSRS_FIELDS:
            conn.execute(f"ALTER TABLE practice_review_state DROP COLUMN {name}")
    legacy = repo.state(entry["id"])
    for _ in range(2):
        SQLiteSessionStore(store.db_path)
        assert fsrs_columns() == set(FSRS_FIELDS)
        assert repo.state(entry["id"]) == {**legacy, **dict.fromkeys(FSRS_FIELDS)}


def test_malformed_files_are_validation_errors(bank):
    _, _, client = bank
    for filename, data in [
        ("broken.xlsx", b"not a workbook"),
        ("broken.json", b"{"),
        ("old.xls", b"old format"),
    ]:
        response = client.post("/practice/import/preview", files={"file": (filename, data)})
        assert response.status_code == 422


def test_import_receipts_and_questions_are_isolated_between_stores(bank, tmp_path):
    _, repo, _ = bank
    token = repo.stage_import(
        "q.json", "bank", [normalize_question({"question": "Q", "answer": "A"})]
    )
    other = SQLiteSessionStore(tmp_path / "other-user.db")
    other_repo = PracticeStore(other.db_path)
    with pytest.raises(LookupError):
        other_repo.commit_import(token)
    repo.commit_import(token)
    assert other_repo.overview("UTC")["total"] == 0
    with pytest.raises(LookupError):
        other_repo.review(1, "other-user-attempt", 0, "again", "")


def test_course_imports_keep_their_course_and_file_provenance(bank, monkeypatch):
    from types import SimpleNamespace

    monkeypatch.setattr(
        "deeptutor.services.courses.get_course_service", lambda: SimpleNamespace(get=lambda _: {})
    )
    store, repo, client = bank
    question = normalize_question({"question": "Q", "answer": "A"})
    repo.commit_import(repo.stage_import("lesson.csv", "mistakes", [question], "course-a"))
    listing = asyncio.run(store.list_notebook_entries())
    assert listing["total"] == 1
    item = listing["items"][0]
    assert item["session_id"] == ""
    assert item["origin_type"] == "external_import"
    assert item["origin_ref"] == "practice-import"
    assert item["material_id"].startswith("import:")
    assert repo.overview("UTC")["mistakes"] == 1
    # Import provenance must not manufacture a conversation or leak into a
    # conversation-derived course scope.
    assert asyncio.run(store.list_sessions()) == []
    assert client.get("/bank/entries?course_id=course-a").json()["total"] == 0
    assert client.get("/practice/summary?course_id=course-a").json()["mistakes"] == 0
    assert client.get("/bank/entries?course_id=course-b").json()["total"] == 0


def test_analytics_separates_creation_first_mistake_and_repeat_reviews(bank):
    store, repo, client = bank
    now = datetime(2026, 9, 19, 12, tzinfo=timezone.utc).timestamp()
    entry = seed(store, source="mastery_path", result="ungraded")
    with repo.connect() as conn:
        conn.execute(
            "UPDATE notebook_entries SET created_at=? WHERE id=?", (now - 3 * DAY, entry["id"])
        )
    repo.review(entry["id"], "first-wrong", 0, "again", "A", now=now - DAY)
    repo.review(entry["id"], "second-wrong", 1, "again", "A", now=now)
    repo.review(entry["id"], "second-wrong", 1, "again", "A", now=now)
    report = repo.analytics("Asia/Shanghai", 7, now=now)
    assert report["totals"] == {"questions": 1, "mistakes": 1, "reviews": 2}
    assert len(report["daily"]) == 7
    assert report["daily"][-4] == {
        "date": "2026-09-16",
        "questions": 1,
        "mistakes": 0,
        "reviews": 0,
    }
    assert report["daily"][-2]["mistakes"] == 1
    assert report["daily"][-1]["mistakes"] == 0
    assert report["sources"] == [{"source": "mastery_path", **report["totals"]}]
    asyncio.run(store.update_notebook_entry(entry["id"], {"resolved": True}))
    assert repo.analytics("Asia/Shanghai", 7, now=now)["totals"] == report["totals"]
    assert repo.analytics("Asia/Shanghai", 7, session_ids=[], now=now)["totals"]["questions"] == 0
    assert client.get("/practice/analytics?days=8").status_code == 422
    assert client.get("/practice/analytics?timezone=not-a-zone").status_code == 422


def test_analytics_local_midnight_dst_zero_days_and_deleted_sources(bank):
    store, repo, _ = bank
    now = datetime(2026, 3, 10, 12, tzinfo=timezone.utc).timestamp()
    # These straddle local midnight during the daylight-saving transition.
    entries = [seed(store, question_id=f"dst-{i}", source="book") for i in range(2)]
    timestamps = [
        datetime(2026, 3, 8, 4, 59, tzinfo=timezone.utc).timestamp(),
        datetime(2026, 3, 8, 5, 1, tzinfo=timezone.utc).timestamp(),
    ]
    with repo.connect() as conn:
        for entry, ts in zip(entries, timestamps, strict=True):
            conn.execute("UPDATE notebook_entries SET created_at=? WHERE id=?", (ts, entry["id"]))
            conn.execute(
                "UPDATE practice_review_state SET first_wrong_at=? WHERE entry_id=?",
                (ts, entry["id"]),
            )
    data = repo.analytics("America/New_York", 7, now=now)
    by_day = {row["date"]: row for row in data["daily"]}
    assert by_day["2026-03-07"]["questions"] == by_day["2026-03-08"]["questions"] == 1
    assert by_day["2026-03-09"]["questions"] == 0
    with repo.connect() as conn:
        conn.execute("UPDATE sessions SET deleted_at=? WHERE id='lesson'", (now,))
    assert repo.analytics("America/New_York", 7, now=now)["sources"] == []

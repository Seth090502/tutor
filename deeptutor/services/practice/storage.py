"""Transactional review history and import staging in the user's notebook DB."""

from __future__ import annotations

from contextlib import contextmanager
import json
from pathlib import Path
import secrets
import sqlite3
import time
import uuid

from .answers import check_answer, grade
from .attempts import ANSWER_RUNG, attempt_rating
from .scheduler import DAY, Rating, day_bounds, replay, schedule

# Legacy correct or manually mastered questions start with a three-day interval.
# Once reviewed, the saved adaptive schedule is authoritative.
_DUE_AT_SQL = """(CASE WHEN (n.resolved=1 OR n.is_correct=1)
    AND COALESCE(r.review_count, 0)=0 THEN n.updated_at + 259200
    ELSE COALESCE(r.due_at, n.created_at) END)"""

# FSRS memory state (the py-fsrs Card fields). NULL until a card's first FSRS
# review; review() rebuilds it from practice_review_events for older rows.
_FSRS_COLUMNS = (
    ("fsrs_stability", "REAL"),
    ("fsrs_difficulty", "REAL"),
    ("fsrs_state", "INTEGER"),
    ("fsrs_step", "INTEGER"),
)


class ReviewConflict(ValueError):
    """A different tab or source changed this question since it was opened."""


class AttemptConflict(ValueError):
    """The attempt is past the step this request needs (checked, revealed or reviewed)."""


# A language card's code from its ``lang:<code>`` tag (contract C10), or NULL.
_LANG_SQL = """(SELECT substr(c.name, 6) FROM notebook_entry_categories ec
    JOIN notebook_categories c ON c.id=ec.category_id
    WHERE ec.entry_id=n.id AND c.name LIKE 'lang:_%' ORDER BY c.name LIMIT 1) AS lang"""

# The attempt with the question fields its steps need, if the question is visible.
_ATTEMPT_SQL = f"""SELECT a.*, n.question_type, n.options_json, n.correct_answer, n.explanation,
        n.misconception, n.hints_json, n.accepted_answers_json, {_LANG_SQL}
    FROM practice_attempts a JOIN notebook_entries n ON n.id=a.entry_id
    LEFT JOIN sessions s ON s.id=n.session_id
    WHERE a.attempt_id=? AND (n.session_id IS NULL OR s.deleted_at IS NULL)"""


def _hints(row: sqlite3.Row) -> list[str]:
    return json.loads(row["hints_json"] or "[]")


def initialize_practice(conn: sqlite3.Connection) -> None:
    existing = conn.execute(
        "SELECT 1 FROM sqlite_master WHERE name='practice_review_state'"
    ).fetchone()
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS practice_review_state (
            entry_id INTEGER PRIMARY KEY REFERENCES notebook_entries(id) ON DELETE CASCADE,
            is_mistake INTEGER NOT NULL DEFAULT 1,
            first_wrong_at REAL NOT NULL,
            due_at REAL NOT NULL,
            interval_days REAL NOT NULL DEFAULT 1,
            ease REAL NOT NULL DEFAULT 2.5,
            streak INTEGER NOT NULL DEFAULT 0,
            lapses INTEGER NOT NULL DEFAULT 0,
            review_count INTEGER NOT NULL DEFAULT 0,
            last_review_at REAL,
            version INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_practice_due ON practice_review_state(due_at, entry_id);
        CREATE TABLE IF NOT EXISTS practice_review_events (
            request_id TEXT PRIMARY KEY,
            entry_id INTEGER NOT NULL REFERENCES notebook_entries(id) ON DELETE CASCADE,
            rating TEXT NOT NULL,
            answer TEXT NOT NULL,
            reviewed_at REAL NOT NULL,
            outcome_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_practice_history ON practice_review_events(entry_id, reviewed_at);
        CREATE TABLE IF NOT EXISTS practice_attempts (
            attempt_id TEXT PRIMARY KEY,
            entry_id INTEGER NOT NULL REFERENCES notebook_entries(id) ON DELETE CASCADE,
            started_at REAL NOT NULL,
            confidence INTEGER,
            hint_rung_max INTEGER NOT NULL DEFAULT 0,
            revealed INTEGER NOT NULL DEFAULT 0,
            answer TEXT,
            correct INTEGER,
            closed_at REAL,
            reviewed INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_practice_attempts_closed ON practice_attempts(closed_at);
        CREATE TABLE IF NOT EXISTS practice_imports (
            token TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            target TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at REAL NOT NULL,
            result_json TEXT
        );
        CREATE TRIGGER IF NOT EXISTS practice_capture_insert
        AFTER INSERT ON notebook_entries
        WHEN NEW.is_correct = 0 AND COALESCE(NEW.result, '') != 'ungraded'
        BEGIN
            INSERT INTO practice_review_state(entry_id, first_wrong_at, due_at)
            SELECT NEW.id, NEW.created_at, NEW.created_at + 86400
            WHERE NOT EXISTS(SELECT 1 FROM practice_review_state WHERE entry_id=NEW.id);
        END;
        CREATE TRIGGER IF NOT EXISTS practice_invalidate_question
        AFTER UPDATE OF question, options_json, correct_answer ON notebook_entries
        WHEN OLD.question != NEW.question OR OLD.options_json != NEW.options_json OR OLD.correct_answer != NEW.correct_answer
        BEGIN
            UPDATE practice_review_state SET version=version+1 WHERE entry_id=NEW.id;
        END;
        DROP TRIGGER IF EXISTS practice_capture_update;
        CREATE TRIGGER practice_capture_update
        AFTER UPDATE OF is_correct, result ON notebook_entries
        WHEN NEW.is_correct = 0 AND COALESCE(NEW.result, '') != 'ungraded'
        BEGIN
            INSERT INTO practice_review_state(entry_id, first_wrong_at, due_at)
            SELECT NEW.id, NEW.updated_at, NEW.updated_at + 86400
            WHERE NOT EXISTS(SELECT 1 FROM practice_review_state WHERE entry_id=NEW.id);
            UPDATE practice_review_state SET
                first_wrong_at = CASE WHEN is_mistake=0 THEN NEW.updated_at ELSE first_wrong_at END,
                is_mistake = 1,
                due_at = NEW.updated_at + 86400, interval_days = 1, streak = 0,
                version = version + 1
            WHERE entry_id = NEW.id AND (OLD.is_correct = 1 OR OLD.result = 'ungraded');
        END;
    """)
    # Added in place so existing databases upgrade. ``ease`` stays for old rows
    # but is no longer written: FSRS keeps its memory state in these columns.
    columns = {row[1] for row in conn.execute("PRAGMA table_info(practice_review_state)")}
    for name, kind in _FSRS_COLUMNS:
        if name not in columns:
            conn.execute(f"ALTER TABLE practice_review_state ADD COLUMN {name} {kind}")
    if not existing:
        # Additive migration: original IDs, answers, timestamps and categories stay intact.
        conn.execute("""
            INSERT OR IGNORE INTO practice_review_state(entry_id, first_wrong_at, due_at)
            SELECT id, created_at, created_at + 86400 FROM notebook_entries
            WHERE is_correct = 0 AND COALESCE(result, '') != 'ungraded'
        """)


class PracticeStore:
    def __init__(self, db_path: Path):
        self.db_path = db_path

    @contextmanager
    def connect(self):
        conn = sqlite3.connect(self.db_path, timeout=30)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA foreign_keys = ON")
        try:
            with conn:
                yield conn
        finally:
            conn.close()

    @staticmethod
    def _scope(session_ids: list[str] | None) -> tuple[str, list]:
        # Independent imports/document questions have no chat to delete. They
        # stay visible in the global bank, while an explicit course/session
        # scope continues to select conversation-backed entries only.
        sql = "(n.session_id IS NULL OR s.deleted_at IS NULL)"
        params: list = []
        if session_ids is not None:
            sql += " AND n.session_id IN (" + (",".join("?" for _ in session_ids) or "NULL") + ")"
            params.extend(session_ids)
        return sql, params

    def overview(
        self, timezone: str, session_ids: list[str] | None = None, now: float | None = None
    ) -> dict:
        now = time.time() if now is None else now
        start, end = day_bounds(timezone, now)
        scope, params = self._scope(session_ids)
        with self.connect() as conn:
            row = conn.execute(
                f"""
                SELECT COUNT(*) AS total,
                    COALESCE(SUM(r.is_mistake = 1), 0) AS mistakes,
                    COALESCE(SUM({_DUE_AT_SQL} <= ?), 0) AS due,
                    COALESCE(SUM({_DUE_AT_SQL} <= ?), 0) AS overdue,
                    MIN({_DUE_AT_SQL}) AS next_due_at
                FROM notebook_entries n LEFT JOIN sessions s ON s.id = n.session_id
                LEFT JOIN practice_review_state r ON r.entry_id = n.id WHERE {scope}
            """,  # nosec B608 - fixed scope SQL; values bound
                [now, start, *params],
            ).fetchone()  # nosec B608 - bound scope values
            completed = conn.execute(
                f"""
                SELECT COUNT(DISTINCT e.entry_id) FROM practice_review_events e
                JOIN notebook_entries n ON n.id = e.entry_id LEFT JOIN sessions s ON s.id = n.session_id
                WHERE e.reviewed_at >= ? AND e.reviewed_at < ? AND {scope}
            """,  # nosec B608 - fixed scope SQL; values bound
                [start, end, *params],
            ).fetchone()[0]  # nosec B608
        return {**dict(row), "reviewed_today": completed, "day_end": end, "timezone": timezone}

    def analytics(
        self,
        timezone: str,
        days: int = 30,
        session_ids: list[str] | None = None,
        now: float | None = None,
    ) -> dict:
        from .analytics import analytics

        scope, params = self._scope(session_ids)
        with self.connect() as conn:
            return analytics(
                conn, timezone, days, time.time() if now is None else now, scope, params
            )

    def queue(
        self,
        timezone: str,
        session_ids: list[str] | None = None,
        category_id: int | None = None,
        limit: int = 20,
        now: float | None = None,
    ) -> list[int]:
        now = time.time() if now is None else now
        day_bounds(timezone, now)
        scope, params = self._scope(session_ids)
        if category_id is not None:
            scope += " AND EXISTS(SELECT 1 FROM notebook_entry_categories c WHERE c.entry_id=n.id AND c.category_id=?)"
            params.append(category_id)
        with self.connect() as conn:
            rows = conn.execute(
                f"""
                SELECT n.id FROM notebook_entries n LEFT JOIN sessions s ON s.id=n.session_id
                LEFT JOIN practice_review_state r ON r.entry_id=n.id
                WHERE {_DUE_AT_SQL} <= ? AND {scope}
                ORDER BY {_DUE_AT_SQL}, n.id LIMIT ?
            """,  # nosec B608 - fixed scope SQL; values bound
                [now, *params, limit],
            ).fetchall()  # nosec B608
        return [row["id"] for row in rows]

    def state(self, entry_id: int) -> dict:
        with self.connect() as conn:
            row = conn.execute(
                "SELECT * FROM practice_review_state WHERE entry_id=?", (entry_id,)
            ).fetchone()
        return (
            dict(row)
            if row
            else {
                "entry_id": entry_id,
                "is_mistake": 0,
                "version": 0,
                "review_count": 0,
                "interval_days": 1,
                "due_at": None,
            }
        )

    def is_visible(self, entry_id: int) -> bool:
        with self.connect() as conn:
            return (
                conn.execute(
                    """SELECT 1 FROM notebook_entries n LEFT JOIN sessions s ON s.id=n.session_id
                WHERE n.id=? AND (n.session_id IS NULL OR s.deleted_at IS NULL)""",
                    (entry_id,),
                ).fetchone()
                is not None
            )

    def review(
        self,
        entry_id: int,
        request_id: str,
        version: int,
        rating: Rating | None,
        answer: str,
        now: float | None = None,
        *,
        self_report: bool = False,
        attempt_id: str | None = None,
        judged_correct: bool | None = None,
    ) -> dict:
        """Record one review. With ``attempt_id`` the rating comes from that checked attempt."""
        now = time.time() if now is None else now
        with self.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            previous = conn.execute(
                "SELECT * FROM practice_review_events WHERE request_id=?", (request_id,)
            ).fetchone()
            if previous:
                outcome = json.loads(previous["outcome_json"])
                if (
                    previous["entry_id"] != entry_id
                    or previous["answer"] != answer
                    or outcome.get("attempt_id") != attempt_id
                    or outcome.get("judged_correct") != judged_correct
                    or (attempt_id is None and previous["rating"] != rating)
                    or (attempt_id is None and bool(outcome.get("self_report")) != self_report)
                ):
                    raise ReviewConflict(
                        "This submission ID was already used for a different answer"
                    )
                return outcome
            entry = conn.execute(
                f"""SELECT n.*, {_LANG_SQL} FROM notebook_entries n
                LEFT JOIN sessions s ON s.id=n.session_id
                WHERE n.id=? AND (n.session_id IS NULL OR s.deleted_at IS NULL)""",  # nosec B608
                (entry_id,),
            ).fetchone()
            if not entry:
                raise LookupError("Question not found")
            row = conn.execute(
                "SELECT * FROM practice_review_state WHERE entry_id=?", (entry_id,)
            ).fetchone()
            state = dict(row) if row else {"version": 0}
            if state["version"] != version:
                raise ReviewConflict(
                    "This question changed in another tab. Reload it before reviewing again."
                )
            if attempt_id is None:
                # Objective answers are checked again at the transaction boundary.
                correct = None if self_report else check_answer(dict(entry), answer)
                if correct is False and rating != "again":
                    raise ValueError("An incorrect answer must be reviewed again")
            else:
                attempt = conn.execute(
                    "SELECT * FROM practice_attempts WHERE attempt_id=? AND entry_id=?",
                    (attempt_id, entry_id),
                ).fetchone()
                if not attempt:
                    raise LookupError("Attempt not found for this question")
                if attempt["closed_at"] is None:
                    raise AttemptConflict("Check the answer before reviewing the attempt")
                if attempt["reviewed"]:
                    raise AttemptConflict("This attempt was already reviewed")
                if answer != attempt["answer"]:
                    raise ValueError("The answer does not match the checked attempt")
                correct = None if attempt["correct"] is None else bool(attempt["correct"])
                # Only an answer the checker cannot grade is judged by the learner.
                self_report = correct is None
                if self_report and judged_correct is None:
                    raise ValueError("judged_correct is required for an answer checked by hand")
                if not self_report and judged_correct is not None:
                    raise ValueError("This answer was checked automatically; omit judged_correct")
                rating = attempt_rating(
                    judged_correct if self_report else correct,
                    attempt["hint_rung_max"],
                    bool(attempt["revealed"]),
                )
                conn.execute(
                    "UPDATE practice_attempts SET reviewed=1 WHERE attempt_id=?", (attempt_id,)
                )
            if state.get("fsrs_state") is None:
                # Scheduled before FSRS (or never reviewed): rebuild the memory
                # state from the retained history; none means a new card.
                history = conn.execute(
                    """SELECT reviewed_at, rating FROM practice_review_events
                    WHERE entry_id=? ORDER BY reviewed_at, rowid""",
                    (entry_id,),
                ).fetchall()
                state.update(replay((event["reviewed_at"], event["rating"]) for event in history))
            next_state = schedule(state, rating, now)
            mistake = bool(state.get("is_mistake")) or rating == "again"
            conn.execute(
                """
                INSERT INTO practice_review_state(entry_id, is_mistake, first_wrong_at, due_at,
                    interval_days, streak, lapses, review_count, last_review_at, version,
                    fsrs_stability, fsrs_difficulty, fsrs_state, fsrs_step)
                VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(entry_id) DO UPDATE SET is_mistake=excluded.is_mistake, due_at=excluded.due_at,
                    first_wrong_at=CASE WHEN practice_review_state.is_mistake=0 AND excluded.is_mistake=1
                        THEN excluded.first_wrong_at ELSE practice_review_state.first_wrong_at END,
                    interval_days=excluded.interval_days, streak=excluded.streak,
                    lapses=excluded.lapses, review_count=excluded.review_count,
                    last_review_at=excluded.last_review_at, version=excluded.version,
                    fsrs_stability=excluded.fsrs_stability, fsrs_difficulty=excluded.fsrs_difficulty,
                    fsrs_state=excluded.fsrs_state, fsrs_step=excluded.fsrs_step
            """,
                (
                    entry_id,
                    int(mistake),
                    now,
                    *(
                        next_state[key]
                        for key in (
                            "due_at",
                            "interval_days",
                            "streak",
                            "lapses",
                            "review_count",
                            "last_review_at",
                            "version",
                            "fsrs_stability",
                            "fsrs_difficulty",
                            "fsrs_state",
                            "fsrs_step",
                        )
                    ),
                ),
            )
            if rating == "again":
                conn.execute("UPDATE notebook_entries SET resolved=0 WHERE id=?", (entry_id,))
            outcome = {
                **next_state,
                "entry_id": entry_id,
                "is_mistake": mistake,
                "correct": correct,
                "rating": rating,
                "self_report": self_report,
                "mastered": rating in {"good", "easy"},
            }
            if attempt_id is not None:
                outcome.update(attempt_id=attempt_id, judged_correct=judged_correct)
            conn.execute(
                "INSERT INTO practice_review_events VALUES(?, ?, ?, ?, ?, ?)",
                (request_id, entry_id, rating, answer, now, json.dumps(outcome)),
            )
        return outcome

    def open_attempt(self, entry_id: int) -> dict:
        with self.connect() as conn:
            entry = conn.execute(
                """SELECT n.hints_json FROM notebook_entries n LEFT JOIN sessions s ON s.id=n.session_id
                WHERE n.id=? AND (n.session_id IS NULL OR s.deleted_at IS NULL)""",
                (entry_id,),
            ).fetchone()
            if not entry:
                raise LookupError("Question not found")
            attempt_id = secrets.token_hex(16)
            conn.execute(
                "INSERT INTO practice_attempts(attempt_id, entry_id, started_at) VALUES(?, ?, ?)",
                (attempt_id, entry_id, time.time()),
            )
        return {"attempt_id": attempt_id, "rungs_available": min(len(_hints(entry)), 4)}

    @staticmethod
    def _open_attempt(conn: sqlite3.Connection, attempt_id: str) -> sqlite3.Row:
        """The attempt, locked for this transaction; it must not be checked yet."""
        conn.execute("BEGIN IMMEDIATE")
        attempt = conn.execute(_ATTEMPT_SQL, (attempt_id,)).fetchone()
        if not attempt:
            raise LookupError("Attempt not found")
        if attempt["closed_at"] is not None:
            raise AttemptConflict("This attempt was already checked")
        return attempt

    def set_confidence(self, attempt_id: str, level: int) -> None:
        with self.connect() as conn:
            self._open_attempt(conn, attempt_id)
            conn.execute(
                "UPDATE practice_attempts SET confidence=? WHERE attempt_id=?", (level, attempt_id)
            )

    def next_hint(self, attempt_id: str) -> dict:
        with self.connect() as conn:
            attempt = self._open_attempt(conn, attempt_id)
            rung = attempt["hint_rung_max"] + 1
            if rung > ANSWER_RUNG:
                raise AttemptConflict("The answer was already revealed")
            if rung == ANSWER_RUNG:
                text = attempt["correct_answer"] or ""
            else:
                hints = _hints(attempt)
                text = hints[rung - 1] if rung <= len(hints) else None
            conn.execute(
                "UPDATE practice_attempts SET hint_rung_max=?, revealed=? WHERE attempt_id=?",
                (rung, int(rung == ANSWER_RUNG), attempt_id),
            )
        return {"rung": rung, "text": text, "is_answer": rung == ANSWER_RUNG}

    def check_attempt(self, attempt_id: str, answer: str) -> dict:
        with self.connect() as conn:
            attempt = self._open_attempt(conn, attempt_id)
            correct, diagnostic = grade(dict(attempt), answer)
            conn.execute(
                "UPDATE practice_attempts SET answer=?, correct=?, closed_at=? WHERE attempt_id=?",
                (answer, None if correct is None else int(correct), time.time(), attempt_id),
            )
        # The diagnostic is additive: present only when there is one (contract C10).
        return {
            **({"diagnostic": diagnostic} if diagnostic else {}),
            "correct": correct,
            "rating": None
            if correct is None
            else attempt_rating(correct, attempt["hint_rung_max"], bool(attempt["revealed"])),
            "correct_answer": attempt["correct_answer"] or "",
            "explanation": attempt["explanation"] or "",
            "misconception": attempt["misconception"] or None,
        }

    def stage_import(
        self, filename: str, target: str, questions: list[dict], course_id: str = ""
    ) -> str:
        token, now = uuid.uuid4().hex, time.time()
        with self.connect() as conn:
            conn.execute(
                "DELETE FROM practice_imports WHERE created_at < ? AND result_json IS NULL",
                (now - DAY,),
            )
            conn.execute(
                "INSERT INTO practice_imports VALUES(?, ?, ?, ?, ?, NULL)",
                (
                    token,
                    filename[:200],
                    target,
                    json.dumps(
                        {"questions": questions, "course_id": course_id}, ensure_ascii=False
                    ),
                    now,
                ),
            )
        return token

    def commit_import(self, token: str) -> dict:
        now = time.time()
        with self.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            staged = conn.execute(
                "SELECT * FROM practice_imports WHERE token=?", (token,)
            ).fetchone()
            if not staged:
                raise LookupError("Import preview not found. Select the file again.")
            if staged["result_json"]:
                return json.loads(staged["result_json"])
            if now - staged["created_at"] > DAY:
                raise ValueError("Import preview expired. Select the file again.")
            payload = json.loads(staged["payload_json"])
            questions = payload if isinstance(payload, list) else payload["questions"]
            # All practice-file imports share one durable origin namespace.
            # ``question_id`` is a content hash, so re-importing the same
            # question remains idempotent without tying ownership to an
            # ephemeral preview receipt or a synthetic chat session.
            origin_ref = "practice-import"
            created, skipped = 0, 0
            for question in questions:
                cursor = conn.execute(
                    """
                    INSERT OR IGNORE INTO notebook_entries(
                        session_id, origin_type, origin_ref, question_id, question, question_type,
                        options_json, correct_answer, explanation, misconception, hints_json,
                        accepted_answers_json, difficulty, user_answer, source, result,
                        assessment_type, created_at, updated_at, material_title, material_id)
                    VALUES(NULL, 'external_import', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'import', ?, 'quiz', ?, ?, ?, ?)
                """,
                    (
                        origin_ref,
                        question["question_id"],
                        question["question"],
                        question["question_type"],
                        json.dumps(question["options"], ensure_ascii=False),
                        question["correct_answer"],
                        question["explanation"],
                        # Previews staged before this field existed lack the key.
                        question.get("misconception"),
                        json.dumps(question.get("hints") or [], ensure_ascii=False),
                        json.dumps(question.get("accepted_answers") or [], ensure_ascii=False),
                        question["difficulty"],
                        question["user_answer"],
                        "incorrect" if staged["target"] == "mistakes" else "ungraded",
                        now,
                        now,
                        staged["filename"],
                        "import:" + token,
                    ),
                )
                created += cursor.rowcount
                skipped += 1 - cursor.rowcount
                entry_id = conn.execute(
                    """SELECT id FROM notebook_entries
                    WHERE origin_type='external_import' AND origin_ref=?
                      AND question_id=? AND turn_id=''""",
                    (origin_ref, question["question_id"]),
                ).fetchone()[0]
                if staged["target"] == "mistakes":
                    conn.execute(
                        "INSERT OR IGNORE INTO practice_review_state(entry_id, first_wrong_at, due_at) VALUES(?, ?, ?)",
                        (entry_id, now, now + DAY),
                    )
                    conn.execute(
                        """UPDATE practice_review_state SET is_mistake=1, first_wrong_at=?,
                            due_at=?, interval_days=1, version=version+1
                            WHERE entry_id=? AND is_mistake=0""",
                        (now, now + DAY, entry_id),
                    )
                for tag in question["tags"]:
                    conn.execute(
                        "INSERT OR IGNORE INTO notebook_categories(name, created_at) VALUES(?, ?)",
                        (tag, now),
                    )
                    conn.execute(
                        "INSERT OR IGNORE INTO notebook_entry_categories(entry_id, category_id) SELECT ?, id FROM notebook_categories WHERE name=?",
                        (entry_id, tag),
                    )
            result = {"created": created, "duplicates": skipped, "total": created + skipped}
            conn.execute(
                "UPDATE practice_imports SET result_json=?, payload_json='[]' WHERE token=?",
                (json.dumps(result), token),
            )
        return result

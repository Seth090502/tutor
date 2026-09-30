"""Guided Reading sessions in the user's notebook database, beside the practice tables.

The tables are created idempotently (``CREATE ... IF NOT EXISTS``), the same way the practice
attempts table was added, the first time a store touches a database in this process. The
preparation, including prequestion keys and idea-unit texts, stays in ``prep_json`` and never
leaves the server except through :mod:`deeptutor.services.reading.service`'s public views.
Accepted atoms become ordinary practice entries in ``notebook_entries``.
"""

from __future__ import annotations

from contextlib import contextmanager
import json
from pathlib import Path
import sqlite3
import threading
import time

_initialized: set[str] = set()
_init_lock = threading.Lock()

# Accepted atoms: one practice entry per (session, atom), so accepting again is idempotent.
ATOM_ORIGIN_TYPE = "document_analysis"
ATOM_SOURCE = "immersive_reading"


class ReadingConflict(ValueError):
    """The session is not at the step this request needs."""


def initialize_reading(conn: sqlite3.Connection) -> None:
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS reading_sessions (
            session_id TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL,
            doc_title TEXT NOT NULL,
            heading_path_json TEXT NOT NULL,
            chunks_json TEXT NOT NULL,
            ordinal_start INTEGER NOT NULL,
            page_start INTEGER,
            page_end INTEGER,
            created_at REAL NOT NULL,
            prep_json TEXT,
            prepared_at REAL,
            recall_text TEXT,
            recall_at REAL,
            summary_text TEXT,
            summary_judgement INTEGER,
            scoring_json TEXT,
            summary_at REAL
        );
        CREATE INDEX IF NOT EXISTS idx_reading_sessions_doc ON reading_sessions(doc_id, created_at);
        CREATE TABLE IF NOT EXISTS reading_prequestion_answers (
            session_id TEXT NOT NULL REFERENCES reading_sessions(session_id) ON DELETE CASCADE,
            qid TEXT NOT NULL,
            answer TEXT NOT NULL,
            confidence INTEGER NOT NULL,
            correct INTEGER,
            answered_at REAL NOT NULL,
            PRIMARY KEY(session_id, qid)
        );
    """)


class ReadingStore:
    def __init__(self, db_path: Path):
        self.db_path = db_path

    @contextmanager
    def connect(self):
        conn = sqlite3.connect(self.db_path, timeout=30)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA foreign_keys = ON")
        try:
            key = str(Path(self.db_path).resolve())
            if key not in _initialized:
                with _init_lock:
                    initialize_reading(conn)
                    _initialized.add(key)
            with conn:
                yield conn
        finally:
            conn.close()

    def create(self, session: dict) -> None:
        with self.connect() as conn:
            conn.execute(
                """INSERT INTO reading_sessions(session_id, doc_id, doc_title, heading_path_json,
                    chunks_json, ordinal_start, page_start, page_end, created_at)
                VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    session["session_id"],
                    session["doc_id"],
                    session["doc_title"],
                    json.dumps(session["heading_path"], ensure_ascii=False),
                    json.dumps(session["chunks"], ensure_ascii=False),
                    session["ordinal_start"],
                    session["page_start"],
                    session["page_end"],
                    time.time(),
                ),
            )

    def get(self, session_id: str) -> dict:
        with self.connect() as conn:
            row = conn.execute(
                "SELECT * FROM reading_sessions WHERE session_id=?", (session_id,)
            ).fetchone()
        if row is None:
            raise LookupError("Reading session not found")
        session = dict(row)
        session["heading_path"] = json.loads(session.pop("heading_path_json"))
        session["chunks"] = json.loads(session.pop("chunks_json"))
        session["prep"] = json.loads(session.pop("prep_json") or "null")
        return session

    def save_preparation(self, session_id: str, prep_json: str) -> None:
        """Keep the first stored preparation; a concurrent second one is discarded."""
        with self.connect() as conn:
            conn.execute(
                """UPDATE reading_sessions SET prep_json=?, prepared_at=?
                WHERE session_id=? AND prep_json IS NULL""",
                (prep_json, time.time(), session_id),
            )

    def answer_prequestion(
        self, session_id: str, qid: str, answer: str, confidence: int, correct: bool | None
    ) -> None:
        with self.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            row = conn.execute(
                "SELECT recall_at FROM reading_sessions WHERE session_id=?", (session_id,)
            ).fetchone()
            if row["recall_at"] is not None:
                raise ReadingConflict("Prequestions are answered before reading, not after recall")
            cursor = conn.execute(
                """INSERT OR IGNORE INTO reading_prequestion_answers
                VALUES(?, ?, ?, ?, ?, ?)""",
                (
                    session_id,
                    qid,
                    answer,
                    confidence,
                    None if correct is None else int(correct),
                    time.time(),
                ),
            )
            if cursor.rowcount == 0:
                raise ReadingConflict("This prequestion was already answered")

    def save_recall(self, session_id: str, text: str) -> None:
        with self.connect() as conn:
            cursor = conn.execute(
                """UPDATE reading_sessions SET recall_text=?, recall_at=?
                WHERE session_id=? AND recall_at IS NULL""",
                (text, time.time(), session_id),
            )
            if cursor.rowcount == 0:
                raise ReadingConflict("The recall was already recorded")

    def save_summary(self, session_id: str, text: str, judgement: int, scoring: dict) -> None:
        with self.connect() as conn:
            cursor = conn.execute(
                """UPDATE reading_sessions SET summary_text=?, summary_judgement=?,
                    scoring_json=?, summary_at=?
                WHERE session_id=? AND summary_at IS NULL""",
                (text, judgement, json.dumps(scoring), time.time(), session_id),
            )
            if cursor.rowcount == 0:
                raise ReadingConflict("The summary was already scored")

    def add_atom_entries(self, session: dict, atoms: list[dict]) -> list[int]:
        """One ungraded practice entry per atom (a new card), tagged with its source chunk."""
        now = time.time()
        origin_ref = f"guided-reading:{session['session_id']}"
        pages = {c["chunk_id"]: c for c in session["chunks"]}
        entry_ids = []
        with self.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            for atom in atoms:
                chunk = pages[atom["chunk_id"]]
                conn.execute(
                    """INSERT OR IGNORE INTO notebook_entries(
                        session_id, origin_type, origin_ref, question_id, question,
                        question_type, options_json, correct_answer, explanation, source,
                        material_id, material_title, section_id, section_title, result,
                        assessment_type, created_at, updated_at)
                    VALUES(NULL, ?, ?, ?, ?, 'short_answer', '{}', ?, ?, ?, ?, ?, ?, ?,
                        'ungraded', 'quiz', ?, ?)""",
                    (
                        ATOM_ORIGIN_TYPE,
                        origin_ref,
                        atom["id"],
                        atom["front"],
                        atom["back"],
                        _citation(session, chunk),
                        ATOM_SOURCE,
                        f"library:{session['doc_id']}",
                        session["doc_title"],
                        chunk["chunk_id"],
                        " > ".join(session["heading_path"]),
                        now,
                        now,
                    ),
                )
                entry_id = conn.execute(
                    """SELECT id FROM notebook_entries WHERE origin_type=? AND origin_ref=?
                    AND turn_id='' AND question_id=?""",
                    (ATOM_ORIGIN_TYPE, origin_ref, atom["id"]),
                ).fetchone()[0]
                tags = [f"chunk:{chunk['chunk_id']}"] + (
                    ["provisional"] if atom["provisional"] else []
                )
                for tag in tags:
                    conn.execute(
                        "INSERT OR IGNORE INTO notebook_categories(name, created_at) VALUES(?, ?)",
                        (tag, now),
                    )
                    conn.execute(
                        """INSERT OR IGNORE INTO notebook_entry_categories(entry_id, category_id)
                        SELECT ?, id FROM notebook_categories WHERE name=?""",
                        (entry_id, tag),
                    )
                entry_ids.append(entry_id)
        return entry_ids


def _citation(session: dict, chunk: dict) -> str:
    where = session["doc_title"]
    if chunk.get("page_start") is not None:
        where += f", p. {chunk['page_start']}-{chunk.get('page_end') or chunk['page_start']}"
    if session["heading_path"]:
        where += ", " + " > ".join(session["heading_path"])
    return f"From {where}. [c:{chunk['chunk_id']}]"

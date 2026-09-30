"""Learning sessions and attempts driven by commands (contract C12).

The practice store is the single writer of this state. One SQLite transaction checks a command, appends
its events, updates the attempt and, when an attempt closes with a known first outcome, applies that
outcome's FSRS review through ``PracticeStore.apply_review``. A replayed command id returns the stored
result, and the review request id ``ev:<attempt_id>`` makes a second review of one attempt impossible.

The first response is sealed when it is committed. Checks, repairs and assistance are appended and never
rewritten. Answer keys, unrequested hints and explanations never enter events or open-attempt state; they
are read from ``notebook_entries`` only when an attempt releases them.
"""

from __future__ import annotations

import hashlib
import json
import re
import sqlite3
import time
from typing import Any, Callable
import uuid

from .answers import FREE_RESPONSE, entry_language, grade
from .attempts import POLICY_VERSION
from .storage import _DUE_AT_SQL, _LANG_SQL, QUEUE_EXCLUSIONS_SQL, PracticeStore

SCHEMA_VERSION = 1
CHECKER_VERSION = "1"
MODES = ("practice", "assessment", "instruction")
SOURCES = ("ui", "voice_tool")
# Ladder rungs 1-4 carry the question's written hints; the answer is give_up, not a rung.
LADDER_KINDS = {1: "nudge", 2: "hint", 3: "step", 4: "example"}
LADDER_TOP = 4
EXTERNAL_ASSISTANCE = ("gloss", "source_open", "other")
MAX_REPAIRS = 2
MAX_RESPONSE = 20000
MAX_TRANSCRIPT = 4000
# A hint or answer seen this recently makes the next attempt on the item relearning, not evidence.
EXPOSURE_WINDOW = 24 * 3600
LIVE = ("open", "committed", "ready", "remediating", "awaiting_self_assessment", "submitted")
# Submitted assessment answers wait for the session to close; they do not block the next item.
BLOCKING = tuple(status for status in LIVE if status != "submitted")
KINDS = (
    "start_session",
    "open_item",
    "request_assistance",
    "propose_response",
    "commit_response",
    "record_confidence",
    "check",
    "self_assess",
    "submit_repair",
    "give_up",
    "abandon",
    "invalidate_item",
    "close_session",
)
_HEX32 = re.compile(r"^[0-9a-f]{32}$")


class CommandError(Exception):
    """A refused command. Nothing was written."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        retryable: bool = False,
        current_version: int | None = None,
    ):
        super().__init__(message)
        self.code = code
        self.message = message
        self.retryable = retryable
        self.current_version = current_version

    def body(self) -> dict:
        body = {"code": self.code, "message": self.message, "retryable": self.retryable}
        if self.current_version is not None:
            body["current_version"] = self.current_version
        return body


def _invalid(message: str) -> CommandError:
    return CommandError("invalid_input", message)


def _transition(message: str) -> CommandError:
    return CommandError("invalid_transition", message)


def initialize_loop(conn: sqlite3.Connection) -> None:
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS learning_sessions (
            session_id TEXT PRIMARY KEY,
            mode TEXT NOT NULL,
            status TEXT NOT NULL,
            policy_version TEXT NOT NULL,
            curriculum_version TEXT NOT NULL DEFAULT '',
            version INTEGER NOT NULL,
            last_event_seq INTEGER NOT NULL,
            started_at REAL NOT NULL,
            closed_at REAL
        );
        CREATE INDEX IF NOT EXISTS idx_learning_sessions_status
            ON learning_sessions(status, started_at);
        CREATE TABLE IF NOT EXISTS learning_attempts (
            attempt_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES learning_sessions(session_id),
            entry_id INTEGER NOT NULL,
            status TEXT NOT NULL,
            opened_at REAL NOT NULL,
            released_at REAL,
            exposed_at REAL,
            state_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_learning_attempts_session
            ON learning_attempts(session_id, opened_at);
        CREATE INDEX IF NOT EXISTS idx_learning_attempts_entry
            ON learning_attempts(entry_id, exposed_at);
        CREATE TABLE IF NOT EXISTS learning_events (
            event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL UNIQUE,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            attempt_id TEXT,
            kind TEXT NOT NULL,
            source TEXT NOT NULL,
            command_id TEXT NOT NULL,
            recorded_at REAL NOT NULL,
            payload_json TEXT NOT NULL,
            UNIQUE(session_id, seq)
        );
        CREATE TABLE IF NOT EXISTS learning_commands (
            command_id TEXT PRIMARY KEY,
            payload_hash TEXT NOT NULL,
            result_json TEXT NOT NULL,
            recorded_at REAL NOT NULL
        );
        CREATE TRIGGER IF NOT EXISTS learning_events_no_update
        BEFORE UPDATE ON learning_events
        BEGIN SELECT RAISE(ABORT, 'learning events are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS learning_events_no_delete
        BEFORE DELETE ON learning_events
        BEGIN SELECT RAISE(ABORT, 'learning events are append-only'); END;
        CREATE TABLE IF NOT EXISTS learning_meta (key TEXT PRIMARY KEY, value TEXT);
    """)
    # Identifies this notebook database to the Rust projections (C13); generated once.
    conn.execute(
        "INSERT OR IGNORE INTO learning_meta(key, value) VALUES('store_id', ?)",
        (str(uuid.uuid4()),),
    )


def item_version(entry: sqlite3.Row | dict) -> str:
    """A digest of everything that decides what the question asks and what counts as correct."""
    parts = [
        entry["question"],
        entry["question_type"],
        entry["options_json"],
        entry["correct_answer"],
        entry["accepted_answers_json"],
        entry["hints_json"],
    ]
    return hashlib.sha256(json.dumps(parts, ensure_ascii=False).encode()).hexdigest()[:16]


def _digest(command: dict) -> str:
    keys = ("kind", "session_id", "attempt_id", "expected_version", "payload")
    canonical = json.dumps({k: command.get(k) for k in keys}, sort_keys=True, ensure_ascii=False)
    return hashlib.sha256(canonical.encode()).hexdigest()


def validate_envelope(command: Any) -> dict:
    """The C12 envelope, or ``invalid_input``. Payload fields are checked by each kind."""
    if not isinstance(command, dict):
        raise _invalid("The command must be a JSON object")
    if command.get("schema_version") != SCHEMA_VERSION:
        raise _invalid(f"schema_version must be {SCHEMA_VERSION}")
    try:
        command_id = str(uuid.UUID(str(command.get("command_id"))))
    except ValueError as exc:
        raise _invalid("command_id must be a UUID") from exc
    kind = command.get("kind")
    if kind not in KINDS:
        raise _invalid(f"Unknown command kind {kind!r}")
    for key in ("session_id", "attempt_id"):
        value = command.get(key)
        if value is not None and (not isinstance(value, str) or not _HEX32.match(value)):
            raise _invalid(f"{key} must be 32 lowercase hex characters")
    expected = command.get("expected_version")
    if expected is not None and (type(expected) is not int or expected < 0):
        raise _invalid("expected_version must be a non-negative integer or null")
    payload = command.get("payload", {})
    if not isinstance(payload, dict):
        raise _invalid("payload must be an object")
    if kind == "start_session":
        if command.get("session_id") is not None or expected is not None:
            raise _invalid("start_session takes no session_id and a null expected_version")
    else:
        if command.get("session_id") is None or expected is None:
            raise _invalid("session_id and expected_version are required")
    return {
        "schema_version": SCHEMA_VERSION,
        "command_id": command_id,
        "session_id": command.get("session_id"),
        "attempt_id": command.get("attempt_id"),
        "expected_version": expected,
        "kind": kind,
        "payload": payload,
    }


def _text(payload: dict, key: str, limit: int, *, required: bool = True) -> str:
    value = payload.get(key, "")
    if not isinstance(value, str):
        raise _invalid(f"{key} must be text")
    if len(value) > limit:
        raise _invalid(f"{key} is longer than {limit} characters")
    if required and not value.strip():
        raise _invalid(f"{key} is required")
    return value


class _Command:
    """One command's transaction: the loaded session and attempt, and the events to append."""

    def __init__(self, conn: sqlite3.Connection, command: dict, source: str, now: float):
        self.conn = conn
        self.command = command
        self.payload = command["payload"]
        self.source = source
        self.now = now
        self.session: dict | None = None
        self.attempt: dict | None = None
        self.events: list[dict] = []
        self.touched: dict[str, dict] = {}

    # ---- loading ------------------------------------------------------------------------------

    def load_session(self) -> dict:
        row = self.conn.execute(
            "SELECT * FROM learning_sessions WHERE session_id=?", (self.command["session_id"],)
        ).fetchone()
        if not row:
            raise CommandError("not_found", "Learning session not found")
        self.session = dict(row)
        if self.session["version"] != self.command["expected_version"]:
            raise CommandError(
                "conflict",
                "The session changed since you last read it. Reload it and try again.",
                retryable=True,
                current_version=self.session["version"],
            )
        if self.session["status"] != "active":
            raise _transition("This session is closed")
        return self.session

    def load_attempt(self, *, required: bool = True) -> dict | None:
        attempt_id = self.command["attempt_id"]
        if attempt_id is None:
            if required:
                raise _invalid("attempt_id is required")
            return None
        row = self.conn.execute(
            "SELECT state_json FROM learning_attempts WHERE attempt_id=? AND session_id=?",
            (attempt_id, self.session["session_id"]),
        ).fetchone()
        if not row:
            raise CommandError("not_found", "Attempt not found in this session")
        self.attempt = json.loads(row["state_json"])
        return self.attempt

    def entry(self, entry_id: int) -> sqlite3.Row:
        row = self.conn.execute(
            f"""SELECT n.*, {_LANG_SQL} FROM notebook_entries n
            LEFT JOIN sessions s ON s.id=n.session_id
            WHERE n.id=? AND (n.session_id IS NULL OR s.deleted_at IS NULL)""",  # nosec B608
            (entry_id,),
        ).fetchone()
        if not row:
            raise CommandError("not_found", "Question not found")
        return row

    def live_attempts(self, statuses: tuple[str, ...] = LIVE) -> list[dict]:
        rows = self.conn.execute(
            f"""SELECT state_json FROM learning_attempts WHERE session_id=?
            AND status IN ({",".join("?" for _ in statuses)}) ORDER BY opened_at""",  # nosec B608
            (self.session["session_id"], *statuses),
        ).fetchall()
        return [json.loads(row["state_json"]) for row in rows]

    # ---- writing ------------------------------------------------------------------------------

    def emit(self, kind: str, payload: dict, attempt: dict | None = None) -> None:
        self.events.append(
            {
                "event_id": uuid.uuid4().hex,
                "kind": kind,
                "attempt_id": attempt["attempt_id"] if attempt else None,
                "recorded_at": self.now,
                "source": self.source,
                "payload": payload,
            }
        )

    def save(self, attempt: dict) -> None:
        self.touched[attempt["attempt_id"]] = attempt

    def finish(self) -> dict:
        session = self.session
        for attempt in self.touched.values():
            self.conn.execute(
                """INSERT INTO learning_attempts(attempt_id, session_id, entry_id, status, opened_at,
                    released_at, exposed_at, state_json) VALUES(?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(attempt_id) DO UPDATE SET status=excluded.status,
                    released_at=excluded.released_at, exposed_at=excluded.exposed_at,
                    state_json=excluded.state_json""",
                (
                    attempt["attempt_id"],
                    attempt["session_id"],
                    attempt["entry_id"],
                    attempt["status"],
                    attempt["opened_at"],
                    attempt.get("released_at"),
                    attempt.get("exposed_at"),
                    json.dumps(attempt, ensure_ascii=False),
                ),
            )
        seq = session["last_event_seq"]
        for event in self.events:
            seq += 1
            event["seq"] = seq
            self.conn.execute(
                """INSERT INTO learning_events(event_id, session_id, seq, attempt_id, kind, source,
                    command_id, recorded_at, payload_json) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    event["event_id"],
                    session["session_id"],
                    seq,
                    event["attempt_id"],
                    event["kind"],
                    event["source"],
                    self.command["command_id"],
                    event["recorded_at"],
                    json.dumps(event["payload"], ensure_ascii=False),
                ),
            )
        session["version"] += 1
        session["last_event_seq"] = seq
        self.conn.execute(
            """UPDATE learning_sessions SET status=?, closed_at=?, version=?, last_event_seq=?
            WHERE session_id=?""",
            (
                session["status"],
                session["closed_at"],
                session["version"],
                seq,
                session["session_id"],
            ),
        )
        attempt = self.attempt
        return {
            "schema_version": SCHEMA_VERSION,
            "command_id": self.command["command_id"],
            "aggregate_version": session["version"],
            "last_event_seq": seq,
            "public_state": {
                "session": public_session(session),
                "attempt": public_attempt(self.conn, attempt) if attempt else None,
            },
            "events": [public_event(event) for event in self.events],
        }


# ---- public views ------------------------------------------------------------------------------


def public_session(session: dict) -> dict:
    return {
        key: session[key]
        for key in (
            "session_id",
            "mode",
            "status",
            "policy_version",
            "curriculum_version",
            "version",
            "last_event_seq",
            "started_at",
            "closed_at",
        )
    }


def public_event(event: dict) -> dict:
    return {
        key: event[key]
        for key in ("event_id", "seq", "kind", "attempt_id", "recorded_at", "source", "payload")
    }


def _released(conn: sqlite3.Connection, entry_id: int) -> dict:
    row = conn.execute(
        """SELECT correct_answer, accepted_answers_json, explanation, misconception
        FROM notebook_entries WHERE id=?""",
        (entry_id,),
    ).fetchone()
    if not row:
        return {
            "correct_answer": "",
            "accepted_answers": [],
            "explanation": "",
            "misconception": None,
        }
    return {
        "correct_answer": row["correct_answer"] or "",
        "accepted_answers": json.loads(row["accepted_answers_json"] or "[]"),
        "explanation": row["explanation"] or "",
        "misconception": row["misconception"] or None,
    }


def public_attempt(conn: sqlite3.Connection, attempt: dict) -> dict:
    """What the learner, the UI and the teaching model may see now. Keys appear only once released."""
    view = {key: value for key, value in attempt.items() if key not in {"pending_review"}}
    hidden = attempt["mode"] == "assessment" and not attempt.get("released_at")
    if hidden:
        view["checks"] = [
            {**check, "verdict": "recorded", "diagnostic": None} for check in attempt["checks"]
        ]
    if attempt["status"] != "closed":
        view.pop("measurement_eligibility", None)
    view["released"] = _released(conn, attempt["entry_id"]) if attempt.get("released_at") else None
    return view


# ---- checking and the review policy ------------------------------------------------------------


def _check(entry: sqlite3.Row, response: dict, now: float) -> dict:
    record = {
        "response_id": response["response_id"],
        "checked_at": now,
        "checker_version": CHECKER_VERSION,
        "assumptions": [],
        "limitations": [],
        "evidence_refs": [],
        "diagnostic": None,
    }
    if response["input_mode"] == "dont_know":
        return {**record, "verdict": "incorrect", "checker_id": "dont_know"}
    data = dict(entry)
    kind = data.get("question_type") or ""
    options = json.loads(data.get("options_json") or "{}")
    lang = entry_language(data)
    correct, diagnostic = grade(data, response["response"])
    if lang and not options:
        checker = f"language:{lang}"
        limitations = ["Accepts the stored answer and its listed variants only."]
    elif options:
        checker, limitations = "choice", []
    else:
        checker, limitations = "none", []
    if correct is None:
        # A choice or language key that cannot be read is a broken item, not a wrong learner.
        broken = kind not in FREE_RESPONSE and (bool(options) or bool(lang))
        if broken:
            return {
                **record,
                "verdict": "invalid_item",
                "checker_id": checker,
                "limitations": ["The stored answer key could not be read."],
            }
        return {
            **record,
            "verdict": "unverifiable",
            "checker_id": "none",
            "limitations": ["No automatic checker for this answer type; the learner judges it."],
        }
    return {
        **record,
        "verdict": "correct" if correct else "incorrect",
        "checker_id": checker,
        "limitations": limitations,
        "diagnostic": diagnostic,
    }


def _review_evidence(attempt: dict, judged: str | None = None) -> dict:
    """ReviewEvidence for the first outcome (policy ``review-policy/2``)."""
    first = attempt["first_response"]
    verdict = attempt["checks"][0]["verdict"]
    assisted = bool(first["assisted_before_commit"])
    if verdict == "unverifiable":
        outcome = judged
        provenance = "self_report"
    else:
        outcome = verdict
        provenance = "assisted" if assisted else "verified"
    rating = "good" if outcome == "correct" and not assisted else "again"
    eligible = attempt["mode"] in {"practice", "assessment"} and outcome in {"correct", "incorrect"}
    return {
        "first_outcome": outcome,
        "assistance_before_check": assisted,
        "eligible_for_fsrs": eligible,
        "rating": rating if eligible else None,
        "provenance": provenance,
        "algorithm_version": "py-fsrs 6",
        "parameters_version": "fsrs-default",
        "policy_version": POLICY_VERSION,
        "applied": None,
    }


def _eligibility(attempt: dict, reason: str) -> dict:
    reasons = list(attempt["measurement_eligibility"]["reasons"])
    first = attempt["first_response"]
    if attempt["mode"] == "instruction":
        reasons.append("instruction_mode")
    if first is None or not attempt["checks"]:
        reasons.append("no_checked_response")
    else:
        if first["assisted_before_commit"]:
            reasons.append("assisted")
        if attempt["checks"][0]["verdict"] not in {"correct", "incorrect"}:
            reasons.append("unverified")
    if reason == "invalidated":
        reasons.append("invalid_item")
    reasons = sorted(set(reasons))
    return {"eligible": not reasons, "reasons": reasons}


# ---- command handlers --------------------------------------------------------------------------


def _new_response(payload: dict, *, first: bool, source: str, now: float) -> dict:
    mode = payload.get("input_mode")
    if mode not in ("typed", "spoken", "dont_know"):
        raise _invalid("input_mode must be typed, spoken or dont_know")
    if not first and mode == "dont_know":
        raise _invalid("To stop, give up instead of answering 'don't know' to a repair")
    if source != "ui":
        raise CommandError(
            "unauthorized",
            "The learner confirms answers on screen; the voice tutor can only propose",
        )
    response = _text(payload, "response", MAX_RESPONSE, required=mode != "dont_know")
    if mode == "dont_know" and response.strip():
        raise _invalid("A don't-know response has no text")
    confirmation = payload.get("transcript_confirmation")
    if mode == "spoken":
        if not isinstance(confirmation, dict):
            raise _invalid("A spoken answer needs transcript_confirmation from the learner")
        raw = _text(confirmation, "raw_transcript", MAX_TRANSCRIPT)
        if _text(confirmation, "confirmed_text", MAX_RESPONSE) != response:
            raise _invalid("confirmed_text must equal the response")
        confirmation = {"raw_transcript": raw, "confirmed_text": response}
    elif confirmation is not None:
        raise _invalid("transcript_confirmation goes only with a spoken answer")
    return {
        "response_id": uuid.uuid4().hex,
        "response": response,
        "input_mode": mode,
        "committed_at": now,
        "transcript_confirmation": confirmation,
    }


def _start_session(cmd: _Command) -> None:
    mode = cmd.payload.get("mode")
    if mode not in MODES:
        raise _invalid("mode must be practice, assessment or instruction")
    curriculum = _text(cmd.payload, "curriculum_version", 100, required=False)
    cmd.session = {
        "session_id": uuid.uuid4().hex,
        "mode": mode,
        "status": "active",
        "policy_version": POLICY_VERSION,
        "curriculum_version": curriculum,
        "version": 0,
        "last_event_seq": 0,
        "started_at": cmd.now,
        "closed_at": None,
    }
    cmd.conn.execute(
        """INSERT INTO learning_sessions(session_id, mode, status, policy_version, curriculum_version,
            version, last_event_seq, started_at) VALUES(?, ?, 'active', ?, ?, 0, 0, ?)""",
        (cmd.session["session_id"], mode, POLICY_VERSION, curriculum, cmd.now),
    )
    cmd.emit(
        "session_started",
        {"mode": mode, "policy_version": POLICY_VERSION, "curriculum_version": curriculum},
    )


def _quarantined(conn: sqlite3.Connection, entry_id: int) -> bool:
    row = conn.execute(
        "SELECT status FROM practice_item_status WHERE entry_id=?", (entry_id,)
    ).fetchone()
    return bool(row) and row["status"] == "quarantined"


def _open_item(cmd: _Command) -> None:
    if cmd.command["attempt_id"] is not None:
        raise _invalid("open_item creates the attempt; send no attempt_id")
    if cmd.live_attempts(BLOCKING):
        raise _transition("Finish, give up or abandon the current item first")
    entry_id = cmd.payload.get("entry_id")
    if type(entry_id) is not int or entry_id <= 0:
        raise _invalid("entry_id must be a positive integer")
    entry = cmd.entry(entry_id)
    if _quarantined(cmd.conn, entry_id):
        raise _transition("This item is quarantined")
    options = json.loads(entry["options_json"] or "{}")
    lang = entry_language(dict(entry))
    kind = entry["question_type"] or ""
    if cmd.session["mode"] == "assessment" and (kind in FREE_RESPONSE or not (options or lang)):
        raise _invalid("Assessment items must be automatically checkable")
    reasons = []
    since = cmd.now - EXPOSURE_WINDOW
    recent = (
        cmd.conn.execute(
            """SELECT 1 FROM learning_attempts WHERE entry_id=? AND exposed_at>=? LIMIT 1""",
            (entry_id, since),
        ).fetchone()
        or cmd.conn.execute(
            """SELECT 1 FROM practice_attempts WHERE entry_id=?
        AND (closed_at>=? OR (hint_rung_max>0 AND started_at>=?)) LIMIT 1""",
            (entry_id, since, since),
        ).fetchone()
    )
    if recent:
        reasons.append("exposed_within_24h")
    hints = json.loads(entry["hints_json"] or "[]")
    # Skills and the variant family travel with the event so projections (C13) never read items.
    tags = [
        row["name"]
        for row in cmd.conn.execute(
            """SELECT c.name FROM notebook_entry_categories ec
            JOIN notebook_categories c ON c.id=ec.category_id WHERE ec.entry_id=? ORDER BY c.name""",
            (entry_id,),
        )
    ]
    skill_ids = [t[len("skill:") :] for t in tags if t.startswith("skill:") and len(t) > 6]
    family_id = next(
        (t[len("family:") :] for t in tags if t.startswith("family:") and len(t) > 7), None
    )
    attempt = {
        "attempt_id": uuid.uuid4().hex,
        "session_id": cmd.session["session_id"],
        "entry_id": entry_id,
        "item_version": item_version(entry),
        "mode": cmd.session["mode"],
        "status": "open",
        "opened_at": cmd.now,
        "item": {
            "prompt": entry["question"],
            "question_type": kind,
            "options": options,
            "lang": lang,
            "rungs_available": 0
            if cmd.session["mode"] == "assessment"
            else min(len(hints), LADDER_TOP),
            "skill_ids": skill_ids,
            "family_id": family_id,
        },
        "first_response": None,
        "repairs": [],
        "confidence": None,
        "checks": [],
        "assistance": [],
        "pending_proposal": None,
        "pending_review": None,
        "closure": None,
        "offer": None,
        "released_at": None,
        "exposed_at": None,
        "measurement_eligibility": {"eligible": None, "reasons": reasons},
    }
    cmd.attempt = attempt
    cmd.save(attempt)
    cmd.emit(
        "item_opened",
        {
            "entry_id": entry_id,
            "item_version": attempt["item_version"],
            "skill_ids": skill_ids,
            "family_id": family_id,
            "exposure": reasons,
        },
        attempt,
    )


def _require(attempt: dict, *statuses: str) -> None:
    if attempt["status"] not in statuses:
        raise _transition(f"Not allowed while the attempt is {attempt['status']}")


def _request_assistance(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    kind = cmd.payload.get("kind")
    if kind in EXTERNAL_ASSISTANCE:
        _require(attempt, "open", "remediating")
        ref = _text(cmd.payload, "content_ref", 200, required=False) or None
        record = {"kind": kind, "rung": None, "source": cmd.source, "timestamp": cmd.now}
        record["content_ref"] = ref
        record["text"] = None
    elif kind == "hint":
        if attempt["mode"] == "assessment":
            raise _transition("No hints during an assessment")
        _require(attempt, "open", "remediating")
        rung = 1 + sum(1 for a in attempt["assistance"] if a["rung"])
        if rung > LADDER_TOP:
            raise _transition("No more hints; give up to see the answer")
        entry = cmd.entry(attempt["entry_id"])
        hints = json.loads(entry["hints_json"] or "[]")
        record = {
            "kind": LADDER_KINDS[rung],
            "rung": rung,
            "source": cmd.source,
            "timestamp": cmd.now,
            "content_ref": f"hint:{rung}",
            "text": hints[rung - 1] if rung <= len(hints) else None,
        }
    else:
        raise _invalid("kind must be hint, gloss, source_open or other")
    record["before_commit"] = attempt["first_response"] is None
    attempt["assistance"].append(record)
    if record["rung"]:
        attempt["exposed_at"] = cmd.now
    cmd.save(attempt)
    cmd.emit("assistance_given", record, attempt)


def _propose_response(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "open", "remediating")
    mode = cmd.payload.get("input_mode")
    if mode not in ("spoken", "dont_know"):
        raise _invalid("A proposal is spoken or dont_know")
    raw = _text(cmd.payload, "raw_transcript", MAX_TRANSCRIPT, required=mode == "spoken")
    attempt["pending_proposal"] = {
        "raw_transcript": raw,
        "input_mode": mode,
        "proposed_at": cmd.now,
        "source": cmd.source,
    }
    cmd.save(attempt)
    cmd.emit("response_proposed", attempt["pending_proposal"], attempt)


def _commit_response(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "open")
    response = _new_response(cmd.payload, first=True, source=cmd.source, now=cmd.now)
    response["assisted_before_commit"] = any(a["before_commit"] for a in attempt["assistance"])
    attempt["first_response"] = response
    attempt["pending_proposal"] = None
    attempt["status"] = "committed"
    cmd.save(attempt)
    cmd.emit("response_committed", response, attempt)


def _record_confidence(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "committed")
    level = cmd.payload.get("level")
    if level is None:
        reason = _text(cmd.payload, "skipped_reason", 200)
        confidence = {"level": None, "recorded_at": cmd.now, "skipped_reason": reason}
    elif type(level) is int and 1 <= level <= 4:
        confidence = {"level": level, "recorded_at": cmd.now, "skipped_reason": None}
    else:
        raise _invalid("level must be 1-4, or null with a skipped_reason")
    attempt["confidence"] = confidence
    attempt["status"] = "ready"
    cmd.save(attempt)
    cmd.emit("confidence_recorded", confidence, attempt)


def _release(cmd: _Command, attempt: dict, why: str) -> None:
    if not attempt.get("released_at"):
        attempt["released_at"] = cmd.now
        attempt["exposed_at"] = cmd.now
        cmd.emit("answer_released", {"reason": why}, attempt)


def _quarantine(cmd: _Command, entry_id: int, reason: str) -> None:
    cmd.conn.execute(
        """INSERT INTO practice_item_status(entry_id, status, reason, updated_at)
        VALUES(?, 'quarantined', ?, ?) ON CONFLICT(entry_id) DO UPDATE SET status='quarantined',
            reason=excluded.reason, updated_at=excluded.updated_at""",
        (entry_id, reason, cmd.now),
    )


def _close(cmd: _Command, attempt: dict, reason: str) -> None:
    review = None if reason == "invalidated" else attempt.get("pending_review")
    if review and review["eligible_for_fsrs"]:
        request_id = f"ev:{attempt['attempt_id']}"
        exists = cmd.conn.execute(
            "SELECT outcome_json FROM practice_review_events WHERE request_id=?", (request_id,)
        ).fetchone()
        if exists:
            outcome = json.loads(exists["outcome_json"])
        else:
            outcome = PracticeStore.apply_review(
                cmd.conn,
                attempt["entry_id"],
                request_id,
                review["rating"],
                attempt["first_response"]["response"],
                cmd.now,
                correct=review["first_outcome"] == "correct",
                self_report=review["provenance"] == "self_report",
                extra={"attempt_id": attempt["attempt_id"], "policy_version": POLICY_VERSION},
                provenance=review["provenance"],
            )
        review = {**review, "applied": {"request_id": request_id, "due_at": outcome["due_at"]}}
    disposition = review or {"eligible_for_fsrs": False, "rating": None}
    attempt["pending_review"] = None
    attempt["status"] = "closed"
    attempt["closure"] = {
        "reason": reason,
        "closed_at": cmd.now,
        "scheduler_disposition": disposition,
    }
    attempt["measurement_eligibility"] = _eligibility(attempt, reason)
    cmd.save(attempt)
    cmd.emit(
        "attempt_closed",
        {
            "reason": reason,
            "scheduler_disposition": disposition,
            "measurement_eligibility": attempt["measurement_eligibility"],
        },
        attempt,
    )
    # Abandoning releases nothing, so it is never a way to see the answer without an outcome.
    if reason in {"correct", "revealed", "self_assessed"}:
        if attempt["mode"] == "assessment" and attempt["checks"]:
            cmd.emit("checks_released", {"checks": attempt["checks"]}, attempt)
        _release(cmd, attempt, reason)


def _entry_unchanged(cmd: _Command, attempt: dict) -> sqlite3.Row:
    entry = cmd.entry(attempt["entry_id"])
    if item_version(entry) != attempt["item_version"]:
        raise CommandError(
            "conflict", "The question was edited after it was opened; abandon it and open it again"
        )
    return entry


def _check_command(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "ready")
    entry = _entry_unchanged(cmd, attempt)
    check = _check(entry, attempt["first_response"], cmd.now)
    attempt["checks"].append(check)
    verdict = check["verdict"]
    if attempt["mode"] == "assessment" and verdict != "invalid_item":
        # Events are public: the verdict is published when the session releases it.
        cmd.emit("response_checked", {**check, "verdict": "recorded", "diagnostic": None}, attempt)
    else:
        cmd.emit("response_checked", check, attempt)
    if verdict == "invalid_item":
        _quarantine(cmd, attempt["entry_id"], "The stored answer key could not be read")
        _close(cmd, attempt, "invalidated")
        return
    if verdict != "unverifiable":
        attempt["pending_review"] = _review_evidence(attempt)
    if attempt["mode"] == "assessment":
        attempt["status"] = "submitted"
        cmd.save(attempt)
    elif verdict == "correct":
        _close(cmd, attempt, "correct")
    elif verdict == "incorrect":
        attempt["status"] = "remediating"
        cmd.save(attempt)
    else:
        # The learner needs the reference answer to judge their own.
        attempt["status"] = "awaiting_self_assessment"
        _release(cmd, attempt, "self_assessment")
        cmd.save(attempt)


def _self_assess(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "awaiting_self_assessment")
    if cmd.source != "ui":
        raise CommandError("unauthorized", "Only the learner judges their own answer, on screen")
    judged = cmd.payload.get("judged")
    if judged not in ("correct", "incorrect"):
        raise _invalid("judged must be correct or incorrect")
    attempt["self_assessment"] = {"judged": judged, "recorded_at": cmd.now}
    attempt["pending_review"] = _review_evidence(attempt, judged)
    cmd.emit("self_assessed", attempt["self_assessment"], attempt)
    _close(cmd, attempt, "self_assessed")


def _submit_repair(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "remediating")
    entry = _entry_unchanged(cmd, attempt)
    response = _new_response(cmd.payload, first=False, source=cmd.source, now=cmd.now)
    attempt["repairs"].append(response)
    attempt["pending_proposal"] = None
    cmd.emit("repair_committed", response, attempt)
    check = _check(entry, response, cmd.now)
    attempt["checks"].append(check)
    cmd.emit("response_checked", check, attempt)
    if check["verdict"] == "correct":
        _close(cmd, attempt, "correct")
        return
    if len(attempt["repairs"]) >= MAX_REPAIRS and not attempt["offer"]:
        attempt["offer"] = "worked_example_then_fresh_problem"
        cmd.emit("offer_made", {"offer": attempt["offer"]}, attempt)
    cmd.save(attempt)


def _dont_know(cmd: _Command, attempt: dict) -> None:
    response = {
        "response_id": uuid.uuid4().hex,
        "response": "",
        "input_mode": "dont_know",
        "committed_at": cmd.now,
        "transcript_confirmation": None,
        "assisted_before_commit": any(a["before_commit"] for a in attempt["assistance"]),
    }
    attempt["first_response"] = response
    cmd.emit("response_committed", response, attempt)
    check = _check(cmd.entry(attempt["entry_id"]), response, cmd.now)
    attempt["checks"].append(check)
    cmd.emit("response_checked", check, attempt)
    attempt["pending_review"] = _review_evidence(attempt)


def _peeked(attempt: dict) -> bool:
    """Hints were seen and no answer followed."""
    return attempt["status"] == "open" and any(a["rung"] for a in attempt["assistance"])


def _give_up(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, "open", "remediating")
    if attempt["mode"] == "assessment":
        raise _transition("During an assessment, answer 'don't know' instead")
    if attempt["first_response"] is None:
        # Giving up before answering is an explicit don't-know: a failed retrieval.
        _dont_know(cmd, attempt)
    record = {
        "kind": "answer",
        "rung": None,
        "source": cmd.source,
        "timestamp": cmd.now,
        "content_ref": "answer",
        "text": None,
        "before_commit": False,
    }
    attempt["assistance"].append(record)
    cmd.emit("assistance_given", record, attempt)
    _close(cmd, attempt, "revealed")


def _abandon(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    _require(attempt, *LIVE)
    if attempt["status"] == "submitted":
        raise _transition("Submitted assessment answers are released when the session closes")
    if _peeked(attempt):
        # Leaving after hints is an assisted failed retrieval, not a clean exit.
        _dont_know(cmd, attempt)
    _close(cmd, attempt, "abandoned")


def _invalidate_item(cmd: _Command) -> None:
    attempt = cmd.load_attempt()
    reason = _text(cmd.payload, "reason", 500)
    _quarantine(cmd, attempt["entry_id"], reason)
    cmd.emit("item_quarantined", {"entry_id": attempt["entry_id"], "reason": reason}, attempt)
    if attempt["status"] != "closed":
        _close(cmd, attempt, "invalidated")
    else:
        cmd.save(attempt)


def _close_session(cmd: _Command) -> None:
    for attempt in cmd.live_attempts():
        if attempt["status"] == "submitted":
            verdict = attempt["checks"][0]["verdict"]
            _close(cmd, attempt, "correct" if verdict == "correct" else "revealed")
        else:
            # A known first outcome still counts; an unchecked response does not.
            if _peeked(attempt):
                _dont_know(cmd, attempt)
            _close(cmd, attempt, "abandoned")
    cmd.session["status"] = "closed"
    cmd.session["closed_at"] = cmd.now
    cmd.emit("session_closed", {})


_HANDLERS: dict[str, Callable[[_Command], None]] = {
    "start_session": _start_session,
    "open_item": _open_item,
    "request_assistance": _request_assistance,
    "propose_response": _propose_response,
    "commit_response": _commit_response,
    "record_confidence": _record_confidence,
    "check": _check_command,
    "self_assess": _self_assess,
    "submit_repair": _submit_repair,
    "give_up": _give_up,
    "abandon": _abandon,
    "invalidate_item": _invalidate_item,
    "close_session": _close_session,
}


class LearningLoop:
    def __init__(self, store: PracticeStore):
        self.store = store

    def execute(self, raw: Any, source: str, now: float | None = None) -> dict:
        """Run one command; the same command id and content returns the stored result."""
        if source not in SOURCES:
            raise _invalid("Unknown command source")
        command = validate_envelope(raw)
        digest = _digest(command)
        with self.store.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            stored = conn.execute(
                "SELECT payload_hash, result_json FROM learning_commands WHERE command_id=?",
                (command["command_id"],),
            ).fetchone()
            if stored:
                if stored["payload_hash"] != digest:
                    raise _invalid("This command_id was already used for a different command")
                return json.loads(stored["result_json"])
            cmd = _Command(conn, command, source, time.time() if now is None else now)
            if command["kind"] != "start_session":
                cmd.load_session()
            _HANDLERS[command["kind"]](cmd)
            result = cmd.finish()
            conn.execute(
                "INSERT INTO learning_commands VALUES(?, ?, ?, ?)",
                (command["command_id"], digest, json.dumps(result, ensure_ascii=False), cmd.now),
            )
        return result

    def session(self, session_id: str, after_seq: int = 0) -> dict:
        with self.store.connect() as conn:
            row = conn.execute(
                "SELECT * FROM learning_sessions WHERE session_id=?", (session_id,)
            ).fetchone()
            if not row:
                raise CommandError("not_found", "Learning session not found")
            attempts = conn.execute(
                "SELECT state_json FROM learning_attempts WHERE session_id=? ORDER BY opened_at",
                (session_id,),
            ).fetchall()
            events = conn.execute(
                """SELECT * FROM learning_events WHERE session_id=? AND seq>? ORDER BY seq""",
                (session_id, after_seq),
            ).fetchall()
            return {
                "session": public_session(dict(row)),
                "attempts": [public_attempt(conn, json.loads(a["state_json"])) for a in attempts],
                "events": [
                    public_event({**dict(e), "payload": json.loads(e["payload_json"])})
                    for e in events
                ],
                "last_event_seq": row["last_event_seq"],
            }

    def active_session(self) -> dict | None:
        with self.store.connect() as conn:
            row = conn.execute(
                """SELECT session_id FROM learning_sessions WHERE status='active'
                ORDER BY started_at DESC LIMIT 1"""
            ).fetchone()
        return self.session(row["session_id"]) if row else None

    def queue(
        self, limit: int = 20, tag: str | None = None, now: float | None = None
    ) -> list[dict]:
        """Due items with public fields only: never the key, hints or explanation."""
        now = time.time() if now is None else now
        where, params = "", []
        if tag:
            where = """AND EXISTS(SELECT 1 FROM notebook_entry_categories tc
                JOIN notebook_categories tn ON tn.id=tc.category_id
                WHERE tc.entry_id=n.id AND lower(tn.name)=lower(?))"""
            params.append(tag)
        with self.store.connect() as conn:
            rows = conn.execute(
                f"""SELECT n.id, n.question, n.question_type, n.options_json, {_LANG_SQL},
                    {_DUE_AT_SQL} AS due_at FROM notebook_entries n
                LEFT JOIN sessions s ON s.id=n.session_id
                LEFT JOIN practice_review_state r ON r.entry_id=n.id
                WHERE (n.session_id IS NULL OR s.deleted_at IS NULL) AND {QUEUE_EXCLUSIONS_SQL}
                    AND {_DUE_AT_SQL} <= ? {where}
                ORDER BY {_DUE_AT_SQL}, n.id LIMIT ?""",  # nosec B608 - fixed SQL, values bound
                [now, *params, limit],
            ).fetchall()
            items = []
            for row in rows:
                tags = [
                    t["name"]
                    for t in conn.execute(
                        """SELECT c.name FROM notebook_entry_categories ec
                        JOIN notebook_categories c ON c.id=ec.category_id WHERE ec.entry_id=?
                        ORDER BY c.name""",
                        (row["id"],),
                    )
                ]
                items.append(
                    {
                        "entry_id": row["id"],
                        "prompt": row["question"],
                        "question_type": row["question_type"] or "",
                        "options": json.loads(row["options_json"] or "{}"),
                        "lang": row["lang"],
                        "tags": tags,
                        "due_at": row["due_at"],
                    }
                )
        return items

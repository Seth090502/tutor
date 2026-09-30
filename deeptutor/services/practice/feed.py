"""Read-only feed and candidate list for the Rust learner projections (contract C13).

Pure data access: one SQLite connection per call, no writes. Events leave exactly as the loop stored
them (public-safe payloads); candidates carry scheduling state and tags, never keys, hints or
explanations.
"""

from __future__ import annotations

from datetime import datetime, timezone
import json
import time

from .scheduler import _SCHEDULER, _card
from .storage import _DUE_AT_SQL, _LANG_SQL, QUEUE_EXCLUSIONS_SQL, PracticeStore

SCHEMA_VERSION = 1
_SPECIAL_PREFIXES = ("lang:", "card:", "skill:", "family:", "chunk:")


def _store_id(conn) -> str:
    return conn.execute("SELECT value FROM learning_meta WHERE key='store_id'").fetchone()[0]


def feed(store: PracticeStore, after_event_seq: int = 0, limit: int = 500) -> dict:
    """Events with ``event_seq`` greater than ``after_event_seq``, oldest first."""
    with store.connect() as conn:
        rows = conn.execute(
            """SELECT event_seq, session_id, seq, attempt_id, kind, recorded_at, source, payload_json
            FROM learning_events WHERE event_seq > ? ORDER BY event_seq LIMIT ?""",
            (after_event_seq, limit),
        ).fetchall()
        store_id = _store_id(conn)
    events = [
        {
            **{k: row[k] for k in row.keys() if k != "payload_json"},
            "payload": json.loads(row["payload_json"]),
        }
        for row in rows
    ]
    return {
        "schema_version": SCHEMA_VERSION,
        "store_id": store_id,
        "events": events,
        "next_after": events[-1]["event_seq"] if events else after_event_seq,
    }


def track_of(lang: str | None, tags: list[str]) -> str:
    """``lang:<code>`` for a language card, else the first ordinary tag lowercased, else ``general``."""
    if lang:
        return f"lang:{lang}"
    for tag in tags:
        name = tag.lower()
        if name and name != "provisional" and not name.startswith(_SPECIAL_PREFIXES):
            return name
    return "general"


def candidates(store: PracticeStore, now: float | None = None) -> dict:
    """Every visible item that is neither quarantined nor provisional, with its FSRS state."""
    now = time.time() if now is None else now
    at = datetime.fromtimestamp(now, timezone.utc)
    with store.connect() as conn:
        rows = conn.execute(
            f"""SELECT n.id, n.question_type, {_LANG_SQL}, {_DUE_AT_SQL} AS due_at,
                r.fsrs_stability, r.fsrs_difficulty, r.fsrs_state, r.fsrs_step,
                COALESCE(r.review_count, 0) AS review_count, r.last_review_at
            FROM notebook_entries n LEFT JOIN sessions s ON s.id=n.session_id
            LEFT JOIN practice_review_state r ON r.entry_id=n.id
            WHERE (n.session_id IS NULL OR s.deleted_at IS NULL) AND {QUEUE_EXCLUSIONS_SQL}
            ORDER BY n.id""",  # nosec B608 - fixed SQL fragments, no caller input
        ).fetchall()
        tags: dict[int, list[str]] = {}
        for row in conn.execute(
            """SELECT ec.entry_id, c.name FROM notebook_entry_categories ec
            JOIN notebook_categories c ON c.id=ec.category_id ORDER BY ec.entry_id, c.name"""
        ):
            tags.setdefault(row["entry_id"], []).append(row["name"])
        store_id = _store_id(conn)
    items = []
    for row in rows:
        names = tags.get(row["id"], [])
        has_state = row["fsrs_state"] is not None and row["fsrs_stability"] is not None
        items.append(
            {
                "entry_id": row["id"],
                "track": track_of(row["lang"], names),
                "question_type": row["question_type"] or "",
                "lang": row["lang"],
                "skill_ids": [t[6:] for t in names if t.startswith("skill:") and len(t) > 6],
                "family_id": next(
                    (t[7:] for t in names if t.startswith("family:") and len(t) > 7), None
                ),
                "tags": names,
                "due_at": row["due_at"],
                "retrievability": (
                    _SCHEDULER.get_card_retrievability(_card(dict(row), at), at)
                    if has_state
                    else None
                ),
                "stability": row["fsrs_stability"],
                "difficulty": row["fsrs_difficulty"],
                "review_count": row["review_count"],
                "last_review_at": row["last_review_at"],
                "fsrs_state": row["fsrs_state"],
                "estimated_seconds": None,
                "is_new": row["review_count"] == 0,
            }
        )
    return {"schema_version": SCHEMA_VERSION, "store_id": store_id, "now": now, "items": items}

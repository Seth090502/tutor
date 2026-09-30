"""Known-word coverage of a text from the learner's own FSRS state (contract C10).

A form is known when a production or recognition card for it, tagged
``lang:<code>``, has FSRS retrievability of at least 0.9 now. A card's form is
its target-language side: the answer of a production card, the prompt of a
recognition card. Coverage counts running words, not distinct ones.
"""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import sqlite3
import time

from fsrs import Card, Scheduler, State

from deeptutor.services.practice.scheduler import DESIRED_RETENTION, FSRS_PARAMETERS

from .text import normalize, tokenize

KNOWN_RETRIEVABILITY = 0.9
# Only a kind that tests the word itself can make it known.
FORM_KINDS = {"card:production": "correct_answer", "card:recognition": "question"}

_FSRS = Scheduler(parameters=FSRS_PARAMETERS, desired_retention=DESIRED_RETENTION)

_CARDS_SQL = """
    SELECT n.question, n.correct_answer, k.name AS kind, r.fsrs_stability, r.fsrs_difficulty,
        r.fsrs_state, r.fsrs_step, r.last_review_at
    FROM notebook_entries n
    JOIN practice_review_state r ON r.entry_id = n.id
    JOIN notebook_entry_categories ek ON ek.entry_id = n.id
    JOIN notebook_categories k ON k.id = ek.category_id
        AND k.name IN ('card:production', 'card:recognition')
    LEFT JOIN sessions s ON s.id = n.session_id
    WHERE (n.session_id IS NULL OR s.deleted_at IS NULL)
      AND r.fsrs_state IS NOT NULL AND r.last_review_at IS NOT NULL
      AND EXISTS (SELECT 1 FROM notebook_entry_categories el
          JOIN notebook_categories l ON l.id = el.category_id
          WHERE el.entry_id = n.id AND l.name = ?)
"""


def retrievability(row: sqlite3.Row | dict, now: float) -> float:
    """py-fsrs retrievability of a stored card at ``now``."""
    at = datetime.fromtimestamp(now, timezone.utc)
    card = Card(
        card_id=0,
        state=State(row["fsrs_state"]),
        step=row["fsrs_step"],
        stability=row["fsrs_stability"],
        difficulty=row["fsrs_difficulty"],
        due=at,
        last_review=datetime.fromtimestamp(row["last_review_at"], timezone.utc),
    )
    return _FSRS.get_card_retrievability(card, at)


def known_forms(db_path: Path, lang: str, now: float | None = None) -> set[str]:
    now = time.time() if now is None else now
    conn = sqlite3.connect(db_path, timeout=30)
    conn.row_factory = sqlite3.Row
    try:
        rows = conn.execute(_CARDS_SQL, (f"lang:{lang}",)).fetchall()
    finally:
        conn.close()
    return {
        form
        for row in rows
        if retrievability(row, now) >= KNOWN_RETRIEVABILITY
        and (form := normalize(lang, row[FORM_KINDS[row["kind"]]]))
    }


def coverage(lang: str, text: str, known: set[str]) -> dict:
    _, tokens = tokenize(lang, text)
    unknown: dict[str, None] = {}
    count = 0
    for token in tokens:
        if known.intersection(token.forms):
            count += 1
        else:
            unknown.setdefault(token.forms[0])
    return {
        "tokens": len(tokens),
        "known": count,
        "coverage": count / len(tokens) if tokens else 0.0,
        "unknown": list(unknown),
    }

"""Language cards: recognition, production and listening practice entries (contract C10).

Cards go through the practice import pipeline (stage, then commit), so FSRS,
attempts and analytics apply unchanged. A card is identified by its language,
kind and normalised target-language form, so adding the same word again (even
one that came from a starter deck) reuses the existing card and its schedule.
"""

from __future__ import annotations

import hashlib
from pathlib import Path
import re
import sqlite3

from deeptutor.services.practice.importing import MAX_ACCEPTED
from deeptutor.services.practice.storage import PracticeStore

from .text import normalize

KINDS = ("recognition", "production", "listening")
# Typed and auto-checked against the answer and its variants (not free response).
CARD_TYPE = "fill_blank"
# Where each kind keeps its target-language form.
FORM_FIELD = {"recognition": "question", "production": "correct_answer", "listening": "question"}

_EXISTING_SQL = """
    SELECT n.id, n.question, n.correct_answer, substr(k.name, 6) AS kind
    FROM notebook_entries n
    JOIN notebook_entry_categories ek ON ek.entry_id = n.id
    JOIN notebook_categories k ON k.id = ek.category_id
        AND k.name IN ('card:recognition', 'card:production', 'card:listening')
    LEFT JOIN sessions s ON s.id = n.session_id
    WHERE (n.session_id IS NULL OR s.deleted_at IS NULL)
      AND EXISTS (SELECT 1 FROM notebook_entry_categories el
          JOIN notebook_categories l ON l.id = el.category_id
          WHERE el.entry_id = n.id AND l.name = ?)
    ORDER BY n.id
"""


def meaning_variants(meaning: str) -> list[str]:
    """Each sense of "house; home" or "to eat" is accepted on its own ("home", "eat")."""
    variants = []
    for part in (p.strip() for p in re.split(r"[;,/]", meaning)):
        if part:
            variants.append(part)
            if part.lower().startswith("to ") and part[3:].strip():
                variants.append(part[3:].strip())
    return [v for v in dict.fromkeys(variants) if v != meaning.strip()][:MAX_ACCEPTED]


def card_question(lang: str, kind: str, text: str, meaning: str, example: str | None) -> dict:
    """One card in the practice import format."""
    if kind == "production":
        question, answer, accepted = meaning, text, []
    else:
        question, answer, accepted = text, meaning, meaning_variants(meaning)
    explanation = "\n".join(
        [
            f"{text}: {meaning}",
            *([f"Example: {example}"] if example else []),
            "Source: added by the learner while reading (unverified)",
            "Licence: the learner's own note",
        ]
    )
    key = hashlib.sha256(normalize(lang, text).encode()).hexdigest()
    return {
        "question_id": f"lang:{lang}:{kind}:{key}",
        "question": question,
        "question_type": CARD_TYPE,
        "options": {},
        "correct_answer": answer,
        "accepted_answers": accepted,
        "explanation": explanation,
        "misconception": None,
        "hints": [],
        "difficulty": "",
        "user_answer": "",
        "tags": [f"lang:{lang}", f"card:{kind}"],
    }


def _existing(conn: sqlite3.Connection, lang: str) -> dict[tuple[str, str], int]:
    found: dict[tuple[str, str], int] = {}
    for row in conn.execute(_EXISTING_SQL, (f"lang:{lang}",)):
        form = normalize(lang, row[FORM_FIELD[row["kind"]]])
        found.setdefault((row["kind"], form), row["id"])
    return found


def add_cards(db_path: Path, lang: str, items: list[dict]) -> dict:
    """Create the missing cards for each ``{"text", "meaning", "example"?}`` item."""
    store = PracticeStore(db_path)
    with store.connect() as conn:
        existing = _existing(conn, lang)
    keys: list[tuple[str, str]] = []
    planned: dict[tuple[str, str], dict] = {}
    for item in items:
        text, meaning = item["text"].strip(), item["meaning"].strip()
        form = normalize(lang, text)
        if not form or not meaning:
            raise ValueError("Each card needs a word or phrase and its meaning")
        for kind in KINDS:
            key = (kind, form)
            keys.append(key)
            if key not in existing and key not in planned:
                planned[key] = card_question(lang, kind, text, meaning, item.get("example"))
    created = 0
    if planned:
        token = store.stage_import(f"language-cards-{lang}", "bank", list(planned.values()))
        created = store.commit_import(token)["created"]
        with store.connect() as conn:
            for key, question in planned.items():
                existing[key] = conn.execute(
                    """SELECT id FROM notebook_entries WHERE origin_type='external_import'
                    AND origin_ref='practice-import' AND turn_id='' AND question_id=?""",
                    (question["question_id"],),
                ).fetchone()[0]
    return {"created": created, "entry_ids": list(dict.fromkeys(existing[key] for key in keys))}

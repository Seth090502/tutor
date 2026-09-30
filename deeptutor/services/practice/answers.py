"""Conservative objective grading; prose and qualitative work uses self-assessment."""

from __future__ import annotations

import json
import re

from deeptutor.services.languages.text import normalize, strip_accents

FREE_RESPONSE = frozenset({"qualitative", "short_answer", "essay", "free_response"})


def check_answer(entry: dict, answer: str) -> bool | None:
    return grade(entry, answer)[0]


def entry_language(entry: dict) -> str | None:
    """The ``lang:<code>`` tag's code, from a SQL column, the categories or import tags."""
    if entry.get("lang"):
        return str(entry["lang"])
    names = entry.get("tags") or [c.get("name", "") for c in entry.get("categories") or []]
    return next((name[5:] for name in names if name.startswith("lang:") and name[5:]), None)


def accepted_answers(entry: dict) -> list[str]:
    if "accepted_answers" in entry:
        return list(entry["accepted_answers"] or [])
    return json.loads(entry.get("accepted_answers_json") or "[]")


def grade(entry: dict, answer: str) -> tuple[bool | None, str | None]:
    """``(correct, diagnostic)``. The only diagnostic is "accent" (contract C10)."""
    kind = entry.get("question_type", "")
    options = entry.get("options") or json.loads(entry.get("options_json") or "{}")
    lang = entry_language(entry)
    if lang and not options:
        if kind in FREE_RESPONSE:
            return None, None  # rubric grading
        targets = [str(entry.get("correct_answer") or ""), *accepted_answers(entry)]
        return check_language_answer(lang, targets, answer)
    return _check_choice(entry, answer, kind, options), None


def check_language_answer(
    lang: str, targets: list[str], answer: str
) -> tuple[bool | None, str | None]:
    wanted = {form for form in (normalize(lang, target) for target in targets) if form}
    if not wanted:
        return None, None  # no key to grade against
    given = normalize(lang, answer)
    if given in wanted:
        return True, None
    if given and strip_accents(given) in {strip_accents(form) for form in wanted}:
        return False, "accent"
    return False, None


def _check_choice(entry: dict, answer: str, kind: str, options: dict) -> bool | None:
    expected = str(entry.get("correct_answer") or "").strip()
    if not options or not expected or kind in FREE_RESPONSE:
        return None

    def folded(text: str) -> str:
        return " ".join(text.strip().upper().split())

    def keys(value: str) -> set[str]:
        for key, text in options.items():
            if folded(value) in {
                folded(key),
                folded(text),
                folded(f"{key}. {text}"),
                folded(f"{key}) {text}"),
            }:
                return {key}
        parts = {part.strip().upper() for part in re.split(r"[,;，；\s]+", value) if part.strip()}
        if (
            kind in {"multi_choice", "multiple_select"}
            and len(parts) == 1
            and not parts <= options.keys()
        ):
            parts = set(value.strip().upper())
        return parts

    expected_keys, actual_keys = keys(expected), keys(answer)
    if not expected_keys or not expected_keys <= options.keys():
        return None  # A malformed legacy answer key must never invent a grade.
    return bool(actual_keys) and actual_keys == expected_keys

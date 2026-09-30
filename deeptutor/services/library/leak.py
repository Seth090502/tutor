"""The answer-leak guard: while a practice attempt is open (C7), library search must not
return what would give the answer away (CORPUS.md Stage 0 step 4, C11 ``withheld``).

A chunk is withheld when either

* its normalised text contains one of the attempt's normalised answer strings, or
* one of its headings is a solution or answer-key heading (English, and the five target
  languages of ``docs/tutor/LANGUAGES.md``: es, fr, de, ja, zh).

Answer strings: for a choice question whose key is option letters (``"B"``, ``"A,C"``),
the option texts are the answer; the letter alone says nothing about a passage.
Otherwise the key itself.

Short answers are not tested for containment: an answer under 3 characters (``"4"``,
``"12"``, ``"x"``) or a single letter occurs in almost every chunk of its subject, so
the test would withhold the whole shelf while telling the learner nothing. Those
answers stay protected by the heading rule, and by the tutor never seeing the key
(PEDAGOGY.md).

Containment is a plain substring test after normalisation, so it errs towards
withholding (``"cat"`` also matches ``"concatenate"``).
"""

from __future__ import annotations

from dataclasses import dataclass, field
import json
import re
import sqlite3
from typing import Any

from .normalize import normalize_loose

MIN_ANSWER_CHARS = 3

# Words that name exercise material, so "Solutions to the exercises" is an answer
# section while "Solutions of linear equations" (a maths topic) is not.
_EXERCISE_WORDS = {
    "en": r"exercises?|problems?|questions?|chapters?|sections?|review|practice|quiz(?:zes)?|"
    r"tests?|exams?|homework|assignments?|activities|activity|worksheets?",
    "es": r"ejercicios?|problemas?|preguntas?|cap[ií]tulos?|secci[oó]n|secciones|pr[aá]cticas?|"
    r"ex[aá]menes|examen|actividades|actividad|tareas?",
    "fr": r"exercices?|probl[eè]mes?|questions?|chapitres?|sections?|entra[iî]nements?|"
    r"examens?|devoirs?|activit[eé]s?",
    "de": r"aufgaben|aufgabe|[üu]bungen|[üu]bung|[üu]bungsaufgaben|kapitel|abschnitte?|fragen|"
    r"frage|pr[üu]fungen|pr[üu]fung|tests?|klausuren|klausur",
}
# Heading heads that open an answer section, per language.
_ANSWER_HEADS = {
    "en": r"(?:worked |selected |brief |full |hints and |hints & )?"
    r"(?:solutions?|answers?)(?: key| keys| manual| guide)?|answer keys?|key to",
    "es": r"(?:soluciones|soluci[oó]n|respuestas?|solucionario|clave de respuestas|"
    r"claves de respuestas)",
    "fr": r"(?:solutions?|r[eé]ponses?|corrig[eé]s?|corrections?|[eé]l[eé]ments de correction)",
    "de": r"(?:l[öo]sungen|l[öo]sung|antworten|antwort|musterl[öo]sungen|musterl[öo]sung|"
    r"l[öo]sungshinweise|l[öo]sungsschl[üu]ssel|l[öo]sungsvorschl[äa]ge|l[öo]sungsvorschlag)",
}
# Connectors between a head and what it answers ("Solutions to", "Lösungen der").
_CONNECTORS = {
    "en": r"to|for|of|on",
    "es": r"a|de|del|para|a los|a las|de los|de las",
    "fr": r"des|du|de|aux|au|de la|pour",
    "de": r"zu|zur|zum|der|des|f[üu]r|von",
}
# Headings that are an answer section on their own, whatever follows.
_ALWAYS_ANSWER = {
    "en": r"answer keys?|solutions? manual|key to",
    "es": r"solucionario|clave de respuestas|claves de respuestas",
    "fr": r"corrig[eé]s?|[eé]l[eé]ments de correction",
    "de": r"musterl[öo]sungen|musterl[öo]sung|l[öo]sungsschl[üu]ssel|l[öo]sungshinweise",
}
# Leading numbering such as "3.2", "A.", "Chapter 5:", "§ 4".
_NUMBERING = re.compile(
    r"^(?:(?:chapter|section|part|appendix|anhang|kapitel|cap[ií]tulo|chapitre|annexe|ap[eé]ndice)"
    r"\s+)?(?:§\s*)?(?:[0-9ivxlcdm]+|[a-z])?(?:[.\-:)][0-9a-z]*)*[.:)\-]?\s+"
)


def _latin_patterns() -> list[re.Pattern[str]]:
    patterns = []
    for lang, head in _ANSWER_HEADS.items():
        tail = rf"(?:\s*[:\-]\s*|\s+(?:{_CONNECTORS[lang]})\s+)"
        patterns.append(
            re.compile(
                rf"^(?:(?:{_ALWAYS_ANSWER[lang]})\b.*"
                rf"|(?:{head})(?:\s+[0-9][0-9.\-]*[a-z]?)?"
                rf"|(?:{head}){tail}(?:[0-9][0-9.\-]*|.*\b(?:{_EXERCISE_WORDS[lang]})\b.*))$",
                re.IGNORECASE,
            )
        )
    return patterns


_LATIN = _latin_patterns()
# Japanese and Chinese write no spaces, so these are contained anywhere in the heading.
# 解答题/解答題 is the Chinese "problems to solve" section (questions, not answers) and
# 解析 alone also means "analytic" (解析几何), so neither counts.
_CJK_ANSWER = re.compile(r"解答(?![题題])|答え|正解|略解|模範解答|答案|题解|題解")


def is_solution_heading(heading: str) -> bool:
    """Whether one heading names a solution or answer-key section."""
    text = normalize_loose(heading).strip(" .:;")
    if not text:
        return False
    if _CJK_ANSWER.search(text):
        return True
    text = _NUMBERING.sub("", text, count=1).strip(" .:;")
    return any(pattern.match(text) for pattern in _LATIN)


def answer_strings(correct_answer: str, options: dict[str, Any] | None) -> list[str]:
    """What an open attempt's answer looks like in a passage (see the module docstring)."""
    expected = str(correct_answer or "").strip()
    if not expected:
        return []
    options = options or {}
    keys = [part.strip() for part in re.split(r"[,;，；\s]+", expected) if part.strip()]
    by_key = {str(key).strip().upper(): str(text) for key, text in options.items()}
    if keys and all(key.upper() in by_key for key in keys):
        return [by_key[key.upper()] for key in keys]
    return [expected]


def _testable(answer: str) -> str | None:
    text = normalize_loose(answer)
    if len(text) < MIN_ANSWER_CHARS or (len(text) == 1 and text.isalpha()):
        return None
    return text


@dataclass
class LeakGuard:
    """Decides which chunks an open attempt withholds, and counts them."""

    answers: list[str] = field(default_factory=list)
    active: bool = False
    withheld_ids: set[str] = field(default_factory=set)

    @classmethod
    def for_answers(cls, answers: list[str]) -> LeakGuard:
        return cls(answers=[a for a in map(_testable, answers) if a], active=True)

    def leaks(self, chunk: dict[str, Any]) -> bool:
        if not self.active:
            return False
        headings = chunk.get("heading_path")
        if isinstance(headings, list) and any(is_solution_heading(str(h)) for h in headings):
            return True
        text = normalize_loose(str(chunk.get("text") or ""))
        return any(answer in text for answer in self.answers)

    def allow(self, chunk: dict[str, Any]) -> bool:
        """True to keep the chunk; a withheld chunk is counted once by id."""
        if not self.leaks(chunk):
            return True
        self.withheld_ids.add(str(chunk.get("chunk_id") or id(chunk)))
        return False

    @property
    def withheld(self) -> int:
        return len(self.withheld_ids)


class AttemptNotFound(LookupError):
    pass


def attempt_guard(db_path: Any, attempt_id: str | None) -> LeakGuard:
    """The guard for ``attempt_id``: active while the attempt is open, inactive otherwise.

    Raises :class:`AttemptNotFound` for an id the practice store does not know.
    """
    if not attempt_id:
        return LeakGuard()
    try:
        conn = sqlite3.connect(db_path, timeout=30)
    except sqlite3.Error as exc:
        raise AttemptNotFound(attempt_id) from exc
    try:
        row = conn.execute(
            """SELECT a.closed_at, n.correct_answer, n.options_json
            FROM practice_attempts a JOIN notebook_entries n ON n.id=a.entry_id
            WHERE a.attempt_id=?""",
            (attempt_id,),
        ).fetchone()
    except sqlite3.OperationalError as exc:  # no practice tables yet: nothing was opened
        raise AttemptNotFound(attempt_id) from exc
    finally:
        conn.close()
    if row is None:
        raise AttemptNotFound(attempt_id)
    closed_at, correct_answer, options_json = row
    if closed_at is not None:
        return LeakGuard()
    try:
        options = json.loads(options_json or "{}")
    except ValueError:
        options = {}
    return LeakGuard.for_answers(
        answer_strings(correct_answer, options if isinstance(options, dict) else {})
    )


__all__ = [
    "AttemptNotFound",
    "LeakGuard",
    "answer_strings",
    "attempt_guard",
    "is_solution_heading",
]

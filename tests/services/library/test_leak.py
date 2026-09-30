"""The answer-leak guard (CORPUS.md Stage 0 step 4): answers in text, solution headings, attempts."""

from __future__ import annotations

import asyncio
import uuid

import pytest

from deeptutor.services.library.leak import (
    AttemptNotFound,
    LeakGuard,
    answer_strings,
    attempt_guard,
    is_solution_heading,
)
from deeptutor.services.practice.loop import LearningLoop
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore


@pytest.mark.parametrize(
    "heading",
    [
        # English
        "Solutions",
        "Solution",
        "Answers",
        "Answer Key",
        "Answers to Selected Exercises",
        "Solutions to the Problems",
        "Hints and Solutions",
        "Worked solutions",
        "Solution 3.4",
        "Solutions Manual",
        "3.7 Answers to exercises",
        "Chapter 12: Solutions",
        "Appendix B Answers to Odd-Numbered Problems",
        "Answers: Chapter 3",
        # Spanish
        "Soluciones",
        "Soluciones a los ejercicios",
        "Respuestas a las preguntas",
        "Solucionario",
        "Clave de respuestas",
        # French
        "Corrigés",
        "Corrigé des exercices",
        "Solutions des exercices",
        "Réponses",
        "Éléments de correction",
        # German
        "Lösungen",
        "Lösungen der Aufgaben",
        "Musterlösung",
        "Lösungshinweise zu Kapitel 4",
        "Antworten zu den Übungen",
        # Japanese
        "解答",
        "練習問題の解答",
        "模範解答",
        "答えと解説",
        # Chinese
        "答案",
        "习题答案",
        "參考答案",
        "习题解答",
        "题解",
    ],
)
def test_solution_and_answer_key_headings(heading: str) -> None:
    assert is_solution_heading(heading)


@pytest.mark.parametrize(
    "heading",
    [
        "Solutions of Linear Equations",
        "Existence and uniqueness of solutions",
        "Buffer solutions",
        "Question answering systems",
        "3.2 Limits",
        "Lösungen von Differentialgleichungen",
        "Solución de ecuaciones cuadráticas",
        "Solutions tampons",
        "三、解答题",
        "解析几何",
        "方程式の解",
        "",
    ],
)
def test_topic_headings_are_not_answer_sections(heading: str) -> None:
    assert not is_solution_heading(heading)


def test_answer_strings_use_option_text_for_choice_keys() -> None:
    options = {"A": "ribosome", "B": "mitochondrion", "C": "nucleus"}

    assert answer_strings("B", options) == ["mitochondrion"]
    assert answer_strings("a, c", options) == ["ribosome", "nucleus"]
    assert answer_strings("the powerhouse", options) == ["the powerhouse"]
    assert answer_strings("x = 4", None) == ["x = 4"]
    assert answer_strings("", options) == []


def chunk(text: str, headings: list[str] | None = None, chunk_id: str = "d:0") -> dict:
    return {"chunk_id": chunk_id, "text": text, "heading_path": headings or []}


def test_guard_withholds_chunks_containing_the_normalised_answer() -> None:
    guard = LeakGuard.for_answers(["Mitochondrion"])

    assert guard.leaks(chunk("The MITOCHONDRION makes ATP."))
    assert guard.leaks(chunk("the mito­chondrion"))  # soft hyphen from line breaking
    assert not guard.leaks(chunk("The nucleus holds DNA."))


def test_guard_normalises_typography_in_both_answer_and_chunk() -> None:
    guard = LeakGuard.for_answers(["x² – 1"])

    assert guard.leaks(chunk("so f(x) = x2  -  1 here"))


@pytest.mark.parametrize("answer", ["4", "12", "x", "B", " b ", "π"])
def test_short_answers_skip_the_containment_test(answer: str) -> None:
    guard = LeakGuard.for_answers([answer])

    assert guard.active
    assert not guard.leaks(chunk(f"{answer} {answer} {answer} appears everywhere"))
    assert guard.leaks(chunk("anything", ["Answers"]))


def test_three_character_answers_are_tested() -> None:
    assert LeakGuard.for_answers(["6.0"]).leaks(chunk("pH 6.0 at equilibrium"))


def test_guard_withholds_chunks_under_solution_headings() -> None:
    guard = LeakGuard.for_answers(["something else entirely"])

    assert guard.leaks(chunk("Step 1 ...", ["Chapter 2", "Exercises", "Solutions"]))
    assert not guard.leaks(chunk("Step 1 ...", ["Chapter 2", "Exercises"]))


def test_inactive_guard_withholds_nothing() -> None:
    guard = LeakGuard()

    assert not guard.leaks(chunk("mitochondrion", ["Answer Key"]))
    assert guard.allow(chunk("mitochondrion", ["Answer Key"]))
    assert guard.withheld == 0


def test_allow_counts_each_withheld_chunk_once() -> None:
    guard = LeakGuard.for_answers(["mitochondrion"])

    for _ in range(3):
        assert not guard.allow(chunk("mitochondrion", chunk_id="d:1"))
    assert not guard.allow(chunk("x", ["Solutions"], chunk_id="d:2"))
    assert guard.allow(chunk("nucleus", chunk_id="d:3"))
    assert guard.withheld == 2


# ---------------------------------------------------------------------------
# Attempts (C7), against a real practice store
# ---------------------------------------------------------------------------


@pytest.fixture
def practice(tmp_path):
    store = SQLiteSessionStore(tmp_path / "practice.db")

    async def seed() -> int:
        await store.ensure_notebook_session("lesson", "Lesson")
        await store.upsert_notebook_entries(
            "lesson",
            [
                {
                    "question_id": "q1",
                    "question": "Which organelle makes most of the cell's ATP?",
                    "question_type": "single_choice",
                    "options": {"A": "ribosome", "B": "mitochondrion"},
                    "correct_answer": "B",
                    "explanation": "",
                    "result": "ungraded",
                }
            ],
        )
        return (await store.find_notebook_entry("lesson", "q1"))["id"]

    entry_id = asyncio.run(seed())
    return store.db_path, PracticeStore(store.db_path), entry_id


def test_open_attempt_guards_its_answer(practice) -> None:
    db_path, store, entry_id = practice
    attempt_id = store.open_attempt(entry_id)["attempt_id"]

    guard = attempt_guard(db_path, attempt_id)

    assert guard.active
    assert guard.leaks(chunk("Mitochondrion: the site of oxidative phosphorylation"))
    assert not guard.leaks(chunk("Ribosomes build proteins."))


def test_closed_attempt_guards_nothing(practice) -> None:
    db_path, store, entry_id = practice
    attempt_id = store.open_attempt(entry_id)["attempt_id"]
    store.check_attempt(attempt_id, "B")

    guard = attempt_guard(db_path, attempt_id)

    assert not guard.active
    assert not guard.leaks(chunk("mitochondrion", ["Answer key"]))


def test_no_attempt_guards_nothing(practice) -> None:
    db_path, _, _ = practice

    assert not attempt_guard(db_path, None).active


def test_unknown_attempt_is_an_error(practice) -> None:
    db_path, _, _ = practice

    with pytest.raises(AttemptNotFound):
        attempt_guard(db_path, "no-such-attempt")


# ---------------------------------------------------------------------------
# Study attempts (C12)
# ---------------------------------------------------------------------------


def _study(db_path, entry_id, *kinds):
    loop = LearningLoop(PracticeStore(db_path))
    state = {"session_id": None, "attempt_id": None, "version": None}

    def send(kind, payload=None, attempt=True):
        result = loop.execute(
            {
                "schema_version": 1,
                "command_id": str(uuid.uuid4()),
                "session_id": state["session_id"],
                "attempt_id": state["attempt_id"] if attempt else None,
                "expected_version": state["version"],
                "kind": kind,
                "payload": payload or {},
            },
            "ui",
        )
        state["version"] = result["aggregate_version"]
        state["session_id"] = result["public_state"]["session"]["session_id"]
        if result["public_state"]["attempt"]:
            state["attempt_id"] = result["public_state"]["attempt"]["attempt_id"]

    send("start_session", {"mode": "practice"}, attempt=False)
    send("open_item", {"entry_id": entry_id}, attempt=False)
    for kind, payload in kinds:
        send(kind, payload)
    return state["attempt_id"]


def test_study_attempt_is_guarded_until_its_answer_is_released(practice) -> None:
    db_path, _, entry_id = practice
    wrong = [
        ("commit_response", {"response": "A", "input_mode": "typed"}),
        ("record_confidence", {"level": 2}),
        ("check", {}),
    ]
    remediating = _study(db_path, entry_id, *wrong)
    assert attempt_guard(db_path, remediating).leaks(chunk("the mitochondrion"))

    revealed = _study(db_path, entry_id, ("give_up", {}))
    assert not attempt_guard(db_path, revealed).active


def test_abandoned_study_attempt_guards_nothing(practice) -> None:
    db_path, _, entry_id = practice
    abandoned = _study(db_path, entry_id, ("abandon", {}))
    assert not attempt_guard(db_path, abandoned).active

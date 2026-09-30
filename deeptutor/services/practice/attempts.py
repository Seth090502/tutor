"""The review rating an attempt earns, computed by the server (contract C7).

FSRS learns only from unassisted retrieval: hints lower the rating, and a
revealed answer or a worked example counts as a lapse. Confidence is not an
input on purpose; it feeds the calibration chart, never the schedule.
"""

from __future__ import annotations

from .scheduler import Rating

# Rungs 1-4 are the question's pre-written hints; rung 5 reveals the answer.
ANSWER_RUNG = 5


def attempt_rating(correct: bool, hint_rung_max: int, revealed: bool) -> Rating:
    if not correct or revealed or hint_rung_max >= 4:
        return "again"
    if hint_rung_max >= 2:
        return "hard"
    return "good"

"""The review rating an attempt earns, computed by the server (contracts C7 and C12).

Review policy 2: FSRS gets positive credit only for unassisted retrieval. Any hint, worked example or
revealed answer before the check makes the attempt a failed retrieval (Again). Hard is not produced:
Anki defines it as a correct recall made with effort, not a recall made with help (evidence ledger
EV-002, EV-014). Confidence is not an input on purpose; it feeds the calibration chart, never the
schedule.
"""

from __future__ import annotations

from .scheduler import Rating

POLICY_VERSION = "review-policy/2"

# Rungs 1-4 are the question's pre-written hints; rung 5 reveals the answer.
ANSWER_RUNG = 5


def attempt_rating(correct: bool, hint_rung_max: int, revealed: bool) -> Rating:
    if not correct or revealed or hint_rung_max > 0:
        return "again"
    return "good"

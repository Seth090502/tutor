"""Spaced repetition for question-bank practice, scheduled by FSRS (py-fsrs).

The model uses the library's default parameters at a fixed desired retention;
nothing is fitted per user. A card's FSRS memory state (stability, difficulty,
learning state and step) is persisted with the review state. ``interval_days``
is derived from ``due_at``, and the ``ease`` column left from the previous
SM-2-style policy is no longer read or written. New and relearning cards pass
through short learning steps (minutes) before day-scale intervals. Review
events are retained, so a card stored without FSRS state is rebuilt by
replaying them.
"""

from __future__ import annotations

from collections.abc import Iterable
from datetime import datetime, time, timedelta, timezone
from typing import Literal
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

from fsrs import Card, Scheduler, State
from fsrs import Rating as FSRSRating
from fsrs.scheduler import DEFAULT_PARAMETERS

DAY = 86400
Rating = Literal["again", "hard", "good", "easy"]

FSRS_PARAMETERS = DEFAULT_PARAMETERS
DESIRED_RETENTION = 0.9
FSRS_FIELDS = ("fsrs_stability", "fsrs_difficulty", "fsrs_state", "fsrs_step")

_SCHEDULER = Scheduler(parameters=FSRS_PARAMETERS, desired_retention=DESIRED_RETENTION)
_RATINGS = {
    "again": FSRSRating.Again,
    "hard": FSRSRating.Hard,
    "good": FSRSRating.Good,
    "easy": FSRSRating.Easy,
}


def day_bounds(timezone: str, now: float) -> tuple[float, float]:
    try:
        zone = ZoneInfo(timezone)
    except (ZoneInfoNotFoundError, ValueError) as exc:
        raise ValueError("Unknown timezone") from exc
    today = datetime.fromtimestamp(now, zone).date()
    # Construct both midnights: a local day is not necessarily 24 hours (DST).
    return (
        datetime.combine(today, time.min, zone).timestamp(),
        datetime.combine(today + timedelta(days=1), time.min, zone).timestamp(),
    )


def _card(state: dict, now: datetime) -> Card:
    """The stored FSRS card, or a new one when the state carries none."""
    # card_id is only used for review logs; passing it skips Card's 1 ms sleep.
    # review_card never reads ``due`` and always replaces it.
    if state.get("fsrs_state") is None:
        return Card(card_id=0, due=now)
    last = state.get("last_review_at")
    return Card(
        card_id=0,
        state=State(state["fsrs_state"]),
        step=state["fsrs_step"],
        stability=state["fsrs_stability"],
        difficulty=state["fsrs_difficulty"],
        due=now,
        last_review=None if last is None else datetime.fromtimestamp(last, timezone.utc),
    )


def schedule(state: dict, rating: Rating, now: float) -> dict:
    if rating not in _RATINGS:
        raise ValueError("Unknown review rating")
    reviewed_at = datetime.fromtimestamp(now, timezone.utc)
    card, _ = _SCHEDULER.review_card(_card(state, reviewed_at), _RATINGS[rating], reviewed_at)
    due_at = card.due.timestamp()
    return {
        "interval_days": (due_at - now) / DAY,
        "streak": int(state.get("streak", 0)) + 1 if rating in {"good", "easy"} else 0,
        "lapses": int(state.get("lapses", 0)) + (1 if rating == "again" else 0),
        "due_at": due_at,
        "last_review_at": now,
        "review_count": int(state.get("review_count", 0)) + 1,
        "version": int(state.get("version", 0)) + 1,
        "fsrs_stability": card.stability,
        "fsrs_difficulty": card.difficulty,
        "fsrs_state": int(card.state),
        "fsrs_step": card.step,
    }


def replay(events: Iterable[tuple[float, Rating]]) -> dict:
    """Rebuild FSRS memory state from ``(reviewed_at, rating)`` events in review order.

    With no events every field is None, so the next review starts a new card.
    """
    state: dict = {}
    for reviewed_at, rating in events:
        state = schedule(state, rating, reviewed_at)
    return {field: state.get(field) for field in FSRS_FIELDS}

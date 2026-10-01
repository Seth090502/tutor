"""SM-2 spaced repetition (simplified, stdlib only)."""

import time


def sm2(quality, reps=0, interval=0, ef=2.5):
    """quality 0..5. Returns (reps, interval_days, ef). 5=perfect, 0=blackout."""
    quality = max(0, min(5, int(quality)))
    ef = max(1.3, ef + (0.1 - (5 - quality) * (0.08 + (5 - quality) * 0.02)))
    if quality < 3:
        return 0, 1, ef
    reps += 1
    if reps == 1:
        interval = 1
    elif reps == 2:
        interval = 6
    else:
        interval = max(1, round(interval * ef))
    return reps, interval, ef


def now_ts():
    return int(time.time())


def due_ts(days_from_now):
    return now_ts() + int(days_from_now) * 86400

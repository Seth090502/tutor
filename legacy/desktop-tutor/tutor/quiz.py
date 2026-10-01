"""Quiz/test/pop-quiz engine: retrieval practice with shuffling + scoring."""

import random


def select(items, n=5, seed=None, difficulty=None):
    pool = list(items)
    if difficulty is not None:
        pool = [i for i in pool if i.get("difficulty") == difficulty]
    if not pool:
        return []
    rng = random.Random(seed)
    rng.shuffle(pool)
    return pool[: max(1, min(n, len(pool)))]


def check(item, given):
    """given: 'A'/'b'/full text. Returns bool."""
    if given is None:
        return False
    g = str(given).strip().upper()
    if g.startswith(item["answer"].upper()):
        return True
    # allow 'A) ...' full match
    return g == item["answer"].upper()


def grade_letter(correct, total):
    if total <= 0:
        return "N/A"
    pct = 100.0 * correct / total
    if pct >= 90:
        return "A"
    if pct >= 80:
        return "B"
    if pct >= 70:
        return "C"
    if pct >= 60:
        return "D"
    return "F"

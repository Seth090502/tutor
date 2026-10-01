"""JSON progress store (local only, gitignored)."""

import json
import os

DEFAULT_PATH = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data", "progress.json")


def _blank():
    return {"attempts": [], "items": {}, "mastered_topics": {}, "facts": []}


def load(path=DEFAULT_PATH):
    if not os.path.exists(path):
        return _blank()
    try:
        with open(path, "r", encoding="utf-8") as f:
            d = json.load(f)
    except Exception:
        return _blank()
    for k in ("attempts", "items", "mastered_topics"):
        d.setdefault(k, {} if k != "attempts" else [])
    return d


def save(state, path=DEFAULT_PATH):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=2)
    os.replace(tmp, path)
    return path


def record_attempt(state, item_id, domain, correct, quality=None):
    from .spaced import sm2, due_ts

    item = state["items"].get(item_id, {"reps": 0, "interval": 0, "ef": 2.5, "due": 0, "seen": 0, "correct": 0})
    item["seen"] = item.get("seen", 0) + 1
    item["correct"] = item.get("correct", 0) + (1 if correct else 0)
    q = 5 if correct else 2
    if quality is not None:
        q = quality
    reps, interval, ef = sm2(q, item.get("reps", 0), item.get("interval", 0), item.get("ef", 2.5))
    item.update({"reps": reps, "interval": interval, "ef": round(ef, 3), "due": due_ts(interval)})
    state["items"][item_id] = item
    state["attempts"].append({"item": item_id, "domain": domain, "correct": bool(correct)})
    return item


def accuracy_by_domain(state):
    out = {}
    for a in state.get("attempts", []):
        d = out.setdefault(a["domain"], {"n": 0, "c": 0})
        d["n"] += 1
        d["c"] += 1 if a["correct"] else 0
    return {k: {"n": v["n"], "correct": v["c"], "acc": (v["c"] / v["n"] if v["n"] else 0)} for k, v in out.items()}

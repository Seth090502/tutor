"""Local memory layer (mem0 pattern, stdlib only, no LLM).

mem0 distillate: ADD-only facts (never overwrite), entity tags (domain/topic),
multi-signal recall (keyword + recency + tag overlap), temporal ranking (ts).
Lives inside the existing store.json under state["facts"] (backward compatible).
"""

import re
import time


def _facts(state):
    return state.setdefault("facts", [])


def add_fact(state, text, tags=None):
    """Append-only. Returns the fact record."""
    fact = {"text": str(text)[:500], "tags": list(tags or []),
            "ts": int(time.time())}
    _facts(state).append(fact)
    return fact


def recall(state, query, limit=5):
    """Score = keyword overlap + tag overlap + recency. Returns top facts."""
    terms = set(t for t in re.findall(r"[a-z0-9]+", (query or "").lower()) if len(t) > 1)
    if not terms:
        return []
    now = int(time.time())
    scored = []
    for f in _facts(state):
        low = f["text"].lower()
        kw = sum(low.count(t) for t in terms)
        tag = len(terms & set(t.lower() for t in f.get("tags", [])))
        age_days = max(0, (now - f.get("ts", now)) / 86400.0)
        recency = 1.0 / (1.0 + age_days / 30.0)
        score = kw + 2.0 * tag + 0.5 * recency
        if score > 0.3:
            scored.append((score, f))
    scored.sort(key=lambda kv: -kv[0])
    return [f for _, f in scored[:limit]]


def summarize(state):
    """Compact context for the agent loop: accuracy + weak domains + recent facts."""
    from .store import accuracy_by_domain

    acc = accuracy_by_domain(state)
    facts = _facts(state)[-5:]
    weak = min(acc.items(), key=lambda kv: kv[1]["acc"])[0] if acc else None
    return {"accuracy": acc, "weakest": weak,
            "recent_facts": [f["text"] for f in facts],
            "tracked": len(state.get("items", {}))}

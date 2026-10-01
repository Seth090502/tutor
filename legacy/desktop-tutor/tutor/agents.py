"""Agent teaching loop (langgraph/crewai pattern, stdlib only).

State machine: explain -> quiz -> grade -> review -> next.
Durable: every grade persists via store.record_attempt (same as CLI).
Human-in-loop: core takes answers as input (no input() here); CLI passes them.
Misses are logged as memory facts (mem0 pattern) for later recall.
answers=None auto-marks correct (pipeline demo only, not mastery).
"""

import time

from . import curriculum as CU
from . import question_bank as QB
from . import quiz as QZ
from . import store as ST
from . import memory as ME


NODES = ("explain", "quiz", "grade", "review", "next")


def resolve_topic(domain, state, topic=None):
    topics = CU.topics_for(domain)
    if not topics:
        return None
    if topic in topics:
        return topic
    mastered = state.get("mastered_topics", {}).get(domain, [])
    return CU.next_topic(domain, mastered) or next(iter(topics))


def preview_items(domain, state, topic=None, n=3, seed=7):
    """Item list for a lesson without grading (lets CLI ask first)."""
    topic = resolve_topic(domain, state, topic)
    if topic is None:
        return None, []
    pool = [i for i in QB.by_domain(domain) if i["topic"] == topic] or QB.by_domain(domain)
    return topic, QZ.select(pool, n=n, seed=seed)


def run_lesson(domain, state, topic=None, n=3, seed=7, answers=None):
    """Run one lesson. Returns {"trace": [...], "correct": int, "total": int}."""
    trace = []
    topic = resolve_topic(domain, state, topic)
    if topic is None:
        return {"trace": [{"node": "error", "text": f"unknown domain {domain!r}"}],
                "correct": 0, "total": 0}
    expl = CU.EXPLAINERS.get(topic, "")
    trace.append({"node": "explain", "topic": topic, "text": expl[:800]})
    _, items = preview_items(domain, state, topic=topic, n=n, seed=seed)
    trace.append({"node": "quiz", "items": [i["id"] for i in items]})
    correct = 0
    for k, it in enumerate(items):
        given = it["answer"] if answers is None else (answers[k] if k < len(answers) else "")
        ok = QZ.check(it, given)
        correct += 1 if ok else 0
        ST.record_attempt(state, it["id"], domain, ok)
        if not ok:
            ME.add_fact(state, f"missed {it['id']}: {it['prompt'][:120]}", tags=[domain, it["topic"]])
        trace.append({"node": "grade", "item": it["id"], "correct": ok,
                      "why": it["explanation"], "watch": it["misconception"]})
    due = sum(1 for m in state.get("items", {}).values() if m.get("due", 0) <= time.time())
    trace.append({"node": "review", "due": due})
    mastered = state.get("mastered_topics", {}).get(domain, [])
    trace.append({"node": "next", "next_topic": CU.next_topic(domain, mastered)})
    return {"trace": trace, "correct": correct, "total": len(items)}

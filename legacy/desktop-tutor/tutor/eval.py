"""Search eval (llm-zoomcamp pattern + hillclimb discipline): ground truth + hit@k.

Each bank item's prompt must retrieve its topic note in top-k.
split_items() gives a deterministic train/test split so hillclimbing can read
train failures while test stays held out. Never hillclimb against the test set.
"""

from . import question_bank as QB

TOPIC_DOC = {
    "arithmetic-laws": "corpus/domains/math/foundations.md",
    "fractions": "corpus/domains/math/foundations.md",
    "linear-equations": "corpus/domains/math/foundations.md",
    "quadratics": "corpus/domains/math/foundations.md",
    "functions": "corpus/domains/math/foundations.md",
    "exponents-logs": "corpus/domains/math/foundations.md",
    "geometry-basics": "corpus/domains/math/foundations.md",
    "grammar-core": "corpus/domains/english/craft.md",
    "punctuation": "corpus/domains/english/craft.md",
    "thesis": "corpus/domains/english/craft.md",
    "evidence": "corpus/domains/english/craft.md",
    "rhetoric": "corpus/domains/english/craft.md",
    "scientific-method": "corpus/domains/science/mechanisms.md",
    "cells": "corpus/domains/science/mechanisms.md",
    "photosynthesis": "corpus/domains/science/mechanisms.md",
    "genetics": "corpus/domains/science/mechanisms.md",
    "newton": "corpus/domains/science/mechanisms.md",
    "atoms": "corpus/domains/chemistry/quant.md",
    "bonding": "corpus/domains/chemistry/quant.md",
    "reactions": "corpus/domains/chemistry/quant.md",
    "stoichiometry": "corpus/domains/chemistry/quant.md",
    "acids-bases": "corpus/domains/chemistry/quant.md",
}


def split_items(seed=7, frac=0.7):
    """Deterministic train/test split of bank ids. Returns (train, test)."""
    import random

    ids = sorted(i["id"] for i in QB.ITEMS if i["topic"] in TOPIC_DOC)
    rng = random.Random(seed)
    rng.shuffle(ids)
    cut = int(len(ids) * frac)
    return ids[:cut], ids[cut:]


def evaluate(k=3, ids=None):
    """Hit@k over ids (default: whole bank). Returns dict with misses."""
    from . import corpus as CO

    want = set(ids) if ids is not None else None
    items = [
        it for it in QB.ITEMS if (want is None or it["id"] in want) and it["topic"] in TOPIC_DOC
    ]
    n = hit = 0
    misses = []
    for it, hits in zip(
        items, CO.search_many([it["prompt"] for it in items], limit=k), strict=True
    ):
        expected = TOPIC_DOC[it["topic"]]
        n += 1
        paths = [h["path"].replace("\\", "/") for h in hits]
        if expected in paths:
            hit += 1
        else:
            misses.append({"id": it["id"], "expected": expected, "got": paths})
    return {"n": n, "hit": hit, "k": k, "hit_rate": (hit / n if n else 0), "misses": misses}

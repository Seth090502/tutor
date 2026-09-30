"""Deterministic, seeded train/test split by source document, never by chunk.

Questions about the same document must land in the same split, otherwise near-duplicate
passages leak from train into test. Documents are identified by doc_id and by path; one
gold entry ties its doc_id and path together, and a question ties all of its gold entries
together. Each connected group of documents is hashed (sha256 of seed + the group's
smallest document key) to a number in [0, 1); below the train fraction means train.
A document's split therefore depends only on the seed and on the documents it shares
questions with, not on the rest of the dataset, so adding questions keeps old test
documents in test.
"""

from __future__ import annotations

from collections import defaultdict
import hashlib

from .dataset import norm_path

DEFAULT_SEED = 0
DEFAULT_TRAIN_FRAC = 0.7
RATIO_WARN = 0.10


class _Groups:
    """Union-find whose root is always the smallest key in the component."""

    def __init__(self) -> None:
        self.parent: dict[str, str] = {}

    def find(self, key: str) -> str:
        self.parent.setdefault(key, key)
        while self.parent[key] != key:
            self.parent[key] = self.parent[self.parent[key]]
            key = self.parent[key]
        return key

    def union(self, a: str, b: str) -> None:
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.parent[max(ra, rb)] = min(ra, rb)


def _gold_nodes(entry: dict) -> list[str]:
    nodes = ["path:" + norm_path(entry["path"])]
    if entry.get("doc_id"):
        nodes.append("doc:" + entry["doc_id"])
    return nodes


def document_groups(rows: list[dict]) -> dict[str, str]:
    """qid -> key of its document group (the smallest document key in the group)."""
    groups = _Groups()
    first_node: dict[str, str] = {}
    for row in rows:
        nodes = [n for entry in row["gold"] for n in _gold_nodes(entry)]
        first_node[row["qid"]] = nodes[0]
        for node in nodes[1:]:
            groups.union(nodes[0], node)
    return {qid: groups.find(node) for qid, node in first_node.items()}


def bucket(group_key: str, seed: int) -> float:
    digest = hashlib.sha256(f"{seed}\x00{group_key}".encode()).digest()
    return int.from_bytes(digest[:8], "big") / 2**64


def apply_split(
    rows: list[dict], *, seed: int = DEFAULT_SEED, train_frac: float = DEFAULT_TRAIN_FRAC
) -> dict:
    """Set row["split"] on every row in place; return a summary."""
    if not 0 < train_frac < 1:
        raise ValueError("train_frac must be between 0 and 1 (exclusive)")
    group_of = document_groups(rows)
    changed = 0
    by_split: dict[str, int] = defaultdict(int)
    docs: dict[str, set[str]] = defaultdict(set)
    by_domain: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for row in rows:
        key = group_of[row["qid"]]
        split = "train" if bucket(key, seed) < train_frac else "test"
        if row.get("split") not in (None, split):
            changed += 1
        row["split"] = split
        by_split[split] += 1
        docs[split].add(key)
        by_domain[row["domain"]][split] += 1
    actual = by_split["train"] / len(rows) if rows else 0.0
    warnings = []
    if rows and abs(actual - train_frac) > RATIO_WARN:
        warnings.append(
            f"train fraction is {actual:.2f}, target {train_frac:.2f}: too few document groups "
            f"({len(set(group_of.values()))}) for the hash split to balance"
        )
    if changed:
        warnings.append(
            f"{changed} questions changed split (different seed or new cross-document gold)"
        )
    return {
        "n": len(rows),
        "train_frac_actual": actual,
        "by_split": {s: by_split[s] for s in ("train", "test")},
        "document_groups": {s: len(docs[s]) for s in ("train", "test")},
        "by_domain": {
            d: {s: c[s] for s in ("train", "test")} for d, c in sorted(by_domain.items())
        },
        "changed": changed,
        "warnings": warnings,
    }

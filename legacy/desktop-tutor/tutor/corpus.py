"""Corpus search: BM25 ranking by the Rust `tutor-search` binary (search-rs/).

Searches every .md/.txt under corpus/ plus user-paper metadata
(corpus/papers_index.json). Build it first: `cargo build --release`.
"""

import json
import os
import re
import subprocess

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _search_bin():
    """tutor-search in the cargo target-dir set by .cargo/config.toml."""
    target = os.path.join(BASE, "target")
    cfg = os.path.join(BASE, ".cargo", "config.toml")
    if os.path.exists(cfg):
        with open(cfg, "r", encoding="utf-8") as fh:
            m = re.search(r'^target-dir\s*=\s*"([^"]+)"', fh.read(), re.M)
        if m:
            target = os.path.join(BASE, m.group(1))
    return os.path.join(target, "release", "tutor-search" + (".exe" if os.name == "nt" else ""))


def search_many(queries, limit=5):
    """One tutor-search process for all queries. Returns one hit list per query."""
    exe = _search_bin()
    if not os.path.exists(exe):
        raise RuntimeError(f"tutor-search not built ({exe}); run `cargo build --release` in {BASE}")
    lines = "".join(" ".join((q or "").split()) + "\n" for q in queries)
    out = subprocess.run(
        [exe, "--root", BASE, "--limit", str(limit)],
        input=lines,
        stdout=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        check=True,
    )
    return [json.loads(line) for line in out.stdout.splitlines()]


def search(query, limit=5):
    return search_many([query], limit)[0]

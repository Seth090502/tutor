"""Synthetic Markdown corpus for ingest throughput checks (see README, "Scale and memory").

200 books of seeded random vocabulary (about 34 MB, 8 chapters of 4 sections each), spread
over the four domains' books/ and papers/ folders. Usage:

    python crates/rag-engine/bench/synthetic_corpus.py <out-dir>
"""

import os
import random
import sys

root = sys.argv[1]
rng = random.Random(42)
vocab = [
    "limit", "function", "derivative", "integral", "series", "vector", "matrix", "energy", "reaction",
    "electron", "molecule", "acid", "base", "cell", "protein", "evolution", "narrative", "metaphor",
    "clause", "grammar", "theorem", "proof", "continuity", "gradient", "entropy", "enthalpy", "orbital",
    "equilibrium", "velocity", "momentum", "force", "field", "wave", "photon", "genome", "enzyme",
] + [f"term{i}" for i in range(3000)]  # fmt: skip
domains = ["math", "science", "chemistry", "english"]
total = 0
for i in range(200):
    folder = os.path.join(root, domains[i % 4], "books" if i % 3 else "papers")
    os.makedirs(folder, exist_ok=True)
    parts = [f"# Book {i}\n"]
    for chapter in range(1, 9):
        parts.append(f"\n## {chapter} Chapter {chapter}\n")
        for section in range(1, 5):
            parts.append(f"\n### {chapter}.{section} Section\n")
            for _ in range(6):
                words = [rng.choice(vocab) for _ in range(rng.randint(60, 140))]
                sentences = []
                while words:
                    n = rng.randint(8, 20)
                    sentences.append(" ".join(words[:n]).capitalize() + ".")
                    words = words[n:]
                parts.append("\n" + " ".join(sentences) + "\n")
    text = "".join(parts)
    with open(os.path.join(folder, f"book{i:03d}.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    total += len(text.encode("utf-8"))
print(f"{total / 1e6:.1f} MB in 200 files under {root}")

"""Build corpus/index.json from manifest + local markdown (never deletes sources)."""

import json
import os
import re

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(BASE, "corpus")
MANIFEST = os.path.join(CORPUS, "manifest.json")
INDEX = os.path.join(CORPUS, "index.json")


def tokens(text):
    return re.findall(r"[a-z0-9]+", text.lower())


def main():
    docs = []
    subdirs = ["domains", "papers", "books"]
    for sub in subdirs:
        base = os.path.join(CORPUS, sub)
        for root, _, files in os.walk(base):
            for f in files:
                if f.endswith((".md", ".txt")) and not f.startswith("."):
                    fp = os.path.join(root, f)
                    rel = os.path.relpath(fp, BASE).replace(os.sep, "/")
                    with open(fp, "r", encoding="utf-8") as fh:
                        text = fh.read()
                    docs.append(
                        {
                            "id": rel,
                            "local_path": rel,
                            "chars": len(text),
                            "tokens": len(tokens(text)),
                        }
                    )
    ingest = os.path.join(CORPUS, "ingest")
    if os.path.isdir(ingest):
        for f in sorted(os.listdir(ingest)):
            if f.startswith("."):
                continue
            fp = os.path.join(ingest, f)
            if os.path.isfile(fp) and f.endswith((".md", ".txt")):
                rel = os.path.relpath(fp, BASE)
                with open(fp, "r", encoding="utf-8") as fh:
                    text = fh.read()
                docs.append(
                    {"id": rel, "local_path": rel, "chars": len(text), "tokens": len(tokens(text))}
                )
    manifest_n = 0
    if os.path.exists(MANIFEST):
        with open(MANIFEST, "r", encoding="utf-8") as fh:
            manifest_n = len(json.load(fh).get("sources", []))
    out = {
        "version": "1.0.0",
        "documents": docs,
        "manifest_sources": manifest_n,
        "note": "Full text only for own-notes/open licenses. Manifest holds links for the rest.",
    }
    with open(INDEX, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=2)
    print(f"Indexed {len(docs)} local docs + {manifest_n} manifest sources -> corpus/index.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

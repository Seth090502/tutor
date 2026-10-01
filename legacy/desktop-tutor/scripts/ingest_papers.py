"""Ingest user-provided papers (metadata-first, license-aware).

You bring the PDFs; this script never downloads, never copies full text into
git, never strips DRM. It records filename/size/title-hint into
corpus/papers_index.json so search + agents can cite them. PDF text is never
extracted; .md/.txt inside corpus/ are indexed by build_index separately.

Usage:
  python scripts/ingest_papers.py --src "D:\\Papers" --limit 5000
  python scripts/ingest_papers.py --src "D:\\Papers" --merge-manifest
"""

import argparse
import hashlib
import json
import os
import re

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(BASE, "corpus", "papers_index.json")
MANIFEST = os.path.join(BASE, "corpus", "manifest.json")
EXTS = (".pdf", ".md", ".txt")


def title_hint(name):
    s = re.sub(r"[_\-]+", " ", os.path.splitext(name)[0]).strip()
    s = re.sub(r"\s+", " ", s)
    return s[:160]


def scan(src, limit=5000):
    out = []
    n = 0
    for root, _, files in os.walk(src):
        for f in sorted(files):
            if not f.lower().endswith(EXTS):
                continue
            fp = os.path.join(root, f)
            try:
                size = os.path.getsize(fp)
            except OSError:
                continue
            pid = (
                "paper-"
                + hashlib.sha1(os.path.relpath(fp, src).encode("utf-8", "ignore")).hexdigest()[:12]
            )
            out.append(
                {
                    "id": pid,
                    "filename": f,
                    "path": fp if os.path.isabs(fp) else os.path.relpath(fp, BASE),
                    "size": size,
                    "title_hint": title_hint(f),
                    "license": "user-provided; verify rights before redistributing",
                }
            )
            n += 1
            if n >= limit:
                return out
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description="Index user papers (metadata only)")
    p.add_argument("--src", required=True, help="Folder holding your PDFs/md/txt")
    p.add_argument("--limit", type=int, default=5000)
    p.add_argument(
        "--merge-manifest",
        action="store_true",
        help="Also append entries to corpus/manifest.json (dedupe by id)",
    )
    args = p.parse_args(argv)
    if not os.path.isdir(args.src):
        print(f"No such folder: {args.src}")
        return 1
    docs = scan(args.src, args.limit)
    with open(OUT, "w", encoding="utf-8") as fh:
        json.dump({"version": "1.1.0", "count": len(docs), "documents": docs}, fh, indent=2)
    print(
        f"Indexed {len(docs)} papers -> corpus/papers_index.json (metadata only, files untouched)"
    )
    if args.merge_manifest and os.path.exists(MANIFEST):
        with open(MANIFEST, "r", encoding="utf-8") as fh:
            m = json.load(fh)
        have = {s.get("id") for s in m.get("sources", [])}
        added = 0
        for d in docs:
            if d["id"] in have:
                continue
            m["sources"].append(
                {
                    "id": d["id"],
                    "title": d["title_hint"],
                    "license": d["license"],
                    "url": None,
                    "local_path": d["path"],
                    "domains": [],
                }
            )
            added += 1
        with open(MANIFEST, "w", encoding="utf-8") as fh:
            json.dump(m, fh, indent=2)
        print(f"Merged {added} entries into manifest.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

"""Safe shallow cloner for catalog repos. Opt-in only."""

import argparse
import json
import os
import subprocess
import sys

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CATALOG = os.path.join(BASE, "repos", "catalog.json")
DEST = os.path.join(BASE, "repos", "clones")


def load():
    with open(CATALOG, "r", encoding="utf-8") as f:
        return json.load(f)["repos"]


def run(cmd):
    print("+ " + " ".join(cmd))
    r = subprocess.run(cmd, cwd=BASE)
    return r.returncode


def main(argv=None):
    p = argparse.ArgumentParser(description="Shallow-clone catalog repos (opt-in)")
    p.add_argument("--list", action="store_true")
    p.add_argument("--clone", nargs="*", default=[])
    p.add_argument("--clone-all", action="store_true")
    p.add_argument("--depth", type=int, default=1)
    args = p.parse_args(argv)
    repos = {r["name"]: r for r in load()}
    if args.list or (not args.clone and not args.clone_all):
        for name, r in repos.items():
            print(f"{name}: {r['url']} — {r['why']}")
        return 0
    targets = (
        list(repos.values()) if args.clone_all else [repos[n] for n in args.clone if n in repos]
    )
    missing = [n for n in args.clone if n not in repos]
    for m in missing:
        print(f"Unknown repo '{m}'. Use --list.", file=sys.stderr)
    os.makedirs(DEST, exist_ok=True)
    for r in targets:
        dest = os.path.join(DEST, r["name"])
        if os.path.exists(dest):
            print(f"Exists, skipping: {dest}")
            continue
        code = run(["git", "clone", "--depth", str(args.depth), r["url"], dest])
        if code != 0:
            print(f"Clone failed for {r['name']}", file=sys.stderr)
    print(f"Done. Clones in {DEST} (gitignored).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

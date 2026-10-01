"""Print keyword-search hit@k (zoomcamp-style eval with train/test split)."""
import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from tutor.eval import evaluate, split_items


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--k", type=int, default=3)
    p.add_argument("--split", choices=("all", "train", "test"), default="all")
    p.add_argument("--seed", type=int, default=7)
    args = p.parse_args()
    ids = None
    if args.split != "all":
        train, test = split_items(seed=args.seed)
        ids = train if args.split == "train" else test
    r = evaluate(k=args.k, ids=ids)
    print(f"hit@{r['k']} [{args.split}]: {r['hit']}/{r['n']} = {100*r['hit_rate']:.1f}%")
    for m in r["misses"]:
        print(f"  MISS {m['id']}: want {m['expected']}, got {m['got']}")


if __name__ == "__main__":
    main()

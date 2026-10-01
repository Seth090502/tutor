#!/usr/bin/env python
"""Convert the downloaded BRIGHT parquet files (psychology, biology, economics) to JSONL and
compute Qwen3 token statistics. Run with:

  uv run --with pyarrow --with tokenizers python prep_bright.py

Inputs : <bright>/raw/{documents,examples}/<split>.parquet  (fetch_assets.py)
         <models>/tokenizer/Qwen__Qwen3-Embedding-0.6B__tokenizer.json
Outputs: <bright>/<split>/documents.jsonl   {id, content}
         <bright>/<split>/examples.jsonl    {id, query, gold_ids, excluded_ids}
         <bright>/token_stats.json
BRIGHT licence: CC-BY-4.0 (dataset card, xlangai/BRIGHT). Attribution is kept in <bright>/LICENSE.md.
"""

import argparse
import json
import os
import statistics

import pyarrow.parquet as pq
from tokenizers import Tokenizer

SPLITS = ["psychology", "biology", "economics"]


def pct(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p / 100 * len(xs)))]


def summarize(lens):
    return {
        "n": len(lens),
        "total": sum(lens),
        "mean": round(statistics.fmean(lens), 1),
        "p50": pct(lens, 50),
        "p90": pct(lens, 90),
        "p95": pct(lens, 95),
        "p99": pct(lens, 99),
        "max": max(lens),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bright", default=r"C:\Users\sethy\Projects\vault-eval\external\bright")
    ap.add_argument(
        "--tokenizer",
        default=r"C:\Users\sethy\Projects\vault-models\tokenizer\Qwen__Qwen3-Embedding-0.6B__tokenizer.json",
    )
    a = ap.parse_args()
    tok = Tokenizer.from_file(a.tokenizer)
    tok.no_truncation()
    stats = {"tokenizer": os.path.basename(a.tokenizer), "splits": {}}
    for s in SPLITS:
        out = os.path.join(a.bright, s)
        os.makedirs(out, exist_ok=True)
        docs = pq.read_table(os.path.join(a.bright, "raw", "documents", f"{s}.parquet")).to_pylist()
        exs = pq.read_table(os.path.join(a.bright, "raw", "examples", f"{s}.parquet")).to_pylist()
        with open(os.path.join(out, "documents.jsonl"), "w", encoding="utf-8", newline="\n") as f:
            for d in docs:
                f.write(
                    json.dumps({"id": d["id"], "content": d["content"]}, ensure_ascii=False) + "\n"
                )
        with open(os.path.join(out, "examples.jsonl"), "w", encoding="utf-8", newline="\n") as f:
            for e in exs:
                f.write(
                    json.dumps(
                        {
                            "id": e["id"],
                            "query": e["query"],
                            "gold_ids": list(e["gold_ids"]),
                            "excluded_ids": list(e["excluded_ids"]),
                        },
                        ensure_ascii=False,
                    )
                    + "\n"
                )
        dlens = [len(x.ids) for x in tok.encode_batch([d["content"] for d in docs])]
        qlens = [len(x.ids) for x in tok.encode_batch([e["query"] for e in exs])]
        gold = {g for e in exs for g in e["gold_ids"]}
        ids = {d["id"] for d in docs}
        glens = [n for d, n in zip(docs, dlens) if d["id"] in gold]
        stats["splits"][s] = {
            "documents": summarize(dlens),
            "queries": summarize(qlens),
            "gold_docs": summarize(glens),
            "gold_ids_total": sum(len(e["gold_ids"]) for e in exs),
            "gold_unique": len(gold),
            "gold_missing_from_documents": len(gold - ids),
            "queries_with_excluded_ids": sum(
                1 for e in exs if e["excluded_ids"] and e["excluded_ids"] != ["N/A"]
            ),
        }
        print(s, json.dumps(stats["splits"][s]))
    with open(os.path.join(a.bright, "token_stats.json"), "w", encoding="utf-8") as f:
        json.dump(stats, f, indent=2)


if __name__ == "__main__":
    main()

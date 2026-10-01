#!/usr/bin/env python
"""Build the seeded BRIGHT subset used in the GPU window, plus the CPU smoke subset, and the time budget.

  uv run --with pyarrow --with tokenizers python make_subset.py

Subset rule (documented in docs/phase0/model-bench-prep.md):
  per split: ALL gold documents of ALL queries (union of gold_ids) + a seeded random sample of N negatives
  (non-gold documents, ids sorted then random.Random(f"{seed}:{split}").sample), documents truncated to
  --max-doc-tokens Qwen3 tokens, queries truncated to --max-query-tokens, and a seeded sample of
  --rerank-queries queries per split for the reranker stage.
This is far easier than full BRIGHT (about 3-4k candidates per split instead of 50-57k): relative comparison only.
"""

import argparse
import json
import os
import random

from tokenizers import Tokenizer

SPLITS = ["psychology", "biology", "economics"]
# Conservative effective throughput: 25 TFLOPS achieved on the 5090 through llama.cpp, 2 FLOPs per
# non-embedding parameter per token. (Model: non-embedding parameters in billions.)
EFFECTIVE_TFLOPS = 25.0
MODELS = {  # name: (non-embedding params in B, kind)
    "qwen3-embedding-0.6b-q8_0": (0.44, "embed"),
    "qwen3-embedding-0.6b-f16": (0.44, "embed"),
    "bge-m3-q8_0": (0.30, "embed"),
    "qwen3-embedding-4b-q8_0": (3.6, "embed"),
    "qwen3-embedding-8b-q8_0": (6.95, "embed"),
    "qwen3-reranker-0.6b-q8_0": (0.44, "rerank"),
    "qwen3-reranker-4b-q8_0": (3.6, "rerank"),
    "qwen3-reranker-8b-q8_0": (6.95, "rerank"),
}


def read_jsonl(p):
    with open(p, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def truncate(tok, texts, cap):
    encs = tok.encode_batch(texts)
    out, lens = [], []
    for t, e in zip(texts, encs):
        if len(e.ids) > cap:
            t = t[: e.offsets[cap - 1][1]]
        out.append(t)
        lens.append(min(len(e.ids), cap))
    return out, lens


def build(a, tok, name, neg, rq, nq, max_doc, max_q):
    out_root = os.path.join(a.bright, name)
    meta = {
        "name": name,
        "seed": a.seed,
        "negatives_per_split": neg,
        "rerank_queries_per_split": rq,
        "max_doc_tokens": max_doc,
        "max_query_tokens": max_q,
        "queries_per_split": nq,
        "tokenizer": "Qwen3 (Qwen/Qwen3-Embedding-0.6B tokenizer.json)",
        "splits": {},
        "rerank_queries": {},
        "dataset": "xlangai/BRIGHT (CC-BY-4.0): documents + examples",
        "rule": "all gold documents of the selected queries + seeded random negatives; see make_subset.py",
    }
    for s in SPLITS if nq is None else SPLITS[: a.smoke_splits]:
        docs = read_jsonl(os.path.join(a.bright, s, "documents.jsonl"))
        exs = read_jsonl(os.path.join(a.bright, s, "examples.jsonl"))
        rng = random.Random(f"{a.seed}:{s}")
        exs = [e for e in exs if e["gold_ids"]]
        if nq is not None:
            exs = sorted(rng.sample(sorted(exs, key=lambda e: e["id"]), nq), key=lambda e: e["id"])
        gold = {g for e in exs for g in e["gold_ids"]}
        by_id = {d["id"]: d for d in docs}
        negs_pool = sorted(i for i in by_id if i not in gold)
        negs = sorted(random.Random(f"{a.seed}:{s}:neg").sample(negs_pool, neg))
        keep = sorted(gold | set(negs))
        texts, lens = truncate(tok, [by_id[i]["content"] for i in keep], max_doc)
        qtexts, qlens = truncate(tok, [e["query"] for e in exs], max_q)
        os.makedirs(os.path.join(out_root, s), exist_ok=True)
        with open(
            os.path.join(out_root, s, "documents.jsonl"), "w", encoding="utf-8", newline="\n"
        ) as f:
            for i, t, n_tok in zip(keep, texts, lens):
                f.write(
                    json.dumps(
                        {"id": i, "content": t, "n_tokens": n_tok, "gold": i in gold},
                        ensure_ascii=False,
                    )
                    + "\n"
                )
        with open(
            os.path.join(out_root, s, "queries.jsonl"), "w", encoding="utf-8", newline="\n"
        ) as f:
            for e, t, n_tok in zip(exs, qtexts, qlens):
                f.write(
                    json.dumps(
                        {
                            "id": e["id"],
                            "query": t,
                            "gold_ids": e["gold_ids"],
                            "excluded_ids": e["excluded_ids"],
                            "n_tokens": n_tok,
                        },
                        ensure_ascii=False,
                    )
                    + "\n"
                )
        qids = sorted(e["id"] for e in exs)
        meta["rerank_queries"][s] = sorted(
            random.Random(f"{a.seed}:{s}:rr").sample(qids, min(rq, len(qids)))
        )
        gl = [n for i, n in zip(keep, lens) if i in gold]
        nl = [n for i, n in zip(keep, lens) if i not in gold]
        meta["splits"][s] = {
            "n_docs": len(keep),
            "n_gold": len(gold),
            "n_negatives": len(negs),
            "n_queries": len(exs),
            "doc_tokens": sum(lens),
            "query_tokens": sum(qlens),
            "mean_gold_tokens": round(sum(gl) / max(1, len(gl)), 1),
            "mean_negative_tokens": round(sum(nl) / max(1, len(nl)), 1),
            "full_corpus_docs": len(docs),
        }
    # Budget at conservative throughput.
    tot_doc = sum(v["doc_tokens"] for v in meta["splits"].values())
    tot_q = sum(
        v["query_tokens"] + 30 * v["n_queries"] for v in meta["splits"].values()
    )  # +30: instruction prefix
    rr_q = sum(len(v) for v in meta["rerank_queries"].values())
    mean_q = tot_q / max(1, sum(v["n_queries"] for v in meta["splits"].values()))
    mean_doc = tot_doc / max(1, sum(v["n_docs"] for v in meta["splits"].values()))
    cand_doc = min(
        max_doc, max(mean_doc * 2.5, 250)
    )  # candidates skew long (gold are long); conservative
    pair_tokens = mean_q + cand_doc + 110  # 110: rerank template
    pairs = rr_q * 100
    budget = {
        "effective_tflops": EFFECTIVE_TFLOPS,
        "assumed_candidate_doc_tokens": round(cand_doc),
        "assumed_pair_tokens": round(pair_tokens),
        "rerank_pairs_per_model": pairs,
        "per_model_minutes": {},
    }
    total = 0.0
    for m, (params, kind) in MODELS.items():
        tps = EFFECTIVE_TFLOPS * 1e12 / (2 * params * 1e9)
        toks = tot_doc + tot_q if kind == "embed" else pairs * pair_tokens
        mins = toks / tps / 60
        budget["per_model_minutes"][m] = {
            "assumed_tokens_per_s": round(tps),
            "tokens": round(toks),
            "minutes": round(mins, 1),
        }
        total += mins
    overhead = 16 * 25 / 60 + 3  # 16 server starts at 25 s each + autotune and warm-up
    budget["compute_minutes"] = round(total, 1)
    budget["overhead_minutes"] = round(overhead, 1)
    budget["total_minutes"] = round(total + overhead, 1)
    meta["budget"] = budget
    with open(os.path.join(out_root, "subset.json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, indent=2)
    print(name, json.dumps({k: meta[k] for k in ("splits", "budget")}, indent=1))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bright", default=r"C:\Users\sethy\Projects\vault-eval\external\bright")
    ap.add_argument(
        "--tokenizer",
        default=r"C:\Users\sethy\Projects\vault-models\tokenizer\Qwen__Qwen3-Embedding-0.6B__tokenizer.json",
    )
    ap.add_argument("--seed", default="vault-phase0-1")
    ap.add_argument("--negatives", type=int, default=3000)
    ap.add_argument("--rerank-queries", type=int, default=20)
    ap.add_argument("--max-doc-tokens", type=int, default=512)
    ap.add_argument("--max-query-tokens", type=int, default=512)
    ap.add_argument("--smoke-splits", type=int, default=1)
    a = ap.parse_args()
    tok = Tokenizer.from_file(a.tokenizer)
    tok.no_truncation()
    build(
        a,
        tok,
        "subset-v1",
        a.negatives,
        a.rerank_queries,
        None,
        a.max_doc_tokens,
        a.max_query_tokens,
    )
    # CPU smoke: 1 split, 3 queries with their gold docs (capped at 256 tokens) + negatives -> about 50 documents.
    build(a, tok, "subset-smoke", 8, 3, 3, 256, 256)


if __name__ == "__main__":
    main()

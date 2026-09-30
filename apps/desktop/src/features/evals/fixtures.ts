// Synthetic C2 run files: a believable retrieval hillclimb, used by the tests and the dev harness.
import type { EvalRun, SplitResult } from "@/lib/backend"

const DOMAINS = [
  { name: "math", offset: 0.05 },
  { name: "english", offset: -0.03 },
  { name: "science", offset: 0.01 },
  { name: "chemistry", offset: -0.06 },
] as const

const clamp01 = (v: number) => Math.min(1, Math.max(0, v))
const r3 = (v: number) => Math.round(v * 1000) / 1000

/** Normal-approximation 95% interval, like a bootstrap over n questions. */
function ci(p: number, n: number): [number, number] {
  const h = 1.96 * Math.sqrt((p * (1 - p)) / n)
  return [r3(clamp01(p - h)), r3(clamp01(p + h))]
}

function split(recall10: number, n: number, withCi: boolean): SplitResult {
  const metrics = {
    "recall@5": r3(clamp01(recall10 - 0.072)),
    "recall@10": r3(recall10),
    "mrr@10": r3(clamp01(recall10 - 0.218)),
    "ndcg@10": r3(clamp01(recall10 - 0.121)),
  }
  const ci95 = withCi ? Object.fromEntries(Object.entries(metrics).map(([k, v]) => [k, ci(v, n)])) : undefined
  return { n, metrics, ...(ci95 ? { ci95 } : {}) }
}

type Step = { id: string; day: number; change: string; decision: EvalRun["decision"]; train: number; test: number; cost: number; config: Record<string, unknown> }

// Day 0 is 2026-09-03 10:00 UTC. Train climbs faster than test; two reverts are pure overfit.
const STEPS: Step[] = [
  { id: "bm25-baseline", day: 0, change: "baseline: BM25 only", decision: "baseline", train: 0.64, test: 0.612, cost: 0, config: { mode: "bm25", k: 10 } },
  { id: "hybrid-rrf", day: 2, change: "Hybrid: BM25 plus voyage-3 embeddings, reciprocal rank fusion", decision: "keep", train: 0.731, test: 0.684, cost: 0.41, config: { mode: "hybrid", k: 10, fusion: "rrf" } },
  { id: "chunk-256", day: 4, change: "Chunk size 512 to 256 tokens", decision: "revert", train: 0.752, test: 0.671, cost: 0.44, config: { mode: "hybrid", k: 10, chunk_tokens: 256 } },
  { id: "heading-path", day: 7, change: "Prepend the heading path to each chunk before embedding", decision: "keep", train: 0.748, test: 0.703, cost: 0.46, config: { mode: "hybrid", k: 10, heading_path: true } },
  { id: "rerank-30", day: 10, change: "Cross-encoder rerank of the top 30", decision: "keep", train: 0.792, test: 0.741, cost: 0.93, config: { mode: "hybrid", k: 10, rerank: 30 } },
  { id: "query-rewrite", day: 13, change: "LLM query rewrite before retrieval", decision: "revert", train: 0.801, test: 0.738, cost: 1.12, config: { mode: "hybrid", k: 10, rewrite: "haiku" } },
  { id: "rerank-50", day: 16, change: "Rerank the top 50 instead of 30", decision: "revert", train: 0.797, test: 0.735, cost: 1.31, config: { mode: "hybrid", k: 10, rerank: 50 } },
  { id: "parent-page", day: 19, change: "Merge neighbouring chunks from the same page into one hit", decision: "keep", train: 0.812, test: 0.764, cost: 0.95, config: { mode: "hybrid", k: 10, rerank: 30, merge_page: true } },
  { id: "per-domain-k", day: 22, change: "Tune k per domain on the train split", decision: "revert", train: 0.865, test: 0.748, cost: 0.97, config: { mode: "hybrid", k: "per-domain", rerank: 30, merge_page: true } },
  { id: "rrf-weight", day: 25, change: "Lower the BM25 weight in fusion from 0.4 to 0.3", decision: "keep", train: 0.82, test: 0.772, cost: 0.96, config: { mode: "hybrid", k: 10, rerank: 30, merge_page: true, bm25_weight: 0.3 } },
  { id: "math-notation", day: 26, change: "Normalise LaTeX and Unicode math before indexing", decision: "pending", train: 0.826, test: 0.779, cost: 1.04, config: { mode: "hybrid", k: 10, rerank: 30, merge_page: true, normalise_math: true } },
]

const BASE = Date.UTC(2026, 8, 3, 10, 0, 0) / 1000

export function sampleRuns(): EvalRun[] {
  return STEPS.map((s, i) => {
    const stamp = new Date((BASE + s.day * 86400 + i * 1260) * 1000)
    const iso = stamp.toISOString().replace(/[-:]/g, "").slice(0, 15) + "Z"
    return {
      run_id: `${iso}-${s.id}`,
      created_at: BASE + s.day * 86400 + i * 1260,
      eval: "retrieval",
      dataset: "evals/datasets/retrieval-v1.jsonl",
      change: s.change,
      config: s.config,
      splits: { train: split(s.train, 240, true), test: split(s.test, 96, true) },
      per_domain: Object.fromEntries(DOMAINS.map((d) => [d.name, { train: split(s.train + d.offset * 0.6, 60, false), test: split(s.test + d.offset, 24, false) }])),
      decision: s.decision,
      cost_usd: s.cost,
    }
  })
}

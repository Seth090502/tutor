"""Command line: python -m tutor_evals {split,generate,run,compare}."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from . import __version__
from .compare import compare_runs, format_comparison, write_decision
from .dataset import load_dataset, write_dataset
from .generate import DEFAULT_DOMAINS, MIN_CHUNK_WORDS, generate
from .paths import DEFAULT_DATASET, RUNS_DIR
from .run_retrieval import ARMS, MODES, format_report, run_retrieval
from .split import DEFAULT_SEED, DEFAULT_TRAIN_FRAC, apply_split

EPILOG = """\
workflow (details in evals/README.md):
  generate -> human spot-check (~50 questions, mark hard cases) -> split -> run --baseline
  -> make ONE change -> run --change "..." -> compare -> keep or revert

run from the evals/ directory, or set PYTHONPATH=evals.
"""


def _add_split(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser(
        "split",
        help="assign train/test by source document (deterministic, seeded)",
        description=(
            "Fill in the `split` field of a dataset. Questions are grouped by the document "
            "their gold points at (never by chunk) and each group is hashed with the seed, so "
            "a document never appears in both splits and a test document stays test when the "
            "dataset grows. Rewrites the file in place unless --out is given."
        ),
    )
    p.add_argument("dataset", type=Path, help="dataset JSONL file")
    p.add_argument(
        "--seed", type=int, default=DEFAULT_SEED, help="split seed (default %(default)s)"
    )
    p.add_argument(
        "--train-frac",
        type=float,
        default=DEFAULT_TRAIN_FRAC,
        help="target train share of document groups (default %(default)s)",
    )
    p.add_argument("--out", type=Path, help="write here instead of rewriting the input")
    p.set_defaults(func=_cmd_split)


def _add_generate(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser(
        "generate",
        help="write synthetic questions for sampled chunks with an LLM",
        description=(
            "Sample chunks from rag-engine (or a chunks JSONL export), stratified by domain, "
            "and ask an OpenAI-compatible Chat Completions model for one question per chunk. "
            "Questions sharing 8+ consecutive tokens with their chunk, or under 6 words, are "
            "dropped. Configure the model with EVAL_LLM_BASE_URL, EVAL_LLM_API_KEY and "
            "EVAL_LLM_MODEL; set EVAL_LLM_PRICE_IN_PER_MTOK / EVAL_LLM_PRICE_OUT_PER_MTOK (USD) "
            "to get a cost in the estimate and in <dataset>.gen.json. The generator must come "
            "from a different model family than the tutor. Rows are appended as they are "
            "accepted; rerun the same command to resume."
        ),
    )
    p.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_DATASET,
        help="dataset JSONL to append to (default evals/datasets/retrieval-v1.jsonl)",
    )
    p.add_argument(
        "--domains",
        default=",".join(DEFAULT_DOMAINS),
        help="comma-separated domains (default %(default)s)",
    )
    p.add_argument(
        "--per-domain", type=int, default=250, help="questions per domain (default %(default)s)"
    )
    p.add_argument("--seed", type=int, default=0, help="sampling seed (default %(default)s)")
    p.add_argument(
        "--chunks-jsonl",
        type=Path,
        help="sample from this chunks export (one C1 hit object per line) instead of rag-engine",
    )
    p.add_argument(
        "--engine-url", help="rag-engine URL (default: $RAG_ENGINE_URL or http://127.0.0.1:8811)"
    )
    p.add_argument(
        "--seed-terms",
        type=Path,
        help="seed terms file: `term` or `domain<TAB>term` per line (default: built-in lists)",
    )
    p.add_argument(
        "--oversample",
        type=float,
        default=1.5,
        help="sample this many times --per-domain chunks so filtered questions can be replaced (default %(default)s)",
    )
    p.add_argument(
        "--max-per-doc",
        type=int,
        default=5,
        help="at most this many chunks from one document (default %(default)s)",
    )
    p.add_argument(
        "--min-chunk-words",
        type=int,
        default=MIN_CHUNK_WORDS,
        help="skip chunks shorter than this (default %(default)s)",
    )
    p.add_argument(
        "--engine-k", type=int, default=10, help="hits per sampling query (default %(default)s)"
    )
    p.add_argument(
        "--max-queries",
        type=int,
        default=500,
        help="sampling queries per domain before giving up (default %(default)s)",
    )
    p.add_argument(
        "--workers", type=int, default=4, help="parallel LLM requests (default %(default)s)"
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="print prompts and a cost estimate; make no LLM calls, need no key",
    )
    p.add_argument(
        "--show", type=int, default=3, help="prompts to print in --dry-run (default %(default)s)"
    )
    p.set_defaults(func=_cmd_generate)


def _add_run(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser(
        "run",
        help="score retrieval against rag-engine and write evals/runs/<run_id>.json",
        description=(
            "POST every question to rag-engine /search and score the hits against gold: "
            "recall@5, recall@10, mrr@10, ndcg@10 per split and per domain, with 95% bootstrap "
            "CIs over questions (2,000 resamples, fixed seed). A hit matches gold on chunk_id, "
            "or on doc_id plus page, or on path plus page. Writes contract C2 "
            "evals/runs/<run_id>.json plus a per-question sidecar that `compare` needs."
        ),
    )
    p.add_argument(
        "--change",
        required=True,
        help="the ONE change under test, in a sentence (goes in the run_id)",
    )
    p.add_argument(
        "--baseline",
        action="store_true",
        help='mark this run decision="baseline" (default "pending")',
    )
    p.add_argument(
        "--dataset",
        type=Path,
        default=DEFAULT_DATASET,
        help="dataset JSONL with splits assigned (default evals/datasets/retrieval-v1.jsonl)",
    )
    p.add_argument(
        "--mode", choices=MODES, default="hybrid", help="search mode (default %(default)s)"
    )
    p.add_argument(
        "--k",
        type=int,
        default=10,
        help="hits requested per query, at least 10 (default %(default)s)",
    )
    p.add_argument(
        "--arm",
        choices=sorted(ARMS),
        help=(
            "run through DeepTutor library search with this CORPUS.md Stage 0 arm's settings: "
            "A raw BM25, B A's settings on the fields/tokenizer engine build, C plus rewrites "
            "and RRF, D plus rerank, E plus agentic search (needs --k 10-12)"
        ),
    )
    p.add_argument(
        "--url",
        help="rag-engine URL (default: $RAG_ENGINE_URL or http://127.0.0.1:8811); with --arm, "
        "the DeepTutor URL (default: $DEEPTUTOR_URL or http://127.0.0.1:8001)",
    )
    p.add_argument(
        "--config",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="record engine-side settings in the run's config, e.g. reranker=none (repeatable)",
    )
    p.add_argument(
        "--workers", type=int, default=4, help="parallel search requests (default %(default)s)"
    )
    p.add_argument(
        "--runs-dir", type=Path, default=RUNS_DIR, help="where run files go (default evals/runs)"
    )
    p.set_defaults(func=_cmd_run)


def _add_compare(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser(
        "compare",
        help="paired bootstrap between two runs; verdict keep / revert / noise",
        description=(
            "Compare two runs on the same dataset, per split, with a paired bootstrap over "
            "questions. Verdict: keep only if train AND test improve and both lower CI bounds "
            "on the delta are above 0; revert if either split gets worse; otherwise noise "
            "(do not keep). --write-decision records it in the newer run (keep->keep, "
            "revert->revert, noise->revert)."
        ),
    )
    p.add_argument("baseline", help="baseline run: a file path or a run_id in --runs-dir")
    p.add_argument("new", help="newer run to judge: a file path or a run_id")
    p.add_argument(
        "--metric",
        choices=["recall@5", "recall@10", "mrr@10", "ndcg@10"],
        default="ndcg@10",
        help="metric the verdict is based on (default %(default)s)",
    )
    p.add_argument(
        "--write-decision", action="store_true", help="write the verdict into the newer run file"
    )
    p.add_argument(
        "--runs-dir",
        type=Path,
        default=RUNS_DIR,
        help="where run ids are looked up (default evals/runs)",
    )
    p.set_defaults(func=_cmd_compare)


def _cmd_split(args: argparse.Namespace) -> int:
    rows = load_dataset(args.dataset)
    summary = apply_split(rows, seed=args.seed, train_frac=args.train_frac)
    write_dataset(args.out or args.dataset, rows)
    print(f"{summary['n']} questions, {sum(summary['document_groups'].values())} document groups")
    for split in ("train", "test"):
        print(
            f"  {split}: {summary['by_split'][split]} questions, {summary['document_groups'][split]} document groups"
        )
    for domain, counts in summary["by_domain"].items():
        print(f"  {domain}: train {counts['train']}, test {counts['test']}")
    for warning in summary["warnings"]:
        print(f"warning: {warning}")
    print(f"wrote {args.out or args.dataset}")
    return 0


def _cmd_generate(args: argparse.Namespace) -> int:
    domains = [d.strip() for d in args.domains.split(",") if d.strip()]
    result = generate(
        out_path=args.out,
        domains=domains,
        per_domain=args.per_domain,
        seed=args.seed,
        chunks_jsonl=args.chunks_jsonl,
        engine=args.engine_url,
        seed_terms=args.seed_terms,
        oversample=args.oversample,
        max_per_doc=args.max_per_doc,
        min_chunk_words=args.min_chunk_words,
        engine_k=args.engine_k,
        max_queries=args.max_queries,
        workers=args.workers,
        dry_run=args.dry_run,
        show=args.show,
    )
    if not args.dry_run:
        cost = (
            "unknown (set EVAL_LLM_PRICE_*_PER_MTOK)"
            if result["cost_usd"] is None
            else f"${result['cost_usd']:.4f}"
        )
        print(
            f"accepted {sum(result['accepted'].values())} questions {result['accepted']}, dropped {result['dropped']}"
        )
        print(
            f"{result['requests']} LLM requests, {result['prompt_tokens']} prompt + {result['completion_tokens']} completion tokens, cost {cost}"
        )
        print(f"next: spot-check ~50 questions, then `python -m tutor_evals split {args.out}`")
    return 0


def _cmd_run(args: argparse.Namespace) -> int:
    config = {}
    for item in args.config:
        key, sep, value = item.partition("=")
        if not sep or not key:
            raise ValueError(f"--config expects KEY=VALUE, got {item!r}")
        config[key] = value
    result = run_retrieval(
        args.dataset,
        change=args.change,
        mode=args.mode,
        k=args.k,
        url=args.url,
        arm=args.arm,
        runs_dir=args.runs_dir,
        baseline=args.baseline,
        extra_config=config,
        workers=args.workers,
    )
    print(format_report(result))
    return 0


def _cmd_compare(args: argparse.Namespace) -> int:
    result = compare_runs(args.baseline, args.new, metric=args.metric, runs_dir=args.runs_dir)
    print(format_comparison(result))
    if args.write_decision:
        decision = write_decision(result)
        print(f"wrote decision={decision} to {result['new_path']}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python -m tutor_evals",
        description="Retrieval eval harness: train/test split by document, one change per round, "
        "keep only if train AND test improve beyond the noise floor, 95% CIs on every number.",
        epilog=EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--version", action="version", version=f"tutor_evals {__version__}")
    sub = parser.add_subparsers(
        dest="command", required=True, metavar="{split,generate,run,compare}"
    )
    _add_split(sub)
    _add_generate(sub)
    _add_run(sub)
    _add_compare(sub)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.func(args)
    except (ValueError, RuntimeError, OSError) as exc:  # user-facing failures, not bugs
        print(f"error: {exc}", file=sys.stderr)
        return 2

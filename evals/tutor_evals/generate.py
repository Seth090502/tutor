"""Synthetic question generation: one LLM-written question per sampled chunk.

Chunks come from rag-engine (POST /search with sampled seed terms) or from a chunks JSONL
export, stratified by domain. The LLM is any OpenAI-compatible Chat Completions endpoint,
configured by EVAL_LLM_BASE_URL, EVAL_LLM_API_KEY and EVAL_LLM_MODEL. Use a model from a
different family than the tutor's: a generator (or judge) from the tutor's own family
favours the tutor's phrasing and inflates scores.

Rows are appended as they are accepted, so an interrupted run loses no paid work: run the
same command again and chunks that already have a question are skipped.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Mapping
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re

from .dataset import dump_row, load_dataset
from .httpjson import engine_url, post_json
from .paths import repo_relative

DEFAULT_DOMAINS = ("math", "english", "science", "chemistry")
MIN_QUESTION_WORDS = 6
VERBATIM_SPAN_TOKENS = 8
MIN_CHUNK_WORDS = 60
CHARS_PER_TOKEN = 4  # rough English/math-text average, only used for the dry-run estimate
EST_OUTPUT_TOKENS = 40
MAX_PASSAGE_CHARS = 8000

ENV_REQUIRED = ("EVAL_LLM_BASE_URL", "EVAL_LLM_API_KEY", "EVAL_LLM_MODEL")
ENV_PRICE_IN = "EVAL_LLM_PRICE_IN_PER_MTOK"
ENV_PRICE_OUT = "EVAL_LLM_PRICE_OUT_PER_MTOK"

SEED_TERMS = {
    "math": "theorem proof derivative integral matrix eigenvalue probability group function limit series vector".split(),
    "english": "metaphor syntax narrative rhetoric sentence grammar poetry argument essay character tone".split(),
    "science": "energy cell evolution force momentum ecosystem gravity wave genetics experiment entropy".split(),
    "chemistry": "bond molecule reaction acid equilibrium electron enthalpy orbital solution oxidation".split(),
}

SYSTEM_PROMPT = """\
You write retrieval-evaluation questions for a personal study library.
Given one passage, write exactly ONE question that a student could answer using only that passage.
Rules:
- The question must make sense without seeing the passage: name the subject explicitly. Never say
  "the passage", "the text", "the author", "this chapter", "the figure" or "above".
- Do not copy wording from the passage. Paraphrase; never reuse four or more consecutive words.
- Write at least 8 words. No yes/no questions.
- Reply with only the question, nothing else."""

_TOKEN = re.compile(r"\w+")


class GenerateError(ValueError):
    """Generation cannot proceed (bad configuration, unusable source, failed LLM call)."""


# --- filters -----------------------------------------------------------------------------


def tokenize(text: str) -> list[str]:
    return _TOKEN.findall(text.lower())


def shares_verbatim_span(question: str, passage: str, n: int = VERBATIM_SPAN_TOKENS) -> bool:
    """True if the question and passage share n or more consecutive tokens."""
    q = tokenize(question)
    if len(q) < n:
        return False
    p = tokenize(passage)
    grams = {tuple(p[i : i + n]) for i in range(len(p) - n + 1)}
    return any(tuple(q[i : i + n]) in grams for i in range(len(q) - n + 1))


def filter_reason(question: str, passage: str) -> str | None:
    """Why a generated question is dropped, or None to keep it."""
    if len(question.split()) < MIN_QUESTION_WORDS:
        return "too_short"
    if shares_verbatim_span(question, passage):
        return "verbatim_span"
    return None


def clean_question(raw: str) -> str:
    lines = [line.strip() for line in raw.strip().splitlines() if line.strip()]
    text = lines[0] if lines else ""
    text = re.sub(r"^(question\s*:\s*)", "", text, flags=re.IGNORECASE)
    return text.strip().strip("\"'`").strip()


# --- prompts and LLM ---------------------------------------------------------------------


def build_messages(chunk: dict) -> list[dict]:
    passage = chunk["text"][:MAX_PASSAGE_CHARS]
    user = f'Domain: {chunk["domain"]}\n\nPassage:\n"""\n{passage}\n"""\n\nWrite the question.'
    return [{"role": "system", "content": SYSTEM_PROMPT}, {"role": "user", "content": user}]


@dataclass(frozen=True)
class LLMConfig:
    base_url: str
    api_key: str
    model: str
    price_in: float | None  # USD per million input tokens
    price_out: float | None

    @classmethod
    def from_env(cls, env: Mapping[str, str], *, require: bool) -> LLMConfig:
        missing = [k for k in ENV_REQUIRED if not env.get(k)]
        if require and missing:
            raise GenerateError(
                f"set {', '.join(missing)} (an OpenAI-compatible Chat Completions endpoint); "
                "or use --dry-run to see the prompts and a cost estimate without a key"
            )
        return cls(
            base_url=env.get("EVAL_LLM_BASE_URL", ""),
            api_key=env.get("EVAL_LLM_API_KEY", ""),
            model=env.get("EVAL_LLM_MODEL", ""),
            price_in=_price(env, ENV_PRICE_IN),
            price_out=_price(env, ENV_PRICE_OUT),
        )


def _price(env: Mapping[str, str], name: str) -> float | None:
    raw = env.get(name)
    if not raw:
        return None
    try:
        value = float(raw)
    except ValueError:
        raise GenerateError(
            f"{name} must be a number (USD per million tokens), got {raw!r}"
        ) from None
    if value < 0:
        raise GenerateError(f"{name} must not be negative")
    return value


def chat(cfg: LLMConfig, messages: list[dict], *, timeout: float = 120.0) -> tuple[str, dict]:
    data = post_json(
        f"{cfg.base_url.rstrip('/')}/chat/completions",
        {"model": cfg.model, "messages": messages},
        headers={"Authorization": f"Bearer {cfg.api_key}"},
        timeout=timeout,
    )
    try:
        content = data["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError):
        raise GenerateError(f"unexpected chat completion response: {str(data)[:200]}") from None
    if not isinstance(content, str):
        raise GenerateError(f"chat completion content is not text: {str(data)[:200]}")
    usage = data.get("usage")
    return content, usage if isinstance(usage, dict) else {}


def request_cost(usage: dict, cfg: LLMConfig) -> float | None:
    """Billed cost if the endpoint reports one (e.g. OpenRouter), else tokens x configured price."""
    if isinstance(usage.get("cost"), (int, float)):
        return float(usage["cost"])
    if cfg.price_in is None or cfg.price_out is None:
        return None
    return (
        usage.get("prompt_tokens", 0) * cfg.price_in
        + usage.get("completion_tokens", 0) * cfg.price_out
    ) / 1e6


def estimate(chunks: list[dict], cfg: LLMConfig) -> dict:
    chars = sum(len(m["content"]) for c in chunks for m in build_messages(c))
    in_tokens = math.ceil(chars / CHARS_PER_TOKEN)
    out_tokens = EST_OUTPUT_TOKENS * len(chunks)
    cost = None
    if cfg.price_in is not None and cfg.price_out is not None:
        cost = (in_tokens * cfg.price_in + out_tokens * cfg.price_out) / 1e6
    return {
        "requests": len(chunks),
        "input_tokens": in_tokens,
        "output_tokens": out_tokens,
        "cost_usd": cost,
    }


# --- sampling ----------------------------------------------------------------------------


def synthetic_qid(chunk_id: str) -> str:
    return "syn-" + hashlib.sha256(chunk_id.encode()).hexdigest()[:12]


def _eligible(chunk: dict, domain: str, min_words: int) -> bool:
    return (
        chunk.get("domain") == domain
        and all(isinstance(chunk.get(k), str) and chunk[k] for k in ("chunk_id", "path", "text"))
        and len(chunk["text"].split()) >= min_words
    )


def select_candidates(
    chunks: list[dict], need: int, max_per_doc: int, rng: random.Random
) -> list[dict]:
    """Shuffle, then take up to `need` chunks with at most max_per_doc per document."""
    shuffled = list(chunks)
    rng.shuffle(shuffled)
    per_doc: dict[str, int] = defaultdict(int)
    picked = []
    for chunk in shuffled:
        doc = chunk.get("doc_id") or chunk["path"]
        if per_doc[doc] < max_per_doc:
            per_doc[doc] += 1
            picked.append(chunk)
            if len(picked) == need:
                break
    return picked


def _domain_rng(seed: int, domain: str) -> random.Random:
    return random.Random(f"{seed}:{domain}")


def sample_from_jsonl(
    path: Path, domain: str, *, need: int, seed: int, max_per_doc: int, min_words: int
) -> list[dict]:
    found: dict[str, dict] = {}
    with path.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            if not line.strip():
                continue
            try:
                chunk = json.loads(line)
            except ValueError as exc:
                raise GenerateError(f"{path.name}:{lineno}: invalid JSON ({exc})") from exc
            if isinstance(chunk, dict) and _eligible(chunk, domain, min_words):
                found.setdefault(chunk["chunk_id"], chunk)
    return select_candidates(list(found.values()), need, max_per_doc, _domain_rng(seed, domain))


def sample_from_engine(
    url: str,
    domain: str,
    *,
    need: int,
    seed: int,
    terms: list[str],
    k: int,
    max_queries: int,
    max_per_doc: int,
    min_words: int,
) -> list[dict]:
    """Random-walk sampling: query BM25 with random terms, grow the term pool from the hits."""
    if not terms:
        raise GenerateError(f"no seed terms for domain {domain!r}; pass --seed-terms")
    rng = _domain_rng(seed, domain)
    pool = list(terms)
    found: dict[str, dict] = {}
    for _ in range(max_queries):
        if len(found) >= 2 * need:
            break
        query = " ".join(rng.sample(pool, min(2, len(pool))))
        response = post_json(
            f"{url}/search", {"query": query, "k": k, "domains": [domain], "mode": "bm25"}
        )
        for hit in response.get("hits", []):
            if _eligible(hit, domain, min_words):
                found.setdefault(hit["chunk_id"], hit)
                words = [w for w in tokenize(hit["text"]) if len(w) >= 6 and w.isalpha()]
                pool.extend(rng.sample(words, min(2, len(words))))
    return select_candidates(list(found.values()), need, max_per_doc, rng)


def load_seed_terms(path: Path | None, domains: list[str]) -> dict[str, list[str]]:
    """Seed terms per domain. File lines: `term` (all domains) or `domain<TAB>term`; # comments."""
    terms = {d: list(SEED_TERMS.get(d, [])) for d in domains}
    if path is None:
        return terms
    custom: dict[str, list[str]] = defaultdict(list)
    shared: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        domain, sep, term = line.partition("\t")
        if sep:
            custom[domain.strip()].append(term.strip())
        else:
            shared.append(line)
    return {d: (custom.get(d, []) + shared) or terms[d] for d in domains}


# --- manifest ----------------------------------------------------------------------------


def manifest_path(out_path: Path) -> Path:
    return out_path.with_name(out_path.stem + ".gen.json")


def record_invocation(out_path: Path, invocation: dict) -> Path:
    """Append this invocation's cost record to <dataset>.gen.json and refresh the total."""
    path = manifest_path(out_path)
    manifest = (
        json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"invocations": []}
    )
    manifest["dataset"] = repo_relative(out_path)
    manifest["invocations"].append(invocation)
    known = [i["cost_usd"] for i in manifest["invocations"] if i["cost_usd"] is not None]
    manifest["total_cost_usd"] = sum(known)
    manifest["total_cost_complete"] = all(i["cost_complete"] for i in manifest["invocations"])
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    return path


# --- main entry --------------------------------------------------------------------------


def generate(
    *,
    out_path: Path,
    domains: list[str],
    per_domain: int,
    seed: int = 0,
    chunks_jsonl: Path | None = None,
    engine: str | None = None,
    seed_terms: Path | None = None,
    oversample: float = 1.5,
    max_per_doc: int = 5,
    min_chunk_words: int = MIN_CHUNK_WORDS,
    engine_k: int = 10,
    max_queries: int = 500,
    workers: int = 4,
    dry_run: bool = False,
    show: int = 3,
    env: Mapping[str, str] | None = None,
) -> dict:
    env = os.environ if env is None else env
    if per_domain < 1 or oversample < 1:
        raise GenerateError("--per-domain must be at least 1 and --oversample at least 1.0")
    cfg = LLMConfig.from_env(env, require=not dry_run)
    need = math.ceil(per_domain * oversample)

    candidates: dict[str, list[dict]] = {}
    if chunks_jsonl is not None:
        source = f"chunks-jsonl:{chunks_jsonl.name}"
        for domain in domains:
            candidates[domain] = sample_from_jsonl(
                chunks_jsonl,
                domain,
                need=need,
                seed=seed,
                max_per_doc=max_per_doc,
                min_words=min_chunk_words,
            )
    else:
        url = engine_url(engine)
        source = f"engine:{url}"
        terms = load_seed_terms(seed_terms, domains)
        for domain in domains:
            candidates[domain] = sample_from_engine(
                url,
                domain,
                need=need,
                seed=seed,
                terms=terms[domain],
                k=engine_k,
                max_queries=max_queries,
                max_per_doc=max_per_doc,
                min_words=min_chunk_words,
            )
    for domain in domains:
        if len(candidates[domain]) < per_domain:
            print(
                f"warning: only {len(candidates[domain])} usable chunks for {domain!r} "
                f"(wanted {per_domain}); the corpus or sampling budget is too small"
            )

    existing = load_dataset(out_path) if out_path.exists() else []
    done_qids = {r["qid"] for r in existing}
    have = defaultdict(int)
    for row in existing:
        if row["kind"] == "synthetic":
            have[row["domain"]] += 1

    if dry_run:
        return _dry_run(candidates, domains, per_domain, have, done_qids, cfg, show)
    return _generate(
        candidates,
        domains,
        per_domain,
        have,
        done_qids,
        cfg,
        out_path,
        seed=seed,
        source=source,
        workers=workers,
    )


def _dry_run(candidates, domains, per_domain, have, done_qids, cfg, show) -> dict:
    target: list[dict] = []
    for domain in domains:
        todo = [c for c in candidates[domain] if synthetic_qid(c["chunk_id"]) not in done_qids]
        target += todo[: max(per_domain - have[domain], 0)]
    every = [
        c for d in domains for c in candidates[d] if synthetic_qid(c["chunk_id"]) not in done_qids
    ]
    print("DRY RUN: no LLM calls are made and nothing is written.")
    print(f"model: {cfg.model or '(EVAL_LLM_MODEL not set)'}")
    for domain in domains:
        print(
            f"  {domain}: {len(candidates[domain])} candidate chunks, {have[domain]} questions already generated"
        )
    for i, chunk in enumerate(target[:show], 1):
        print(f"\n--- prompt {i}/{len(target)} ({chunk['domain']}, chunk {chunk['chunk_id']}) ---")
        for message in build_messages(chunk):
            print(f"[{message['role']}]\n{message['content']}")
    if len(target) > show:
        print(f"\n(+{len(target) - show} more prompts not shown; --show N prints more)")
    est, cap = estimate(target, cfg), estimate(every, cfg)
    print(
        f"\nestimate: {est['requests']} requests, ~{est['input_tokens']} input tokens "
        f"(chars/{CHARS_PER_TOKEN}), ~{est['output_tokens']} output tokens ({EST_OUTPUT_TOKENS} per question)"
    )
    if est["cost_usd"] is None:
        print(f"cost: unknown; set {ENV_PRICE_IN} and {ENV_PRICE_OUT} (USD per million tokens)")
    else:
        print(
            f"cost: ~${est['cost_usd']:.4f} (upper bound ~${cap['cost_usd']:.4f} if filters drop "
            f"questions and every oversampled candidate is used)"
        )
    return {"dry_run": True, "estimate": est, "upper_bound": cap}


def _generate(
    candidates, domains, per_domain, have, done_qids, cfg, out_path, *, seed, source, workers
) -> dict:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    started = datetime.now(timezone.utc)
    stats = {"requests": 0, "prompt_tokens": 0, "completion_tokens": 0, "unpriced": 0, "cost": 0.0}
    dropped: dict[str, int] = defaultdict(int)
    accepted: dict[str, int] = defaultdict(int)
    error: Exception | None = None
    needs_newline = (
        out_path.exists()
        and out_path.stat().st_size > 0
        and not out_path.read_bytes().endswith(b"\n")
    )

    def call(chunk: dict) -> tuple[str, dict]:
        return chat(cfg, build_messages(chunk))

    try:
        with (
            out_path.open("a", encoding="utf-8", newline="\n") as handle,
            ThreadPoolExecutor(max(1, workers)) as pool,
        ):
            if needs_newline:
                handle.write("\n")
            for domain in domains:
                queue = [
                    c for c in candidates[domain] if synthetic_qid(c["chunk_id"]) not in done_qids
                ]
                while have[domain] < per_domain and queue and error is None:
                    take = per_domain - have[domain]
                    batch, queue = queue[:take], queue[take:]
                    for chunk, future in [(c, pool.submit(call, c)) for c in batch]:
                        try:
                            raw, usage = future.result()
                        except Exception as exc:  # keep paid results from the rest of the wave
                            error = error or exc
                            continue
                        stats["requests"] += 1
                        stats["prompt_tokens"] += int(usage.get("prompt_tokens") or 0)
                        stats["completion_tokens"] += int(usage.get("completion_tokens") or 0)
                        cost = request_cost(usage, cfg)
                        if cost is None:
                            stats["unpriced"] += 1
                        else:
                            stats["cost"] += cost
                        question = clean_question(raw)
                        reason = filter_reason(question, chunk["text"])
                        if reason:
                            dropped[reason] += 1
                            continue
                        gold = {
                            "chunk_id": chunk["chunk_id"],
                            "doc_id": chunk.get("doc_id"),
                            "path": chunk["path"],
                            "page": chunk.get("page_start"),
                        }
                        row = {
                            "qid": synthetic_qid(chunk["chunk_id"]),
                            "question": question,
                            "gold": [{k: v for k, v in gold.items() if v is not None}],
                            "domain": domain,
                            "split": None,
                            "kind": "synthetic",
                            "source_chunk_id": chunk["chunk_id"],
                        }
                        handle.write(dump_row(row) + "\n")
                        handle.flush()
                        have[domain] += 1
                        accepted[domain] += 1
    finally:
        priced_any = stats["requests"] > stats["unpriced"]
        invocation = {
            "started_at": started.isoformat(timespec="seconds"),
            "model": cfg.model,
            "base_url": cfg.base_url,
            "source": source,
            "domains": domains,
            "per_domain": per_domain,
            "seed": seed,
            "requests": stats["requests"],
            "prompt_tokens": stats["prompt_tokens"],
            "completion_tokens": stats["completion_tokens"],
            "cost_usd": stats["cost"] if priced_any or stats["requests"] == 0 else None,
            "cost_complete": stats["unpriced"] == 0,
            "accepted": dict(accepted),
            "dropped": dict(dropped),
        }
        record_invocation(out_path, invocation)
    if error is not None:
        raise GenerateError(
            f"LLM call failed ({error}); accepted rows and cost were saved, rerun to resume"
        ) from error
    return invocation

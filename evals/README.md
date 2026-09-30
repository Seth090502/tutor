# Retrieval evals

Hillclimbing harness for the rag-engine. The rules:

- A **train/test split by source document**. The test split is held out: it gates a change, it is
  never used to choose what to try.
- **One change per round.**
- **Keep a change only if train AND test both improve beyond the noise floor.**
- A **95% bootstrap CI on every number**.

## Setup

```powershell
.venv\Scripts\python -m pip install -r evals\requirements.txt    # ir_measures (+ pytrec_eval, scipy), pytest
$env:PYTHONPATH = "evals"      # or `cd evals`; then `python -m tutor_evals --help`
```

rag-engine must be serving (`rag-engine serve ...`, default `http://127.0.0.1:8811`, override with
`RAG_ENGINE_URL` or `--url`).

## Workflow

1. **Generate** synthetic questions, stratified by domain. Preview first; no key needed:
   ```powershell
   python -m tutor_evals generate --dry-run --per-domain 250
   $env:EVAL_LLM_BASE_URL = "https://.../v1"; $env:EVAL_LLM_API_KEY = "..."; $env:EVAL_LLM_MODEL = "..."
   $env:EVAL_LLM_PRICE_IN_PER_MTOK = "..."; $env:EVAL_LLM_PRICE_OUT_PER_MTOK = "..."   # USD, for the cost record
   python -m tutor_evals generate --per-domain 250
   ```
   Output: `evals/datasets/retrieval-v1.jsonl` (no splits yet) and `retrieval-v1.gen.json` (cost).
   An interrupted run is resumed by running the same command again.
2. **Spot-check by hand**: read about 50 questions. Delete lines that are bad (unanswerable, not
   self-contained, answered by a different chunk). Set `"hard": true` on the hard ones and
   `"kind": "human"` on any you rewrote. Add human-written questions as extra lines; see
   `datasets/README.md`. Do this before splitting.
3. **Split**: `python -m tutor_evals split evals/datasets/retrieval-v1.jsonl`. Re-run after adding
   questions; existing documents keep their split.
4. **Baseline**: `python -m tutor_evals run --baseline --change "baseline: BM25 only" --mode bm25`
5. **Make one change** (an embedding model, a reranker, a chunk size, ...). Re-ingest if needed.
   Do not change the dataset or corpus in the same round.
6. **Run**: `python -m tutor_evals run --change "add voyage rerank" --mode hybrid --config reranker=voyage-rerank-2.5`
   (`--config KEY=VALUE` records engine-side settings the harness cannot see).
7. **Compare**: `python -m tutor_evals compare <baseline_run_id> <new_run_id> --write-decision`
   Verdict, on `--metric` (default `ndcg@10`; pick the primary metric before looking):
   - **keep**: train AND test both improve and both lower CI bounds on the delta are above 0.
   - **revert**: either split got worse.
   - **noise**: neither got worse, but the gain is not clear of the noise floor. Do not keep it.
     `--write-decision` records it as `pending`, since contract C2 has no `noise` decision.
8. **Keep or revert** the change in the engine accordingly. The next baseline is the last kept run.

## Commands and files

| Command | What |
|---|---|
| `split` | Fill `split` by document hash (seeded, default 70/30). |
| `generate` | Sample chunks, LLM writes one question each, filters, cost record. |
| `run` | `POST /search` for every question, score, write the run file. |
| `compare` | Paired bootstrap per split, verdict, optional decision write. |

- `evals/runs/<UTC timestamp>-<slug of change>.json`: the run file, contract C2 exactly (gitignored).
  Desktop app reads these.
- `evals/runs/<run_id>.questions.jsonl`: per-question scores and retrieved chunk ids. Not part of
  C2 (hence a different extension, so the app does not read it as a run); `compare` needs it for
  the paired bootstrap.
- `per_domain` cells in C2 have no CI slot, so per-domain CIs are printed by `run` but not stored.
- `cost_usd` in a run is 0.0: scoring makes no LLM calls, and engine-side query-embedding spend is
  not attributed to runs. Generation cost lives in `<dataset>.gen.json`.

## Metrics

recall@5, recall@10, mrr@10, ndcg@10 (binary gains, log2 discount), computed by
[ir_measures](https://ir-measur.es) (pytrec_eval backend) over a relevance list built with the
matching rules in `datasets/README.md`. `ir_measures.parse_measure()` is broken on Python 3.14
(`ast.Num`), so the harness uses its object API (`ir_measures.R @ 5`). Hand-computed cases are in
`tests/test_metrics.py`. CIs: percentile bootstrap over questions, 2,000 resamples, fixed seed
(`stats.py`); `compare` resamples the same questions for both runs (paired).

## LLM for generation: use a different model family than the tutor

The tutor runs Muse Spark (via OpenCode Go) or Claude. A question writer or judge from the same
family shares its phrasing and blind spots, which inflates the scores. Set `EVAL_LLM_MODEL` to a
model from a different family than whichever one the tutor is using. Any OpenAI-compatible
Chat Completions endpoint works (`EVAL_LLM_BASE_URL` ends before `/chat/completions`). The model
name is recorded in `<dataset>.gen.json`. The harness cannot check the family for you.

Generation filters: a question is dropped if it is under 6 words or shares a verbatim span of 8 or
more tokens (lowercased, punctuation ignored) with its chunk. It does not catch paraphrases that
keep the chunk's rare words, so synthetic questions still favour lexical retrieval; the hand-written
hard cases are the check against that.

## Statistical caveats

- **Sample size.** The CI half-width on a paired difference is about `1.96 * sd / sqrt(n)`, where
  `sd` is the spread of per-question score differences. For one change to a system (runs agree on
  most questions, `sd` around 0.25 for a 0/1 metric) a 3-point gain needs roughly 270 questions in
  the test split to get its lower bound above 0 about half the time, and about 550 for 80% power.
  With a 70/30 split that is the "800-1,000 questions" figure at best and closer to 1,800 for
  reliable detection. Below that, expect `noise` for real 3-point gains. Gains of 5+ points resolve
  sooner. Read the printed CIs, not the rule of thumb.
- **The test split is the binding constraint**, because it is 30% of the data and must clear the
  bar on its own.
- **Questions from one document are correlated**, but the bootstrap resamples questions as if
  independent, so CIs are somewhat too narrow. `generate --max-per-doc` (default 5) limits it.
- **Reusing the test split wears it out.** Every `compare` looks at it. After many rounds the kept
  changes are optimistic on test too. Do not choose changes by reading test failures; add new test
  documents periodically.
- **Check one metric per round**, chosen in advance. Picking the metric after the fact raises the
  false keep rate.
- **Paired comparison needs identical questions and the same corpus.** `compare` refuses runs with
  different datasets or question sets. Re-ingesting between baseline and candidate (new chunk ids,
  new documents) invalidates the pair; change only one thing.
- **Page-based gold matching is lenient**: any chunk on the gold page counts.

## Tests

```powershell
.venv\Scripts\python -m pytest -q evals\tests
```
Uses a local stub HTTP server for rag-engine and the chat LLM; no network, no key, no spend.

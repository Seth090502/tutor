# Repo adapters: what was taken from each clone, where it lives natively

Clones live OUTSIDE OneDrive at `C:\Users\sethy\AppData\Local\Temp\opencode\tutor-repos\`
(shallow depth 1, never vendored). Only distilled patterns were copied as small
stdlib-only modules. No upstream code is vendored.

- rasbt/LLMs-from-scratch (ch01: no code, first-principles + bonus materials)
  -> `tutor/curriculum.py` EXPLAINERS stay definition->mechanism->3 checks;
  `tutor/cli.py learn` shows prereqs + checks before retrieval.
- DataTalksClub/llm-zoomcamp (cohorts/2026/homework/04-evaluation: ground-truth
  Q/A per doc, measure keyword vs vector instead of intuition)
  -> `tutor/eval.py` + `scripts/eval_search.py`: each bank item must retrieve its
  topic note in top-k; `python -m tutor.cli eval-search` prints hit@k + misses.
- mem0ai/mem0 (README: ADD-only extraction, entity linking, multi-signal
  semantic+BM25+entity fusion, temporal ranking)
  -> `tutor/memory.py`: append-only facts with domain/topic tags, recall scores
  keyword + tag overlap + recency, `summarize()` feeds the agent loop.
- langchain-ai/langgraph (libs/langgraph: stateful durable agents, persistence,
  human-in-loop)
  -> `tutor/agents.py`: explicit nodes explain->quiz->grade->review->next,
  persisted via `tutor/store.py`, human input stays in CLI/API boundary.
- pydantic/pydantic-ai (typed results, validation over free-form output)
  -> `tutor/items.py`: `validate_item`/`validate_bank` (required fields,
  domain/difficulty, answer-in-choices); bank stays valid without new deps.
- openai/openai-cookbook: clone checkout failed on Windows (bad path
  `examples/data/hotel_invoices/extracted_invoice_json<space>/...`, trailing
  space). Skipped clone; grading pattern (rubric + misconception feedback)
  already present in `question_bank.explanation/misconception` + `agents.grade`.

Unison surface: `tutor/api.py` (stdlib HTTP JSON on 127.0.0.1:8797) is the single
contract. `ui-rust/` and `repos/open-webui-plugin/` both speak to it; CLI stays
canonical (`agent`, `serve`, `eval-search` added alongside quiz/review).

## Reuse audit 2026-09-30 (V1.1 handoff, P0)

Rule: `tutor/` Python stays stdlib-only, so PyPI equivalents are excluded there;
external code enters only through new Rust components, as crate dependencies
(nothing vendored). Licenses checked via the GitHub and crates.io APIs.

Adopted (dependencies of `search-rs/`, crate `tutor-search`):
- Michael-JB/bm25 (crate `bm25` 2.3.2, MIT): BM25 scorer (k1=1.2, b=0.75) and
  its default tokenizer (Snowball English stemming, NLTK stopwords, unicode
  normalization). Replaced the substring-count scoring in `tutor/corpus.py`
  (`search`, `_files`, `_paper_hits`, deleted). Our code adds corpus loading,
  path tie-break (the crate's tie order is random), snippets.
- serde-rs/json (crate `serde_json` 1, MIT OR Apache-2.0): stdout JSON and
  `papers_index.json` parsing.
Measured (hit@3, seed 7): train 35/42 -> 39/42, test 14/18 -> 15/18, all
49/60 -> 54/60. Not significant on 60 items (exact McNemar p~0.27); kept because
both splits improved, no parameter was fit to train, and the two train losses
(eng-07, sci-01) were substring artifacts ("is" inside other words), not content matches.

Kept (per module):
| Module | Equivalent considered (license) | Why kept |
|---|---|---|
| `tutor/spaced.py` SM-2 | open-spaced-repetition/fsrs-rs (BSD-3), py-fsrs (MIT) | FSRS needs real review logs to fit; `data/progress.json` was reset. Revisit with real history. |
| `tutor/eval.py` | AmenRa/ranx (MIT, PyPI) | hit@k is one line; stdlib rule. |
| `tutor/memory.py` | mem0ai/mem0 (Apache-2.0) | needs LLM + vector store; 50-line distillate suffices. |
| `tutor/agents.py` | langchain-ai/langgraph (MIT) | 60-line state machine vs heavy dependency. |
| `tutor/items.py` | pydantic/pydantic-ai (MIT) | stdlib rule; 40 lines. |
| `tutor/api.py` | tokio-rs/axum (MIT), tiny-http (MIT/Apache-2.0) | Rust successor only when earned; server-side grading is a small Python fix. |
| `store`, `quiz`, `curriculum`, `question_bank`, `cli`, `scripts/*` | none better than stdlib json/argparse/random | content or thin glue. |
| `ui-rust/` | ratatui/ratatui (MIT) | 120-line std-only menu; builds and runs (verified 2026-09-30). |

Excluded: khoj-ai/khoj (AGPL-3.0, copyleft); open-webui (custom license, link
only, never vendor); DataTalksClub/llm-zoomcamp (no license: patterns only,
nothing copied).

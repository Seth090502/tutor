# Implementation inventory, 2026-09-30 (Phase 0)

Checkout: `custom` at d48dce1f, clean tree. Read-only code map by a Sonnet 5.5 worker, reviewed by the lead
(Opus 5.5), who also read `deeptutor/services/practice/*`, `routers/practice.py` and `voice/tools.ts`
directly. Status: implemented / partial / missing / docs-only (a document claims it and the code lacks it).
File references are relative to the repository root.

## A. Practice

| Element | Status | Evidence | Gap |
|---|---|---|---|
| Routes at `/api/question-notebook/practice` | implemented | `routers/practice.py`: summary, queue, analytics, questions/{id}, questions/{id}/check, review, attempts, attempts/{id}/confidence, hint, check, import | Queue has no domain or language filter; returns full entries (key, hints, explanation) to the client |
| Tables | implemented | `practice_review_state` (+ `fsrs_*`), `practice_review_events` (request_id PK), `practice_attempts`, `practice_imports` (`services/practice/storage.py:58-150`); items in `notebook_entries` | Upstream `assessment_attempts` is a separate log |
| Attempt flow | partial | open, confidence, hint, check (check seals the answer and closes), review (`storage.py:278-497`) | No separate commit step, no confidence-after-commit rule, no remediation, no closure reasons, no event log, no command idempotency on open/hint/check |
| Rating policy | implemented, conflicts with mandate | `practice/attempts.py:16-21`: rung 1 correct = Good, rung 2-3 correct = Hard | Hinted success earns positive credit (EV-002, EV-014) |
| Client-rated reviews | implemented, conflicts with mandate | `ReviewRequest` accepts `rating` + `self_report` without an attempt; `judged_correct` from the client or the voice model decides free-response ratings | Untrusted ratings enter FSRS with no provenance label |
| Stateless answer check | implemented, conflicts with mandate | `POST /questions/{id}/check` grades any answer with no attempt | Answer oracle before an attempt |
| Single FSRS writer | partial | Only `PracticeStore.review` writes `fsrs_*` | Triggers `practice_capture_insert/update` and `commit_import` reset `due_at`/`interval_days` outside FSRS |
| Mastery | partial | `deeptutor/learning/mastery.py:24` recency-weighted accuracy (upstream Mastery Path) with its own interval scheduler `learning/scheduler.py` | Second scheduler and mastery authority; ignores assistance; Beta-posterior mastery absent |
| Variant families | missing | none | PEDAGOGY "family = one FSRS card" not representable (and dropped per mandate) |
| Other writers into `notebook_entries` | implemented | `learning/assessment.py:record_assessment` from Book focus-check, immersive reading, Mastery Path tools | Wrong answers there create practice review rows through the triggers |

## B. Library and retrieval

| Element | Status | Evidence | Gap |
|---|---|---|---|
| Document metadata | partial | `crates/rag-engine/src/store.rs:17-43` (doc_id = BLAKE3, path, kind, title, domain, source_type, pages, indexed_at) | `indexed_at` rewritten on re-index; domain/source_type only from folder |
| Edition, licence, rights | missing | no hits | No per-purpose rights; retrieved text reaches the lesson LLM ungated |
| Parse records | missing | only `documents.kind`, `ocr_queue.provider` | No parser/chunker version |
| Chunking | partial | `chunk.rs`: 400 tokens, overlap 40, heading paths, page ranges | Equations/tables/proofs not kept whole; no boxes or offsets stored; BM25 English stemmer for all languages |
| Chunk ids and gold survival | partial | `<doc_id>:<ordinal>` | Positional ordinals renumber silently on rechunk/OCR |
| Retrieval result fields | partial | `SearchResponse{mode_used, hits}` | No requested mode, degradation reason, index/corpus version or latency |
| Citation verification | partial | `library/verify.py`: normalized substring + page range | No semantic support, number or derivation check; voice search returns no chunk ids |
| OCR / embeddings | partial | voyage/openai-compatible HTTP embedders off by default; OCR trait only, any provider rejected | No local adapter; no cloud fallback (good) |
| Ingest paths | partial | scoped roots canonicalized | `POST /survey {root}` accepts any directory |
| Ingest limits, cancellation | missing | none | Huge or hung file blocks the job and the PDFium lock |
| Interrupted ingest | implemented | half-committed documents redone | - |
| Moved/changed files | partial | moves handled; change = size + mtime | Same-size same-mtime edits missed |
| Schema upgrade | partial | BM25 recreated empty on schema change | Empty until next ingest; no SQLite migrations |

## C. Guided reading (C11)

| Element | Status | Gap |
|---|---|---|
| Routes and lifecycle | implemented (6 routes) | `GET /sessions/{id}` and `/answers` (C11 amendments) missing; no resume |
| Annotations | missing | Reader deliberately has none |
| Recall/summary scoring | implemented | Both passes same model |
| Prequestion checks | partial | Explanations never reach the UI |
| Proposed items | partial | Atoms only; `provisional` is a tag with no queue/FSRS exclusion, so provisional cards are due immediately |
| Paper mode | partial | Frontend only |

## D. Items and checkers

| Element | Status | Gap |
|---|---|---|
| Item schema | partial | No version history, status lifecycle, family, skills, checker spec, rubric, structured provenance or eval role |
| Auto-checking | partial | Only choice items and language cards; everything else self-report |
| Numeric/symbolic/unit/chemistry checkers | missing | No SymPy, Pint or RDKit |
| Blind solver, quarantine | missing | - |

## E. Evaluation

| Element | Status | Gap |
|---|---|---|
| Split | implemented | By document group (union-find), seed 0; not by edition/near-duplicate or family |
| Test-split reuse control | missing | No access log or audit set; but **no run has ever been made** (no `evals/runs/`, no datasets), so the test split is still untouched |
| Compare statistics | partial | Paired bootstrap unclustered; no pre-registered metric or margin |
| Run file (C2) | implemented | No corpus/index/engine version, dataset hash or seeds recorded |
| Tutor-behaviour and verifier evals | missing | - |

## F. Cost and usage

| Element | Status | Gap |
|---|---|---|
| LLM usage ledger | implemented | `services/llm/usage_ledger.py` (`usage.sqlite3`); standalone `llm.complete` calls unattributed |
| Price tables | partial | Stale table (gpt-4o, claude-3); Claude 5.x and Muse priced as gpt-4o-mini |
| Budget/reservation/ceiling | missing | - |
| Voice/TTS/STT cost | missing | Not counted |

## G. Voice and languages

| Element | Status | Gap |
|---|---|---|
| Voice state | partial | Display mapping only; no confirmation or reconnect states |
| Transcript confirmation | missing | The voice model's rendering is graded directly |
| TTS cache | partial | Key lacks output format/model revision; no eviction, integrity check or tmp cleanup |
| Normalization | partial | NFKC/accents; no script variant rules |
| CJK segmentation | partial | Backend jieba/fugashi (optional extra); rag-engine not segmented |
| Pronunciation scoring | not built | Consistent with docs except LANGUAGES.md Azure claim |
| Language logs | partial | WebView localStorage only |
| Voice privacy config | partial | Tutor agent config lacks `record_voice:false`/retention |

## H. N-back

Storage and per-trial logging implemented (`nback.rs`). Transfer probe n is `trainedLevel-1`, so it changes
between checks. No schema versioning.

## I. Progress and queue

No daily caps, new-item caps, global budget, per-language accounting or next-session plan. New items are due
at creation. Voice picks the first due item when the domain filter misses; the Languages tab filters the top
100 global items client-side, so language cards starve.

## J. Stale owner decisions

See PLAN.md "Owner decisions" after the 2026-09-30 reconciliation.

## Docs versus code

1. C11 resume/answers routes, paper mode, provisional exclusion, `X-Quote-Matches`, startup re-index: docs-only.
2. C1 optional OCR provider: rejected at startup.
3. CORPUS.md Citations API and PEDAGOGY number/equation check: not built.
4. PEDAGOGY Beta mastery, families, FSRS personalization, daily caps, checkers, confirmed voice answers,
   90-minute stop, tutor-behaviour evals, question schema: not built.
5. LANGUAGES.md cache location, Azure scoring, placement, CEFR bands, retests, audio-unverified gate: not built;
   LANGUAGES.md first-candidates list conflicts with the owner's choice.
6. PLAN.md "chunker single-threaded" and "Running" lists: stale.
7. NBACK.md "fixed n" versus per-session `level-1`.

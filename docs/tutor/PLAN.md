# Fork plan: custom DeepTutor tutor

Base: HKUDS/DeepTutor v1.6.12 (`ef2d9e5`, Apache-2.0). Remote `upstream`, work branch `custom`.
Home: `C:\Users\sethy\Projects\tutor` (outside OneDrive on purpose: node_modules, indexes, corpus).
Layout map: `TUTOR.md`. Start with `HANDOFF.md`. Since 2026-10-01 this is one private monorepo with the vault
(`vault/`): https://github.com/Seth090502/tutor, `custom` pushed as `main`; the owner allows pushing freely (2026-10-01).

| Document | What it covers |
|---|---|
| `CONTRACTS.md` | Interfaces C1-C12 |
| `tasks.json` | Task and gate register: dependencies, owner, status, evidence, acceptance |
| `evidence-ledger.json` | Claims the design relies on, with what was actually read |
| `QA.md` | Commands run, results, artifacts, accepted baseline failures |
| `qa/` | Phase reports (inventory, boundary review) |
| `PEDAGOGY.md`, `NBACK.md`, `DESIGN.md`, `LANGUAGES.md`, `CORPUS.md` | Specs |
| `research/` | Research syntheses (leads, not verification) |

## Goals (owner)

- A personal desktop tutor that measurably improves independent learning, with its own GUI, ElevenLabs
  voice, and deep work in mathematics, advanced first-language English, science (physics, biology, earth
  and space, neuroscience and cognitive science), chemistry, and Spanish, French, German, Japanese and
  Mandarin. These are subject destinations, not validated doctoral curricula or certificates.
- Order of optimisation (build mandate, 2026-09-30): trustworthy gains in independent learning, then
  correctness and usability, then commercial readiness. One complete, inspectable learning workflow comes
  before breadth.
- Honest claims only: no IQ, brain-age, guaranteed-outcome, certified-CEFR or PhD-readiness claims. Code
  completion and measured educational benefit are separate release states.

## Hard constraints

- **GPU:** no local models, and the RTX 5090 is off-limits (Cinference owns it). The UI adds no WebGL
  besides the Orb.
- **Disk:** C: 141 GB and D: 61 GB free, so a 250 GB corpus plus indexes does not fit today.
- **Secrets:** Windows Credential Manager through the Tauri shell, handed to services as environment
  variables. Never in chat, git, logs, URLs, prompts or webview state.
- **Corpus:** indexing stays local until the second desktop exists; no cloud OCR or embeddings for the
  library. Selected retrieved passages may reach authorised lesson LLMs. Bulk whole-book runs wait for a
  spend ceiling.
- **Spend:** no new paid usage without the owner's key and approval; unknown usage is reported as unknown.

## Implementation state (2026-09-30)

Status words follow `tasks.json`. The detailed code map is `qa/2026-09-30-inventory.md`.

**Built and merged on `custom`:**
- rag-engine (C1, C9): incremental ingest, BM25, optional vectors, remote mode, library browsing, parallel
  ingest, page crops. No OCR provider; no rights or parse records yet.
- DeepTutor adapter and library use (C11): search with rewrites, answer-leak guard, quote verification
  (text provenance only), guided reading (resume and paper mode not yet in the backend).
- Practice (C7): FSRS (py-fsrs 6), attempts, hint ladder, confidence, calibration and assistance analytics.
- Desktop: Tauri shell, keyring secrets, CSP, the Tutor (voice), Library, Read, Languages, Evals, Progress,
  Train (Dual N-Back, C6), Plan and Settings screens, design kits.
- Languages (C10): speech commands, cards with accent diagnostic, coverage, graded texts, starter decks,
  word glosses.
- Evals: retrieval harness (split by document, paired bootstrap). No dataset or run exists yet.

**Built on `wt/loop`, not yet merged (Phase 1):**
- C12 study sessions (`/api/study`): command-driven attempts with an append-only event log, sealed first
  response, confidence after commit, remediation, explicit closure, review policy 2, review provenance,
  item quarantine, public queue. 24 new tests; C7 tests updated to policy 2.
- `lib/study.ts` client; the library leak guard understands C12 attempts.

**In progress:** Phase 0 baseline and licence inventory; boundary fixes (`wt/harden`); Tutor screen and
Languages practice on C12 (`wt/voice-loop`, `wt/lang-loop`).

## Phases (build mandate, 2026-09-30)

0. Baseline and reconciliation. 1. Authoritative learning loop. 2. Learner model, budget and measurement.
3. Evidence-first library and guided reading. 4. Verified item pipeline and the first curriculum.
5. Subject workspaces and language depth. 6. Voice reliability and cost control. 7. Usability, n-back
containment and release hardening. 8. Independent review and final gate. Tasks, dependencies and gates:
`tasks.json`.

## Decisions (2026-09-30)

- **D1. Owner decisions stand** (not reopened): local indexing; retrieved passages may reach authorised lesson
  LLMs; a second desktop for local embeddings and OCR; private use of CC BY-SA, GFDL and CC BY-NC-SA
  material subject to their terms; OpenStax excluded; United States (public domain before 1931); languages
  Spanish, French, German, Japanese, Mandarin. English is a first-language track, not ESL. All five
  languages stay available; they are not five simultaneous daily courses (one global budget, Phase 2).
- **D2. One scheduling authority per material type.** Atoms use FSRS (existing Python owner, defaults,
  retention 0.90 with the 0.95 cap). Procedures and proofs use skill practice queues with fresh variants
  (C13, Phase 2); there is no family-level FSRS. This supersedes PEDAGOGY's "variant family = one FSRS
  card". Upstream Mastery Path (`deeptutor/learning/`) keeps its own scheduler for its own surface; its
  events are not verified evidence and do not feed C13.
- **D3. Confidence timing:** after the first response is sealed and before any verdict; a skip needs a
  reason (C12).
- **D4. Attempt closure:** `correct`, `revealed`, `self_assessed`, `abandoned`, `invalidated`
  (`self_assessed` extends the mandate's list so self-judged free response is never labelled correct).
  Abandoning releases nothing; leaving after hints records an assisted failure.
- **D5. Review policy 2:** only a verified, unassisted correct first response earns Good; everything else
  is Again; Hard is never produced (EV-002, EV-014). A local default, not a validated rule.
- **D6. Where the loop lives:** Python, next to the FSRS writer, so events, attempt state and the review
  commit in one transaction (C12 explains). Rust takes the projections (C13).
- **D7. Held-out data:** no retrieval dataset or run exists, so the current test split is untouched. When
  evals resume, the split used by `compare` each round is renamed **dev**, and a separate frozen **audit**
  set is created and opened only at declared release checkpoints (Cawley and Talbot, EV-005).
- **D8. Parsing:** local only (PDFium) now. The second-host adapter is disabled by default; no silent cloud
  fallback exists or will be added.
- **D9. Citation semantics:** an exact span match proves text provenance only. Semantic support, changed
  signs, units, negation and derived results need their own checks (C14, Phase 3); until then the UI says
  "quoted from" and never "verified".
- **D10. Routes:** study sessions live at `/api/study`; upstream retired `/api/learning`.
- **D11. Rollback safety:** review provenance is stored in `outcome_json`, not a new column, so an older
  build still runs against a migrated database.
- **D12. Decisions the owner delegated (2026-09-30):**
  - The literal key in `data/user/settings/model_catalog.json` is left untouched: moving it would mean reading
    and copying a secret value, which the machine rules keep with the owner; key-to-host binding (C3) now stops
    it being sent to another server. If it is real, re-enter it in Settings so it lives in Credential Manager.
  - `sandbox_allow_subprocess` is now off in `data/user/settings/system.json` (backup:
    `system.json.bak-2026-09-30`), so text in imported books cannot make an agent run code as the owner.
  - The live ElevenLabs smoke test waits for the key; then one short call (a couple of minutes).
- **D13. RAG direction (sweep of 2026-09-30, `research/rag-sota/`):** tantivy stays the lexical engine with
  per-language analyzers; disk, not CPU, is the binding constraint; pages are routed (text layer first, OCR or
  vision models only where needed, on the GPU desktop); whole-library dense embedding waits for the GPU desktop;
  whole-library graph extraction is rejected on cost and evidence. The deep dive and adoption plan follow.

## Pending owner decisions

1. **Keys:** ElevenLabs, OpenCode, Anthropic (and Voyage only if cloud embeddings are ever wanted outside
   the library), entered in Settings.
2. **Voice model:** the voice agent's LLM is `claude-opus-4-8` at low reasoning, for latency;
   `claude-opus-5-5` is stronger but slower.
3. **Muse tier:** Go contributor (trains on prompts, cheaper) or Zen standard (default).
4. **Spend ceiling** for bulk corpus runs, generation and judging.
5. **Library location** on disk, and whether restricted-copy PDFs you own may be indexed.
6. **Target programmes** per subject (which qualifying-exam formats to use as labelled templates).
7. **Second desktop hardware**, and vector precision (float32 about 41 GB per 10M vectors; float16 or int8
   less).
8. Resolved by delegation: see D12.

## Known issues

- `lancedb` is pinned to 0.37.1 (0.38+ need the `remote` feature); the build needs `protoc`, vendored
  under `crates/rag-engine/vendor/protoc` (gitignored).
- No OCR provider, so scanned and maths-heavy pages stay queued.
- After a BM25 schema change the index is empty until the next ingest (it does not re-index at startup).
- The Orb's texture loads from ElevenLabs' CDN; the CSP covers ElevenLabs' US region only.
- Price tables are stale (Claude 5.x and Muse priced as gpt-4o-mini); there is no budget or reservation.
- The Settings screen can't pick `xhigh` for OpenCode; `max` on the Go contributor tier would be rejected.
- DeepTutor caps some requests at 4,096 max tokens, which may be tight for Opus 5.5 at high effort.
- `tutor_library` has no preflight entry: `deeptutor doctor` reports that preflight could not run.
- Claude Code subagent config defaults to `bypassPermissions` (`services/subagent/config.py:54`); leave it
  unconfigured.
- Windows test failures: see `QA.md` for the named baseline list.

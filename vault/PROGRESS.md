# Vault build: progress checklist

Read `HANDOFF.md` first, then this checklist. Update both as work lands; commit them with each phase.

- **Deployment stage:** Stage 1 (local; vault SSD in the main PC). Transfer planned for Monday 2026-10-05
  (exact time not confirmed yet).
- **Current phase:** Phase 1 started 2026-10-01 on the owner's go-ahead, in parallel with the Phase 0 items still open
  (GPU model matrix, vault SSD). Wave 1 is merged; wave 2 waits for the owner's go. Defaults: originals copied (not
  moved), GROBID waits for hosting, GPU extraction waits for owner windows. Stand-in vault: `E:\Stand-In Vault`.
- **Open owner question:** code scope (HANDOFF.md section 3).
- **Scope additions (owner, 2026-10-01):** source code folders as a first-class delivery type; any agent (not only the
  tutor) can reference chosen material through collections; everything state of the art and in Rust.
- **Repository:** `C:\Users\sethy\Projects\vault` (canonical copy on the main PC), pushed to the private GitHub
  repo https://github.com/Seth090502/vault on 2026-10-01 at the owner's request (push only when asked). Eval
  directory and model files live outside the vault and outside this repository.

## Phase 0: explore, audit, measure, propose (STOP for approval)
- [x] Machine snapshot: drives, GPU, WSL, tools (`docs/phase0/README.md`)
- [ ] Vault SSD located and inspected (stand-in E:\Stand-In Vault measured; SSD arrives 2026-10-01): letter, connection, file system, free space, BitLocker, sequential and random I/O
- [x] Inference engine inventory and tutor connection recommendation (`docs/phase0/engine-inventory.md`)
- [x] Existing code audit (`docs/phase0/existing-code-audit.md`)
- [x] Delivered material characterised; cold-tier projection (`docs/phase0/delivered-material.md`)
- [x] Project status checks, version pins (`docs/phase0/projects-*.md`, `references.md`)
- [ ] Embedding and reranker candidates benchmarked on the 5090 (needs a GPU window). Harrier-OSS-v1 0.6B added
  2026-10-01 (provenance checked, `docs/phase0/model-bench-prep.md`). Cohere Embed 5 considered and declined by the
  owner 2026-10-01: hosted only, and library indexing stays local.
- [x] Vector index candidates A and B benchmarked on synthetic 6M data; projection to the AI PC (`docs/phase0/vector-bench.md`)
- [x] Storage plan and 1 TB budget
- [x] Intake handoff mechanism; pilot composition request
- [x] Pre-transfer schedule
- [x] Disagreements with the design, with evidence
- [ ] Questions to the owner: real tutoring questions, past failures, 5-10 hand-written questions, transfer date and time
- [ ] Gate: owner approves plan, Stage 1 model sizes, vector index, interface, intake mechanism, pilot composition, schedule

## Phase 1: intake, extraction, canonical store, exact access (pilot)
- [x] Stand-in vault laid out; INSTRUCTIONS.md, AGENTS.md, category map (OpenAlex topics + overlays + crosswalks)
- [x] vault-api contract and docs/phase1/SPEC.md
- [x] Wave 1 merged 2026-10-01 (1338d81): vault-core (canon, store, SQLite, jobs, publish, read/outline/find_docs/
  vault_info), libraryd + vault-client (MCP and gRPC, identical results), vault-extract (detection, converters;
  spike picks PDFium for PDF and own EPUB/DOCX/ODT converters, `docs/phase1/extract-spike.md`). 408 tests pass.
- [x] libraryd serves the real vault (blocking-pool adapter, end-to-end test over MCP and gRPC); pins hoisted to
  the workspace; rustfmt 120 columns (dcb5701; 416 tests pass, fmt and clippy clean)
- [ ] Wave 1 follow-ups: run PDF conversion in a child process with a timeout (PDFium is native code); pilot
  and full targets in `CategoryCoverage`; quarantine list/show/retry queries for vault-cli; CJK tokenising for
  `find_docs` (Phase 3); live test against a real Claude Code client; one unreproduced 60 s hang in a vault-client
  restart test
- [x] Code research and repo sweep -> code design (`docs/phase1/CODE-DESIGN.md`; SPEC section 9 summarises it)
- [ ] vault-code increments C1-C7 (CODE-DESIGN section 7; about 26-32 worker-days, estimate)
- [ ] Wave 2: intake pipeline + `vault` CLI, grep with trigram, collections, MCP instructions + skill
- [ ] Per-machine config (hot and cold roots); nothing machine-specific in the vault
- [ ] Intake: format detection, routing, metadata, language, dedup, quality checks, quarantine, intake report page
- [ ] Markdown store with structure maps and sidecars; BLAKE3 integrity manifest
- [ ] vault-core: read, outline, grep (incl. trigram corpus-wide), find_docs (metadata), vault_info; MCP and gRPC; tutor connected; Claude Code config
- [ ] tutor-runtime: headless Claude Code / OpenCode Go session, locked down to libraryd; runtime/model/effort picker in the desktop app
- [ ] TUTOR.md v0 (the tutor's CLAUDE.md), from the owner's working-agreement form, PEDAGOGY.md and the vault tool loop
- [ ] Gate: layer A invariants 100% on the pilot incl. portability; extraction sample reviewed per path

## Phase 2: eval harness and baselines (STOP twice)
- [ ] Layers A-D built; input review page (STOP); sample grades (STOP)
- [ ] Baselines: Phase 1 tools only, and no vault tools; confidence intervals; diagnostics

## Phase 3: lexical search (pilot)
- [ ] tantivy with contextual headers and per-language tokenizers; `search` BM25-only; layer B vs baseline

## Phase 4: dense + fusion + rerank (pilot)
- [ ] Embeddings as data, approved index, RRF, batched rerank, caches, sanity and parity tests
- [ ] Gate: recall@10 up on train and test beyond noise; BRIGHT recorded

## Phase 5: agentic layer, enrichment, pilot tuning
- [ ] workspace, citations, define; system prompt and tool descriptions; enrichment experiments; tuning loop
- [ ] TUTOR.md tuned through the tuning loop (one root-cause rule per round; held-out test decides)
- [ ] Gate: layer D recorded; citation validity >= 95%; frozen pilot settings file

## Transfer phase (planned 2026-10-05)
- [ ] Clean checkpoint, integrity verifier, layer A; NTFS and removal steps (STOP for the move)
- [ ] OpenSSH, deploy to the AI PC, config, firewall/Defender/long-path/share steps
- [ ] Integrity and layer A on the AI PC; quality matches Stage 1; clients switched; Stage 2 placement benchmarked and approved

## Phase 6: full-corpus processing
## Phase 7: full-corpus tuning
## Phase 8: operations

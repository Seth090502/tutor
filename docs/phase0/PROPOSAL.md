# Phase 0 proposal: knowledge vault and retrieval (2026-09-30)

This is the approval gate for Phase 0.

Evidence tags:
- **M**: measured on this PC this session.
- **D**: documented by a source in the linked report.
- **E**: my estimate.

Detail reports are in `docs/phase0/`:
- `README.md`: machine snapshot.
- `engine-inventory.md`
- `existing-code-audit.md`
- `delivered-material.md`
- `projects-search.md`, `projects-platform.md` and `projects-sidecars.md`
- `references.md`
- `vector-bench.md`
- `model-bench-prep.md`
- `design-notes.md`

## 0. What is measured and what is still pending

| Item | State |
|---|---|
| Machine, drives, GPU, tools | Done (M) |
| Inference engine inventory | Done (M/D) |
| Existing code audit | Done (read-only; D) |
| Delivered material | Done (M): **almost nothing delivered yet** (17 real PDFs) |
| Project status and version pins | Done (D, crates.io and GitHub on 2026-09-30) |
| Vector index candidates A and B on synthetic 6M | **Running** (`vector-bench.md`) |
| Embedding and reranker matrix on the 5090 | **Prepared; waits for you to free the GPU** (about 90 min) |
| Vault SSD inspection and I/O | **Waits for the SSD** (the stand-in `E:\Stand-In Vault` is measured: a USB HDD, 70 random IOPS) |

## 1. What I found that shapes the design

1. **The tutor is the Rust/React desktop app** (`C:\Users\sethy\Projects\tutor\apps\desktop`). For its LLM work
   it runs a **headless session of Claude Code or OpenCode Go** (owner, 2026-09-30). The app asks you which
   model and effort to use.
   - Both are installed (M):
     - **Claude Code 2.1.286** has `-p` with stream-json input and output, `--model`,
       `--effort low|medium|high|xhigh|max`, `--tools ""` (no built-in tools), `--mcp-config` with
       `--strict-mcp-config`, `--system-prompt`, `--restricted` and `--no-session-persistence`.
     - **OpenCode 1.18.33** has `run` (`-m opencode-go/<model>`, `--variant <effort>`, `--format json`,
       `--agent`) and `serve`. 29 `opencode-go/*` models are listed, and your default is
       `muse-spark-1.3-contributor` with a `max` variant.
   - Both are MCP clients, and both accept outside tools only over MCP. So the tutor reaches libraryd over MCP;
     see 2.4.
   - The app's current Python backend calls LLM APIs directly (`deeptutor/services/llm`). It does not use
     headless sessions yet.
2. **Your engine (Cinference, `ninfer-serve` in WSL2, Qwen3.8-27B) is a separate project, not the tutor's
   runtime.** It matters here only because, while it runs, it holds 29.8 of 32.6 GB of the 5090 (M) with a
   preallocated KV cache (D). Its integration doc says not to run other GPU work beside it (D). So GPU windows
   (bulk embedding, MinerU, the model matrix) mean times when Cinference is stopped, which you control.
3. **WSL runs in NAT mode (M).** Code inside WSL cannot reach a Windows `127.0.0.1` service. libraryd, the
   desktop app and the headless sessions all run natively on Windows, so this does not affect the vault.
4. **Delivered so far:** 17 real born-digital PDFs (10 MB). Everything else on D: is oa-forge test fixtures (M).
   There are no books, no psychology corpus and no Chinese yet. The cold-tier projection rests on estimates
   until the pilot arrives.
5. **The manifest columns you listed are oa-forge's *input* format, not what it delivers (D).** Delivered
   batches carry `_meta\manifest.jsonl` (`domain, source, category, title, id, version, timestamp, bytes,
   sha256, file, license`, plus optional `authors, year, dedup_of, license_class`) and a `.meta.json` sidecar
   per file (with the URL).
   - `level`, `type` and `mode` are dropped on the way.
   - `license_class` means two different things in input and output.
   - There is no batch-complete marker.
6. **The existing `rag-engine` (in the tutor repo) is not a base** (D): it is a binary-only crate, it stores
   chunk text per SQLite row, its ids are page/offset based, and it has CJK and stemming defects.
   - **Worth porting:** BLAKE3 dedup, Markdown heading parsing, RRF, the job runner, the maths normaliser,
     PDFium guards, and the bearer and bind rules.
7. **Listed projects that changed (D):**
   - **kreuzberg was renamed xberg** (MIT, 1.3.0; it reads EPUB, HTML, DOCX, JATS and `.tex`). On Windows it
     uses a pure-Rust PDF backend; PDFium is not in the Windows build.
   - **MinerU 4.0** (now Apache-2.0 with extra terms, not AGPL) runs natively on Windows through llama.cpp on
     Vulkan. Its CUDA high-throughput path (LMDeploy) has a fresh failure report on an RTX 5090.
   - **GROBID 0.9.1 has no native Windows support** and needs Docker or WSL.
   - **No official Qwen3-Reranker GGUF exists for 4B or 8B.** Only ggml-org's 0.6B Q8_0 is maintainer-confirmed.
     Several community conversions lack the classifier head and score near zero.
   - **OpenAlex is now metered:** a free key gives $1/day; DOI lookups are free; batch filters cost $0.0001 per
     call. It has no ISBN lookup.
   - **lancedb is at 0.39.0 (lance 12)**, and its Rust wrapper still defaults RQ to 1 bit unless set.
   - rmcp 3.5.0 has no built-in server auth (we add an axum middleware) and allows only loopback hosts by
     default.

## 2. Decisions for your approval

### 2.1 Architecture (as briefed, with the changes in section 3)

- **Cargo workspace** in `C:\Users\sethy\Projects\vault`:
  - `vault-core`: all retrieval logic.
  - `libraryd`: MCP over rmcp streamable HTTP, and gRPC over tonic.
  - `vault-client`: the Rust gRPC client.
  - `vault-intake`: intake, extraction routing and the job queue.
  - `vault-eval`: runner, graders and load generator.
  - `tutor-runtime`: the headless Claude Code or OpenCode Go session; see 2.4.
  - `iobench`: already built.
- **Vault layout,** with every path relative to a tier root:
  - **Hot root:** `text/`, `sidecars/`, `db/` (SQLite: meta, jobs, citations, glossary), `index/tantivy/`,
    `index/vectors/` (Lance tables, plus the ANN index), `cache/`.
  - **Cold root:** `originals/`, `quarantine/`.
  - **Inbox:** `inbox/` on the vault drive.
- **Per-machine config:** `%ProgramData%\vault\libraryd.toml` (outside the vault). It holds the hot root, cold
  root, inbox root, bind address, token file, sidecar URLs, model paths and stage.
- **Extraction routing,** best first:

  | Input | Extractor |
  |---|---|
  | JATS | our Rust converter (roxmltree) |
  | arXiv LaTeX | pandoc sidecar, or xberg's `.tex` reader, whichever wins the 20-document sample |
  | arXiv or other HTML | our converter (MathML alttext becomes LaTeX) |
  | EPUB, HTML, DOCX | xberg |
  | Born-digital paper PDF | GROBID (TEI), converted by us |
  | Born-digital book PDF | xberg's native backend, or PDFium directly, chosen by the 20-document sample |
  | Maths-heavy and scanned PDF | MinerU (its OCR covers Chinese) |

  - **Retry ladder:** xberg or GROBID, then MinerU (standard tier), then MinerU OCR, then quarantine with a
    reason.

### 2.2 Stage 1 model sizes: pending the GPU matrix; constraints known now

- **The embedder sets query-time placement for the whole vault life.** Corpus and queries must use the
  identical model file.
  - The tutor itself needs no GPU (its model runs in the cloud through Claude Code or OpenCode Go).
  - In Stage 2, queries will likely run on the 7800 XT (16 GB), because the 5090 is shared with Cinference
    whenever you run it.
  - 8B Q8_0 (about 8 GB) plus a 4B reranker (about 4.3 GB) fits in 16 GB; 8B plus 8B does not (E).
- **The reranker is what threatens the 1 s hybrid target.** Reranking 50-100 passages of about 400 tokens means
  20-40k tokens of prefill per search (E).
  - On the 7800 XT, that is plausible within the target for 0.6B and doubtful for 4B unless rerank depth is
    small (E; measured in Stage 2).
- **Bulk embedding of 2.5B tokens needs Cinference stopped:** about 5 h (0.6B), 15-20 h (4B) or 30-40 h (8B)
  on the 5090 (E; the matrix measures it).
- **Provisional recommendation, to be confirmed or overturned by the matrix:**
  - **Embedder:** Qwen3-Embedding-4B Q8_0. Store full 2560-dimension vectors as data, and index a Matryoshka
    truncation if the matrix shows less than 1 point of nDCG loss.
  - **Reranker:** Qwen3-Reranker-0.6B Q8_0 (ggml-org), with 4B tested at shallow depth.
  - 8B is kept only if it beats 4B by more than the noise.

### 2.3 Vector index: pending the benchmark

I will recommend the option that meets recall@10 and recall@100 of at least 0.95 at the lowest p95 within the
AI PC's 32 GB. The constraint is RAM shared with tantivy's page cache. The decision lands in this section when
the run finishes.

### 2.4 Interfaces and the tutor connection

- **libraryd** exposes the same nine tools over MCP (rmcp, streamable HTTP, bearer token) and gRPC (tonic),
  all batched.
  - Tool names use only `[A-Za-z0-9_-]`.
  - Tool descriptions stay byte-stable, which keeps the model provider's prompt cache warm.
- **The tutor connects over MCP; it has no other way.** Claude Code and OpenCode take outside tools only
  through MCP, so the brief's "default the tutor to native" cannot apply. The native path still matters:
  - **The desktop app's own non-LLM features** (opening a cited passage, a reading view, a search box) link
    `vault-client` into the Tauri shell and call gRPC directly.
  - **The eval's programmatic layers** (A-C) and the load generator use gRPC.
- **MCP versus native:** measured per tool in Phase 1 (local) and Stage 2 (network), and reported. For the
  tutor this is information, not a choice.
- **`tutor-runtime` (Rust, new, in the vault workspace):** one component that starts and drives a headless
  session, used by both the desktop app and the eval runner, so layer D measures exactly what you use.
  - **Inputs:** runtime (Claude Code or OpenCode Go), model, effort, and the tutor system prompt.
  - **The app asks you for runtime, model and effort** when a conversation starts, and remembers your last
    choice.
    - Claude Code models: opus, sonnet, fable, haiku, or full names. Effort: low, medium, high, xhigh, max.
    - OpenCode Go models come from `opencode models opencode-go`, each with its own variants.
  - **The session is locked down so it can reach only libraryd:**
    - **Claude Code:** `--tools ""`, `--strict-mcp-config` with a generated config holding only libraryd,
      our own `--system-prompt`, no user CLAUDE.md, memory, hooks, plugins or skills, an empty working
      directory, and `--no-session-persistence` for eval runs.
    - **OpenCode:** an app-only config with every built-in tool disabled and only `libraryd_*` allowed,
      with none of your global instructions or AGENTS.md.
    - The exact switches for each are being verified now (`docs/phase0/opencode-headless.md` and the Claude
      Code docs check), and are proven by the canary test run *through the session itself*.
  - **No web tools by default,** so every claim has to cite the vault. Please confirm.

### 2.5 Intake handoff

- **Readiness:**
  - For oa-forge batches, a file counts as delivered when its bytes and sha256 match its `manifest.jsonl` row.
    A half-copied file can never pass.
  - A batch is ready when its `_meta\manifest.jsonl` exists and every row's file matches.
  - Batches from other sources: the `_BATCH_COMPLETE` marker. Loose files: 15 minutes with no change, plus an
    exclusive-open check.
- **Reader:** oa-forge's `manifest.jsonl` and the `.meta.json` sidecars are first-class. Your 8-column TSV is
  accepted when present.
  - Unmatched files get metadata from the document itself and OpenAlex, with a recorded confidence.
- **Originals are moved, not copied,** into `originals/{h2}/{blake3}.{ext}`, by rename on the same drive.
  Copying would store every original twice on a 1 TB drive. **This touches originals; please approve.**

### 2.6 Pilot composition: about 7,000 documents

See `design-notes.md` section 7 for the full table.
- Psychology and cognitive science: 3,500.
- Engineering 700, mathematics 700, medicine 600, CS/AI 700, natural sciences 500, humanities 300.
- Every extraction path at least a few hundred times, including 300 Chinese papers, 300 scanned and 300
  maths-heavy.
- 150 books with indexes, 50 code-heavy books, and about 150 deliberately messy files.

### 2.7 Pre-transfer schedule

See `design-notes.md` section 8. The schedule assumes:
- the SSD and Phase 0 approval on Thursday;
- the pilot delivered Thursday and Friday;
- GPU windows on request.

| By | Target |
|---|---|
| Saturday | Phase 1 complete |
| Sunday | Phase 2 harness and baselines, with your two review stops; Phase 3 if time allows |
| Sunday evening | Clean checkpoint: drained, committed, verified, and backed up to E: |

**Honest limit:** Phase 1's gate needs the pilot. If it arrives Saturday or later, Phase 1 finishes after the
transfer and the checkpoint covers what is ingested by then.

### 2.8 Changes to the phases

- Phase 1 gains the 20-document extraction bake-offs per path (xberg against PDFium, pandoc against xberg on
  `.tex`) before routing is frozen.
- Phase 4's model choice is re-validated on real pilot embeddings, because synthetic vectors cannot test
  Matryoshka truncation.

## 3. Where I disagree with the brief, with evidence

1. **Six-hex ids collide.** The brief's example `ab12cd` is 24 bits. At 160k documents that gives about 760
   colliding pairs (E: birthday bound).
   - Use 16 hex characters (about 7e-10 chance of any collision) plus a slug frozen at first publication.
   - The resolver accepts the bare hash.
2. **Text files must be immutable.** `doc_id = hash(normalized text)` already means a re-extraction yields a
   new id. Make it explicit:
   - Never overwrite. A better extraction becomes a new version, linked by lineage.
   - Old addresses keep resolving to exactly the cited text, plus a `superseded_by` pointer.
   - The existing code hashed the source file instead, which would move lines under a stable id (D).
3. **One sentence per line** for prose (semantic line breaks). Citations then point at sentences, not
   2,000-character paragraphs, and Markdown renders the same.
   - A line is defined strictly as LF-separated. U+2028, U+0085, CR, VT and FF are normalised away, because
     Python's `splitlines()` and Rust's `lines()` disagree on them (D: existing-code audit).
4. **The document-level BM25 index should cover metadata and structure, not full text.** That means title,
   authors, abstract, TOC, index terms and venue. BM25 over whole books mostly ranks by length.
5. **Trigram granularity.** Every book contains nearly every common trigram, so document-level prefiltering
   is weak. Test about 16 KiB line-aligned blocks against whole documents in Phase 1.
   - tantivy's n-gram tokenizer records no positions (D), so verification with the ripgrep crates is
     mandatory either way.
6. **"Stage 1 is the best time for GPU work because the drive is local" mostly doesn't hold.**
   - MinerU is GPU-bound at about 1-3 pages/s, which is under 1 MB/s of PDF input (E). Embedding sends text.
     A gigabit LAN covers both (to be measured in Stage 2).
   - The real constraint is that the 5090 hosts the tutor. So I prioritise a verified checkpoint, and run GPU
     jobs in windows: they never start a unit unless enough VRAM is free, and they yield on request
     (`design-notes.md` section 5).
7. **No query-time models on the 5090 while Cinference is running** (its own rule). When it runs, Stage 1 query
   embedding uses a CPU build of the same GGUF, gated by the 0.999 parity test. Phase 4 likely falls after the
   transfer anyway, where the 7800 XT is the natural home.
8. **LaTeX sources need a converter the brief didn't list.**
   - **Proposed:** pandoc as a sidecar process. It keeps maths verbatim. It is GPL, but runs as an external
     tool, not linked.
   - It runs against xberg's `.tex` reader on 20 documents.
   - When both LaTeX source and arXiv HTML arrive, prefer whichever extracts better on the sample.
9. **text-splitter cannot keep a block whole once it exceeds the chunk size** (D: open upstream issues). We
   pre-segment into atomic blocks (code, tables, display maths, proofs) and use text-splitter only inside
   prose.
10. **4B and 8B reranker GGUFs:** convert them ourselves from Qwen's official weights with llama.cpp's
    converter. Verify the classifier head, and run the sanity test before any scoring. Do not use community
    conversions.
11. **Enrichment model.** The brief asks for local generation on the 5090. Cinference serves one request at a
    time, so context sentences for about 500k pilot chunks would take days through it (E).
    - **Proposed:** a small instruct model in llama-server with batched requests, in GPU windows.
    - Measure throughput first, and restrict enrichment to a sample if needed.
    - Sending book text to cloud models for enrichment would also conflict with the earlier "library indexing
      stays local" decision (tutor PLAN D8), so I don't propose using the headless cloud sessions for it.
15. **Unreachable eval answers when the tutor is a local agent CLI.** Claude Code and OpenCode normally have
    file, shell and web tools on the same disk as the eval directory. The guarantee therefore rests on:
    - the session lock-down in 2.4;
    - the eval directory lying outside every vault root;
    - libraryd refusing paths outside its roots;
    - a canary test that asks the *locked-down tutor session itself* to find the canary.
    - **Optional extra layer:** run tutor sessions under a separate Windows account that has no access to the
      eval directory. That is a system change you would make; I won't.
16. **The judge runs through the same runtimes, with a model different from the tutor's.** For example, a
    Claude Code tutor gets an OpenCode Go judge (GPT or GLM family), and an OpenCode Go tutor gets a Claude
    judge.
    - No API key is needed. It uses your subscriptions' quotas instead, so a full layer D run (cases × repeats
      × 2 sessions) draws on those limits. I'll report the run size before each run.
12. **The claude-api skill's `build-eval` and `hillclimb`** expect a Claude-provider app plus node or bun (D).
    I apply their principles manually to our Rust eval.
13. **RAPTOR's controlled gain is about 0.5-2 points (D).** It stays an experiment that has to earn its place.
14. **moka has open crash reports, one with a Windows stack trace (D).** `quick_cache` is the alternative;
    decide in Phase 4 under load.

## 4. Version pins

Checked on 2026-09-30. The final pins go into `Cargo.lock`.

**Rust crates:**

| Crate | Pin |
|---|---|
| tantivy | 0.26.2 |
| Chinese tokenizer | tantivy-jieba 0.20.0 or cang-jie 0.21.0, plus our own bigram field; chosen by eval in Phase 3 |
| lancedb | 0.39.0 (lance 12.0.0; needs protoc) |
| usearch | 2.26.2 |
| grep-searcher / grep-regex | 0.1.17 / 0.1.14, plus regex-syntax |
| xberg | 1.3.0 (MIT; pin exact, because it briefly went to a source-available licence) |
| roxmltree | 0.21.1 (allow_dtd) |
| text-splitter | 0.33.0 |
| tokenizers | 0.23.2 (onig) |
| rmcp | 3.5.0 |
| tonic / prost | 0.14.6 / 0.14.4 |
| axum | 0.8.9 |
| reqwest | 0.13.5 (without default features, to avoid aws-lc and CMake) |
| rusqlite | 0.40.2 (bundled SQLite 3.53.2) |
| moka or quick_cache | decided in Phase 4 |
| blake3, zstd, tokio, rayon | latest |
| gaoya | 0.2.2 (MinHash/LSH, behind an internal trait) |
| whichlang | 0.1.1, plus a script-ratio rule for Chinese |
| criterion | 0.8.2 |

**Tools and sidecars:**

| Tool | Pin |
|---|---|
| Load generator | oha for HTTP and MCP; a tonic client of our own for gRPC |
| llama.cpp | b11309: CUDA 13.4 build (native sm_120) on the 5090; Vulkan build on the 7800 XT |
| GROBID | 0.9.1 (JDK 21) |
| MinerU | 4.0.10 (it changes fast; pinned) |
| pandoc | pinned at install |

## 5. Storage plan and budget

**Vault drive:** 1 TB, about 931 GiB usable. The 85% warning threshold is about 790 GiB in total; each tier
gets its own warning, reported by `vault_info`, the status page and intake.

**Hot tier at full scale (E; replaced by pilot measurements in Phase 1):**

| Item | Size |
|---|---|
| Text | ~11.5 GB |
| Sidecars | ~5 GB |
| SQLite | ~4 GB |
| tantivy chunks | ~6 GB |
| Document-level index | ~0.5 GB |
| Trigram | 3-10 GB |
| Raw vectors, chunk + section + document levels | ~15 GB at 1024-d f16, or ~38 GB at 2560-d |
| ANN index | 1-8 GB |
| Cache | ~2 GB |
| Rebuild headroom | ~10-40 GB |
| **Hot total** | **about 60-130 GB** |

**Cold tier:**
- Papers: 150k × about 2 MB ≈ 300 GB.
- Books: 10k × about 15 MB ≈ 150 GB, plus up to 150 GB more if 10% are large scans.
- **Cold total: about 300-600 GB (E).** It is based on 17 delivered samples, so it is weak until the pilot
  arrives.

**Verdict:**
- The whole vault fits at 360-730 GB, but the high end leaves little room.
- Scanned books are the main risk. The cold root can move to the AI PC's 1 TB HDD with a config change, as
  designed.
- I'll warn you before either tier crosses 85%.

## 6. What I need from you

1. **The vault SSD:** connect it when you have it. I'll inspect it, measure it, and move the stand-in's
   contents over (nothing is written to the stand-in before then).
2. **A GPU window** of about 90 minutes for the model matrix. Tell me when Cinference is stopped.
3. **Scope of the headless runtime.**
   - **Recommendation:**
     - Build `tutor-runtime` and the runtime, model and effort picker as part of this work (Phase 1), for the
       tutoring chat, which is where the vault is used.
     - Move the app's other LLM features (quiz generation, guided reading, voice and the rest, which today
       call LLM APIs from the Python backend) onto the same runtime as a separate track after Phase 2.
   - **Tradeoff:** doing all of it now delays the vault by days. Is that the split you want?
4. **The judge pairing** (section 3, item 16): a model family different from the tutor's, run through the
   other runtime.
   - Which tutor configurations should the eval cover? I'd default to your most likely daily choice plus one
     cheaper one.
   - Do you accept the subscription quota the eval runs will use?
5. **OpenAlex:** a free API key (you create it; I never see the value), or accept keyless rates, which make
   enrichment slower (about 3 days for 160k DOIs).
6. **GROBID hosting:** install Docker Desktop (a system install), or let me set up GROBID with JDK 21 in a
   new, dedicated WSL distro (also a system change). Both need your approval. I won't touch the `Ubuntu`
   distro, which holds Cinference.
7. **The pilot slice,** per 2.6. Also these changes on your side to oa-forge:
   - carry `level`, `type`, `mode` and `url` into `manifest.jsonl`;
   - write `_meta\_BATCH_COMPLETE` at the end of a run;
   - for books, record ISBN, edition and language when known.
8. **For the eval:**
   - real tutoring transcripts or logs, if any exist;
   - questions the tutor got wrong;
   - **5-10 questions you write by hand** of the kinds you actually ask (concepts, landmark studies, exact
     quotes, definitions, comparisons, citation chains, maths and notation, code, navigating a book, Chinese
     sources), plus 2-3 whose answer you know is not in your material.
9. **The transfer:** the exact date and time once you know them. Also the AI PC's Windows edition and network
   link speed (1 or 2.5 GbE).
   - **Optional:** enabling OpenSSH on the AI PC before the move would let me measure the 7800 XT and the
     network early. The Stage 2 decisions would still wait for Stage 2.
10. **Approvals:**
    - this plan;
    - the Stage 1 model sizes (after the matrix);
    - the vector index (after the benchmark);
    - the interface choice (2.4);
    - the intake mechanism, including moving originals (2.5);
    - the pilot composition (2.6);
    - the schedule (2.7);
    - the phase changes (2.8).

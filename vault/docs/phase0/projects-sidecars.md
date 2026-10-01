# Phase 0: sidecar services and extraction stack status

Date of audit: 2026-09-30 (GitHub/PyPI timestamps are UTC, so some read as 2026-10-01).
Scope: llama.cpp, Qwen3 / BGE-M3 models, GROBID, MinerU (+ Marker), kreuzberg (now xberg).
Nothing was downloaded, installed, started or changed. All numbers come from API/README/doc reads done today.

Evidence tags used on every number or claim:
- **[M]** measured: fetched by me today from an API (GitHub, Hugging Face, PyPI, crates.io, Docker Hub). Re-checkable with the URL.
- **[D]** documented by the source (README, docs, model card, issue text). The source's claim, not independently tested.
- **[E]** my estimate or inference. Reasoning given.
- **[U]** unverified or not found. Absence of a hit is not proof of absence.

Raw API responses were saved under `C:\Users\sethy\AppData\Local\Temp\claude\C--Users-sethy-OneDrive-Desktop-Tutor\ba4722e7-2537-4ced-b4a0-4062c34e0fa8\scratchpad\p\` (scratch only, not part of the deliverable).

---

## 0. Headline findings for Stage 1 GPU planning

1. **llama.cpp Windows CUDA 13.4 binary covers the RTX 5090 natively; the CUDA 12.4 binary does not.** Per the build recipe on master, the default arch list adds `120a-real` only when the toolkit is >= 12.8, and the Windows matrix is CUDA 12.4 and 13.4 **[D]** (`.github/workflows/release.yml` L1239-1250; `ggml/src/ggml-cuda/CMakeLists.txt` L40-67). Use the `cuda-13.4-x64` zip for the 5090. The 12.4 build would depend on PTX JIT from `90-virtual` **[E]**.
2. **There are no official Qwen GGUFs for Qwen3-Reranker.** `Qwen/Qwen3-Reranker-{0.6B,4B,8B}-GGUF` do not exist **[M]** (HTTP 401 on the model API; absent from the HF search listing). The only maintainer-blessed one is `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` (0.6B Q8_0 only, 639 MB). Many community conversions score near zero because the classifier head is missing. 4B and 8B need a conversion you trust (Voodisss repos claim to be correct) or one you make yourself with current `convert_hf_to_gguf.py`.
3. **Qwen3-Embedding official GGUFs exist** for all three sizes under `Qwen/*-GGUF` (Apache-2.0). 0.6B has only Q8_0 and f16. Run with `--pooling last`. Instruction format is client-side and the query side needs it.
4. **MinerU is now 4.0 (stable since 2026-09-16) and is a different product from 3.x.** License is no longer AGPL: Apache-2.0 plus extra terms. Windows is supported natively, but the Windows high-throughput engine is LMDeploy, and there is a fresh RTX 5090 failure report on exactly that path (issue #5499, 2026-09-16). The base install runs the VLM through llama.cpp on Vulkan, which also fits the AI PC's RX 7800 XT.
5. **GROBID does not run natively on Windows** (docs say so; Windows-restore issue #1424 is open). Use the Docker image (CRF-only is 510 MB) or a Linux JDK 21 build inside WSL2. Throughput claim is 10.6 PDF/s on a 16-thread CPU box (2019 figure, CRF).
6. **kreuzberg was renamed xberg.** The project is now `xberg-io/xberg` (MIT, v1.3.0 on 2026-09-28). The `kreuzberg` crate stopped at 4.10.4 (LTS). The default PDF backend is pure Rust (no PDFium). The docs say Tesseract OCR needs a system install plus language packs; PaddleOCR is bundled with models downloaded on first use.

---

## 1. llama.cpp (ggml-org/llama.cpp)

### 1.1 Release tags and dates
- GitHub "latest" release is **`v0.5.0`**, published 2026-09-23T20:50Z **[M]** (`https://api.github.com/repos/ggml-org/llama.cpp/releases/latest`). It is a semantic-version release whose only asset is `nightly-tag.txt` (no binaries) **[M]**.
- The binaries live on the rolling `bNNNNN` releases, which GitHub flags as prerelease **[M]** (`/releases?per_page=3`). Newest seen: **`b11309`**, 2026-10-01T01:34Z, with 35 assets; the previous two builds are b11308 and b11307, about 30 minutes apart each. Release pages: `https://github.com/ggml-org/llama.cpp/releases/tag/b11309`.
- Implication [E]: pin to a specific `bNNNNN` tag, not "latest". Pick one that is at or after the merge of PR #28876 (2026-09-27, see 1.4); b11309 qualifies.

### 1.2 Windows release assets in b11309 **[M]** (sizes from asset list)
| Asset | Size |
|---|---|
| `llama-b11309-bin-win-cuda-13.4-x64.zip` + `cudart-llama-bin-win-cuda-13.4-x64.zip` | 145 MB + 403 MB |
| `llama-b11309-bin-win-cuda-12.4-x64.zip` + `cudart-llama-bin-win-cuda-12.4-x64.zip` | 251 MB + 373 MB |
| `llama-b11309-bin-win-vulkan-x64.zip` | 31 MB |
| `llama-b11309-bin-win-rocm-10.0-x64.zip` | 244 MB |
| `llama-b11309-bin-win-sycl-x64.zip`, `...-openvino-2026.4-x64.zip`, `...-cpu-x64.zip` (18 MB), arm64 variants | n/a |

- CUDA versions shipped: **12.4 and 13.4** (x64), 13.4 also for arm64 **[D]** (`release.yml` L1239-1250 matrix; README links L1989-1991). A recent commit "ci: Bump CUDA Windows x64 builds to 13.4.1 (#28930)" is listed in the v0.5.0 notes **[M]**.
- Vulkan Windows build uses Vulkan SDK 1.4.357.0 **[D]** (`release.yml` L1150). This is the asset for the AI PC's RX 7800 XT. A ROCm/HIP Windows build also exists with `gfx1101` (the 7800 XT's target) in its `gpu_targets` list **[D]** (`release.yml` L988), but I did not check whether 7800 XT is a supported ROCm-on-Windows card **[U]**.
- **sm_120 (RTX 5090):** the Windows CUDA job passes `-DGGML_NATIVE=OFF` and no explicit `CMAKE_CUDA_ARCHITECTURES`, so it uses the default list **[D]**. That list in `ggml/src/ggml-cuda/CMakeLists.txt` (master, today) is:
  - always: `75-virtual 80-virtual 86-real`; if CUDA >= 11.8: `89-real 90-virtual`; if CUDA < 13: also `50-virtual 61-virtual 70-virtual`;
  - if CUDA >= 12.8: **`120a-real`**; if CUDA >= 12.9: `121a-real`.
  - So CUDA 13.4 build: native sm_120. CUDA 12.4 build: no sm_120 code, only PTX (`90-virtual`) that the driver would JIT **[E]** (slower first launch, possibly slower kernels).
  - Caveat: this is master's CMake read today; the exact b11309 tag could differ by a commit or two **[U]**.
- NVIDIA driver needed for a CUDA 13.x runtime: I believe R580 or newer **[E, not verified here]**. The cudart zip bundles the runtime DLLs but not the driver.
- Open Blackwell issues on llama.cpp as of today **[M]** (search `sm_120 OR blackwell`, `https://github.com/ggml-org/llama.cpp/issues?q=blackwell`):
  - #27330 (open): CUDA graphs hang on RTX 5090 **Laptop**; workaround `GGML_CUDA_DISABLE_GRAPHS`.
  - #28877 (open): misaligned address in sigmoid op on long prompts, RTX 5090 **Laptop**.
  - #28196 (open): qwen35 on RTX 5090 decodes at 76% of roofline on native Linux; Windows reportedly lower.
  - #28282 (open): illegal memory access on a GLM model at `-ub 2048` (sm_120). Different model, but relevant if you push big `-ub` for embeddings.
  - #28784 (closed 2026-09-22): IQ1/IQ2/IQ3_S garbage output on sm_120, fixed. Avoid IQ quants anyway.
  - None of these names Qwen3-Embedding/Reranker on a desktop 5090 [U].

### 1.3 Embedding endpoint (llama-server)
Source: `tools/server/README.md` on master **[D]** unless noted.
- Flags: `--embedding/--embeddings` (embedding-only mode), `--pooling {none,mean,cls,last,rank}` ("use model default if unspecified"), `--embd-normalize N` (default 2 = Euclidean; -1 none, 0 max-abs, 1 taxicab, >2 p-norm), `-b/--batch-size` (logical, default 2048), `-ub/--ubatch-size` (physical, default 512). Env equivalents: `LLAMA_ARG_POOLING`, `LLAMA_ARG_BATCH`, `LLAMA_ARG_UBATCH`.
- `POST /v1/embeddings` (OpenAI-compatible): requires pooling != none; vectors are **always L2-normalised**. `POST /embedding` (non-OAI) accepts `embd_normalize` per request. `POST /embeddings` (non-OAI): normalised for all poolings except `none`, which returns unnormalised per-token vectors.
- **Qwen3-Embedding settings** (Qwen model card for the GGUF repos): `llama-server -m model.gguf --embedding --pooling last -ub 8192 --verbose-prompt` **[D]** (`https://huggingface.co/Qwen/Qwen3-Embedding-8B-GGUF`, "Usage > llama.cpp").
- **Batch/ubatch caveat:** an issue (#25293, 2026-07-04, closed stale 2026-08-31, not maintainer-confirmed) reports that with embeddings + pooling a single input must fit in one ubatch; otherwise HTTP 500 "input (N tokens) is too large ... increase the physical batch size" **[D, user report]**. So set `-ub` >= the longest chunk in tokens. Chunk size is under your control, so 2048-8192 is enough [E].
- **EOS handling:** Qwen3-Embedding expects `<|endoftext|>` appended to the input. Issues #14234/#14252 (June 2025) show it had to be added manually (`"input": ["text<|endoftext|>"]`); a later change relied on GGUF `add_eos_token` metadata, and the maintainer noted `<|endoftext|>` is not formally EOS in this model **[D]**. Whether the current Qwen GGUFs get it appended automatically is **[U]**. Cheap check at Stage 1: compare a llama-server vector against the Hugging Face reference vector for the same text, with and without a manual `<|endoftext|>`.
- **No server-side Matryoshka truncation:** I found no `dimensions` parameter in the server README **[U: not found]**. Truncate-then-renormalise client-side (see 2.4).
- Pooling for the Hugging Face GGUF API metadata of 0.6B: arch `qwen3`, `context_length` 32768 **[M]** (`https://huggingface.co/api/models/Qwen/Qwen3-Embedding-0.6B-GGUF?expand[]=gguf`); 8B: 40960 **[M]**.

### 1.4 Rerank endpoint and Qwen3-Reranker
- Server flags: `--rerank/--reranking` (enable endpoint), `--pooling rank`; README says reranking needs `--embedding --pooling rank` **[D]**. Endpoints `/reranking`, `/rerank`, `/v1/rerank`, `/v1/reranking`; body `{query, documents[], top_n}` **[D]**.
- Scores are raw model scores; for the Qwen3 yes/no head they come out in [0,1] (e.g. 0.999 / 0.0001 in the maintainer's example). Jina-style rerankers need client normalisation (maintainer comment on #16407) **[D]**.
- **How a Qwen3-Reranker GGUF works:** the official `convert_hf_to_gguf.py` extracts a `cls.output.weight` (yes/no classifier) from `lm_head`, sets `pooling_type = RANK`, bakes in the rerank chat template, sets `classifier.output_labels = ["yes","no"]` **[D]** (`https://huggingface.co/Voodisss/Qwen3-Reranker-4B-GGUF-llama_cpp`, section "What's different"). A plain conversion without those leaves the server nothing to score with.
- **Near-zero score bug (the known issue):** scores like `1e-20`/`4.5e-23` from `/v1/rerank`. Root cause is a GGUF without the classifier head (llama.cpp #16407, closed 2025-10-09, maintainer ggerganov: "your GGUF is not converted properly", and uploaded a correct one) **[D]**. Related: #25447 (2026-07-08, closed stale/not planned) reproduced it with zerank-2 (a Qwen3-based reranker lacking the head); PR #25448 (logit-margin scoring path for headless GGUFs) was **closed without merging** **[M]** (`pull_request.merged_at` null).
- **Which conversions work (evidence):**
  | Repo | Status | Evidence |
  |---|---|---|
  | `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` (639,153,184 bytes, uploaded 2025-10-03, via GGUF-my-repo) | **Works** | maintainer's own posted output in #16407; HF API `gguf` shows `eos_token: <\|im_end\|>` [M] |
  | `Voodisss/Qwen3-Reranker-{0.6B,4B,8B}-GGUF-llama_cpp` (apache-2.0; HF `lastModified` 2026-03-10) | **Reported working** | card shows 0.999966 / 0.000069 on a test pair and a 4B quant benchmark (NDCG@10 F16 0.7003, Q4_K_M 0.7058, Q2_K 0.6691 on AskUbuntuDupQuestions, RTX 3090). Also cited as working by a commenter on #25447. Card's "Converted 2025-03-09" date is internally inconsistent with the repo date [D, treat as typo] |
  | `DevQuasar/Qwen.Qwen3-Reranker-{0.6B,4B,8B}-GGUF` (2025-06-05) | **Broken** | Voodisss card: "confirmed broken with llama.cpp" [D] |
  | `mradermacher/Qwen3-Reranker-{0.6B,4B,8B}-GGUF` (HF modified 2026-08-12 / 08-12 / 04-18) | **Unclear; 8B probably headless** | A user reported "whacky results" with the 0.6B in Nov 2025 (#16407 comment). Size evidence for 8B: their 8B f16 is 16,383.7 MB, same as DevQuasar's broken one, vs 15,141.2 MB for Voodisss; the 1.24 GB difference equals a full `lm_head` (151,669 x 4096 x 2 B) that the working conversion drops [E]. 0.6B and 4B have tied embeddings so sizes cannot discriminate [E] |
  | `prithivMLmods/Qwen3-Reranker-0.6B-seq-cls-GGUF` | Different model (tomaarsen seq-cls conversion), card file table is copy-pasted from another model [M] | not a drop-in for the official model |
  - A direct check is `gguf-dump` for a `cls.output.weight` tensor and `pooling_type=4 (rank)` in the file header. I did not do this (no downloads) **[U]**.
- **Custom instruction:** server-side custom instruct for Qwen3-Reranker is PR #20009, **open, not merged** **[M]**. The baked template uses the default instruction "Given a web search query, retrieve relevant passages that answer the query" **[D]**.
- **Batching constraint, recently relaxed:** PR #28876 "allow RANK pooling batch splitting for causal LLM rerankers (Qwen3 and Qwen3-VL)" was **merged 2026-09-27** **[M]**. Before it, a rerank prompt (query + doc + template) had to fit in one physical batch (default 512 tokens) or it failed. Builds older than that merge keep the old limit. PR #27715 (reranker memory grows roughly O(n^2) with ubatch size) is **open** **[M]**. Do not set `-ub` far beyond what the chunk size needs.
- KV cache quantisation: maintainer advises against `-ctk q4_0 -ctv q4_0` and says q8_0 is sometimes OK but not recommended (#16407) **[D]**. With `-np 16` one user saw rank-order differences at the tail (not reproduced by the maintainer).
- Recommended invocation for a working GGUF **[D]**: `llama-server -m Qwen3-Reranker-4B-f16.gguf --reranking --pooling rank --embedding --port 8081`, then call `/v1/rerank`. The Voodisss card warns `/v1/embeddings` on a reranker returns zeros.

---

## 2. Models: Qwen3-Embedding, Qwen3-Reranker, BGE-M3

Sizes below are Hugging Face `siblings[].size` bytes / 1e6 (MB, decimal) **[M]**, from `https://huggingface.co/api/models/<repo>?blobs=true`, fetched 2026-09-30.

### 2.1 Qwen3-Embedding official GGUF (`Qwen/*-GGUF`, license Apache-2.0 [M cardData])
| Quant | 0.6B | 4B | 8B |
|---|---|---|---|
| Q4_K_M | not offered | 2,496.7 | 4,676.8 |
| Q5_0 | not offered | 2,823.1 | 5,292.0 |
| Q5_K_M | not offered | 2,888.9 | 5,422.3 |
| Q6_K | not offered | 3,305.7 | 6,214.5 |
| Q8_0 | **639.2** | 4,279.7 | 8,047.1 |
| f16 | 1,197.6 | 8,049.9 | 15,141.2 |
Repos: `Qwen/Qwen3-Embedding-0.6B-GGUF` (last modified 2025-07-14), `...-4B-GGUF` (2025-08-03), `...-8B-GGUF` (2025-07-15). The upstream safetensors repos were touched more recently: 0.6B on 2026-04-20; the 4B/8B embedding repos on 2025-06/07 **[M]**. Whether the GGUFs were regenerated after the 0.6B tokenizer fix is **[U]**.

### 2.2 Qwen3-Reranker GGUF
- **`Qwen/Qwen3-Reranker-*-GGUF`: do not exist** **[M]** (`/api/models/Qwen/Qwen3-Reranker-0.6B-GGUF` returns 401; HF search for `Qwen3-Reranker` lists only the safetensors repos under `Qwen/`).
- `ggml-org`: only `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF`, one file `qwen3-reranker-0.6b-q8_0.gguf`, 639.2 MB, license apache-2.0 [M]. `ggml-org` has no 4B/8B reranker repo [M] (`/api/models?author=ggml-org&search=rerank` returns this and `jina-reranker-v1-turbo-en-GGUF` only).
- Sizes in the community repos (all MB, **[M]**):
  | Quant | 0.6B | 4B | 8B (Voodisss) | 8B (DevQuasar / mradermacher) |
  |---|---|---|---|---|
  | Q4_K_M | 396.5 | 2,496.7 | 4,676.8 | 5,026.3 |
  | Q5_K_M | 444.2 | 2,888.9 | 5,422.3 | 5,849.5 |
  | Q6_K | 494.9 | 3,305.7 | 6,214.5 | 6,724.1 |
  | Q8_0 | 639.2 | 4,279.7 | 8,047.1 | 8,707.2 |
  | f16 | 1,197.6 | 8,049.9 | 15,141.2 | 16,383.7 |
  The 8B size gap between the two columns is the missing-head signal discussed in 1.4.
- Upstream safetensors (for self-conversion): `Qwen/Qwen3-Reranker-4B` 8,043.6 MB total, `-8B` 16,377 MB total, license apache-2.0 [M].

### 2.3 BGE-M3
- `ggml-org/bge-m3-Q8_0-GGUF`: one file `bge-m3-q8_0.gguf`, 634.6 MB, license MIT, last modified 2025-10-12; GGUF metadata: arch `bert`, `context_length` 8192, non-causal [M].
- `gpustack/bge-m3-GGUF` (MIT, 2024-10-31): FP16 1,157.7; Q8_0 634.6; Q6_K 499.4; Q5_K_M 467.7; Q5_0 459.3; Q4_K_M 437.8; Q4_0 421.6; Q3_K 402.3; Q2_K 366.1 MB [M]. `KimChen/bge-m3-GGUF` (MIT, 2024-08-29): f32 2,273.7; f16 1,157.7; q8_0 634.6; q6_k 499.4; q4_k_m 437.8 MB [M].
- Model facts from `BAAI/bge-m3` card [D]: dimension 1024, max sequence length 8192, **no query instruction needed**, license MIT; it provides dense, sparse and multi-vector (ColBERT) retrieval in the FlagEmbedding library. The GGUFs are a plain BERT-arch encoder; I found nothing indicating llama.cpp exposes the sparse or ColBERT heads **[U / E: assume dense only]**. Pooling type is stored in the GGUF; BGE-M3 dense uses CLS pooling in the reference implementation **[E]**, so check `--pooling` default when loading.

### 2.4 Embedding dimensions, MRL, instruction format, length, license (Qwen3)
From the model cards (`https://huggingface.co/Qwen/Qwen3-Embedding-8B-GGUF`, `.../Qwen3-Embedding-0.6B`) **[D]** and configs **[M]**:
| | 0.6B | 4B | 8B |
|---|---|---|---|
| Layers | 28 | 36 | 36 |
| Embedding dim (max) | 1024 | 2560 | 4096 |
| User-defined output dim (MRL) | 32 to 1024 | 32 to 2560 | 32 to 4096 |
| Context ("Sequence Length" in card) | 32K | 32K | 32K |
| `max_position_embeddings` in HF config [M] | 32,768 | 40,960 | 40,960 |
| GGUF `context_length` [M] | 32,768 | not read | 40,960 |
| License | Apache-2.0 | Apache-2.0 | Apache-2.0 |
- **MRL: yes** (card table, "MRL Support: Yes" for all three). Implementation is the usual take-first-d-dims then re-normalise; do it client-side since llama-server has no `dimensions` field [E].
- **Query instruction format** (card code, `get_detailed_instruct`) [D]: `f'Instruct: {task_description}\nQuery:{query}'`. Note there is **no space after `Query:`**. Documents get **no** instruction. Card says omitting the instruction on the query side can cost about 1% to 5% retrieval quality, and recommends English instructions even for multilingual use. Default task string in the card example is a one-sentence retrieval instruction, e.g. "Given a web search query, retrieve relevant passages that answer the query".
- Pooling: **last-token pooling** (`last_token_pool` in card code; llama.cpp `--pooling last`), L2-normalise, and the model expects `<|endoftext|>` at the end of each input [D] (see EOS caveat in 1.3).
- The card's reference code uses `max_length = 8192` for tokenisation; the 32K figure is the architectural context [D].
- Qwen3-Reranker input format (card code) [D]: system prompt "Judge whether the Document meets the requirements based on the Query and the Instruct provided. Note that the answer can only be "yes" or "no"." then user `<Instruct>: {instruction}\n<Query>: {query}\n<Document>: {doc}`, then assistant prefix `<think>\n\n</think>\n\n`; score = softmax over the "yes" vs "no" logits at the last position. The card's code uses `max_length = 8192`. In llama.cpp this template is baked into a correctly converted GGUF (1.4).
- Multilingual / Chinese: Qwen3-Embedding cards claim 100+ languages and report C-MTEB means of 66.33 (0.6B), 72.27 (4B), 73.84 (8B); BGE-M3 is 59.56 on MTEB multilingual vs 64.33 / 69.45 / 70.58 for Qwen3 0.6B/4B/8B [D, vendor-reported].
- Qwen3-VL-Embedding / VL-Reranker (2B, 8B) exist as separate Hugging Face models [M]; llama.cpp work for typed/multimodal embeddings is in flight (PR #29556 closed 2026-09-28; #25921 open). Out of scope for this task.

---

## 3. GROBID

- **Repository:** the project now lives at `grobidOrg/grobid` (GitHub API for `kermitt2/grobid` resolved to `grobidOrg/grobid`) **[M]**. License Apache-2.0 **[M]**. Pushed 2026-09-30 **[M]**.
- **Latest release: 0.9.1**, published 2026-08-04T06:56Z **[M]** (`https://github.com/grobidOrg/grobid/releases/tag/0.9.1`). Dev line is `0.9.2-SNAPSHOT` **[D]** (`doc/Install-Grobid.md`). Notable in 0.9.1: pdfalto 0.6.2 (deterministic output, bounded memory on huge PDFs), rootless Docker images, fail-fast when `grobid-home` path contains spaces with Wapiti **[D]** (release notes).
- **Windows:** "Grobid running natively on Windows is not supported. We recommend Windows users to use the docker image" **[D]** (`doc/Frequently-asked-questions.md` L372). README: "We cannot ensure currently support for Windows as we did before (help welcome!)" **[D]**. Issue #1424 "Restore Windows support: native pdfalto build + Java process fixes" is **open** (created 2026-04-12) **[M]**.
- **WSL2 without Docker:** the FAQ warns that users reported problems in WSL and links issue #954 **[D]**. Reading that issue, the reporter ran **Windows** `java.exe` from inside WSL (`java.exe -Xmx4G -jar ...`), and the maintainer's test showed Java reporting `windows` as OS; the reporter then switched to Docker **[D]**. My reading: a Linux JDK installed inside the WSL distro should look like plain Linux to GROBID **[E, not tested]**. Needs JDK 21 and a Gradle build (`./gradlew clean build`), source path with no spaces **[D]**. Deep-learning models additionally need Python 3.10-3.11 + JEP + DeLFT/TensorFlow **[D]**, so WSL without Docker realistically means the **CRF/Wapiti** models only **[E]**.
- **JDK:** OpenJDK 21 or higher for building and running **[D]** (`Install-Grobid.md`, FAQ L5).
- **Docker images** (`grobid/grobid` and equivalent `lfoppiano/grobid`) **[D]** (`doc/Grobid-docker.md`):
  - `{version}-full` (CRF + Deep Learning, "~8GB" per docs; Docker Hub reports 0.9.1-full amd64 at **14,823 MB** **[M]** `https://hub.docker.com/v2/repositories/grobid/grobid/tags`); recommended to run with a GPU; GPU use only on Linux hosts per docs; needs at least 4 GB GPU memory; amd64 only.
  - `{version}-crf` (CRF only, "~500MB"): Docker Hub 0.9.1-crf **510 MB amd64, 506 MB arm64** **[M]**; best runtime and memory; the docs say to prefer it for "fulltext only, for RAG/LLM preprocessing/search".
  - Run: `docker run --rm --init --ulimit core=0 -p 8070:8070 grobid/grobid:0.9.1-crf`; with GPU: add `--gpus all` to the `-full` image **[D]**.
  - Tags `latest-crf` and `latest-full` on `lfoppiano/grobid` were updated today (2026-09-30) **[M]**, i.e. they track the dev branch; use the version-pinned `0.9.1-*` tags.
  - Full image uses DL models by default for bibliographic parsing, reportedly +2 to +4 F1 on reference parsing and +2 to +5 on citation-context identification versus CRF **[D]**.
  - The docs say Docker GPU use is "currently not supported" on Windows/macOS hosts **[D]**; Docker Desktop with WSL2 does NVIDIA passthrough in general, so this may be stale **[E]**. Whether the TensorFlow build in the full image has Blackwell (sm_120) kernels is **[U]**. Treat GROBID DL on the 5090 as unverified; CRF needs no GPU.
- **Memory:** 4 GB RAM per container for full-text (`processFulltextDocument`), 2 GB header-only, 3 GB citations, 6 to 8 GB for heavy parallel batches **[D]** (`Grobid-docker.md` L172). `pdfalto.memoryLimitMb` default 6096 **[D]** (`Configuration.md`).
- **Throughput:** "around 10.6 PDF per second (around 915,000 PDF per day, around 20M pages per day)" for `processFulltextDocument`, one 16-CPU machine, 16 threads, 32 GB RAM, no SSD, node.js client, run for a week; 11.3M PDFs in ~6 days on two such servers **[D]** (README L97; FAQ L129; the numbers come from a 2019 issue comment, so the hardware is old). Tuning advice **[D]**: set `concurrency` in `grobid.yaml` slightly above server thread count (e.g. 16 to 20 for 16 threads); keep client concurrency a bit above that; with DL models running on CPU, halve it.
- **My estimate for the AI PC (5600X, 6C/12T, 32 GB):** roughly 3 to 6 full-text PDFs/s on CRF [E]. I scale the 10.6 PDF/s result down by threads (16 to 12) and nothing else; the newer core partly offsets the older figure. At 4 PDF/s, 150,000 papers is about 10 hours [E]. Books (10k, hundreds of pages each) are a poor fit for GROBID, which targets scholarly articles [E].
- Consolidation (CrossRef / biblio-glutton) adds external lookups and needs 200 to 600 s client timeouts when enabled **[D]** (FAQ L69).

---

## 4. MinerU (opendatalab/MinerU) and Marker

### 4.1 MinerU version, license, packaging
- **Latest: 4.0.10**, released 2026-09-29 (`mineru-4.0.10-released`) **[M]**; PyPI `mineru 4.0.10`, Python `>=3.10,<3.15` **[M]**.
- **4.0 stable was first published 2026-09-16 (PyPI `4.0.0`)**, after alphas from 2026-07-14; **10 patch releases in 13 days** **[M]** (PyPI release dates). It is a rewrite ("MinerU 4.0 brings document parsing, a local document library, and service tools into one workflow"; new native PDF kernel DocVortex; tiers `flash/basic/standard/advanced`; Gradio 6 WebUI; `/v1/*` API; config in `config.yaml`) **[D]** (README, `docs/en/reference/migration_4.md`). Expect churn.
- **License: Apache-2.0 with additional terms. Not AGPL** **[D]** (`LICENSE.md` on master, README "License Information"): (1) a separate commercial license is needed only if you and affiliates exceed 100 million MAU or USD 20 million monthly revenue; (2) providers of online services built on MinerU must state prominently that MinerU is used; (3) the license terminates automatically if either is breached. For a personal tutor this is not a constraint, but term (2) matters if the tutor is ever offered to third parties [E]. Third-party dependency licenses (docvortex, models) were not audited **[U]**.
- **Windows:** supported natively. Base package `mineru>=4.0,<5` = ONNX small models on CPU + **llama.cpp VLM in Vulkan mode** ("good compatibility on the vast majority of devices"); Windows GPU Torch must be installed separately; `mineru[full]` on Windows = Torch + **LMDeploy `>=0.17.0,<0.18`**, on Linux = vLLM `>=0.19.1,<0.29.0` **[D]** (README L119; `docs/en/usage/tiers.md`; `docs/en/quick_start/extension_modules.md`). `mineru-llama-cpp 0.1.2` (in-process llama.cpp VLM engine) ships `win_amd64` wheels of 21 MB **[M]** (PyPI). Docker workflow targets Linux + NVIDIA ("WSL2 on Windows"); default image uses CUDA 13.0 and vLLM 0.21.0, with a cu129 alternative **[D]** (`docs/en/quick_start/docker_deployment.md`). AMD and other vendor accelerators stay on `mineru<4` with legacy guides **[D]**.
- **RTX 50-series (Blackwell, sm_120):** no explicit Blackwell statement in the 4.0 docs **[U]**. Evidence from issues **[M]** (GitHub issue search on `opendatalab/MinerU`):
  - **#5499** (2026-09-16, closed same day): Windows, RTX 5090, 4.0 with LMDeploy/TurboMind: `[TM][FATAL] kernels\gemm\tuner\measurer.cu(83): Check failed: status == cudaSuccess no kernel image is available for execution on the device`. The single reply points to the FAQ "Windows CUDA acceleration" entry (reinstall `mineru[full]`, check Torch CUDA support/driver) **[D]**; no confirmed fix in the thread. This is the same engine and OS the 4.0 docs prescribe for Windows high throughput, so assume it is risky on the 5090 until proven **[E]**.
  - #5121 (closed 2026-06-18): RTX 5060 Ti (sm_120) cannot use the lmdeploy engine (3.x era). #4957 (closed): Windows + 5070 Ti, latest VLM mode failed to start. #4122 / #3582 (closed 2026-05-02): 5090 with vllm engine failed in 2025.
  - Older: #3470 reported 5090 about 25% slower than 4090 on 0.4.10 images.
  - Practical read [E]: on this PC the working options to test are (a) base install (llama.cpp Vulkan VLM + ONNX; no CUDA kernels involved), (b) vLLM inside WSL2 or Docker (CUDA 13 image), (c) llama.cpp CUDA 13.4 for the VLM if MinerU can point at an external server (not checked **[U]**).
- **Open reliability issues that matter for a 150k-document batch** **[M]**: #5313 (open): API process RSS grows without bound across documents until OOM (3.4.4; reporter's workaround is `malloc_trim(0)`); #5472 (open, 2026-09-07): `mineru-api` leaks about `nproc` OS threads per parsed document via OpenMP teams, fixed by setting `OMP_NUM_THREADS`, no default set. Plan worker recycling and `OMP_NUM_THREADS` [E].

### 4.2 VRAM and RAM by backend (documented)
From the README "Model Engines" table **[D]** (`readme_mineru`, L440-460, quoted):
| Tier | Engines | Model download | Min RAM | Accelerator |
|---|---|---|---|---|
| `basic` | ONNX | ~0.8 GB | 2 GB | none (CPU works) |
| `basic` | PyTorch | ~0.8 GB | 8 GB | GPU/MPS recommended |
| `standard`/`advanced` | ONNX + llama.cpp | ~2 GB | 8 GB | CPU works, Vulkan recommended |
| `standard`/`advanced` | PyTorch + llama.cpp | ~2 GB | 16 GB | GPU required, 8 GB+ VRAM |
| `standard`/`advanced` | PyTorch + vLLM/LMDeploy | ~3 GB | 16 GB | GPU required, 8 GB+ VRAM |
- `full` extra needs 16 GB RAM, a Volta-or-newer NVIDIA GPU with 8 GB+ free VRAM, ~5 GB extra disk **[D]**. `tiers.md` says "8 GB is only a planning starting point, not a guarantee". The "pipeline vs VLM" split of 2.x/3.x is replaced by tiers: `basic` = small-model pipeline (OCR/layout/formula/table), `standard`/`advanced` = small models + a VLM; `advanced` = more inference compute, slowest **[D]**. The AI PC (16 GB VRAM) clears the VRAM planning figure with margin [E]; the 5600X box has 32 GB RAM.
- ONNX layout/OCR/formula models run on CPU regardless; only the VLM device is independent; table models can use `onnxruntime-gpu` via `MINERU_TABLE_DEVICE` **[D]**.

### 4.3 Output formats
- `mineru-kit parse`: `markdown`, `middle_json`, `zip`. Self-hosted V1 API: `markdown`, `middle_json`, `structured_content`, `zip`. Python `render()` offers nine targets: Markdown, HTML, LaTeX, DOCX, EPUB, PDF, Structured Content, Content List V1, Content List V2 (per page) **[D]** (`docs/en/reference/output_files.md`).
- Page indices in image names refer to the original document and start at 0; this gives a stable page/block locator for citations. The document library also exposes "stable citation locators" **[D]** (README).
- Whether middle JSON carries LaTeX for formulas and HTML for tables: the 4.0 docs I read show LaTeX as a render target and tables/charts as region images or HTML in visual blocks, but I did not read the middle JSON schema, so the field-level answer is **[U]**. Read `https://opendatalab.github.io/MinerU/reference/output_files/` before wiring ingestion.
- Native (no model) inputs at `flash` tier: PDF text layer, DOC/DOCX, PPT/PPTX, XLS/XLSX, RTF, ODT/ODS/ODP, **EPUB**, OFD, HTML/MHTML, CSV/TSV; plain text is read directly **[D]**. So MinerU alone covers books in EPUB and HTML, at lowest quality tier.
- Chinese: the project's core audience is Chinese-language documents (jieba is a hard dependency; CJK text wrapping in PDF render) **[D, from package metadata and docs]**; I found no per-language accuracy table **[U]**.

### 4.4 Throughput claims
- No first-party throughput number in the 4.0 README or docs I read **[U]**. The MinerU 2.5 technical report is at `https://arxiv.org/abs/2509.22186`; I did not read it.
- Third-party, from Marker's README (a competitor, so treat as marketing) **[D]**: on olmOCR-bench, "MinerU - pipeline (GPU)" scores 72.7 overall and 0.54 pages/s sustained on one B200 host; the same table has Marker balanced 76.0 at 2.9 pg/s, Marker fast 66.6 at 7.4 pg/s, docling 50.3 at 2.1 pg/s, Marker fast without OCR on CPU 43.6 at 23.7 pg/s. A B200 is far faster than either of your GPUs, so absolute numbers do not transfer; only the ranking is of use [E].
- My rough planning figure for MinerU `standard` on the 5090: unknown; measure on 100 representative papers at Stage 1 **[U]**.

### 4.5 Marker (datalab-to/marker)
- **Status:** active. v2.0.0 released 2026-07-20; repo pushed 2026-09-13; 40k stars; PyPI `marker-pdf 2.0.0`, Python `>=3.10,<4`, depends on `torch>=2.7`, `surya-ocr>=0.22.1`, `transformers>=5.12.1` **[M]**. Marker 2 is a rewrite built on Surya OCR 2, a 20M-parameter layout model and a faster pdftext **[D]** (release notes).
- **License:** code **Apache-2.0** [M, GitHub license field]. Model weights: "modified AI Pubs Open Rail-M license (free for research, personal use, and startups under $5M funding/revenue)", with a paid tier above that **[D]** (README, "Commercial usage"). Hugging Face `datalab-to/surya-ocr-2` carries license tag `openrail` **[M]**. Personal use of weights is permitted by this wording; confirm the exact terms in the `LICENSE` file of the weights repo if the tutor ever goes beyond personal use [E].
- **Runtime:** VLM runs in a spawned inference server: vLLM via Docker (NVIDIA GPU) or llama.cpp `llama-server` (CPU/Apple); modes `balanced` (GPU) and `fast` (CPU); `--disable_ocr` runs CPU text-layer extraction only **[D]**. Windows-native support is not stated in the README **[U]**.
- **Throughput (self-reported, one B200 host):** balanced 2.9 pg/s, fast 7.4 pg/s, fast no-OCR CPU 23.7 pg/s **[D]**.
- Output: Markdown and JSON (README tagline) **[D]**.

---

## 5. kreuzberg (now xberg)

- **Renamed.** The GitHub API for `kreuzberg-dev/kreuzberg` returns `full_name: xberg-io/xberg`, homepage `https://docs.xberg.io` **[M]**. The README states "Xberg is the next iteration of Kreuzberg ... Same document-intelligence engine, rebuilt and rebranded under a fresh v1 line"; v4 is kept as `kreuzberg-dev/kreuzberg-v4-lts` **[D]** (`https://github.com/xberg-io/xberg`). This matches the earlier study that saw it called "xberg".
- **Is it a Rust crate:** yes. The core is Rust (workspace edition 2024, `rust-version 1.92`) with 15 language bindings, CLI, REST server and MCP server **[D]** (`Cargo.toml`, README). Crates: `xberg` **1.3.0** (2026-09-29; first 0.0.1 on 2026-06-25) and legacy `kreuzberg` **4.10.4** (2026-09-21, repository `kreuzberg-dev/kreuzberg-lts`) **[M]** (crates.io API). PyPI `xberg 1.3.0` (Python >= 3.10) and `kreuzberg 4.10.4` **[M]**.
- **Latest version:** xberg **v1.3.0**, published 2026-09-29T04:56Z (changelog dated 2026-09-28) **[M]**. Cadence: 1.2.6 (09-20), 1.2.7 (09-22), 1.2.8 (09-23), 1.2.9 (09-24), 1.3.0 (09-28) **[M]** (CHANGELOG headings). Very fast churn.
- **License: MIT** for xberg and for the 4.x line **[M]** (GitHub license field; crates.io; `LICENSE` "Copyright (c) 2025-2026 Kreuzberg, Inc."). Caveat: crates.io shows `kreuzberg` 5.0.0-rc.1 to rc.35 and 4.10.0-rc.9..15 published with license **Elastic-2.0** between 2026-04-29 and 2026-06-24, all marked yanked **[M]**. So the company briefly tried a source-available license and reverted to MIT before the xberg line. Pin to an MIT-tagged release and re-check the license on each upgrade [E].
- **Formats:** "107 formats, 141 file extensions" **[D]**. Relevant ones **[D]** (README format tables): PDF; Word `.docx/.doc/.odt/...`; eBooks `.epub`, `.fb2`; HTML/XHTML `.html/.htm/.xhtml`; scientific `.tex`, `.jats`, `.nxml`; citations `.bib/.ris`; email; archives; images (OCR). Output formats: plain, Markdown, Djot, HTML, JSON tree, DocTags **[D]**.
- **PDF backend and Windows:** default is a **pure-Rust PDF backend** (`xberg-native-pdf`, based on `pdf_oxide`): "no PDFium, no system libraries" **[D]** (`crates/xberg/README.md`; CHANGELOG 1.0.0: "pdfium is gone; pdf_oxide ... is now the sole PDF backend"). Later releases re-added an **opt-in** Pdfium backend; in the published Python/Node/etc. packages it is compiled in but `libpdfium` is **not bundled** (supply it and set `PDFIUM_DYNAMIC_LIB_PATH`), and the **Windows and Intel-mac packages build from a reduced feature set without it** **[D]** (CHANGELOG 1.3.0, GH#1863). So on Windows you get the native backend only; that is the default anyway.
- **Windows packaging:** PyPI wheel `xberg-1.3.0-cp310-abi3-win_amd64.whl`, 46 MB **[M]**; CLI via Scoop on Windows; Elixir/PHP Windows artifacts in the release **[D/M]**. Python README: "There are no optional extras to install. The wheel bundles the feature set for your platform - OCR (including PaddleOCR), layout detection, embeddings, chunking" **[D]**.
- **OCR on Windows (Tesseract):** the documented install is a **system install**: Windows tab says "Download from GitHub releases" (Tesseract), plus separate language packs (`tesseract --list-langs` to verify) **[D]** (`https://docs.xberg.io/guides/ocr/`). Language data resolution order: `OcrConfig.tessdata_path`, then `TESSDATA_PREFIX`, then xberg cache, then system paths **[D]** (CHANGELOG, GH#1857/#1892). Conflicting evidence: the crate `xberg-tesseract` builds and statically links Tesseract and Leptonica from source by default ("No runtime dependencies"), including fixed MSVC static linking **[D]** (`crates/xberg-tesseract/README.md`), so whether the **published wheel** needs a separate Tesseract install is **[U]**: test `pip install xberg` then OCR a scan on a clean Windows profile. Language `.traineddata` files (for example `chi_sim`, `chi_tra` for Chinese papers) still need to be provided in any case [E].
- **OCR without Tesseract:** PaddleOCR bundled via the `paddle-ocr` feature, models download automatically on first use, supports Chinese (`language="ch"` in README example); Candle backends (GLM-OCR, DeepSeek-OCR, PaddleOCR-VL, TrOCR) compiled in by default for Windows packages, weights download from Hugging Face on first use to `~/.cache/huggingface/`; ONNX Runtime must be installed separately on Windows only for the embeddings feature (`scoop install onnxruntime`) **[D]**. "Models download" means network access at first run; the brief's no-download rule applied to this audit only.
- **Throughput:** "10-50x speed improvements" is the PyPI summary for the old `kreuzberg` Python package **[D, marketing]**; benchmark page `https://xberg.io/benchmarks` not read **[U]**.

---

## 6. Open items the lead may want to decide or verify (not decided here)

1. Which `bNNNNN` llama.cpp tag to pin (>= merge of #28876, today's b11309 qualifies) and whether to take CUDA 13.4 (5090 native sm_120) vs Vulkan on both machines for parity with Stage 2.
2. Which Qwen3-Reranker GGUF to trust for 4B/8B (Voodisss vs self-conversion), and a one-time `gguf-dump` check for `cls.output.weight` + `pooling_type=rank` before any scoring run.
3. A one-time numeric check that Qwen3-Embedding via llama-server matches a Hugging Face reference vector, including the `<|endoftext|>` question.
4. MinerU on the 5090: Windows LMDeploy path has a fresh failure report; decide among base/Vulkan install, WSL2/Docker vLLM, or a different extractor for Stage 1. Measure pages/s on a sample.
5. GROBID: Docker Desktop CRF image vs WSL2 JDK 21 build; DL full image on a 5090 unverified.
6. xberg: confirm on a clean Windows profile whether the wheel's Tesseract works without a system install; confirm Chinese OCR path (Tesseract `chi_sim` data vs bundled PaddleOCR).
7. Things I did not read: MinerU 2.5 paper, MinerU middle JSON schema, xberg benchmark page, NVIDIA driver minimum for CUDA 13.4, whether `mineru` can use an external llama.cpp/OpenAI-compatible VLM server.

## 7. Source index (URLs)

- llama.cpp: https://github.com/ggml-org/llama.cpp/releases (b11309, v0.5.0); `.github/workflows/release.yml`; `ggml/src/ggml-cuda/CMakeLists.txt`; `tools/server/README.md`; issues/PRs #16407, #25447, #25448, #28876, #27715, #20009, #25293, #14234, #14252, #27330, #28877, #28196, #28282, #28784.
- Models: https://huggingface.co/Qwen/Qwen3-Embedding-{0.6B,4B,8B}-GGUF; https://huggingface.co/Qwen/Qwen3-Embedding-0.6B; https://huggingface.co/Qwen/Qwen3-Reranker-0.6B; https://huggingface.co/ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF; https://huggingface.co/Voodisss/Qwen3-Reranker-4B-GGUF-llama_cpp; https://huggingface.co/DevQuasar/Qwen.Qwen3-Reranker-4B-GGUF; https://huggingface.co/mradermacher/Qwen3-Reranker-8B-GGUF; https://huggingface.co/BAAI/bge-m3; https://huggingface.co/ggml-org/bge-m3-Q8_0-GGUF; https://huggingface.co/gpustack/bge-m3-GGUF.
- GROBID: https://github.com/grobidOrg/grobid (README, `doc/Grobid-docker.md`, `doc/Install-Grobid.md`, `doc/Frequently-asked-questions.md`, `doc/Configuration.md`), https://hub.docker.com/r/grobid/grobid/tags, issues #954, #1424.
- MinerU: https://github.com/opendatalab/MinerU (README, `LICENSE.md`, `docs/en/usage/tiers.md`, `docs/en/quick_start/*`, `docs/en/reference/output_files.md`, `docs/en/faq/index.md`), https://pypi.org/project/mineru/, issues #5499, #5313, #5472, #5121; Marker: https://github.com/datalab-to/marker (README, release v2.0.0), https://huggingface.co/datalab-to/surya-ocr-2.
- xberg/kreuzberg: https://github.com/xberg-io/xberg (README, CHANGELOG.md, `crates/xberg/README.md`, `crates/xberg-tesseract/README.md`), https://crates.io/crates/xberg, https://crates.io/crates/kreuzberg, https://pypi.org/project/xberg/, https://docs.xberg.io/guides/ocr/.

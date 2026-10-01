# Stage 1 model comparison: preparation (no GPU used)

Prepared 2026-09-30 by a worker for the lead. Nothing here touched the GPU: no CUDA or Vulkan process was started,
`nvidia-smi` was not run, and the engine, Salad, WSL and services were left alone. The CPU smoke test used the CPU
llama.cpp build with 8 threads. Nothing was committed to git.

## What is ready

| Item | Location |
| --- | --- |
| llama.cpp b11310 (CUDA 13.4 + cudart, Vulkan, CPU), unzipped | `C:\Users\sethy\Projects\vault-models\llama.cpp\llama-b11310-{cuda-13.4,vulkan,cpu}` |
| GGUF models (8 files) and the Qwen3 `tokenizer.json` | `C:\Users\sethy\Projects\vault-models\{embedders,rerankers,tokenizer}` |
| Download manifest with verified hashes | `C:\Users\sethy\Projects\vault-models\manifest.json` (and `fetch.log`) |
| BRIGHT raw parquet, JSONL, token stats, licence | `C:\Users\sethy\Projects\vault-eval\external\bright` |
| Seeded GPU-window subset and the CPU smoke subset | `...\bright\subset-v1`, `...\bright\subset-smoke` (each has `subset.json`) |
| Rust harness (own `[workspace]`, built) | `C:\Users\sethy\Projects\vault\bench\model-bench`, binary `C:\Users\sethy\Projects\vault\target\release\model-bench.exe` |
| One-command runner | `C:\Users\sethy\Projects\vault\bench\model-bench\run-gpu-matrix.ps1` |
| Helper scripts | `bench\model-bench\scripts\{fetch_assets.py,prep_bright.py,make_subset.py,gguf_inspect.py}` |
| Smoke-test output | `C:\Users\sethy\Projects\vault\docs\phase0\model-bench\smoke\` |

Total disk used under `vault-models`: 29.3 GB (28.4 GB of verified downloads plus 0.9 GB unzipped). Free on C: before
starting was 216 GB. BRIGHT adds 0.11 GB.

## The command to run once the owner frees the GPU

```
pwsh -NoProfile -File C:\Users\sethy\Projects\vault\bench\model-bench\run-gpu-matrix.ps1
```

`-DryRun` prints the plan and every harness command without starting anything (use `-OutDir` to keep the dry run out of
the real results). `-Only 'rerank-*'` filters configurations. Results go to
`C:\Users\sethy\Projects\vault\docs\phase0\model-bench\results.jsonl`, logs next to it (`runner.log`, one log per run,
server logs, VRAM samples `*-vram.csv`). Behaviour:

- Pre-flight: all files exist, one `nvidia-smi` query (not a loop) requires at least 18,000 MiB free (`-MinFreeMiB`),
  then `llama-server --list-devices` for the CUDA and Vulkan builds and picks the device whose name contains "5090"
  (Vulkan may also list the CPU's integrated GPU). It aborts with a clear message if the GPU is still busy.
- Runs sequentially, cheapest and most decisive first, the 8B runs last. Order: embed 0.6B Q8_0 (autotune; writes the
  first-stage candidates), parity 0.6B Q8_0 CUDA vs Vulkan, embed 0.6B f16, embed BGE-M3, embed Harrier-OSS-v1 0.6B,
  embed 4B, rerank 0.6B, rerank 4B, embed 8B, rerank 8B, parity 0.6B f16.
- Resumable: a configuration with a result line (`status` ok, sanity_failed or parity_failed) is skipped. The 0.6B Q8_0
  embed run is also re-run if `first-stage-candidates.json` is missing.
- Stops on the first server or infrastructure failure (harness exit 2) with the log tail and the log path. A reranker
  sanity failure (exit 3) or a parity failure is recorded and the matrix continues. Each run has a 60-minute watchdog.
- Never leaves a process behind: the harness stops its own server and `nvidia-smi` child (also on error, through `Drop`),
  writes their PIDs to `running.pids`, and the runner stops anything still listed there (only if the process name still
  matches) after every run and in `finally`.
- Prints a compact summary table from `results.jsonl` at the end.

Conservative time estimate for the whole matrix: about 85 minutes (see the budget below). Throughput on the real card is
unknown; the runner is resumable if the window closes early.

## What was downloaded (all SHA-256 verified against the host's published digest)

llama.cpp release **b11310** (published 2026-09-30 22:05 local, `llama-server --version` reports `0.5.0-dev (build 11310,
commit f872b5911)`, built with Clang 20.1.8). GitHub digests come from the release asset `digest` field. The CUDA build is
the 13.4 one because the 12.4 build predates Blackwell (sm_120 needs CUDA 12.8 or newer); `ggml-cuda.dll` contains
`sm_120` and `sm_121` kernels (string scan, not a run). Driver on the machine: 617.14 (from `docs/phase0/README.md`).

| File | Size | SHA-256 |
| --- | ---: | --- |
| `llama-b11310-bin-win-cuda-13.4-x64.zip` | 153 MB | b3560de0bc65f253474330334a3adad083c8892379a2fe96554a52fd1b7176f5 |
| `cudart-llama-bin-win-cuda-13.4-x64.zip` (unzipped next to `llama-server.exe`) | 424 MB | 738f8c251ac22b70c3ae6f83a10cf222725df0395246a2cf58f32bdb85fbe668 |
| `llama-b11310-bin-win-vulkan-x64.zip` | 33 MB | b93a7e765c09ce5151457b71028fae2a61d4887af88d5280faa324f87162b4c1 |
| `llama-b11310-bin-win-cpu-x64.zip` | 19 MB | 7e4f6a5d9dee8dd8048fb3f258a9a6ecb5c917e84de00cf2db34a3d04397ed9c |

Models (Hugging Face LFS oid from the repo tree API; downloads pinned to the repo commit shown):

| File | Repo @ commit | Size | SHA-256 |
| --- | --- | ---: | --- |
| Qwen3-Embedding-0.6B-Q8_0.gguf | Qwen/Qwen3-Embedding-0.6B-GGUF @ 370f27d7550e | 639 MB | 06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439 |
| Qwen3-Embedding-0.6B-f16.gguf | same | 1,198 MB | 421a27e58d165478cc7acb984a688c2aa41404968b0203e7cd743ece44c54340 |
| Qwen3-Embedding-4B-Q8_0.gguf | Qwen/Qwen3-Embedding-4B-GGUF @ f4602530db1d | 4,280 MB | b60ae5ce2dd6a0b77f82cadf21def1f310a3e10cde380ad0081b07a9d416949d |
| Qwen3-Embedding-8B-Q8_0.gguf | Qwen/Qwen3-Embedding-8B-GGUF @ 69d0e58a13e4 | 8,047 MB | d20ddc71e8a5c4344f2343481e242233a997dc5eaff442427a945836c97b4deb |
| bge-m3-q8_0.gguf | ggml-org/bge-m3-Q8_0-GGUF @ 9eba04c5d75b | 635 MB | aa473d51f451a22f0fcf39ba3330c14bed38a385712b1113440f69df4047a173 |
| harrier-oss-v1-0.6b.Q8_0.gguf (added 2026-10-01) | mradermacher/harrier-oss-v1-0.6b-GGUF @ d79decec1ab9 | 639 MB | ba2c9408f82cfdb73aaf70aaea9f125f3fb24c5e76ce0efb469e1530e5ec53e4 |
| qwen3-reranker-0.6b-q8_0.gguf | ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF @ a02f48bb4f05 | 639 MB | 22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48 |
| Qwen3-Reranker-4B.Q8_0.gguf | giladgd/Qwen3-Reranker-4B-GGUF @ 618ca919a196 | 4,280 MB | d060a1bfb805debcd6c2258c70ba21c56ee1ea837bc23b0bc49e09d13115987c |
| Qwen3-Reranker-8B.Q8_0.gguf | giladgd/Qwen3-Reranker-8B-GGUF @ 3398b1b2ee12 | 8,047 MB | 0f062cc2d6b013c2bd708549dcf36a153043323bc37e17b3b993d95fa0a2105d |
| tokenizer.json (Qwen3 vocabulary used for token counts) | Qwen/Qwen3-Embedding-0.6B @ 97b0c614be4d | 11 MB | def76fb086971c7867b829c23a26261e38d9d74e02139253b38aeb9df8b4b50a |

Local file names are `<owner>__<repo>__<file>` (see `manifest.json`). The harness looks the SHA-256 up in the manifest
(it must be verified and the size must match the file), so every result line carries it.

GGUF facts checked by reading the headers (`scripts/gguf_inspect.py`): Qwen3-Embedding has `pooling_type=3` (last) and
`add_eos_token=true`; a server test confirmed that "Hello world" becomes 3 tokens (the last is `<|endoftext|>`) and that
returned vectors have norm 1.0. BGE-M3 is architecture `bert`, `pooling_type=2` (CLS), 8192 context, adds BOS/EOS.

**Harrier-OSS-v1 0.6B (Microsoft, MIT; added 2026-10-01 as the local alternative from the Cohere Embed 5 comparison
chart).** Same architecture as Qwen3-Embedding-0.6B (`qwen3`, 28 layers, 1024 dimensions, last-token pooling, EOS
appended), so it is a drop-in. Microsoft publishes no GGUF, so the provenance of the conversion was checked:

- The conversion names `calculatortamer/harrier-oss-v1-0.6b` as its source. That mirror's `model.safetensors` and
  `tokenizer.json` have the same LFS SHA-256 as `microsoft/harrier-oss-v1-0.6b` @ f9b9dc8d367d (6bb12422... and
  def76fb0...).
- The header matches the official Qwen3-Embedding GGUF: `pooling_type=3`, `add_eos_token=true`, `add_bos_token=false`,
  EOS and padding id 151643, pre-tokenizer `qwen2`.
- The weights were compared with Microsoft's tensors (fetched by byte range, BF16). Dequantised Q8_0 values for
  `token_embd` (256 rows), `blk.0.attn_q`, `blk.13.attn_k` and `blk.27.ffn_down` are within 0.537 of a quantisation step
  everywhere, with a relative RMS error of 0.52-0.56%, and `output_norm` is exact. The same comparison against the
  Qwen3-Embedding-0.6B GGUF gives 23-51% relative error, so the check tells the two models apart.
- Query format from the model card: `Instruct: <task>\nQuery: <q>` (a space after "Query:", unlike Qwen3-Embedding);
  the harness flag is `--query-format harrier`. The card does not claim Matryoshka training, so its 512 and 256
  dimension numbers may be weak by design.
- CPU smoke on `subset-smoke` (2026-10-01): loads, embeds and scores; like every model it saturates that tiny subset
  (nDCG@10 1.0), so this shows only that the path works.

Not added: `perplexity-ai/pplx-embed-v1` (architecture `bidirectional_pplx_qwen3` with custom code; llama.cpp support
for its bidirectional attention is unverified, and a causal conversion would give wrong vectors).

## Reranker conversions and why

- **Qwen publishes no GGUF reranker repos.** `Qwen/Qwen3-Reranker-{0.6B,4B,8B}-GGUF` do not exist (the Hugging Face API
  returns 401 for them); `Qwen` only has the safetensors repos (and the VL rerankers).
- **ggml-org has only the 0.6B**: `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` (converted with GGUF-my-repo). No 4B or 8B.
  Header check: `pooling_type=4` (rank), `classifier.output_labels=['yes','no']`, a `tokenizer.chat_template.rerank`
  entry, and a `cls.output.weight` tensor `[1024, 2]`.
- **llama.cpp's own docs and tests do not use a Qwen3 reranker.** The server README names bge-reranker-v2-m3 and the
  server tests use `jina-reranker-v1-tiny-en`, so there was no "documented" 4B/8B conversion to fall back on.
- **4B and 8B: `giladgd/Qwen3-Reranker-{4B,8B}-GGUF`, Q8_0 (community conversions, flagged for the lead).** They have the same
  structure as the ggml-org file, verified on the downloaded files: `pooling_type=4`, `['yes','no']` labels,
  `tokenizer.chat_template.rerank`, and `cls.output.weight` `[2560, 2]` (4B) and `[4096, 2]` (8B). The rerank template
  is Qwen's, with the `<think>\n\n</think>` assistant prefix.
- **Rejected:** plain conversions of the causal model (checked `mradermacher/Qwen3-Reranker-4B-GGUF`: no
  `cls.output.weight`, no pooling type, no rerank template; Mungert, QuantFactory and DevQuasar are the same kind). In
  llama.cpp these score near zero for everything. A negative control confirmed the failure mode and that the sanity test
  catches it: running the Qwen3-Embedding-0.6B model as a "reranker" gave scores around 1e-6 and the harness exited 3
  (`smoke/negative-control-stdout.txt`).
- **Fallbacks with the same structure, if the sanity test ever fails for giladgd:** `Voodisss/Qwen3-Reranker-{4B,8B}-GGUF-llama_cpp`
  (4B Q8_0 `Qwen3-Reranker-4B.Q8_0.gguf`, oid 102ac400d68f02877c2fdfecf0732872652298448352b6faf28ec7f8dca8c913), `sinjab/Qwen3-Reranker-{4B,8B}-Q8_0-GGUF`.
  Only the header of the Voodisss 4B and the sinjab 4B was inspected; they were not downloaded.
- The reranker prompt comes from the GGUF template, so every reranker sees Qwen's generic instruction ("Given a web
  search query, retrieve relevant passages that answer the query"), not a BRIGHT-specific one. The `/v1/rerank` score is
  a probability in [0, 1].

## BRIGHT data

- Source `xlangai/BRIGHT` at revision `3066d29c9651a576c8aba4832d249807b181ecae`; splits psychology, biology, economics
  (`documents` and `examples` configs). Parquet hashes verified (LFS oid).
- **Licence: CC-BY-4.0**, as declared in the dataset card front matter (`license: cc-by-4.0`); copy of the card in
  `DATASET_CARD.md`, note in `LICENSE.md`. The card does not discuss the licences of the underlying web and Q&A sources,
  so keep the data local (it is outside the vault repository) and cite the paper (arXiv 2407.12883).
- Converted by `scripts/prep_bright.py` to `<split>/documents.jsonl` (`id`, `content`) and `<split>/examples.jsonl`
  (`id`, `query`, `gold_ids`, `excluded_ids`). All gold ids exist in the document sets; no query has excluded ids.

Sizes and Qwen3 token counts (full splits, tokenizer.json from Qwen3-Embedding-0.6B):

| Split | Parquet docs / examples | JSONL docs | Documents | Doc tokens (total, mean, p50, p95, max) | Queries (n, mean tokens, max) | Gold docs (n, mean tokens, p50) |
| --- | --- | ---: | ---: | --- | --- | --- |
| psychology | 11.4 MB / 0.18 MB | 24.5 MB | 52,835 | 6,123,587; 115.9; 39; 346; 83,846 | 101; 143.9; 445 | 688; 443; 273 |
| biology | 11.0 MB / 0.20 MB | 23.2 MB | 57,359 | 4,899,881; 85.4; 101; 150; 6,348 | 103; 113.3; 502 | 372; 638; 372 |
| economics | 11.0 MB / 0.22 MB | 24.1 MB | 50,220 | 5,874,409; 117.0; 36; 546; 10,895 | 103; 171.2; 568 | 800; 489; 423 |

The three splits hold 16.9 million document tokens, so embedding all of them with the 8B model is out of reach for the
window. That is why the subset exists.

## The subset rule and the 90-minute budget

`scripts/make_subset.py` (seed `vault-phase0-1`), per split:

1. Keep **all gold documents** of all queries (688, 372, 800).
2. Add **3,000 negatives**: non-gold ids sorted, then `random.Random("<seed>:<split>:neg").sample(...)`.
3. Truncate documents to **512 Qwen3 tokens** and queries to 512 (cut at the tokenizer's character offset).
4. For the reranker stage use a seeded sample of **20 queries per split** (`rerank_queries` in `subset.json`; 60 queries,
   6,000 pairs per reranker over the fixed 0.6B top-100). Embedding quality uses all 101/103/103 queries.

Result (`subset-v1`): 3,688 + 3,372 + 3,800 = 10,860 documents, 1,350,361 document tokens, 43,714 query tokens (plus a
30-token instruction per query). This **makes the task much easier than full BRIGHT** (3.4k to 3.8k candidates per split
instead of 50k to 57k), so use the numbers for relative comparison only, not against published BRIGHT scores.
Two further caveats: gold documents are much longer than negatives (mean 292/344/362 tokens against about 80), a length
cue the full corpus does not give in the same way; and truncation at 512 tokens cuts the long tail of gold documents
(p90 of gold is 900 to 1,450 tokens).

Budget at a deliberately conservative **25 TFLOPS achieved** (tokens/s = 25e12 / (2 x non-embedding parameters)); the
per-config minutes come from `subset.json` and the rerank pair length assumes 311-token candidate documents (the
candidates skew long) so 593 tokens per pair:

| Config | Assumed tok/s | Tokens | Minutes |
| --- | ---: | ---: | ---: |
| Embed 0.6B Q8_0 | 28,409 | 1,403,285 | 0.8 |
| Embed 0.6B f16 | 28,409 | 1,403,285 | 0.8 |
| Embed BGE-M3 Q8_0 | 41,667 | 1,403,285 | 0.6 |
| Embed Harrier 0.6B Q8_0 (added 2026-10-01) | 28,409 | 1,403,285 | 0.8 |
| Embed 4B Q8_0 | 3,472 | 1,403,285 | 6.7 |
| Embed 8B Q8_0 | 1,799 | 1,403,285 | 13.0 |
| Rerank 0.6B | 28,409 | 3,559,485 | 2.1 |
| Rerank 4B | 3,472 | 3,559,485 | 17.1 |
| Rerank 8B | 1,799 | 3,559,485 | 33.0 |
| Compute subtotal | | | 74.9 |
| Overhead: 17 server starts at 25 s plus autotune and warm-up | | | 10.3 |
| **Total** | | | **85.2** |

Parity (500 chunks, about 60k tokens per backend, two quantisations) is not in the table; it adds about a minute. The
numbers are estimates, not measurements: real throughput should be better than this, and the lead should read the first
two runs' `tokens_per_s` before trusting the rest of the projection.

## The harness

`model-bench embed | rerank | parity` (clap, ureq, serde; pinned crates: anyhow 1.0.104, clap 4.6.7, serde 1.0.229,
serde_json 1.0.151, ureq 3.4.2 without TLS). cargo 1.98.1, target `C:\Users\sethy\Projects\vault\target`.
`cargo build --release` is clean (no warnings) and `cargo test --release` passes 4 tests (nDCG, recall, Matryoshka
renormalisation, cosine search with excluded ids).

- Starts `llama-server` with the given binary, model and flags, polls `/health` until 200 (fails fast if the process
  exits, with the log tail), and kills exactly that process. Embedders: `--embedding --pooling last` (Qwen3) or `cls`
  (BGE-M3). Rerankers: `--reranking --pooling rank`. Common: `-m -c <np x slot_ctx> -np -b -ub -ngl 99 -t 4 --device <5090>
  --no-webui`.
- Server settings used by the runner: small models np 32, 1024 tokens per slot, `-b -ub 8192`; 4B/8B embedders np 16,
  `-ub 4096`; rerankers np 16 (0.6B, `-ub 8192`) or 8 (4B/8B, `-ub 4096`) with 2048 tokens per slot.
- Documents are embedded with no instruction. Queries use `Instruct: <task>\nQuery:<query>` with the task
  "Given a {Biology|Economics|Psychology} post, retrieve relevant passages that help answer the post" (BRIGHT's own
  per-domain wording). BGE-M3 queries are raw.
- Batching: `--autotune` times three (batch, concurrency) pairs on a 256-document sample after a warm-up and uses the best
  by tokens/s; the trials are stored in the result line.
- Exact cosine search over unit vectors, excluded ids removed, top-100 per query. nDCG@10 (binary relevance, pytrec
  convention) and recall@100 per split at the full dimension and at 512 and 256 (first N dims, renormalised), plus the
  macro mean over splits. The 0.6B Q8_0 run writes `first-stage-candidates.json` (top-100 doc ids per query).
- Rerank: runs the sanity test first (two queries, one relevant and two irrelevant passages each; fails if the relevant
  passage does not beat both by at least 0.05 or the top score is below 0.05), then reranks the stored candidates of the
  chosen queries (100 documents per request) and reports first-stage and reranked nDCG@10 on the same queries,
  candidate recall, pairs per second and tokens per second.
- Throughput: server-reported prompt tokens over wall time of the document passes; the result line includes
  `projected_hours_for_2.5B_tokens`.
- VRAM: with `--monitor-vram` the harness starts one long-running
  `nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits -lms 250` child writing to `<config>-<role>-vram.csv`,
  waits 1.2 s for baseline samples, and kills it after the run (the runner passes the flag for GPU runs only). The result
  records baseline, peak, delta and sample count.
- Parity: embeds the same 500 documents (first 500 by FNV-1a hash of `parity:<id>`) through server A and then server B
  (sequentially, each stopped before the next), and reports min, mean, failures and pass for cosine >= 0.999.
- Result lines (one JSON object per line, `kind` embed / rerank / parity, `status` ok / sanity_failed / parity_failed)
  carry `config` (model file, SHA-256, quantisation, llama.cpp build, backend, device, binary, the exact server flags,
  slot settings, batch sizes, dataset subset directory plus `subset.json` contents), `metrics`, `throughput`, `vram` and a
  UTC timestamp. Exit codes: 0 ok, 2 server or infrastructure failure, 3 reranker sanity failure.

## CPU smoke test (CPU build b11310, 8 threads, `subset-smoke`)

`bench\model-bench\smoke-cpu.sh` runs it. The smoke subset is psychology only: 3 queries, their 44 gold documents plus 8
negatives (52 documents, docs capped at 256 tokens, 9,571 tokens). Results are in `docs\phase0\model-bench\smoke\results.jsonl`.
Another heavy benchmark was running, so the CPU timings are noisy.

```
embed   qwen3-embedding-0.6b Q8_0  (--embed-batch 8, autotune)
  autotune: batch 8 / 32 / 128 -> 390 / 391 / 391 tok/s, picked batch 32, concurrency 2
  psychology: embedded 52 docs, 9591 tokens in 24.5s (391 tok/s)
  nDCG@10 full/512/256 = 1.000/1.000/1.000, recall@100 = 1.000      (52 documents: too easy to discriminate)
rerank  qwen3-reranker-0.6b Q8_0 (ggml-org)
  sanity: PASS  scores [0.99729, 0.0000178, 0.0000120] and [0.99875, 0.0000277, 0.0000177] (relevant passage first)
  psychology: reranked 3 queries (156 pairs) in 156.2s; nDCG@10 1.0000 -> 0.9421   (1.0 pairs/s, 397 tok/s)
embed   bge-m3 Q8_0 (cls pooling, raw queries)
  psychology: embedded 52 docs, 9987 tokens in 15.3s (651 tok/s); nDCG@10 full/512/256 = 1.000/1.000/0.979
parity  qwen3-embedding-0.6b Q8_0, CPU build vs the same CPU build, 30 chunks
  pairs 30, min cosine 0.9999999, mean 1.0, failures 0, pass
negative control  Qwen3-Embedding-0.6B run as a reranker
  scores about 1e-6, relevant not ahead of irrelevant -> SANITY FAILED, exit code 3, result line status sanity_failed
```

After the run, no `llama-server` or `nvidia-smi` process was left. What the smoke test proves: the server start and
health logic, both pooling modes, the `/v1/embeddings` and `/v1/rerank` clients, metrics, candidates hand-off, the
sanity test (pass and fail), the parity path, and the result format. It does not say anything about model quality.

## Open issues

1. **Nothing GPU-side has run.** Unverified until the window: CUDA 13.4 and Vulkan builds starting, `--device` selection
   (the runner picks the entry whose name contains "5090"), the memory fit of `np 32` x 1024 and `-ub 8192`, and the VRAM
   monitor. In particular `nvidia-smi` may buffer its output when redirected to a file; if `vram.samples` is 0 in the
   first result line, peak VRAM was not captured (the run still succeeds). Check the first `*-vram.csv`.
2. **llama.cpp b11310 is a pre-release nightly** (`0.5.0-dev`). GitHub's "latest" release is `v0.5.0` (2026-09-23) and has no
   Windows assets, so the newest `bNNNNN` tag with assets was used. Re-pin with `fetch_assets.py --tag <b-tag>` and
   `-Build` if the lead prefers an older build.
3. **4B and 8B rerankers are community conversions.** Structure checked, quality not (until the GPU sanity test and results).
   The generic instruction in the template is used for BRIGHT, which probably understates reranker quality somewhat.
4. **Subset bias** (see above): easier than BRIGHT, length cue for gold documents, rerank sample of 60 queries (per-query nDCG
   varies a lot, so differences of a few points between rerankers are within noise even though the queries are paired).
5. **Tokens and BGE-M3.** Documents are truncated by Qwen3 tokens; BGE-M3 uses its own tokenizer, so its tokens per
   document differ slightly. The 2.5B-token projection uses BRIGHT passages (mean about 120 tokens); real chunks of a
   different length will have different tokens/s, so measure again with the production chunk size.
6. **Budget assumptions are guesses** (25 TFLOPS achieved). If the first two runs are much slower, drop to fewer
   `rerank_queries` with `make_subset.py --rerank-queries 10` and re-run the matrix (changing the subset changes
   `subset.json`; use a fresh `-OutDir` for comparable results).
7. **Parity is judged at cosine >= 0.999 as asked.** Q8_0 CUDA vs Vulkan may differ in the 4th decimal; the result line
   gives min and mean so a near miss is visible. `status` is `parity_failed` (not an error) when any pair fails.
8. **BRIGHT licence** is the card's CC-BY-4.0; the underlying sources are not separately cleared here.
9. Build times: `cargo build --release` took 6 to 9 minutes while the other benchmark was running (about 4 s once cached).

# Phase 0: dense-vector index benchmark (lancedb/lance vs usearch)

Date: 2026-09-30 to 2026-10-01. Author: benchmark worker (Opus 5.5) for the Phase 0 lead. Scope: pick the dense-vector index for libraryd at about 6M chunk vectors, projected to the AI PC (Ryzen 5 5600X, 6C/12T, Zen 3, AVX2 only, 32 GB RAM).

Code: `bench/vector-bench` (standalone cargo package). Raw results: `docs/phase0/vector-bench-results.jsonl`, one JSON object per line; every line carries the full configuration, versions, machine, thread count, dataset parameters and a UTC timestamp. Synthetic data and indexes: `C:\Users\sethy\Projects\vault-synth` (kept for re-runs).

## 1. Answer

**Recommendation: candidate B, usearch 2.26.2 in-memory HNSW with i8 vectors, connectivity M=32, expansion_add 128, ef_search ≈ 1024, no rescoring.**

On 6M synthetic 1024-dimensional vectors it reached recall@10 0.963 and recall@100 0.964 against exact search. Single-query latency at 6 threads (approximating the 5600X) was p50 2.4 ms and p95 5.1 ms, with 7.9 GiB resident. Candidate A's best configurations meeting both recall targets were 3x (IVF_PQ + refine) to 7x (IVF_RQ + refine) slower at p95. They use less process RAM, but they need the 11.4 GiB raw f16 vector column hot in the OS page cache, or their refine step becomes I/O-bound.

Main table: 6M x 1024, k=100, 1,000 queries against exact top-100, warm. Each row is the cheapest grid point meeting both recall targets, plus reference rows.

| Option | Settings | R@10 | R@100 | T6 p50 / p95 / p99 ms | T6 batch-of-4 p95 ms | T6 QPS (16 clients) | T12 p95 ms | RAM needed for this latency |
|---|---|---|---|---|---|---|---|---|
| **B usearch HNSW i8 M32** | ef 1024, no rescore | **0.963** | **0.964** | **2.40 / 5.09 / 7.03** | 7.30 | 1,828 | 5.57 | 7.9 GiB resident (7.8 GB file) |
| B usearch HNSW i8 M32 | ef 2048, rescore 200 from f16 | 0.984 | 0.983 | 5.28 / 10.47 / 14.61 | 12.79 | 1,030 | 8.76 | 7.9 GiB + f16 store cached (11.4 GiB) |
| B usearch HNSW i8 M16 | ef 1536, no rescore | 0.958 | 0.958 | 3.26 / 6.46 / 8.51 | 8.83 | 1,429 | 6.39 | 7.2 GiB resident (7.0 GB file) |
| A lance IVF_PQ (128 x 8 bit), 2450 partitions | nprobes 20, refine 5 | 0.977 | 0.958 | 13.82 / 15.88 / 16.51 | 20.09 | 306 | 45.16 (f16 column partly evicted) | 0.75 GiB index + f16 column cached (11.4 GiB) |
| A lance IVF_RQ 4-bit, 2450 partitions | nprobes 40, refine 2 | 0.973 | 0.970 | 24.08 / 34.48 / 42.19 | 42.67 | 232 | 31.55 | 3.0 GiB index cache + f16 column for refine |
| A lance IVF_RQ 1-bit, 2450 partitions | nprobes 20, refine 5 | 0.983 | 0.981 | (T12, contended) 21.56 / 35.69 / 53.63 | (T12) 39.90 | (T12) 196 | 35.69 | 0.81 GiB index + f16 column for refine |
| B usearch HNSW b1 M16 | ef 2048, rescore 2048 | 0.943 | 0.939 | (T12) 5.31 / 8.97 / 10.90 | (T12) 11.05 | (T12) 1,848 | 8.97 | 1.6 GiB + f16 store cached (11.4 GiB); misses the target |

The tradeoff in numbers:
- **Latency:** usearch i8 M32 meets both targets at p95 5.1 ms. The best Lance option is at 15.9 ms (PQ + refine 5, only while the f16 column is cached) or 34.5 ms (RQ 4-bit + refine 2).
- **Throughput at 16 clients on 6 cores:** 1,828 QPS versus 306 (PQ) and 232 (RQ).
- **RAM:** usearch holds 7.9 GiB resident at 1024 dimensions: 1,024 bytes of i8 vector plus 276 bytes of graph per row. Lance's process holds 0.9 GiB (PQ) to 3.2 GiB (RQ). Both Lance options still want the 11.4 GiB f16 column in page cache for their refine step.
- **Fit on the AI PC:** with 1024 dimensions, the usearch option leaves about 18 GiB of the 26 GiB budget (32 GB minus 4 GB OS minus 2 GB libraryd) for tantivy's page cache. A Lance option with its f16 column hot leaves about 11-14 GiB.

Conditions on the recommendation (details in sections 6-9):
1. **Filtered search needs a guard.** With a 10% field filter:
   - Queries that belong to the field stay fast with the plain HNSW predicate: p50 4.5-7 ms, p95 12-15 ms, recall@10 0.97-1.0.
   - Queries whose neighbourhood lies outside the field took 200-600 ms.
   - A random 10% attribute filter costs 16-35 ms p50 at recall 0.99.

   libraryd needs a fallback for the slow path: an exact scan over the filtered subset, per-field indexes, or a time budget. This is a design question for the lead (section 9).
2. **Load with `Index::new` + `load`, never `restore`/`view`, on Windows.** usearch 2.26.2 cannot open index files over 2 GiB through `restore` or `view` on Windows (32-bit `fseek`/`GetFileSize`).
3. **Embedder size matters.** 2560-dimension (4B embedder) i8 HNSW at 6M is about 15 GiB and still fits but squeezes tantivy. 4096 dimensions (8B embedder) is about 24 GiB and does not fit with tantivy. For those, MRL truncation to 1024, or Lance IVF_PQ/RQ, is the fallback.
4. **Real data could change the HNSW numbers in either direction.** The synthetic data's cluster separation made HNSW work hard: ef 1024 was needed at 6M, though 512 was enough at 1M. Real text embeddings may be easier or harder, and that single number moves the latency most. Phase 4 must re-measure it.

## 2. Setup

**Versions:** pinned exactly in `bench/vector-bench/Cargo.toml`; all were the latest on crates.io on 2026-09-30.
- lancedb 0.39.0, lance 12.0.0 and lance-core 12.0.0.
- usearch 2.26.2, which compiles NumKong 7.8.3 kernels.
- Toolchain: rustc/cargo 1.98.1, MSVC Build Tools 18.7.3 for the usearch C++ core.
- Arrow 58.4.0 and DataFusion 54.1.0 come in through lancedb.
- lancedb 0.39.0 must be built with its `remote` feature (section 8).

**Build:** `cargo build --release -j 6` with `-C target-cpu=x86-64-v3` (AVX2 + FMA + F16C, the AI PC's instruction set), set in `bench/vector-bench/.cargo/config.toml`. The same file sets `target-dir = C:/Users/sethy/Projects/vault/target` and `PROTOC`. Every compile-time-vectorised loop is therefore AVX2-only: this benchmark's own kernels and the Rust code inside lance. Kernels that choose their ISA at runtime are covered in section 7.

**Machine:**
- Ryzen 7 9800X3D (8C/16T, Zen 5, AVX-512, 96 MB L3).
- 64 GB RAM with 22-27 GB available during the runs (a WSL VM holds the rest).
- Samsung 990 PRO NVMe; Windows 11 Home 10.0.26200.
- Other sessions shared the machine throughout, so latencies carry background noise.

**Thread model (approximating the 5600X):** each benchmark process pins itself with `SetProcessAffinityMask` and sizes its rayon pool, tokio runtime and `LANCE_CPU_THREADS` to the same count. Core k = logical CPUs 2k and 2k+1, verified with `GetLogicalProcessorInformation`.
- **T6:** mask 0x555, one logical CPU on each of physical cores 0-5, no SMT siblings; 6 threads.
- **T12:** mask 0xFFF, both SMT siblings of cores 0-5 (6C/12T, like the 5600X); 12 threads.

Throughput tests run 8 or 16 closed-loop clients: OS threads for usearch, tokio tasks for Lance. They run inside the same mask, so CPU use never exceeds 6 or 12 logical CPUs. Index builds use 12 threads without a mask, spread over 8 cores, so build times are for this machine.

**Limits:**
- A watchdog thread in every step aborts if available RAM drops under 6 GiB or the working set exceeds 16 GiB. Above 13 GiB it drops the mmapped f16 store from the working set with `VirtualUnlock`; the pages stay in the OS cache.
- Every large step checks free RAM before it starts.
- No step was aborted. The highest peak working set was 13.45 GiB (an early diagnostic before the trim existed).
- Work runs in-process: one process per pipeline step, about 30 processes in total.

**Measurement conventions:**
- k = 100 for every query. Recall@10 compares the first 10 ids with the exact top 10; recall@100 compares all 100 with the exact top 100.
- Recall is computed on queries 0-999 in the 12-thread run (recall does not depend on threads).
- Latency is wall time per search call:
  - usearch: HNSW search plus optional rescoring.
  - Lance: build, execute and collect the stream, selecting only `id` and `_distance`.
- Disjoint query pools: 1000-1519 for cold start and warm-up, 2000-2999 for single-query latency (1,000 for usearch, 500 for Lance), 3000-3999 for batches of 4 (250 / 125 batches), 4000-9999 for throughput (5 s per client count).
- A batch of 4 issues 4 queries in parallel; its latency runs until all 4 return.
- usearch rescoring reads the candidates' f16 rows from a memory-mapped `base_f16.bin`. Before rescoring runs, the file is pre-read with plain reads, so it sits in the OS cache without entering the working set. That is the "f16 store resident in RAM" case.

## 3. Synthetic data

**Generator "vb-gen-1"** (`src/data.rs`, seed 20260930):
- RNG: SplitMix64 seeding plus xoshiro256++, Marsaglia polar normals. Each row and each query has its own stream, so the output does not depend on the thread count.
- Each vector before normalisation is `x = a_g·m + a_f·F[field] + a_t·C[topic] + U_topic·(a_s·w[topic,sub] + a_l·z) + a_e·ε`, where:
  - `m`: a global direction (the anisotropy behind positive random-pair cosine).
  - `F`: 10 field directions; each topic belongs to one field.
  - `C`: 2,000 topic centres, drawn from a shared 64-dimensional latent space; topic sizes ~1/(t+20), from about 65k rows down to about 650.
  - `U_t`: a per-topic 48-column basis, the low-rank intrinsic dimensions, with spectrum λ_i ~ 1/(1 + i/8).
  - `w`: 25 subtopic offsets per topic, inside the topic subspace.
  - `z`: per-vector low-rank variation.
  - `ε`: isotropic noise.
- Energies: global 0.15, field 0.09, topic 0.25, subtopic 0.25, local 0.21, isotropic 0.05.
- 3% of rows (180,224) are near-duplicates (cosine about 0.97) of an earlier row.
- Rows are unit-normalised in f32 and stored as f16: `base_f16.bin` is 12.3 GB. Metadata per row: topic, subtopic, field, a uniform random attribute `rattr` (0-9), and a duplicate flag.
- Generation of 6M rows took 19 s on 12 threads.

**Queries (10,000):**
- 85% are perturbations of a uniformly chosen base row. Cosine to the source is uniform in [0.55, 0.80]; 90% of the perturbation energy lies inside the topic subspace and 10% is isotropic.
- 15% come from topic centres (topic part plus isotropic noise). Their neighbour lists are flat.
- Queries 0-999 are the ground-truth set: 853 perturbation and 147 centre queries.

**Tuning:** done with the `tune` command, which samples full-density rows of 40 topics in about 1.5 s per setting. It moved the query top-1 to top-10 gap from 0.125 to about 0.07.

**One structural choice made after a smoke test.** Independent random topic centres in 1,024 dimensions are mutually orthogonal islands. On a 200k smoke set at the same per-topic density, they capped HNSW recall@10 at 0.995 even with exact rescoring of 4,096 candidates, and left queries with zero recall. Drawing the centres from a shared 64-dimensional latent space (related topics exist) raised the ceiling to 0.999 at ef 512. The cosine statistics barely changed: random-pair 0.166 versus 0.166, top-1 0.689 versus 0.691, LID 29 versus 29. **HNSW is sensitive to how clustered the data is; IVF much less so.** That is the main reason the HNSW numbers must be re-measured on real embeddings.

**Measured statistics** of the 6M set (exact neighbours from the ground-truth pass; percentiles p25 / p50 / p75; LID is the Levina-Bickel MLE on Euclidean distances of unit vectors):

| Statistic | Value |
|---|---|
| Random-pair cosine (200k pairs of 20k random rows) | mean 0.164, sd 0.056, p05 0.083, p95 0.258 |
| Query → 1st neighbour cosine, all 1,000 queries | 0.660 / 0.713 / 0.750 |
| ... perturbation queries (853) | 0.652 / 0.701 / 0.756 |
| ... topic-centre queries (147) | 0.725 / 0.735 / 0.744 |
| Query 10th / 100th neighbour cosine (median) | 0.639 / 0.574 |
| Gap 1st − 10th neighbour, all queries | 0.036 / 0.059 / 0.096 (centre queries: median 0.019) |
| Local intrinsic dimensionality, queries (median) | 33.3 at k=20, 29.9 at k=100 (centre queries 68.7 / 59.0) |
| Document → nearest other document cosine (500 rows) | 0.829 / 0.844 / 0.860; 10th 0.812; 100th 0.757 |
| Document LID (median) | 26.5 at k=20, 18.3 at k=100 |

These are within the range commonly reported for modern text embedders, but none of them was compared with real Qwen3-Embedding-0.6B output in this phase. The lead should compute the same statistics on real chunks; the `gt` command's code (`neighbour_stats`, `lid_mle`) can be reused.

**Synthetic data cannot validate Matryoshka (MRL) truncation quality.** The generator has no nested-dimension structure, so the 512- and 256-dimension rows in section 7 are RAM and latency arithmetic only. Recall after truncation must be measured on real vectors.

## 4. Ground truth

- **Method:** exact top-100 by brute-force inner product for queries 0-999, unfiltered and under each filter. The f16 base file is streamed in 8,192-row blocks and converted with F16C; a 1-row x 4-query AVX2/FMA kernel runs in parallel with rayon on 12 threads. The scan took 32 s.
- **Doc-to-doc neighbours:** the same pass computes top-100 neighbours of 500 random base rows (self excluded) for the document statistics.
- **Verification:** in the same pass, an independent naive path (scalar f16 decode via a bit-level table, f64 accumulation) scans all 6M rows for queries 0-4.
  - Result: identical top-100 sets for all 5, with 0 mismatches and a maximum rank-wise score difference of 1.0e-7.
  - The command asserts that any mismatch is explained by a tie (within 1e-5 of the 100th score).
- **Filter ground truth:**
  - The field filter keeps field 7: 619,200 rows, 10.32% (the field closest to 10%).
  - The attribute filter keeps `rattr = 3`: 10.0%, uncorrelated with topics.

## 5. Results at 6M x 1024

### 5.1 Builds

| Candidate | Index | Build time (12 threads, this machine) | On disk | Memory |
|---|---|---|---|---|
| A | Lance ingest, 6M rows, f16 `FixedSizeList` + id/field/rattr/topic | 10 s (+1 s for two BITMAP scalar indexes) | 11.47 GiB data | peak WS 1.1 GiB |
| A | IVF_RQ, 2450 partitions, 4 bits (`num_bits` set explicitly) | 70 s | 3.00 GiB (537 B/row) | build peak WS 1.6 GiB |
| A | IVF_PQ, 2450 partitions, 128 sub-vectors x 8 bits | 334 s | 0.75 GiB (134 B/row) | build peak WS 1.4 GiB |
| B | HNSW i8 M16, efc 128 | 384 s (15.6k vec/s) | 6.55 GiB (1,173 B/row) | after build: 10.0 GiB allocated; after `load`: 7.2 GiB resident, 10.4 GiB committed |
| B | HNSW i8 M32, efc 128 | 365 s | 7.27 GiB (1,300 B/row) | after `load`: 7.9 GiB resident, 10.4 GiB committed |
| B | HNSW b1 M16, efc 128 (sign of x − corpus mean) | 149 s | 1.55 GiB | after `load`: 2.2 GiB resident, 3.4 GiB committed |
| | Extra Lance partition counts and 1-bit RQ | see 5.6 | | |

usearch's i8 codes use my own per-vector max-abs scaling to ±127. That is safe because cosine is scale-invariant. usearch's built-in f32→i8 cast normalises to unit length and multiplies by 127, which leaves a 1024-dimension unit vector only about ±4 quantisation levels per component; section 5.6 compares the two. Measured quantisation ceiling: exact search over the i8 codes reaches recall@10 0.994 and recall@100 0.996 without rescoring, and 1.000 / 1.000 with rescoring of 256 candidates. So HNSW recall losses below come from graph navigation, not quantisation.

usearch commits about 1.4x the file size (the vector and graph allocators reserve capacity they never touch), but the resident set after `load` is about the file size. Physical RAM need equals the file size; commit charge is larger (it counts against RAM plus pagefile).

### 5.2 Recall versus latency (warm, T6 unless noted)

Full grid in appendix A (generated by `summarize.py`).

usearch HNSW (no rescoring unless stated):

| Index | ef | R@10 | R@100 | p50 ms | p95 ms | p99 ms | QPS, 16 clients |
|---|---|---|---|---|---|---|---|
| i8 M16 | 256 | 0.840 | 0.828 | 0.90 | 1.72 | 2.37 | 5,871 |
| i8 M16 | 512 | 0.903 | 0.899 | 1.32 | 2.77 | 3.54 | 3,623 |
| i8 M16 | 1024 | 0.937 | 0.936 | 2.32 | 4.70 | 6.24 | 2,171 |
| i8 M16 | 1536 | 0.958 | 0.958 | 3.26 | 6.46 | 8.51 | 1,429 |
| i8 M16 | 2048 | 0.969 | 0.970 | 4.26 | 8.39 | 10.78 | 1,192 |
| i8 M32 | 256 | 0.902 | 0.893 | 0.84 | 1.80 | 2.25 | 4,814 |
| i8 M32 | 512 | 0.936 | 0.935 | 1.54 | 3.31 | 4.50 | 3,038 |
| i8 M32 | 1024 | 0.963 | 0.964 | 2.40 | 5.09 | 7.03 | 1,828 |
| i8 M32 | 1536 | 0.972 | 0.974 | 3.57 | 7.27 | 9.76 | 1,263 |
| i8 M32 | 2048 | 0.978 | 0.979 | 5.07 | 9.85 | 13.26 | 961 |
| i8 M32 | 2048 + rescore 200 | 0.984 | 0.983 | 5.28 | 10.47 | 14.61 | 1,030 |
| b1 M16 (T12) | 1024 + rescore 1000 | 0.906 | 0.893 | 2.01 | 3.89 | 4.71 | 3,750 |
| b1 M16 (T12) | 2048 + rescore 2048 | 0.943 | 0.939 | 5.31 | 8.97 | 10.90 | 1,848 |

Notes on usearch:
- **Rescoring:** exact rescoring of the top 200 from the cached f16 store adds only 0.1-0.6 ms. It lifts recall@10 by about 0.005, because it only fixes the order inside the candidate set.
- **Uncached store:** when the f16 store was not in the OS cache, an early diagnostic rescoring 512-2048 candidates per query took 30-64 ms per query, because each candidate costs a hard page fault (about 50-60 µs on this NVMe). Rescoring is only cheap if the 11.4 GiB store is RAM-resident.
- **Threads:** single-query latency is almost the same at T6 and T12, because a usearch search is single-threaded.
- **Query type:** perturbation queries are the hard case for HNSW. At M16 ef 1024 they reach recall@10 0.929; centre queries reach 0.980. IVF shows the opposite pattern.

Lance (L2 on unit vectors; nprobes and refine factor as listed):

| Index | nprobes | refine | R@10 | R@100 | T6 p50 ms | T6 p95 ms | T12 p95 ms | T6 QPS, 16 clients | Process WS |
|---|---|---|---|---|---|---|---|---|---|
| IVF_RQ 4-bit | 10 | - | 0.890 | 0.904 | 7.58 | 16.00 | 12.17 | 578 | 3.2 GiB |
| IVF_RQ 4-bit | 20 | - | 0.907 | 0.922 | 12.84 | 24.87 | 17.50 | 452 | 3.2 GiB |
| IVF_RQ 4-bit | 40 | - | 0.931 | 0.947 | 22.51 | 35.80 | 28.37 | 255 | 3.2 GiB |
| IVF_RQ 4-bit | 80 | - | 0.946 | 0.965 | 43.00 | 56.68 | 49.30 | 142 | 3.2 GiB |
| IVF_RQ 4-bit | 160 | - | 0.950 | 0.970 | 75.81 | 89.81 | 78.32 | 70 | 3.2 GiB |
| IVF_RQ 4-bit | 40 | 2 | 0.973 | 0.970 | 24.08 | 34.48 | 31.55 | 232 | 3.2 GiB |
| IVF_RQ 4-bit | 80 | 2 | 0.992 | 0.990 | 44.46 | 58.51 | 52.26 | 134 | 3.2 GiB |
| IVF_PQ 128x8 | 20 | - | 0.678 | 0.750 | 9.31 | 11.67 | 10.45 | 585 | 0.9 GiB |
| IVF_PQ 128x8 | 80 | - | 0.682 | 0.757 | 29.53 | 33.05 | 34.89 | 199 | 0.9 GiB |
| IVF_PQ 128x8 | 20 | 5 | 0.977 | 0.958 | 13.82 | 15.88 | 45.16* | 306 | 0.9 GiB |
| IVF_PQ 128x8 | 40 | 5 | 0.989 | 0.971 | 20.85 | 23.90 | 25.90 | 212 | 0.9 GiB |
| IVF_PQ 128x8 | 80 | 5 | 0.996 | 0.979 | 33.97 | 36.96 | 44.26 | 151 | 0.9 GiB |

\* The 12-thread PQ run came first, right after usearch runs had filled the page cache with `base_f16.bin`. Lance's own f16 column was then partly evicted, so refine reads went to disk; the 6-thread run that followed found it cached. This is the same residency effect as usearch's rescoring: **refine/rescore latency depends on the raw-vector column being in RAM.**

Notes on Lance:
- **RQ 4-bit without refine** saturates at recall@10 0.95 even at nprobes 160. Refine 2 is needed to reach 0.97-0.99.
- **PQ with 128 sub-vectors (8 dimensions each)** is a poor approximation on this data without refine: 0.68 recall@10 at any nprobes. With refine 5 it is the fastest Lance option.
- **Partition scan time** dominates Lance latency: about 0.55 ms per probed partition for RQ 4-bit and 0.37 ms for PQ, at about 2,450 rows per partition.

### 5.3 Concurrency and batches

| Option | T6 single p95 | T6 batch-of-4 p50 / p95 | T6, 8 clients: QPS / p95 | T6, 16 clients: QPS / p95 | T12, 16 clients: QPS / p95 |
|---|---|---|---|---|---|
| usearch i8 M32 ef 1024 | 5.09 ms | 4.18 / 7.30 ms | 1,876 / 18.9 ms | 1,828 / 40.3 ms | 2,807 / 17.5 ms |
| usearch i8 M16 ef 1536 | 6.46 ms | 5.12 / 8.83 ms | 1,499 / 20.5 ms | 1,429 / 42.8 ms | 2,134 / 20.3 ms |
| Lance IVF_PQ np20 rf5 | 15.88 ms | 17.23 / 20.09 ms | 297 / 37.8 ms | 306 / 68.8 ms | 405 / 54.2 ms (f16 partly uncached) |
| Lance IVF_RQ4 np40 rf2 | 34.48 ms | 32.50 / 42.67 ms | 231 / 51.7 ms | 232 / 103.4 ms | 286 / 76.6 ms |

With 16 clients on 6 cores, every option is CPU-saturated, so latency becomes queueing. The 12-thread setting (SMT on the same 6 cores) adds 49-54% throughput for usearch and 23-32% for Lance.

### 5.4 Metadata filter keeping about 10% of rows

Two filters:
- **field = 7**, which follows the topic structure.
- **rattr = 3**, a uniform random attribute standing for language, licence or similar.

Recall is against the exact filtered top-100. "Query in field" means queries whose own topic lies in field 7: 95 of the 1,000 ground-truth queries. That is the realistic use of a field filter.

| Option | Filter | Setting | R@10 all | R@10, queries in field | R@10, queries outside | p50 / p95 ms (mixed queries, T6) |
|---|---|---|---|---|---|---|
| usearch i8 M16 | field | ef 1024, rescore 200 | 0.801 | 0.965 | 0.784 | 207.5 / 455.4 |
| usearch i8 M16 | field | ef 2048, rescore 200 | 0.878 | 0.988 | 0.866 | 255.5 / 541.3 |
| usearch i8 M32 | field | ef 1024, rescore 200 | 0.830 | 0.999 | 0.812 | 226.0 / 518.6 |
| usearch i8 M16 | rattr | ef 1024, rescore 200 | 0.988 | - | - | 16.2 / 25.9 |
| usearch i8 M32 | rattr | ef 1024, rescore 200 | 0.994 | - | - | 17.6 / 29.2 |
| Lance IVF_RQ4 | field | np 40, refine 2 (prefilter, BITMAP index) | 0.159 | 0.991 | 0.072 | 25.4 / 37.4 |
| Lance IVF_RQ4 | rattr | np 40, refine 2 | 0.973 | - | - | 26.6 / 40.8 |
| Lance IVF_PQ | field | np 40, refine 2 | 0.112 | 0.989 | 0.020 | 15.6 / 26.2 |
| Lance IVF_PQ | rattr | np 40, refine 2 | 0.984 | - | - | 20.4 / 23.2 |

Latency for queries inside the filtered field only: see 5.6.

Reading:
- **Both candidates handle the realistic field case.** For queries inside the field, recall@10 is 0.96-1.0.
- **Outside the field, they fail differently.**
  - usearch keeps searching and eventually finds the filtered neighbours (recall@10 0.78-0.87), but at 200-600 ms, because the traversal visits a large part of the graph.
  - Lance answers in normal time but returns almost nothing relevant (recall 0.02-0.07), because the probed partitions hold no rows of the field.
  - Lance's `maximum_nprobes` (unset here) could widen the search when the filter starves results.
- **Random 10% attribute:** HNSW keeps 0.99 recall at 3-7x its unfiltered latency (16-35 ms p50). Lance keeps its unfiltered latency, with recall 0.92-0.98.
- Lance supports scalar indexes (BTREE, BITMAP, LABEL_LIST, FTS) for prefiltering. usearch only offers a key predicate callback during traversal; libraryd would hold the metadata itself (6M x 2 bytes here).

### 5.5 Cold start (approximate)

"Cold" means a fresh process, measured right after the index was opened. The OS page cache could not be dropped without admin rights, so the index files may have been cached. These are lower bounds for a true cold start.

| Option | Open / load | First query | First 20 queries p50 |
|---|---|---|---|
| usearch i8 M16 (7.0 GB file) | `load` 5.7 s (T6) - 9.8 s (T12) | 5.4-7.1 ms | 4.3-4.8 ms |
| usearch i8 M32 (7.8 GB file) | `load` 5.7-9.0 s | 9.4-11.3 ms | 4.7-5.3 ms |
| usearch b1 M16 (1.7 GB file) | `load` 1.4-1.6 s | 5.7-8.2 ms | 5.3 ms |
| Lance IVF_RQ4 | open < 0.01 s (lazy) | 61-101 ms | 18.3-18.4 ms |
| Lance IVF_PQ | open < 0.05 s | 23-170 ms | 10.6-19.4 ms |

usearch has to read its whole file before the first query: about 7-8 GB, so seconds from NVMe (sequential 990 PRO about 5-7 GB/s, plus allocation). Lance answers immediately, then warms its index cache (6 GiB default) partition by partition. Truly cold, the usearch load would be bounded by disk speed: about 2 s for 7.8 GB at 4 GB/s on top of the memory copy.

### 5.6 Additional runs (stage 3)

These ran while other sessions loaded the machine. Three `rustc` builds, Windows Defender (about 1.7 cores) and others brought total CPU to 100% at a 04:58 UTC check, so latencies in this subsection are inflated and comparable only within a row group. Recall is unaffected.

**(a) Lance partition count and RQ bit width (6M x 1024, 12 threads):**

| Index | Build | Index size | nprobes / refine | R@10 | R@100 | p50 ms | p95 ms |
|---|---|---|---|---|---|---|---|
| IVF_RQ 4-bit, 2450 partitions (reference, stage 1) | 70 s | 3.00 GiB | 40 / 2 | 0.973 | 0.970 | 22.5 | 31.6 |
| IVF_RQ 4-bit, 4900 partitions | 106 s | 3.00 GiB | 40 / 2 | 0.964 | 0.960 | 26.3 | 42.9 |
| | | | 80 / 2 | 0.990 | 0.987 | 44.7 | 63.8 |
| IVF_RQ 4-bit, 9800 partitions (sample rate 71) | 127 s | 3.0 GiB | 40 / - | 0.925 | 0.940 | 39.4 | 71.4 |
| | | | 80 / - | 0.939 | 0.956 | 67.9 | 103.7 |
| IVF_RQ **1-bit**, 2450 partitions | 82 s | 0.81 GiB | 20 / - | 0.736 | 0.817 | 12.6 | 21.5 |
| | | | 20 / 5 | **0.983** | **0.981** | 21.6 | 35.7 |
| | | | 40 / 5 | 0.992 | 0.991 | 28.1 | 49.2 |

What this shows:
- **More partitions do not help Lance here.** At the same nprobes, recall drops (fewer rows per probe) and latency does not fall. A per-probe cost dominates, so about sqrt(N) partitions (2,450) is the best of the three.
- **IVF_RQ 1-bit + refine 5** is a strong, memory-light Lance option: 0.81 GiB index, about 1.0 GiB process working set, recall 0.98 at nprobes 20. But it reads 500 f16 rows per query for refine, so its latency depends on the f16 column being cached, like PQ + refine.
- **Unexplained:** with IVF_RQ 4-bit, refine factor 5 returns exactly the same recall as refine 2 at every nprobes and both partition counts (to 4 decimals over 1,000 queries). Meanwhile, 1-bit + refine 5 at nprobes 20 beats 4-bit + refine 2 or 5 at nprobes 20 (0.983 versus 0.946). Lance sizes the candidate heap as k x refine_factor in both paths (`lance-12.0.0/src/index/vector/ivf/v2.rs`), so the cause is not obvious. Worth an upstream question before relying on 4-bit RQ with refine.

**(b) Filtered search, latency for queries inside the filtered field** (6 threads, 300 queries whose own field is 7; `--filter-only` re-runs):

| Index | Setting | p50 ms | p95 ms | Mixed-query p50 / p95 ms in the same run |
|---|---|---|---|---|
| usearch i8 M16 | ef 1024, rescore 200 | 4.54 | 11.76 | 306 / 999 |
| usearch i8 M16 | ef 2048, rescore 200 | 7.03 | 13.69 | 313 / 848 |
| usearch i8 M32 | ef 2048, rescore 200 | 7.25 | 14.80 | 424 / 1,588 |
| usearch i8 M32 | ef 1024, rescore 200 | 53.6 | 192.7 | 516 / 2,099 (contention spike; the same run's rattr p50 was 122 ms against 18 ms in stage 2, so this row is unreliable) |
| usearch b1 M16 | ef 1024, rescore 1024 | 3.72 | 7.58 | 171 / 468 |
| Lance IVF_RQ4 | nprobes 20 | 13.44 | 21.82 | 15.1 / 31.2 |
| Lance IVF_RQ4 | nprobes 40 | 23.93 | 34.69 | 24.4 / 36.4 |
| Lance IVF_PQ | nprobes 20 | 13.83 | 16.36 | 7.6 / 14.6 |

With the filter, HNSW is as fast as unfiltered when the query belongs in the filtered region. The slow path (hundreds of ms) is only for queries whose neighbourhood is outside it, and those are the queries for which a field filter makes little sense. libraryd still needs a guard (a timeout and fallback, or an exact filtered scan) for that case.

**(c) 1M x 1024 (same generator, 1M rows, so 6x fewer rows per topic), 12 threads, M16:**

| Index | ef | R@10 | R@100 | p95 ms | File / memory |
|---|---|---|---|---|---|
| i8, own per-vector scaling | 512 | 0.969 | 0.968 | 1.74 | 1.09 GiB / 1.24 GiB |
| i8, own per-vector scaling | 1024 | 0.980 | 0.980 | 2.76 | |
| i8, usearch built-in f32→i8 cast | 1024 | 0.924 | 0.947 | 3.87 | 1.09 GiB / 1.24 GiB |
| i8, usearch built-in cast | 2048 | 0.932 | 0.956 | 5.92 | |
| i8, built-in cast, rescore 200 | 1024 | 0.981 | 0.981 | 3.69 | + f16 store |
| f16 | 1024 | 0.984 | 0.984 | 4.74 | 2.05 GiB / 2.24 GiB |
| f16 | 2048 | 0.990 | 0.990 | 21.8 (contended) | |
| Lance IVF_RQ4, 1000 partitions | nprobes 20 / refine 2 | 0.981 | 0.973 | 10.15 | 0.50 GiB index |

What this shows:
- **usearch's own float→i8 cast costs 0.05 recall@10** against per-vector scaling (it caps at 0.93 recall@10 without rescoring). Quantise before calling usearch, or rescore.
- **f16 HNSW beats i8 by only 0.004-0.006 recall** at twice the memory.
- **Density matters:** the same generator at 1M reaches 0.98 at ef 1024 (M16), against 0.937 at 6M. HNSW effort grows with density per cluster.
- **f16 uses AVX-512 here:** NumKong chose the AVX-512 "skylake" kernel for f16 (`isa = skylake` in the build record), while i8 and b1 use haswell. An f16 index would therefore lose AVX-512 on the 5600X.

**(d) 1M x 2560 (the 4B embedder's size) against 1M x 1024, 12 threads, contended:**

| Index | Setting | R@10 (2560 / 1024) | R@100 (2560 / 1024) | p95 ms (2560 / 1024) | Size (2560 / 1024) |
|---|---|---|---|---|---|
| usearch i8 M16 | ef 512 | 0.965 / 0.969 | 0.967 / 0.968 | 3.32 / 1.74 | 2.52 / 1.09 GiB file |
| usearch i8 M16 | ef 1024 | 0.980 / 0.980 | 0.982 / 0.980 | 6.58 / 2.76 | |
| Lance IVF_RQ4, 1000 partitions | nprobes 20, no refine | 0.971 / 0.949 | 0.976 / 0.958 | 17.58 / 7.13 | 1.22 / 0.50 GiB index |
| Lance IVF_RQ4, 1000 partitions | nprobes 20, refine 2 | 0.992 / 0.981 | 0.987 / 0.973 | 28.0 / 10.2 | |

What this shows:
- **Size:** 2560 dimensions measured 2,706 B/row for i8 M16 (formula d + 149 = 2,709) and 1,309 B/row for RQ4 (d/2 + 29). This confirms the section-7 projections: 6M x 2560 i8 M16 = 15.1 GiB.
- **Latency:** grows about 2-2.5x for both candidates. (The generator's statistics at 2560 dimensions match 1024 within 0.01, because the model is dimension-agnostic, so this says nothing about whether the 4B embedder retrieves better.)

## 6. What decides it

| Criterion | A: lancedb/lance IVF | B: usearch HNSW i8 |
|---|---|---|
| p95 at ≥0.95 / ≥0.95 recall, 6 threads | 15.9 ms (PQ+refine, f16 column hot) / 34.5 ms (RQ4+refine) | 5.1 ms (M32) / 6.5 ms (M16) |
| Throughput, 16 clients, 6C | 230-310 QPS | 1,400-1,800 QPS |
| RAM at 6M x 1024 | 0.75-3.0 GiB process + 11.4 GiB f16 column in page cache for refine | 7.2-7.9 GiB resident; optional 11.4 GiB f16 store only if rescoring |
| Behaviour if RAM is short | degrades to disk reads in refine (measured 45 ms p95 when partly uncached) | needs the whole index resident; paging would be catastrophic |
| Filters | built in (SQL predicate + scalar indexes), fast; IVF misses out-of-cluster matches | predicate callback; accurate but 200-600 ms slow path when the filter excludes the query's neighbourhood |
| Updates | append + optimize/reindex; versioned dataset | incremental add/remove (tombstones); libraryd owns persistence and crash safety |
| Build at 6M | 70 s (RQ) / 334 s (PQ) | 365-384 s (12 threads on 8 cores) |
| Operational risk found | lancedb `remote` feature needed to compile; RQ `num_bits` still defaults to 1 bit; same-column indexes are silently ambiguous | `restore`/`view` broken for files > 2 GiB on Windows; 1.4x commit overhead |

## 7. Projection to the AI PC (6M vectors, 32 GB RAM)

### 7.1 RAM

Budget: 32 GiB − OS 4 − libraryd 2 = **26 GiB** for the vector index plus tantivy's page cache.

Per-row formulas from the 6M x 1024 measurements:
- usearch i8 M16 = d + 149 B
- usearch i8 M32 = d + 276 B
- usearch f16 M16 = 2d + 149 B
- usearch b1 M16 = d/8 + 149 B
- Lance RQ b bits = d·b/8 + 25 B
- Lance PQ with d/8 sub-vectors = d/8 + 6 B
- Raw f16 store / column = 2d B

The 2560-dimension usearch figure is checked against a 1M x 2560 measurement in 5.6.

| d | usearch i8 M16 | usearch i8 M32 | usearch f16 M16 | usearch b1 M16 (+f16 store) | Lance RQ4 | Lance RQ1 | Lance PQ d/8 | f16 store (refine/rescore) | Left for tantivy with usearch i8 M32 (no rescore) | Left with Lance PQ + hot f16 column |
|---|---|---|---|---|---|---|---|---|---|---|
| 256 | 2.26 | 2.98 | 3.69 | 1.01 (+2.86) | 0.85 | 0.32 | 0.21 | 2.86 | 23.0 | 22.9 |
| 512 | 3.69 | 4.40 | 6.55 | 1.19 (+5.72) | 1.57 | 0.50 | 0.39 | 5.72 | 21.6 | 19.9 |
| **1024** | **6.55** | **7.27** | 12.27 | 1.55 (+11.44) | 3.00 | 0.85 | 0.75 | 11.44 | **18.7** | 13.8 |
| 2560 (4B) | 15.14 | 15.85 | 29.44 | 2.62 (+28.61) | 7.29 | 1.93 | 1.82 | 28.61 | 10.2 | does not fit hot (−4.4) |
| 4096 (8B) | 23.72 | 24.43 | 46.61 | 3.69 (+45.78) | 11.58 | 3.00 | 2.89 | 45.78 | 1.6 (does not fit) | does not fit hot |

All values are GiB at 6M rows. "Left" = 26 − column(s).

Reading:
- **1024 dimensions:** usearch i8 M32 without rescoring fits with about 18.7 GiB left for tantivy. Lance needs its f16 column hot for the measured latency, leaving about 13.8 GiB.
- **2560 dimensions:** usearch i8 fits with about 10 GiB left for tantivy, which may be tight. Lance PQ without a hot f16 column fits easily, but its refine reads then go to NVMe (expect tens of ms).
- **4096 dimensions:** only Lance (PQ or RQ1 with on-disk refine) or MRL truncation fits.
- **Rebuild headroom:** building in-process allocates about 1.4x the file size for usearch (10.0 GiB for M16 at 1024 dimensions). libraryd should build offline or in a separate process and `load` the saved file, or budget the extra commit.

### 7.2 Latency penalty Zen 3 / AVX2 versus this Zen 5 / AVX-512 machine

What the SIMD paths are:
- **usearch / NumKong:** the MSVC build of NumKong 7.8.3 exposes only the serial, haswell (AVX2) and skylake backends on this machine (`hardware_acceleration_available = "serial, haswell, skylake"`). The kernel chosen for i8 cosine and for b1 Hamming was **haswell (AVX2) on this AVX-512 machine** (`isa = haswell` in the i8 and b1 build records). Only the f16 index (1M test) got the AVX-512 "skylake" kernel. usearch's graph-traversal C++ is compiled by MSVC with `/O2` and no `/arch`, which is SSE2 baseline. **So the usearch measurements already use the AI PC's instruction set.** A forced-AVX2 run was unnecessary; the hook for it (`--metric avx2`, through NumKong's exported `nk_find_kernel_punned` and usearch `change_metric`) is in the code but was not run.
- **Lance:** `lance_core::utils::cpu::simd_info()` reports tier `avx512` here. Lance's distance kernels (lance-linalg, lance-index RaBitQ/PQ) choose AVX-512 or AVX2 at runtime with `is_x86_feature_detected!`, and there is no environment override. **Forcing AVX2 was not possible** without patching Lance, so the Lance penalty below is an estimate. Rust code compiled into the benchmark is AVX2-only (`x86-64-v3`) on both machines.

Estimate (not measured):
- **General gap:** the 5600X boosts to 4.6 GHz against about 5.2 GHz here (×1.13), with roughly 25-30% lower IPC (Zen 3 → Zen 5, vendor figures, not measured). It has a 32 MB L3 against 96 MB, and DDR4 against DDR5 (similar latency, lower bandwidth).
- **usearch:** HNSW search here is memory-latency-bound (random 1-1.3 KB vector reads across a 7 GiB index) and runs the same AVX2 kernel. Expect **×1.3-1.6**: the recommended setting goes from p95 5.1 ms to **about 7-8 ms**.
- **Lance:** partition scanning is compute-bound and loses AVX-512 in its RaBitQ/PQ distance kernels. Expect **×1.5-2.2**: RQ4 np40 rf2 from 34.5 ms to **about 50-75 ms**; PQ np20 rf5 from 15.9 ms to **about 24-35 ms**.
- Both need one confirming run on the AI PC (section 9).

## 8. Library findings (for libraryd)

1. **lancedb 0.39.0 does not compile without the `remote` feature.** `src/job.rs` uses `Error::Http`, which only exists under `remote`, so `default-features = false` and the default feature set (`[]`) both fail. Workaround: `features = ["remote"]` (pulls in reqwest, tonic, arrow-flight).
2. **lancedb 0.39.0 still defaults IVF_RQ `num_bits` to 1** (`src/table/create_index.rs:326`, `index.num_bits.unwrap_or(1)`), while lance's own `RQBuildParams` documents a default of 5. Always set `num_bits` explicitly; this benchmark uses 4.
3. **lancedb `create_index(...).replace(true)` replaces only an index with the same name.** A second vector index under another name on the same column is accepted, and queries then silently use one of them (lancedb queries cannot choose an index by name). This bit the benchmark: the first IVF_PQ results were really IVF_RQ. Their 10 JSONL lines were deleted and the runs repeated. The harness now drops other vector indexes before building and asserts that exactly one vector index exists at the benchmarked version.
4. **usearch 2.26.2 on Windows cannot `restore` or `view` index files over 2 GiB.** `index_dense_metadata_from_path` seeks with `std::fseek(file, static_cast<long>(offset))` (32-bit `long` on Windows), and `view` uses `GetFileSize(h, 0)` (low 32 bits only). `Index::restore` fails with "Not a dense USearch index!". `Index::new(&options)` + `load(path)` reads sequentially and works (7-8 GB files loaded in 6-10 s). Worth an upstream issue; `view_from_buffer` over a Rust `memmap2` mapping is an untested alternative that would avoid both the copy and the bug.
5. **usearch's built-in float→i8 conversion** normalises to unit length and scales by 127. At 1024 dimensions that leaves typical components at only about ±4 levels. Quantise yourself: per-vector max-abs scaling to ±127 reached the 0.994/0.996 exact-search ceiling. Measured at 1M: the built-in cast gave recall@10 0.924 against 0.980 at ef 1024 (5.6(c)).
6. **Lance IVF_RQ 4-bit ignores refine factors above 2 in effect.** Refine 5 gave the same recall as refine 2 in every case, and 1-bit + refine 5 beat 4-bit + refine at the same nprobes (5.6(a)). This is unexplained; ask upstream before relying on it.
7. **usearch memory:** the vector allocator reserves about 1.4x what it uses (10.0 GiB committed versus 7.2 GiB resident for M16 at 1024 dimensions). Plan commit and pagefile accordingly.
8. **usearch thread contexts:** concurrent searches beyond the reserved thread count fail. Call `reserve_capacity_and_threads(size, max_concurrency)` after `load`. The benchmark reserves 64.
9. **Comparable Rust crates (read only, not benchmarked):**
   - hnsw_rs 0.3.4 (pure Rust, f32, no built-in quantisation).
   - instant-distance 0.6.1 (last release 2023-06).
   - hannoy 0.2.0 (Meilisearch's LMDB-backed HNSW with binary quantisation, MIT, released 2026-08-14; disk-backed through mmap; interesting if RAM gets tight, but 0.x).
   - arroy 0.8.0 (Annoy-style trees on LMDB, being superseded by hannoy).
   - **diskann 0.59.0** (Microsoft's Rust DiskANN, MIT, three releases in September 2026, so the API is still moving).

   None is clearly better than usearch for an in-RAM index today. diskann is the one to revisit if the 2560/4096-dimension embedders win the Phase 4 bake-off and RAM becomes binding, because it keeps only compressed vectors in RAM and the graph plus full vectors on SSD.

## 9. Limitations, deviations, skipped work, Phase 4 re-measurements

**Deviations from the method:**
- Topic centres come from a shared 64-dimensional latent space (3, smoke-test evidence).
- Latency and recall use k = 100 throughout; k = 10 latency was not measured separately. For HNSW, ef dominates the cost, so k = 10 at the same ef costs about the same.
- usearch rescoring is measured with the f16 store pre-read into the OS cache, because a 7-8 GiB index plus the mmapped store would breach the 16 GiB working-set limit if pinned. The uncached case is shown by the diagnostic (30-64 ms).
- The "6 threads" setting is 6 physical cores without SMT, and "12" is the same 6 cores with SMT, rather than 12 threads over this CPU's 8 cores. That is a closer model of the 5600X.
- usearch throughput clients are OS threads (up to 16) confined to the 6 or 12 logical-CPU mask. Lance clients are tokio tasks.
- f16 HNSW at 6M was not built: 12.3 GiB of vectors plus graph is about 13 GiB, near the 16 GiB working-set limit with only about 25 GiB available. It is measured at 1M instead (5.6).
- The AVX2-forced usearch run was not needed (the AVX2 kernel is already the one used). Lance could not be forced (7.2).
- IVF_HNSW_SQ was not benchmarked. lancedb 0.39.0 offers it (`Index::IvfHnswSq`), but its index (SQ at 1 byte per dimension, about 6 GiB, plus graphs) would exceed Lance's default 6 GiB index cache. It would land between the two candidates; I skipped it for time.

**Limitations:**
- All results are on synthetic data (3).
- The machine was shared with other sessions; 16-client runs especially are noisy. During stage 3 (04:14-05:22 UTC: the Lance variant, filter-only, 1M and 2560 runs), other sessions' `rustc` builds and Windows Defender scanning pushed total CPU to 100%. Those latencies are inflated: the same settings measured 1.5-1.8x slower than in stage 2. The main tables (sections 1, 5.2, 5.3) come from stages 1-2 (02:42-04:14 UTC), which were not checked for contention.
- Not tested: usearch M32 with efc 256; IVF_HNSW_SQ; true cold start after a reboot; k=10 latency; f16 HNSW at 6M.
- Cold start is approximate (5.5).
- Projections in 7 are arithmetic and estimates, not AI PC measurements.

**Re-measure in Phase 4 on real Qwen3-Embedding vectors (and the bake-off winner):**
1. The section-3 statistics on real chunks, to calibrate how hard HNSW really is. The ef needed for 0.95/0.95 is the number that moves most.
2. HNSW i8 M32 recall@10 and recall@100 versus ef (512-2048), and M32 with efc 256.
3. Recall after MRL truncation to 512 and 256 dimensions (not testable synthetically).
4. Filtered-search policy on real field and licence distributions: per-field index or exact filtered scan versus the HNSW predicate. Measure p95 for queries outside the field.
5. One run on the AI PC itself: usearch M32 ef 1024 and Lance IVF_PQ/RQ + refine, warm and after a reboot (true cold), with tantivy loaded alongside.
6. Lance refine latency when the f16 column is not cached (NVMe-bound), since that is the realistic Lance state with tantivy competing for RAM.

**Design questions recorded for the lead (not resolved here):**
1. How libraryd executes filtered vector search:
   - an exact scan over the filtered subset when selectivity is at most about 10-20% (about 0.6 GB of i8 codes per 600k rows),
   - per-field sub-indexes,
   - or the HNSW predicate with a timeout and fallback.
2. Whether libraryd keeps an f16 store at all. Rescoring is unnecessary for 0.95 recall with i8 at 1024 dimensions; it would serve exact scoring for fusion and reranking.
3. Where usearch index builds happen (in-process versus a separate builder process), given the 1.4x commit overhead and the 6-minute build at 6M.

## 10. Reproduce

From `bench/vector-bench` (cargo is `C:\Users\sethy\.cargo\bin\cargo.exe`):

```
cargo build --release -j 6
pwsh run.ps1 "gen" "gt" "us-build --kind i8 --m 32" "us-bench --kind i8 --m 32 --threads 12 --recall" ...
python summarize.py            # markdown tables from the JSONL (stdlib only)
```

Commands: `tune`, `gen`, `gt`, `us-build`, `us-bench`, `lance-ingest`, `lance-build`, `lance-bench`, all taking `--key value` arguments. Every step skips work whose output already exists and verifies:
- `gen` checks the parameters and file sizes, plus the CRC with `--verify`.
- `gt` checks its manifest.
- Builds check their manifests and file sizes, and for Lance the dataset version that must hold exactly one vector index.
- Benches check for their `bench_done` record in the JSONL.

`pipeline.log` and the per-step logs are in `vault-synth\logs`.

`vault-synth` contents: `d1024` (12.4 GB: base, queries, metadata, ground truth), `lance\d1024` (dataset plus all index versions), `usearch\d1024` (index files), plus the 1M datasets from 5.6. Total at the end: see appendix B.

## Appendix A: full tables

Generated by `python summarize.py [--data ...]` from the JSONL. Rows with ef=0 are exact search over the i8 codes (the quantisation ceiling). Rows with only T12 values come from the diagnostic run that preceded the store pre-read (rescoring hit the disk), from the b1 12-thread run, or from 1M runs (12 threads only). "filter-only" rows are the in-field re-runs from 5.6(b).

### A.1 6M x 1024

#### Builds

| candidate | index | build s | threads | file/index GiB | RAM (usearch memory_usage) GiB | peak WS GiB |
|---|---|---|---|---|---|---|
| B | us_i8_m16_efc128 | 384 | 12 | 6.55 | 10.03 | 7.71 |
| A | ingest 6000000 rows f16 | 10 (+1 bitmap) | 12 | 11.47 data | - | 1.07 |
| A | ivf_rq_p2450_b4 | 70 | 12 | 3.00 | - | 1.60 |
| A | ivf_pq_p2450_m128 | 334 | 12 | 0.75 | - | 1.43 |
| B | us_i8_m32_efc128 | 365 | 12 | 7.27 | 10.03 | 8.43 |
| B | us_b1_m16_efc128 | 149 | 12 | 1.55 | 3.03 | 2.67 |
| A | ivf_rq_p4900_b4 | 106 | 12 | 3.00 | - | 1.69 |
| A | ivf_rq_p2450_b1 | 82 | 12 | 0.81 | - | 1.44 |
| A | ivf_rq_p9800_b4 | 127 | 12 | 3.01 | - | 1.67 |

#### Query points (k=100; recall vs exact top-100 of 1000 queries)

| index | metric | params | R@10 | R@100 | R@10 centre q | T6 p50 | T6 p95 | T6 p99 | T6 batch4 p95 | T12 p95 | T6 qps c8 | T6 qps c16 | T12 qps c16 | peak WS GiB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ivf_pq_p2450_m128 | native | nprobes=10 refine=- | 0.672 | 0.742 | 0.376 | 5.56 | 6.96 | 8.08 | 10.50 | 6.34 | 887 | 865 | 1272 | 0.88 |
| ivf_pq_p2450_m128 | native | nprobes=160 refine=- | 0.682 | 0.757 | 0.384 | 56.87 | 61.29 | 63.50 | 64.56 | 64.25 | 105 | 101 | 133 | 0.89 |
| ivf_pq_p2450_m128 | native | nprobes=20 refine=- | 0.678 | 0.750 | 0.383 | 9.31 | 11.67 | 12.98 | 13.35 | 10.45 | 513 | 585 | 843 | 0.88 |
| ivf_pq_p2450_m128 | native | nprobes=20 refine=2 | 0.970 | 0.912 | 0.927 | 10.54 | 12.37 | 13.70 | 14.45 | 39.18 | 434 | 431 | 485 | 0.89 |
| ivf_pq_p2450_m128 | native | nprobes=20 refine=5 | 0.977 | 0.958 | 0.969 | 13.82 | 15.88 | 16.51 | 20.09 | 45.16 | 297 | 306 | 405 | 0.91 |
| ivf_pq_p2450_m128 | native | nprobes=40 refine=- | 0.681 | 0.754 | 0.384 | 15.39 | 17.40 | 19.29 | 18.30 | 18.96 | 365 | 347 | 475 | 0.88 |
| ivf_pq_p2450_m128 | native | nprobes=40 refine=2 | 0.983 | 0.922 | 0.937 | 17.14 | 19.62 | 21.17 | 22.18 | 21.63 | 293 | 295 | 375 | 0.90 |
| ivf_pq_p2450_m128 | native | nprobes=40 refine=5 | 0.989 | 0.971 | 0.980 | 20.85 | 23.90 | 25.91 | 28.87 | 25.90 | 206 | 212 | 342 | 0.91 |
| ivf_pq_p2450_m128 | native | nprobes=80 refine=- | 0.682 | 0.757 | 0.384 | 29.53 | 33.05 | 35.32 | 33.46 | 34.89 | 198 | 199 | 247 | 0.88 |
| ivf_pq_p2450_m128 | native | nprobes=80 refine=2 | 0.989 | 0.928 | 0.939 | 31.35 | 34.34 | 35.62 | 38.40 | 36.15 | 174 | 172 | 219 | 0.90 |
| ivf_pq_p2450_m128 | native | nprobes=80 refine=5 | 0.996 | 0.979 | 0.986 | 33.97 | 36.96 | 38.75 | 42.59 | 44.26 | 144 | 151 | 187 | 0.91 |
| ivf_rq_p2450_b1 | native | nprobes=20 refine=- | 0.736 | 0.817 | 0.614 | - | - | - | - | 21.53 | - | - | 432 | 0.97 |
| ivf_rq_p2450_b1 | native | nprobes=20 refine=5 | 0.983 | 0.981 | 0.990 | - | - | - | - | 35.69 | - | - | 196 | 1.00 |
| ivf_rq_p2450_b1 | native | nprobes=40 refine=- | 0.740 | 0.822 | 0.615 | - | - | - | - | 33.66 | - | - | 273 | 0.98 |
| ivf_rq_p2450_b1 | native | nprobes=40 refine=10 | 0.992 | 0.992 | 0.995 | - | - | - | - | 58.78 | - | - | 150 | 1.03 |
| ivf_rq_p2450_b1 | native | nprobes=40 refine=5 | 0.992 | 0.991 | 0.995 | - | - | - | - | 49.21 | - | - | 103 | 1.00 |
| ivf_rq_p2450_b1 | native | nprobes=80 refine=- | 0.742 | 0.824 | 0.615 | - | - | - | - | 73.72 | - | - | 109 | 0.98 |
| ivf_rq_p2450_b1 | native | nprobes=80 refine=10 | 0.996 | 0.996 | 0.998 | - | - | - | - | 98.26 | - | - | 81 | 1.03 |
| ivf_rq_p2450_b1 | native | nprobes=80 refine=5 | 0.996 | 0.996 | 0.997 | - | - | - | - | 75.06 | - | - | 79 | 1.00 |
| ivf_rq_p2450_b4 | native | nprobes=10 refine=- | 0.890 | 0.904 | 0.837 | 7.58 | 16.00 | 18.87 | 17.77 | 12.17 | 617 | 578 | 907 | 3.16 |
| ivf_rq_p2450_b4 | native | nprobes=160 refine=- | 0.950 | 0.970 | 0.915 | 75.81 | 89.81 | 97.65 | 107.31 | 78.32 | 72 | 70 | 88 | 3.18 |
| ivf_rq_p2450_b4 | native | nprobes=20 refine=- | 0.907 | 0.922 | 0.861 | 12.84 | 24.87 | 31.81 | 26.31 | 17.50 | 439 | 452 | 574 | 3.17 |
| ivf_rq_p2450_b4 | native | nprobes=20 refine=2 | 0.946 | 0.943 | 0.928 | 14.72 | 25.54 | 29.18 | 29.18 | 30.97 | 304 | 309 | 433 | 3.19 |
| ivf_rq_p2450_b4 | native | nprobes=20 refine=5 | 0.946 | 0.943 | 0.928 | 15.70 | 25.34 | 28.97 | 28.80 | 42.13 | 282 | 284 | 353 | 3.20 |
| ivf_rq_p2450_b4 | native | nprobes=40 refine=- | 0.931 | 0.947 | 0.895 | 22.51 | 35.80 | 42.43 | 35.99 | 28.37 | 279 | 255 | 315 | 3.17 |
| ivf_rq_p2450_b4 | native | nprobes=40 refine=2 | 0.973 | 0.970 | 0.965 | 24.08 | 34.48 | 42.19 | 42.67 | 31.55 | 231 | 232 | 286 | 3.19 |
| ivf_rq_p2450_b4 | native | nprobes=40 refine=5 | 0.973 | 0.970 | 0.965 | 25.69 | 37.44 | 43.00 | 44.80 | 42.72 | 196 | 198 | 237 | 3.20 |
| ivf_rq_p2450_b4 | native | nprobes=80 refine=- | 0.946 | 0.965 | 0.903 | 43.00 | 56.68 | 61.70 | 58.85 | 49.30 | 135 | 142 | 165 | 3.18 |
| ivf_rq_p2450_b4 | native | nprobes=80 refine=2 | 0.992 | 0.990 | 0.980 | 44.46 | 58.51 | 63.47 | 61.36 | 52.26 | 132 | 134 | 150 | 3.19 |
| ivf_rq_p2450_b4 | native | nprobes=80 refine=5 | 0.992 | 0.990 | 0.980 | 47.68 | 62.22 | 71.12 | 68.18 | 64.81 | 118 | 120 | 137 | 3.20 |
| ivf_rq_p4900_b4 | native | nprobes=10 refine=- | 0.863 | 0.875 | 0.862 | 6.28 | 18.14 | 25.31 | 20.94 | 16.84 | 665 | 611 | 874 | 3.20 |
| ivf_rq_p4900_b4 | native | nprobes=160 refine=- | 0.951 | 0.971 | 0.925 | 70.48 | 95.58 | 117.83 | 147.43 | 111.21 | 59 | 49 | 86 | 3.23 |
| ivf_rq_p4900_b4 | native | nprobes=20 refine=- | 0.894 | 0.909 | 0.880 | 9.37 | 20.88 | 27.36 | 27.27 | 23.17 | 389 | 386 | 584 | 3.21 |
| ivf_rq_p4900_b4 | native | nprobes=20 refine=2 | 0.933 | 0.928 | 0.937 | 12.39 | 28.02 | 35.51 | 66.89 | 28.73 | 224 | 260 | 396 | 3.23 |
| ivf_rq_p4900_b4 | native | nprobes=20 refine=5 | 0.933 | 0.928 | 0.937 | 17.29 | 36.31 | 51.05 | 41.72 | 34.41 | 209 | 197 | 316 | 3.24 |
| ivf_rq_p4900_b4 | native | nprobes=40 refine=- | 0.923 | 0.939 | 0.894 | 28.72 | 62.95 | 88.15 | 70.62 | 37.16 | 120 | 183 | 278 | 3.22 |
| ivf_rq_p4900_b4 | native | nprobes=40 refine=2 | 0.964 | 0.960 | 0.954 | 28.81 | 56.41 | 93.86 | 57.03 | 42.88 | 161 | 132 | 239 | 3.23 |
| ivf_rq_p4900_b4 | native | nprobes=40 refine=5 | 0.964 | 0.960 | 0.954 | 32.57 | 53.02 | 61.00 | 79.35 | 54.00 | 142 | 135 | 185 | 3.24 |
| ivf_rq_p4900_b4 | native | nprobes=80 refine=- | 0.945 | 0.963 | 0.908 | 53.19 | 82.18 | 112.31 | 92.93 | 58.13 | 94 | 94 | 175 | 3.23 |
| ivf_rq_p4900_b4 | native | nprobes=80 refine=2 | 0.990 | 0.987 | 0.974 | 54.00 | 85.27 | 105.21 | 93.87 | 63.82 | 107 | 85 | 129 | 3.23 |
| ivf_rq_p4900_b4 | native | nprobes=80 refine=5 | 0.990 | 0.987 | 0.974 | 54.67 | 80.57 | 99.11 | 81.11 | 73.04 | 91 | 82 | 133 | 3.24 |
| ivf_rq_p9800_b4 | native | nprobes=10 refine=- | 0.825 | 0.830 | 0.713 | - | - | - | - | 26.79 | - | - | 437 | 3.18 |
| ivf_rq_p9800_b4 | native | nprobes=160 refine=- | 0.952 | 0.970 | 0.916 | - | - | - | - | 119.72 | - | - | 73 | 3.32 |
| ivf_rq_p9800_b4 | native | nprobes=20 refine=- | 0.880 | 0.893 | 0.757 | - | - | - | - | 51.81 | - | - | 265 | 3.24 |
| ivf_rq_p9800_b4 | native | nprobes=20 refine=2 | 0.914 | 0.910 | 0.812 | - | - | - | - | 52.98 | - | - | 195 | 3.32 |
| ivf_rq_p9800_b4 | native | nprobes=20 refine=5 | 0.914 | 0.910 | 0.812 | - | - | - | - | 194.59 | - | - | 132 | 3.33 |
| ivf_rq_p9800_b4 | native | nprobes=40 refine=- | 0.925 | 0.940 | 0.813 | - | - | - | - | 71.38 | - | - | 112 | 3.28 |
| ivf_rq_p9800_b4 | native | nprobes=40 refine=2 | 0.965 | 0.962 | 0.877 | - | - | - | - | 62.82 | - | - | 82 | 3.32 |
| ivf_rq_p9800_b4 | native | nprobes=40 refine=5 | 0.965 | 0.962 | 0.877 | - | - | - | - | 76.80 | - | - | 147 | 3.33 |
| ivf_rq_p9800_b4 | native | nprobes=80 refine=- | 0.939 | 0.956 | 0.844 | - | - | - | - | 103.71 | - | - | 82 | 3.30 |
| ivf_rq_p9800_b4 | native | nprobes=80 refine=2 | 0.981 | 0.980 | 0.911 | - | - | - | - | 98.43 | - | - | 97 | 3.32 |
| ivf_rq_p9800_b4 | native | nprobes=80 refine=5 | 0.981 | 0.980 | 0.911 | - | - | - | - | 157.52 | - | - | 99 | 3.33 |
| us_b1_m16_efc128 | native | ef=1024 cand=1000 rescore | 0.906 | 0.893 | 0.953 | 1.94 | 3.60 | 4.57 | 4.59 | 3.89 | 2766 | 2683 | 3750 | 12.99 |
| us_b1_m16_efc128 | native | ef=2048 cand=1000 rescore | 0.940 | 0.929 | 0.954 | 3.44 | 6.66 | 8.99 | 8.45 | 8.27 | 1477 | 1480 | 2131 | 13.02 |
| us_b1_m16_efc128 | native | ef=2048 cand=2048 rescore | 0.943 | 0.939 | 0.972 | 5.02 | 9.03 | 11.17 | 10.20 | 8.97 | 1222 | 1249 | 1848 | 13.17 |
| us_b1_m16_efc128 | native | ef=512 cand=512 rescore | 0.846 | 0.819 | 0.901 | 1.49 | 2.42 | 2.91 | 2.90 | 2.51 | 4655 | 4640 | 5759 | 11.30 |
| us_i8_m16_efc128 | native | ef=0 cand=100 no-rescore | 0.994 | 0.996 | 0.988 | - | - | - | - | 305.01 | - | - | 12 | 8.20 |
| us_i8_m16_efc128 | native | ef=0 cand=256 rescore | 1.000 | 1.000 | 1.000 | - | - | - | - | 421.97 | - | - | 9 | 9.06 |
| us_i8_m16_efc128 | native | ef=1024 cand=100 no-rescore | 0.937 | 0.936 | 0.980 | 2.32 | 4.70 | 6.24 | 6.11 | 4.67 | 2072 | 2171 | 3039 | 7.48 |
| us_i8_m16_efc128 | native | ef=1024 cand=1024 rescore | 0.942 | 0.938 | 0.990 | - | - | - | - | 48.00 | - | - | 237 | 11.38 |
| us_i8_m16_efc128 | native | ef=1024 cand=200 rescore | 0.942 | 0.938 | 0.990 | 2.28 | 4.56 | 5.81 | 6.57 | 4.82 | 1971 | 1978 | 2935 | 12.35 |
| us_i8_m16_efc128 | native | ef=1536 cand=100 no-rescore | 0.958 | 0.958 | 0.981 | 3.26 | 6.46 | 8.51 | 8.83 | 6.39 | 1499 | 1429 | 2134 | 7.48 |
| us_i8_m16_efc128 | native | ef=2048 cand=100 no-rescore | 0.969 | 0.970 | 0.982 | 4.26 | 8.39 | 10.78 | 11.05 | 8.60 | 1212 | 1192 | 1722 | 7.48 |
| us_i8_m16_efc128 | native | ef=2048 cand=200 rescore | 0.974 | 0.972 | 0.993 | 4.38 | 8.65 | 11.46 | 12.00 | 8.39 | 994 | 1079 | 1642 | 12.45 |
| us_i8_m16_efc128 | native | ef=2048 cand=2048 rescore | 0.974 | 0.972 | 0.993 | - | - | - | - | 63.87 | - | - | 186 | 13.45 |
| us_i8_m16_efc128 | native | ef=256 cand=100 no-rescore | 0.840 | 0.828 | 0.952 | 0.90 | 1.72 | 2.37 | 2.34 | 1.68 | 5707 | 5871 | 8095 | 7.48 |
| us_i8_m16_efc128 | native | ef=512 cand=100 no-rescore | 0.903 | 0.899 | 0.969 | 1.32 | 2.77 | 3.54 | 3.60 | 2.94 | 3569 | 3623 | 5118 | 7.48 |
| us_i8_m16_efc128 | native | ef=512 cand=200 rescore | 0.908 | 0.900 | 0.978 | 1.62 | 3.07 | 3.79 | 3.88 | 3.13 | 3407 | 3278 | 4661 | 12.17 |
| us_i8_m16_efc128 | native | ef=512 cand=512 rescore | 0.908 | 0.900 | 0.978 | - | - | - | - | 34.27 | - | - | 236 | 9.96 |
| us_i8_m32_efc128 | native | ef=1024 cand=100 no-rescore | 0.963 | 0.964 | 0.977 | 2.40 | 5.09 | 7.03 | 7.30 | 5.57 | 1876 | 1828 | 2807 | 8.19 |
| us_i8_m32_efc128 | native | ef=1024 cand=200 rescore | 0.969 | 0.967 | 0.988 | 2.65 | 5.69 | 7.42 | 8.15 | 5.92 | 1765 | 1787 | 2924 | 13.01 |
| us_i8_m32_efc128 | native | ef=1536 cand=100 no-rescore | 0.972 | 0.974 | 0.978 | 3.57 | 7.27 | 9.76 | 10.24 | 7.19 | 1327 | 1263 | 2124 | 8.19 |
| us_i8_m32_efc128 | native | ef=2048 cand=100 no-rescore | 0.978 | 0.979 | 0.979 | 5.07 | 9.85 | 13.26 | 12.53 | 9.69 | 956 | 961 | 1470 | 8.19 |
| us_i8_m32_efc128 | native | ef=2048 cand=200 rescore | 0.984 | 0.983 | 0.990 | 5.28 | 10.47 | 14.61 | 12.79 | 8.76 | 1025 | 1030 | 1544 | 13.01 |
| us_i8_m32_efc128 | native | ef=256 cand=100 no-rescore | 0.902 | 0.893 | 0.960 | 0.84 | 1.80 | 2.25 | 2.27 | 2.19 | 5485 | 4814 | 7168 | 8.19 |
| us_i8_m32_efc128 | native | ef=512 cand=100 no-rescore | 0.936 | 0.935 | 0.973 | 1.54 | 3.31 | 4.50 | 4.01 | 2.90 | 3185 | 3038 | 4677 | 8.19 |
| us_i8_m32_efc128 | native | ef=512 cand=200 rescore | 0.941 | 0.937 | 0.984 | 1.95 | 3.96 | 4.93 | 4.44 | 3.45 | 2648 | 2801 | 4444 | 12.90 |

#### Cheapest point per index meeting R@10 >= 0.95 and R@100 >= 0.95 (by T6 single-query p95)

| index | metric | params | R@10 | R@100 | T6 p95 ms |
|---|---|---|---|---|---|
| us_i8_m32_efc128 | native | ef=1024 cand=100 no-rescore | 0.963 | 0.964 | 5.09 |
| us_i8_m16_efc128 | native | ef=1536 cand=100 no-rescore | 0.958 | 0.958 | 6.46 |
| ivf_pq_p2450_m128 | native | nprobes=20 refine=5 | 0.977 | 0.958 | 15.88 |
| ivf_rq_p2450_b4 | native | nprobes=40 refine=2 | 0.973 | 0.970 | 34.48 |
| ivf_rq_p4900_b4 | native | nprobes=40 refine=5 | 0.964 | 0.960 | 53.02 |

#### Filtered search (~10% of rows kept)

| index | filter | params | threads | run | R@10 | R@100 | R@10 q in field | R@10 q outside | p50 ms (mixed q) | p95 ms (mixed q) | p50 ms (q in field) | p95 ms (q in field) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ivf_pq_p2450_m128 | field | nprobes=20 refine=- | 6 | main | - | - | - | - | 7.67 | 15.02 | - | - |
| ivf_pq_p2450_m128 | field | nprobes=20 refine=- | 6 | filter-only | - | - | - | - | 7.61 | 14.61 | 13.83 | 16.36 |
| ivf_pq_p2450_m128 | field | nprobes=20 refine=- | 12 | main | 0.069 | 0.077 | 0.707 | 0.002 | 8.05 | 15.40 | - | - |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=- | 6 | main | - | - | - | - | 13.89 | 24.38 | - | - |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=- | 6 | filter-only | - | - | - | - | 13.54 | 22.81 | 22.71 | 25.66 |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=- | 12 | main | 0.071 | 0.081 | 0.707 | 0.004 | 15.13 | 25.98 | - | - |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=2 | 6 | main | - | - | - | - | 15.62 | 26.15 | - | - |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=2 | 6 | filter-only | - | - | - | - | 17.23 | 34.28 | 34.63 | 48.13 |
| ivf_pq_p2450_m128 | field | nprobes=40 refine=2 | 12 | main | 0.112 | 0.102 | 0.989 | 0.020 | 18.83 | 32.92 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=20 refine=- | 6 | main | - | - | - | - | 10.23 | 12.32 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=20 refine=- | 6 | filter-only | - | - | - | - | 9.81 | 12.16 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=20 refine=- | 12 | main | 0.736 | 0.778 | 0.762 | 0.734 | 10.63 | 12.93 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=- | 6 | main | - | - | - | - | 18.15 | 21.13 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=- | 6 | filter-only | - | - | - | - | 17.24 | 20.28 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=- | 12 | main | 0.741 | 0.783 | 0.763 | 0.739 | 22.70 | 33.06 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=2 | 6 | main | - | - | - | - | 20.36 | 23.20 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=2 | 6 | filter-only | - | - | - | - | 49.34 | 191.31 | - | - |
| ivf_pq_p2450_m128 | rattr | nprobes=40 refine=2 | 12 | main | 0.984 | 0.909 | 0.993 | 0.984 | 36.63 | 55.40 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=20 refine=- | 6 | main | - | - | - | - | 13.43 | 23.46 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=20 refine=- | 6 | filter-only | - | - | - | - | 15.14 | 31.15 | 13.44 | 21.82 |
| ivf_rq_p2450_b4 | field | nprobes=20 refine=- | 12 | main | 0.106 | 0.105 | 0.946 | 0.018 | 12.84 | 21.41 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=- | 6 | main | - | - | - | - | 23.80 | 35.54 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=- | 6 | filter-only | - | - | - | - | 24.38 | 36.40 | 23.93 | 34.69 |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=- | 12 | main | 0.151 | 0.147 | 0.952 | 0.067 | 25.63 | 39.97 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=2 | 6 | main | - | - | - | - | 25.35 | 37.42 | - | - |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=2 | 6 | filter-only | - | - | - | - | 46.45 | 87.15 | 58.16 | 83.72 |
| ivf_rq_p2450_b4 | field | nprobes=40 refine=2 | 12 | main | 0.159 | 0.150 | 0.991 | 0.072 | 26.72 | 40.33 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=20 refine=- | 6 | main | - | - | - | - | 11.91 | 22.85 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=20 refine=- | 6 | filter-only | - | - | - | - | 13.32 | 23.63 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=20 refine=- | 12 | main | 0.919 | 0.899 | 0.954 | 0.915 | 12.29 | 21.53 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=- | 6 | main | - | - | - | - | 23.65 | 36.23 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=- | 6 | filter-only | - | - | - | - | 25.27 | 40.53 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=- | 12 | main | 0.944 | 0.930 | 0.963 | 0.942 | 26.83 | 40.11 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=2 | 6 | main | - | - | - | - | 26.63 | 40.81 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=2 | 6 | filter-only | - | - | - | - | 59.38 | 93.95 | - | - |
| ivf_rq_p2450_b4 | rattr | nprobes=40 refine=2 | 12 | main | 0.973 | 0.941 | 0.987 | 0.972 | 29.36 | 40.76 | - | - |
| ivf_rq_p4900_b4 | field | nprobes=20 refine=- | 6 | main | - | - | - | - | 13.10 | 34.12 | 9.40 | 26.53 |
| ivf_rq_p4900_b4 | field | nprobes=20 refine=- | 12 | main | 0.131 | 0.131 | 0.797 | 0.061 | 13.49 | 32.44 | 7.63 | 21.05 |
| ivf_rq_p4900_b4 | field | nprobes=40 refine=- | 6 | main | - | - | - | - | 30.93 | 51.27 | 26.09 | 50.42 |
| ivf_rq_p4900_b4 | field | nprobes=40 refine=- | 12 | main | 0.194 | 0.191 | 0.904 | 0.119 | 27.49 | 52.12 | 24.60 | 45.63 |
| ivf_rq_p4900_b4 | field | nprobes=40 refine=2 | 6 | main | - | - | - | - | 33.48 | 59.63 | 28.57 | 49.42 |
| ivf_rq_p4900_b4 | field | nprobes=40 refine=2 | 12 | main | 0.201 | 0.194 | 0.936 | 0.123 | 30.67 | 52.36 | 24.37 | 40.25 |
| ivf_rq_p4900_b4 | rattr | nprobes=20 refine=- | 6 | main | - | - | - | - | 13.15 | 32.05 | - | - |
| ivf_rq_p4900_b4 | rattr | nprobes=20 refine=- | 12 | main | 0.901 | 0.873 | 0.814 | 0.910 | 10.58 | 26.22 | - | - |
| ivf_rq_p4900_b4 | rattr | nprobes=40 refine=- | 6 | main | - | - | - | - | 28.16 | 53.95 | - | - |
| ivf_rq_p4900_b4 | rattr | nprobes=40 refine=- | 12 | main | 0.931 | 0.915 | 0.908 | 0.934 | 23.86 | 39.85 | - | - |
| ivf_rq_p4900_b4 | rattr | nprobes=40 refine=2 | 6 | main | - | - | - | - | 36.25 | 67.75 | - | - |
| ivf_rq_p4900_b4 | rattr | nprobes=40 refine=2 | 12 | main | 0.960 | 0.924 | 0.922 | 0.964 | 27.69 | 44.65 | - | - |
| us_b1_m16_efc128 | field | ef=1024 cand=1024 rescore | 6 | main | - | - | - | - | 120.43 | 283.56 | - | - |
| us_b1_m16_efc128 | field | ef=1024 cand=1024 rescore | 6 | filter-only | - | - | - | - | 170.62 | 468.29 | 3.72 | 7.58 |
| us_b1_m16_efc128 | field | ef=1024 cand=1024 rescore | 12 | main | 0.470 | 0.373 | 0.976 | 0.417 | 117.40 | 291.68 | - | - |
| us_b1_m16_efc128 | field | ef=2048 cand=2048 rescore | 6 | main | - | - | - | - | 148.46 | 359.32 | - | - |
| us_b1_m16_efc128 | field | ef=2048 cand=2048 rescore | 6 | filter-only | - | - | - | - | 206.16 | 511.74 | 5.24 | 9.33 |
| us_b1_m16_efc128 | field | ef=2048 cand=2048 rescore | 12 | main | 0.582 | 0.476 | 0.978 | 0.540 | 154.27 | 376.63 | - | - |
| us_b1_m16_efc128 | rattr | ef=1024 cand=1024 rescore | 6 | main | - | - | - | - | 12.02 | 18.35 | - | - |
| us_b1_m16_efc128 | rattr | ef=1024 cand=1024 rescore | 6 | filter-only | - | - | - | - | 17.62 | 28.16 | - | - |
| us_b1_m16_efc128 | rattr | ef=1024 cand=1024 rescore | 12 | main | 0.963 | 0.942 | 0.977 | 0.961 | 11.55 | 17.86 | - | - |
| us_b1_m16_efc128 | rattr | ef=2048 cand=2048 rescore | 6 | main | - | - | - | - | 23.74 | 32.29 | - | - |
| us_b1_m16_efc128 | rattr | ef=2048 cand=2048 rescore | 6 | filter-only | - | - | - | - | 31.35 | 44.26 | - | - |
| us_b1_m16_efc128 | rattr | ef=2048 cand=2048 rescore | 12 | main | 0.966 | 0.953 | 0.978 | 0.964 | 26.14 | 36.11 | - | - |
| us_i8_m16_efc128 | field | ef=1024 cand=200 rescore | 6 | main | - | - | - | - | 207.52 | 455.38 | - | - |
| us_i8_m16_efc128 | field | ef=1024 cand=200 rescore | 6 | filter-only | - | - | - | - | 306.18 | 999.34 | 4.54 | 11.76 |
| us_i8_m16_efc128 | field | ef=1024 cand=200 rescore | 12 | main | 0.801 | 0.778 | 0.965 | 0.784 | 213.05 | 467.69 | - | - |
| us_i8_m16_efc128 | field | ef=2048 cand=200 rescore | 6 | main | - | - | - | - | 255.52 | 541.27 | - | - |
| us_i8_m16_efc128 | field | ef=2048 cand=200 rescore | 6 | filter-only | - | - | - | - | 313.26 | 848.02 | 7.03 | 13.69 |
| us_i8_m16_efc128 | field | ef=2048 cand=200 rescore | 12 | main | 0.878 | 0.865 | 0.988 | 0.866 | 258.77 | 569.87 | - | - |
| us_i8_m16_efc128 | rattr | ef=1024 cand=200 rescore | 6 | main | - | - | - | - | 16.20 | 25.87 | - | - |
| us_i8_m16_efc128 | rattr | ef=1024 cand=200 rescore | 6 | filter-only | - | - | - | - | 29.40 | 80.54 | - | - |
| us_i8_m16_efc128 | rattr | ef=1024 cand=200 rescore | 12 | main | 0.988 | 0.987 | 0.988 | 0.988 | 16.46 | 25.77 | - | - |
| us_i8_m16_efc128 | rattr | ef=2048 cand=200 rescore | 6 | main | - | - | - | - | 34.67 | 47.77 | - | - |
| us_i8_m16_efc128 | rattr | ef=2048 cand=200 rescore | 6 | filter-only | - | - | - | - | 42.57 | 66.60 | - | - |
| us_i8_m16_efc128 | rattr | ef=2048 cand=200 rescore | 12 | main | 0.990 | 0.990 | 0.988 | 0.991 | 31.95 | 44.05 | - | - |
| us_i8_m32_efc128 | field | ef=1024 cand=200 rescore | 6 | main | - | - | - | - | 226.03 | 518.56 | - | - |
| us_i8_m32_efc128 | field | ef=1024 cand=200 rescore | 6 | filter-only | - | - | - | - | 515.50 | 2099.17 | 53.63 | 192.67 |
| us_i8_m32_efc128 | field | ef=1024 cand=200 rescore | 12 | main | 0.830 | 0.810 | 0.999 | 0.812 | 225.98 | 503.09 | - | - |
| us_i8_m32_efc128 | field | ef=2048 cand=200 rescore | 6 | main | - | - | - | - | 279.94 | 623.50 | - | - |
| us_i8_m32_efc128 | field | ef=2048 cand=200 rescore | 6 | filter-only | - | - | - | - | 424.29 | 1588.00 | 7.25 | 14.80 |
| us_i8_m32_efc128 | field | ef=2048 cand=200 rescore | 12 | main | 0.908 | 0.898 | 0.999 | 0.898 | 248.23 | 545.83 | - | - |
| us_i8_m32_efc128 | rattr | ef=1024 cand=200 rescore | 6 | main | - | - | - | - | 17.61 | 29.16 | - | - |
| us_i8_m32_efc128 | rattr | ef=1024 cand=200 rescore | 6 | filter-only | - | - | - | - | 121.54 | 424.95 | - | - |
| us_i8_m32_efc128 | rattr | ef=1024 cand=200 rescore | 12 | main | 0.994 | 0.993 | 0.997 | 0.994 | 19.16 | 52.79 | - | - |
| us_i8_m32_efc128 | rattr | ef=2048 cand=200 rescore | 6 | main | - | - | - | - | 36.15 | 51.40 | - | - |
| us_i8_m32_efc128 | rattr | ef=2048 cand=200 rescore | 6 | filter-only | - | - | - | - | 45.01 | 64.58 | - | - |
| us_i8_m32_efc128 | rattr | ef=2048 cand=200 rescore | 12 | main | 0.995 | 0.994 | 0.997 | 0.995 | 35.70 | 52.33 | - | - |

#### Cold start (approximate: fresh process, OS file cache not dropped)

| index | metric | threads | load/open s | first query ms | first-20 p50 ms | first-20 max ms |
|---|---|---|---|---|---|---|
| ivf_rq_p2450_b4 | native | 12 | 0.00 | 100.7 | 18.44 | 100.7 |
| ivf_rq_p2450_b4 | native | 6 | 0.00 | 60.6 | 18.32 | 60.6 |
| us_i8_m16_efc128 | native | 12 | 9.10 | 101.9 | 78.21 | 101.9 |
| us_i8_m16_efc128 | native | 12 | 9.82 | 7.1 | 4.75 | 8.0 |
| us_i8_m16_efc128 | native | 6 | 5.68 | 5.4 | 4.28 | 7.2 |
| us_i8_m32_efc128 | native | 12 | 9.02 | 11.3 | 5.27 | 11.3 |
| us_i8_m32_efc128 | native | 6 | 5.67 | 9.4 | 4.71 | 9.4 |
| ivf_pq_p2450_m128 | native | 12 | 0.05 | 170.0 | 19.38 | 170.0 |
| ivf_pq_p2450_m128 | native | 6 | 0.01 | 23.0 | 10.64 | 23.0 |
| us_b1_m16_efc128 | native | 12 | 1.55 | 8.2 | 5.31 | 8.4 |
| us_b1_m16_efc128 | native | 6 | 1.41 | 5.7 | 5.32 | 7.9 |
| ivf_rq_p4900_b4 | native | 12 | 0.00 | 110.4 | 24.04 | 110.4 |
| ivf_rq_p4900_b4 | native | 6 | 0.00 | 67.1 | 19.49 | 67.1 |
| ivf_rq_p2450_b1 | native | 12 | 0.00 | 29.3 | 12.09 | 29.3 |
| ivf_rq_p9800_b4 | native | 12 | 0.00 | 60.0 | 21.32 | 89.0 |
| us_i8_m16_efc128 | native | 6 | 14.19 | 8.3 | 4.61 | 10.0 |
| us_i8_m32_efc128 | native | 6 | 28.03 | 13.0 | 6.28 | 13.0 |
| us_b1_m16_efc128 | native | 6 | 5.08 | 8.1 | 6.60 | 11.5 |
| ivf_rq_p2450_b4 | native | 6 | 0.26 | 1143.8 | 169.99 | 1143.8 |
| ivf_pq_p2450_m128 | native | 6 | 0.01 | 139.9 | 22.85 | 139.9 |

### A.2 1M x 1024

#### Builds

| candidate | index | build s | threads | file/index GiB | RAM (usearch memory_usage) GiB | peak WS GiB |
|---|---|---|---|---|---|---|
| B | us_i8_m16_efc128 | 63 | 12 | 1.09 | 1.24 | 1.74 |
| B | us_i8auto_m16_efc128 | 52 | 12 | 1.09 | 1.24 | 1.93 |
| B | us_f16_m16_efc128 | 87 | 12 | 2.05 | 2.24 | 2.74 |
| A | ingest 1000000 rows f16 | 2 (+0 bitmap) | 12 | 1.91 data | - | 1.04 |
| A | ivf_rq_p1000_b4 | 14 | 12 | 0.50 | - | 1.56 |

#### Query points (k=100; recall vs exact top-100 of 1000 queries)

| index | metric | params | R@10 | R@100 | R@10 centre q | T6 p50 | T6 p95 | T6 p99 | T6 batch4 p95 | T12 p95 | T6 qps c8 | T6 qps c16 | T12 qps c16 | peak WS GiB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ivf_rq_p1000_b4 | native | nprobes=10 refine=- | 0.944 | 0.949 | 0.933 | - | - | - | - | 4.07 | - | - | 1606 | 0.62 |
| ivf_rq_p1000_b4 | native | nprobes=10 refine=2 | 0.975 | 0.964 | 0.977 | - | - | - | - | 9.39 | - | - | 696 | 0.64 |
| ivf_rq_p1000_b4 | native | nprobes=20 refine=- | 0.949 | 0.958 | 0.933 | - | - | - | - | 7.13 | - | - | 1099 | 0.63 |
| ivf_rq_p1000_b4 | native | nprobes=20 refine=2 | 0.981 | 0.973 | 0.977 | - | - | - | - | 10.15 | - | - | 827 | 0.64 |
| ivf_rq_p1000_b4 | native | nprobes=40 refine=- | 0.963 | 0.972 | 0.951 | - | - | - | - | 10.69 | - | - | 765 | 0.63 |
| ivf_rq_p1000_b4 | native | nprobes=40 refine=2 | 0.997 | 0.989 | 0.998 | - | - | - | - | 12.01 | - | - | 644 | 0.64 |
| us_f16_m16_efc128 | native | ef=1024 cand=100 no-rescore | 0.984 | 0.984 | 0.996 | - | - | - | - | 4.74 | - | - | 2626 | 2.22 |
| us_f16_m16_efc128 | native | ef=2048 cand=100 no-rescore | 0.990 | 0.990 | 0.996 | - | - | - | - | 21.78 | - | - | 1408 | 2.23 |
| us_f16_m16_efc128 | native | ef=256 cand=100 no-rescore | 0.944 | 0.940 | 0.988 | - | - | - | - | 1.07 | - | - | 8093 | 2.22 |
| us_f16_m16_efc128 | native | ef=512 cand=100 no-rescore | 0.966 | 0.965 | 0.996 | - | - | - | - | 2.37 | - | - | 3455 | 2.22 |
| us_i8_m16_efc128 | native | ef=1024 cand=100 no-rescore | 0.980 | 0.980 | 0.985 | - | - | - | - | 2.76 | - | - | 3778 | 1.49 |
| us_i8_m16_efc128 | native | ef=1024 cand=200 rescore | 0.985 | 0.982 | 0.996 | - | - | - | - | 3.75 | - | - | 2662 | 3.03 |
| us_i8_m16_efc128 | native | ef=2048 cand=100 no-rescore | 0.984 | 0.984 | 0.985 | - | - | - | - | 5.88 | - | - | 2011 | 1.49 |
| us_i8_m16_efc128 | native | ef=256 cand=100 no-rescore | 0.946 | 0.943 | 0.982 | - | - | - | - | 0.96 | - | - | 12123 | 1.49 |
| us_i8_m16_efc128 | native | ef=512 cand=100 no-rescore | 0.969 | 0.968 | 0.985 | - | - | - | - | 1.74 | - | - | 7434 | 1.49 |
| us_i8auto_m16_efc128 | native | ef=1024 cand=100 no-rescore | 0.924 | 0.947 | 0.872 | - | - | - | - | 3.87 | - | - | 3712 | 1.49 |
| us_i8auto_m16_efc128 | native | ef=1024 cand=200 rescore | 0.981 | 0.981 | 0.993 | - | - | - | - | 3.69 | - | - | 3052 | 3.07 |
| us_i8auto_m16_efc128 | native | ef=2048 cand=100 no-rescore | 0.932 | 0.956 | 0.872 | - | - | - | - | 5.92 | - | - | 1491 | 1.49 |
| us_i8auto_m16_efc128 | native | ef=256 cand=100 no-rescore | 0.890 | 0.910 | 0.872 | - | - | - | - | 1.11 | - | - | 10252 | 1.49 |
| us_i8auto_m16_efc128 | native | ef=512 cand=100 no-rescore | 0.909 | 0.930 | 0.872 | - | - | - | - | 2.81 | - | - | 5658 | 1.49 |

#### Cheapest point per index meeting R@10 >= 0.95 and R@100 >= 0.95 (by T6 single-query p95)

| index | metric | params | R@10 | R@100 | T6 p95 ms |
|---|---|---|---|---|---|

#### Filtered search (~10% of rows kept)

| index | filter | params | threads | run | R@10 | R@100 | R@10 q in field | R@10 q outside | p50 ms (mixed q) | p95 ms (mixed q) | p50 ms (q in field) | p95 ms (q in field) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|

#### Cold start (approximate: fresh process, OS file cache not dropped)

| index | metric | threads | load/open s | first query ms | first-20 p50 ms | first-20 max ms |
|---|---|---|---|---|---|---|
| us_i8_m16_efc128 | native | 12 | 0.76 | 2.0 | 1.91 | 4.5 |
| us_i8auto_m16_efc128 | native | 12 | 0.93 | 1.8 | 2.78 | 4.4 |
| us_f16_m16_efc128 | native | 12 | 1.52 | 2.2 | 2.42 | 4.1 |
| ivf_rq_p1000_b4 | native | 12 | 0.00 | 22.0 | 8.34 | 22.0 |

### A.3 1M x 2560

#### Builds

| candidate | index | build s | threads | file/index GiB | RAM (usearch memory_usage) GiB | peak WS GiB |
|---|---|---|---|---|---|---|
| B | us_i8_m16_efc128 | 81 | 12 | 2.52 | 4.24 | 3.99 |
| A | ingest 1000000 rows f16 | 7 (+1 bitmap) | 12 | 4.77 data | - | 2.54 |
| A | ivf_rq_p1000_b4 | 97 | 12 | 1.22 | - | 3.07 |

#### Query points (k=100; recall vs exact top-100 of 1000 queries)

| index | metric | params | R@10 | R@100 | R@10 centre q | T6 p50 | T6 p95 | T6 p99 | T6 batch4 p95 | T12 p95 | T6 qps c8 | T6 qps c16 | T12 qps c16 | peak WS GiB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ivf_rq_p1000_b4 | native | nprobes=10 refine=- | 0.964 | 0.968 | 0.959 | - | - | - | - | 11.34 | - | - | 494 | 1.42 |
| ivf_rq_p1000_b4 | native | nprobes=10 refine=2 | 0.985 | 0.979 | 0.995 | - | - | - | - | 30.12 | - | - | 182 | 1.44 |
| ivf_rq_p1000_b4 | native | nprobes=20 refine=- | 0.971 | 0.976 | 0.959 | - | - | - | - | 17.58 | - | - | 395 | 1.42 |
| ivf_rq_p1000_b4 | native | nprobes=20 refine=2 | 0.992 | 0.987 | 0.995 | - | - | - | - | 28.02 | - | - | 204 | 1.45 |
| ivf_rq_p1000_b4 | native | nprobes=40 refine=- | 0.974 | 0.982 | 0.961 | - | - | - | - | 36.38 | - | - | 151 | 1.42 |
| ivf_rq_p1000_b4 | native | nprobes=40 refine=2 | 0.996 | 0.993 | 0.997 | - | - | - | - | 37.19 | - | - | 125 | 1.45 |
| us_i8_m16_efc128 | native | ef=1024 cand=100 no-rescore | 0.980 | 0.982 | 0.983 | - | - | - | - | 6.58 | - | - | 1176 | 3.35 |
| us_i8_m16_efc128 | native | ef=1024 cand=200 rescore | 0.984 | 0.984 | 0.991 | - | - | - | - | 9.38 | - | - | 1235 | 5.96 |
| us_i8_m16_efc128 | native | ef=2048 cand=100 no-rescore | 0.988 | 0.990 | 0.983 | - | - | - | - | 12.32 | - | - | 619 | 3.35 |
| us_i8_m16_efc128 | native | ef=256 cand=100 no-rescore | 0.950 | 0.950 | 0.967 | - | - | - | - | 3.23 | - | - | 6222 | 3.35 |
| us_i8_m16_efc128 | native | ef=512 cand=100 no-rescore | 0.965 | 0.967 | 0.976 | - | - | - | - | 3.32 | - | - | 2755 | 3.35 |

#### Cheapest point per index meeting R@10 >= 0.95 and R@100 >= 0.95 (by T6 single-query p95)

| index | metric | params | R@10 | R@100 | T6 p95 ms |
|---|---|---|---|---|---|

#### Filtered search (~10% of rows kept)

| index | filter | params | threads | run | R@10 | R@100 | R@10 q in field | R@10 q outside | p50 ms (mixed q) | p95 ms (mixed q) | p50 ms (q in field) | p95 ms (q in field) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|

#### Cold start (approximate: fresh process, OS file cache not dropped)

| index | metric | threads | load/open s | first query ms | first-20 p50 ms | first-20 max ms |
|---|---|---|---|---|---|---|
| us_i8_m16_efc128 | native | 12 | 2.09 | 5.6 | 3.57 | 6.3 |
| ivf_rq_p1000_b4 | native | 12 | 0.01 | 107.8 | 22.91 | 107.8 |

## Appendix B: disk use of vault-synth at the end

| Folder | GB |
|---|---|
| d1024 (base 12.3 GB, queries, metadata, ground truth) | 12.36 |
| d1024n1m | 2.08 |
| d2560n1m | 5.18 |
| lance (6M dataset with 6 index versions, plus both 1M datasets) | 32.71 |
| usearch (6M i8 M16, i8 M32, b1; 1M i8, i8auto, f16; 1M x 2560 i8) | 23.75 |
| bin (copies of the benchmark binary used by run.ps1) | 0.76 |
| **total** | **76.83** (cap 80) |


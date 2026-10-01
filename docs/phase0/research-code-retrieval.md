# Research: retrieving and navigating source code for LLM agents (state of the art, 2026-10-01)

Worker report (Sonnet 5.5) for the lead engineer. Read-only research. The only file written is this one. Clones for
measurement are in `C:/Users/sethy/Projects/refs/rag/{aider,requests,networkx}` (shallow, never executed).

**Evidence tags.** M = measured by me today. D = documented by the cited source; D-self means the authors or vendor
measured their own method, D-ind means an independent party measured it. E = my estimate. U = I could not verify.

**Limits of this research (read first).**
- WebSearch was out of budget (200 of 200), so discovery used the arXiv API, the Hugging Face API, the GitHub API
  and the crates.io API plus WebFetch. A blog post or paper that is on none of those may be missing.
- WebFetch answers come from a small summarising model. Where I only saw an abstract, the number list says
  "abstract only". Primary-source code (Aider, Zoekt) I read directly.
- Dates in `[YYYY-MM-DD]` are arXiv submission, publication, or repository dates I saw.
- I found no ambiguity in the brief that blocks the work. Open decisions for the lead are in section 11.

---

## 0. Answers in brief

1. **Chunking.** Structure-aware chunking (cAST, 2025-06) beats fixed line windows, but the retrieval gain is small
   (+0.7 to +4.3 points Recall@5 on its benchmarks, D-self). I measured that fixed 60-line windows cut 19 to 27
   percent of ordinary functions in half on three Python repositories (M). Use an AST split-then-merge chunker, with
   every chunk snapped to whole lines so `{doc_id}#L{a}-L{b}` stays exact.
2. **Repo maps.** Aider's map (tree-sitter tags, file graph, personalised PageRank, binary search to a token budget)
   is still the reference. In a 2026 independent benchmark a vectorless Aider-style map gave the best result per
   8k tokens of context, ahead of Qwen3-Embedding-8B (D-ind). LLM-built knowledge graphs lost to deterministic
   AST graphs (2026-01, D-ind). Build the map deterministically and fit it to a budget at query time.
3. **Code intelligence.** SCIP is the living standard (v0.10.0, 2026-09-03; now hosted partly in a `scip-code` org).
   GitHub's stack-graphs is archived (M). SCIP indexers need a build or dependency environment, which the vault
   does not have for delivered folders (dependencies are excluded by rule), and they execute project tooling.
   Tree-sitter tags give definitions and name-matched references for 69 of 371 bundled grammars (M), which covers
   the mainstream languages and is enough for outline, define and a ranking graph. Make SCIP an opt-in later step.
4. **Search engines.** Zoekt's BM25F boosts symbol-definition and file-name matches by 5, uses constant IDF, and
   down-weights tests, generated and vendored files (D, source read). Sourcegraph reports about +20 percent on its
   own evaluation (D-self, 2025-04). Tantivy has no camelCase splitter (M); a small custom filter is needed.
5. **Agentic search versus embeddings.** Evidence is mixed and recent work says "use both". The reasons Anthropic gave
   for agentic search (index staleness, index sync cost, third-party vector store risk) do not apply to an immutable
   local vault. The vault's agent has no shell, so the vault tools must themselves be the agentic primitives.
6. **Embeddings.** One general embedder (Qwen3-Embedding-4B) is probably good enough for natural-language to code
   queries (E, supported by D-ind). Code-specialised models win on code-to-code and on CoIR averages. Several
   code models are non-commercial licences. Only `bge-code-v1` (Apache-2.0, 1.5B) and `nomic-embed-code`
   (Apache-2.0, 7B) are permissive, GGUF-available code embedders that I found.
7. **Linking.** Papers with Code is gone (redirects to Hugging Face trending papers, M); a frozen snapshot of 300,161
   paper-code links exists (CC-BY-SA-4.0, 2025-07-28, M). Software metadata is inconsistent (83.9 percent of 117
   projects have a core-field conflict, D-ind), so store links as typed edges with evidence flags. SWHID lines
   qualifiers use the same 1-based inclusive LF line convention as `#L{a}-L{b}` (D).
8. **Evaluation.** Use CoIR (ACL 2025) and MTEB-Code for embedders, CoREB, CLARC and ExecRetrieval as 2026
   diagnostics, SWE-bench Lite/Verified localisation (LocBench, SWE-Explore, ContextBench, Agent Retrieval Bench)
   for end-to-end, plus a vault-internal keyword-agnostic set, because public issue benchmarks are saturated with
   keywords (2026-04, D-ind).
9. **Recommended design** is in section 9 and the ordered increments in section 10.

---

## 1. Structure-aware chunking of code

### 1.1 cAST and what it measured

cAST: "Enhancing Code Retrieval-Augmented Generation with Structural Chunking via Abstract Syntax Tree",
arXiv 2506.15655 [2025-06-18, v2], code at github.com/yilinjz/astchunk (MIT; Python toolkit supporting Python, Java,
C# and TypeScript; M via GitHub API and README).

Algorithm (D-self): parse with tree-sitter, walk top-down, try to fit each node in one chunk, recursively split nodes
that exceed the limit, then greedily merge adjacent siblings up to the limit. Size is non-whitespace characters, not
lines (limit 2000; peak performance 2000 to 2500; 4000-token context, about five chunks).

Measured gains versus fixed-size line chunks (D-self, abstract plus tables via fetch):

| Benchmark | Metric | Gain |
|---|---|---|
| RepoEval | Recall@5 | +1.8 to +4.3 points across four retrievers (BGE-base, GIST-base, CodeSage-small-v2, Jina-v2-code) |
| RepoEval | Precision@5 | +1.2 to +3.3 |
| RepoEval | Pass@1 (StarCoder2-7B) | +4.2 to +5.6 |
| SWE-bench Lite | Recall@5 / Precision@5 | +0.7 to +1.1 / +0.5 to +1.4 |
| SWE-bench Lite | Pass@1 (Claude 3.7) | +2.6 to +2.7 |
| CrossCodeEval | identifier match EM | +2.7 to +3.9 |

Ablations (D-self): split-only without the merge step dropped nDCG from 71.1 to 53.5 (BGE retriever); 8000-token
context gave minimal gain over 4000; 3500 tokens degraded. The authors' stated limits: chunks are isolated (no
multi-level AST context), single view (no comments or pseudocode view), no dynamic information.

Read the gains honestly: on SWE-bench the Recall@5 gain is under 1.2 points. Chunking matters for generation
(precision correlates with Pass@1, the paper says) more than for recall.

### 1.2 Successors and neighbours (what I found)

I found no 2026 paper that supersedes cAST on chunking itself (E; discovery limited as stated above). Related work
that changes what a chunk is *for*:

| Work | Date | Finding | Tag |
|---|---|---|---|
| SpIDER (arXiv 2512.16956) | 2025-12-18 | Expand dense hits with graph neighbours. Containment edges: at least +13 percent relative Recall@20 (+0.05 to +0.12 absolute); adding call edges: at least +27 percent relative (+0.11 to +0.19). Python, Java, JS, TS. | D-self, abstract only |
| Retrieval-Oriented Code Representations (2607.11046) | 2026-07-13 | For file-level bug localisation, role-aware LLM summaries beat file-path representations by up to 40 percent Hit@5, and are 10.4 to 20.9 times smaller than raw source. | D-self, abstract only |
| Agent Retrieval Bench (2607.24882) | 2026-07-27 | Used 7,922,369 chunks over 391,932 files; mentions a legacy 8,000-character packing in a sensitivity analysis. | D-ind |
| Anthropic contextual retrieval (already in `references.md`) | 2024-09 | Prepend context to each chunk. For code, the equivalent is the scope chain and signature header. | D-self |

### 1.3 My measurement: how often line windows cut code

Method: Python stdlib `ast` over all `.py` files (tests included) of three shallow clones; no code executed.
A "small" function is at most 2000 non-whitespace characters (cAST's limit). A window cuts a function if its first
and last line fall in different fixed, non-overlapping windows.

| Repo (commit) | Files | Lines | Small functions cut by 30-line windows | 60-line | 100-line |
|---|---|---|---|---|---|
| requests (`611c6162`) | 37 | 12,069 | 35.4% | 19.0% | 10.9% |
| networkx (`92f497e2`) | 693 | 205,629 | 39.1% | 23.3% | 13.7% |
| aider (`5dc9490b`) | 147 | 38,428 | 47.9% | 27.2% | 16.2% |

Other measured facts that shape the design (all M, Python only, tests included):

| Fact | requests | networkx | aider |
|---|---|---|---|
| File length p50 / p90 / max (lines) | 129 / 1,014 / 3,095 | 162 / 712 / 2,994 | 129 / 658 / 2,486 |
| Share of bytes in files over 500 lines | 73.9% | 56.8% | 63.9% |
| Function size p50 / p90 (non-whitespace chars) | 245 / 756 | 277 / 1,470 | 379 / 1,278 |
| Functions over 2000 non-whitespace chars | 0.7% | 6.1% | 4.2% |
| Classes over 2000 non-whitespace chars | 17.7% | 33.1% | 38.6% |
| Mean file size | about 11 KB | about 10 KB | about 9 KB |

Consequences: most bytes sit in long files, so whole-file reads are expensive and an outline is needed; classes
usually exceed the chunk limit, so the merge step must split a class into header plus method groups; and a typical
file is about one 16 KiB trigram block (the Phase 0 design's proposed block size), so for code the block is nearly
a file.

### 1.4 Rust tooling for the chunker

- `tree-sitter` 0.27.0 and `tree-sitter-tags` 0.27.0 (2026-08-30, MIT, M via crates.io).
- `text-splitter` 0.33.0 (MIT, updated 2026-09-24) has a tree-sitter code splitter. The proposal already notes it
  cannot keep a block whole beyond the size limit. A purpose-written split-then-merge over tree-sitter nodes is
  small (E: 1 to 2 days with tests) and gives exact line control.
- `astchunk` 0.2.0 (updated 2026-04-01, 342 downloads) and `code-splitter` 0.1.5 (2024-09, 5.5k downloads) exist on
  crates.io. I did not evaluate either.
- `tree-sitter-language-pack` 1.20.0 (2026-09-14, MIT, 371 languages, "parsers downloaded on first use"). For an
  offline vault, vendor or prefetch the grammars and pin versions. Its README links the xberg-io organisation, which
  the proposal already pins carefully because xberg briefly changed licence; apply the same pin-and-vendor policy.

---

## 2. Repository maps, graphs, and fitting them to a token budget

### 2.1 Aider's repo map (primary source read, D)

Source: `aider/repomap.py` on main (repository pushed 2026-05-22, Apache-2.0); docs page and 2023-10-22 blog.
The blog gives no measured results (no numbers in it).

Mechanism:
1. Tree-sitter `tags.scm` queries per language produce definition and reference tags. Aider ships queries for about
   30 languages (M: directory listing of `aider/queries/tree-sitter-language-pack`).
2. Graph: one node per file. An edge runs from each referencing file to each defining file, per identifier.
3. Edge weight: base 1, times 10 if the identifier is mentioned in the conversation, times 10 for snake, kebab or
   camel identifiers of 8 or more characters, times 0.1 for a leading underscore, times 0.1 if defined in more than
   5 files, times 50 if the referencer is a file already in the chat, times sqrt(reference count) so heavy repetition
   does not dominate. A defined-but-unreferenced identifier gets a 0.1 self edge.
4. Personalised PageRank (NetworkX): personalisation mass `100 / number of files` to files in the chat, mentioned
   files, and files whose path components match mentioned identifiers.
5. Render: for the top ranked tags, print each file's definition lines with parent context and `⋮...` elisions.
6. Budget fit: binary search on how many ranked tags to render until the rendered map is within 15 percent of the
   token budget (starting guess budget/25 tags). Default budget `--map-tokens` is 1,024; Aider multiplies it by 8
   when no files are in the chat.

### 2.2 Independent 2026 evidence on repo maps

| Source | Date | Finding | Tag |
|---|---|---|---|
| Agent Retrieval Bench, arXiv 2607.24882 | 2026-07-27 | "RepoMap" (vectorless: path, symbol, reference and graph signals, inspired by Aider) scores BCY@8k (budgeted context yield at 8k tokens) 0.3788 versus Qwen3-Embedding-8B 0.3732 and 4B 0.3409. MRR: RepoMap 0.2158, Qwen3-4B 0.2379, Qwen3-8B 0.2336, BM25 0.1520. Recall@20: Qwen3-8B 0.7029, RepoMap 0.6333, Qwen3-4B 0.6306. 345 positive samples, 25 repositories. RRF fusion lifted MRR 0.2296 to 0.2713 on 287 samples. | D-ind |
| Formal architecture descriptors, 2604.13108 | 2026-04-11 | Giving the agent an architecture descriptor cut navigation steps 33 to 44 percent (24 tasks, Claude Sonnet 4.6, p=0.009); a 7,012-session field study found 52 percent lower behavioural variance. | D-self, small n |
| Reliable Graph-RAG for Codebases, 2601.08773 | 2026-01-13 | Deterministic tree-sitter graph beat LLM-extracted knowledge graphs on 15 architecture queries per repo (Java: Shopizer, ThingsBoard, OpenMRS): indexes in seconds, covered all files (the LLM graph missed 377 files on Shopizer), best correctness; vector-only worst on architectural queries. | D-ind, small |
| RepoGraph, ICLR 2025 (2410.14684) | 2024-10-03 | Line-level def/ref nodes, invoke/contain edges, k-hop ego graph. SWE-bench Lite resolve rate with versus without: Agentless 27.33 to 29.67, RAG 2.67 to 5.33, AutoCodeRover 19.00 to 21.33, SWE-agent 18.33 to 20.33; average relative gain 32.8 percent; about 1,419 nodes and 26,392 edges per repository; Python only; +4,947 to +20,446 tokens. | D-self |
| CodexGraph (2408.03910) | 2024-08-07 | Neo4j plus LLM-written Cypher. CrossCodeEval Lite EM 27.90, SWE-bench Lite Pass@1 22.96; the model sometimes loops failing to write an executable query; 22.16k tokens versus 10.74k for AutoCodeRover. Python only. | D-self |
| RepoAtlas (2609.16936) | 2026-09-15 | Evolving views over a code graph: +2.4 points resolution on SWE-bench Verified over the strongest multimodal-graph baseline, 5.8 percent fewer input tokens. | D-self, abstract only |
| CodeNib (2607.25431) | 2026-07-28 | An incremental code index "can complete every update without error and still return the wrong graph half the time"; static definitions match live language servers on 86 percent of requests (1.2 s); policies use 50 to 87 percent fewer tokens than grep-based ones; with LocAgent, 65 to 78 of 90 runs completed. | D-self, abstract only |
| Code Graph Model, 2505.16901 | 2025-05-22 | Graph-integrated open LLM for repository tasks. | abstract not read |

Reading: a simple deterministic reference graph is competitive with embeddings per unit of context, and the two are
complementary (the benchmark's own conclusion). The staleness problem CodeNib measures is about incremental updates;
the vault publishes immutable snapshots and rebuilds a repository's graph whole, so it does not arise.

### 2.3 Fitting a map to a token budget (patterns found)

1. Aider: rank tags, binary search count to budget (above).
2. SWE-Explore (2606.07297, 2026-06-05): returns a ranked list of code regions under a fixed line budget; judges
   coverage, ranking and context efficiency.
3. Agent Retrieval Bench: BCY@8k, yield per 8k tokens.
4. SemNav (2609.31176, 2026-09-25): "semantic cards" (compact, issue-conditioned entity summaries) cut working
   context 48.2 percent versus reading full source (D-self).
5. Formal descriptors: a static compact descriptor at the start of the task.

Recommendation: store a static generic map inside the repository document (so it is searchable and citable), and
compute a personalised map at query time from the symbol and reference tables with the Aider recipe, fitted to a
caller-supplied token budget. Count tokens with a calibrated estimate (about 3.5 bytes per token for code, E) and
the same 15 percent tolerance, unless the Qwen tokenizer is already linked in.

---

## 3. Precise code intelligence: SCIP, stack-graphs, LSP, tree-sitter tags

### 3.1 State of each (M unless noted)

| Technology | Status on 2026-10-01 | Notes |
|---|---|---|
| SCIP protocol | `scip-code/scip`, Apache-2.0, pushed 2026-09-29, v0.10.0 released 2026-09-03; Rust crate `scip` 0.10.0 (2,645,703 downloads). | Protobuf. The design doc says SCIP is a transmission format, not a query/storage format; compression of 10 to 20 percent typical (D). Sourcegraph removed LSIF support (D). Site says "The SCIP Authors" and has a governance page; I did not read it. |
| SCIP indexers | scip-typescript, scip-python, scip-clang, scip-dotnet, scip-ruby in the `sourcegraph` org; scip-java, scip-go, scip-rust, scip-dart under `scip-code`; scip-php and Dart under other owners; rust-analyzer emits SCIP. All pushed within the last month (latest 2026-09-11 to 2026-10-01), except scip-kotlin which is archived (Kotlin is covered by scip-java per the SCIP README). scip-python is MIT (bundles Pyright). | Languages per scip-code.org: C#/VB, C/C++, Dart, Go, Java/Scala/Kotlin, PHP, Python, Ruby, Rust, TypeScript/JavaScript. |
| stack-graphs (GitHub) | Archived; README says "no longer supported or updated by GitHub"; last push 2025-09-09. `tree-sitter-graph` last release 2024-12-11. | Do not depend on it. Superseded in practice by SCIP and by tags-based name matching. |
| LSP | Alive, per-query JSON-RPC. ABCoder reports per-symbol JSON-RPC as an indexing bottleneck and built a TypeScript-compiler-API indexer for up to 1.2M lines (2604.18413, 2026-04-20, D-self). Claude Code ships an `LSP` tool (definitions, references, type errors) that stays inactive until a code-intelligence plugin is installed (D, docs read 2026-10-01). Serena is an MCP toolkit over LSP, 40+ languages, GPL-3.0-or-later (D, README). | Good for live editing sessions. Wrong shape for a batch-indexed immutable corpus. |
| tree-sitter tags | `tree-sitter-tags` 0.27.0 gives definition and reference captures from `tags.scm`. Aider relies on it. | See coverage below. |

### 3.2 What each indexer needs (the constraint for delivered folders)

From the READMEs (D): scip-typescript expects `npm install` (or yarn/pnpm) first and offers `--infer-tsconfig`;
scip-python uses `pip` to learn installed packages or a JSON `--environment` file, Node 16 or newer; scip-clang needs a
`compile_commands.json` from a configured build; scip-dotnet runs through Docker or the .NET tool; scip-ruby is built
on Sorbet and works best with Sorbet type sigils. I did not read scip-java or scip-go requirements.

The vault excludes `node_modules`, virtual environments, `vendor`, `target`, `build` and lockfiles by rule, so a
delivered folder typically cannot be built or resolved. Running these tools means executing project build tooling
and, often, network fetches, on untrusted code. My brief forbids running cloned code; for the vault the same caution
applies: any SCIP step would need a sandbox and must be opt-in per repository.

### 3.3 Where tree-sitter tags are enough (and where they are not)

Coverage measured from the `tree-sitter-language-pack` README table (M): 369 grammar rows, 69 with a bundled
`tags.scm`. Present: Python, JavaScript, TypeScript, TSX, Rust, Go, Java, C, C++, C#, Ruby, PHP, Kotlin, Swift,
Scala, Lua, R, OCaml, Elixir, Dart, Fortran, MATLAB, CUDA, Nix, Solidity, among others. Missing: Haskell, Julia, Bash,
Zig, Verilog, SQL, Perl, Clojure, Erlang, LaTeX. The vault's research-heavy mix (Python, R, Julia, MATLAB, Fortran,
C/C++, CUDA) is covered except Julia.

Tags are enough for: the symbol table (`define`), per-file symbol tree and repository map (`outline`), the
name-matched reference graph that drives Aider-style ranking and graph expansion, and the enclosing-symbol label on
grep hits. They are not enough for: overload, shadowing and dynamic-dispatch resolution; cross-package resolution;
type-based "find implementations". Evidence that precision matters only in special queries: a 2026 paper shows
state-of-the-art localisers collapse when queries contain no names (section 5), and the answer there was a
Datalog-over-extracted-facts approach, not name matching.

Fallback for languages without a tags query: a short hand-written query (E: 30 minutes per language), or
universal-ctags run as a subprocess (GPL-2.0, 7,292 stars, pushed 2026-09-27; Zoekt runs it under seccomp, which does
not exist on Windows, so prefer in-process tree-sitter), or an outline-less line-window chunk with a `source_type`
flag recording "no symbols".

---

## 4. Code search engines, ranking signals, identifier tokenisation

### 4.1 Zoekt (source read, D)

`sourcegraph/zoekt` (Apache-2.0, pushed 2026-09-23). `google/zoekt` is archived (last push 2024-01-16). Sourcegraph's
main repository went to an archived public snapshot (2024-09-02).

BM25 path in `index/score.go` (read directly):
- `k = 1.2`, `b = 0.75`, document length measured in bytes against the shard average; line-level BM25 uses an assumed
  average line length of 100 bytes.
- **IDF is constant.** The code comment says evaluations showed IDF "can down-weight the score of some keywords too
  much" for keyword-style queries.
- `importantTermBoost = 5`: a match in the file name or in a symbol definition counts 5 times as a content match.
- `lowPriorityFilePenalty = 5`: term frequency divided by 5 for tests, generated and vendored files (categories from
  go-enry, the Linguist port).
- Per-line scores pick the chunk to display; a per-query boost multiplies the clause score.

Legacy (non-BM25) constants in `index/contentprovider.go`: filename and symbol match 7000 (partial 4000), word match
500 (partial 50), symbol-kind bonus 100 per kind weight. Symbols come from universal-ctags.

Evidence: Sourcegraph's own post, "Keeping it boring and relevant with BM25F" (Julie Tibshirani, 2025-04-04), says
treating symbol definitions and file names as fields boosted by 5 gave "roughly 20% improvement across all key
metrics" against Sourcegraph's internal evaluation sets (D-self). Zoekt's end-to-end rank test file lists 16 queries:
recall@1 56 percent, recall@5 69 percent, MRR 0.632 (M, tiny sample, read from the repository). The post also says
lexical matching stays essential for agentic search and for low-resource languages without embedding models.

### 4.2 GitHub Blackbird (2023-02-06, D-self)

Sparse n-grams rather than fixed trigrams (intervals where inner weights are below the border weights, recursively),
posting lists k-merged by score so relevant documents get low ids, content-addressed deduplication, and delta indexing.
Scale quoted: 45 million repositories, 115 TB of code, 15.5 billion documents, 25 TB index, about 120,000 documents
per second ingest (about 18 hours for a full re-index), shard P99 about 100 ms, about 640 queries per second per
64-core host. The relevant lesson is deduplication by content hash, which the vault's `doc_id` already provides, and
choosing n-grams to avoid common ones, which the Phase 0 trigram benchmark can test on code.

### 4.3 Identifier tokenisation

- Tantivy 0.26.2 tokenizers (M, from docs.rs): `SimpleTokenizer`, `WhitespaceTokenizer`, `RawTokenizer`,
  `RegexTokenizer`, `NgramTokenizer`, `FacetTokenizer`, plus filters `LowerCaser`, `AsciiFoldingFilter`,
  `AlphaNumOnlyFilter`, `RemoveLongFilter`, `SplitCompoundWords`, `Stemmer`, `StopWordFilter`. **No camelCase or digit
  boundary splitter.** `SimpleTokenizer` splits on non-alphanumerics, so it already breaks `snake_case`.
  A crates.io search for a tantivy camelCase or word-delimiter crate found none.
- Recommended analyzer for the code `content` and `symbol` fields (E, to be checked by the code eval): a custom
  filter that, for each identifier token, emits the lower-cased whole identifier plus its sub-words (split on `_`,
  `-`, case changes, letter/digit boundaries, acronym runs such as `HTTPServer` into `http`, `server`) at the same
  position, so phrase and proximity still work. This is the behaviour of Lucene's word-delimiter filter with
  "preserve original". No stemming on code fields.
- Measured vocabulary effect (M, Python identifiers via `tokenize`, dunder names excluded):

| Repo | Unique identifiers | Unique sub-words after splitting | Identifier occurrences with more than one part |
|---|---|---|---|
| requests | 1,843 | 1,117 | 19.4% |
| networkx | 13,484 | 4,449 | 21.4% |
| aider | 4,910 | 2,033 | 29.0% |

- Evidence that splitting improves BM25 retrieval: I found no controlled 2025 to 2026 measurement (U). Zoekt and
  Blackbird avoid the question through substring and n-gram matching plus word-boundary scoring. The vault already has
  a trigram index for `grep`, so exact substring queries are covered; sub-word fields serve the ranked `search` path.
  Treat splitting as a hypothesis for the code eval, not a settled gain.
- Lexical retrieval is weak for natural-language issue to code: BM25 36.86 versus CodeRankEmbed 58.76 versus
  SweRankEmbed-Large 82.12 Acc@10 function level on SWE-bench Lite (D-self, SweRank paper); BRIGHT coding splits
  BM25 24.4 (LeetCode) and 7.9 (Pony) nDCG@10 (D-ind via fetch); Agent Retrieval Bench BM25 MRR 0.1520 versus Qwen3-4B
  0.2379 (D-ind). It is strong when the query contains the identifier, which is why section 5 says keep both.

---

## 5. Agentic localisation versus embeddings

### 5.1 Chronological evidence

| Date | Work | Finding | Tag |
|---|---|---|---|
| 2024-07 | Agentless (2407.01489) | Fixed three-phase pipeline, hierarchical localisation, embedding retrieval used for file candidates; 32.00 percent on SWE-bench Lite at $0.70. | D-self |
| 2025-02 | OrcaLoca (2502.00350) | Function match 65.33 percent, file match 83.33 percent, resolved 41.00 percent (+6.33) on Lite; $0.87 per instance for localisation. | D-self |
| 2025-03 | LocAgent (2503.09089) | Graph (directory, file, class, function; contain, import, invoke, inherit) plus BM25 index, three tools (SearchEntity, TraverseGraph, RetrieveEntity). Lite file Acc@5 77.74 (Claude 3.5) versus CodeRankEmbed 52.55; function Acc@10 77.37 versus OpenHands 70.07 and SWE-agent 64.60; fine-tuned Qwen2.5-7B $0.05 versus Claude $0.66 per example. | D-self |
| 2025-03 (rev. 2025-10) | CoSIL (2503.22424) | Top-1 localisation 43.3 percent (Lite) and 44.6 percent (Verified) with Qwen2.5-Coder-32B. | D-self, abstract only |
| 2025-05 | SweRank (2505.07849) | Retrieve and rerank beats agents: function Acc@10 on Lite SweRankLLM-Large 88.69, SweRankEmbed-Large 82.12, versus LocAgent 77.37; LocBench 76.25 versus 60.71; about $0.011 per instance (7B) versus $0.66; 7 times faster than LocAgent. Models are CC-BY-NC-4.0, Python-trained. | D-self |
| 2025-12 | SweRank+ (2512.20482) | Adds a multilingual ranker and "SweRankAgent": multi-turn search over ranker output with a candidate memory, improving on single-pass ranking. | D-self, abstract only |
| 2026-02 | ContextBench (2602.05892) | 1,136 tasks, 66 repositories, 8 languages. "Sophisticated agent scaffolding yields only marginal gains in context retrieval"; models favour recall over precision; a gap between context explored and context used. | D-ind |
| 2026-03 | CodeScout (2603.17829) | RL-trained agents using only a Unix terminal match or beat 2 to 18 times larger models on SWE-bench Verified, Pro and Lite, with no graph tooling. | D-self, abstract only |
| 2026-04 | Neurosymbolic repo-level localisation (2604.16021) | Issue benchmarks are "saturated with keyword references" (paths, function names). A keyword-agnostic benchmark, KA-LogicQuery, shows a "catastrophic performance drop" of state-of-the-art approaches; Datalog over extracted program facts recovers it. | D-ind, abstract only |
| 2026-06 | SWE-Explore (2606.07297) | 848 issues, 10 languages, 203 repositories, line-level ground truth from successful agent trajectories, fixed line budget. "Agentic explorers form a clear tier above classical retrieval"; file level is solved, line-level coverage and ranking are not. | D-ind |
| 2026-07 | Agent Retrieval Bench (2607.24882) | No family dominates (lexical, dense Qwen3-Embedding, agentic selection, repo map). Logged agent trajectories missed gold files on 27 to 35 percent of samples. RRF of families helps. | D-ind |
| 2026-07 | Retrieval-oriented representations (2607.11046) | LLM role-aware summaries help; 94 percent Hit@6 file localisation with Agentless (+4.7). | D-self, abstract only |
| 2026-08 | Deep Agentic Search vs Semantic Search (2608.01507) | On SWE-QA read-only repository questions (Gemini 2.5 Flash/Pro, Gemini 3 Flash, Qwen3-235B): semantic search 65.2 percent pass, $0.32 per correct answer; "deep agentic search" (planner plus isolated sub-agent with grep/glob/read) 46.2 percent, $0.74; 41.8 percent of agentic failures were at the planner-to-sub-agent hand-off. Semantic baseline: 500-token recursive splitter, text-embedding-3-small. | D-ind |
| 2026-08 | Cost-effective exploration (2608.29675) | Cheaper explorer models keep 78 to 94 percent of Hit@3 with 84 to 95 percent fewer tokens (about 1,000 tasks). | D-ind, abstract only |
| 2026-09 | SemNav (2609.31176) | Deterministic retrieval seeds candidates, a language-server graph links entities, an LLM agent refines. File Hit@10 68.33 to 82.67 with a 4B model; resolution 44.00 to 52.33; context load down 48.2 percent. | D-self |

Vendor evidence:
- **Cursor** (blog 2025-11-06, D-self, data not published): custom embedding model trained from agent-session
  traces; semantic search in the agent gave on average 12.5 percent higher accuracy (6.5 to 23.5 percent depending on
  model) on "Cursor Context Bench", +0.3 percent code retention overall and +2.6 percent on repositories over 1,000
  files, and 2.2 percent more dissatisfied follow-ups when it was unavailable. Their conclusion: "the combination of
  [grep and semantic search] leads to the best outcomes."
- **GitHub Copilot** (blog 2025-09-24, D-self): new code embedding model, retrieval quality 0.362 to 0.498 (+37.6
  percent), about 2 times throughput, about 8 times smaller index memory.

### 5.2 Anthropic's stated reasons for agentic search

- Boris Cherny, Latent Space interview (2025-05-07): agentic search "outperformed everything. By a lot." (internal
  evaluation and impressions, not a published benchmark); no index to drift out of sync; no third-party vector store
  to be hacked; the price is latency and tokens (D-self).
- Anthropic engineering, "Effective context engineering for AI agents" (2025-09-29): "just in time" retrieval with
  lightweight identifiers such as file paths; Claude Code uses `glob` and `grep` "effectively bypassing the issues of
  stale indexing and complex syntax trees."
- Cline (2025-05-27): chunking tears code apart, an index is a frozen snapshot, embeddings are a second copy of your
  IP.
- Claude Code docs (read 2026-10-01): search tools (file patterns, regex content search) plus optional LSP-based code
  intelligence through plugins. No vector index is mentioned.

### 5.3 What this means for the vault

1. Reasons 2 and 3 (staleness, third-party store) do not apply: the vault publishes immutable text, the index is local,
   and nothing leaves the PC. Reason 1 (accuracy) is about a live repository with a shell.
2. The vault's agent has only the nine tools, no shell. `grep`, `read` and `outline` must therefore be strong enough
   to act as `grep`, `cat -n` and `ls/tree` do for Claude Code. The prose evidence in `references.md` (DCI: agentic
   grep wins on localisation because of "interface resolution") says the same.
3. Embeddings add recall for concept queries with no shared identifiers, where the 2026 results show semantic search
   ahead (Cursor, Deep Agentic Search paper, Agent Retrieval Bench MRR) and RRF fusion helps (+0.042 MRR).
4. The keyword-shortcut result means public localisation scores overstate how well grep-like methods do on
   concept queries. Test both with names removed.
5. Conclusion: ship the hybrid, make the exact tools excellent, and let the agent choose.

---

## 6. Embedding models and rerankers for code (2026-10-01)

### 6.1 Models

| Model | Date | Size / dims / context | Licence | Code score and who measured it | GGUF / llama.cpp |
|---|---|---|---|---|---|
| Qwen3-Embedding 0.6B / 4B / 8B | 2025-06-03/05 | 0.6B/4B/8B; 1024/2560/4096 dims; 32K ctx; MRL; instruction-aware | Apache-2.0 | MTEB-Code 75.41 / 80.06 / 80.68 (D-self, paper Table 9). jina's run of the 0.6B gives 74.69 (D, competitor). | Official GGUF for all three (`Qwen/Qwen3-Embedding-*-GGUF`) |
| Qwen3-Reranker 0.6B / 4B / 8B | 2025-05/06 | pointwise | Apache-2.0 | MTEB-Code rerank 73.42 / 81.20 / 81.22 (D-self) | The proposal cites ggml-org GGUF |
| bge-code-v1 (CodeR) | 2025-05-15 | 1.54B (Qwen2.5-Coder-1.5B base) | Apache-2.0 | CoIR average 81.77 versus voyage-code-3 78.53, CodeXEmbed-7B 78.20; CodeRAG average 72.8 (D-self). Trained on 2.9M synthetic CodeR-Pile samples. | Community GGUF only (Q4/Q6/Q8 repos, 2025) |
| nomic-embed-code | 2025-03-24 | 7.07B (Qwen2.5-Coder-7B-Instruct base) | Apache-2.0 | CodeSearchNet: Python 81.7, Java 80.5, Ruby 81.8, PHP 72.3, JS 77.1, Go 93.8; versus voyage-code-3 80.8/80.5/84.6/71.7/79.2/93.2 (D-self) | Official `nomic-ai/nomic-embed-code-GGUF` |
| CodeRankEmbed / CodeRankLLM | 2024-11 | 137M embedder; 7.6B listwise reranker | MIT | CoIR 60.1 (third-party table on the SFR card) | Community GGUF for both |
| SFR-Embedding-Code (CodeXEmbed) | 2025-01 | 400M / 2B | CC-BY-NC-4.0 | CoIR 61.9 / 67.4 on the original leaderboard; the same family scores 75.65 (2B) in a different harness (CodeR table) | None found |
| jina-code-embeddings 0.5B / 1.5B | 2025-08-29 (rev. 2026-08-17) | 0.5B/1.5B; up to 1536 dims, MRL 128 to 1536; 32768 ctx; five task prefixes (nl2code, code2code, code2nl, code2completion, qa); last-token pooling | CC-BY-NC-4.0 | MTEB-Code 78.72 / 78.94 versus Qwen3-0.6B 74.69, voyage-code-3 79.84, gemini-embedding-001 76.48 (D-self) | Official GGUF for both |
| Qodo-Embed-1 1.5B / 7B | 2025-02 | 1.5B/7B | QodoAI-Open-RAIL-M (use restrictions) | Listed lower than nomic on CSN in nomic's own table | Community GGUF |
| CodeSage-large-v2 | 2024-12 | 1.3B | Apache-2.0 | CoIR 64.2 (leaderboard) | not checked |
| voyage-code-3 | 2024-12-04 | API only; 2048 to 256 dims; 32K ctx | Proprietary | +13.80 percent over OpenAI-v3-large on 32 code datasets (D-self) | n/a; code would leave the machine |
| SweRankEmbed Small / Large | 2025-06-24 | 137M / 7B | CC-BY-NC-4.0 | Lite function Acc@10 74.45 / 82.12 (D-self) | None found |
| Nemotron-3-Embed 1B / 8B | 2026-07-14 | 2048 / 4096 dims, 32k ctx | OpenMDW-1.1 | RTEB 72.4 / 78.5, "#1 on RTEB as of 2026-07-16"; no code subset published in the article (D-self) | GGUF not mentioned (U) |
| zembed-1 | 2026-03-02 | 4B, 2560 dims, 32k | Apache-2.0 | vendor table, code column 0.6452 (D-self, vendor) | not checked |
| jina-embeddings-v5, jina-reranker-v3.5 | 2026-05, 2026-07-14 | text/omni; 0.6B reranker | CC-BY-NC-4.0 | no code numbers read | GGUF exists |

CoIR scores are **not comparable across sources**: SFR-Embedding-Code-2B is 67.4 on the original CoIR leaderboard but
75.65 in the CodeR table, and CodeR's 81.77 is inflated by APPS (98.08, a task the models may train on). Compare
models only inside one harness run, ideally your own.

2026 additions found in my queries are general-purpose (Nemotron-3-Embed, zembed-1, jina v5, Qwen3-VL-Embedding,
pplx-embed). I found no new open code-specific embedder since jina-code-embeddings (2025-08) (E; discovery limited).
Superseded: CodeSage, CodeXEmbed and Qodo-Embed-1 have been passed on published numbers by CodeR and jina-code.

### 6.2 Rerankers

- Qwen3-Reranker (Apache-2.0) is the practical choice for local GPU use.
- CoREB paper (2605.04615, 2026-05-06): off-the-shelf rerankers are inconsistent on code. nDCG@10 delta from not
  reranking: Qwen3-Reranker-0.6B -0.6 / -8.2 / -2.3 (text-to-code / code-to-text / code-to-code), Qwen3-Reranker-4B
  -0.1 / -3.2 / +3.3, Jina v3 -2.2 / -5.0 / -0.1. Its CoREB-Reranker (LoRA on Qwen3-Reranker-4B, Apache-2.0 per the
  model card, 4B) gives +1.1 / +0.8 / +5.1 (D-self). No GGUF is listed.
- CodeRankLLM (MIT, 7.6B listwise) and SweRankLLM (CC-BY-NC) exist; I did not measure them.
- Conclusion: reranking code is a measured gain only for some query types. Enable it behind the code eval, for
  natural-language queries only.

### 6.3 Is one general embedder good enough for prose and code?

For: Qwen3-Embedding-4B and 8B were the best dense models in the independent Agent Retrieval Bench (MRR 0.2379 and
Recall@20 0.7029); Qwen's self-reported MTEB-Code (80.68 at 8B) is on par with voyage-code-3 (79.84 in jina's table,
different run); SWE-Explore and ContextBench do not show a code-embedder advantage for localisation.

Against: code-specialised embedders dominate code-to-code (about 2 times over general encoders, CoREB); CodeR leads the
CoIR average; and the 2026 diagnostics show a ceiling for every embedder: CLARC (2603.04484, ICLR 2026) finds "sharp
drops" and "persistent reliance on lexical features" when identifiers are anonymised, and ExecRetrieval (2609.01865,
2026-09-01) finds the best of 23 configurations reaches exec@1 of only 0.331 and ranks the correct implementation below
a buggy near-clone in 67 to 78 percent of queries.

Verdict (E): for natural-language-to-code and mixed prose/code questions, which is the vault's main case, one 4B
general embedder is good enough and keeps one vector space and one ANN index. Run a bake-off with `bge-code-v1`,
`nomic-embed-code` and `CodeRankEmbed` on the vault's code eval, and switch only if a code model wins by about 3
nDCG@10 points or more, accepting a second vector table. Do not rely on any embedder to separate a function from a
slightly buggy copy; that is a lexical and exact-read job.

### 6.4 Capacity warning

Code is token-heavy. At about 3.5 bytes per token (E), 10 GB of source is about 2.9 billion tokens, more than the
whole prose corpus (2.5 billion tokens in `design-notes.md`). The proposal's bulk estimate for a 4B embedder is 15 to
20 hours per 2.5 billion tokens (E). So: embed only primary, non-duplicate source (skip tests, generated and vendored
categories, files under about 3 lines, and duplicate `doc_id`s) while keeping all text in BM25, trigram and `grep`.

---

## 7. Linking code to papers and books; source identity

### 7.1 Where the links live

| Surface | What it holds | State |
|---|---|---|
| `CITATION.cff` | `preferred-citation`, `references`, `identifiers` (doi, swh, url), `repository-code` | Latest release is 1.2.0 (2021-08-09, M); GitHub renders it and offers BibTeX. |
| `codemeta.json` | `referencePublication`, `citation`, `identifier`, `codeRepository`, `softwareHeritageId` style fields | Release 4.0 published 2026-09-29 (M). Fetch summary says it moves terms to `w3id.org/codemeta/terms/`, aligns with schema.org 30, and adds funding, `creditText`, `softwareSuggestions` and GitLab and code.json crosswalks. I did not confirm `referencePublication` handling in 4.0. |
| `.zenodo.json`, README, docs | DOIs, arXiv ids, BibTeX blocks | Free text; regex extraction. |
| Manifests | `pyproject.toml` project URLs, `package.json` repository, `Cargo.toml` repository and documentation | Structured but incomplete. |
| Paper side | GitHub, GitLab, Hugging Face and Zenodo links in paper text | Reachable with the vault's own `grep`. |

### 7.2 Papers with Code and its successors

- `paperswithcode.com` answers HTTP 302 to `https://huggingface.co/papers/trending` (M, 2026-10-01).
- The final data snapshot is on Hugging Face as `pwc-archive/links-between-paper-and-code`: 300,161 rows, columns
  `paper_arxiv_id`, `repo_url`, `is_official`, `mentioned_in_paper`, `mentioned_in_github`, `framework`; CC-BY-SA-4.0;
  card says "will not be updated", snapshot retrieved 2025-07-28 (M). The data repository
  `paperswithcode/paperswithcode-data` shows no push after 2025-09-08 (M).
- Live replacement: the Hugging Face papers API returns a `githubRepo` and `githubStars` field for papers where
  someone attached one. I confirmed it on two arXiv ids (cAST 2506.15655 gives `yilinjz/astchunk`; LocAgent 2503.09089
  gives `gersteinlab/locagent`; the Transformer paper has none) (M). Coverage is unknown (U).
- Link rot: of 14,727 unique GitHub links in 12,087 ACL-venue papers (2015-2025), 354 return 404 and 326 are empty or
  placeholder repositories, about 4.6 percent combined (computed from D-ind counts, "Papers Without Code",
  2609.34534, 2026-09-28); 6,330 of the 12,087 papers had no GitHub link. A vault copy is the durable resolver.

### 7.3 How trustworthy are the metadata surfaces

"A Multi-Surface Consistency Audit of Software Citation Metadata" (2608.17159, 2026-08-17, D-ind): over 117 projects,
83.9 percent of those with comparable surfaces have at least one core-field conflict; 28 of 32 projects with a
`preferred-citation` route citations to records that contradict the software's own declared metadata; about half the
conflicts are a surface describing the associated paper instead of the software. "From Code Archival to Knowledge
Graph" (2609.21667, 2026-09-18, D-ind) reconciled 4,397 DOI-to-repository pairs (JOSS, SoftwareX, IPOL) and created
4,182 new Wikidata software items keyed on SWHIDs. So: record every link as an edge with its evidence (the PwC
fields `is_official`, `mentioned_in_paper`, `mentioned_in_github` are a good model) and never merge a paper and
a repository identity on one weak signal.

### 7.4 Software Heritage identifiers (SWHID)

Specification v1.2 (released 2025-11-07, M); the specification's foreword says it was the starting point for
ISO/IEC 18670. I could not open iso.org (HTTP 403), so the standard's publication status and year are U.
Facts from the specification text (read via GitHub, D):
- `swh:1:cnt:` is the SHA-1 of `"blob" + space + decimal byte length + NUL + content`, the Git blob hash. Directory,
  revision, release and snapshot ids are defined the same way; Git compatibility is described as "practical, but
  incidental and is not guaranteed."
- The `lines` qualifier: line break is ASCII LF, numbers start at 1, range inclusive, `lines=9-15` or `lines=9`. This
  is the same convention as `{doc_id}#L9-L15`. `bytes` is 0-based inclusive and ignores `lines` if both are present.
- Qualifiers: `origin`, `visit`, `anchor`, `path`, `lines`, `bytes`.
- The directory hash uses entry modes (`100644`, `100755`, `120000`, `40000`).

Consequences for the vault (E): (1) hash the original bytes before normalising; store the file's `swh:1:cnt` and a
flag saying whether canonical text equals original bytes; the `lines` qualifier stays valid after CRLF-to-LF or BOM
removal because line count is unchanged, but not for lone-CR files or bytes ranges. (2) When `.git` is present,
the HEAD commit id is the revision SWHID `swh:1:rev:<sha1>` with `origin=<remote>`; recompute no directory hashes
from a Windows copy (NTFS loses the executable bit) and use the Git tree id instead. (3) Presence in the Software
Heritage archive is an optional online enrichment, not a dependency.

---

## 8. Evaluation of code retrieval (external and internal checks)

| Benchmark | What it is | Use in the vault | Caveat |
|---|---|---|---|
| CoIR (2407.02883, 2024-07-03; ACL 2025 main) | 10 datasets, 8 tasks, 7 domains; same schema as MTEB and BEIR; pip package; now an MTEB code subset | Embedder bake-off, regression gate | Scores differ across harness versions (section 6.1). APPS is near saturation. |
| MTEB-Code | Used by Qwen3 and jina papers | Cross-check to vendor numbers | Vendor-run. |
| CoREB (2605.04615, 2026-05-06) | Counterfactually rewritten LiveCodeBench problems, text-to-code, code-to-text, code-to-code, with reranking; data under CC-BY-4.0 | Contamination-resistant ranker and reranker check | New, few independent results. |
| CLARC (2603.04484, ICLR 2026) | 1,245 eval and 5,472 train C/C++ query-code pairs; identifier anonymisation and compiled forms | Robustness probe | C/C++ only. |
| ExecRetrieval (2609.01865, 2026-09-01) | 939 Python tasks, one correct and up to four single-edit buggy distractors; 23 dense configs plus BM25 | Shows what embeddings cannot do (near-clone discrimination) | Diagnostic, not a target. |
| BRIGHT coding splits (2407.12883) | LeetCode (142 queries, 413,932 docs) and Pony (112 queries, 7,894 docs), web-page corpus | Reasoning-heavy query check | Not a source-code corpus. Best nDCG@10 29.9 and 22.0 (D-ind via fetch). |
| CodeRAG-Bench (2406.14497) | Retrieval for code generation over docs, tutorials, StackOverflow, GitHub | Tests whether retrieved context helps generation | Older; retrievers struggle with low lexical overlap. |
| SWE-bench Lite / Verified / Multilingual, LocBench (560) | Issue to file/function localisation, Acc@k | Comparable external numbers (LocAgent file Acc@5 77.74; SweRank function Acc@10 82.12 to 88.69) | Contamination risk (2512.10218); training sets such as SweLoc overlap PyPI repositories; keyword shortcut (2604.16021). |
| SWE-Explore (2606.07297) | 848 issues, 203 repos, 10 languages, line-level truth under a line budget | Best end-to-end match for `outline` plus `read` budgets | Truth derived from agent trajectories. |
| ContextBench (2602.05892) | 1,136 tasks, 66 repos, 8 languages, block and line level recall, precision, efficiency | Context quality, not just localisation | |
| Agent Retrieval Bench (2607.24882) | 427 samples, 25 repos, includes 50 no-gold and 32 counterfactual controls; 7.9M chunks | Abstention and fusion tests | Early. |

Internal set the vault should add (E): (a) deterministic checks against ground truth: `grep` versus ripgrep output,
`define` versus tags output; (b) keyword-agnostic natural-language queries written for functions with identifiers
and docstrings removed from the query; (c) paper to repository link recall against the PwC archive for papers that
are in the vault; (d) SWE-bench-style issue to file tasks over repositories already delivered. Keep a held-out split,
as the vault's eval rules require.

---

## 9. Recommended design for code in this vault

### 9.1 Principles

1. Per-file verbatim documents (as planned) stay the unit of citation. Chunks, symbols and graphs are derived
   indexes pointing at line ranges of those documents. Nothing derived is ever shown without an address.
2. Exact tools first, hybrid ranking second, optional precise indexers third.
3. Deterministic structure (tags, name-matched references, imports) over LLM-built graphs.
4. Additive, optional parameters only; keep tool names and descriptions byte-stable.

### 9.2 Document model

| Item | Proposal |
|---|---|
| Source file | One document per file, verbatim, `type=code`, subtype `source-file`. `doc_id` as designed (BLAKE3 of canonical text), so identical files in many repositories collapse to one document with many `(repository, path, commit)` rows in SQLite. |
| Repository | One generated document, subtype `repository`. Sections in order: metadata block (name, remote, HEAD commit, `swh:1:rev`, licence SPDX, language shares, linked papers/DOIs/arXiv ids), README, file tree with each file's address, a static ranked symbol map (4k-token and 16k-token variants), dependency list. |
| Notebooks and docs in a repository | As in INSTRUCTIONS section 4: normal routes. Note a converted notebook's line numbers refer to the converted Markdown, not the `.ipynb` bytes. |
| `find_docs` default | Return repositories and prose documents; return source files only when the query looks like a path or symbol or `type=code` is requested. A large file count must not drown the 160k-document prose router. |
| Identity extras | `swh:1:cnt` per file, flag `canonical_equals_original`, git remote, HEAD commit, tree id. |

### 9.3 Derived indexes

| Index | Content |
|---|---|
| `symbols` (SQLite) | `(repo, doc_id, kind, name, qualified_name, parent, start_line, end_line, signature, doc_first_line, language, source: tags or scip)` |
| `refs` (SQLite) | name-matched references from tags `@reference.*`: `(doc_id, line, name, enclosing_symbol)`; plus import edges from per-language import queries |
| `repo_graph` | file-level weighted graph with Aider multipliers; stored PageRank score per file and per symbol (global, unpersonalised) |
| Code chunks (tantivy, separate index from prose) | cAST chunks, fields: `symbol` (names defined in the chunk, boost 5), `path` (boost 5), `content`, `scope` (class and function chain); analyzer with identifier sub-word splitting; fast fields `lang`, `repo`, `category` (source, test, generated, vendored, docs), `chunk_key`. Separate from the prose index so BM25 statistics do not mix. |
| Trigram index | Same plan as prose: line-aligned blocks of about 16 KiB, with a file as one block when it fits (M: a typical file is about 10 KB). |
| Dense vectors | Embed the chunk with a header `repo | path | language | scope chain | signature | first docstring line` followed by the code. Same embedder as prose unless the bake-off says otherwise. |

### 9.4 Chunking

- Parse with tree-sitter. Split-then-merge by non-whitespace size, limit 2000 (cAST optimum 2000 to 2500; about 700
  to 800 tokens, E). A class that exceeds the limit becomes header plus groups of methods; each group keeps the class
  header line in its `scope` field, not duplicated in text.
- Every boundary snaps to a whole line, so each chunk is exactly `#L{a}-L{b}`.
- Files with parse errors, no grammar, or over the size limit: 60-line windows with 10-line overlap, flagged
  `chunker=lines` (M: 60-line windows cut 19 to 27 percent of ordinary functions, which is why this is the fallback
  and not the default).
- Skip dense embedding for the `tests`, `generated`, `vendored` and `docs` categories and for tiny or duplicate
  files; keep them in BM25, trigram and `grep`.

### 9.5 Ranking signals (code `search`)

Lexical (BM25F on the code index): symbol-name and path fields times 5, content times 1, constant IDF, k 1.2, b 0.75,
term frequency divided by 5 for test, generated and vendored files (Zoekt, D). Dense: cosine over the chunk vector,
query with an instruction prefix for code. Fusion: RRF of lexical and dense (Agent Retrieval Bench: MRR 0.2296 to
0.2713, D-ind). Priors: global file PageRank as a small tie-breaker; exact identifier match to a defined symbol
promotes that definition to rank 1; collapse multiple chunks of one file into one result with the best chunk shown.
Optional: graph expansion of the top 10 by containment and call edges (SpIDER, +13 to +27 percent relative Recall@20,
D-self), and a reranker for natural-language queries only (section 6.2).

### 9.6 The nine tools for code

All are additive extensions of the existing signatures.

| Tool | What it does for code |
|---|---|
| `search(queries[])` | Hybrid ranked chunks. New optional filters: `language`, `repo`, `path_glob`, `symbol_kind`, `category` (default excludes `test`, `generated`, `vendored` unless asked). Each hit returns address, repository, path, scope chain, signature, and the reason (matched symbol, path, or semantic). |
| `find_docs(queries[])` | Repositories first (README, description, topics, linked paper titles, top symbol names), files only for path or symbol-like queries. Repository results show languages, licence and linked papers. |
| `workspace(action, doc_ids[])` | Adding a repository adds all its source files; allow `paths` globs to add a subtree. |
| `grep(patterns[], scope)` | The primary exact tool. Add `path_glob`, `language`, `files_only` (list matching files with counts), `word`, and `symbol_only` (match only definitions). Every hit is annotated with the enclosing symbol and its line range from `symbols` (cheap interval lookup). Trigram prefilter, verification with the ripgrep crates, as designed. |
| `read(addresses[], expand)` | Output is line-numbered. `expand="section"` for code means the enclosing function or class; add `expand="skeleton"` (signatures and docstrings of a class or file with bodies elided, `⋮` as in Aider). Whole-file reads are guarded by a size cap with a pointer to `outline`. |
| `outline(doc_ids[])` | Repository: directory tree with file counts, plus a ranked symbol map fitted to optional `budget_tokens` (default 4k) and optional `focus` terms for personalisation. File: symbol tree with kinds, signatures, line ranges, and imports. |
| `citations(doc_ids[], direction)` | Edge kinds via an optional `kinds` list. Code to code: `import` (file level inside a repository) and `depends` (repository to repository from manifests, resolved to vault repositories). Paper and repository: `cites` and `cited_by` carrying `evidence` flags (CITATION.cff, codemeta, README, paper text mention, PwC archive) and `is_official`-style confidence. Symbol-level `call` edges only when SCIP data exists. |
| `define(terms[])` | Resolves a term against both the glossary and `symbols`. For code returns `kind:symbol`, qualified name, signature, first docstring line, address, and definition count; exact match, then identifier-split match. A term such as "softmax" can return a textbook definition and the code definitions in one call. |
| `vault_info()` | Adds repository, file and line counts by language; per-language symbol coverage (tags, scip, none); exclusion counts by rule; share of files with a `swh:1:cnt`; link counts by evidence kind. |

Example shapes (illustrative):

```
grep(patterns=["def (fit|transform)\\b"], scope={"repo":"scikit-learn"}, files_only=false, max_hits=20)
  -> 3f9a...-sklearn-base/#L120-L121   in BaseEstimator.fit   [L100-L180]   "def fit(self, X, y=None):"

outline(doc_ids=["a1b2...-networkx"], budget_tokens=4000, focus=["shortest path"])
  -> networkx/algorithms/shortest_paths/generic.py  (a1b2...#L1-L640)
       shortest_path(G, source=None, target=None, weight=None, method='dijkstra')   L18-L120
       ...
```

### 9.7 Precise indexers (optional)

Keep tags as the baseline. Where a repository carries enough environment, an opt-in step may run `rust-analyzer scip`,
`scip-go`, `scip-typescript`, `scip-python` or `scip-java` in a sandbox with no network, then load the `.scip` through
the `scip` crate into `symbols` and `refs` (source `scip`), replacing name-matched references for those files. Do this
only after the code eval shows a gap that precision would close.

---

## 10. Ordered increments (effort estimates E, one engineer)

| # | Increment | Effort | Depends on | Check that proves it |
|---|---|---|---|---|
| 1 | Code intake: language detection, file categories (source, test, generated, vendored, docs), exclusions as listed, per-file docs, repository doc with metadata and README, file tree. Plus identity extras (`swh:1:cnt` from original bytes, HEAD commit, remote, tree id). | 2 to 3 days | Phase 1 intake | Delivered sample (aider, requests, networkx) ingests; `read` of any `#L` range equals the original lines; hashes match `git hash-object`. |
| 2 | Tags extraction with vendored grammars (`tree-sitter-tags`, language pack) into `symbols` and `refs`; per-file `outline`; `define`. Hand-written tags for gaps (Julia first). | 3 to 4 days | 1 | Symbol counts match `ctags` or `ast` on the Python sample (M ground truth available); `define` returns definitions for 50 sampled names. |
| 3 | cAST chunker plus code tantivy index (fields, sub-word analyzer, boosts) and code trigram blocks; `grep` with enclosing-symbol annotation, `path_glob`, `files_only`. | 4 to 5 days | 2 | `grep` equals ripgrep on the sample; chunk line ranges tile each file exactly; BM25 query for a known function returns it in top 3. |
| 3b | Code eval harness: deterministic checks, keyword-agnostic NL set, CoIR subset, SWE-bench-style file localisation over delivered repositories. | 3 to 4 days | 3 | Baseline numbers recorded; held-out split fixed. |
| 4 | Dense vectors for code chunks with the prose embedder, RRF fusion, `search` filters. Bake-off with `bge-code-v1`, `nomic-embed-code`, `CodeRankEmbed`. | 2 to 3 days (plus GPU time) | 3, 3b, Phase 0 embedder choice | Hybrid beats BM25 alone on the eval by a stated margin; decision on single versus separate space recorded. |
| 5 | Repo map: reference graph, Aider multipliers, PageRank, budget fitting; `outline` with `budget_tokens` and `focus`; static map in the repository document. | 3 days | 2 | Map fits within 15 percent of the budget; on the eval, repo map plus `read` reaches gold files at least as often as BM25 at equal tokens (Agent Retrieval Bench style yield). |
| 6 | Links: CITATION.cff, codemeta, README DOI/arXiv extraction; manifest dependencies; paper-side URL extraction with `grep`; PwC archive seed; `citations` kinds. | 4 to 5 days | 1, prose corpus present | Link recall and precision on a hand-labelled sample of 100 repositories; every edge carries evidence. |
| 7 | Reranker (Qwen3-Reranker, optionally CoREB-Reranker) for natural-language code queries; graph expansion. | 2 days | 4, 5 | Each must raise eval nDCG@10 or Recall@20 or be left off. |
| 8 | SCIP opt-in sandbox for Rust, Go, TypeScript, Python, Java. | 5 to 8 days | 2, sandbox decision | Reference counts differ from tags on a sample, and the eval shows the gain justifies the cost. |
| 9 | LLM role-aware summaries for repositories and top-ranked files; commit-history memory sidecar (`.git` history is excluded by rule today; see section 11). | 3 to 5 days plus GPU or API cost | 4 | Hit@5 gain on file localisation versus path-only (reference: up to +40 percent, D-self). |

Total for increments 1 to 7 (including 3b): about 23 to 29 working days (E).

---

## 11. Open decisions and things I could not resolve

1. **Non-commercial licences.** `jina-code-embeddings`, `SFR-Embedding-Code`, `SweRank*` and `jina-reranker-v3.5`
   are CC-BY-NC-4.0, Qodo is use-restricted RAIL. Whether private personal use is acceptable is the owner's call; I
   recommended only permissive models.
2. **Git history.** The delivered rule excludes `.git` and keeps only HEAD. History is useful for localisation
   (Repository Memory, ICLR 2026, 2510.01003, abstract only) and for citing a version. A compact sidecar (commit
   subjects and touched paths) would need an owner decision on intake.
3. **Sandbox for SCIP.** Running indexers executes project tooling. Needs an explicit decision and an isolated
   environment (no network, throwaway profile).
4. **`type` values.** INSTRUCTIONS lists a single `code` type; I propose subtypes `repository` and `source-file`.
5. **Tool parameters.** I propose optional additions to existing tools. If the tool schema must stay frozen, the
   fallback is a `mode` string inside existing parameters.
6. **Unverified items.** ISO/IEC 18670 publication status (iso.org returned 403); `scip-java` and `scip-go`
   requirements; whether CodeMeta 4.0 handles `referencePublication` as I assume; Nemotron-3-Embed GGUF support;
   the SCIP governance page; the fetched summary dated CodeMeta 4.0 as 2024-09-29 while the GitHub API gave
   2026-09-29 (I used the API value).
7. **Abstract-only reads** (numbers not cross-checked in the full text): CoSIL, SweRank+, SpIDER, SemNav, CodeScout,
   2604.16021, SWE-Explore, ContextBench, Agent Retrieval Bench (full page read for numbers), cost-effective
   exploration, retrieval-oriented representations, RepoAtlas, CodeNib, ExecRetrieval, CLARC, CoREB.
8. **FastContext (2606.14066)** was withdrawn by its authors on 2026-06-30; I did not use its numbers.

---

## 12. Sources (with dates)

Chunking and graphs
- cAST, arXiv 2506.15655, 2025-06-18. astchunk, github.com/yilinjz/astchunk (MIT).
- RepoGraph 2410.14684 (2024-10-03, ICLR 2025). CodexGraph 2408.03910 (2024-08-07). Code Graph Model 2505.16901
  (2025-05-22). SpIDER 2512.16956 (2025-12-18). Reliable Graph-RAG 2601.08773 (2026-01-13). TypeScript Repository
  Indexing 2604.18413 (2026-04-20). CodeNib 2607.25431 (2026-07-28). RepoAtlas 2609.16936 (2026-09-15). Formal
  Architecture Descriptors 2604.13108 (2026-04-11).
- Aider: `aider/repomap.py` (repository pushed 2026-05-22); docs "Repository map"; blog "Building a better repository
  map with tree sitter" (2023-10-22).

Code intelligence
- SCIP: github.com/scip-code/scip (v0.10.0, 2026-09-03), `docs/DESIGN.md`, scip-code.org; indexer repositories listed
  in section 3. stack-graphs: github.com/github/stack-graphs (archived). tree-sitter 0.27.0 and tree-sitter-tags
  0.27.0 (2026-08-30, crates.io). tree-sitter-language-pack 1.20.0 (2026-09-14). Serena (oraios/serena, GPL-3.0).
  Claude Code docs, tools reference (read 2026-10-01).

Search engines
- Zoekt source: `index/score.go`, `index/contentprovider.go`, `doc/ctags.md`, `internal/e2e/testdata/rank_stats.txt`
  (main, pushed 2026-09-23). Sourcegraph, "Keeping it boring and relevant with BM25F" (2025-04-04). GitHub, "The
  technology behind GitHub's new code search" (2023-02-06). tantivy 0.26.2 tokenizer docs (docs.rs).

Agentic localisation and evidence
- Agentless 2407.01489 (2024-07-01). OrcaLoca 2502.00350 (2025-02-01). LocAgent 2503.09089 (2025-03-12). CoSIL
  2503.22424 (2025-03-28, rev. 2025-10-10). SweRank 2505.07849 (2025-05-07). SweRank+ 2512.20482 (2025-12-23).
  ContextBench 2602.05892 (2026-02-05). CodeScout 2603.17829 (2026-03-18). Neurosymbolic localisation 2604.16021
  (2026-04-17). SWE-Explore 2606.07297 (2026-06-05). Agent Retrieval Bench 2607.24882 (2026-07-27). Retrieval-oriented
  representations 2607.11046 (2026-07-13). Deep Agentic Search 2608.01507 (2026-08-02). Cost-effective exploration
  2608.29675 (2026-08-30). SemNav 2609.31176 (2026-09-25). Repository Memory 2510.01003 (2025-10-01).
- Latent Space, Claude Code interview (2025-05-07). Anthropic, "Effective context engineering for AI agents"
  (2025-09-29). Cline, "Why Cline doesn't index your codebase" (2025-05-27). Cursor, "Improving agent with semantic
  search" (2025-11-06). GitHub, Copilot embedding model post (2025-09-24).

Embeddings and rerankers
- Qwen3 Embedding 2506.05176 (2025-06-05; Hugging Face model cards). jina-code-embeddings 2508.21290 (2025-08-29, rev.
  2026-08-17). CodeR / bge-code-v1 2505.12697 (2025-05-19). CoRNStack 2412.01007 (ICLR 2025) with nomic-embed-code and
  CodeRankEmbed cards. CodeXEmbed 2411.12644 (2024-11-19) with SFR cards. Qodo-Embed-1 card (2025-02). voyage-code-3
  post (2024-12-04). Nemotron-3-Embed post (2026-07-16). Hugging Face cards for zembed-1, jina v5, jina-reranker-v3.5,
  CoREB-Reranker.
- CoREB 2605.04615 (2026-05-06). CLARC 2603.04484 (2026-03-04). ExecRetrieval 2609.01865 (2026-09-01). CoIR
  2407.02883 (2024-07-03, ACL 2025). BRIGHT 2407.12883 (2024-07-16, rev. 2025-03-26). CodeRAG-Bench 2406.14497.
  SWE-bench contamination study 2512.10218 (2025-12-11).

Linking and identity
- Papers with Code redirect and `pwc-archive` datasets on Hugging Face (checked 2026-10-01); `paperswithcode/
  paperswithcode-data`. Hugging Face papers API. "Papers Without Code" 2609.34534 (2026-09-28). "A Multi-Surface
  Consistency Audit of Software Citation Metadata" 2608.17159 (2026-08-17). "From Code Archival to Knowledge Graph"
  2609.21667 (2026-09-18). CFF 1.2.0 (citation-file-format repository). CodeMeta 4.0 (GitHub release, 2026-09-29).
  SWHID specification v1.2 (2025-11-07; chapters 5 and 6 read in the `swhid/specification` repository).

Measurements by me (M)
- `C:/Users/sethy/AppData/Local/Temp/claude/C--Users-sethy-OneDrive-Desktop-Tutor/ba4722e7-2537-4ced-b4a0-4062c34e0fa8/scratchpad/meas.py`
  (Python stdlib only, parses files as data) over `C:/Users/sethy/Projects/refs/rag/{requests,networkx,aider}`;
  GitHub, Hugging Face and crates.io API queries run 2026-10-01.

# Phase 0 code study: how autonomous coding agents explore and localise code

Author: worker (Sonnet 5.5), read-only study for the lead. Date: 2026-10-01.
Scope: cluster "agent exploration and localisation". Nothing was built or run; every claim below comes from reading the cloned sources, the repo READMEs and docs, or the arXiv abstract pages (fetched 2026-10-01). Where a number comes from the project itself it is marked "self-reported". The web-search budget for the session was exhausted, so no independent third-party comparison could be looked up; independent evidence is therefore thin and flagged as such.

All clones are shallow, under `C:/Users/sethy/Projects/refs/code/<short-name>` (paths below are relative to that folder).

## 0. Clone register and currency (today = 2026-10-01)

| Short name | Upstream | HEAD | Last commit | Licence (file read) | Status |
|---|---|---|---|---|---|
| swe-agent | SWE-agent/SWE-agent | 3ea751c087f32b16e039a2233dd6eefecef325d5 | 2026-07-16 | MIT (`LICENSE`) | Maintained but superseded: README banner says mini-swe-agent "has superseded SWE-agent" (`swe-agent/README.md` lines 11-17) |
| mini-swe-agent (extra) | SWE-agent/mini-swe-agent | 04d809ceab9df28f9adaed044884180159172930 | 2026-09-03 | MIT (`LICENSE.md`) | Active, the successor |
| openhands | OpenHands/OpenHands | 953c944eee62b8077a65436c2c8fd6b355342562 | 2026-09-30 | MIT (`LICENSE`) | Active, but the repo is now the "Agent Canvas" TypeScript UI. The agent code moved to software-agent-sdk (`openhands/README.md` line 168, `AGENTS.md` line 33) |
| openhands-sdk (extra, same project) | OpenHands/software-agent-sdk | fad63774459171b08f889b12fd3b4d6346168c3e | 2026-09-30 | MIT (`LICENSE`) | Active; the actual tools live here |
| agentless | OpenAutoCoder/Agentless | 5ce5888b9f149beaace393957a55ea8ee46c9f71 | 2024-12-22 | MIT (`LICENSE`) | Stale (about 21 months), research artefact, not archived on GitHub |
| locagent | gersteinlab/LocAgent | 4935b557326c154bad8e8dcf3747cc8d32d1f387 | 2026-08-12 | Apache-2.0 (`LICENSE`) | Research code, updated recently (README now points to CodeNib) |
| moatless-tools | aorwall/moatless-tools | 011ead57a5c81664e9c45e07e1f50b17e695cc63 | 2025-09-01 | MIT (`LICENSE`) | Stale (13 months); MCTS paper code lives in a separate repo `moatless-tree-search` (`README.md` line 3) |
| astchunk (the cAST repo) | yilinjz/astchunk | 82029ada8a8dd8d668283c076980cedb886e6d4c | 2026-06-26 | MIT (`LICENSE`) | Small research repo (362 KB), idle 3 months |
| cocoindex | cocoindex-io/cocoindex | 72d479838a7610b8a5f7bfa187986a6ca3202fe4 | 2026-09-30 | Apache-2.0 (`LICENSE`) | Very active, Rust core with Python API |
| chonkie | feyninc/chonkie (the brief's `chonkie-inc/chonkie` redirects here; README badges still say chonkie-inc) | 664454b689d68614bcfc0b1d57fc2a28b5862825 | 2026-08-26 | MIT (`LICENSE`) | Active, v1.7.0 (`pyproject.toml` line 8) |
| text-splitter | benbrandt/text-splitter | e51a648690f85ee2b148cc42658e8989ee2e29b8 | 2026-09-29 | MIT (`LICENSE.txt`) | Active; v0.33.0 released 2026-09-24, so the vault pin is current |
| cocoindex-code (extra) | cocoindex-io/cocoindex-code | 8ec0ff3510be526e699028db5b64e10a0a8359b3 | 2026-09-22 | Apache-2.0 (`LICENSE`) | Active, 2.7k stars, agent-facing |
| codenib (extra) | sysevol-ai/CodeNib | 4232ddfab2ee0cf08d1649d4e97fa416848873fd | 2026-09-29 | Apache-2.0 (`LICENSE`) | Active beta (README: "public interfaces may change") |
| tree-sitter-language-pack (extra, not cloned, 1.5 GB; read through `gh api`) | xberg-io/tree-sitter-language-pack (the `kreuzberg-dev/...` URL I first tried resolved to this) | main b7fc9ae9f7149697fb2ca320bad0156c18ee6519 | 2026-09-22 | MIT (repo badge; grammars carry their own licences, see `ATTRIBUTIONS.md`, not read) | Active, release v1.20.0 on 2026-09-14 |

None of the studied repos is archived. No GPL/AGPL/NC/source-available licence was found in this cluster. Caveat: tree-sitter-language-pack bundles 371 third-party grammars whose individual licences I did not audit.

Extras used: mini-swe-agent, openhands-sdk (both are where the named projects now live), cocoindex-code, codenib, tree-sitter-language-pack. That is more than "3 extras" only if mini-swe-agent and openhands-sdk are counted as separate projects; I treat them as the current home of SWE-agent and OpenHands.

---

## 1. SWE-agent (ACI) and mini-swe-agent

**What it is.** Princeton/Stanford software-engineering agent. The core idea is the Agent-Computer Interface (ACI): purpose-built tools instead of raw shell (`swe-agent/docs/background/aci.md` lines 1-17). Python. Maturity: widely used research code, but now superseded by mini-swe-agent.

**Ingestion.** None. The agent operates on a checked-out repo inside a Docker container and discovers code at runtime with tools. No index, no language detection, no ignore handling beyond what the tools do:
- `tools/search/bin/search_dir` uses `find ... ! -path '*/.*' -exec grep -nIH` (hidden dirs skipped, binaries skipped by `-I`); `.gitignore` is not consulted (`tools/search/bin/search_dir` line 15).
- Directory view = `find -maxdepth 2 -not -path '*/\.*'` (`tools/edit_anthropic/bin/str_replace_editor` lines 438-452).

**Indexing.** None. Only one structural trick: `filemap` / `show_filemap` uses tree-sitter on Python only to elide the bodies of functions of 5 or more lines, printing `... eliding lines a-b ...` and keeping original line numbers (`tools/filemap/bin/filemap` lines 21-41; same class at `tools/edit_anthropic/bin/str_replace_editor` lines 193-228). It is applied only when a `.py` file exceeds `MAX_RESPONSE_LEN = 16000` characters and `USE_FILEMAP` is true (same file lines 28, 493-507). The viewer also has a `WindowExpander` that tries to extend a window to whole function/class boundaries, at most 30 extra lines (same file, class `WindowExpander`).

**Ranking.** None. Search returns unranked lists.

**Exposed to agents.** Tools are shell scripts with a YAML signature (`tools/*/config.yaml`):
- `find_file <file_name> [<dir>]`, `search_dir <search_term> [<dir>]`, `search_file <search_term> [<file>]` (`tools/search/config.yaml`).
- `open "<path>" [<line_number>]`, `goto`, `scroll_up`, `scroll_down`, `create` (windowed viewer, `tools/windowed/config.yaml`). Defaults `WINDOW=100`, `OVERLAP=2` (`tools/windowed/install.sh` lines 9-10).
- Modern default config uses `str_replace_editor` (`view` with `view_range`, `create`, `str_replace`, `insert`, `undo_edit`) plus a bash tool (`config/default.yaml` lines 40-60).
- Output budgeting: the `search_dir` tool refuses to answer when more than 100 files match ("Please narrow your search", `search_dir` lines 29-33) and prints only "path (N matches)" per file, never the matching lines. `docs/background/aci.md` line 12 states why: "showing the model more context about each match proved to be too confusing". Observations over `max_observation_length` (default 100,000 chars in `sweagent/agent/agents.py` line 79; 10,000 with head/tail halves in `config/bash_only.yaml` lines 181-200) are clipped with a "response clipped" note. Empty output returns an explicit "ran successfully and did not produce any output" message (`aci.md` line 14).

**Scale and performance evidence.**
- Paper abstract (arXiv 2405.15793, fetched): pass@1 12.5% on SWE-bench and 87.7% on HumanEvalFix; claims the ACI design is what makes the difference. Independent: no (authors' own paper). I did not retrieve the ablation tables, so I cannot quote the per-tool deltas.
- mini-swe-agent README (`mini-swe-agent/README.md` lines 28 and 40-50): ">74% on SWE-bench verified" with bash as the only tool, and "does not have any tools other than bash". Self-reported; it points to the swebench.com bash-only leaderboard (third-party hosted, but the same authors run it) and to external adopters (Ramp, DeepSWE evaluation harness).
- Implication: for strong current models a plain grep/find/cat/sed loop is enough to localise in repos of SWE-bench size (hundreds to a few thousand files). The ACI conveniences matter more for weak models.

**Ideas to adopt in the Rust vault.**
1. Summarised search view as a first stage: file path plus match count, hard cap (100 files) with an explicit "narrow your query" message. Effort S. Pure design, no crate.
2. Outline view that elides long bodies and keeps real line numbers (`... eliding lines a-b ...`). This is exactly what `outline(doc_ids[])` should return, with the threshold configurable (SWE-agent uses 5 lines). Effort M with tree-sitter (see tree-sitter-language-pack and cocoindex below). Licence-safe: MIT idea, we write our own.
3. Windowed read: default window about 100 lines, window snapped to enclosing symbol boundary (max 30 lines of expansion). Effort S on top of the symbol table.
4. Explicit empty-result messages and "next step" hints in every tool result. Effort S.

**Fit.** SWE-agent: port_ideas (ACI principles, filemap). mini-swe-agent: reference_only (evidence that a thin tool set suffices, and baseline for evaluating whether the vault actually beats `grep`+`cat`).

---

## 2. OpenHands (software-agent-sdk)

**What it is.** The `OpenHands/OpenHands` repo at HEAD is now a TypeScript "Agent Canvas" UI (`openhands/README.md` lines 1-30). The agent, tools and server are in `OpenHands/software-agent-sdk` (Python, MIT). Studied: `openhands-sdk/openhands-tools/openhands/tools/`.

**Ingestion / indexing.** None. No code index of any kind. Exploration is purely tool driven.

**Ranking.** Only modification time: `grep` and `glob` both call ripgrep with `--sortr=modified` (`tools/grep/impl.py` lines 231-236; `tools/glob/impl.py` lines 151-158). This respects `.gitignore` as a side effect of ripgrep (ripgrep default; the code has a Python fallback and a system-grep fallback, `grep/impl.py` line 261, so behaviour differs between backends).

**Exposed to agents (exact names and shapes).**
- `grep`: params `pattern` (regex), `path` (dir, absolute), `include` (glob such as `*.{ts,tsx}`). Returns `GrepObservation{matches: list[str] of file paths, pattern, search_path, include_pattern, truncated: bool}`. It returns file paths only, not matching lines, uses `rg -l`, cap `_MAX_MATCHES = 100` (`tools/grep/definition.py` lines 18-45, `impl.py` lines 36 and 142; description text lines 47-55 says "Returns matching file paths sorted by modification time").
- `glob`: params `pattern`, `path`; uses `rg --files -g`.
- `file_editor` (Anthropic-style `str_replace_editor`: view/create/str_replace/insert/undo_edit); view output clipped at `MAX_RESPONSE_LEN_CHAR = 16000` (`file_editor/utils/constants.py` line 1).
- `terminal`, `browser_use`, `delegate`/`task` (sub-agents), `ask_oracle` (second model as advisor), `task_tracker`, `planning_file_editor`, `apply_patch`.
- Tool metadata uses MCP-style annotations (`readOnlyHint`, `idempotentHint`) and a `declared_resources` hook so read-only tools run in parallel (`grep/definition.py` lines 62-80).
- Context budgeting: an LLM summarising condenser (`context/condenser/llm_summarizing_condenser.py`, `max_size=240` events, `keep_first=2`) rather than smart tool output budgeting.
- Skills: keyword-triggered markdown "skills" injected into context (`sdk/skills/skill.py` lines 157-175), similar in spirit to repo instructions.

**Scale and performance evidence.** None in the repo for localisation specifically. SWE-bench numbers for OpenHands exist elsewhere but I did not verify them here.

**Ideas to adopt.**
1. Mark every vault tool read-only/idempotent via MCP annotations so clients can parallelise. Effort S.
2. Sort ties by recency (mtime or last-commit time) as a weak ranking feature. Effort S.
3. Note the failure mode: grep that returns only file names forces an extra read call per candidate; the vault should return file plus symbol plus first matching line(s) inside the same budget. Effort S.

**Fit.** reference_only. Useful as the baseline tool shape every Claude Code/Codex/OpenCode client already knows (grep, glob, view, edit), which the vault must be compatible with, not copy.

---

## 3. Agentless (OpenAutoCoder)

**What it is.** A fixed three-phase pipeline (localise, repair, validate), no agent loop. Python only; the repo structure parser handles only `.py` files (`agentless/get_repo_structure/get_repo_structure.py` line 182, `endswith(".py")`). MIT. Stale (last commit 2024-12-22).

**Ingestion.** Pre-computes a repo "structure" JSON (directories, files, classes, functions) per SWE-bench instance with Python `ast`; no ignore rules beyond that, no size limits found.

**Indexing.** Optional embedding index with LlamaIndex: OpenAI `text-embedding-3-small`, `chunk_size=512`, `chunk_overlap=0`, retriever `similarity_top_k=100` (`agentless/fl/retrieve.py` lines 144-147; `agentless/fl/Index.py` lines 220-290). Two document builders: "simple" (whole file, with a metadata header of class and function names; metadata is dropped when it exceeds half the chunk budget, `Index.py` `check_meta_data`) and "complex" (one document per class, method, function and one for global statements, `build_file_documents_complex`). No symbol graph, no call graph.

**Ranking and the hierarchical loop (the important part).**
1. File level, LLM prompt 1: whole repo tree plus issue text, "return at most 5 files" (`agentless/fl/FL.py`, `obtain_relevant_files_prompt`).
2. In parallel: LLM prompt 2 lists irrelevant folders to exclude; embedding retrieval runs only on the remaining files (`README_swebench.md` lines 81-106).
3. Merge by vote counting: `Counter` over the top-n of the LLM list and the top-n of the embedding list, `most_common()` (`agentless/fl/combine.py` lines 20-50). README uses `--top_n 3`.
4. Related elements: files are shown as compressed skeletons (function bodies replaced by a placeholder, `agentless/util/compress_file.py`, `libcst`) and the LLM names classes and functions.
5. Fine-grained: full code of those elements is shown with line numbers and the LLM names exact lines, sampled 4 times at temperature 0.8, then merged (`README_swebench.md` lines 120-184).

**Exposed to agents.** Nothing (batch research scripts, JSONL in/out).

**Scale and performance evidence.** Paper abstract (arXiv 2407.01489, fetched): 32.00% (96 of 300) on SWE-bench Lite at $0.70 per issue. README (lines 14-16): v1.0 27.3% at $0.34; Dec 2024 integration with Claude 3.5 Sonnet 40.7% (Lite) and 50.8% (Verified). All self-reported. Superseded in the leaderboard sense by agentic systems; its importance is as the proof that a staged "file, then symbol, then line" localisation is competitive.

**Ideas to adopt.**
1. Make staged disclosure the default agent workflow: `workspace`/`vault_info` (repo map) then `find_docs` (files) then `outline` (skeleton with line ranges) then `read` (exact lines). The vault should make each stage one call. Effort M; design in the tools.
2. "Irrelevant folder pruning": an agent-side step; the vault can support it by returning a directory tree with sizes and language mix at a requested depth, so a 100k-file repo (Unreal) fits in a prompt. Effort S.
3. Cheap fusion of two independent candidate lists by voting (rank-agnostic, robust). For the vault's own hybrid retrieval prefer reciprocal-rank fusion, which is the same idea with weights. Effort S, trivially in Rust.

**Fit.** reference_only (prompt designs, pipeline shape). Python-only and stale; nothing to import.

---

## 4. LocAgent (gersteinlab)

**What it is.** Graph-guided localisation agent (ACL 2025). Python, Apache-2.0. Python-only parsing (`ast`). The README now carries a note that CodeNib can serve LocAgent's tools from prebuilt indexes (`locagent/README.md` line 12).

**Ingestion.** Walks the repo, skipping `.git`/`.github` (`dependency_graph/build_graph.py` lines 24-27), test files are excluded from retrieval (`bm25_retriever.py` exclude globs: `**/test/**`, `**/tests/**`, `**/test_*.py`, `**/*_test.py`; `required_exts=['.py']` hard-coded with a TODO). No `.gitignore`, no binary/vendored detection.

**Indexing.**
- Graph (`networkx`, pickled): node types directory, file, class, function; edge types `contains`, `inherits`, `invokes`, `imports` (`build_graph.py` lines 12-20). Built with Python `ast`; invocation edges are name-resolved heuristically (imports plus qualified names). Entity id format `file_path:Qualified.name`.
- Two BM25 indexes (LlamaIndex BM25 with English stemmer): one over entity ids (module-name retrieval) and one over code chunks from `EpicSplitter` (the moatless splitter: min 100, target 500, max 2000 tokens, hard limit 2000, max 200 chunks per file; `bm25_retriever.py` lines 37-46). No embeddings in the localisation path (an embedding index package exists under `repo_index/index/`, unused by the three-tool loop).
- Incremental updates: none; indexes are pre-built per benchmark instance.

**Ranking.** BM25 on entity names and on chunk text. File-name matches via the module index. No PageRank/centrality (`grep -ri "pagerank|centrality"` found nothing). Traversal output is a tree/graph listing.

**Exposed to agents.** Three tools (`util/runtime/function_calling.py` line 24):
- `search_code_snippets(search_terms: list[str]?, line_nums: list[int]?, file_path_or_pattern: str = "**/*.py")`: terms can be a keyword, a `file_path:QualifiedName`, or a bare file path (returns full entity or file). If no entity matches, falls back to BM25 over chunks. Line numbers return the enclosing code block with a 20-line context window (`repo_ops.py` lines 187-237 and 490-690).
- `get_entity_contents(entity_names: list[str])` (full code of named entities).
- `explore_tree_structure(start_entities, direction: upstream|downstream|both, traversal_depth: int (-1 = unbounded, capped), entity_type_filter, dependency_type_filter)`: renders a tree of related entities. Invalid names produce BM25 "did you mean" candidates (`repo_ops.py` lines 864-1000). Note: the docstring defines `upstream` as "dependencies that the entity relies on", which is the opposite of the usual caller/callee reading; this is a naming trap to avoid in the vault.
- Budgeting: results are rendered in four formats chosen per result set, `fold` (signature or first lines plus a hint "Search `<id>` for the full content if needed"), `preview`, `complete`, `code_snippet`; in `preview`, smaller (<100 lines) blocks nested inside an already-shown larger range are dropped (`repo_ops.py` lines 560-660, `utils/result_format.py`). This is a concrete progressive-disclosure design. The loop is capped at 20 iterations (`auto_search_main.py` line 119).

**Scale and performance evidence.** Paper (arXiv 2503.09089, fetched abstract): up to 92.7% file-level localisation accuracy with a fine-tuned Qwen2.5-Coder-32B, about 86% cost reduction versus proprietary models, +12% Pass@10 on downstream issue resolution; introduces Loc-Bench. Self-reported by the authors (peer reviewed at ACL 2025, not independently reproduced here). CodeNib's docs state its LocAgent provider is revision-pinned and "compatibility does not imply reproduced paper scores" (`codenib/docs/agent_integrations.md`).

**Ideas to adopt.**
1. Entity address scheme `file:Qualified.name` with fuzzy "did you mean" on miss. The vault's `read(addresses[])` should accept symbol addresses in addition to `doc_id#Lx-Ly`, resolving to real line ranges and returning candidates on a miss. Effort S-M.
2. Typed graph with exactly four edge kinds (contains, imports, invokes, inherits) and a depth/direction/type-filtered traversal that returns a compact tree. Effort M-L. In Rust: `petgraph` or plain adjacency in SQLite. Licence-safe (we design our own).
3. Fold/preview/complete rendering selected by result count and size, with a "fetch full" hint. Effort S-M.

**Fit.** port_ideas (graph schema, address format, formatting modes). Python-only parsing and pickled per-instance graphs are not reusable.

---

## 5. moatless-tools (aorwall)

**What it is.** Hobby framework for LLM code editing with strong context tooling. Python, MIT. README (`README.md` line 9): "Claude 4 Sonnet 70.8% solve rate, $0.63 per instance" on SWE-bench (self-reported). Stale since 2025-09-01.

**Ingestion.** Per-instance index from a git checkout. Language support is Python and Java only (`moatless/codeblocks/parser/create.py` lines 6-25). Parse failures skip the file silently with a log (`epic_split.py` "Failed to use epic splitter... continue"). Generated-file heuristic: a file above the hard token limit whose first comment contains "generated" is dropped (`epic_split.py` `_chunk_contents`). Test files are marked a separate category and excluded from default semantic search (`category="implementation"`).

**Indexing.**
- `EpicSplitter` (tree-sitter code blocks): defaults `chunk_size=750` soft, `min_chunk_size=100`, `max_chunk_size=1500` (index settings: hard limit 2000, max 200 chunks per file; `moatless/index/settings.py`). Algorithm: recursively split children of module/class/function nodes; split when a child of a "split type" exceeds the minimum or the parent plus child exceeds max; then merge small neighbours (`_merge_chunks`) without crossing the hard limit. Each chunk's text is rendered as the path tree of ancestors with out-commented placeholders for omitted code (`_to_context_string`), so a method chunk carries its class header. Comments can be INCLUDE, ASSOCIATE (attach to following block), or EXCLUDE.
- Dense vectors: FAISS plus LlamaIndex docstore, OpenAI `text-embedding-3-small` (1536 dims) by default, Voyage optional (`settings.py`, `pyproject.toml`). No BM25 in moatless proper (BM25 appears in LocAgent which copied the splitter), no trigram, no call graph in the search path (the code graph is disabled, `enable_code_graph=False`).
- Incremental: none beyond rebuilding the persisted index.

**Ranking.** Vector similarity, top_k 500 candidates, then span aggregation per file with token accounting; an "exact match if possible" mode that keeps only spans containing the query string and stops after 5 (`code_index.py` lines 252-395, `_vector_search` lines 582-690). `find_class` / `find_function` use name matching with a reranker that prefers path matches (`_rerank_files`, line 805).

**Exposed to agents (action names).** `SemanticSearch(query, category, file_pattern)`, `FindClass(class_name, file_pattern)`, `FindFunction(function_name, class_name, file_pattern)`, `FindCodeSnippet(code_snippet, file_pattern)`, `GrepTool(pattern, include, max_results)`, `Glob`, `ViewCode(files with line spans)`, `ListFiles`, `ReadFile(s)`, plus edit and test actions (`moatless/actions/`).
Output budget: `SearchBaseAction` defaults `max_search_tokens=2000`, `max_hits=10`, `max_identify_tokens=8000`, `max_identify_prompt_tokens=16000` (`actions/search_base.py` lines 84-100). When results exceed the budget the tool does not truncate silently: it asks the agent to refine ("Search too large... will ask for clarification", lines 227-232). An optional LLM "identify" step selects the relevant spans inside the hits before returning them.

**Scale and performance evidence.** Self-reported solve rates only (README table lines 160-167, models from 16% to 50% on a 20-instance "verified mini" subset). Not independent.

**Ideas to adopt.**
1. Ancestor path header in every embedded chunk (class and function signatures, with placeholders for elided siblings). Effort M. Also helps BM25 on qualified names.
2. Token-accounted result assembly: stop adding hits at a budget, report "N more not shown", suggest how to narrow. Effort S; belongs in the shared result formatter.
3. Test/implementation category as an indexed filter, default to implementation. Effort S.
4. Do not copy: "identify" LLM step (the vault should stay model-free in the server).

**Fit.** port_ideas. Licence MIT; the algorithm is easy to reimplement.

---

## 6. cAST / ASTChunk (yilinjz/astchunk)

**What it is.** Reference implementation of cAST (EMNLP Findings 2025, README lines 8-12). Python, MIT, 754 lines total. Languages: Python, Java, C#, TypeScript (README "Configuration Options"; `astchunk_builder.py` language dispatch).

**The algorithm (read from `src/astchunk/astchunk_builder.py`).**
- Size metric: count of non-whitespace characters, with a prefix-sum array so any node's size is O(1) (`preprocessing.py`, `get_nws_count`).
- If the whole tree fits in `max_chunk_size`, one chunk.
- Otherwise iterate the root's children greedily, accumulating siblings into a window while the sum stays within the limit; if a node alone exceeds the limit, recurse into its children, then merge adjacent child windows greedily (`assign_nodes_to_windows`, `merge_adjacent_windows`). Only siblings are merged, so chunks never cross structural boundaries upward.
- Optional overlap by whole AST nodes (`chunk_overlap`), and optional "chunk expansion": prefix each chunk with `'''\n<filepath>\n<ancestor first lines, indented>\n'''` (`astchunk.py` `apply_chunk_expansion`, `build_chunk_ancestors` keeps only class and function ancestors and only their first line).
- Metadata: start/end line, node count, chunk size.

**Scale and performance evidence.** README abstract (paper): "+4.3 points Recall@5 on RepoEval retrieval and +2.67 Pass@1 on SWE-bench generation" versus line-based chunking. Source: the authors' own paper; the gains are modest and measured on Python-centric benchmarks, so they are an argument for "AST chunking is better than fixed windows", not for any specific size. I did not read the paper's ablations on chunk size.

**Ideas to adopt.** (1) Non-whitespace size measure with prefix sums (stable across indentation styles, cheap). (2) Sibling-only merge so chunks stay structurally coherent. (3) Ancestor header ("chunk expansion") as separate embedded text, not stored text, so the stored document stays verbatim. All effort S-M and about 150 lines of Rust on top of `tree-sitter`. MIT, trivial to reimplement.

**Fit.** port_ideas. Do not depend on it (Python, 4 languages).

---

## 7. CocoIndex (code embedding example) and cocoindex-code

**What it is.** CocoIndex is an incremental data-pipeline engine with a Rust core and a Python API (`cocoindex/README.md`). Apache-2.0. The workspace crates carry version `999.0.0` (`Cargo.toml` line 16), i.e. they are internal path crates, not published stable crates; reuse means vendoring or porting with attribution, not a dependency.

**Ingestion (code embedding example `examples/code_embedding/main.py`).** `localfs.walk_dir` with include globs (`**/*.py`, `**/*.rs`, `**/*.toml`, `**/*.md`) and exclude globs (`**/.*`, `**/target`, `**/node_modules`); `live=True` enables a file-system watcher. Language by file extension via `detect_code_language`. Memoisation is per file (`@coco.fn(memo=True)`), keyed on content so unchanged files are not re-embedded. The watcher (python `watchdog` observer) feeds an event queue and performs a periodic full rescan to catch events the OS watcher dropped (`python/cocoindex/connectors/localfs/_source.py` lines 235-290). That "watch plus periodic rescan" pattern is worth copying.

**Indexing / chunking (the valuable Rust part).**
- `RecursiveChunker` in `rust/ops_text/src/split/recursive.rs` (1034 lines): breaks a file into "atom chunks" at tree-sitter node boundaries (or regex separators when no grammar exists), then picks the minimum-cost partition into chunks of target size with a dynamic program (`merge_atom_chunks`, line 425). Costs: `SYNTAX_LEVEL_GAP_COST = 512`, `MISSING_OVERLAP_COST = 512`, `PER_LINE_BREAK_LEVEL_GAP_COST = 64`, `TOO_SMALL_CHUNK_COST = 1,048,576` (constants at the top of the file). Parameters: `chunk_size` (bytes), `min_chunk_size` (default half), `chunk_overlap`. Output chunks carry byte range, char offset, line and column.
- Defaults: example uses 1000 / 300 / 300 (size/min/overlap); cocoindex-code uses 1000 / 250 / 150 (`cocoindex-code/src/cocoindex_code/indexer.py` lines 24-27).
- Tree-trust hazards (`rust/code_ast/src/hazards.rs`): a pre-pass flags a file as untrustworthy when an ERROR/MISSING node of at least 8 bytes appears at depth 3 or less, calibrated on llvm and spark ("damaged files show >=17B errors at depth <= 3; healthy files max out at 1B shallow", lines 14-21), and marks subtrees deeper than 500 levels as opaque. Untrusted files fall back to regex separators. This is directly relevant to macro-heavy C++ (Unreal `UCLASS()`, `GENERATED_BODY()`), where tree-sitter-cpp is likely to produce shallow errors; the vault needs exactly this guard. I have not run it on Unreal code, so the claim that Unreal triggers it is an inference, not a measurement.
- Language registry (`rust/code_ast/src/prog_langs.rs`): about 60+ languages with extensions, but only some have a grammar attached. C, C++, C#, Rust, Lua, CMake, Python, Java, JS/TS, Go, Dart, Kotlin, Swift, Bash, etc. have grammars (`code_ast/Cargo.toml` lines 18-60). GLSL, HLSL, GDScript, CUDA are registered by extension with `None` for the grammar (`prog_langs.rs` lines 176-226), so they get regex chunking only.
- Structural extraction: `elements/` has declaration/namespace/reference tables per language (C-family hooks in `elements/lang/cfamily.rs`, 324 lines), with a normalised `DeclarationKind` (class, struct, union, enum, function, method, constructor, field, extension...) (`elements/types.rs` lines 11-50). `rust/code_match` implements by-example structural patterns over 14 languages with an index prefilter (`prefilter.rs`, `index_terms`).
- Embeddings: pluggable (sentence-transformers or LiteLLM); example uses `all-MiniLM-L6-v2`. Store: Postgres/pgvector, LanceDB, Qdrant, SQLite (sqlite-vec) etc.

**cocoindex-code (agent-facing wrapper, extra).** `ccc` CLI, MCP stdio server with exactly one tool:
`search(query: str, limit: int = 5 (1-100), offset: int = 0, refresh_index: bool = true, languages: list[str]?, paths: list[str]? (GLOB))` returning `{success, results: [{file_path, language, content, start_line, end_line, score}], total_returned, offset, message}` (`src/cocoindex_code/server.py` lines 40-130). Ranking is pure vector similarity from sqlite-vec KNN (L2 converted to cosine score, `query.py` lines 14-40); no BM25, no symbol boost, no recency. Default embedding for `[full]` installs: `Snowflake/snowflake-arctic-embed-xs` (`README.md` line 59). Includes `ccc grep` (by-example structural grep that needs no index, `src/cocoindex_code/grep.py` lines 1-12). Walking: nested `.gitignore` honoured (`file_walk.py`), default include list of about 60 extensions that omits `.cu`, `.glsl`, `.hlsl`, `.gd`, `.cmake` and default excludes `**/.*`, `node_modules`, `target`, `dist`, `vendor` (`settings.py` lines 19-70), optional `max_file_size`. Incremental indexing is triggered by an agent hook after Edit/Write tool calls and by `refresh_index=true` (`README.md` lines 99-101). README claims "instant token saving by 70%" with no method or data in the repo (searched README for benchmark; none). Treat as marketing, not evidence.

**Scale and performance evidence.** None in the code-search area beyond that claim. Chonkie-style speed numbers do not apply.

**Ideas to adopt.**
1. Port the DP chunk-merge with syntax-level gap cost. Effort M (about 400 lines Rust), output is better-balanced chunks than greedy merging (text-splitter) and works with any `tree_sitter::Language`. Apache-2.0: port with NOTICE attribution. Value: highest-quality chunk boundaries in this cluster.
2. Port `hazards.rs` (about 80 lines): shallow-error and depth guard; fall back to line/regex chunks for untrusted trees; also record `parse_degraded` in the document metadata so agents know symbols for that file are approximate. Effort S.
3. Watch plus periodic rescan, with content-hash memoisation. In Rust: `notify` plus `blake3` (or the hash already planned) plus a "last full scan" timer. Effort M.
4. One-call agent surface with `refresh_index` param and offset pagination: good, but the vault already plans richer tools.
5. `DeclarationKind` vocabulary for the `outline` and symbol map. Effort S.

**Fit.** cocoindex: port_ideas (chunker DP, hazards, declaration kinds, watcher pattern). cocoindex-code: reference_only (single-tool UX, hook-driven refresh; weak retrieval, pure dense).

---

## 8. Chonkie (chonkie-inc, now feyninc)

**What it is.** Python RAG-ingestion library with many chunkers (token, sentence, recursive, semantic, late, neural, slumber, table, code, fast). MIT. v1.7.0.

**Code chunker.** `src/chonkie/chunker/code.py` (284 lines) is now a thin wrapper over the `tree-sitter-language-pack` Python package: `process(text, ProcessConfig(language, chunk_max_size=<bytes>))` returns `chunks` with `content`, `start_byte`, `end_byte` (lines 130-160). Chunk size is given in tokens and converted to bytes using the whole file's bytes-per-token ratio (`_estimate_chunk_max_bytes`, lines 117-124), default 2048 tokens and a character tokenizer. Language auto-detection: shebang/content detection, then trial-parsing with every downloaded grammar and ranking by fewest errors, then most structure, then most nodes (lines 21-60). No ancestors, no symbols, no line numbers (byte offsets only), no overlap. Cost: first use downloads all grammars.

**Other.** `FastChunker` is a SIMD byte-delimiter splitter from the separate `chonkie-core` package (docstring claim "~100+ GB/s", `chunker/fast.py` lines 11-30; unverified, and I did not find the `chonkie-core` repo under chonkie-inc). BENCHMARKS.md reports speed on 100k Wikipedia articles on an A100 Colab: token chunking 58 s (4.82 MB/s), recursive 1 min 19 s; these are self-reported, text only, not code.

**Ideas to adopt.** Only the trial-parse language detection (fewest parse errors wins) as a fallback for extension-less or ambiguous files (for example `.h` that is C or C++, `.inl`, `.cginc`). Effort S. Everything else is Python or wrapped.

**Fit.** reference_only. For Rust, go to tree-sitter-language-pack (below) or text-splitter directly.

---

## 9. text-splitter (benbrandt), already pinned at 0.33.0

**What it is.** Rust crate (MIT) for size-bounded splitting: `TextSplitter`, `MarkdownSplitter`, `CodeSplitter` (feature `code`, needs `tree-sitter` 0.27 in 0.33.0, `Cargo.toml` lines 75, 94). Sizers: characters (default), `tokenizers` (HF), `tiktoken-rs`, or any `ChunkSizer`. Config: capacity or a (desired, max) range, `with_overlap`, `with_trim` (`src/chunk_size.rs` lines 66-130 and 280-383). v0.33.0 released 2026-09-24; its changelog confirms tree-sitter 0.27 and icu_segmenter 2.3.

**Code splitting algorithm (`src/splitter/code.rs`).**
- Parses with tree-sitter, takes every node of the tree in depth-first order, and uses depth as the "semantic level" (shallower is a better boundary) via a `Depth` newtype; below nodes it falls back to graphemes, words, sentences, characters. Chunks are formed by taking the highest level whose first section still fits and merging as many neighbours as fit, never crossing a boundary of a higher level (doc comment on `chunks`).
- Returns `&str` slices plus byte offsets (`chunk_indices`) or byte and char offsets (`chunk_char_indices`). Indentation preserved (`Trim::PreserveIndentation`).
- It does not add ancestor context, symbol names, or line numbers; lines must be derived from byte offsets.
- Behaviour to know: the unit test in the crate splits `fn main() {\n    let x = 5;\n}` at capacity 16 into `["fn main()", "{\n    let x = 5;", "}"]` (`code.rs` tests), i.e. at small capacities a signature is separated from its body. Use capacities comfortably above typical function size, and add our own symbol/ancestor header.
- Language handling: any `tree_sitter::Language`; no error-tree guard (grep for `is_error` in `src` found none), so macro-broken trees will be chunked by whatever structure tree-sitter recovered.

**Scale and performance.** The repo has criterion benches (`benches/chunk_size.rs`) and the changelog records worst-case fixes for distant boundaries (v0.32.0) and a binary-search bound (changelog line 295), but I found no published throughput table. Independent evidence: none read.

**Ideas to adopt.** Use it as the safe general chunker for prose, Markdown, plain text, and as the last-resort chunker for code in languages without a good grammar or when the tree is flagged untrusted. Pair it with a `tokenizers` sizer using the same tokenizer as the embedding model (Qwen3-Embedding) so chunk sizes are in real tokens. Effort S (already pinned). For code with grammars, prefer the DP chunker above or a cAST-style sibling merge, falling back to `CodeSplitter`.

**Fit.** adopt_dependency (already adopted). Not sufficient alone for the code path.

---

## 10. CodeNib (extra, closest direct competitor)

**What it is.** MCP/CLI tool that gives Claude Code and Codex source context plus typed graph navigation. Python plus a C++ core (`core/`, `native/`). Apache-2.0. Beta. Languages: chunking for 14; graph backends for 12 (Python, Go, Rust, C/C++, C#, Java, Ruby, PHP, Kotlin, Scala, JS, TS); Swift and Lua chunking only (`docs/language_capabilities.md`, generated table). Compiler-resolved symbols come from SCIP indexers and clangd (C/C++ uses clangd, "SCIP cold-start: none", incremental via clangd). This requires toolchains and, for C/C++, a compilation database; that is impractical for Unreal-scale trees.

**Ingestion.** Requires "a clean repository" and a git checkout, builds a manifest `repo_manifest.json` with views (BM25, vector, symbol graph, Zoekt) under `$CODENIB_HOME/repositories/<repo>-<id>/indexes` (`codenib/mcp/README.md`). Source-surface selection and exclusion rules exist (`repository_filters.py`, `repository_source_selection.py`); there is a content-fingerprint binding so reads are verified against the indexed bytes.

**Indexing.**
- Chunking: tree-sitter chunkers with depth levels L0 file (optionally signature-only skeleton), L1 top-level definitions (class, function, macro `#define`, etc.), L2 members (default) (`codenib/code_chunking/README.md`).
- Sparse: BM25 over symbols. Dense: configurable (the benchmark uses CodeRankEmbed and Qwen3-Reranker-4B). Trigram: it shells out to Sourcegraph Zoekt (`codenib/index/trigram/zoekt_searcher.py` docstring: "sub-50 ms substring/regex search over millions of lines of code"). Also a symbol-content regex index.
- Incremental: `codenib/index/incremental/` has git-diff and chunk-store modules, but the README says "file-level delta repair is not enabled" in the product (language section), so effectively rebuild or view reuse.

**Ranking.** `search_context` picks a deterministic plan (BM25, dense, hybrid RRF, or graph-expanded) from loaded views and budget. `explore_context` composes retrieval, LSP-shaped route, dependency expansion and verified source reads.

**Exposed to agents (exact names).** `explore_context(query, symbols?, top_k=8, budget="fast|balanced|thorough", direction="impact|dependencies|both", include_dependencies=true, filter_test=false)`, `search_context(query, top_k=10, budget, level="l0|l1|l2", filter_test)`, `search_semantic`, `search_bm25`, `search_regex`, `search_zoekt`, `dependency_subgraph(symbol, direction, depth<=8, max_nodes=60, max_edges=400)`, `lsp_definition`, `lsp_references`, `lsp_route`, `read_source(file_path, start_line=1, end_line?)` (max 200 lines, 1-based inclusive, content-authenticated), `get_manifest` (`codenib/mcp/server.py` lines 204-575, `mcp/README.md` tool table). A `--tool-surface explore` mode exposes only `explore_context`, and calls to hidden tools are rejected.
Budgeting: serialised result cap 256 KiB with a 4 KiB envelope reserve (`mcp/explore_bounds.py` lines 24-27), query-focused excerpts of 800-2,400 chars with a 10,000-char total default (`context_delivery.py` lines 9-12), per-connection delivery ledger so the same content is not re-sent (`explore_session.py`, `runtime.ledger.project(response)` in `server.py`), all lines 1-based with conversion at the boundary.

**Scale and performance evidence (self-reported, vendor-run, not independent).** `docs/evaluation/grep_jev.md`: on 100 issues over 25 repositories (C/C++, Go, Python, Rust, TS/JS; 151 target code blocks; dataset CodeNib Base), code-block Recall@5: model-planned grep 58.62%; model-planned grep reranked by their "Jev" 71.40%; BM25 then Jev 59.35%; dense (CodeRankEmbed) then Qwen3-Reranker-4B 63.40%; hybrid BM25 plus dense then Qwen3-Reranker-4B 65.03%; hybrid then Jev 67.92%. Paired bootstrap interval +6.44 to +19.67 points, 20 wins, 78 ties, 2 regressions. Cost reported $1.049 for 100 issues (planning plus rerank via OpenRouter). The authors state the result does not establish end-to-end agent success or token savings. This is the most direct measured comparison of "agent-planned grep versus embeddings" in this cluster, with the caveat that the best route needs a remote reranker and a remote planning model.
Also notable: it validates its SWE-Explore scorer against the official evaluator (1,020 of 1,020 cells match on a fixed 20-case run) and runs paired comparisons for LocAgent, Agentless, CoSIL, OrcaLoca (`docs/agent_integrations.md`).

**Ideas to adopt.**
1. A single composed "explore" call that returns grouped source windows with budget presets (fast/balanced/thorough), plus separate precise tools. Effort M. The vault already splits `search` / `grep` / `read`; add an optional composed mode only if tests show it saves calls.
2. Per-session delivery ledger (do not re-send ranges already shown in this connection, or mark them "already delivered"). Effort M, needs a connection/session id in MCP. Strong win for the "few tool calls, little context" goal.
3. Hard byte cap on the full serialised result, with metadata truncation rules and an explicit "omitted N" summary. Effort S.
4. Zoekt-style positional trigram index as the grep prefilter: the vault's plan (tantivy plus trigram verified by ripgrep crates) matches. Reference for design; do not shell out to Go.
5. `read_source` with a verified content binding (hash at index time equals bytes on disk) so stale line numbers are detected. Effort S: store blake3 per document and compare on read.
6. Capability matrix generated from the language registry (which languages have chunker, symbols, graph). Effort S, good for agent trust.

**Fit.** port_ideas (tool surface, budgets, ledger, evaluation method). Not adoptable as a dependency (Python, requires toolchains, SCIP/clangd per language).

---

## 11. tree-sitter-language-pack (extra, via gh api)

**What it is.** MIT-licensed Rust crate `tree-sitter-language-pack` with bindings in 15 languages; 371 grammars at ABI 14 or 15; bundled `highlights/injections/locals/tags/indents/folds` queries per grammar where they exist. Latest release v1.20.0 (2026-09-14). Repo is 1.5 GB so I read it through `gh api`, not a clone.
- Game-relevant grammar rows (README table): C, Cpp, Csharp, Rust, Lua, CMake, GLSL, HLSL, GDScript, Gdshader, CUDA, WGSL, Slang, Odin, Zig, Objc all present; but only some ship `tags.scm` (C, Cpp, Csharp, Rust, Lua have it; GLSL, HLSL and GDScript do not), and HLSL/GDScript lack even highlights.
- `process(source, ProcessConfig)` returns structure, imports, exports, comments, docstrings, symbols, diagnostics, chunks, with `chunk_max_size` bytes; defaults recommend `RECOMMENDED_MAX_SOURCE_BYTES = 16 MiB` and `RECOMMENDED_PARSE_TIMEOUT_MS = 5000` (`crates/ts-pack-core/src/process_config.rs` lines 11-18).
- Its `split_code` is vendored from text-splitter (header comment of `text_splitter.rs`: "Vendored and adapted from text-splitter by Ben Brandt (MIT License)"), simplified to return byte ranges, with a traversal depth limit.
- Runtime model: grammars are shared libraries downloaded on first use and cached locally (default features `dynamic-loading`, `download` pull `ureq`, `sha2`, `tar`, `zstd`, `libloading`; `Cargo.toml` lines 15-30). For an offline vault that is a supply-chain and reproducibility point to settle: pre-download and pin by hash, disable `download`, or vendor grammars.
- Scale evidence: none read; no performance table.

**Ideas to adopt.** Use it (or the individual `tree-sitter-*` crates it wraps) as the single grammar source so the vault gets GLSL, HLSL, GDScript, CMake, CUDA and Slang without maintaining grammars; use its `tags`/`locals` queries where present to extract definitions and references. Effort M. Risk: grammar quality for HLSL/GDScript is unknown to me; test on real engine files before relying on it.

**Fit.** adopt_dependency, conditional on a spike that confirms (a) pre-bundled offline use, (b) parse quality on C++ with Unreal macros, HLSL and GDScript, (c) licence audit of the grammars actually shipped.

---

## 12. Cross-cutting findings

### 12.1 Fit table

| Repo | Fit | One-line reason |
|---|---|---|
| SWE-agent | port_ideas | ACI principles: windowed view, summarised search, body-eliding filemap |
| mini-swe-agent | reference_only | Evidence that bash alone reaches >74% (self-reported); baseline to beat |
| OpenHands / software-agent-sdk | reference_only | Standard grep/glob/view tool shapes; no code index |
| Agentless | reference_only | Staged file, symbol, line localisation; Python-only, stale |
| LocAgent | port_ideas | Entity address scheme, four-edge graph, fold/preview formatting |
| moatless-tools | port_ideas | Ancestor-path chunk text, token-accounted result assembly |
| astchunk (cAST) | port_ideas | Sibling-merge AST chunking, nws size metric, chunk expansion |
| cocoindex | port_ideas | DP chunker, tree-trust hazards, watcher plus rescan |
| cocoindex-code | reference_only | One-tool MCP UX and hook refresh; pure dense ranking |
| chonkie | reject (for the code path) | Wrapper over tree-sitter-language-pack, no symbols, Python |
| text-splitter | adopt_dependency | Already pinned; general and fallback chunker |
| CodeNib | port_ideas | Closest competitor; tool surface, budgets, ledger, eval method |
| tree-sitter-language-pack | adopt_dependency (conditional) | Broadest grammar set including shaders, GDScript, CMake |

### 12.2 Best practices this cluster converges on

1. **Staged disclosure, always with real line numbers.** Repo tree, then file candidates, then skeleton with elided bodies (`... eliding lines a-b ...`), then exact line window. Seen in Agentless, SWE-agent filemap, LocAgent fold/preview, CodeNib L0/L1/L2 and `read_source`.
2. **Summarise before showing.** Listing files with match counts and a hard cap (SWE-agent 100 files, OpenHands 100 files) beats dumping matches; the SWE-agent authors say per-match context confused the model. Budgets are explicit and visible: 16,000-char clips (SWE-agent editor, OpenHands editor), 2,000-token and 10-hit search results (moatless), 256 KiB serialised cap (CodeNib), and when over budget the tool asks to narrow instead of truncating silently (moatless).
3. **Grep stays first-class even in "semantic" systems.** The simplest agent (mini-swe-agent, bash only) is competitive on SWE-bench-size repos; the only head-to-head retrieval numbers in this cluster (CodeNib, self-reported) have model-planned grep at 58.6% Recall@5 beating BM25 and being in the range of dense-plus-reranker (63-65%), with grep plus reranking at 71.4%. Dense retrieval alone is not the winner. So: lexical/regex precise search first, embeddings as recall booster, fuse candidates (Agentless vote, RRF).
4. **AST-aware chunks with ancestor context.** cAST, moatless, CodeNib and cocoindex all chunk on syntax boundaries and add or can add the enclosing signature path. The measured gain of AST over line chunking is real but modest (cAST: +4.3 Recall@5, +2.67 Pass@1, self-reported).
5. **Typed code graph with few edge kinds** (contains, imports, invokes, inherits; plus definition/reference) and a depth-limited, direction-aware traversal that returns a compact tree (LocAgent, CodeNib). Accuracy is bounded by name resolution; CodeNib needs SCIP/clangd for exactness and does not have it for C++ without a compile database.
6. **Fuzzy recovery on bad addresses** ("did you mean" via BM25, LocAgent; fuzzy symbol match, CodeNib).
7. **Test/implementation separation** as a filter default (moatless, LocAgent, CodeNib `filter_test`).
8. **Staleness handling.** cocoindex: content-hash memoisation, watcher plus periodic rescan. CodeNib: content-bound reads. Most research agents ignore it.

### 12.3 Gaps in every project here (the vault's opening)

- **Single repository only.** Every system indexes one repo at a time (per-instance graphs and manifests). None gives one address space, one search, and cross-repo definition following over 5-7 repositories.
- **Python-first.** LocAgent, Agentless, and SWE-agent filemap parse Python only; moatless adds Java; cAST four languages; CodeNib 12 graph languages but with per-language toolchains. No evidence of competent handling of Unreal-style macro C++, HLSL/GLSL, GDScript, or CMake. cocoindex registers HLSL/GLSL/GDScript/CUDA by extension but without grammars.
- **Macro-heavy C++.** Only cocoindex has an explicit "can this tree be trusted" guard; none runs a preprocessor-aware or macro-expansion step. CodeNib requires clangd and a compile database for C++.
- **Retrieval fusion is shallow.** cocoindex-code is dense only; Agentless fuses by vote; only CodeNib has hybrid RRF plus rerank, and its best route depends on remote models.
- **No ranking signals beyond text relevance:** no definition boost, filename boost, graph centrality or recency except mtime sort in OpenHands. Nobody implements a PageRank-style repository map in this cluster (that idea is in other clusters).
- **Few-call ergonomics.** Everyone needs several sequential calls (search, then view, then traverse). No tool takes `queries[]` batches (the vault's design does), and only CodeNib de-duplicates already-delivered content.
- **Little independent evidence.** All performance figures are self-reported by the project or authors. The one external-ish element is the swebench.com bash-only leaderboard, run by the mini-swe-agent authors.

### 12.4 What the vault must do to beat these for the target use case

For "an agent studies 5-7 game-engine and game repos, maps them, finds anything, follows definitions and references across files and repos, reads exactly what it needs, in few calls":

1. **Make the map one call.** `workspace` should return, per repository: directory tree to a requested depth with file counts and language mix, entry points, the generated repository document, and top symbols. Budgeted and cached. Agentless shows a tree in the prompt works only when pruned; Unreal needs depth control and collapse of generated/third-party folders.
2. **Be at least as cheap as `grep` + `cat`, then exceed it.** `grep(patterns[], scope)` must be ripgrep-fast and verbatim, but return grouped results (file, enclosing symbol, line numbers, capped matches) and counts, rather than file names only (OpenHands) or raw lines. Batched `queries[]` and `patterns[]` remove round trips that bash-only agents pay for.
3. **Hybrid search with symbol-aware ranking.** BM25 (tantivy) over code identifiers with camel/snake sub-token splitting, exact-definition and filename boosts, trigram for substrings, dense vectors for conceptual queries, fused with RRF. This addresses the evidence that grep-like precision and dense recall are complementary. Evaluate against the CodeNib-style Recall@5 on our own test set.
4. **Definition and reference following that works without a compiler.** Extract definitions and references with tree-sitter (tags/locals queries, cocoindex-style declaration tables), resolve by name plus scope plus import and `#include` hints, rank candidates, and label each result with a confidence tier (`exact-scope`, `unique-name`, `ambiguous`, `textual`). Use the trigram index plus ripgrep grep crates to verify reference lists. Offer a deliberate ceiling: if names are ambiguous, say so rather than guess (LocAgent and CodeNib both depend on exact backends; the vault cannot assume compile databases for Unreal).
5. **Cross-repo by construction.** One address space `repo/path#Lx-Ly`; `citations(doc_ids, direction)` and `define(terms[])` span repositories (for example engine headers referenced by game code). This is what no project here does.
6. **Macro and parse-trust honesty.** Port the hazard check; for untrusted files emit line-based chunks and flag `parse_degraded`; add a lightweight macro table (collect `#define`, UE-style macro names such as `UCLASS`, `UPROPERTY`, `GENERATED_BODY`) so symbol extraction can skip or understand them. This needs a spike on real Unreal files; no project in this cluster shows it works.
7. **Budgeted, resumable output.** Every tool takes a `budget` (tokens or chars), reports "N more omitted" with a cursor, de-duplicates ranges already delivered in this session (CodeNib ledger), and caps the serialised response. Defaults informed by this cluster: about 100-line windows, 100-file listing cap, roughly 2,000 tokens for a search page, 16 KB per read before eliding.
8. **Outline with elision as the default read for large files**, `read` with `expand` to symbol or enclosing scope, address returns real line numbers and a content hash so stale reads are detectable.
9. **Freshness.** Content-hash memoisation, watcher plus periodic rescan (cocoindex pattern), git-diff based invalidation; expose index age in `vault_info`.
10. **Prove it.** Build a small evaluation set on the owner's actual repos: questions with ground-truth files and symbols, scored for Recall@k, tool-call count, and tokens consumed, with `grep+cat` (mini-swe-agent style) and CodeNib-style tool sets as baselines. Everything above in this report is self-reported by third parties; our claim of "beats them" has to rest on our own harness.

### 12.5 Open questions and ambiguities (for the lead)

- Whether `tree-sitter-language-pack` can be used fully offline with pinned grammars, and the licences of the grammars it actually bundles (not audited).
- Real parse quality of tree-sitter-cpp, HLSL and GDScript grammars on Unreal and Godot code; not tested, no code was run.
- cAST paper's chunk-size ablation and SWE-agent paper's per-tool ablation numbers were not read (only abstracts and repo docs). The lead may want a follow-up that reads the PDFs if chunk-size defaults depend on them.
- The brief's `chonkie-inc/chonkie` and my first guess for the language pack URL had moved organisations; recorded canonical URLs above. I did not find a public repo for `chonkie-core` under `chonkie-inc`.
- OpenHands: the repository named in the brief no longer holds the agent; I studied the SDK repo instead.
- No independent (non-author) comparison of grep versus embeddings for agent localisation could be fetched; the search budget was exhausted. The CodeNib numbers should be treated as one vendor's measurement.

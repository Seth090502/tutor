# Phase 0 – Reference designs and external services

Prepared 2026-09-30 by a worker agent for the lead engineer. Scope: the eight items in the brief. Everything here is read-only research; no program, service or setting was touched.

Tags used on every number or claim:
- **[measured]**: I observed it myself in this session (API call, parsed dataset metadata, text extracted from a paper PDF).
- **[documented]**: stated by the cited source; I did not reproduce it.
- **[estimate]**: my own inference or arithmetic; treat as a hypothesis to test.

"Takeaway" lines are observations for the lead to decide on, not design decisions.

## 0. Method, caveats and one correction to a tool summary

- Papers were read from locally extracted PDF text (arXiv PDFs for 2605.05242, 2606.14885, 2607.24223, 2401.18059, 2409.04701, 2407.12883; 6 PDFs in total, saved only in the session scratchpad, not in the project). I did not download datasets. For BRIGHT I read only the Hugging Face dataset card/metadata and two sample rows per config from the datasets-server rows endpoint.
- **A web-fetch summarizer returned wrong numbers for the Direct Corpus Interaction paper** (it reported "Psychology 71.2%, Biology 68.4%, Economics 65.8%" for DCI). Those figures are not in the paper. The paper's BRIGHT table has no psychology column, and 71.2% is DR-DCI's BrowseComp-Plus accuracy from a different paper. Every number below comes from the PDF text, not from that summary. Treat any other secondhand summaries of these three 2026 papers with the same suspicion.
- OpenAlex: about 10 keyless API calls, each at most $0.001 (total about $0.003 of the $0.10/day keyless budget). No key, no secret was read or used.
- The three 2026 papers are preprints (no peer review). Their BrowseComp-Plus ablations use 100-question samples and a single small model (GPT-5.4 nano or mini). At n=100 and about 80% accuracy the 95% interval is roughly ±8 points [estimate], so differences of 2-3 points in those tables are not distinguishable from noise.

---

## 1. OpenAlex

Sources: https://help.openalex.org/ (the docs moved there; docs.openalex.org and developers.openalex.org both 301 to it), pages /api/authentication, /access/pricing, /access/example-costs, /access/snapshot, /access/sync, /data/works/attributes, /data/works/citations, the LLM quick reference https://developers.openalex.org/api/llm-quick-reference (page dated 2026-09-30), plus live calls to https://api.openalex.org.

### 1.1 Current terms (changed from the old "polite pool" model)
- Data is CC0 and free [documented]. The API now meters **usage in dollars per day**, not requests per second.
  - **No key**: $0.10/day. **Free key** (account, no payment method): $1/day, "10x the keyless budget". Budget resets at midnight UTC [documented]. I measured the keyless limit on a response: `X-RateLimit-Limit: 1000` credits, `X-RateLimit-Limit-USD: 0.1`, reset 78,895 s away [measured].
  - Hard rate cap: 100 requests/second; exceeding it or the daily budget returns 429 [documented]. Docs advise exponential backoff.
  - A key is **not required** for basic use but "strongly recommended" [documented]. Send as `?api_key=` or `Authorization: Bearer`.
  - Per-call costs [documented, and measured where marked]: singleton `/works/{id or doi:...}` **free** [measured, `X-RateLimit-Cost-USD: 0`]; list+filter **$0.0001** per call [measured: batched DOI filter, `cites:` filter, language/field counts all cost $0.0001]; search **$0.001** per call [documented; my `title.search` filter also cost $0.001 [measured]]; PDF download $0.01 each.
  - Paid: prepaid usage in $1 increments (expires 3 months after last purchase); annual Member $5,000/yr ($20/day budget), Member+ $10,000/yr ($100/day, daily snapshot, "updated since" sync filters), Partner from $20,000/yr [documented].
- **Polite pool / `mailto=`**: not mentioned anywhere in the current docs (zero hits for "mailto" or "polite" across the pricing, authentication, example-costs, snapshot, sync, attributes and quick-reference pages, and the site index llms.txt) [measured by search of fetched pages]. A call with `mailto=` returned the same limit headers as one without [measured, n=1]. Do not design around a polite pool.
- Query limits: `per_page` max 100, up to 100 OR-values per filter, basic paging limit 10,000 results (use cursor paging beyond) [documented].
- Default corpus is the curated "core" (~327M works [measured: `meta.count` 326,797,579]). `corpus=all` adds the ~193M-work expansion corpus (formerly "XPAC"); the snapshot contains all of it [documented].

### 1.2 Fields useful for verifying title / authors / DOI / ISBN and for citation edges
Documented work attributes (https://help.openalex.org/data/works/attributes):
- Identity: `id` (W-id), `doi` (always the published version's DOI if several), `ids` (`openalex`, `doi`, `mag`, `pmid`, `pmcid`), `title`/`display_name`, `type`, `language` (metadata language auto-detected, not necessarily the document language), `publication_year`/`publication_date`, `biblio` (volume/issue/pages), `is_retracted` (from Retraction Watch), `is_paratext` (deprecated, use `type`).
- Authors: `authorships` (author id/display name/ORCID, position, institutions, `raw_author_name`, `raw_affiliation_strings`, `is_corresponding`), capped at the first 100 authors.
- Abstract: `abstract_inverted_index` only (no plaintext, "for legal reasons"); null for some works.
- Citation edges: `referenced_works` (outbound OpenAlex IDs) and `referenced_works_count`; `cited_by_count`; inbound edges via filter `cites:W...`; `related_works`; `fwci`, `citation_normalized_percentile`. Edges are built by matching extracted reference lists first by DOI, otherwise by metadata, so `referenced_works` can be shorter than the printed reference list [documented, /data/works/citations].
- Full text pointers: `has_content` (`pdf`, `grobid_xml`; >50M works) and `content_urls` (API-only, key needed, $0.01/file).
- **ISBN: no ISBN field exists on works** (zero occurrences of "isbn" in the works attribute dictionary, the quick reference, or the access overview) and a filter on `ids.isbn` returned HTTP 400 [measured]. Books must be matched by DOI where one exists, otherwise by `title.search` plus author, which costs search-rate $0.001/call [measured]. This is a gap for the 10k-book target; ISBN verification needs another source (not evaluated here).
- Measured examples (n=1 each, anecdotes, not rates):
  - DOI singleton `/works/doi:10.1037/0033-295X.84.2.191` (Bandura 1977) returned correct title and `cited_by_count` 43,861 but `referenced_works_count: 0`, and the author's institution came back as "University of Indonesia / Faculty of Psychology, Universitas Indonesia", which is plainly a mis-assigned affiliation for that paper [measured]. Metadata verification should compare title and DOI but should not trust affiliations or reference lists blindly.
  - Batch lookup `filter=doi:a|b|c` returned 3 records for 1 credit ($0.0001) [measured]. `cites:` filter returned a count of 10,416 for one work in one $0.0001 call [measured].
  - Chinese: 5,425,754 works have `language:zh` metadata [measured]. This counts metadata language, so it is not a coverage guarantee for Chinese journal papers.
- `select=` accepts top-level fields only; `select=authorships.author.display_name` returned HTTP 400 [measured].

### 1.3 Snapshot versus API for about 160k documents
- Snapshot [documented, /access/snapshot and /access/sync]: S3 bucket `s3://openalex/data/`, anonymous (`--no-sign-request`), two complete copies of the same data: JSON Lines gzip (~745 GB, works alone ~615 GB) and Parquet snappy (~770 GB); ~626M records per format as of the September 2026 release. Partitioned by `updated_date`, part files up to 400,000 records. Free public snapshot is **quarterly**: second Wednesday of Jan/Apr/Jul/Oct; next is **2026-10-14**, then 2027-01-13. The bucket holds only the current release (no history). Daily full snapshots are paid (Member+ and up).
- API [documented + measured]: free singleton lookups by DOI (no cost, within the 100 req/s cap); batched OR filters at $0.0001 per 100 DOIs.
- Arithmetic for 160k documents [estimate from the measured/documented rates]: 160,000 DOI singletons is free and takes at least about 27 minutes at the 100 req/s cap; 1,600 batched calls cost about $0.16 (above the keyless $0.10/day, inside the $1/day free key). Citation edges arrive in the same records (`referenced_works`); inbound counts are one `cites:` call per work or are read from `cited_by_count`.
- Takeaway: the ~750 GB snapshot is the wrong tool for verifying 160k known documents, and it would consume most of a 1 TB vault SSD. The API with a free key is sufficient. Snapshot only makes sense if the lead wants OpenAlex as a local graph for corpus discovery or citation traversal at scale (then Parquet + DuckDB per the docs); that is a scoping decision, not evaluated here.

---

## 2. BRIGHT (reasoning-intensive retrieval benchmark)

Sources: https://brightbenchmark.github.io/ (a project page with little text), https://huggingface.co/datasets/xlangai/BRIGHT (card + `api/datasets` metadata + datasets-server), https://github.com/xlang-ai/BRIGHT README, paper https://arxiv.org/abs/2407.12883 (ICLR 2025; v4 of 26 Mar 2025).

### 2.1 What it is
- 12 datasets, 1,384 queries per the abstract (the GitHub header comment says 1,385) [documented]. StackExchange domains (biology, earth_science, economics, psychology, robotics, stackoverflow, sustainable_living), coding (leetcode, pony), and theorem-based (aops, theoremqa_questions, theoremqa_theorems). Queries are long StackExchange posts; documents are web pages (blogs, news, Wikipedia, articles) that answers linked to, split into chunks [documented].
- **Licence: CC-BY-4.0** on the Hugging Face card and the GitHub repo [documented]. The documents are third-party crawled pages, and the card does not address their separate copyright; keep BRIGHT as an external eval fixture, outside the vault's canonical store [estimate of prudent handling].
- Dataset last modified 2025-03-01, ~29k downloads [measured, HF API].

### 2.2 The three splits asked about (plus the two the agent papers use)
Measured from the dataset card YAML (`num_examples` / `num_bytes`):

| split | queries | chunk docs (`documents`) | chunk text | long pages (`long_documents`) | long text |
|---|---|---|---|---|---|
| psychology | 101 | 52,835 | 23.2 MB | 512 | 20.5 MB |
| biology | 103 | 57,359 | 22.0 MB | 524 | 19.5 MB |
| economics | 103 | 50,220 | 22.8 MB | 516 | 20.1 MB |
| (earth_science) | 116 | 121,249 | 47.0 MB | 601 | 41.8 MB |
| (robotics) | 101 | 61,961 | 20.7 MB | 508 | 18.2 MB |

Psychology + biology + economics together: 307 queries, 160,414 chunk documents, ~68 MB of chunk text [measured, arithmetic on the above]. This is about 1/40 of the target 6M chunks [estimate], so BRIGHT is a quality probe, not a scale test. Whole `documents` config for all 12 splits is ~465 MB download; per-split parquet files exist (`documents/psychology-00000-of-00001.parquet` etc.) [measured, HF file list], so the three splits can be fetched alone (size not measured).

### 2.3 Structure (two rows read per config, psychology split) [measured]
- Config `examples` (queries): `id` ("0", "1"...), `query` (StackExchange question text), `reasoning` (human gold reasoning, not used in the paper's experiments [documented]), `gold_ids` (chunk ids), `gold_ids_long` (page ids), `gold_answer`, `excluded_ids` (`["N/A"]` for StackExchange splits; used only for theoremqa/aops/leetcode [documented]).
- Config `documents` (retrieval corpus at chunk level): `id` like `horn_effect/attractivepeoplegetunfairadvantagesatworkaicanhelp_65.txt`, `content`. Chunk id = `<topic folder>/<page>_<chunk number>.txt`. Example gold ids for query 0: `confirmation_bias/seeds_model_11.txt`, `..._12.txt`, `..._9.txt` and others (6 gold chunks); corresponding `gold_ids_long`: `confirmation_bias/seeds_model.txt` etc. (3 pages).
- Config `long_documents`: same naming without the `_N` suffix, whole pages. Contents are raw crawl text, including navigation chrome and ads ("Jump to content / Main menu", "CNN values your feedback"), so the corpus is noisy.
- Average chunk length is tiny for the StackExchange splits: 48 words (biology), 52 (economics) per the DCI paper's corpus table [documented, DCI Table 7], versus roughly 400-token chunks the vault is likely to use [estimate]. Gold chunk sets are multi-chunk per query.
- Metric: nDCG@10 [documented].

### 2.4 Published reference numbers on our three splits (nDCG@10)
From the BRIGHT paper Table 2 [documented, read from PDF text], psychology / biology / economics:
BM25 12.5 / 18.9 / 14.9; Qwen (the table's "Qwen" row; exact model not checked) 24.6 / 30.6 / 17.8; Google 27.8 / 22.7 / 19.6; OpenAI 27.6 / 23.3 / 19.5; Inst-XL 27.4 / 21.6 / 22.4; SFR 19.0 / 19.1 / 17.8.
Agent systems from the 2026 papers report **biology, earth science, economics, robotics but not psychology** (DCI-Agent-CC with Claude Sonnet 4.6: biology 77.1, economics 46.8; DCI-Agent-Lite with GPT-5.4 nano: 60.0 / 32.3; ReasonRank-32B reranker: 58.2 / 36.6; RARG+ with GPT-5.4-mini: 66.70 / 37.23). So psychology, the owner's first domain, has no published agentic baseline in these papers; any result we produce there is new, and cross-paper comparison on psychology is impossible.
Caveat: the BRIGHT corpora are public web text likely seen by the models during training [estimate; the paper's contamination treatment was not checked], so use BRIGHT to compare our own retriever configurations against each other, not as an absolute score.

### 2.5 Running it against our own retriever
- Official harness [documented, GitHub README]: `python run.py --task psychology --model <name>`; add a custom model by implementing `retrieval_model_function_name(queries, query_ids, documents, doc_ids, excluded_ids, **kwargs)` in `retrievers.py` and registering it in `RETRIEVAL_FUNCS`; it must return `{query_id: {doc_id: score}}`. Options: `--long_context` (page-level setting), `--query_max_length`, `--doc_max_length`, `--output_dir`, `--cache_dir`. Install needs Python 3.10, a conda environment and `openjdk=22` (for the BM25 baseline via a Java stack). The harness lists 13 built-in models.
- Alternative without the harness (simpler to wire to a Rust retriever) [estimate]: pull the three `examples` and `documents` parquet splits, ingest each chunk as one vault document keyed by its BRIGHT id, run each `query` through the retriever (a long StackExchange post, so query length limits and rewriting matter), emit the top-k ranking keyed by BRIGHT id, and score nDCG@10 against `gold_ids` (chunk level) or `gold_ids_long` (page level) with any qrels scorer. The `{doc_id}#L{start}-L{end}` addressing maps naturally if each chunk is a vault doc, or if long pages are ingested and returned line ranges are mapped back to chunk ids (design decision for the lead).
- To evaluate an **agentic** tool surface rather than a ranker, the DCI papers give the agent the raw corpus as files (one file per document) and have it output a ranked list; DCI-Agent-Lite and DR-DCI code are MIT-licensed (below).

---

## 3. The three 2026 agentic-search papers

All three exist [measured: fetched arXiv/Hugging Face pages and PDFs]. Code repositories: DCI https://github.com/DCI-Agent/DCI-Agent-Lite (MIT, last push 2026-06-01), DR-DCI https://github.com/EigenTom/DR-DCI (MIT, 2026-08-02), RARG https://github.com/LeqsNaN/RARG (MIT, 2026-09-24) [measured via GitHub API].

### 3.1 Direct Corpus Interaction (arXiv 2605.05242, submitted 3 May 2026)
"Beyond Semantic Similarity: Rethinking Retrieval for Agentic Search via Direct Corpus Interaction", Li, Zhang, ..., Chen, Lin, Jiang, Zhang (Texas A&M, Waterloo and others).
- **Mechanism**: no embedding model, index or retrieval API. The agent searches the raw corpus (one file per document) with `bash` (grep/rg/find/glob/head/sed, pipes) and `read`. Two scaffolds: DCI-Agent-Lite (Pi-based, only bash+read, GPT-5.4 nano) and DCI-Agent-CC (Claude Code, Sonnet 4.6, web and subagents disabled) [documented].
- **Measured gains** [documented]:
  - BrowseComp-Plus (830 questions, Sonnet 4.6): Qwen3-Embedding-8B retrieval tool 69.0% to DCI 80.0% (+11.0 points); cost $1,440 to $1,016. GPT-5.4 nano Lite: 62.9% at $93.
  - BRIGHT biology/earth/economics/robotics + BEIR ArguAna/SciFact (nDCG@10 average): DCI-Agent-CC 68.5 vs best retrieval baseline ReasonRank-32B 47.0; DCI-Agent-Lite 56.7; BM25 20.3. Per-split for CC: 77.1 / 69.0 / 46.8 / 56.8 / 85.3 / 75.7. Multi-hop QA (6 datasets, 50-question samples except Bamboogle): CC average 83.0 vs strongest retrieval agent 52.3.
- **Why it wins**: not higher recall of gold documents but "retrieval interface resolution". On a 100-question subset, Lite had mean gold coverage 28.0 vs 56.7 for dense retrieval but any-gold coverage was similar (70.0 vs 74.0), and **localization** (how small a span the agent reaches in a gold doc) was 48.4 vs 21.7; accuracy 73 vs 45. Claude Code's tool use was 62.4% Bash and 33.0% Grep, with only 9.1% full-document reads [documented].
- **Tool-set ablation** (n=100, GPT-5.4 nano): `read + grep` only: 61% at $0.0355/question and 19 tool calls; open bash: 73% at $0.1021 and 35 calls; BM25 retrieval tool 32%; dense tool 45% at $0.0498. "A small set of bash commands is sufficient to unlock most of the improvement" [documented].
- **Context management** (levels L0-L4; truncation of each tool result to 20K chars; "compaction" replaces old tool results with placeholders once accumulated output passes 240K chars keeping the last 12 turns; optional summarization): accuracy 72 / 75 / 69 / 77 / 73 across L0-L4, non-monotonic, differences within noise at n=100 [documented, my reading of noise is an estimate].
- **Scaling limit**: expanding BrowseComp-Plus from 100K to 200K documents with FineWeb distractors raised tool calls 38.5 to 86.9, more than doubled latency and cost, and cut accuracy 13.6 points; at 400K accuracy fell to 37.5% and 20 examples hit the tool budget [documented]. BrowseComp-Plus documents average 5,179 words (about 100K docs) [documented], so the vault's ~2.5B tokens is several times larger than where raw grep already broke [estimate].
- **Takeaway**: the "grep and read" tool pair, with line-addressed matches and offset reads, is the high-value interface. The paper itself shows full-corpus grep does not scale past a few hundred thousand documents, which is what the next two papers address.

### 3.2 DR-DCI (arXiv 2606.14885, submitted 12 June 2026)
"DR-DCI: Scaling Direct Corpus Interaction via Dynamic Workspace Expansion", Lu, Li, Nie, ... Jiang, Zhang. Retrieval narrowing first, then grep and read over a bounded workspace.
- **Mechanism**: retrieval is an agent-callable action `pull(query, topK)`. The harness retrieves from the whole corpus, deduplicates against documents already visible, materializes the new ones into a per-query workspace directory, and returns the newly added documents, a compact ranked preview and workspace statistics. The agent then uses rg/grep/find/read inside the workspace (inter-document and intra-document operations) [documented].
- **Tool/harness design details worth noting** [documented, Appendix A.8 and A.9]: hard-link materialization (no copies) from an immutable corpus store, agent restricted to search and read; **root-flat workspace** with deduplicated, shell-safe normalized filenames (original titles, URLs, ranks kept in metadata) because per-pull folders and rank-encoding paths hurt accuracy; retrieval rank is disclosed through the tool response, not paths; bash output capped at last 2000 lines or 10 KB with an explicit continuation hint; long lines clipped with a ready-made `read` call (path, offset/charOffset, limit); 30 s tool timeout; parallel tool execution.
- **Measured gains** [documented]: on BrowseComp-Plus (830 questions, GPT-5.4 nano) accuracy 62.9% (raw DCI) to **71.2%** (+8.3), average wall time 3,139 s to 146 s, cost $88.13 to $34.91, average tool calls 37.5 to 30.9. A "workspace-preserving context reset" (discard the reasoning history, keep the workspace, re-run raw DCI when the first pass had confidence ≤70 and abstained; 49 cases) raised it to 73.25% at $39.35.
- **Scaling** [documented]: corpus grown from 100K to 10M documents (FineWeb distractors, BCP-100 subset) accuracy 80/100 to 70/100, workspace stays about 1K-1.4K docs, cost about $4-5 per 100 questions; raw DCI hit tool timeouts and could not be completed beyond its measurable range; a 20M-document Wiki-18 file-per-document QA setting averaged 63.0 (50-question samples).
- **Ablations on BCP-100 (n=100, so ±8 points)** [documented]: dynamic pull 82/100 at $3.44 vs one-shot "single pull" of top-500, 79/100 at $8.83; ranked preview 82 vs shuffled preview 76 vs hidden preview 72; **blocking inter-document search while keeping the ranked preview: 82 to 40**; BM25 backend 80/100 ($5.17) vs dense Qwen3 8B backend 82/100 ($4.03), so a sparse retriever "remains effective".
- **Takeaway**: the design (retrieve-then-narrow, then exact tools over the narrowed set) is what the vault's BM25 + trigram + dense stack serves. Cross-document regex over the pulled set was the most important single component by their ablation. Sparse-only retrieval lost only about 2 points of 100 (not significant), so BM25 can be the first narrowing path.

### 3.3 RARG (arXiv 2607.24223; Hugging Face papers page huggingface.co/papers/2607.24223; v2 of 3 Aug 2026)
"A New Role for Relevance: Guiding Corpus Interaction in Agentic Search", Li, Li, Yu, Zhang, Zhou (Tencent, IIE-CAS). Relevance as an execution prior for ripgrep.
- **Mechanism** [documented]: built on DCI-Agent-Lite (tools `Bash` and `Read`; ls/find over the whole corpus forbidden). New tool `embed_recall(scope_query)` ranks the corpus with an embedding model and writes ranked document paths (capped at 10,000) to `/tmp/scope_N.txt`, returning only the path-to-query mapping. The agent runs `cat scope | xargs rg ...`; the harness injects `-j1` so ripgrep scans sequentially in rank order, because its default multi-threaded output would destroy the order. Scope mappings survive context compaction.
- **RARG+**: appends the top-10 query-relevant paragraphs (400-1000 chars) from the top-ranked scope documents to the recall output as entry points. **RARG++**: reranks a pool of up to M=500 rg matches with the embedding model using a constructed query ("Query: scope query / RG focus: keywords from the rg pattern") and shows only the top m=30 (BrowseComp-Plus) or 60 (BRIGHT), each match truncated to 1,000/500 chars. A variant where the LLM writes its own rerank query ("generative") did worse (75%) [documented].
- **Measured gains** [documented, BrowseComp-Plus 100-question RISE sample, 100K docs]: GPT-5.4-mini: RARG 80 / RARG+ 81 / RARG++ **84** vs DCI 78 and RISE 78; average tool calls 29.8 / 29.6 / **23.9** vs DCI 99.1 and RISE 28.7. GPT-5.4: 91 vs RISE 82. GPT-5.4-nano: 73 / 74 / 79 vs DCI 71. Corpus grown to 1M documents (900K long FineWeb-Edu pages): RARG++ 79 vs RISE-BM25 69.
- **BRIGHT (biology/earth/economics/robotics, nDCG@10, GPT-5.4-mini)**: RARG+ 53.36 best, NeMo agent 52.89, RARG 51.75, **RARG++ 50.55**, DCI 48.43, RISE-BM25 41.60. Match-level reranking helped QA (single-path, depth-first) but did not help ranking-style retrieval (breadth-first). Embedders used: Qwen3-Embedding-4B for BrowseComp-Plus; llama-nv-embed-reasoning-3b for BRIGHT ranking and Qwen3-Embedding for match rerank. Not tested here: any Chinese or psychology data.
- **Takeaway**: with a dense index, the cheap move is to feed ranked paths into the grep tool's traversal order and to rerank matches before truncation. The truncation rule (max matches and max chars per match) is where relevance matters. The paper's own limits: depends on embedding quality; scaling to 1M still dropped accuracy for all methods.

### 3.4 What is shared across the three (for tool design)
- Two primary tools: line-addressed pattern search and offset reads, with bounded output and explicit continuation hints; narrow the corpus with a retriever before regex rather than scanning everything.
- Retrieval should expose rank through the tool response while keeping document identities stable and shell-safe.
- None of the three reports Chinese text, books, or psychology; none reports grounding/citation accuracy, only answer accuracy or nDCG.

---

## 4. PaperQA2 (Future-House/paper-qa)

Sources: https://github.com/Future-House/paper-qa (Apache-2.0, last push 2026-09-25 [measured]); README; `src/paperqa/agents/tools.py` read via the GitHub API; paper https://arxiv.org/abs/2409.13740.
- **Tool set in the open repo** [documented, `tools.py`]: `paper_search` (LLM-generated keyword query, optional min/max year, adds papers to state), `gather_evidence` (question-focused), `gen_answer`, `reset`, `complete`, and `clinical_trials_search`. The agent may call them in any order; a fixed "fake" agent path (search, gather, answer) exists to cut tokens.
- **Evidence gathering ("RCS", ranking and contextual summarization)** [documented]: embed the query, rank top-k chunks from the current state (`answer.evidence_k` default 10, MMR parameter available), have an LLM produce a scored summary of each chunk in the context of the question (about 100 words by default), have an LLM re-score and select the best summaries, then answer from the best summaries with in-text citations (`answer_max_sources` default 5). Chunk text is embedded with document metadata ("metadata-awareness") [documented].
- **Metadata and citations**: fetches metadata redundantly from Crossref, Semantic Scholar, OpenAlex and Unpaywall, including citation counts with a retraction check [documented].
- **Citation traversal is not in the open repo**: the FAQ says FutureHouse internally has extra tools "like citation traversal" they are "trying to get ... into this repo" [documented, README FAQ]. Their paper results were obtained with licensed paper access the repo cannot share.
- **Measured claims** [documented, abstract of 2409.13740]: matches or exceeds subject-matter-expert performance on three literature tasks; introduces LitQA2; 2.34 ± 1.99 contradictions found per biology paper, 70% validated by experts. No retrieval nDCG comparison with the vault's planned stack is given.
- Uses tantivy (via its local index) among its dependencies [documented].
- **Takeaway**: the summarize-then-rescore step is an LLM cost per chunk that the vault design appears to replace with the calling agent's own reading. The citation-edge verification pattern (metadata from several providers, retraction flag) is the part directly relevant to Phase 0's OpenAlex work.

---

## 5. Zoekt and Russ Cox's trigram index

Sources: https://swtch.com/~rsc/regexp/regexp4.html (Cox, "Regular Expression Matching with a Trigram Index"); https://github.com/sourcegraph/zoekt `doc/design.md` (Apache-2.0, last push 2026-09-23); tantivy 0.26.2 docs for `NgramTokenizer` and `RegexQuery` (docs.rs); `regex-syntax` 0.8.11 docs.

### 5.1 How regex becomes a trigram query
- **Cox** [documented]: an inverted index over every 3-byte substring ("too few distinct 2-grams, too many distinct 4-grams"). A regex is analysed bottom-up into a boolean AND/OR expression of trigrams that any matching text must contain (for `Google.*Search`: `Goo AND oog AND ogl AND gle AND Sea AND ear AND arc AND rch`). The analysis tracks per sub-expression: whether the empty string matches, the exact set of matches (or unknown), sets of prefixes and suffixes, and a match query. If a part yields fewer than three characters the trigram function returns ANY (no constraint). The index returns **candidate** files; the real regex then runs on each candidate, so false positives are removed and there are no false negatives. Measured by Cox on Linux 3.1.3 (420 MB): index 77 MB (~20% of corpus); the query `hello world` narrowed 36,972 files to 25, about 100x faster; case-insensitive search needs all case variants of each trigram, narrowing to 599 files and about 20x faster [documented].
- **Zoekt** [documented]: positional trigrams (stores each trigram's offset per file). A string search uses only two trigrams (start and end) and checks they sit at the right distance apart, so only two posting lists are touched and they may live on SSD; it may pick the two trigrams with the fewest matches. A regex is reduced to substring atoms combined with AND/OR (example: `(Path|PathFragment).*=.*/usr/local` becomes `AND(OR(substr "Path", substr "PathFragment"), substr "/usr/local")`), then candidate documents run the full regex. Index is ~3.5x corpus size including content; shards are mmap'd; case-insensitive search ORs over case variants; UTF-8 handled with rune offsets.

### 5.2 Mapping onto a tantivy ngram field
- Tantivy provides `NgramTokenizer::new(min_gram, max_gram, prefix_only)`; with `(3,3,false)` it emits all character trigrams [documented]. **"With this tokenizer, the position is always 0"** (and `POSITION_GAP * value index` across multiple values of one field) [documented, tantivy 0.26.2]. So a tantivy trigram field cannot do Zoekt-style adjacency or distance checks; it behaves like Cox's non-positional per-document trigram index [my inference from the documented behavior].
- Resulting pattern [estimate / design idea, not tested]: one tantivy document per chunk with a trigram field (lowercase filter before the n-gram, record option docs-only so no frequencies or positions are stored); translate regex to an AND/OR tree of `TermQuery` on trigrams (AND of `Must` terms; OR as `Should` groups); take the candidate chunks; run the real regex over those chunks' text in the canonical Markdown store; return `{doc_id}#L{start}-L{end}` from the verified matches. A tantivy `Searcher` can look up per-term document frequency [exists in tantivy; exact API name and cost not verified here], so choosing only the K rarest trigrams (Zoekt's trick) would avoid intersecting a posting list for common trigrams that spans millions of chunk documents.
- **Not equivalent**: tantivy `RegexQuery` matches a regex against whole **terms** in a field's term dictionary (an FST) [documented example `d[ai]{2}ry` on a text field]. On a word-tokenized field it cannot match substrings or patterns that cross word boundaries, so it does not replace the trigram path.
- The Rust `regex-syntax` crate (0.8.11) has `hir::literal::Extractor`, which extracts prefix or suffix literal sequences from a parsed regex [documented]. It does not itself build Cox's AND/OR trigram query, so the translation layer must be written (or ported from google/codesearch's `index/regexp.go`, the file Cox names) [estimate].
- **Unicode / Chinese**: tantivy's ngram tokenizer works on characters (its non-ASCII example shows byte offsets for multi-byte characters) [documented]. For CJK, a 3-character gram is a lot more selective than a Latin trigram, and short 2-character CJK terms would degrade to ANY under the trigram rule, so a bigram field may be needed for Chinese [estimate; to measure on real data].
- **Index size** at 6M chunk documents is unknown. Cox's ~20% and Zoekt's ~3.5x (positions plus content) bracket it; treat any figure for the vault as to be measured on a sample [estimate].

---

## 6. Contextual Retrieval, RAPTOR, Late Chunking

### 6.1 Anthropic Contextual Retrieval (blog, published 2024-09-19) https://www.anthropic.com/news/contextual-retrieval
- Method [documented]: an LLM writes a 50-100 token context for each chunk from the whole document ("situate this chunk within the overall document"), prepended before embedding and before building the BM25 index.
- Gains [documented, Anthropic's own experiments, not peer reviewed]: top-20-chunk retrieval failure rate (1 minus recall@20), averaged across codebases, fiction, arXiv papers and science papers, with Gemini Text 004 embeddings:
  - contextual embeddings alone: 5.7% to 3.7% (-35%);
  - contextual embeddings plus contextual BM25: 5.7% to 2.9% (-49%);
  - plus a Cohere reranker: 5.7% to 1.9% (-67%).
  Per-domain breakdowns are only in appendix charts. Improvement held for every embedding and source combination they tested; Gemini and Voyage embeddings did best. They recommend a domain-tailored contextualizer prompt, and note that knowledge bases under 200,000 tokens can simply go in the prompt.
- Cost [documented]: about $1.02 per million document tokens one-time, assuming prompt caching, 800-token chunks, 8k-token documents, 50-token instructions and 100 tokens of context per chunk. At that rate 2.5B tokens would be about $2,550 on the 2024 API price list [estimate, plain multiplication]. On a local engine the cost is GPU time instead: roughly 6M chunks times about 100 generated tokens is about 600M generated tokens plus a prefill per chunk unless prefix caching holds the document [estimate]. Full papers often exceed the 8k-token document in their assumption [estimate].
- A late-chunking-paper replication of this blog used `claude-3-haiku-20240307` as the contextualizer [documented, arXiv 2409.04701 Section 4.5] (only a one-example demonstration there, not a benchmark).

### 6.2 RAPTOR (arXiv 2401.18059, ICLR 2024, Stanford)
- Method [documented]: split text into ~100-token chunks (measured average leaf 85.6 tokens), embed (SBERT), cluster with UMAP + Gaussian mixture models, summarize each cluster with an LLM (gpt-3.5-turbo; average summary 131 tokens, 72% compression, 6.7 children per parent), re-embed and repeat to build a tree. Query by "collapsed tree" (search all nodes together) or layer-by-layer traversal. Build cost and time scale linearly with document length (appendix).
- **Headline gains and what they actually compare** [documented, read from PDF]:
  - QuALITY with GPT-4: 82.6% vs previous best 62.3% (+20.3 absolute); +21.5 over CoLISA on QuALITY-HARD. QASPER with GPT-4: F1 55.7 vs prior best CoLT5 XL 53.9. These are comparisons against other full systems, not retrieval ablations.
  - Controlled same-retriever comparisons (RAPTOR vs without), the fair measure of the tree: QuALITY (UnifiedQA 3B) SBERT 56.6 vs 54.9, BM25 52.1 vs 49.9, DPR 54.7 vs 53.1 (+1.7 to +2.2); QASPER F1 (UnifiedQA) SBERT 36.70 vs 36.23, BM25 27.00 vs 26.47, DPR 32.23 vs 31.70 (about +0.5); NarrativeQA ROUGE SBERT 30.87 vs 29.26, BM25 27.93 vs 23.52, DPR 30.94 vs 29.56. Versus other retrievers with GPT-4 on QASPER F1: RAPTOR 55.7, DPR 53.0, BM25 50.2.
  - Share of retrieved nodes that were non-leaf summaries: NarrativeQA 57.36% (DPR) / 36.78% (SBERT), QuALITY 32.28% / 24.41%, QASPER 22.93% / 18.49% [documented, Table 18].
- Conditions: tasks are questions over a single long document (stories, a paper, a passage), with LLM summaries as the generator; none is corpus-wide retrieval across many papers.
- Takeaway: the tree matters for thematic or whole-document questions; on QASPER (full-text papers) the controlled gain over a flat retriever is under one F1 point for UnifiedQA.

### 6.3 Late Chunking (arXiv 2409.04701, Jina AI, v3 7 Jul 2025)
- Method [documented]: run a long-context embedding model over the whole document, then mean-pool token embeddings per chunk span, instead of embedding chunks independently. Needs no extra training; they also propose a fine-tuning method. "Long late chunking" handles documents longer than the model window using overlapping macro-chunks.
- Gains [documented, nDCG@10, 4 small BEIR datasets SciFact/NFCorpus/FiQA/TRECCOVID, 3 embedding models]: replacing naive with late chunking averaged **+3.63% relative (+1.9 absolute)** for sentence-boundary chunks; fixed 256-token chunks average 52.2 to 54.0. Largest per-dataset effect: NFCorpus jina-embeddings-v2-small 23.5 to 30.0 (fixed 256). With jina-v3 on the same four datasets, gains are smaller. Late chunking beat naive chunking most at small chunk sizes; for some reading-comprehension tasks (LongEmbed) naive was better with large chunks. On LongEmbed, chunking itself (512 tokens) beat no-chunking by an average +24.47% relative.
- Conditions: needs an embedding model with long context and access to token-level outputs before pooling, and mean pooling. I believe Qwen3-Embedding pools the last token rather than the mean (my recollection, not checked in this session), which would make the method not directly applicable to that model; also unverified is whether llama.cpp exposes unpooled token embeddings for it. Both are cheap to check in Phase 0 [estimate].
- Comparison with Contextual Retrieval in the paper is a single toy example, not a benchmark [documented].

---

## 7. Swiftide and Rig (brief)

- **Swiftide** (https://github.com/bosun-ai/swiftide, MIT per README badge; 787 stars; latest published release v0.32.1 on 2025-11-15 per both GitHub and crates.io, but the repository was pushed 2026-09-28 [measured]). README describes an agent harness (LLM loop, tool calls, lifecycle hooks, stop conditions), typed task graphs, and streaming RAG pipelines (`Pipeline::from_loader(...).then_chunk(...).then_in_batch(Embed).then_store_with(...)`), with caches, batch transformers, MCP toolboxes, tracing/Langfuse; integrations list Qdrant, LanceDB, PgVector, DuckDB, Redb, Redis, and providers including Ollama and OpenAI-compatible; it is "pre-1.0" and APIs "can change" [documented]. No tantivy or tonic integration is listed. Unverified: whether the task-graph features the master README describes are in the 0.32.1 release.
  Worth borrowing as patterns [estimate]: tools as plain async functions with a generated JSON schema; a streaming ingestion pipeline with a batch stage for embedding calls and a content-hash cache so re-ingest is idempotent.
- **Rig** (https://github.com/0xPlaygrounds/rig; rig-core 0.43.0 published 2026-09-30, ~3.1M downloads; 8.8k stars [measured]). README warns of upcoming breaking changes; provides provider-neutral completion and embedding traits, 20+ model providers, 10+ vector-store integrations, tool contracts, a serializable agent run state machine, typed hooks, GenAI OpenTelemetry conventions; MCP via a `rig-rmcp` crate that is native-only [documented].
  Worth borrowing as patterns [estimate]: the provider-neutral model trait (to talk to the owner's own inference engine and llama.cpp sidecars without coupling), and typed hooks around tool calls for logging.
- Observation: libraryd's role is a retrieval server with MCP/gRPC tools; the agent loop lives in the tutor and Claude Code. Neither crate's agent features are needed there; at most the ingestion-pipeline shape and the model-trait idea transfer. Adopting either as a dependency means tracking pre-1.0 or fast-moving APIs.

---

## 8. Anthropic claude-api skill: build-eval and hillclimb

Sources: GitHub https://github.com/anthropics/skills/tree/main/skills/claude-api (last commit touching it 2026-09-29 [measured]); local bundle `C:\Users\sethy\AppData\Local\Temp\claude\bundled-skills\2.1.285\87f46b01c0cfe1151d9c368f7c62e7bd\claude-api\shared\evals\` containing `build-eval.md`, `eval-audit.md`, `eval-hillclimb.md` (read the first and last in full). The local and GitHub copies differ slightly in size (build-eval 55,438 vs 54,819 bytes; eval-hillclimb 69,502 vs 68,773); I did not diff them, so I have not established whether the differences are content or encoding. GitHub also lists a `cost-hillclimb.md` that the local bundle copy references.

- **Subcommands** (skill's SKILL.md [documented]): `/claude-api build-eval` runs the guide `shared/evals/build-eval.md`; `/claude-api hillclimb` runs `shared/evals/eval-hillclimb.md`. Both direct the agent to read `eval-audit.md` first.
- **build-eval does** [documented]: an interview to build a runnable eval for **one flow** of a Claude-powered app. It (1) picks the flow and finds the real entry point, (2) sources 15-100 inputs (production transcripts, tickets, hand-written, then synthesized last), requiring the user's explicit sign-off on the actual inputs, (3) chooses a grader (programmatic check, pairwise blind comparison, pointwise rubric by a judge model, or human spot-check) and requires sign-off on the grading after showing about five graded examples, (4) writes a runner and per-case results (`results.jsonl`, `traces/<id>_rep<k>.json`, `errors.jsonl`) under `.claude/hillclimb/<flow>/baseline/` with hard per-case timeout, jittered backoff, resume at (case, rep), served-model assertion, then (5) runs oracle and null controls and an audit, states cost from a measured pilot, and proceeds only on a yes before the first paid pass.
- **hillclimb does** [documented]: requires a runnable eval (else routes to build-eval). Establishes goal (raise the metric, cut cost, cut latency, or migrate model), what is in scope and off-limits, stopping rule, and approves a plan. Fixes a **train/test split** stratified by tag; a fresh analyzer subagent reads only train transcripts each round and proposes one change; the full set is rerun every round; the **held-out test score** picks the winner and is the headline; on a stall it categorizes failures (artifact gap, grader disagreement, harness error, structural, variance). Before round 1 it runs "can the eval be climbed" checks: noise floor vs headroom vs the smallest win worth acting on (about 1/sqrt(n·reps) for a pass rate, e.g. 25 cases x 2 reps is about ±14 points), mechanism wired, headline recomputed from raw rows, grader spot-check.
- **Requirements** [documented]: an app that calls a Claude provider (first-party API, Bedrock, Vertex, Foundry, or the Claude Platform on AWS); the runner and judge must use the same provider client; `node` or `bun` on PATH for the runner scaffold and the report builder (a Python-only project writes the runner itself against the file contract); the `AskUserQuestion` tool for sign-offs (falls back to prose when headless); paid model calls with a consent step; ground-truth must not be reachable from the model's context; the user, not the agent, must pass the harness-integrity flag `--approve-harness` for the runner scaffold; the full HTML viewer (`build-report.mjs`) is only present in the EAP package, the CLI extracts the lite builder; hillclimb runs the eval as a background `Bash` call.
- **Fit to this project** (observations): the tutor runs on the owner's own inference engine, not a Claude provider, so the commands themselves do not apply to scoring the tutor's answers; the portable parts are the methodology (sign-offs on inputs and grading, train/test split with a held-out headline, oracle/null controls, noise-floor arithmetic, per-case traces). These match the Tutor workspace rule that `eval-search` supports a held-out split and "never tune against the test split". Retrieval evals (nDCG@10, gold-id recall) are programmatic checks and need no judge model.

---

## 9. Open questions and things I could not verify

1. OpenAlex ISBN: no field and filter returns 400. Book verification path unresolved (OpenAlex DOI or title+author only).
2. Whether OpenAlex gives better or worse metadata for Chinese-language papers than Crossref/CNKI: only the language-tagged count was checked.
3. BRIGHT psychology: no published agentic baseline; contamination not assessed; page-level vs chunk-level gold choice (`gold_ids` vs `gold_ids_long`) is for the lead.
4. DCI/DR-DCI/RARG gains are on web-page QA (BrowseComp-Plus, Wikipedia), not scholarly PDFs; none measures citation or span-grounding accuracy.
5. Late chunking: Qwen3-Embedding pooling type and llama.cpp token-level output (my recollection, unverified).
6. Trigram index size at 6M chunks and the CJK n-gram size: must be measured on a sample.
7. Swiftide release gap (published release Nov 2025 vs active master) and whether master features are released.
8. Referenced but not read: Pi-Serini (arXiv 2605.10848, "is lexical retrieval sufficient?") and RISE (github.com/texttron/RISE), both named in DR-DCI/RARG as the lexical-narrowing neighbours.

## 10. Source list
- OpenAlex: https://help.openalex.org/api/authentication/, https://help.openalex.org/access/pricing/, https://help.openalex.org/access/example-costs/, https://help.openalex.org/access/snapshot/, https://help.openalex.org/access/sync/, https://help.openalex.org/data/works/attributes/, https://help.openalex.org/data/works/citations/, https://developers.openalex.org/api/llm-quick-reference, https://help.openalex.org/llms.txt, https://api.openalex.org (live calls)
- BRIGHT: https://brightbenchmark.github.io/, https://huggingface.co/datasets/xlangai/BRIGHT, https://huggingface.co/api/datasets/xlangai/BRIGHT, https://datasets-server.huggingface.co (rows, sizes), https://github.com/xlang-ai/BRIGHT, https://arxiv.org/abs/2407.12883
- Papers: https://arxiv.org/abs/2605.05242, https://arxiv.org/abs/2606.14885, https://huggingface.co/papers/2607.24223 (arXiv 2607.24223), https://arxiv.org/abs/2401.18059, https://arxiv.org/abs/2409.04701, https://arxiv.org/abs/2409.13740
- Code: https://github.com/DCI-Agent/DCI-Agent-Lite, https://github.com/EigenTom/DR-DCI, https://github.com/LeqsNaN/RARG, https://github.com/Future-House/paper-qa, https://github.com/sourcegraph/zoekt, https://github.com/bosun-ai/swiftide, https://github.com/0xPlaygrounds/rig
- Index design: https://swtch.com/~rsc/regexp/regexp4.html, https://raw.githubusercontent.com/sourcegraph/zoekt/main/doc/design.md, https://docs.rs/tantivy/latest/tantivy/tokenizer/struct.NgramTokenizer.html, https://docs.rs/tantivy/latest/tantivy/query/struct.RegexQuery.html, https://docs.rs/regex-syntax/latest/regex_syntax/hir/literal/struct.Extractor.html
- Anthropic: https://www.anthropic.com/news/contextual-retrieval, https://github.com/anthropics/skills/tree/main/skills/claude-api, local bundled-skills copy noted in section 8

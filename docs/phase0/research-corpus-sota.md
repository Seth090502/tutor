# Corpus-management design: currency check, 2026-10-01 (deltas only)

Prepared 2026-10-01 by a worker agent (Sonnet 5.5) for the lead engineer. Read-only research. No packages were installed, no cloned code was run, and nothing was downloaded beyond API responses and README/card text. Raw responses are in the session scratchpad only.

Already covered and not repeated: `projects-sidecars.md` (llama.cpp, Qwen3 models, GROBID, MinerU 4.0, Marker, xberg) and `references.md` (OpenAlex pricing/API terms/snapshot, BRIGHT, the DCI papers and other retrieval references). I also read `docs/vault/AGENTS.md`, `docs/vault/CLAUDE.md`, `docs/vault/taxonomy/CATEGORIES.md` and the taxonomy snapshot, because the brief item 5 turned out to overlap with a file that already exists (see section 5.6).

Evidence tags on every number or claim:
- **M** measured by me today (API call, header, file listing). Re-checkable with the URL.
- **D** documented by the cited source. Marked **D-indep** if the source is independent of the thing being measured, otherwise it is the vendor's or maintainer's own claim.
- **E** my estimate or inference. Reasoning is given.
- **U** unverified, blocked, or not found. An empty result is not proof of absence.

Tool limits hit during this run: WebSearch was out of budget, so everything came from direct fetches, `gh api`, and curl. Several sites (APA, ACM, Library of Congress CIP pages) blocked automated fetches. Those are marked U and listed in section 7.

---

## 0. Findings that change something (read this first)

1. **OpenAlex topics are not newer than the vault's snapshot.** The taxonomy is still 4 domains, 26 fields, 252 subfields, 4,516 topics **[M]**, the same 4,516 as the vault's `taxonomy/openalex/topics.json` (fetched 2026-10-01). The topic list dates from January 2024 (first topic's `created_date` 2024-01-23 **[M]**; model artifacts on Zenodo 2024-01-25 **[M]**). The classifier repo's last push is 2025-08-04 and it lists only "v1 (current)" **[M]**. I found no 2025-2026 retraining or new level **[U: absence not proven]**. Nothing to update in the category map.
2. **There is a public way to assign a topic to a non-indexed document, and it is not new in kind but is now documented: the `/text` endpoint** (section 1.3). It returns the same topics and keywords OpenAlex assigns to indexed works. A local alternative also exists: the title/abstract-only BERT model on Hugging Face (Apache-2.0).
3. **OpenAlex keywords were replaced on 2026-09-29 (v3).** A fine-tuned Qwen3-4B now tags all works, with a 1,874,460-keyword CC0 vocabulary **[D]**. The previous 65,004 keywords were topic-derived. This is a new, finer, free facet (section 1.4). It is also a moving vocabulary: store the fetch date.
4. **A topic-to-category mapping risk in the vault's own snapshot.** `categories.json` maps topic T11727 "Advanced Algebra and Logic" (75,920 works **[M in snapshot]**) to primary `cs-ai`, secondary `mathematics`, because OpenAlex files that topic under the Computer Science subfield "Computational Theory and Mathematics". My `/text` call on a universal-algebra textbook blurb returned exactly that topic with score 0.947 **[M, n=1]**. A pure-algebra book would therefore land in `cs-ai` by the topic route. The vault already maps at topic level, so this is a mapping choice to review, not a bug.
5. **Low-confidence primary topics exist and the score is available.** The work returned for DOI 10.1037/0033-295X.84.2.191 (the Bandura 1977 DOI in `references.md`) carries primary topic "Community Health and Development" at score 0.396, with keywords about thalassemia screening **[M, n=1]**, and the vault snapshot maps that topic to `medicine`. That is a metadata error upstream (the record content does not match the DOI's paper), consistent with the affiliation anomaly already noted in `references.md`. The precedence rule `openalex_primary_topic` before `embedding_nearest_topic` has no score gate. A gate would need a measured threshold from the pilot batch **[E]**.
6. **Claude Code now reads AGENTS.md natively (v2.1.277+), and the installed version is 2.1.286 [M: `claude --version`].** Rule: AGENTS.md is used only when no CLAUDE.md or CLAUDE.local.md exists in the working directory or above it. The existing `CLAUDE.md` containing only `@AGENTS.md` is a documented, supported pattern. Details in section 5.
7. **Benchmarks disagree on who leads document extraction, and OmniDocBench is nearly saturated.** Top OmniDocBench overall is 96.91 (TeleOCR, self-reported). The same models score much lower on harder or different benchmarks. No model leads everywhere (section 4).
8. **Hazard for "source code folders work with everything":** delivered repositories may contain their own `AGENTS.md`, `CLAUDE.md`, `.cursorrules`. If any such file sits under its original name inside a directory an agent harness treats as a project, Claude Code loads subdirectory instruction files on demand when the agent reads a file there (section 5.5). That is instruction injection from delivered content **[E]**.

---

## 1. OpenAlex Topics: current state

### 1.1 Version, taxonomy, and what changed

- Hierarchy today: 4 domains, 26 fields, 252 subfields, 4,516 topics **[M]** (`/domains`, `/fields`, `/subfields`, `/topics` meta counts, 2026-10-01). Docs state the same four-level hierarchy **[D]** (`https://help.openalex.org/data/topics`).
- Topic names and descriptions are LLM-generated from citation-network clusters; each topic is mapped to a subfield, field and domain through Scopus ASJC categories **[D]**. The cluster labels come from CWTS (Leiden), built on 71M publications from 2000 to 2023 using the extended direct citation approach plus the Leiden algorithm: 4,521 lowest-level research areas in their classification vs 4,516 topics in OpenAlex **[D]** (Leiden Madtrics post). The topic list has not changed in size since January 2024 **[M for count; E that it is unchanged in content]**.
- Concepts (the old Wikipedia-derived vocabulary, 65,026 entries **[M]**) are deprecated and unmaintained **[D]**. Do not use them.
- Model repo: `ourresearch/openalex-topic-classification`, MIT licence, last pushed 2025-08-04, single iteration "v1 (current)" **[M]**. Its README links to model artifacts on Zenodo (record 10568402, CC0, `topic_classifier_v1_artifacts.tar.gz` 741,549,189 bytes plus `train_val_test_data.parquet` 3,074,184,489 bytes, dated 2024-01-25) **[M]**.
- Superseded approach to note: concepts were replaced by topics; keyword extraction was replaced twice (v1, then topic-derived v2, then LLM-distilled v3 on 2026-09-29, section 1.4).

### 1.2 How topics are assigned and how accurate it is

- A deep-learning classifier reads title, abstract, citations and journal name, scores every topic, and stores the top up to three with scores; the best is `primary_topic`. It can classify a new work with no incoming citations from title, abstract and source alone **[D]** (topics doc).
- The released title/abstract-only model is `bert-base-multilingual-cased` fine-tuned, 181,330,089 parameters, Apache-2.0, created 2024-01-06, last modified 2024-05-13 **[M]** (`OpenAlex/bert-base-multilingual-cased-finetuned-openalex-topic-classification-title-abstract`). The production model adds citations and journal name, so the public BERT is "NOT the full model" **[D]** (its card).
- **Coverage:** 40,588,199 of 326,797,579 core works have no `primary_topic`, 12.4% **[M]** (docs say about 12%, 39.6M of 322M, mid-2026 **[D]**). These are works with no title, abstract or citations.
- **Measured accuracy: not found.** The model card's `results` list is empty **[M]**, and the saved notebook outputs I scanned contain no accuracy, F1 or top-k strings **[M]**. The training labels are CWTS citation-cluster assignments, not human topic judgements, so any reported accuracy would measure agreement with a clustering, not correctness **[E]**. Treat topic accuracy as unknown **[U]** and measure it on the pilot batch against the owner's own category judgements.
- **Score exists and varies a lot** [M, n=2 anecdotes, not rates]: a clean abstract returned primary score 0.886 (Transformers abstract via `/text`); the mismatched Bandura-DOI record returned 0.396; the textbook blurb returned 0.947.

### 1.3 Topic for a document that is not in OpenAlex (a book, a PDF without DOI)

Two routes, both public:

| Route | What it is | Terms | Notes |
|---|---|---|---|
| **`/text`, `/text/topics`, `/text/keywords`** | OpenAlex-hosted classifier. GET or POST `title` plus optional `abstract`; returns `primary_topic`, top topics with subfield/field/domain and scores, and keywords | 20 to 2,000 characters combined; 1 request/s; $0.01 per request **[D]**. I measured `X-RateLimit-Cost-USD: 0.01` on a keyless call **[M]**. Keyless allowance is $0.10/day, so about 10 calls/day without a key; the free key gives $1/day, about 100 calls/day **[D, from references.md budgets; E arithmetic]** | Documented page "Tag Aboutness" `https://help.openalex.org/api/tag-aboutness`. Empty `topics` if the text is short or non-descriptive; sending an abstract helps **[D]**. For 10,000 books the free key allows about 100 per day, so about 100 days **[E]**; paid prepaid credit is $1 increments **[D]** |
| **Run the model yourself** | The public BERT (title/abstract only, 181M params) from Hugging Face, usage format `<TITLE> ... \n<ABSTRACT> ...`, `top_k=10` **[D]**. Full model code in the MIT repo; artifacts CC0 on Zenodo (742 MB) **[M]** | Apache-2.0 (model) / MIT (code) / CC0 (artifacts) **[M]** | Local, no rate limit, multilingual. Loses the citation and journal inputs, so it is weaker than production OpenAlex **[D]**; how much weaker is **U**. Context limit of a BERT-base model is 512 tokens **[E, standard]**, so feed title plus a short description or the first abstract-sized passage, not a whole book |

For books specifically **[E]**: neither route sees a book's citations. The `/text` input should be the title plus the publisher's blurb or the book's own abstract-like description. LCC/DDC from the CIP block (already in the design) remains the primary signal for books; the topic is a secondary signal.

**Licence:** OpenAlex data is CC0 **[D, references.md]**; Zenodo classification dataset record 10560276 (Nov 2023 CWTS classification, 1,043,312,730 bytes) and model record are CC0 **[M]**; keyword vocabulary repo is CC0-1.0 **[M]**.

### 1.4 New since the design: OpenAlex keywords v3

- Shipped 2026-09-29; GitHub release `v3.0` published 2026-09-30 **[M]**. A fine-tuned Qwen3-4B tags all works; a rule-based vocabulary build with synonym merging yields **1,874,460 keywords**; a keyword enters the vocabulary only when it describes at least 100 works **[D]**. The earlier v2 had 65,004 keywords from topics matched with bge-m3 **[D]**.
- Docs report 419 million works with at least one keyword and about 11% with none **[D]**; an AI judge rated 91% of keywords accurate on 400 random works vs 44% for the previous ones **[D, vendor-run, not independent]**.
- Weights, prompts, vocabulary and gold sets are open (CC0-1.0 repo) **[M licence field; D contents]**. `/text` returns keywords from the same model **[D]**.
- **The vocabulary changes** ("store keyword IDs with the date you fetched them, expect some to change") **[D]**. Next step OpenAlex lists: joining keywords to MeSH and other outside vocabularies **[D, future]**.
- Relevance [E]: a ready-made, free, fine-grained, multilingual-capable facet for *search filters and grep scopes*, without a taxonomy build. It does not replace the 4,516-topic backbone for coverage targets because the vocabulary is unstable.

### 1.5 Preprint versions and OpenAlex (delta to references.md)

- A location is a hosted copy; OpenAlex matches copies into one work by shared persistent ID (DOI, PMID, arXiv ID) first, then fuzzy title-plus-author matching **[D]**. An arXiv location exposes `merge_key: {doi, arxiv, pmid, title_author}` **[M]** (`/locations/pmh:oai:arXiv.org:1706.03762`).
- Each location has `version`: `publishedVersion`, `acceptedVersion` or `submittedVersion` (DRIVER vocabulary) **[D]**.
- Counts **[M]**: works with an arXiv location 3,495,056; works whose primary location is arXiv 1,953,974; works with an arXiv location and a journal-typed article primary 1,305,387. The difference, about 1.54M works with an arXiv copy but a non-arXiv primary location, is how many preprints have already been merged into a published work **[E arithmetic]**.
- **Lookup gotcha [M]:** `filter=doi:10.48550/arxiv.1706.03762` returned count 0. arXiv works are not keyed by their DataCite DOI in the `/works` filter. Use `/locations/pmh:oai:arXiv.org:<id>` (returned the work id W2626778328 for the Transformers paper) **[M]**; I did not measure its cost per call **[U]**.
- Other counts **[M]**: `type:preprint` 7,062,218; `type:book` 6,421,773.

---

## 2. Complementary schemes as facets

Question: which are worth adding, and how to get them free. Summary first, details below.

| Scheme | Free? | Obtainable how | Recommend |
|---|---|---|---|
| **MSC2020** (maths) | Yes for non-commercial; CC BY-NC-SA 4.0 **[D]** | arXiv `msc-class` field (author-supplied) and zbMATH Open per-document `msc` (scheme `msc2020`) | **Add**, for `mathematics` only |
| **MeSH 2026** (medicine, clinical psychology) | Yes; public NLM data with attribution terms **[D]** | `desc2026.xml`, RDF, SPARQL; per-paper terms from PubMed/PMC records | **Add**, only for documents with a PMID |
| **OpenAlex keywords v3** | Yes (CC0) | OpenAlex works records, `/text` | **Add** (not in the brief's list; free and new) |
| **CSO v3.5** (CS) | Ontology free to download; licence text not confirmed **[U]** | `.ttl/.owl/.nt/.csv`; `cso-classifier` (Apache-2.0) | Optional, low priority |
| **ACM CCS 2012** | Page blocked, terms not verified **[U]** | n/a | Do not add on current evidence |
| **APA Thesaurus / PsycINFO codes** | **Could not verify**; pages blocked **[U]** | n/a | **Do not add**; see below |
| **Semantic Scholar fields of study** | API terms restrict to internal non-commercial research use **[D]** | S2 Graph API `s2FieldsOfStudy` | Do not add; redundant with OpenAlex fields |
| **LCC** (books) via LoC linked data / Open Library | LoC linked data is a public-domain data set **[D]** | `id.loc.gov`, Open Library `lc_classifications` | Already in design; confirm the source (section 3.4) |

### 2.1 MSC2020 (mathematics)

- Current version is MSC2020; ten-year review cycle stated, no MSC2030 dates published **[D]** (`msc2020.org`). Licence CC BY-NC-SA 4.0; downloads PDF, TeX, CSV (no SKOS on the official site) **[D]**. A third-party SKOS conversion exists (`TIBHannover/MSC2020_SKOS`, licence NOASSERTION, last push 2021-07-28) **[M]**; a conversion made in 2021 is of MSC2020 itself, so it is not stale **[E]**, but its licence is unclear.
- **arXiv:** the OAI-PMH `arXiv` metadata format returns `<msc-class>` and `<acm-class>` per record **[M]** (sample 2012.11459: `msc-class 05C15, 05C85`). These are author-supplied. In that sample the `acm-class` field repeats the MSC codes, a sign of author misuse **[M, n=1]**, so treat them as hints, not truth.
- **zbMATH Open:** REST API `https://api.zbmath.org/v1/` and OAI-PMH `https://oai.zbmath.org/v1/` are live (`earliestDatestamp 1755`) **[M]**. A search result returned `msc: [{code: "01A70", scheme: "msc2020", text: ...}]` **[M, n=1]**. Content generated by zbMATH (reviews, classifications) is CC-BY-SA 4.0 per the API's own OpenAPI text **[D]**. One record in my sample reported editorial contents unavailable "due to conflicting licenses" **[M, n=1]**. Rate limits not read **[U]**.
- Obtain free: arXiv OAI per arXiv ID (no key); zbMATH for DOI-bearing maths papers. Books: the LCC QA schedule and DDC 510 already carry the information **[E]**.
- Share-alike (SA) means a derived classification table, if redistributed, must carry the same terms; a private vault does not redistribute **[E]**.

### 2.2 MeSH 2026

- 2026 production-year files are on the NLM download page in XML, RDF (updated Monday to Friday) and MARC 21; the ASCII format was discontinued as of January 2026 **[D]**. `desc2026.xml` exists, 312,952,703 bytes, `Last-Modified 2026-08-12` **[M]** (HTTP HEAD on `nlmpubs.nlm.nih.gov/projects/mesh/MESH_FILES/xmlmesh/desc2026.xml`). The MeSH SPARQL endpoint answers **[M]**.
- Terms **[D]** (`nlm.nih.gov/databases/download/terms_and_conditions.html`): attribute NLM ("Courtesy of the U.S. National Library of Medicine"); do not imply NLM endorsement; when republishing, either keep the most current version or state the material may not be current; no fees.
- A SPARQL count of `TopicalDescriptor` returned 378,809, which includes versioned URIs from earlier years, so it is not a descriptor count **[M, discarded]**. I did not obtain a clean 2026 descriptor count **[U]**.
- Where per-paper MeSH comes from: PubMed/PMC records for indexed articles. I did not test coverage of psychology journals **[U]**. PsycINFO-only journals will have no MeSH **[E]**. For psychology, the "Psychiatry and Psychology" tree (F) is the free option **[E]**.

### 2.3 CS: CSO and ACM CCS

- **CSO** version 3.5, released 2025-07-29, "about 15K topics linked by 166K semantic relationships", formats `.nt .owl .ttl .csv`; the 3.5 update focused on cybersecurity **[D]** (`cso.kmi.open.ac.uk/downloads`, via fetch summary). Licence text not shown on the pages I read **[U]**. `angelosalatino/cso-classifier`: Apache-2.0, Python 3.11+, last push 2026-04-18, 97 stars **[M]**; it classifies from title, abstract and keywords, unsupervised **[D]**.
- **ACM CCS:** `dl.acm.org/ccs` and `acm.org/publications/class-2012` returned 403 or a Cloudflare challenge **[M]**. Version, terms and any post-2012 revision are **[U]**. Since authors no longer reliably assign CCS concepts [E, from general knowledge, unverified here], it is a weak signal for a corpus built mostly from OpenAlex and arXiv.
- Recommendation context [E]: OpenAlex already splits Computer Science into subfields and arXiv `cs.*` gives a primary class. CSO adds finer topics but the marginal value over those two is untested. Low priority.

### 2.4 APA Thesaurus and PsycINFO classification codes (licensing)

- **I could not read APA's licensing or classification-code pages.** All three URLs returned an Incapsula block page ("Request unsuccessful") to my fetches **[M]**. **Verdict on licensing: unverified [U].**
- My background expectation, unverified here **[E]**: the thesaurus of index terms is APA-proprietary and licensed for use in databases, and PsycINFO record-level indexing is a subscription product. Do not copy APA terms into the vault until the owner has read the terms.
- Free substitutes for the owner's psychology priority [E]: the vault's own study map (`psych_areas`), OpenAlex Psychology field (9,294,459 works **[M]**) and its subfields, MeSH tree F, and OpenAlex keywords v3.

### 2.5 Semantic Scholar fields of study

- The API returns `fieldsOfStudy` and `s2FieldsOfStudy` with a `source` of `external` or `s2-fos-model` per entry **[M]** (Transformers paper: Computer Science from both sources). I could not retrieve the official list of values **[U]**; my recollection is 23 broad fields **[E, unverified]**.
- The API licence agreement limits use to the licensee's internal use for training and evaluating models and for "legitimate, non-commercial, research and/or educational purposes", with public use required to point back to Semantic Scholar **[D]** (`api.semanticscholar.org/license/`). Keyless calls hit HTTP 429 on my second request **[M]**.
- Redundant with OpenAlex fields (26) and friction on licence and rate. Not recommended.

### 2.6 LCC and DDC for books (the design's crosswalks)

- LoC linked data service: "a public domain data set" **[D]** (`id.loc.gov/about`); the classification authority is queryable (`/authorities/classification/Q325.5` returned a record, label "Self-organizing systems. Conscious automata") **[M]**.
- Open Library edition record for ISBN 9780262035613 carries `lc_classifications: ["Q325.5.G66 2017"]`; its search index returns `lcc` and subjects; no DDC appeared in that sample **[M, n=1]**.
- **DDC is OCLC-controlled; I did not verify whether DDC class numbers or schedules may be embedded in a local database [U].** The vault design uses DDC ranges as a crosswalk. If DDC numbers come only from each book's own CIP block, the number is recorded as data supplied with the book **[E]**; the schedule text itself should not be copied.

---

## 3. Preprint-to-published linking, versions, and book identity

### 3.1 Crossref relations

- Relation vocabulary in the deposit schema: Preprint (`isPreprintOf`, `hasPreprint`), Version (`isVersionOf`, `hasVersion`), Manuscript, Expression, Replacement and others **[D]** (Crossref markup guide on relationships). Current deposit schema is 5.5.0 **[D, nav list]**.
- Counts via the REST API **[M]**:
  - works with `relation.type:is-preprint-of`: 842,298 (these are mostly posted-content records pointing at the journal version);
  - works with `relation.type:has-preprint`: 576,212 (journal articles pointing back);
  - `type:posted-content`: 4,107,445; posted-content that also has `is-preprint-of`: 841,755 (so about 20.5% of posted content is linked to a published version **[E arithmetic]**).
- **The link is asymmetric.** The preprint side carries about 1.46 times as many links as the published side **[E arithmetic of M]**. Resolve preprint DOI to published DOI from the preprint record first.
- Sparse: the sample article (LIGO, 10.1103/PhysRevLett.116.061102) has an empty `relation` object **[M, n=1]**. Relations exist only when a depositor asserts them.
- API quirk **[M]**: `select=relation` is accepted on list queries but `select` is rejected on the single-work route; `subtype` is not a valid select.

### 3.2 DataCite and arXiv DOIs

- DataCite schema latest release tag 4.7.4, 2026-09-01 **[M]** (`datacite/schema` releases).
- arXiv assigns every article a DOI `10.48550/arXiv.<id>` through DataCite at no cost; **new versions do not get new DOIs**, the metadata at DataCite is updated, and the DOI resolves to the latest version **[D]** (`info.arxiv.org/help/doi.html`). The DataCite record for 1706.03762 shows `version 7`, updated 2025-04-09, with **no related identifiers** **[M]**; `.../arxiv.1706.03762v7` returns 404 **[M]**. So DataCite does not link arXiv to the journal version; arXiv's `journal-ref` and `doi` fields do **[D]**, and arXiv "will continue to support" those fields and encourages authors to fill them **[D]**.
- DataCite relation counts **[M]**: `IsVersionOf` on 9,813,140 DOIs; `IsPreviousVersionOf` on 1,691,400; `IsPreprintOf` on 0 (no such type there). Version chains in DataCite are mostly repository records (Zenodo-style), not journal papers **[E]**.
- arXiv site banner: "arXiv is now an independent nonprofit!" **[M]**; date and consequences not read **[U]**.
- arXiv bulk access: OAI-PMH preferred for metadata (daily); S3 requester-pays buckets for full text, about 9.2 TB as of April 2025 growing about 100 GB a month; users building indexes must link back to arXiv since most submissions use the default non-transferring licence **[D]** (`bulk_data.html`, `bulk_data_s3.html`). The page `info.arxiv.org/help/jref_updates.html` (publisher journal-ref updates) returned 404 at my guessed URL; its content is **[U]**.

### 3.3 Net recommendation on versions [E]

Keep one document per delivered version, link by lineage, and record three identifiers when known: arXiv ID, preprint DOI, published DOI. Fill the link in this order: arXiv `journal-ref`/`doi`, Crossref `is-preprint-of` from the preprint DOI, then OpenAlex location merge. State which source filled each link, as the design already does for other provenance fields.

### 3.4 Book identity status

- **OpenAlex has no ISBN field** (already in `references.md`). Crossref has `type:book` 857,790 records with an `ISBN` select **[M]**; OpenAlex has 6,421,773 works with `type:book` **[M]**, so Crossref covers at most about 13% of that count **[E arithmetic, different populations]**.
- **Open Library:** `/isbn/<isbn>.json` works (no key) and returned title, publishers, pages, ISBN-10/13, `lc_classifications`, `works` key, a Wikidata ID **[M, n=1]**; `search.json?q=isbn:...&fields=...` returned `lcc` and `subject` **[M]**. Bulk monthly dumps: the "latest" link redirects to `ol_dump_2026-08-31` **[M]**; sizes documented: editions about 9.2 GB, works about 2.9 GB, all types about 12.4 GB, complete about 29.6 GB **[D]**. API etiquette **[D]**: identify yourself with a User-Agent that includes an email (identified requests get a 3x rate limit); no HTML scraping or bulk harvesting; no hundreds of single-book requests (use bulk or `search.json`); no heavy commercial backend. Dump licence text not read **[U]**.
- **Library of Congress:** MDSConnect open-access MARC is "nearly 25 million records ... as distributed in the unabridged 2016 Retrospective file sets", free **[D]**; current-year MARC Distribution Services is a paid subscription **[D]** (`loc.gov/cds/products/marcDist.php`). So **free LoC bulk MARC ends at 2016**; books from 2017 onward need the CIP block printed in the book, Open Library, or per-record lookups. The LoC CIP programme page returned a Cloudflare challenge, so any 2025-2026 programme change is **[U]**.
- **ISBN agencies:** the International ISBN Agency publishes the Range Message (generated XML) and says it does not authorise other sites to reproduce it **[D]** (`isbn-international.org/range_file_generation`). It validates the prefix and registrant group of an ISBN; it is not a title lookup **[E]**. No ISBN-agency title API found **[U]**.

---

## 4. Document extraction: leaders since mid-2026 (not in projects-sidecars.md)

### 4.1 Two benchmarks, and a caution

- **OmniDocBench** is now **v1.7** (2026-04-30); v1.6 (2026-04-10) added 296 harder pages and a new matching algorithm (MGAM); v1.5 (2025-09-25) balanced English and Chinese pages and raised resolution. Last leaderboard update 2026-09-11 **[D]** (`github.com/opendatalab/OmniDocBench`, README "Updates"; repo pushed 2026-09-11 **[M]**). It has 1,651 pages in the original release **[D]**; page count for v1.7 not stated **[U]**. The README leaderboard table caption still says "v1.6_full" while the updates list says v1.7 **[M]**; read the table as the current published one with that ambiguity noted.
- **Not independent:** OmniDocBench is maintained by OpenDataLab, the organisation that makes MinerU **[M: repo owners]**. Rows are submitted by the model authors in many cases ("Evaluation Version/Model Weights" lists) **[D]**.
- **olmOCR-bench** (Ai2, dataset ODC-By **[M]**) tests small, objective pass/fail cases: arXiv maths, old-scan maths, tables, old scans, headers and footers, multi-column, long tiny text, base. The Hugging Face leaderboard for it marks **every entry `verified: false`**, all self-reported by model authors **[M]**.
- **Saturation:** the top OmniDocBench entries sit at 96 to 97 overall. TeleOCR's own card reports a much harder EMNLP 2026 challenge, "Dr.DocBench", where its 67.96 overall leads MinerU 2.5 Pro (62.26), OvisOCR2 (59.25) and PaddleOCR-VL-1.6 (55.11) **[D, vendor-run]**. The Infinity-Parser2 card reports a "ParseBench" with Pro 74.3 and Gemini-3-Pro 69.1 **[D, vendor-run]**.

### 4.2 OmniDocBench leaders (README table, all rows self-reported unless noted)

| Rank | Model | Params | Overall | Text edit (lower better) | Formula CDM | Table TEDS | Licence | Local runtime |
|---|---|---|---|---|---|---|---|---|
| 1 | TeleOCR (renamed from NaviDC-OCR 2026-09-10) | 1.2B card, 1,415,072,768 safetensors | 96.91 | 0.0267 | 96.59 | 96.82 | Apache-2.0 **[M]** | community GGUF only; llama.cpp support status **U** |
| 2 | OvisOCR2 (`ATH-MaaS/OvisOCR2`, base Qwen3.5-0.8B) | 0.85B | 96.47 | 0.0265 | 97.49 | 94.58 | Apache-2.0 **[M]** | community GGUFs (bartowski etc.) **[M]**; llama.cpp support **U** |
| 3 | PaddleOCR-VL-1.6 | 0.96B | 96.34 | 0.0326 | 97.53 | 94.76 | Apache-2.0 **[M]** | **official** `PaddlePaddle/PaddleOCR-VL-1.6-GGUF` **[M]**; llama.cpp PR #18825 **[D]** |
| 4 | MinerU2.5-Pro (2605-1.2B) | 1.16B | 95.75 | 0.036 | 97.45 | 93.42 | Apache-2.0 **[M]** | community GGUFs only **[M]** |
| 5 | GLM-OCR | 1.33B | 95.22 | 0.044 | 97.18 | 92.83 | MIT **[M]** | `ggml-org/GLM-OCR-GGUF`; llama.cpp PR #19677 **[D]** |
| 6 | PaddleOCR-VL-1.5 | 0.9B (README) | 94.93 | 0.038 | 96.89 | 91.67 | Apache-2.0 | see above |
| 8 | Unlimited-OCR (Baidu) | 3.3B | 94.00 | 0.0394 | 95.72 | 90.21 | MIT **[M]** | community GGUFs **[M]** |
| ref | MinerU-Pipeline (3.4.0) | n/a | 86.47 | 0.055 | 83.07 | 81.88 | n/a | n/a |
| ref | olmOCR (7B) | 7B | 85.74 | 0.139 | 88.10 | 83.00 | Apache-2.0 | n/a |
| ref | Marker | n/a | 78.44 | 0.157 | 85.24 | 65.77 | see sidecars | n/a |
| ref | Gemini 3 Pro / GPT-5.2 | closed | 92.91 / 86.59 | 0.064 / 0.114 | n/a | n/a | n/a | n/a |

Notes: HunyuanOCR (89.95) is under a custom Tencent licence `tencent-hunyuan-community` **[M]**; Qianfan-OCR (93.90, 4B) and dots.ocr (90.77, 3B, MIT) also listed. The README gives no per-page-type breakdown (academic papers separately) or per-language breakdown for these new models **[M]**, so "born-digital academic" and "Chinese" rankings cannot be read from it **[U]**. Chinese pages are half the benchmark since v1.5 **[D]**. All the top models come from Chinese labs and target Chinese and English **[E from names and cards; Infinity-Parser2 card says primarily English and Chinese]**.

### 4.3 olmOCR-bench leaders (HF leaderboard, all `verified: false`)

| Rank | Model | Overall | Notes |
|---|---|---|---|
| 1 | `infly/Infinity-Parser2-Pro` | 87.6 | Apache-2.0; 35,107,181,936 BF16 parameters, architecture `Qwen3_5MoeForConditionalGeneration` **[M]**; released 2026-05-11 **[D]**; community AWQ W4A16 and Q8 GGUF exist **[M]** |
| 2 | `datalab-to/chandra-ocr-2` | 85.8 | OpenRAIL-M weights (free for research, personal use, startups under $2M funding/revenue; no competing use) **[D]**; 5,295,564,288 parameters **[M]**; Datalab's own hosted API scores 86.7 (closed) **[D]** |
| 3 | `dots-studio/dots.mocr` | 83.9 | tech report |
| 4 | `jinaai/jina-ocr-v1` | 83.4 | |
| 5 | `datalab-to/surya-ocr-2` | 83.3 | OpenRAIL-style weights **[E from Surya lineage; U]** |
| 7 | `lightonai/LightOnOCR-2-1B` | 83.2 | 1B |
| 9 | `infly/Infinity-Parser-7B` | 82.5 | |
| ref | Infinity-Parser2-Flash | 86.0 | 2,213,241,664 BF16 params, Apache-2.0 **[M]**; not on the HF leaderboard top list but in the model's own table **[D]** |
| ref | olmOCR-2-7B-1025 | 82.4 | Apache-2.0; no new Ai2 model since October 2025 (release v0.4.27 on 2026-03-12 is pipeline code) **[M]** |
| ref | MinerU2.5 (2509) | 75.2 | |
| ref | GLM-OCR | 75.2 | Qianfan-OCR 79.8; DeepSeek-OCR-2 76.3 |

Per column (Chandra 2 card, **[D]** vendor-run): arXiv maths 86.9, old-scan maths 89.1, tables 92.1, old scans 51.1, headers/footers 91.4, multi-column 82.1. The Datalab API row leads old scans at 54.6. **Scans remain the weakest category for every model (best 54.6 on a 0 to 100 scale)** **[D]**. Maths-heavy pages are the strongest area for the leaders (about 87 to 90).

### 4.4 The two benchmarks do not agree [D, from Infinity card table, vendor-run]

| Model | OmniDocBench v1.6 | olmOCR-bench |
|---|---|---|
| Infinity-Parser2-Pro | 93.95 | 87.6 |
| PaddleOCR-VL-1.5 | 94.87 | 78.5 |
| MinerU2.5 | 92.98 | 75.2 |
| DeepSeek-OCR-2 | 90.17 | 76.3 |

So PaddleOCR-VL-1.5 beats Infinity-Parser2-Pro on OmniDocBench but loses by 9 points on olmOCR-bench. The vault should not pick an extractor from either leaderboard; a representative-sample bake-off on the owner's own pages is the check **[E]**.

### 4.5 Can it run locally (RTX 5090 32 GB, Radeon 7800 XT 16 GB on Vulkan)?

- **Feasibility, not measured [E]:** at BF16 a model of 0.85B to 1.4B parameters weighs about 1.7 to 2.8 GB (parameters times 2 bytes), so every OmniDocBench top-5 model fits either card with large margin. Chandra 2 (5.3B, about 10.6 GB at BF16) fits both. Infinity-Parser2-Flash (2.2B, about 4.4 GB) fits both. **Infinity-Parser2-Pro (35.1B, about 70 GB BF16) fits neither without quantisation**; a 4-bit build is about 20 GB, which fits the 5090 and not the 7800 XT once KV cache is counted **[E]**.
- **llama.cpp supported OCR models, documented** **[D]** (`docs/multimodal.md`): PaddleOCR-VL, GLM-OCR, DeepSeek-OCR, dots.ocr, HunyuanOCR, each with its own required prompt and input structure. Models with an official or ggml-org GGUF: PaddleOCR-VL-1.6, GLM-OCR, Qianfan-OCR, dots.ocr **[M]**. For TeleOCR, OvisOCR2, MinerU2.5-Pro, Chandra 2, Unlimited-OCR only community conversions exist **[M]**; whether llama.cpp's current build runs them correctly is **U**. Per `projects-sidecars.md`, use the Windows CUDA 13.4 binary on the 5090 and the Vulkan binary on the 7800 XT. I did not run any of them.
- **vLLM-based models** (Infinity-Parser2 pins `vllm==0.17.1` **[D]**; olmOCR needs an NVIDIA GPU with at least 12 GB and a Linux-style vLLM stack **[D]**) are CUDA-only and Linux-oriented; on Windows that means WSL2 or Docker **[E]**. Not usable on the 7800 XT.
- **MinerU link:** MinerU 4.0 runs its VLM through llama.cpp on Vulkan in the base install (sidecars). Whether 4.0's default VLM is the new 2.5-Pro weights is **U**.

### 4.6 What I would check in a bake-off [E]

Pages from the vault's own pilot batch: born-digital English papers, maths-heavy pages, a handful of scanned book pages, and Chinese pages. Candidates: PaddleOCR-VL-1.6 (official GGUF, small), MinerU2.5-Pro, GLM-OCR, Infinity-Parser2-Flash, TeleOCR, Chandra 2 (check the licence fits). Measure text, formula and table fidelity plus pages per second on both cards.

---

## 5. AGENTS.md, CLAUDE.md, and how harnesses load them

### 5.1 The AGENTS.md open format

- "A README for agents": plain Markdown, flexible headings; recommended topics are project overview, build and test commands, code style, testing, security, commit/PR guidelines, deployment **[D]** (`agents.md`). Nested files: "the closest one takes precedence"; an explicit user chat prompt overrides file instructions **[D]**.
- Stewardship: "now stewarded by the Agentic AI Foundation under the Linux Foundation" **[D]**; the site claims over 60 supporting agents **[D]**. Repo `agentsmd/agents.md`: MIT, last push 2026-09-10, 24,715 stars **[M]**. No spec version or date is stated **[M]**, so the format has no schema; the harness defines behaviour.

### 5.2 Claude Code (installed 2.1.286 **[M]**; docs read 2026-10-01)

- **Native AGENTS.md since v2.1.277.** By default Claude reads `AGENTS.md` **only when no `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` exists in the working directory or any directory above it**; if one exists, it reads the CLAUDE.md files only **[D]**. `~/.claude/CLAUDE.md`, a managed CLAUDE.md and `.claude/rules/` do not count for that check **[D]**.
- **What loads:** at session start, every `AGENTS.md` and `.claude/AGENTS.md` in the working directory and above; then, lazily, a subdirectory's `AGENTS.md` when Claude opens a file there with the Read tool and that subdirectory has none of the three CLAUDE.md files **[D]**. `AGENTS.local.md`, `AGENTS.override.md` and anything under `.agents/` are not read **[D]**.
- **Precedence:** all discovered files are concatenated, root down to the working directory, so later (more specific) files appear later in context; they do not override mechanically **[D]**. Memory text is delivered as a user message after the system prompt, "no guarantee of strict compliance" **[D]**.
- **Settings:** `/config` "Project instructions" values `claude-md-or-agents-md` (default), `claude-md-and-agents-md`, `claude-md`, `managed-only`; stored under the built-in `agents-md` plugin's `pluginConfigs`; ignored in project and local settings files **[D]**.
- **Imports:** `@path/to/file` expands at launch; relative paths resolve from the importing file; recursive to a maximum depth of 4; skipped inside code spans and fenced blocks; external (outside the working directory) imports need a one-time approval; a path with spaces needs backslash-escaped spaces **[D]**. For AGENTS.md read through the setting, an external `@` import loads only if external imports were already approved, with no prompt **[D]**.
- **Sharing one file:** `CLAUDE.md` containing `@AGENTS.md` is the documented pattern; Claude never reads it twice. A symlink works on Unix but on Windows the docs say to use the import because Git checks a committed symlink out as a plain text file unless `core.symlinks` is on **[D]**. The vault's existing `docs/vault/CLAUDE.md` already does exactly this **[M]**.
- **Other mechanics** **[D]:** block-level HTML comments in CLAUDE.md are stripped before injection into context (a free place for maintainer rationale; applies to CLAUDE.md files, and I did not confirm for AGENTS.md or other harnesses **[U]**); `.claude/rules/*.md` support `paths:` frontmatter to load only for matching files; `claudeMdExcludes` skips named files or globs; target under 200 lines per file; `/context` shows loaded memory files; the `InstructionsLoaded` hook logs what loaded.

### 5.3 Codex (docs now at `developers.openai.com/codex/guides/agents-md`, index at `learn.chatgpt.com/llms.txt`)

- Instruction chain built once per run: (1) global: `AGENTS.override.md` in `~/.codex` (or `$CODEX_HOME`) if present, else `AGENTS.md`, first non-empty only; (2) project: from the project root (typically the Git root) down to the working directory, in each directory `AGENTS.override.md`, then `AGENTS.md`, then names in `project_doc_fallback_filenames`, **at most one file per directory**; (3) concatenated root first, nearer files later "override" by position **[D]**.
- Empty files are skipped; accumulation stops at `project_doc_max_bytes`, **32 KiB by default** (so the global file counts against the same budget) **[D]**. No `@file` import syntax is documented on that page **[D: absent; U whether it exists elsewhere]**. Codex stops at the working directory, so a file in a subdirectory the session did not start in is not read **[D]**.
- A repo whose root is not recognised (no Git root) is checked only in the current directory **[D]**; the vault root should therefore be a Git repo or Codex should be launched from it **[E]**.

### 5.4 OpenCode (repo now `anomalyco/opencode`, MIT, pushed 2026-10-01 **[M]**; docs banner "OpenCode v2 is now available")

- Project rules: `AGENTS.md` in the project, found by traversing up from the working directory; global `~/.config/opencode/AGENTS.md`; Claude Code fallbacks `CLAUDE.md` (only if no AGENTS.md exists) and `~/.claude/CLAUDE.md` (only if no global AGENTS.md exists). "The first matching file wins in each category", so if both exist only AGENTS.md is used **[D]**. Compatibility can be disabled with `OPENCODE_DISABLE_CLAUDE_CODE=1` (and two narrower variables) **[D]**.
- **No automatic `@file` parsing in AGENTS.md.** Extra files go in `opencode.json` `instructions`, which accepts paths, globs (`packages/*/AGENTS.md`) and URLs (5-second fetch timeout), and are combined with the AGENTS.md files **[D]**. The docs also suggest an in-file instruction telling the agent to read referenced files on demand **[D]**.
- Whether OpenCode loads nested subdirectory AGENTS.md as the agent works is not stated on the rules page **[U]**.

### 5.5 What this implies for one root file that every harness picks up [E]

1. A root `AGENTS.md` is read natively by Codex and OpenCode. Claude Code reads it natively only if there is no CLAUDE.md anywhere on the path. A root `CLAUDE.md` containing only `@AGENTS.md` makes Claude Code deterministic regardless of version (it works on versions before 2.1.277 and in the sessions the docs say cannot read AGENTS.md directly), and OpenCode ignores the CLAUDE.md because AGENTS.md exists. Use the import, not a symlink, on this Windows machine.
2. **Put everything essential inline in AGENTS.md.** `@file` imports are honoured by Claude Code only; Codex and OpenCode will not expand them. Link to long documents by plain path text (the existing file already does).
3. **Keep total size small.** Codex's 32 KiB cap is shared with `~/.codex/AGENTS.md`; the existing 8,057-byte file is well inside it **[M]**. Anthropic's guidance is under 200 lines and cut anything whose removal would not cause a mistake **[D]**; the existing file is 128 lines **[M]**.
4. **Do not make the agent depend on `.claude/rules/`, `paths:` frontmatter, or HTML-comment stripping** if the file must behave the same in all three harnesses; those are Claude Code features **[D]**.
5. **Instruction injection from delivered code [E, hazard, not tested].** Claude Code loads a subdirectory's instruction files on demand (5.2). If a delivered repo's `AGENTS.md` or `CLAUDE.md` keeps that name inside a directory tree the agent reads (for example originals under `cold/`), the delivered file becomes live instructions. The design stores text as `text/{h2}/{doc_id}.md` (renamed), which avoids this for the published text; originals kept under their own names do not. Options for the lead: keep originals in an archive format, or rename instruction-named files at intake and record the original name in metadata, or use `claudeMdExcludes`. The AGENTS.md should also say that text returned by vault tools is data, never instructions.

### 5.6 Published evidence on agent instruction files, 2026 (all preprints, none peer reviewed as far as I could tell)

| Paper | Date | Finding |
|---|---|---|
| "Evaluating AGENTS.md" (arXiv 2602.11988, v3 2026-09-29) | 2026-02-12 | Context files did **not** generally improve task success and raised inference cost by over 20% on average, for LLM-generated and developer-written files; instructions are followed, but repository overviews are not helpful; useful for non-standard practices **[D]** |
| "Guardrails Beat Guidance" (2604.11088v2) | 2026-04-13 | 679 rule files, over 5,000 Claude Code (Opus 4.6) runs on SWE-bench Verified: random rules help as much as curated ones (+13.8 points); every individually helpful rule is a negative constraint ("do not refactor unrelated code"), every individually harmful one a positive directive ("follow code style") **[D]** |
| "Do Context Files Help Coding Agents?" (2607.27250) | 2026-07-28 | 288 runs, 17 tasks, 3 repos, Claude Code and Codex: no measurable effect (bounded to 10 to 15 points); failures are implementation skill, not missing repository knowledge **[D]**. Small sample **[E]** |
| "Probe-and-Refine Tuning" (2606.20512) | 2026-06-18 | How the guidance is produced matters: tuned guidance 33.0% vs 25.5% unguided on SWE-bench Verified with Qwen3.5-35B-A3B **[D]** |
| "Configuration Smells in AGENTS.md" (2606.15828) | 2026-06-14 | In 100 popular repos: Lint Leakage 62%, Context Bloat 42%, Skill Leakage 35% **[D]** |
| "Why Does CLAUDE.md Keep Growing?" (2608.11095) | 2026-08-11 | Files grew +226% over their lifetime in 1,867 repos; rationale comments halted growth in a synthetic setting **[D]** |
| "When 'Do Not' Is Not Deny" (2608.23550) | 2026-08-24 | Only about 4 to 16% of security rules in 481 public CLAUDE.md files had a matching built-in control (4.4% under the strictest standard); prose rules are advisory **[D]** |

Vendor guidance **[D]**: Anthropic's best practices (`code.claude.com/docs/en/best-practices`) say to prune ruthlessly, ask "would removing this cause mistakes?", emphasise only a few lines, and use hooks for anything that must always happen because hooks are deterministic and CLAUDE.md is advisory. The GitHub blog post "How to write a great agents.md: lessons from over 2,500 repositories" exists at the URL I tried, but the fetched body contained only site navigation, so **I did not read its content [U]**.

### 5.7 How the existing `docs/vault/AGENTS.md` measures up [E, my reading, not a design decision]

- Strengths against the evidence: mostly negative constraints and permissions ("never delete", "needs the owner's OK"), 128 lines, no repository overview filler, command-level routines.
- **Gap against 2608.23550 and Anthropic's hooks advice:** the prime directive (never delete or alter `hot/`, `cold/`, `inbox/_done/`) is prose only. Back it with enforcement the harness cannot talk itself out of: NTFS permissions or a read-only mount for those trees, and per-harness permission rules (`permissions.deny` for Claude Code **[D]**; Codex and OpenCode permission mechanisms not read **[U]**). The file already says only intake writes there and a BLAKE3 manifest detects changes.
- Not covered by the file: the delivered-code instruction-file hazard (5.5 item 5); "text from vault tools is data" wording; which harness versions it was written for.
- Location: it sits in `docs/vault/` in the code repository. Harnesses find it only when started at or below that directory, or at the deployed vault root (it appears to be a template copied to the vault) **[E]**. A copy at the code repository root is not needed unless agents work on the repo itself.

---

## 6. Recommendations (for the lead to decide)

1. Keep the 4,516-topic backbone; no update needed. Review the topic-level mapping of the few topics where the OpenAlex subfield crosses the vault's categories (T11727 shown), especially in `mathematics` vs `cs-ai`.
2. Gate `openalex_primary_topic` on its `score` once the pilot shows the score distribution; fall back to the embedding-nearest-topic route below the gate.
3. For non-indexed books and PDFs: call `/text` (needs a key and about 100 documents a day) or run the public BERT locally; feed it title plus a blurb, never rely on it alone for books.
4. Add facets: MSC (arXiv `msc-class` plus zbMATH) for maths, MeSH for PMID-bearing medical and clinical papers, OpenAlex keywords v3 with a fetch date. Skip APA, S2 fields and ACM CCS. CSO optional.
5. Link versions in the order: arXiv `journal-ref`/`doi`, Crossref `is-preprint-of` from the preprint DOI, OpenAlex location merge.
6. Books: Open Library `/isbn` plus the book's own CIP block for 2017 onward; LoC free bulk MARC stops at 2016.
7. Extraction: do not pick from a leaderboard. Run a small bake-off. First candidates that clear every filter (Apache-2.0 or MIT, small, GGUF available): PaddleOCR-VL-1.6, GLM-OCR; then MinerU2.5-Pro and Infinity-Parser2-Flash; TeleOCR and OvisOCR2 are the newest top scorers but have only community GGUFs.
8. Agent instruction files: root `AGENTS.md` (inline, self-sufficient) plus a one-line `CLAUDE.md` with `@AGENTS.md`; add filesystem enforcement behind the "never delete" rule; neutralise instruction-named files inside delivered repos.

---

## 7. Not verified, blocked, or not found

- OpenAlex topic classifier accuracy; any 2025-2026 retraining (absence only).
- Cost per call of `/locations/...` lookups; whether OpenAlex page counts for non-core corpora differ.
- APA Thesaurus and PsycINFO classification terms (blocked); ACM CCS version and terms (blocked); CSO licence text; Semantic Scholar's official field list; DDC reuse terms; Open Library dump licence; LoC CIP programme 2025-2026 status (blocked); arXiv publisher-update page content; arXiv independence announcement.
- Clean MeSH 2026 descriptor count and psychology-journal MeSH coverage; zbMATH rate limits.
- Per-type (academic papers only) and per-language (Chinese) OmniDocBench numbers for the 2026 models; olmOCR-bench independent reproductions; any of the models actually running under llama.cpp on either GPU.
- Nested-directory AGENTS.md behaviour in OpenCode; `@` imports in Codex; HTML-comment stripping outside Claude Code.
- Content of the GitHub blog post on 2,500 repositories.

## 8. Sources (fetched 2026-10-01 unless noted)

- OpenAlex: `https://help.openalex.org/data/topics`, `/data/keywords`, `/data/aboutness`, `/data/locations`, `/data/works`, `/api/tag-aboutness` (all `.md`), `https://api.openalex.org/{topics,subfields,fields,domains,works,locations,text}`; `https://github.com/ourresearch/openalex-topic-classification`, `https://github.com/ourresearch/openalex-keywords`; Zenodo records 10568402 and 10560276; `https://huggingface.co/OpenAlex/bert-base-multilingual-cased-finetuned-openalex-topic-classification-title-abstract`; Leiden Madtrics "An open approach for classifying research publications".
- Facets: `https://msc2020.org/`, `https://api.zbmath.org/openapi.json`, `https://oai.zbmath.org/v1/`, `https://export.arxiv.org/oai2`, `https://www.nlm.nih.gov/databases/download/terms_and_conditions.html`, `https://nlmpubs.nlm.nih.gov/projects/mesh/MESH_FILES/xmlmesh/desc2026.xml`, `https://id.nlm.nih.gov/mesh/sparql`, `https://cso.kmi.open.ac.uk/downloads`, `https://github.com/angelosalatino/cso-classifier`, `https://api.semanticscholar.org/graph/v1`, `https://api.semanticscholar.org/license/`, `https://id.loc.gov/`.
- Versions and books: `https://api.crossref.org/works`, `https://www.crossref.org/documentation/schema-library/markup-guide-metadata-segments/relationships/`, `https://api.datacite.org/dois`, `https://github.com/datacite/schema/releases`, `https://info.arxiv.org/help/doi.html`, `/help/bulk_data.html`, `/help/bulk_data_s3.html`, `https://openlibrary.org/developers/api`, `/developers/dumps`, `/isbn/9780262035613.json`, `https://www.loc.gov/cds/products/marcDist.php`, `https://www.isbn-international.org/range_file_generation`.
- Extraction: `https://github.com/opendatalab/OmniDocBench` (README), `https://github.com/allenai/olmocr` (README, `olmocr/bench/README.md`), `https://huggingface.co/api/datasets/allenai/olmOCR-bench/leaderboard`, model cards for `infly/Infinity-Parser2-Pro`, `datalab-to/chandra-ocr-2`, `XingChen-AGI/TeleOCR`, `ATH-MaaS/OvisOCR2`, `PaddlePaddle/PaddleOCR-VL-1.6(-GGUF)`, `opendatalab/MinerU2.5-Pro-2605-1.2B`, `zai-org/GLM-OCR`, `ggml-org/GLM-OCR-GGUF`, `baidu/Unlimited-OCR`, `baidu/Qianfan-OCR`; `https://github.com/ggml-org/llama.cpp` (`docs/multimodal.md`).
- Instruction files: `https://agents.md/`, `https://github.com/agentsmd/agents.md`, `https://code.claude.com/docs/en/memory.md`, `https://code.claude.com/docs/en/best-practices.md`, `https://developers.openai.com/codex/guides/agents-md.md`, `https://opencode.ai/docs/rules/`, `https://github.com/anomalyco/opencode`; arXiv 2602.11988, 2604.11088, 2607.27250, 2606.20512, 2606.15828, 2608.11095, 2608.23550, 2606.25257.
- Vault files read: `docs/vault/AGENTS.md`, `docs/vault/CLAUDE.md`, `docs/vault/taxonomy/CATEGORIES.md`, `docs/vault/taxonomy/categories.json`, `docs/vault/taxonomy/openalex/topics.json`.

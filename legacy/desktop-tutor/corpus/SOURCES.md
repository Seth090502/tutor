# Legal open sources (scale path to 10k+ without copyright violation)

V1.0 does NOT vendor Harvard/MIT books or paywalled PhD papers. Instead:
- Index **metadata + links + abstracts** (unlimited scale, ~KB per record).
- Store **full text only** for: public-domain / CC-BY / your own notes.

## Open full-text you can ingest today (free, legal)

Math:
- OpenStax Calculus / Algebra / Precalculus — https://openstax.org (CC-BY)
- MIT OpenCourseWare 18.01/18.02 notes + problems — https://ocw.mit.edu (CC-BY-NC-SA)
- Khan Academy math articles (link + summarize in your own words)

English:
- Purdue OWL (grammar/rhetoric) — https://owl.purdue.edu (link + own notes)
- OpenStax English Composition — https://openstax.org (CC-BY)

Science:
- OpenStax Biology / Physics — https://openstax.org (CC-BY)
- MIT OCW 7.01 / 8.01 — https://ocw.mit.edu
- NASA / NIH open articles (public domain / CC)

Chemistry:
- OpenStax Chemistry — https://openstax.org (CC-BY)
- MIT OCW 5.111 — https://ocw.mit.edu

Papers (US + China, open subset):
- arXiv — https://arxiv.org (open preprints, check license per paper)
- PubMed Central — https://pmc.ncbi.nlm.nih.gov (open biomedical)
- DOAJ — https://doaj.org (open-access journals incl. Chinese publishers)
- CNKI / Wanfang are NOT bulk-scrapable; record metadata + DOI, ingest only
  papers you have lawful access to.

## How to grow to 10,000 records

1. Add rows to `corpus/manifest.json`: {id, title, license, url, local_path?, doi?}.
2. Drop PDFs/md you legally own into `corpus/ingest/`.
3. Run `python scripts/build_index.py` -> updates `corpus/index.json`.
4. Search via `python -m tutor.cli search --q "..."`.

Each manifest row is ~300 bytes; 10,000 rows ≈ 3 MB (fine for this folder).
Full-text bulk of copyrighted books would be 50–200 GB and break OneDrive sync —
that is why V1.0 indexes instead of vendoring.

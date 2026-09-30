# Synthesis: open sources for the Tutor corpus

Date: 2026-09-30. Inputs: F1 to F5. I re-opened 28 primary pages or API records. Fetch results are small-model summaries, so quotes are secondhand. Nothing was downloaded.

## Answer to the owner's question

No. Chinese institutional papers and teaching material are not all open source. The same holds for Japan, Korea, Singapore, Finland and Estonia. Free to read is not the same as openly licensed. What I verified:

- **Open:** MEXT (Japan) site content, reusable with credit and a modification note, and stated compatible with CC BY. Also ChinaXiv preprints, but only those the author licensed CC BY, BY-SA, CC0 or BY-NC-SA. ChinaXiv's bulk rules are silent, so email first.
- **Non-commercial:** Korea's KOGL Type 2 items.
- **All rights reserved, free to read:** PEP textbooks, Singapore MOE, NIER, EDUFI, most J-STAGE articles.
- **Unverified:** OECD/PISA, Estonia, Finland's English documents, smartedu.cn terms.

The usable free corpus is therefore mostly Western and mostly undergraduate-level. PhD-level depth has to come from the owner's own lawfully acquired library. Curricula from East Asia and the Nordics serve as scope-and-sequence maps, not content.

## Source register (acquisition recommendation)

"Auto" means a script may fetch it, with the per-item licence recorded in `corpus/manifest.json`. "Owner" means the owner adds a lawful copy, or the manifest holds metadata and a link only.

| Source | Licence (text link) | Bulk route | Format, size, language | Recommendation |
|---|---|---|---|---|
| MIT OpenCourseWare | CC BY-NC-SA 4.0, AI use explicitly allowed within the licence ([terms](https://ocw.mit.edu/terms/)) | robots `Allow: /` plus sitemap ([robots](https://ocw.mit.edu/robots.txt)). Terms do not address bulk. Use per-course downloads, politely | PDF, HTML, transcripts. A few MB per course. English | **Auto**, private corpus only. Skip video, keep transcripts. Third-party items inside courses have their own terms |
| GitHub-source math texts: Judson (GFDL), Lebl (CC BY-NC-SA / CC BY-SA), OpenIntro Statistics (CC BY-SA 3.0), Stacks (GFDL), APEX (CC BY-NC-SA) | per repo LICENSE ([GFDL](https://www.gnu.org/licenses/fdl-1.3.html); [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/legalcode)) | git clone | LaTeX, PreTeXt. Tens of MB. English | **Auto** after reading each repo's LICENSE. Only some were checked (moderate confidence) |
| Project Gutenberg | Public domain in the US only ([permission](https://www.gutenberg.org/policy/permission.html)) | Sanctioned: `wget -w 2 -m -H "https://www.gutenberg.org/robot/harvest?filetypes[]=txt"` or the feeds directory. The site is "intended for human users only" ([robot policy](https://www.gutenberg.org/policy/robot_access.html)) | `txt-files.tar.zip` 11 GB, `pg_catalog.csv` 20 MB ([feeds](https://www.gutenberg.org/cache/epub/feeds/)). Multilingual | **Auto** from feeds. Index only a curated list of about 300 to 1,000 works |
| Standard Ebooks | CC0 production, texts believed US public domain ([bulk page](https://standardebooks.org/bulk-downloads)) | Bulk zips are a Patrons Circle benefit. Per-book downloads and public git repos are not gated | EPUB, git. About 1,500 titles (inferred). Literature and philosophy | **Auto** per book or repo. Never bypass the gate |
| Wikisource, Wikibooks | CC BY-SA 4.0 + GFDL ([policy](https://en.wikisource.org/wiki/Wikisource:Copyright_policy)) | dumps.wikimedia.org dumps. Do not crawl | bz2 wikitext. Wikisource English about 3.6 GB (F5, not re-checked). Wikibooks size unverified | **Auto**, filtered. Wikibooks quality is uneven and Wikisource OCR fidelity varies |
| MEXT (Japan) | Site terms: reproduction, translation, adaptation and commercial use with credit and a modification note. The terms state compatibility with CC BY ([terms](https://www.mext.go.jp/b_menu/1351168.htm)). Third-party content is excluded | No bulk terms read | 16 or so English PDFs, 19 to 215 KB each. Japanese originals | **Auto**, low value. K-12 standards only. No lower- or upper-secondary maths or science in English. The PDFs carry a separate "Copyright (C) MEXT" footer |
| ChinaXiv | Author-chosen CC BY 4.0, BY-SA 4.0, BY-NC-SA 4.0 or CC0, per paper ([licences](https://chinaxiv.org/user/license.htm)) | Terms silent on mining and crawling | Preprints, not peer reviewed. About 47,900 records. Chinese and English | **Owner decision.** Email eprint@mail.las.ac.cn before any harvest. Until then, metadata and links |
| OpenAlex | Metadata CC0 ([pricing](https://help.openalex.org/access/pricing/)) | API with free key ($1/day) | JSON | **Auto** for the metadata index only. Skip the roughly 745 GB snapshot |
| arXiv | Metadata CC0. Papers carry a per-paper licence ([TOU](https://info.arxiv.org/help/api/tou.html)) | One request per 3 s, one connection | TeX, PDF | **Auto** only for CC BY, BY-SA or CC0 papers, using the stricter rate |
| PMC Open Access | Per article CC licence | Only Cloud (S3/HTTPS), OAI-PMH, E-Utilities, BioC. Other systematic retrieval is prohibited ([list](https://pmc.ncbi.nlm.nih.gov/tools/openftlist/)). Legacy FTP is deprecated August 2026 | JATS XML | **Auto** for CC BY/CC0 items from a curated seed list |
| LibreTexts | Per page. Shelf pages often "not declared" | robots: Crawl-delay 5, `/@*` and `action=*` disallowed ([robots](https://chem.libretexts.org/robots.txt)) | Per-book PDF. English | **Owner-selected**, one book at a time, licence logged per book. No crawler |
| Open Textbook Library / AIM list | Catalogue metadata CC BY 4.0 ([license.xml](https://open.umn.edu/opentextbooks/license.xml)). Each book keeps its own licence | Crawl-delay 120 (F1, not re-checked) | Links | Use as a **selection aid**. CC BY-ND books (Book of Proof) are read-only |
| OpenStax | CC BY-NC-SA 4.0 **plus** a ban on LLM training or ingestion without written permission ([book page](https://openstax.org/books/chemistry-2e/pages/1-introduction)). robots.txt blocks GPTBot on `/books/` | n/a | About 60 books. English | **Do not automate.** Owner may file OpenStax's reuse request. Human reading only until permission arrives |
| Korea NCIC/KICE | KOGL Type 2: attribution, non-commercial, derivatives allowed ([KOGL](https://www.kogl.or.kr/info/licenseTypeEng.do)). Unmarked items need permission | not assessed | Korean. English availability unverified | **Owner**, personal use only, never exported |
| J-STAGE | Publisher or author copyright. CC tag on open-access articles ([terms](https://www.jstage.jst.go.jp/static/pages/TermsAndPolicies/ForIndividuals/-char/en)) | **Forbidden:** downloading a large amount by mechanical means. Translation and derivatives need holder consent | Mostly Japanese PDF/HTML | **Owner**, one article at a time. Metadata and links only otherwise |
| Singapore MOE, NIER, EDUFI, Estonia, IEA/TIMSS, OECD | MOE: fair dealing only, no modification, no commercial reuse ([terms](https://www.moe.gov.sg/terms-of-use)). NIER: "All Rights Reserved". IEA: click-through licence. EDUFI, Estonia, OECD: not verified | none | Mostly PDFs and datasets | **Owner** or link only. Use as syllabus maps |
| CNKI, PEP textbooks, smartedu.cn, Chinese MOOCs, Khan Academy | Proprietary or user agreement | Not permitted or unread | n/a | **Owner** copies only. No scrapers |
| Journals in the learning-science canon (Dunlosky, Adesope, Yang, Murray, Melby-Lervåg) | Publisher TDM licences or none | none | PDF | **Owner** copies. Write own-words evidence notes |
| Internet Archive, HathiTrust, Darwin Online, Newton Project | Terms unread or per item | `ia download` exists | OCR scans, noisy | **Owner** or read-only links until the terms are read |

## Verified evidence table

"Verified" means I opened the source and it says what the report claims. "Corrected" means the claim needed a change.

| Claim | Status | What the source says |
|---|---|---|
| OpenStax is CC BY-NC-SA 4.0 and bars LLM ingestion without written permission | Verified | Chemistry 2e page quotes the clause. robots.txt has `User-agent: GPTBot / Disallow: /books/`. Six books were sampled by F1 and one re-checked here |
| MIT OCW is CC BY-NC-SA 4.0 and permits AI training within its terms | Verified | "OCW content may be used to train, develop, and improve artificial intelligence and machine learning models", provided attribution, non-commercial use and ShareAlike. **Corrected:** the terms do not address bulk download, so the F1 "Download Course" route is unconfirmed here |
| MEXT permits reuse with credit and a modification note, compatible with CC BY | Verified | Japanese text of the terms. The English PDF page has a separate MEXT copyright footer, and the coverage question stays open |
| J-STAGE bans mechanical bulk download | Verified | Article 3, Paragraph 1, Item 5 |
| PG bulk access only by sanctioned routes | Verified | `-w 2` harvest command, human-only statement, feeds sizes 11 GB, 20 MB and 121 MB |
| Standard Ebooks bulk downloads are members-only and its work is CC0 | Verified | Quoted |
| ChinaXiv offers four author-chosen licences and is silent on mining and crawling | Verified | Via a fetch summary |
| PMC forbids other systematic retrieval. Legacy packages are removed | Verified, corrected | The page lists Cloud, OAI-PMH, E-Utilities and BioC. The announcement gives August 2026. The exact day "26 Aug" was not confirmed |
| arXiv: one request per 3 s, metadata CC0 | Verified | The conflicting bulk-page limit was not re-opened, so apply the stricter limit |
| OpenAlex is CC0 with a free key | Verified | $1 a day. The 429 experience was not retested |
| KOGL Type 2 is non-commercial and allows derivatives | Verified | Quoted |
| OTL catalogue metadata is CC BY 4.0 | Verified | Via a fetch summary of the XML |
| US public domain covers works published before 1931 | Verified, corrected | The fetch summary said 1931 works entered in 2026. That is wrong. A 95-year term puts 1930 works in on 1 Jan 2026 and 1931 works in on 1 Jan 2027, so the rule holds for today's date. US only |
| Yang 2021: quizzing g = 0.499, 222 studies | Verified | 48,478 students. Crossref licence is the publisher PDF link, not an open licence |
| Pan & Rickard 2018: transfer d = 0.40, CI [0.31, 0.50] | Verified | 122 experiments, 67 articles, 10,382 participants. Crossref lists an accepted-manuscript route under APA policy after 310 days. That is not a CC licence |
| Melby-Lervåg 2016: no convincing far transfer | Verified | 87 publications, 145 comparisons, compared with treated controls. SAGE TDM licence only |
| Kestin 2025: designed AI tutor beat in-class active learning | Verified | CC BY-NC-ND 4.0. The abstract gives no effect size, so quote none |
| Bastani 2025: unguarded GPT-4 harms unaided performance | Verified | Practice grades +48% (base) and +127% (tutor). After access removal, -17% for GPT Base. Guardrails largely mitigate it. CC BY-NC-ND 4.0 |
| Bego 2024: spacing significant in 2 of 9 STEM courses | Verified, corrected | CC BY. The two courses were Calculus I for Engineers and Chemistry for Health Professionals. The pooled effect disappeared without calculus. F4's reading that spacing is weaker in maths is not supported by this paper alone |
| Murray 2025: smaller spacing effect in maths, testing alone uncertain | **Partly verified** | The title and the TDM-only licence are confirmed. The publisher elided the abstract, so the findings rest on F4 alone. Flagged |
| LibreTexts crawl limits, Wikisource CC BY-SA + GFDL, MOE terms, NIER footer | Verified | Crawl-delay 5, Request-rate 1/5. Copyright 2015 NIER |

## Conflicts between reports and how I resolved them

1. **Which licences count as open for auto-fetch.** F1 puts NC-SA (OCW) in Tier A. F2 and F4 limit Tier A to CC0, CC BY and CC BY-SA. The project's AGENTS.md says full text only for public-domain, CC-BY or the owner's own notes. Resolution: the strongest evidence is the licence text, and OCW's terms allow AI use within NC-SA. But the stated project rule is stricter than either report. **The owner must amend the AGENTS.md rule or lose OCW, BY-SA, GFDL and Wikisource.** My recommendation is to amend it to allow CC BY-SA, GFDL and NC-SA material for a private, single-user, never-distributed corpus, with `ai_use` gating per item. Until then, only PD, CC0 and CC BY are auto.
2. **OpenStax.** Common memory says CC BY. The page says CC BY-NC-SA plus a no-LLM clause. Page text wins. Excluded.
3. **"Only MEXT is open" (F3) versus a larger open set (F1, F5).** F3 covered government curricula only. MEXT is the one verified permissive government source.
4. **ChinaXiv as Tier A (F2).** F2's own text says the terms are silent on crawling. I downgraded it to owner decision pending the email.
5. **Spacing in maths.** F4 implies weaker effects in maths. Bego shows the positive results sat in calculus and chemistry, and the pooled effect depended on calculus. The two fit only under "heterogeneous, do not promise a fixed gain", which is what the rule below says.
6. **Sizes.** F1's Wikibooks dump figure (about 207 GB) looks wrong, and I dropped it. Other sizes are report figures.
7. **arXiv rate limits** conflict between pages. Use one request per 3 s.

## Rules for Tutor, ranked by evidence strength times impact

Corpus rules rest on licence text I re-read. Learning rules rest on effect sizes I confirmed.

1. **[corpus, UI] A licence gate on every item.** Add `license_spdx`, `license_url`, `ai_use` (allowed/restricted/unknown), `nd_flag`, `acquisition` (auto / owner / link-only) and `source_checked_date` to `corpus/manifest.json`. The indexer refuses to embed or send to an LLM any item whose `ai_use` is not allowed, unless the owner overrides. The Library shows a badge (Open, Free to read, Owner copy). Strength: strong. Impact: highest.
2. **[corpus] Exclude OpenStax until written permission.** Also exclude Khan Academy and any free PDF with no licence. The owner may submit OpenStax's reuse request. Strength: strong (quoted clause).
3. **[corpus] Automate only open material, politely.** Sources: OCW per course, GitHub-source texts, PG feeds, Standard Ebooks repos, Wikisource and Wikibooks dumps, OpenAlex metadata with a key. Honour robots, Crawl-delay and 429s, and record SHA-256 and licence text per item. Never crawl J-STAGE, LibreTexts, gutenberg.org pages or CNKI.
4. **[corpus] Keep the corpus private.** NC-SA and GFDL are copyleft and PG has trademark limits. Do not commit fetched texts to a repo or ship them in the app. Keep attribution in a NOTICE file.
5. **[voice/chat tutor prompt] Attempt first, then hints, then a worked solution. Never answer first.** Bastani 2025: access raised practice grades 48% (127% with guardrails) but cut later unaided grades 17%, and guardrails largely removed that harm. Strength: moderate (one RCT, high-school maths, abstract numbers only). Impact: high.
6. **[measurement] Add unassisted delayed retention probes** days after study, scored separately from in-session accuracy, plus a strength tag (strong/moderate/weak/contested) on every evidence card. Strength: moderate. Impact: high, because it detects the Bastani failure.
7. **[question generation] Retrieval items with corrective feedback and varied formats.** Yang 2021: g = 0.499 over 222 classroom studies. Pan & Rickard 2018: transfer d = 0.40, strongest for different formats and application items, weakest for untested material. Keep the atomic item with explanation and misconception. Strength: strong for classroom testing. Impact: high.
8. **[scheduling] Keep FSRS at 0.9, but promise no fixed spacing gain in maths.** Bego 2024 found effects in 2 of 9 courses. Murray 2025 is unverified by me. Measure the owner's own retention instead. Strength: moderate. Impact: medium.
9. **[tutor prompt, corpus] Provenance.** Cite source, licence class and original title. Quote only short passages from non-open items. Tag Gutenberg and similar items `historical-primary`, never the current state of a field. Label English renderings of foreign texts unofficial unless the source provides them. Make no "how Japan, Singapore or China teaches" claims without a retrieved document. Strength: moderate (design rule). Impact: medium.
10. **[question generation] Items carry `source_id` and licence.** ND sources (Book of Proof) are not seeds. Items from non-open sources are original, with no long quotation.
11. **[corpus, measurement] Curate, then measure.** Start with roughly 300 to 1,000 public-domain works, not 79,509. Check `eval-search` on the held-out test split before and after each new source, and prune if the hit rate drops. Use curricula as topic maps only.
12. **[feature] If Dual N-Back ships, make no transfer claims.** Melby-Lervåg 2016: 87 publications, 145 comparisons, no convincing far transfer against treated controls. Training improves the trained task. Strength: strong against far-transfer claims. Impact: medium.
13. **[corpus] Evidence cards use original notes** for NC/ND and TDM-only papers (Kestin, Bastani, Yang): link plus own-words notes with metric, N and boundary conditions.
14. **[UI] "Add my copy" action** for metadata-only entries, plus an attribution export for open items.

## What not to build

- An OpenStax ingester, or anything that sends OpenStax text to an LLM.
- Scrapers for J-STAGE, CNKI, smartedu.cn, PEP, NIER, MOE, EDUFI, OECD, LibreTexts, Khan Academy, gutenberg.org pages, or the Standard Ebooks bulk zips.
- Mirrors of PMC, arXiv, S2ORC or the OpenAlex snapshot (745 GB, no gain over API queries).
- A full 79k-book Gutenberg index.
- A machine-translation pipeline republishing translated Japanese or Korean material.
- An IEA/TIMSS ingester or a national curriculum benchmark score.
- Any pipeline relying on PRC Copyright Law Article 5 for curriculum standards (scope unresolved).
- A default raw-LLM answer mode, or a dual n-back feature sold as raising intelligence.

## Open items for the owner

1. Decide whether to amend the AGENTS.md corpus rule (conflict 1).
2. Decide whether to file the OpenStax reuse request, and whether to email ChinaXiv.
3. State your country. Gutenberg and Cornell rules are US law, and personal-use copying elsewhere differs.
4. Confirm in a browser: OECD/PISA licence, Finland and Estonia English curricula, smartedu.cn terms, MEXT coverage of the English PDFs.
5. Read Murray 2025 before quoting it.

## Citation list with verification status

Verified (opened by me on 2026-09-30):
- openstax.org/books/chemistry-2e/pages/1-introduction (CC BY-NC-SA 4.0 and LLM clause); openstax.org/robots.txt (GPTBot)
- ocw.mit.edu/terms/ (AI use allowed; bulk not addressed); ocw.mit.edu/robots.txt
- mext.go.jp/b_menu/1351168.htm (reuse terms); mext.go.jp/en/policy/education/elsec/title02/detail02/1373859.htm (document list, sizes, footer)
- jstage.jst.go.jp terms (Art. 3 Para. 1 Item 5; Art. 2 Para. 2)
- gutenberg.org/policy/robot_access.html; gutenberg.org/cache/epub/feeds/
- standardebooks.org/bulk-downloads
- chinaxiv.org/user/license.htm
- info.arxiv.org/help/api/tou.html
- help.openalex.org/access/pricing/
- pmc.ncbi.nlm.nih.gov/tools/openftlist/; ncbiinsights.ncbi.nlm.nih.gov/2026/02/12/pmc-article-dataset-distribution-services/
- kogl.or.kr/info/licenseTypeEng.do; ncic policy (F3 only, not re-opened)
- open.umn.edu/opentextbooks/license.xml
- guides.library.cornell.edu/copyright/publicdomain (corrected)
- en.wikisource.org/wiki/Wikisource:Copyright_policy; chem.libretexts.org/robots.txt; moe.gov.sg/terms-of-use; nier.go.jp/English/index.html
- Yang et al. 2021, doi 10.1037/bul0000309 (g = 0.499)
- Pan & Rickard 2018, doi 10.1037/bul0000151 (d = 0.40)
- Melby-Lervåg et al. 2016, doi 10.1177/1745691616635612
- Kestin et al. 2025, doi 10.1038/s41598-025-97652-6
- Bastani et al. 2025, doi 10.1073/pnas.2422633122
- Bego et al. 2024, doi 10.1186/s40594-024-00468-5 (corrected)

Partly verified: Murray, Horner & Göbel 2025, doi 10.1007/s10648-025-10035-1 (title and licence only).

Not re-checked, taken from reports (moderate or unverified): GitHub text licences; OTL Crawl-delay; dump sizes; Standard Ebooks repo count; DOAJ counts; Latimier 2021, Freeman 2014, Tabibian 2019, Diamond & Ling 2016; Unpaywall, Europe PMC, CORE terms; OECD, Finland, Estonia, IEA licences; PRC Copyright Law Art. 5.

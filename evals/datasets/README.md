# Eval datasets

`evals/datasets/*.jsonl`: one JSON object per line, UTF-8, LF line endings. Questions and pointers
only, never chunk text. (They paraphrase your corpus, so decide before committing one whether that
is acceptable; `corpus/` itself is never committed.)

```json
{"qid": "syn-3f9a1c0b7d2e", "question": "Why does an eigenvector keep its direction under its matrix?",
 "gold": [{"chunk_id": "c_8841", "doc_id": "d_17", "path": "corpus/math/books/linear-algebra.pdf", "page": 212}],
 "domain": "math", "split": "train", "kind": "synthetic", "source_chunk_id": "c_8841"}
```

| Field | Type | Notes |
|---|---|---|
| `qid` | string | Unique in the file. Generated ones are `syn-` + a hash of the source chunk id. |
| `question` | string | What the tutor would be asked. |
| `gold` | list of objects, non-empty | The chunks/pages that answer it (see below). |
| `domain` | string | `math`, `english`, `science`, `chemistry`, ... Used for stratified sampling and per-domain metrics. |
| `split` | `"train"`, `"test"` or `null` | `null` until `python -m tutor_evals split` assigns it. `run` refuses unassigned rows. |
| `kind` | `"synthetic"` or `"human"` | Who wrote the question. |
| `source_chunk_id` | string, optional | Chunk a synthetic question was written from. |

Unknown extra fields are kept by every command. Use `"hard": true` to mark hard cases you found
while spot-checking (the harness does not score them separately yet: contract C2 has no slot).

## Gold entries

`{chunk_id?, doc_id?, path, page?}`. `path` is required. An entry needs a `chunk_id` or a `page`,
otherwise no hit could ever match it and loading fails.

A returned hit matches a gold entry if **any** of these holds:

1. `gold.chunk_id == hit.chunk_id`
2. `gold.doc_id == hit.doc_id` and `gold.page` lies in `[hit.page_start, hit.page_end]`
3. `gold.path == hit.path` (separators normalised) and the same page test

Rules 2 and 3 need a gold page and a hit with a page span, so page-less sources (plain text,
markdown notes) can only be matched by `chunk_id`. Rules 2 and 3 keep a dataset usable after a
re-chunk changes chunk ids, at the price of counting any chunk on the gold page as a hit.
Each gold entry is credited at most once (by the highest-ranked matching hit); further hits that
only match already-credited entries count as non-relevant, so metrics stay within [0, 1].

## Split

`python -m tutor_evals split <file>` hashes each **source document** with the seed (default 0)
and sends about 70% of documents to train, 30% to test. The unit is the document, never the chunk,
so near-duplicate neighbouring passages cannot leak across splits. A document is identified by its
`doc_id` and its `path`; a question with gold in several documents ties them into one group.
A document's split depends only on the seed and the documents it shares questions with, so test
documents stay test as the dataset grows. Counts are by question: a few large documents make the
70/30 ratio lumpy, and `split` warns when it is off by more than 10 points.

## Generation record

`generate` writes `<dataset>.gen.json` next to the dataset: per invocation the model, source,
requests, tokens, dropped counts and cost, plus a running total. It never stores the API key.

## Chunks export (for `generate --chunks-jsonl`)

One rag-engine `/search` hit object per line (contract C1). Read fields: `chunk_id`, `doc_id`,
`path`, `domain`, `text`, `page_start`.

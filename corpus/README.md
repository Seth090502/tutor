# Corpus: your books and papers

Put your teaching material here. Git never tracks anything in this folder except this
README and the empty folder skeleton, so books and papers stay on this machine.

## Where things go

```
corpus/
  inbox/            anything not sorted yet
  math/books/       math/papers/
  english/books/    english/papers/
  science/books/    science/papers/
  chemistry/books/  chemistry/papers/
```

Subfolders inside these are fine (for example `math/books/calculus/`).

## What is read

- PDF, EPUB, Markdown (`.md`) and plain text (`.txt`). Other files are counted and skipped.
- Encrypted or copy-restricted PDFs are reported and skipped until you decide otherwise.
- Only material you own or that is openly licensed. DRM is never removed.

## Space

This folder is outside OneDrive on purpose. Plan for roughly 1.3x the library size in free
space once parsed text and search indexes exist (a 250 GB library needs about 325 GB).

## Measure what is here

From the project root:

```
cargo run -p rag-engine --release -- survey --root corpus --out data/rag/survey.json
```

The report gives file and page counts, duplicates, how many pages need OCR (scanned or
math-heavy), and projected tokens and cost for the full library.

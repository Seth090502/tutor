# Extraction spike: xberg 1.3.0 against PDFium for PDF, EPUB, DOCX and ODT

Measured on this Windows 11 machine (MSVC, rustc 1.98.1, `-j 4`, release builds) on 2026-10-01. The spike
program is `crates/vault-extract/spike` (standalone Cargo workspace, own lockfile that is not committed). It
prints counts only; extracted text went to `target/` and is not in git.

## Verdict

- **PDF (born-digital papers and books): adopt PDFium through `pdfium-render` 0.9.4**, with the layout analysis
  now in `vault-extract` (`pdf` feature, on by default). It is about 22 times faster than xberg on the sample,
  keeps the content stream's paragraph order where xberg's re-ordering scrambles dense maths pages, reads
  `/PageLabels` correctly, and exposes the glyph geometry needed for columns, hyphens, headings and
  running-header removal. The cost is native code in the process: a hostile PDF can crash or hang PDFium, so
  intake should convert PDFs in a child process with a timeout.
- **EPUB, DOCX, ODT: use our own converters** (`epub`, `docx`, `odt` modules). xberg builds and works on
  Windows, so the SPEC's condition for using it is met, but on a structural sample it loses or misplaces things
  our converters keep (below), and it brings 417 locked crates against about 110 for `vault-extract`.
- xberg stays a possible second opinion; nothing in the crate depends on it.

## Sample

The brief said 17 PDFs. `D:/Corpus-pilot2` holds 15 and `D:/oaf-v-1991` holds 3 more that are byte-identical
copies of three of the 15, so there are **15 unique PDFs, 275 pages**: 5 arXiv maths papers (15 to 49 pages), 5
MIT items (4 to 42 pages: a psychology article, lecture notes, papers), 5 OCW course handouts. Nothing in it is
book length (the largest has 49 pages), and none carries roman or offset page labels (the 5 that have
`/PageLabels` use labels equal to the physical index). Those two gaps are covered only by synthetic files.

## Build

| | result |
|---|---|
| xberg 1.3.0, features `pdf office html xml archives tokio-runtime`, release | builds on MSVC with no errors, 7 min 35 s at `-j 4`, 328 crates compiled (417 in the lock), 32 MB binary |
| `pdfium-render` 0.9.4, `pdfium_latest`, `thread_safe` | builds in seconds; binds at run time to `C:/Users/sethy/Projects/tutor/crates/rag-engine/vendor/pdfium/bin/pdfium.dll` (PDFium 156.0.8076), no linking |
| `vault-extract` with the `pdfium` feature | 112 crates in the dependency tree (98 without the feature) |

xberg's default PDF backend is pure Rust, so it needs no DLL. Its churn is high (1.2.6 to 1.3.0 in eight days),
and its licence history is mixed (see projects-sidecars.md).

## PDF results on the 15 PDFs

Each tool extracts text with no OCR. "xberg" is `extract` with Markdown output and page extraction.

| | PDFium `text.all()` | xberg | vault-extract (PDFium plus layout) |
|---|---|---|---|
| pages per second, all files | 1,731 | 79 | 360 |
| median per-file pages per second | 1,642 | 78 | |
| speed ratio per file | 10x to 33x faster than xberg | | |
| failures on the 15 | 0 | 0 | 0 |
| non-whitespace characters | 492,889 | 505,725 (includes Markdown marks) | |
| words | 114,326 | 117,945 | |
| empty pages | 1 | 1 (the same page) | |
| replacement characters | 0 | 0 | |
| private-use characters | 229 | 223 | |
| `(cid:` markers | 0 | 0 | |

**Completeness.** Both tools return the same text within about 2 percent. The vault-extract converter keeps
92 to 100 percent of PDFium's alphanumeric characters per document (median 99.4); the remainder is running
headers, footers and page numbers that it removes on purpose (checked on the lowest case, an 8-page handout,
where the missing words are exactly the repeated header and the copyright footer).

**Reading order.** I took segment geometry from PDFium to build a reference order (columns left then right
between full-width lines) and scored how often consecutive unique line-start anchors appear in that order in
each tool's page text. The reference is itself an approximation, so read the numbers as relative.

| | in order, all pairs | in order, two-column pages |
|---|---|---|
| PDFium stream order | 96.5% | 94.1% |
| xberg | 95.8% | 92.9% |

Synthetic two-column pages show the real difference. Stream order is column by column in one file and row by
row in another. PDFium returns the stream order unchanged (right when the file is column by column, interleaved
lines when it is row by row). xberg merges everything on the same row into one line, so the column-by-column
file comes out interleaved too. **Neither tool detects columns by default**; vault-extract does, from
character geometry (gutter detection, full-width lines flush the columns), and tests build both files and
check the order. Only two columns are tested; the gutter search is written for more.

**Maths.** Mathematical character counts are the same (8,833 against 9,085). Display equations are flattened by
both. On dense maths pages xberg's re-ordering splits paragraphs around display-equation fragments (a theorem
paragraph interleaved with its formulas), where PDFium keeps the paragraph whole. This is why maths-heavy PDFs
go to `waiting_gpu` (MinerU) per SPEC 8; the classifier's default threshold (3 percent symbol characters with at
least two maths fonts) flags two of the 15 (4.9 and 3.2 percent; the next is 1.7 percent). The native text of
those two is still returned as a fallback.

**Hyphenation.** PDFium's `text.all()` drops a line-end hyphen and glues the halves with no space (one
variable). The vault-extract layout reads the hyphen (PDFium marks it as U+0002) and joins it only when the next
line starts in lower case.

**Page labels.** On a synthetic six-page file with `/PageLabels` (roman, decimal, then decimal from 7 with the
prefix `A-`), PDFium gives `i, ii, 1, 2, A-7, A-8`, which is right. xberg gives the same list, but only in
`metadata.additional["page_labels"]` (its page objects carry only the physical number). When a PDF has no `/PageLabels`, vault-extract infers labels from printed numbers in
headers and footers and accepts them only where at least three pages agree on the offset (arabic and roman).

**Failures.** A 200 KB truncation of a real PDF: PDFium reports a format error (`corrupt_file`); xberg silently
returns the 8 pages it could parse among 26, 18 of them empty. Random bytes, and a bare `%PDF-1.4` header: both
report errors.

## EPUB, DOCX, ODT on synthetic fixtures

Fixtures written for this spike (headings under a title, bullet and numbered lists, a table with a header, a
footnote, display and inline Office Math, a quote, code; an EPUB 3 with a page list, an aside footnote, a table
and MathML; an ODT with a formula object). Both run in a few milliseconds.

| | xberg | vault-extract |
|---|---|---|
| DOCX inline maths | moved out of its sentence and set as a display block | stays in the sentence as `$x^2$` |
| DOCX footnotes | markers and definitions numbered inconsistently (`[^2]` in the text, `[^1]` in the notes) | `[^1]`, `[^2]` in order of use |
| DOCX heading levels under a Title | Title and Heading 1 both level 1 | Title level 1, headings shifted down |
| DOCX code paragraph | plain text | fenced |
| DOCX display maths | LaTeX | LaTeX |
| EPUB page list (`nav page-list`, `epub:type="pagebreak"`) | dropped | printed labels become page marks (`iv`, `1`, `2`) |
| EPUB MathML | reduced to its digits | `$\frac{1}{2}$` |
| EPUB book title and authors in the text | not in the text | title heading and author line; ISBN in the metadata |
| EPUB footnote | link text and a loose paragraph | `[^1]` and `[^1]: text` |
| ODT footnote and formula | duplicated marker; formula pulled out of its sentence | in place |

Neither tool had a failure. Our converters were written against these shapes, so this is a fixture check and
not a corpus measurement; run both on real books before trusting either on edge cases.

## What vault-extract's PDF path does and does not do

Does: columns, reading order, superscripts and subscripts as `^2` and `_i`, hyphen joins,
paragraphs by gap, indent and short last line, headings (numbered, font size, bold run-ins, standalone
`Abstract` and `References`), the title block (authors and date stay text), bullets, references split by label,
hanging indent or spacing, running header and footer removal, printed page numbers to labels, rotated pages,
rotated margin stamps (kept only for the arXiv id), metadata from the document properties plus DOI, arXiv id and
ISBN from the text, and routing signals (scanned page ratio, maths ratio, CJK ratio, book or paper).

Does not: detect tables (a table's rows read as text lines), recover LaTeX from display maths, use the PDF
outline for headings, OCR, or attach a reference label that is centred beside a multi-line entry (some entries
in label-in-margin bibliographies come out shifted by a line). GROBID will do the paper structure when it is
hosted; its TEI goes through `tei::convert` unchanged.

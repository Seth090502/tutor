// Fixtures for the Reading tests and the dev harness. All titles and text are invented for testing.
import type { Chunk, Doc, DocDetail, OutlineEntry, Prepared, SectionRef } from "./types"

const NOW = 1_790_000_000

export const BOOK: Doc = {
  doc_id: "d-analysis",
  path: "corpus/math/books/first-course-analysis.pdf",
  title: "A First Course in Real Analysis",
  domain: "math",
  source_type: "book",
  pages: 412,
  chunks: 1180,
  bytes: 9_400_000,
  added_at: NOW - 3 * 86_400,
  ocr_pending: 0,
}
export const PAPER: Doc = {
  doc_id: "d-paper",
  path: "corpus/science/papers/spaced-retrieval-classrooms.pdf",
  title: "Spaced Retrieval Practice in Secondary Classrooms: a Cluster Trial",
  domain: "science",
  source_type: "paper",
  pages: 14,
  chunks: 52,
  bytes: 880_000,
  added_at: NOW - 40 * 86_400,
  ocr_pending: 0,
}
export const DOCS: Doc[] = [
  BOOK,
  PAPER,
  {
    doc_id: "d-chem",
    path: "corpus/chemistry/books/equilibrium-notes.pdf",
    title: "Chemical Equilibrium, Worked Notes",
    domain: "chemistry",
    source_type: "note",
    pages: 88,
    chunks: 240,
    bytes: 2_100_000,
    added_at: NOW - 9 * 86_400,
    ocr_pending: 34,
  },
  {
    doc_id: "d-eng",
    path: "corpus/english/books/essay-craft.pdf",
    title: "The Craft of the Short Essay",
    domain: "english",
    source_type: "book",
    pages: 236,
    chunks: 690,
    bytes: 4_800_000,
    added_at: NOW - 70 * 86_400,
    ocr_pending: 0,
  },
  {
    doc_id: "d-inbox",
    path: "corpus/inbox/scan-0042.pdf",
    title: "scan-0042",
    domain: "unsorted",
    source_type: "unknown",
    pages: 19,
    chunks: 0,
    bytes: 6_200_000,
    added_at: NOW - 86_400,
    ocr_pending: 19,
  },
]

export const COUNTS = {
  documents: {
    total: DOCS.length,
    by_domain: { math: 1, science: 1, chemistry: 1, english: 1, unsorted: 1 },
    by_source_type: { book: 2, paper: 1, note: 1, unknown: 1 },
  },
}

export const OUTLINE: OutlineEntry[] = [
  { heading_path: ["1 The real numbers"], page_start: 9, page_end: 31, chunk_id: "d-analysis:9" },
  { heading_path: ["1 The real numbers", "1.1 Ordered fields"], page_start: 9, page_end: 17, chunk_id: "d-analysis:12" },
  { heading_path: ["1 The real numbers", "1.2 The completeness axiom"], page_start: 18, page_end: 31, chunk_id: "d-analysis:19" },
  { heading_path: ["2 Sequences"], page_start: 32, page_end: 70, chunk_id: "d-analysis:40" },
  { heading_path: ["2 Sequences", "2.1 Limits of sequences"], page_start: 32, page_end: 47, chunk_id: "d-analysis:41" },
  { heading_path: ["2 Sequences", "2.2 Monotone convergence"], page_start: 48, page_end: 59, chunk_id: "d-analysis:58" },
]
export const BOOK_DETAIL: DocDetail = { ...BOOK, outline: OUTLINE }

export const PAPER_OUTLINE: OutlineEntry[] = [
  { heading_path: ["Abstract"], page_start: 1, page_end: 1, chunk_id: "d-paper:0" },
  { heading_path: ["1 Introduction"], page_start: 1, page_end: 3, chunk_id: "d-paper:1" },
  { heading_path: ["2 Methods"], page_start: 3, page_end: 6, chunk_id: "d-paper:4" },
  { heading_path: ["3 Results"], page_start: 6, page_end: 10, chunk_id: "d-paper:9" },
  { heading_path: ["4 Discussion"], page_start: 10, page_end: 13, chunk_id: "d-paper:16" },
]
export const PAPER_DETAIL: DocDetail = { ...PAPER, outline: PAPER_OUTLINE }

const PATH = ["2 Sequences", "2.1 Limits of sequences"]
const chunk = (ordinal: number, page_start: number, page_end: number, text: string, heading_path = PATH): Chunk => ({
  chunk_id: `d-analysis:${ordinal}`,
  ordinal,
  text,
  page_start,
  page_end,
  heading_path,
})

/** Section 2.1 as the chunk route returns it: hard-wrapped lines, a hyphenated break, blank lines between paragraphs. */
export const SECTION_CHUNKS: Chunk[] = [
  chunk(
    41,
    32,
    32,
    `A sequence is a function whose domain is the natural numbers. We write a_n for its value at n and (a_n) for the\nwhole sequence. Informally, the sequence converges to L when its terms get as close to L as we please and stay there.\n\nThe informal phrase "as close as we please" hides a quantifier. To make it precise we let the challenger choose a\ntolerance eps > 0, and then we must be able to name a point N beyond which every term lies within eps of L.`,
  ),
  chunk(
    42,
    33,
    33,
    `Definition 2.1. The sequence (a_n) converges to L if for every eps > 0 there is an N such that |a_n - L| < eps for\nall n >= N. We then write a_n -> L. A sequence that does not converge to any real number is said to diverge.\n\nThe order of the quantifiers matters. N may depend on eps, and usually does: a smaller tolerance demands a later\nstarting point. Reversing the order would ask for one N that works for every eps at once, which only the constant\nsequences satisfy.`,
  ),
  chunk(
    43,
    34,
    35,
    `Example 2.2. Let a_n = 1/n. Given eps > 0, choose N with N > 1/eps; such an N exists by the Archimedean property. For\nn >= N we have |a_n - 0| = 1/n <= 1/N < eps, so a_n -> 0.\n\nNotice what the proof does not do. It does not list terms or observe a pattern. It starts from an arbitrary eps and\nproduces N; every convergence proof from the definition has this shape.`,
  ),
  chunk(
    44,
    36,
    36,
    `Theorem 2.3 (uniqueness of limits). If a_n -> L and a_n -> M, then L = M.\n\nProof. Suppose L and M differ and put eps = |L - M|/2. Choose N1 so that terms beyond N1 lie within eps of L, and N2\nso that terms beyond N2 lie within eps of M. For n beyond both, the triangle inequality gives |L - M| <= |L - a_n| +\n|a_n - M| < 2 eps = |L - M|, a contradiction.`,
  ),
  chunk(
    45,
    37,
    38,
    `A convergent sequence is bounded. Take eps = 1 in the definition: beyond some N every term lies within 1 of L, and\nthe finitely many earlier terms have a largest absolute value. Their maximum, together with |L| + 1, bounds the whole\nsequence. The converse fails: (-1)^n is bounded and diverges.`,
  ),
  chunk(
    46,
    39,
    39,
    `Limit laws. If a_n -> L and b_n -> M then a_n + b_n -> L + M and a_n b_n -> LM. The product law needs the bound from\nthe previous paragraph: write a_n b_n - LM = a_n(b_n - M) + M(a_n - L) and control each piece separately.`,
  ),
]

export const SECTION: SectionRef = {
  heading_path: PATH,
  chunk_ids: SECTION_CHUNKS.map((c) => c.chunk_id),
  page_start: 32,
  page_end: 47,
}

export const PREPARED: Prepared = {
  prequestions: [
    { id: "q1", q: "What does it mean, in your own words, for a sequence of real numbers to converge?" },
    { id: "q2", q: "Does the N in the definition of convergence depend on the tolerance eps? Why?" },
  ],
  idea_units: 6,
  atoms: [
    { id: "a1", front: "Definition of a_n -> L", back: "For every eps > 0 there is N with |a_n - L| < eps for all n >= N.", chunk_id: "d-analysis:42" },
    { id: "a2", front: "Is every convergent sequence bounded?", back: "Yes. Beyond some N the terms lie within 1 of L; the earlier finitely many are bounded too.", chunk_id: "d-analysis:45" },
    { id: "a3", front: "Is every bounded sequence convergent?", back: "No. (-1)^n is bounded and diverges.", chunk_id: "d-analysis:45" },
  ],
  warrants: [],
}

export const PAPER_CHUNKS: Chunk[] = [
  {
    chunk_id: "d-paper:0",
    ordinal: 0,
    text: `Background. Retrieval practice spaced over weeks improves retention in laboratory studies, but evidence from classrooms with whole-school scheduling is thin. Methods. We randomised 38 classes in 12 secondary schools to spaced quizzing or an equal amount of re-study, and tested at 4 and 12 weeks. Results. Spaced quizzing raised the 12-week test score by 0.27 standard deviations (95% CI 0.11 to 0.43). The benefit did not differ by subject. Conclusions. Spacing quizzes is a low-cost way to improve retention; effects at longer delays are unknown.`,
    page_start: 1,
    page_end: 1,
    heading_path: ["Abstract"],
  },
]


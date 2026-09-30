//! Token-sized chunks with page ranges and heading paths.
//!
//! A document's text is split by `text-splitter` (Markdown-aware for `.md`, semantic
//! paragraph/sentence levels otherwise) into chunks of at most [`CHUNK_TOKENS`] cl100k tokens.
//! Consecutive chunks overlap by up to [`OVERLAP_TOKENS`] tokens, taken in whole units
//! (sentences, words) of the level the text was split at; when a chunk ends at a paragraph or
//! page break whose last paragraph is longer than that, the next chunk starts fresh. Pages are
//! joined with a blank line, so a chunk may span a page break; its `page_start` and `page_end`
//! then record both pages.

use crate::headings::{self, Section};
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};
use tiktoken_rs::CoreBPE;

pub const CHUNK_TOKENS: usize = 400;
pub const OVERLAP_TOKENS: usize = 40;

/// What joins consecutive pages in a document's text.
pub const PAGE_SEPARATOR: &str = "\n\n";

/// A document's text ready for chunking.
#[derive(Debug, Default)]
pub struct DocText {
    pub text: String,
    /// `(byte offset, 1-based page number)` of every page with text, in order. Empty for
    /// documents without pages (EPUB, Markdown, plain text).
    pub pages: Vec<(usize, u32)>,
    pub sections: Vec<Section>,
    pub markdown: bool,
}

impl DocText {
    /// Join page texts (1-based page numbers) with [`PAGE_SEPARATOR`], skipping blank pages.
    pub fn from_pages<'a>(pages: impl IntoIterator<Item = (u32, &'a str)>) -> DocText {
        let mut doc = DocText::default();
        for (number, text) in pages {
            if text.trim().is_empty() {
                continue;
            }
            if !doc.text.is_empty() {
                doc.text.push_str(PAGE_SEPARATOR);
            }
            doc.pages.push((doc.text.len(), number));
            doc.text.push_str(text);
        }
        doc
    }

    /// Each page's text with its offset, for the PDF heading heuristic.
    pub fn page_slices(&self) -> Vec<(usize, &str)> {
        self.pages
            .iter()
            .enumerate()
            .map(|(i, &(start, _))| {
                let end = self.pages.get(i + 1).map_or(self.text.len(), |&(next, _)| next - PAGE_SEPARATOR.len());
                (start, &self.text[start..end])
            })
            .collect()
    }

    fn page_at(&self, offset: usize) -> Option<u32> {
        match self.pages.partition_point(|&(start, _)| start <= offset) {
            0 => self.pages.first().map(|&(_, n)| n),
            i => Some(self.pages[i - 1].1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub ordinal: u32,
    pub text: String,
    pub tokens: u32,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
    pub heading_path: Vec<String>,
}

pub fn tokenizer() -> &'static CoreBPE {
    tiktoken_rs::cl100k_base_singleton()
}

pub fn chunk(doc: &DocText) -> Vec<Chunk> {
    let bpe = tokenizer();
    let config = ChunkConfig::new(CHUNK_TOKENS)
        .with_sizer(bpe)
        .with_overlap(OVERLAP_TOKENS)
        .expect("overlap is smaller than the chunk size");
    let pieces: Vec<(usize, &str)> = if doc.markdown {
        MarkdownSplitter::new(config).chunk_indices(&doc.text).collect()
    } else {
        TextSplitter::new(config).chunk_indices(&doc.text).collect()
    };
    pieces
        .into_iter()
        .filter(|(_, text)| !text.trim().is_empty())
        .enumerate()
        .map(|(ordinal, (offset, text))| Chunk {
            ordinal: ordinal as u32,
            text: text.to_string(),
            tokens: bpe.encode_ordinary(text).len() as u32,
            page_start: doc.page_at(offset),
            page_end: doc.page_at(offset + text.len().saturating_sub(1)),
            heading_path: headings::path_at(&doc.sections, offset),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// About 60 distinct words per sentence group, so pages hold several hundred tokens.
    fn page_text(page: u32, sentences: usize) -> String {
        (0..sentences)
            .map(|s| format!("Page {page} sentence {s} explains how reaction rates depend on temperature and concentration."))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn from_pages_skips_blank_pages_and_records_offsets() {
        let doc = DocText::from_pages([(1, "alpha"), (2, "  \n "), (3, "beta")]);
        assert_eq!(doc.text, "alpha\n\nbeta");
        assert_eq!(doc.pages, vec![(0, 1), (7, 3)]);
        assert_eq!(doc.page_slices(), vec![(0, "alpha"), (7, "beta")]);
    }

    #[test]
    fn chunks_respect_the_token_limit_and_overlap() {
        let pages: Vec<String> = (1..=3).map(|p| page_text(p, 40)).collect();
        let doc = DocText::from_pages(pages.iter().enumerate().map(|(i, t)| (i as u32 + 1, t.as_str())));
        let chunks = chunk(&doc);
        assert!(chunks.len() >= 4, "{} chunks", chunks.len());
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.ordinal as usize, i);
            assert!(c.tokens as usize <= CHUNK_TOKENS, "chunk {i} has {} tokens", c.tokens);
            assert_eq!(c.tokens as usize, tokenizer().encode_ordinary(&c.text).len());
        }
        // Chunks split inside a page overlap: the next one starts with text the previous one ends with.
        let mut within_page = 0;
        for pair in chunks.windows(2).filter(|p| p[0].page_end == p[1].page_start) {
            let next_start: String = pair[1].text.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
            assert!(pair[0].text.contains(&next_start), "no overlap between\n{}\nand\n{}", pair[0].text, pair[1].text);
            within_page += 1;
        }
        assert!(within_page >= 2, "{within_page} in-page boundaries");
    }

    #[test]
    fn page_ranges_match_the_text_each_chunk_covers() {
        let pages: Vec<(u32, String)> = (10..=13).map(|p| (p, page_text(p, 12))).collect();
        let doc = DocText::from_pages(pages.iter().map(|(p, t)| (*p, t.as_str())));
        let chunks = chunk(&doc);
        let mut crossing = 0;
        for c in &chunks {
            let (start, end) = (c.page_start.unwrap(), c.page_end.unwrap());
            assert!(start <= end);
            // Every page number mentioned in the chunk lies in its range, and both ends appear.
            let mentioned: Vec<u32> = c
                .text
                .split("Page ")
                .skip(1)
                .map(|s| s.split_whitespace().next().unwrap().parse().unwrap())
                .collect();
            assert!(mentioned.iter().all(|p| (start..=end).contains(p)), "{c:?}");
            assert!(mentioned.contains(&start) && mentioned.contains(&end), "{c:?}");
            if start != end {
                crossing += 1;
            }
        }
        assert!(crossing > 0, "expected at least one chunk spanning a page break");
        assert_eq!(chunks.first().unwrap().page_start, Some(10));
        assert_eq!(chunks.last().unwrap().page_end, Some(13));
    }

    #[test]
    fn a_short_page_is_one_chunk_on_that_page() {
        let doc = DocText::from_pages([(1, ""), (2, "Oxidation is loss of electrons.")]);
        let chunks = chunk(&doc);
        assert_eq!(chunks.len(), 1);
        assert_eq!((chunks[0].page_start, chunks[0].page_end), (Some(2), Some(2)));
    }

    #[test]
    fn unpaged_documents_have_no_page_numbers() {
        let doc = DocText { text: "Just a note.".into(), ..Default::default() };
        let chunks = chunk(&doc);
        assert_eq!(chunks.len(), 1);
        assert_eq!((chunks[0].page_start, chunks[0].page_end), (None, None));
    }

    #[test]
    fn markdown_chunks_carry_the_heading_path_at_their_start() {
        let body = |topic: &str| {
            (0..30).map(|i| format!("Fact {i} about {topic} is stated in this sentence.")).collect::<Vec<_>>().join(" ")
        };
        let text = format!("# Chemistry\n\n## Redox\n\n{}\n\n## Acids\n\n{}\n", body("redox"), body("acids"));
        let doc = DocText { sections: headings::markdown(&text), text, markdown: true, ..Default::default() };
        let chunks = chunk(&doc);
        assert!(chunks.len() >= 2);
        for c in &chunks {
            if c.text.contains("about acids") && !c.text.contains("about redox") {
                assert_eq!(c.heading_path, vec!["Chemistry", "Acids"], "{c:?}");
            }
            if c.text.starts_with("# Chemistry") || c.text.contains("about redox") && !c.text.contains("Acids") {
                assert!(c.heading_path.starts_with(&["Chemistry".to_string()]), "{c:?}");
            }
        }
        assert_eq!(chunks[0].heading_path, vec!["Chemistry"]);
        assert!(chunks.iter().any(|c| c.heading_path == ["Chemistry", "Acids"]));
    }

    #[test]
    fn empty_text_gives_no_chunks() {
        assert!(chunk(&DocText::default()).is_empty());
        assert!(chunk(&DocText { text: " \n\n ".into(), ..Default::default() }).is_empty());
    }
}

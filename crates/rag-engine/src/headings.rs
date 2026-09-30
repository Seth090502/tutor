//! Heading paths for chunks: which headings are in force at each point of a document's text.
//!
//! - **Markdown**: ATX headings (`#` to `######`, up to three leading spaces), outside fenced
//!   code blocks. Setext (underlined) headings are not recognised.
//! - **PDF** (a heuristic; the text layer has no structure). A line is a heading when it is a
//!   candidate and the next non-blank line on the same page is body text: at least
//!   [`MIN_BODY_LINE_CHARS`] characters and not a candidate itself. Candidates are at most
//!   [`MAX_HEADING_CHARS`] characters and [`MAX_HEADING_WORDS`] words, contain a letter, and do
//!   not end in `.`, `,`, `;` or `:`. They are one of:
//!   - `Chapter|Part|Section|Appendix|Unit|Lesson|Lecture <number, roman numeral or letter> ...`
//!     at level 1;
//!   - numbered `3`, `3.2`, `3.2.1` (up to four parts, each up to three digits) followed by a
//!     word starting with a capital letter, at the level given by the number of parts;
//!   - title-cased lines: the first character is a capital letter and at least
//!     [`TITLE_CASE_SHARE_MIN`] of the words other than short function words (`of`, `and`,
//!     `the`, ...) start with a capital letter. Single-word run-in labels such as `Proof` or
//!     `Example` are excluded. They nest one level below the last numbered heading (level 1 if
//!     there is none).
//!
//!   Running headers and footers are ignored: a candidate that is the first or last non-blank
//!   line of its page, and whose text (without leading or trailing page numbers) is in that
//!   position on at least [`RUNNING_HEADER_MIN_PAGES`] pages.

use std::collections::HashMap;

pub const MIN_BODY_LINE_CHARS: usize = 40;
pub const MAX_HEADING_CHARS: usize = 80;
pub const MAX_HEADING_WORDS: usize = 12;
pub const TITLE_CASE_SHARE_MIN: f64 = 0.75;
pub const RUNNING_HEADER_MIN_PAGES: usize = 3;

const CHAPTER_WORDS: [&str; 7] = ["chapter", "part", "section", "appendix", "unit", "lesson", "lecture"];
const SMALL_WORDS: [&str; 22] = [
    "a", "an", "the", "and", "or", "nor", "but", "of", "in", "on", "at", "to", "for", "by", "with",
    "from", "as", "vs", "via", "into", "over", "per",
];
const RUN_IN_LABELS: [&str; 14] = [
    "proof", "solution", "solutions", "example", "remark", "note", "definition", "theorem", "lemma",
    "corollary", "exercise", "answer", "hint", "figure",
];

/// From `offset` (a byte offset into the document text) on, `path` is the heading path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub offset: usize,
    pub path: Vec<String>,
}

/// The heading path in force at `offset`: that of the last section starting at or before it.
pub fn path_at(sections: &[Section], offset: usize) -> Vec<String> {
    match sections.partition_point(|s| s.offset <= offset) {
        0 => Vec::new(),
        i => sections[i - 1].path.clone(),
    }
}

/// Nest `(offset, level, title)` headings, in document order, into sections with full paths.
fn nest(headings: impl IntoIterator<Item = (usize, usize, String)>) -> Vec<Section> {
    let mut stack: Vec<(usize, String)> = Vec::new();
    headings
        .into_iter()
        .map(|(offset, level, title)| {
            while stack.last().is_some_and(|(l, _)| *l >= level) {
                stack.pop();
            }
            stack.push((level, title));
            Section { offset, path: stack.iter().map(|(_, t)| t.clone()).collect() }
        })
        .collect()
}

/// Lines of `text` with the byte offset of each line's first character.
fn lines_with_offsets(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0;
    text.split_inclusive('\n').map(move |line| {
        let start = offset;
        offset += line.len();
        (start, line.trim_end_matches(['\n', '\r']))
    })
}

pub fn markdown(text: &str) -> Vec<Section> {
    let mut fence: Option<char> = None;
    let mut headings = Vec::new();
    for (offset, line) in lines_with_offsets(text) {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = line.trim_start_matches(' ');
        if indent <= 3 {
            let fence_char = body.chars().next().filter(|c| *c == '`' || *c == '~');
            if let Some(c) = fence_char
                && body.starts_with(&c.to_string().repeat(3))
            {
                match fence {
                    None => fence = Some(c),
                    Some(open) if open == c => fence = None,
                    Some(_) => {}
                }
                continue;
            }
        }
        if fence.is_some() || indent > 3 {
            continue;
        }
        let level = body.len() - body.trim_start_matches('#').len();
        let rest = &body[level..];
        if !(1..=6).contains(&level) || !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
            continue;
        }
        let mut title = rest.trim();
        // An optional closing sequence of '#' must be preceded by a space.
        let without_closing = title.trim_end_matches('#');
        if without_closing.len() < title.len() && (without_closing.is_empty() || without_closing.ends_with([' ', '\t'])) {
            title = without_closing.trim_end();
        }
        if !title.is_empty() {
            headings.push((offset, level, title.to_string()));
        }
    }
    nest(headings)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Candidate {
    /// `Chapter 3`, `Part II`, ...: level 1.
    Chapter,
    /// `3.2 Limits`: level = number of parts.
    Numbered(usize),
    TitleCase,
}

fn candidate(line: &str) -> Option<Candidate> {
    let chars = line.chars().count();
    if !(2..=MAX_HEADING_CHARS).contains(&chars)
        || line.ends_with(['.', ',', ';', ':'])
        || !line.chars().any(char::is_alphabetic)
    {
        return None;
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() > MAX_HEADING_WORDS {
        return None;
    }
    if words.len() >= 2 && CHAPTER_WORDS.iter().any(|w| words[0].eq_ignore_ascii_case(w)) {
        let label = words[1].trim_end_matches(['.', ':']);
        let is_number = !label.is_empty() && label.chars().all(|c| c.is_ascii_digit());
        let is_roman = !label.is_empty()
            && label.len() <= 7
            && label.chars().all(|c| "ivxlcdmIVXLCDM".contains(c));
        let is_letter = label.len() == 1 && label.chars().all(|c| c.is_ascii_uppercase());
        if is_number || is_roman || is_letter {
            return Some(Candidate::Chapter);
        }
    }
    if let Some(parts) = section_number(words[0]) {
        return words
            .get(1)
            .and_then(|w| w.chars().next())
            .filter(|c| c.is_uppercase())
            .map(|_| Candidate::Numbered(parts));
    }
    if !line.chars().next().is_some_and(char::is_uppercase) {
        return None;
    }
    if words.len() == 1 && RUN_IN_LABELS.iter().any(|l| words[0].eq_ignore_ascii_case(l)) {
        return None;
    }
    let significant: Vec<&&str> =
        words.iter().filter(|w| !SMALL_WORDS.iter().any(|s| w.eq_ignore_ascii_case(s))).collect();
    let capitalised = significant.iter().filter(|w| w.chars().next().is_some_and(char::is_uppercase)).count();
    (!significant.is_empty() && capitalised as f64 / significant.len() as f64 >= TITLE_CASE_SHARE_MIN)
        .then_some(Candidate::TitleCase)
}

/// Number of parts in a section number such as `3`, `3.2` or `3.2.1.` (1 to 4 parts of up to
/// three digits, optional trailing dot).
fn section_number(token: &str) -> Option<usize> {
    let token = token.strip_suffix('.').unwrap_or(token);
    let parts: Vec<&str> = token.split('.').collect();
    let valid = (1..=4).contains(&parts.len())
        && parts.iter().all(|p| (1..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit()));
    valid.then_some(parts.len())
}

/// Text of a line without leading or trailing page numbers, lowercased: how running headers
/// are compared across pages.
fn running_key(line: &str) -> String {
    let words: Vec<&str> = line.split_whitespace().collect();
    let is_page_number = |w: &&str| w.chars().all(|c| c.is_ascii_digit());
    let start = words.iter().position(|w| !is_page_number(w)).unwrap_or(words.len());
    let end = words.iter().rposition(|w| !is_page_number(w)).map_or(start, |i| i + 1);
    words[start..end].join(" ").to_lowercase()
}

/// `pages` holds, in order, the byte offset of each page's text within the document text and
/// the page's text.
pub fn pdf(pages: &[(usize, &str)]) -> Vec<Section> {
    let page_lines: Vec<Vec<(usize, &str)>> = pages
        .iter()
        .map(|(page_offset, text)| {
            lines_with_offsets(text)
                .filter_map(|(o, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty()).then(|| (page_offset + o + (line.len() - line.trim_start().len()), trimmed))
                })
                .collect()
        })
        .collect();

    let mut edge_counts: HashMap<String, usize> = HashMap::new();
    for lines in &page_lines {
        let mut edges: Vec<&str> = lines.first().into_iter().chain(lines.last()).map(|(_, l)| *l).collect();
        edges.dedup();
        for line in edges {
            if candidate(line).is_some() {
                *edge_counts.entry(running_key(line)).or_default() += 1;
            }
        }
    }

    let mut headings = Vec::new();
    let mut last_numbered = 0;
    for lines in &page_lines {
        for (i, &(offset, line)) in lines.iter().enumerate() {
            let Some(kind) = candidate(line) else { continue };
            let is_edge = i == 0 || i + 1 == lines.len();
            if is_edge && edge_counts.get(&running_key(line)).copied().unwrap_or(0) >= RUNNING_HEADER_MIN_PAGES {
                continue;
            }
            let followed_by_body = lines.get(i + 1).is_some_and(|(_, next)| {
                next.chars().count() >= MIN_BODY_LINE_CHARS && candidate(next).is_none()
            });
            if !followed_by_body {
                continue;
            }
            let level = match kind {
                Candidate::Chapter => 1,
                Candidate::Numbered(parts) => parts,
                Candidate::TitleCase => last_numbered + 1,
            };
            if !matches!(kind, Candidate::TitleCase) {
                last_numbered = level;
            }
            headings.push((offset, level, line.split_whitespace().collect::<Vec<_>>().join(" ")));
        }
    }
    nest(headings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "The limit of a function describes the value it approaches as the input approaches some point.";

    fn titles(sections: &[Section]) -> Vec<Vec<&str>> {
        sections.iter().map(|s| s.path.iter().map(String::as_str).collect()).collect()
    }

    #[test]
    fn path_at_uses_the_last_section_at_or_before_the_offset() {
        let sections = vec![
            Section { offset: 10, path: vec!["A".into()] },
            Section { offset: 20, path: vec!["A".into(), "B".into()] },
        ];
        assert!(path_at(&sections, 0).is_empty());
        assert_eq!(path_at(&sections, 10), vec!["A"]);
        assert_eq!(path_at(&sections, 19), vec!["A"]);
        assert_eq!(path_at(&sections, 20), vec!["A", "B"]);
        assert_eq!(path_at(&sections, 999), vec!["A", "B"]);
    }

    #[test]
    fn markdown_headings_nest_by_level() {
        let md = "# Chapter 3\n\nIntro.\n\n## 3.1 Sequences\n\nText.\n\n## 3.2 Limits ##\n\n### Epsilon\n\nMore.\n\n# Chapter 4\n";
        let sections = markdown(md);
        assert_eq!(
            titles(&sections),
            vec![
                vec!["Chapter 3"],
                vec!["Chapter 3", "3.1 Sequences"],
                vec!["Chapter 3", "3.2 Limits"],
                vec!["Chapter 3", "3.2 Limits", "Epsilon"],
                vec!["Chapter 4"],
            ]
        );
        assert_eq!(&md[sections[1].offset..sections[1].offset + 15], "## 3.1 Sequence");
    }

    #[test]
    fn markdown_ignores_code_fences_hashtags_and_deep_indents() {
        let md = "# Real\n\n```python\n# not a heading\n```\n\n#hashtag\n    # indented code\n~~~\n## also code\n~~~\n## Second\n####### seven\n";
        assert_eq!(titles(&markdown(md)), vec![vec!["Real"], vec!["Real", "Second"]]);
    }

    #[test]
    fn markdown_skips_empty_headings() {
        assert!(markdown("#\n##   \n").is_empty());
    }

    #[test]
    fn candidates_recognise_the_documented_shapes() {
        assert_eq!(candidate("Chapter 3"), Some(Candidate::Chapter));
        assert_eq!(candidate("CHAPTER IV The Mole"), Some(Candidate::Chapter));
        assert_eq!(candidate("Appendix B"), Some(Candidate::Chapter));
        assert_eq!(candidate("3 Limits"), Some(Candidate::Numbered(1)));
        assert_eq!(candidate("3.2 Limits and Continuity"), Some(Candidate::Numbered(2)));
        assert_eq!(candidate("3.2.1. One-Sided Limits"), Some(Candidate::Numbered(3)));
        assert_eq!(candidate("Newton's Laws of Motion"), Some(Candidate::TitleCase));
        assert_eq!(candidate("INTRODUCTION"), Some(Candidate::TitleCase));
        assert_eq!(candidate("Summary"), Some(Candidate::TitleCase));
    }

    #[test]
    fn candidates_reject_sentences_equations_and_labels() {
        assert_eq!(candidate("The limit of a function."), None);
        assert_eq!(candidate("the limit of a function"), None);
        assert_eq!(candidate("2 x + 3 = 7"), None);
        assert_eq!(candidate("1999 Annual Report"), None);
        assert_eq!(candidate("3.2 limits are defined"), None);
        assert_eq!(candidate("Proof"), None);
        assert_eq!(candidate("Example"), None);
        assert_eq!(candidate("E = mc"), None);
        assert_eq!(candidate("Chapter three begins here with a long sentence that goes on and on and on and on and on"), None);
        assert_eq!(candidate("42"), None);
    }

    #[test]
    fn pdf_headings_need_body_text_after_them() {
        let page = format!("Chapter 3 Limits\n{BODY}\n3.2 Epsilon and Delta\n{BODY}\nA Short Title\nshort line\n");
        let sections = pdf(&[(0, page.as_str())]);
        assert_eq!(titles(&sections), vec![vec!["Chapter 3 Limits"], vec!["Chapter 3 Limits", "3.2 Epsilon and Delta"]]);
        assert_eq!(sections[1].offset, page.find("3.2 Epsilon").unwrap());
    }

    #[test]
    fn pdf_title_case_headings_nest_under_the_last_numbered_one() {
        let page = format!("2.1 Moles\n{BODY}\nAvogadro's Number\n{BODY}\n2.2 Molar Mass\n{BODY}\n");
        assert_eq!(
            titles(&pdf(&[(0, page.as_str())])),
            vec![vec!["2.1 Moles"], vec!["2.1 Moles", "Avogadro's Number"], vec!["2.2 Molar Mass"]]
        );
    }

    #[test]
    fn pdf_offsets_are_document_offsets() {
        let p1 = format!("{BODY}\n");
        let p2 = format!("  4.1 Vectors\n{BODY}\n");
        let doc = format!("{p1}\n\n{p2}");
        let p2_offset = p1.len() + 2;
        let sections = pdf(&[(0, p1.as_str()), (p2_offset, p2.as_str())]);
        assert_eq!(titles(&sections), vec![vec!["4.1 Vectors"]]);
        assert_eq!(&doc[sections[0].offset..sections[0].offset + 11], "4.1 Vectors");
    }

    #[test]
    fn pdf_running_headers_are_ignored_but_a_single_top_heading_is_kept() {
        let mut pages = Vec::new();
        for n in 1..=4 {
            let top = if n == 2 { "Chapter 2 Stoichiometry".to_string() } else { format!("{n} Calculus Early Transcendentals") };
            pages.push(format!("{top}\n{BODY}\n{BODY}\n"));
        }
        let mut offset = 0;
        let refs: Vec<(usize, &str)> = pages
            .iter()
            .map(|p| {
                let entry = (offset, p.as_str());
                offset += p.len() + 2;
                entry
            })
            .collect();
        // Pages 1, 3 and 4 share the running header (with page numbers); page 2 opens a chapter.
        assert_eq!(titles(&pdf(&refs)), vec![vec!["Chapter 2 Stoichiometry"]]);
    }

    #[test]
    fn running_key_drops_page_numbers() {
        assert_eq!(running_key("12 Calculus Early Transcendentals"), "calculus early transcendentals");
        assert_eq!(running_key("3.2 Limits 47"), "3.2 limits");
        assert_eq!(running_key("47"), "");
    }
}

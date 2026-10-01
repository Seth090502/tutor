//! Markdown writer that tracks line numbers for headings, pages and verbatim blocks as it goes.

use crate::types::{BlockKind, BlockSpan, HeadingMark, PageMark};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Last {
    Start,
    Para,
    Heading,
    List,
    Block,
}

/// Collects canonical-ready Markdown: one blank line between blocks, no blank lines inside tables, code, display
/// maths or tight lists, and the structural marks pointing at the right lines.
#[derive(Default)]
pub(crate) struct MdBuilder {
    lines: Vec<String>,
    last: Option<Last>,
    pub headings: Vec<HeadingMark>,
    pub pages: Vec<PageMark>,
    pub blocks: Vec<BlockSpan>,
    pending_pages: Vec<(Option<String>, u32)>,
    page_index: u32,
}

impl MdBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn last(&self) -> Last {
        self.last.unwrap_or(Last::Start)
    }

    /// Add the separating blank line (unless this continues a tight list), then flush pending page marks onto the
    /// line about to be written.
    fn begin(&mut self, continues_list: bool) {
        let needs_blank = match self.last() {
            Last::Start => false,
            Last::List => !continues_list,
            _ => true,
        };
        if needs_blank {
            self.lines.push(String::new());
        }
        let line = self.lines.len() as u32 + 1;
        for (label, index) in std::mem::take(&mut self.pending_pages) {
            self.pages.push(PageMark { label, index, line });
        }
    }

    pub fn heading(&mut self, level: u8, text: &str) {
        let text = collapse_ws(text);
        if text.is_empty() {
            return;
        }
        let level = level.clamp(1, 6);
        self.begin(false);
        let line = self.lines.len() as u32 + 1;
        self.lines.push(format!("{} {}", "#".repeat(level as usize), text));
        self.headings.push(HeadingMark { level, text, line });
        self.last = Some(Last::Heading);
    }

    /// A paragraph on one line. Whitespace is collapsed; a leading block marker is escaped.
    pub fn paragraph(&mut self, text: &str) {
        let text = collapse_ws(text);
        if text.is_empty() {
            return;
        }
        self.begin(false);
        self.lines.push(escape_block_start(&text));
        self.last = Some(Last::Para);
    }

    pub fn quote(&mut self, text: &str) {
        let text = collapse_ws(text);
        if text.is_empty() {
            return;
        }
        self.begin(false);
        self.lines.push(format!("> {text}"));
        self.last = Some(Last::Para);
    }

    /// One list item. `marker` is `-` or `1.`; nested items indent by two spaces per `depth` level.
    pub fn list_item(&mut self, depth: usize, marker: &str, text: &str) {
        let text = collapse_ws(text);
        if text.is_empty() {
            return;
        }
        self.begin(true);
        self.lines.push(format!("{}{} {}", "  ".repeat(depth), marker, text));
        self.last = Some(Last::List);
    }

    /// End the current list so the next item starts a new one (a blank line separates the two lists).
    pub fn break_list(&mut self) {
        if self.last == Some(Last::List) {
            self.last = Some(Last::Para);
        }
    }

    /// Verbatim lines (no escaping or collapsing), recorded as a block of `kind`.
    pub fn block(&mut self, kind: BlockKind, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }
        self.begin(false);
        let start = self.lines.len() as u32 + 1;
        let n = lines.len() as u32;
        self.lines.extend(lines);
        self.blocks.push(BlockSpan { kind, start, end: start + n - 1 });
        self.last = Some(Last::Block);
    }

    /// Fenced code. The fence is longer than any backtick run in the code.
    pub fn code_block(&mut self, lang: &str, code: &str) {
        let lines = fenced(lang, code);
        if lines.is_empty() {
            return;
        }
        self.block(BlockKind::Code, lines);
    }

    /// Display maths: `$$`, the LaTeX, `$$`.
    pub fn display_math(&mut self, latex: &str) {
        let body: Vec<String> = latex
            .replace("$$", "")
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        if body.is_empty() {
            return;
        }
        let mut lines = vec!["$$".to_string()];
        lines.extend(body);
        lines.push("$$".to_string());
        self.block(BlockKind::Equation, lines);
    }

    /// A table of already-inlined cells. The first `header_rows` rows are headers; GFM needs exactly one header
    /// row, so when there are several they are joined cell by cell with a space, and when there is none the first
    /// row is promoted.
    pub fn pipe_table(&mut self, rows: &[Vec<String>], header_rows: usize) {
        let Some(width) = rows.iter().map(Vec::len).max() else { return };
        if width == 0 {
            return;
        }
        let render = |r: &[String]| {
            let mut cells: Vec<String> = r.iter().map(|c| escape_cell(c)).collect();
            cells.resize(width, String::new());
            format!("| {} |", cells.join(" | "))
        };
        let header_rows = header_rows.clamp(1, rows.len());
        let mut header = vec![String::new(); width];
        for r in &rows[..header_rows] {
            for (i, c) in r.iter().enumerate() {
                if !c.trim().is_empty() {
                    if !header[i].is_empty() {
                        header[i].push(' ');
                    }
                    header[i].push_str(c.trim());
                }
            }
        }
        let mut lines = vec![render(&header), format!("|{}", " --- |".repeat(width))];
        lines.extend(rows[header_rows..].iter().map(|r| render(r)));
        self.block(BlockKind::Table, lines);
    }

    /// A table that cannot be a pipe table, kept as an HTML fence (one source row per line).
    pub fn html_table(&mut self, html_lines: Vec<String>) {
        let mut lines = vec!["```html".to_string()];
        lines.extend(html_lines);
        lines.push("```".to_string());
        self.block(BlockKind::Table, lines);
    }

    /// Start a new page at the next written line. The physical index counts up from 1.
    pub fn page_break(&mut self, label: Option<String>) {
        self.page_index += 1;
        let idx = self.page_index;
        self.pending_pages.push((label.filter(|l| !l.is_empty()), idx));
    }

    /// Start a page whose physical index is known.
    pub fn page_break_at(&mut self, label: Option<String>, index: u32) {
        self.page_index = index;
        self.pending_pages.push((label.filter(|l| !l.is_empty()), index));
    }

    pub fn finish(mut self) -> FinishedMarkdown {
        // A page mark with no content after it points at the last line.
        let line = self.lines.len().max(1) as u32;
        for (label, index) in std::mem::take(&mut self.pending_pages) {
            self.pages.push(PageMark { label, index, line });
        }
        let mut markdown = self.lines.join("\n");
        if !markdown.is_empty() {
            markdown.push('\n');
        }
        FinishedMarkdown { markdown, headings: self.headings, pages: self.pages, blocks: self.blocks }
    }
}

pub(crate) struct FinishedMarkdown {
    pub markdown: String,
    pub headings: Vec<HeadingMark>,
    pub pages: Vec<PageMark>,
    pub blocks: Vec<BlockSpan>,
}

/// The lines of a fenced code block, empty when the code is blank. The fence is longer than any backtick run in
/// the code, and the language tag is reduced to characters that cannot break the fence line.
pub(crate) fn fenced(lang: &str, code: &str) -> Vec<String> {
    let code = code.trim_matches(['\n', '\r']);
    if code.trim().is_empty() {
        return Vec::new();
    }
    let fence = "`".repeat((longest_run(code, '`') + 1).max(3));
    let lang: String = lang
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '+' | '-' | '_' | '#' | '.'))
        .collect();
    let mut lines = vec![format!("{fence}{lang}")];
    lines.extend(code.lines().map(|l| l.trim_end_matches('\r').to_string()));
    lines.push(fence);
    lines
}

fn longest_run(s: &str, ch: char) -> usize {
    let mut best = 0;
    let mut cur = 0;
    for c in s.chars() {
        if c == ch {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 0;
        }
    }
    best
}

/// Collapse every run of whitespace (including newlines, non-breaking spaces and zero-width spaces) to one space
/// and trim both ends.
pub(crate) fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut gap = false;
    for c in s.chars() {
        if c.is_whitespace() || c == '\u{200B}' || c == '\u{FEFF}' {
            gap = true;
        } else {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.push(c);
        }
    }
    out
}

pub(crate) fn escape_cell(c: &str) -> String {
    collapse_ws(c).replace('|', "\\|")
}

/// Prefix a backslash when a paragraph would otherwise read as a heading, list item, quote, fence, table or rule.
pub(crate) fn escape_block_start(line: &str) -> String {
    let t = line;
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let starts_ordered = digits > 0
        && digits <= 9
        && matches!(t[digits..].chars().next(), Some('.') | Some(')'))
        && t[digits + 1..].starts_with(' ');
    let risky = t.starts_with('#')
        || t.starts_with('>')
        || t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
        || t == "-"
        || t.starts_with("```")
        || t.starts_with("~~~")
        || t.starts_with('|')
        || t.starts_with("$$")
        || t.starts_with("---")
        || t.starts_with("***")
        || t.starts_with("===")
        || t.starts_with("<!--")
        || starts_ordered;
    if risky { format!("\\{t}") } else { t.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_ws_trims_and_joins() {
        assert_eq!(collapse_ws("  a \n\t b\u{a0}c  "), "a b c");
        assert_eq!(collapse_ws(" \n "), "");
    }

    #[test]
    fn tracks_headings_blocks_and_pages() {
        let mut b = MdBuilder::new();
        b.heading(1, "Title");
        b.page_break(Some("iv".into()));
        b.paragraph("Hello  world");
        b.display_math("x^2\n\n+y");
        b.pipe_table(&[vec!["a".into(), "b|c".into()], vec!["1".into(), "2".into()]], 1);
        b.code_block("rust", "fn main() {}\n");
        let f = b.finish();
        assert_eq!(
            f.markdown,
            "# Title\n\nHello world\n\n$$\nx^2\n+y\n$$\n\n| a | b\\|c |\n| --- | --- |\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n"
        );
        assert_eq!(f.headings, vec![HeadingMark { level: 1, text: "Title".into(), line: 1 }]);
        assert_eq!(f.pages, vec![PageMark { label: Some("iv".into()), index: 1, line: 3 }]);
        let kinds: Vec<_> = f.blocks.iter().map(|b| (b.kind, b.start, b.end)).collect();
        assert_eq!(kinds, vec![(BlockKind::Equation, 5, 8), (BlockKind::Table, 10, 12), (BlockKind::Code, 14, 16)]);
        let lines: Vec<&str> = f.markdown.lines().collect();
        assert_eq!(lines[0], "# Title");
        assert_eq!(lines[4], "$$");
        assert_eq!(lines[9], "| a | b\\|c |");
    }

    #[test]
    fn tight_lists_have_no_blank_lines_between_items() {
        let mut b = MdBuilder::new();
        b.paragraph("Intro");
        b.list_item(0, "-", "one");
        b.list_item(1, "-", "nested");
        b.list_item(0, "-", "two");
        b.paragraph("After");
        assert_eq!(b.finish().markdown, "Intro\n\n- one\n  - nested\n- two\n\nAfter\n");
    }

    #[test]
    fn code_fence_grows_past_inner_backticks() {
        let mut b = MdBuilder::new();
        b.code_block("md", "```\ninner\n```");
        let md = b.finish().markdown;
        assert!(md.starts_with("````md\n") && md.ends_with("\n````\n"), "{md}");
    }

    #[test]
    fn escapes_block_markers_at_paragraph_start() {
        assert_eq!(escape_block_start("# not a heading"), "\\# not a heading");
        assert_eq!(escape_block_start("1. not a list"), "\\1. not a list");
        assert_eq!(escape_block_start("1.5 is fine"), "1.5 is fine");
        assert_eq!(escape_block_start("plain"), "plain");
    }

    #[test]
    fn multi_row_headers_are_merged() {
        let mut b = MdBuilder::new();
        b.pipe_table(
            &[vec!["Group".into(), "".into()], vec!["".into(), "Mean".into()], vec!["x".into(), "1".into()]],
            2,
        );
        assert_eq!(b.finish().markdown, "| Group | Mean |\n| --- | --- |\n| x | 1 |\n");
    }

    #[test]
    fn trailing_page_mark_points_at_last_line() {
        let mut b = MdBuilder::new();
        b.paragraph("only");
        b.page_break(None);
        let f = b.finish();
        assert_eq!(f.pages[0].line, 1);
    }
}

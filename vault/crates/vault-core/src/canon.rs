//! Canonical text (SPEC section 3).
//!
//! **Prose:** decode, NFC, unify line breaks, remove invisible and control characters, then work line by line:
//! verbatim blocks (fenced code, display maths, tables, HTML, headings, rules, reference-section entries) pass
//! through unchanged; every other paragraph is joined and re-split one sentence per line (UAX #29 plus the
//! abbreviation, maths, code-span and CJK rules). List items and block quotes keep their prefixes, so the
//! Markdown renders as before.
//!
//! **Code:** decode, CRLF and CR to LF, leading BOM removed. Nothing else, so line numbers equal the original's.
//!
//! Idempotence is designed in: a paragraph line break is only ever placed at a single ASCII space between two
//! non-space characters (or right after a CJK full stop), and never where the next line would parse as another
//! block. Re-reading the output therefore rebuilds the same paragraph text and takes the same decisions.

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use vault_api::Heading;

/// Prose lines longer than this (in characters) are split at the last space that fits.
pub const MAX_LINE_CHARS: usize = 4000;

/// Text decoded from bytes, with the encoding that was used (recorded in the sidecar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    /// Lowercase WHATWG label: `utf-8`, `utf-16le`, `windows-1252`, ...
    pub encoding: String,
    /// True when some bytes were invalid in that encoding and became U+FFFD.
    pub had_errors: bool,
}

/// A verbatim block's kind and line span in the canonical text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSpan {
    /// `code`, `math`, `table`, `html` or `references`.
    pub kind: String,
    pub start: u32,
    pub end: u32,
}

/// Canonical prose with the structure found while normalising it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonText {
    pub text: String,
    pub lines: u32,
    /// One entry per form feed: the line where the next page starts.
    pub page_breaks: Vec<u32>,
    /// Top-level ATX headings with their section spans.
    pub headings: Vec<Heading>,
    pub blocks: Vec<BlockSpan>,
}

// ------------------------------------------------------------------------------------------------- decoding

/// Step 1: BOM (UTF-8, UTF-16, UTF-32), then BOM-less UTF-16, then UTF-8, then `chardetng`.
pub fn decode(bytes: &[u8]) -> Decoded {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return with_encoding(encoding_rs::UTF_8, rest);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE, 0, 0]) {
        return decode_utf32(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0, 0, 0xFE, 0xFF]) {
        return decode_utf32(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return with_encoding(encoding_rs::UTF_16LE, rest);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return with_encoding(encoding_rs::UTF_16BE, rest);
    }
    if let Some(enc) = sniff_utf16(bytes) {
        return with_encoding(enc, bytes);
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Decoded { text: s.to_string(), encoding: "utf-8".into(), had_errors: false };
    }
    let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    with_encoding(detector.guess(None, Utf8Detection::Deny), bytes)
}

fn with_encoding(enc: &'static Encoding, bytes: &[u8]) -> Decoded {
    let (text, had_errors) = enc.decode_without_bom_handling(bytes);
    Decoded { text: text.into_owned(), encoding: enc.name().to_ascii_lowercase(), had_errors }
}

fn decode_utf32(bytes: &[u8], big_endian: bool) -> Decoded {
    let mut text = String::with_capacity(bytes.len() / 4);
    let partial = !bytes.len().is_multiple_of(4);
    let mut had_errors = partial;
    for chunk in bytes.as_chunks::<4>().0 {
        let v = if big_endian { u32::from_be_bytes(*chunk) } else { u32::from_le_bytes(*chunk) };
        match char::from_u32(v) {
            Some(c) => text.push(c),
            None => {
                text.push('\u{FFFD}');
                had_errors = true;
            }
        }
    }
    if partial {
        text.push('\u{FFFD}');
    }
    Decoded { text, encoding: if big_endian { "utf-32be" } else { "utf-32le" }.into(), had_errors }
}

/// BOM-less UTF-16: ASCII-range text leaves a zero byte in every other position.
fn sniff_utf16(bytes: &[u8]) -> Option<&'static Encoding> {
    let sample = &bytes[..bytes.len().min(4096) & !1];
    let pairs = sample.len() / 2;
    if pairs < 2 {
        return None;
    }
    let (mut even_zero, mut odd_zero) = (0usize, 0usize);
    for p in sample.as_chunks::<2>().0 {
        even_zero += usize::from(p[0] == 0);
        odd_zero += usize::from(p[1] == 0);
    }
    if odd_zero * 10 >= pairs * 3 && even_zero * 20 <= pairs {
        Some(encoding_rs::UTF_16LE)
    } else if even_zero * 10 >= pairs * 3 && odd_zero * 20 <= pairs {
        Some(encoding_rs::UTF_16BE)
    } else {
        None
    }
}

// ----------------------------------------------------------------------------------------------------- code

/// Canonical code: CRLF and CR become LF and a leading BOM is removed; nothing else changes.
pub fn normalize_code(text: &str) -> String {
    let text = text.trim_start_matches('\u{FEFF}');
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

/// Decode and canonicalise code bytes.
pub fn canonical_code(bytes: &[u8]) -> (Decoded, String) {
    let decoded = decode(bytes);
    let text = normalize_code(&decoded.text);
    (decoded, text)
}

/// Decode and canonicalise prose bytes.
pub fn canonical_prose(bytes: &[u8]) -> (Decoded, CanonText) {
    let decoded = decode(bytes);
    let canon = normalize_prose(&decoded.text);
    (decoded, canon)
}

/// Lines as the vault counts them: split on LF; a final LF does not start another line.
pub fn split_lines(text: &str) -> Vec<&str> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() { Vec::new() } else { body.split('\n').collect() }
}

// ---------------------------------------------------------------------------------------- char-level steps

struct RawLine {
    text: String,
    /// Form feeds immediately before this line.
    breaks: u32,
}

/// Steps 2-4: NFC, line breaks, removals. NFC runs again after removals, because removing a character can
/// bring a base character and a combining mark together.
fn char_level(input: &str) -> Vec<RawLine> {
    let nfc: String = input.nfc().collect();
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut breaks = 0u32;
    let mut removed = false;
    let mut chars = nfc.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(RawLine { text: std::mem::take(&mut cur), breaks: std::mem::take(&mut breaks) });
            }
            '\n' | '\u{0B}' | '\u{85}' | '\u{2028}' | '\u{2029}' => {
                lines.push(RawLine { text: std::mem::take(&mut cur), breaks: std::mem::take(&mut breaks) });
            }
            '\u{0C}' => {
                lines.push(RawLine { text: std::mem::take(&mut cur), breaks: std::mem::take(&mut breaks) });
                breaks = 1;
            }
            '\u{FEFF}' | '\u{200B}' | '\u{AD}' => removed = true,
            c if (c < ' ' && c != '\t') || ('\u{80}'..='\u{9F}').contains(&c) => removed = true,
            c => cur.push(c),
        }
    }
    if !cur.is_empty() || breaks > 0 {
        lines.push(RawLine { text: cur, breaks });
    }
    if removed {
        for line in &mut lines {
            if !line.text.is_ascii() {
                line.text = line.text.nfc().collect();
            }
        }
    }
    lines
}

// ------------------------------------------------------------------------------------------ line grammar

fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// A line split into its container prefix (indentation, quote markers, list markers) and its content.
struct Peeled<'a> {
    /// The prefix as it is written on the first line (tabs expanded, other whitespace as spaces).
    prefix: String,
    /// The prefix for continuation lines: quote markers kept, list markers replaced by spaces.
    cont: String,
    depth: usize,
    has_marker: bool,
    /// Columns of indentation before the first marker.
    indent: usize,
    content: &'a str,
}

fn peel(line: &str) -> Peeled<'_> {
    let mut prefix = String::new();
    let mut cont = String::new();
    let (mut depth, mut col) = (0usize, 0usize);
    let mut has_marker = false;
    let mut indent = None;
    let mut rest = line;
    loop {
        let mut consumed = 0;
        for (i, c) in rest.char_indices() {
            if c == '\t' {
                let w = 4 - col % 4;
                for _ in 0..w {
                    prefix.push(' ');
                    cont.push(' ');
                }
                col += w;
            } else if c.is_whitespace() {
                prefix.push(' ');
                cont.push(' ');
                col += 1;
            } else {
                break;
            }
            consumed = i + c.len_utf8();
        }
        rest = &rest[consumed..];
        indent.get_or_insert(col);
        if let Some(r) = rest.strip_prefix('>') {
            prefix.push('>');
            cont.push('>');
            col += 1;
            depth += 1;
            rest = r;
            if let Some(r) = rest.strip_prefix(' ') {
                prefix.push(' ');
                cont.push(' ');
                col += 1;
                rest = r;
            }
            continue;
        }
        if !is_rule_line(rest)
            && let Some(n) = list_marker_len(rest)
        {
            prefix.push_str(&rest[..n]);
            cont.extend(std::iter::repeat_n(' ', n));
            col += n;
            has_marker = true;
            rest = &rest[n..];
            continue;
        }
        break;
    }
    Peeled { prefix, cont, depth, has_marker, indent: indent.unwrap_or(0), content: rest }
}

/// `-`, `+`, `*`, or 1-9 digits and `.` or `)`, followed by a space, a tab or the end of the line.
fn list_marker_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let n = match b.first()? {
        b'-' | b'+' | b'*' => 1,
        b'0'..=b'9' => {
            let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
            if digits > 9 || !matches!(b.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    match b.get(n) {
        None | Some(b' ' | b'\t') => Some(n),
        _ => None,
    }
}

/// Thematic breaks (`***`, `- - -`, `___`) and setext underlines (`===`, `---`).
fn is_rule_line(s: &str) -> bool {
    let t = s.trim_end();
    if t.is_empty() {
        return false;
    }
    if t.chars().all(|c| c == '=') || t.chars().all(|c| c == '-') {
        return true;
    }
    let mut marker = None;
    let mut count = 0;
    for c in t.chars() {
        match c {
            '-' | '*' | '_' => {
                if marker.is_some_and(|m| m != c) {
                    return false;
                }
                marker = Some(c);
                count += 1;
            }
            ' ' | '\t' => {}
            _ => return false,
        }
    }
    count >= 3
}

fn fence_open(content: &str) -> Option<(char, usize)> {
    let ch = content.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let n = content.chars().take_while(|&c| c == ch).count();
    if n < 3 || (ch == '`' && content[n..].contains('`')) {
        return None;
    }
    Some((ch, n))
}

fn fence_closes(content: &str, ch: char, n: usize) -> bool {
    let t = content.trim_end();
    t.chars().count() >= n && t.chars().all(|c| c == ch)
}

fn atx_heading(content: &str) -> Option<(u8, String)> {
    let hashes = content.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &content[hashes..];
    if !(rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    let text = rest.trim();
    let without_closing = text.trim_end_matches('#');
    let text = if without_closing.is_empty() {
        ""
    } else if without_closing.ends_with([' ', '\t']) {
        without_closing.trim_end()
    } else {
        text
    };
    u8::try_from(hashes).ok().map(|level| (level, text.to_string()))
}

fn html_start(content: &str) -> bool {
    let mut chars = content.chars();
    chars.next() == Some('<') && chars.next().is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?'))
}

fn is_block_start(content: &str) -> bool {
    fence_open(content).is_some()
        || content.starts_with("$$")
        || content.starts_with('|')
        || html_start(content)
        || atx_heading(content).is_some()
        || is_rule_line(content)
}

fn continues_paragraph(line: &str, depth: usize) -> bool {
    if is_blank(line) {
        return false;
    }
    let p = peel(line);
    p.depth == depth && !p.has_marker && !p.content.trim().is_empty() && !is_block_start(p.content)
}

/// `References`, `Bibliography` or `Index`, optionally numbered (`7.`, `VII.`), emphasised or with a colon.
fn is_reference_heading(text: &str) -> bool {
    let mut t = text.trim().trim_matches(['*', '_']).trim();
    if let Some((first, rest)) = t.split_once(char::is_whitespace) {
        let f = first.trim_end_matches('.');
        let numbered = !f.is_empty()
            && (f.chars().all(|c| c.is_ascii_digit() || c == '.') || f.chars().all(|c| "IVXLCivxlc".contains(c)));
        if numbered {
            t = rest.trim();
        }
    }
    let t = t.trim_end_matches([':', '.']).trim().to_lowercase();
    matches!(t.as_str(), "references" | "bibliography" | "index")
}

/// Last line of an HTML block that starts at `i`.
fn html_end(lines: &[RawLine], i: usize, content: &str) -> usize {
    let lower = content.to_ascii_lowercase();
    let raw_tag = ["pre", "script", "style", "textarea"].into_iter().find(|t| {
        lower
            .strip_prefix('<')
            .and_then(|r| r.strip_prefix(t))
            .is_some_and(|r| r.is_empty() || r.starts_with(|c: char| c.is_whitespace() || c == '>'))
    });
    let ender: Option<String> = if lower.starts_with("<!--") {
        Some("-->".into())
    } else if lower.starts_with("<?") {
        Some("?>".into())
    } else if lower.starts_with("<![cdata[") {
        Some("]]>".into())
    } else if let Some(tag) = raw_tag {
        Some(format!("</{tag}>"))
    } else if lower.starts_with("<!") {
        Some(">".into())
    } else {
        None
    };
    let last = lines.len() - 1;
    match ender {
        Some(e) => (i..lines.len()).find(|&j| lines[j].text.to_ascii_lowercase().contains(&e)).unwrap_or(last),
        None => (i + 1..lines.len()).find(|&j| is_blank(&lines[j].text)).map_or(last, |j| j - 1),
    }
}

// --------------------------------------------------------------------------------------------- emitting

#[derive(Default)]
struct Emitter {
    out: Vec<String>,
    page_breaks: Vec<u32>,
    pending_breaks: u32,
    need_blank: bool,
    blocks: Vec<BlockSpan>,
    headings: Vec<(u8, String, u32)>,
}

impl Emitter {
    fn blank(&mut self, breaks: u32) {
        self.pending_breaks += breaks;
        if !self.out.is_empty() {
            self.need_blank = true;
        }
    }

    fn line(&mut self, text: String, breaks: u32) -> u32 {
        if std::mem::take(&mut self.need_blank) {
            self.out.push(String::new());
        }
        self.out.push(text);
        let n = line_no(self.out.len());
        for _ in 0..self.pending_breaks + breaks {
            self.page_breaks.push(n);
        }
        self.pending_breaks = 0;
        n
    }

    fn verbatim(&mut self, lines: &[RawLine], from: usize, to: usize, kind: &str) -> usize {
        let mut start = 0;
        for (k, raw) in lines[from..=to].iter().enumerate() {
            let n = self.line(raw.text.clone(), raw.breaks);
            if k == 0 {
                start = n;
            }
        }
        let end = line_no(self.out.len());
        self.blocks.push(BlockSpan { kind: kind.to_string(), start, end });
        to + 1
    }
}

fn line_no(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Steps 2-9 for prose.
pub fn normalize_prose(input: &str) -> CanonText {
    normalize_prose_with_limit(input, MAX_LINE_CHARS)
}

/// [`normalize_prose`] with a different line limit, so tests can exercise long-line splitting cheaply.
pub(crate) fn normalize_prose_with_limit(input: &str, max_line: usize) -> CanonText {
    let lines = char_level(input);
    let mut em = Emitter::default();
    let mut refs: Option<u8> = None;
    let mut refs_span: Option<(u32, u32)> = None;
    let mut i = 0;
    while i < lines.len() {
        let raw = &lines[i];
        if is_blank(&raw.text) {
            em.blank(raw.breaks);
            i += 1;
            continue;
        }
        let p = peel(&raw.text);
        let content = p.content;
        if let Some((ch, n)) = fence_open(content) {
            let end = (i + 1..lines.len())
                .find(|&j| fence_closes(peel(&lines[j].text).content, ch, n))
                .unwrap_or(lines.len() - 1);
            i = em.verbatim(&lines, i, end, "code");
            continue;
        }
        if content.starts_with("$$") {
            let t = content.trim_end();
            let end = if t.len() >= 4 && t.ends_with("$$") {
                i
            } else {
                (i + 1..lines.len())
                    .take_while(|&j| !is_blank(&lines[j].text))
                    .find(|&j| peel(&lines[j].text).content.trim_end().ends_with("$$"))
                    .unwrap_or(i)
            };
            i = em.verbatim(&lines, i, end, "math");
            continue;
        }
        if content.starts_with('|') {
            let mut end = i;
            while end + 1 < lines.len()
                && !is_blank(&lines[end + 1].text)
                && peel(&lines[end + 1].text).content.starts_with('|')
            {
                end += 1;
            }
            i = em.verbatim(&lines, i, end, "table");
            continue;
        }
        if html_start(content) {
            let end = html_end(&lines, i, content);
            i = em.verbatim(&lines, i, end, "html");
            continue;
        }
        if let Some((level, text)) = atx_heading(content) {
            let n = em.line(raw.text.clone(), raw.breaks);
            if p.depth == 0 && !p.has_marker && p.indent <= 3 {
                if refs.is_some_and(|l| level <= l) {
                    refs = None;
                    if let Some((s, e)) = refs_span.take() {
                        em.blocks.push(BlockSpan { kind: "references".into(), start: s, end: e });
                    }
                }
                if is_reference_heading(&text) {
                    refs = Some(level);
                }
                em.headings.push((level, text, n));
            }
            i += 1;
            continue;
        }
        if is_rule_line(content) {
            em.line(raw.text.clone(), raw.breaks);
            i += 1;
            continue;
        }
        if refs.is_some() {
            let n = em.line(raw.text.clone(), raw.breaks);
            refs_span = Some(refs_span.map_or((n, n), |(s, _)| (s, n)));
            i += 1;
            continue;
        }
        if content.trim().is_empty() {
            em.line(p.prefix.trim_end().to_string(), raw.breaks);
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len() && continues_paragraph(&lines[j].text, p.depth) {
            j += 1;
        }
        emit_paragraph(&mut em, &lines[i..j], &p, max_line);
        i = j;
    }
    if let Some((s, e)) = refs_span {
        em.blocks.push(BlockSpan { kind: "references".into(), start: s, end: e });
    }
    finish(em)
}

fn finish(mut em: Emitter) -> CanonText {
    // A verbatim block that runs to the end (an unclosed fence) may end in blank lines; the file may not.
    while em.out.last().is_some_and(|l| is_blank(l)) {
        em.out.pop();
    }
    let total = line_no(em.out.len());
    em.page_breaks.retain(|&p| p <= total);
    em.blocks.retain(|b| b.start <= total);
    for b in &mut em.blocks {
        b.end = b.end.min(total);
    }
    let headings = em
        .headings
        .iter()
        .enumerate()
        .map(|(k, (level, text, start))| {
            let end = em.headings[k + 1..].iter().find(|(l, _, _)| l <= level).map_or(total, |(_, _, next)| next - 1);
            Heading { level: *level, text: text.clone(), start: *start, end, page: None }
        })
        .collect();
    let mut text = em.out.join("\n");
    if !em.out.is_empty() {
        text.push('\n');
    }
    CanonText { text, lines: total, page_breaks: em.page_breaks, headings, blocks: em.blocks }
}

// ------------------------------------------------------------------------------------------- paragraphs

/// Tabs become spaces, runs of spaces become one, no leading or trailing whitespace.
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == ' ' || c == '\t' {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out.trim_matches(char::is_whitespace).to_string()
}

fn emit_paragraph(em: &mut Emitter, lines: &[RawLine], first: &Peeled<'_>, max_line: usize) {
    let mut joined = String::new();
    let mut marks: Vec<(usize, u32)> = Vec::with_capacity(lines.len());
    for (k, raw) in lines.iter().enumerate() {
        let content = if k == 0 { first.content } else { peel(&raw.text).content };
        let piece = collapse(content);
        if !joined.is_empty() && !piece.is_empty() {
            joined.push(' ');
        }
        marks.push((joined.len(), raw.breaks));
        joined.push_str(&piece);
    }
    let width = first.prefix.chars().count().max(first.cont.chars().count());
    let limit = max_line.saturating_sub(width).max(1);
    let ranges = split_paragraph(&joined, &first.prefix, &first.cont, first.depth, limit);
    if ranges.is_empty() {
        // Only whitespace (callers never pass such a paragraph); keep its page breaks for the next line.
        em.blank(marks.iter().map(|(_, b)| b).sum());
        return;
    }
    let mut breaks = vec![0u32; ranges.len()];
    for (offset, b) in marks {
        let k = ranges.iter().rposition(|r| r.start <= offset).unwrap_or(0);
        breaks[k] += b;
    }
    for (k, r) in ranges.iter().enumerate() {
        let prefix = if k == 0 { &first.prefix } else { &first.cont };
        em.line(format!("{prefix}{}", &joined[r.clone()]), breaks[k]);
    }
}

const ABBREVIATIONS: &[&str] = &[
    "e.g.", "i.e.", "et al.", "cf.", "vs.", "Fig.", "Figs.", "Eq.", "Eqs.", "Ref.", "No.", "Vol.", "pp.", "p.", "ch.",
    "Ch.", "Sec.", "ca.", "approx.", "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "St.", "U.S.", "E.g.", "I.e.", "Cf.", "Vs.",
    "Pp.", "Ca.", "Approx.",
];

fn ends_with_abbreviation(s: &str) -> bool {
    let word_start = |head: &str| head.chars().next_back().is_none_or(|c| !c.is_alphanumeric());
    if ABBREVIATIONS.iter().any(|a| s.strip_suffix(a).is_some_and(word_start)) {
        return true;
    }
    // Initials such as `A.`: one uppercase letter, a full stop, nothing alphanumeric before it.
    let mut rev = s.chars().rev();
    matches!((rev.next(), rev.next()), (Some('.'), Some(c)) if c.is_uppercase()) && {
        let mut rev = s.chars().rev().skip(2);
        rev.next().is_none_or(|c| !c.is_alphanumeric())
    }
}

const CLOSERS: &[char] = &[
    ')', ']', '}', '"', '\'', '\u{201D}', '\u{2019}', '\u{300D}', '\u{300F}', '\u{FF09}', '\u{FF3D}', '\u{FF5D}',
    '\u{3015}', '\u{3011}', '\u{300B}', '\u{3009}', '\u{BB}', '\u{203A}', '\u{3017}', '\u{3019}', '\u{301B}',
];

fn ends_with_cjk_terminator(s: &str) -> bool {
    s.trim_end_matches(CLOSERS).ends_with(['\u{3002}', '\u{FF01}', '\u{FF1F}'])
}

/// Byte ranges of inline code spans and inline maths (`$…$`, `$$…$$`), inside which no line break may fall.
/// Maths follows pandoc's rule: the opening `$` is followed by a non-space, the closing one is preceded by a
/// non-space and not followed by a digit.
fn protected_ranges(s: &str) -> Vec<Range<usize>> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut math: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'$' => {
                let run = if b.get(i + 1) == Some(&b'$') { 2 } else { 1 };
                let next = s[i + run..].chars().next();
                match math {
                    None => {
                        if next.is_some_and(|c| !c.is_whitespace()) {
                            math = Some((i, run));
                        }
                    }
                    Some((start, open_run)) if open_run == run => {
                        let prev_ok = s[..i].chars().next_back().is_some_and(|c| !c.is_whitespace());
                        if prev_ok && (run == 2 || !next.is_some_and(|c| c.is_ascii_digit())) {
                            out.push(start..i + run);
                            math = None;
                        }
                    }
                    Some(_) => {}
                }
                i += run;
            }
            b'`' if math.is_none() => {
                let run = b[i..].iter().take_while(|&&c| c == b'`').count();
                let mut j = i + run;
                let mut close = None;
                while j < b.len() {
                    if b[j] == b'`' {
                        let r = b[j..].iter().take_while(|&&c| c == b'`').count();
                        if r == run {
                            close = Some(j + r);
                            break;
                        }
                        j += r;
                    } else {
                        j += 1;
                    }
                }
                match close {
                    Some(end) => {
                        out.push(i..end);
                        i = end;
                    }
                    None => i += run,
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// The paragraph's lines as byte ranges of `joined`.
fn split_paragraph(joined: &str, prefix: &str, cont: &str, depth: usize, limit: usize) -> Vec<Range<usize>> {
    let protected = protected_ranges(joined);
    let mut sentences: Vec<Range<usize>> = Vec::new();
    for (start, seg) in joined.split_sentence_bound_indices() {
        let lead = seg.len() - seg.trim_start_matches(char::is_whitespace).len();
        let r = start + lead..start + seg.trim_end_matches(char::is_whitespace).len();
        if r.start >= r.end {
            continue;
        }
        if let Some(prev) = sentences.last_mut()
            && !boundary_allowed(joined, prev, &r, &protected)
        {
            prev.end = r.end;
            continue;
        }
        sentences.push(r);
    }
    let mut lines = Vec::with_capacity(sentences.len());
    for r in sentences {
        split_long(joined, r, limit, &mut lines);
    }
    // Every line must re-read as part of this paragraph; merge any that would not.
    loop {
        let bad = lines.iter().enumerate().find_map(|(k, r)| {
            let text = &joined[r.clone()];
            let ok = if k == 0 { first_line_ok(prefix, text) } else { continuation_ok(cont, text, depth) };
            match (ok, k) {
                (true, _) => None,
                (false, 0) if lines.len() > 1 => Some(1),
                (false, 0) => None,
                (false, k) => Some(k),
            }
        });
        match bad {
            Some(k) => {
                let r = lines.remove(k);
                lines[k - 1].end = r.end;
            }
            None => return lines,
        }
    }
}

fn boundary_allowed(s: &str, prev: &Range<usize>, next: &Range<usize>, protected: &[Range<usize>]) -> bool {
    let gap = &s[prev.end..next.start];
    let prev_text = &s[prev.clone()];
    let gap_ok = gap == " " || (gap.is_empty() && ends_with_cjk_terminator(prev_text));
    gap_ok
        && !ends_with_abbreviation(prev_text)
        && !protected.iter().any(|p| p.start < next.start && p.end > prev.end)
        && line_start_ok(&s[next.start..])
}

/// A continuation line may not start like a container or block (checked on its first characters).
fn line_start_ok(text: &str) -> bool {
    let p = peel(text);
    p.prefix.is_empty() && p.depth == 0 && !p.has_marker && !is_block_start(p.content)
}

fn first_line_ok(prefix: &str, text: &str) -> bool {
    let line = format!("{prefix}{text}");
    let p = peel(&line);
    p.prefix == prefix && p.content == text && !text.is_empty() && !is_block_start(text)
}

fn continuation_ok(cont: &str, text: &str, depth: usize) -> bool {
    let line = format!("{cont}{text}");
    let p = peel(&line);
    p.prefix == cont && p.content == text && continues_paragraph(&line, depth)
}

/// Splits a sentence longer than `limit` characters at the last single space that fits, choosing only spaces
/// between two non-space characters, where the rest does not start a block and the piece does not end in `$$`.
fn split_long(s: &str, r: Range<usize>, limit: usize, out: &mut Vec<Range<usize>>) {
    let mut start = r.start;
    loop {
        let rest = &s[start..r.end];
        let Some((cut_max, _)) = rest.char_indices().nth(limit) else {
            out.push(start..r.end);
            return;
        };
        // A space at `cut_max` itself leaves a piece of exactly `limit` characters.
        let candidates = rest[..cut_max].char_indices().map(|(i, _)| i).chain(std::iter::once(cut_max));
        let split = candidates.collect::<Vec<_>>().into_iter().rev().map(|i| start + i).find(|&p| {
            s.as_bytes()[p] == b' '
                && p > start
                && s[..p].chars().next_back().is_some_and(|c| !c.is_whitespace())
                && s[p + 1..].chars().next().is_some_and(|c| !c.is_whitespace())
                && !s[start..p].ends_with("$$")
                && line_start_ok(&s[p + 1..r.end])
        });
        match split {
            Some(p) => {
                out.push(start..p);
                start = p + 1;
            }
            None => {
                out.push(start..r.end);
                return;
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------- tests

#[cfg(test)]
mod tests;

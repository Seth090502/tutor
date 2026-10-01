//! Text decoding and the plain-text and Markdown converters (passthrough with structure discovery).

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{UTF_16BE, UTF_16LE};

use crate::ConvertOptions;
use crate::error::{ExtractError, Result};
use crate::quality;
use crate::types::{BlockKind, BlockSpan, Extracted, HeadingMark, Metadata, tool_string};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextBom {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
}

/// Bytes the encoding detector looks at.
const DETECT_BYTES: usize = 1 << 20;

/// A decoded text file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    /// Lower-case WHATWG encoding name: `utf-8`, `utf-16le`, `windows-1252`, `shift_jis`...
    pub encoding: &'static str,
    /// Some bytes had no mapping in the encoding and became U+FFFD.
    pub lossy: bool,
}

fn bom_of(bytes: &[u8]) -> (TextBom, usize) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (TextBom::Utf8, 3)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (TextBom::Utf16Le, 2)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (TextBom::Utf16Be, 2)
    } else {
        (TextBom::None, 0)
    }
}

/// Guess UTF-16 without a byte-order mark: every other byte is NUL and the rest are mostly printable.
fn utf16_without_bom(b: &[u8]) -> Option<TextBom> {
    if b.len() < 4 {
        return None;
    }
    let n = b.len() / 2;
    let even_zero = (0..n).filter(|&i| b[2 * i] == 0).count();
    let odd_zero = (0..n).filter(|&i| b[2 * i + 1] == 0).count();
    if odd_zero * 10 >= n * 8 && even_zero * 10 <= n {
        Some(TextBom::Utf16Le)
    } else if even_zero * 10 >= n * 8 && odd_zero * 10 <= n {
        Some(TextBom::Utf16Be)
    } else {
        None
    }
}

/// Decide whether `head` is text and, if so, decode it (lossily) for sniffing. `None` means binary.
pub fn sniff_text_head(head: &[u8]) -> Option<(TextBom, String)> {
    let (bom, skip) = bom_of(head);
    match bom {
        TextBom::Utf8 => return Some((bom, String::from_utf8_lossy(&head[skip..]).into_owned())),
        TextBom::Utf16Le => return Some((bom, UTF_16LE.decode_without_bom_handling(&head[skip..]).0.into_owned())),
        TextBom::Utf16Be => return Some((bom, UTF_16BE.decode_without_bom_handling(&head[skip..]).0.into_owned())),
        TextBom::None => {}
    }
    if head.contains(&0) {
        let guess = utf16_without_bom(head)?;
        let enc = if guess == TextBom::Utf16Le { UTF_16LE } else { UTF_16BE };
        return Some((guess, enc.decode_without_bom_handling(head).0.into_owned()));
    }
    let control =
        head.iter().filter(|&&c| (c < 0x20 && !matches!(c, b'\t' | b'\n' | b'\r' | 0x0C | 0x1B)) || c == 0x7F).count();
    if control * 20 > head.len() {
        return None;
    }
    Some((TextBom::None, String::from_utf8_lossy(head).into_owned()))
}

/// Decode a whole text file: BOM, then UTF-8, then a statistical guess.
pub fn decode(bytes: &[u8]) -> Decoded {
    let (bom, skip) = bom_of(bytes);
    match bom {
        TextBom::Utf8 => {
            let (text, lossy) = lossy_utf8(&bytes[skip..]);
            return Decoded { text, encoding: "utf-8", lossy };
        }
        TextBom::Utf16Le => {
            let (t, had) = UTF_16LE.decode_without_bom_handling(&bytes[skip..]);
            return Decoded { text: t.into_owned(), encoding: "utf-16le", lossy: had };
        }
        TextBom::Utf16Be => {
            let (t, had) = UTF_16BE.decode_without_bom_handling(&bytes[skip..]);
            return Decoded { text: t.into_owned(), encoding: "utf-16be", lossy: had };
        }
        TextBom::None => {}
    }
    let head = &bytes[..bytes.len().min(4096)];
    // ASCII-range UTF-16 is also valid UTF-8 (every other byte is NUL), so look for it first.
    if head.contains(&0)
        && let Some(guess) = utf16_without_bom(head)
    {
        let (enc, name) = if guess == TextBom::Utf16Le { (UTF_16LE, "utf-16le") } else { (UTF_16BE, "utf-16be") };
        let (t, had) = enc.decode_without_bom_handling(bytes);
        return Decoded { text: t.into_owned(), encoding: name, lossy: had };
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Decoded { text: s.to_string(), encoding: "utf-8", lossy: false };
    }
    let mut det = EncodingDetector::new(Iso2022JpDetection::Deny);
    let sample = &bytes[..bytes.len().min(DETECT_BYTES)];
    det.feed(sample, sample.len() == bytes.len());
    let enc = det.guess(None, Utf8Detection::Allow);
    let (t, had) = enc.decode_without_bom_handling(bytes);
    Decoded { text: t.into_owned(), encoding: lower_name(enc.name()), lossy: had }
}

fn lower_name(name: &'static str) -> &'static str {
    match name {
        "UTF-8" => "utf-8",
        "windows-1252" => "windows-1252",
        "windows-1251" => "windows-1251",
        "windows-1250" => "windows-1250",
        "ISO-8859-2" => "iso-8859-2",
        "ISO-8859-15" => "iso-8859-15",
        "Shift_JIS" => "shift_jis",
        "EUC-JP" => "euc-jp",
        "EUC-KR" => "euc-kr",
        "GBK" => "gbk",
        "gb18030" => "gb18030",
        "Big5" => "big5",
        "KOI8-R" => "koi8-r",
        "KOI8-U" => "koi8-u",
        "UTF-16LE" => "utf-16le",
        "UTF-16BE" => "utf-16be",
        "windows-1253" => "windows-1253",
        "windows-1254" => "windows-1254",
        "windows-1255" => "windows-1255",
        "windows-1256" => "windows-1256",
        "windows-1257" => "windows-1257",
        "windows-1258" => "windows-1258",
        "windows-874" => "windows-874",
        _ => "other",
    }
}

fn lossy_utf8(b: &[u8]) -> (String, bool) {
    match std::str::from_utf8(b) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (String::from_utf8_lossy(b).into_owned(), true),
    }
}

/// Decode, apply the mojibake repair when enabled, and record what happened.
pub(crate) fn decode_for_conversion(bytes: &[u8], opts: &ConvertOptions, warnings: &mut Vec<String>) -> Decoded {
    let mut d = decode(bytes);
    if d.lossy {
        warnings.push(format!("some bytes were not valid {} and became U+FFFD", d.encoding));
    }
    if opts.repair_mojibake
        && let Some(fixed) = quality::repair_mojibake(&d.text)
    {
        warnings.push("repaired UTF-8 text that had been decoded as Latin-1 (mojibake)".to_string());
        d.text = fixed;
    }
    d
}

fn normalise_newlines(s: &str) -> String {
    if s.contains('\r') { s.replace("\r\n", "\n").replace('\r', "\n") } else { s.to_string() }
}

/// Plain text: the decoded text with Unix line endings. Structure is left to vault-core's normaliser.
pub fn convert_text(bytes: &[u8], opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut warnings = Vec::new();
    let d = decode_for_conversion(bytes, opts, &mut warnings);
    let mut markdown = normalise_newlines(&d.text);
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }
    Ok(Extracted {
        markdown,
        warnings,
        tool: tool_string("text"),
        encoding: Some(d.encoding.to_string()),
        ..Default::default()
    })
}

/// Markdown: the text unchanged, with headings, verbatim blocks and front-matter metadata discovered.
pub fn convert_markdown(bytes: &[u8], opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut warnings = Vec::new();
    let d = decode_for_conversion(bytes, opts, &mut warnings);
    let mut markdown = normalise_newlines(&d.text);
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }
    let a = analyze_markdown(&markdown);
    let mut metadata = a.metadata;
    if metadata.title.is_none() {
        metadata.title = a.headings.iter().find(|h| h.level == 1).map(|h| h.text.clone());
    }
    Ok(Extracted {
        markdown,
        headings: a.headings,
        blocks: a.blocks,
        metadata,
        warnings,
        tool: tool_string("markdown"),
        encoding: Some(d.encoding.to_string()),
        ..Default::default()
    })
}

pub(crate) struct MarkdownAnalysis {
    pub headings: Vec<HeadingMark>,
    pub blocks: Vec<BlockSpan>,
    pub metadata: Metadata,
}

/// Find headings (ATX and setext), fenced code, display maths and tables, outside of one another, and read a
/// leading YAML front matter block.
pub(crate) fn analyze_markdown(md: &str) -> MarkdownAnalysis {
    let lines: Vec<&str> = md.lines().collect();
    let mut headings = Vec::new();
    let mut blocks = Vec::new();
    let mut i = 0usize;
    let mut metadata = Metadata::default();

    if lines.first().is_some_and(|l| l.trim_end() == "---")
        && let Some(end) = lines.iter().skip(1).take(200).position(|l| matches!(l.trim_end(), "---" | "..."))
    {
        read_front_matter(&lines[1..=end], &mut metadata);
        i = end + 2;
    }

    let mut table_start: Option<usize> = None;
    let flush_table = |start: &mut Option<usize>, end: usize, blocks: &mut Vec<BlockSpan>| {
        if let Some(s) = start.take()
            && end - s >= 2
        {
            blocks.push(BlockSpan { kind: BlockKind::Table, start: s as u32 + 1, end: end as u32 });
        }
    };
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();

        if let Some((ch, n)) = fence_open(t) {
            flush_table(&mut table_start, i, &mut blocks);
            let start = i;
            let mut j = i + 1;
            while j < lines.len() && !fence_close(lines[j].trim_start(), ch, n) {
                j += 1;
            }
            let end = j.min(lines.len() - 1);
            blocks.push(BlockSpan { kind: BlockKind::Code, start: start as u32 + 1, end: end as u32 + 1 });
            i = end + 1;
            continue;
        }
        if t.starts_with("$$") {
            flush_table(&mut table_start, i, &mut blocks);
            let single = t.strip_prefix("$$").is_some_and(|rest| rest.contains("$$"));
            let start = i;
            let mut end = i;
            if !single {
                let mut j = i + 1;
                while j < lines.len() && !lines[j].contains("$$") {
                    j += 1;
                }
                end = j.min(lines.len() - 1);
            }
            blocks.push(BlockSpan { kind: BlockKind::Equation, start: start as u32 + 1, end: end as u32 + 1 });
            i = end + 1;
            continue;
        }
        if t.starts_with('|') {
            table_start.get_or_insert(i);
            i += 1;
            continue;
        }
        flush_table(&mut table_start, i, &mut blocks);

        if let Some((level, text)) = atx_heading(line) {
            headings.push(HeadingMark { level, text, line: i as u32 + 1 });
        } else if i + 1 < lines.len() && !line.trim().is_empty() && !t.starts_with(['>', '-', '*', '+']) {
            let next = lines[i + 1].trim_end();
            let underline = |c: char| next.len() >= 2 && next.chars().all(|x| x == c);
            if underline('=') {
                headings.push(HeadingMark { level: 1, text: line.trim().to_string(), line: i as u32 + 1 });
                i += 2;
                continue;
            } else if underline('-') && is_paragraph_line(lines.get(i.wrapping_sub(1)).copied()) {
                headings.push(HeadingMark { level: 2, text: line.trim().to_string(), line: i as u32 + 1 });
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    flush_table(&mut table_start, lines.len(), &mut blocks);
    MarkdownAnalysis { headings, blocks, metadata }
}

/// A setext heading's text line is a paragraph of its own: the line above it is blank (or the file start).
fn is_paragraph_line(prev: Option<&str>) -> bool {
    prev.is_none_or(|p| p.trim().is_empty())
}

fn fence_open(t: &str) -> Option<(char, usize)> {
    let ch = t.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let n = t.chars().take_while(|&c| c == ch).count();
    if n < 3 {
        return None;
    }
    // An opening backtick fence cannot have backticks in its info string.
    if ch == '`' && t[n..].contains('`') {
        return None;
    }
    Some((ch, n))
}

fn fence_close(t: &str, ch: char, n: usize) -> bool {
    let run = t.chars().take_while(|&c| c == ch).count();
    run >= n && t[run..].trim().is_empty()
}

fn atx_heading(line: &str) -> Option<(u8, String)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let t = line.trim_start_matches(' ').trim_end();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &t[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    // A closing run of hashes counts only when it follows a space (or is the whole content).
    let body = rest.trim();
    let stripped = body.trim_end_matches('#');
    let text = if stripped.len() == body.len() || stripped.is_empty() || stripped.ends_with(char::is_whitespace) {
        stripped.trim_end().to_string()
    } else {
        body.to_string()
    };
    if text.is_empty() {
        return None;
    }
    Some((hashes as u8, text))
}

fn read_front_matter(lines: &[&str], md: &mut Metadata) {
    let mut key: Option<String> = None;
    let mut items: Vec<String> = Vec::new();
    let flush = |key: &mut Option<String>, items: &mut Vec<String>, md: &mut Metadata| {
        if let Some(k) = key.take() {
            apply_front_matter(&k, std::mem::take(items), md);
        }
    };
    for l in lines {
        let trimmed = l.trim();
        if let Some(item) = trimmed.strip_prefix("- ") {
            items.push(unquote(item));
            continue;
        }
        if l.starts_with(' ') || l.starts_with('\t') || trimmed.is_empty() || trimmed.starts_with('#') {
            // Continuation of a folded or block scalar: append to the current value.
            if let (Some(_), false) = (&key, trimmed.is_empty() || trimmed.starts_with('#'))
                && let Some(last) = items.last_mut()
            {
                last.push(' ');
                last.push_str(trimmed);
            }
            continue;
        }
        flush(&mut key, &mut items, md);
        if let Some((k, v)) = trimmed.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            key = Some(k);
            if !v.is_empty() && !matches!(v, "|" | ">" | "|-" | ">-") {
                if let Some(list) = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                    items.extend(list.split(',').map(unquote).filter(|s| !s.is_empty()));
                } else {
                    items.push(unquote(v));
                }
            } else {
                items.push(String::new());
            }
        }
    }
    flush(&mut key, &mut items, md);
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(s);
    s.trim().to_string()
}

fn apply_front_matter(key: &str, items: Vec<String>, md: &mut Metadata) {
    let items: Vec<String> = items.into_iter().filter(|s| !s.is_empty()).collect();
    let first = items.first().cloned();
    match key {
        "title" => md.title = first,
        "author" | "authors" => {
            md.authors = items
                .iter()
                .flat_map(|a| a.split(" and "))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        "date" | "year" | "published" | "created" => md.year = first.and_then(|d| crate::ids::year_in(&d)),
        "lang" | "language" => md.language = first,
        "keywords" | "tags" | "tag" => md.keywords = items,
        "doi" => md.doi = first.and_then(|d| crate::ids::find_doi(&d)),
        "arxiv" | "arxiv_id" | "arxiv-id" => md.arxiv_id = first.and_then(|d| crate::ids::find_arxiv(&d).or(Some(d))),
        "pmcid" => md.pmcid = first.and_then(|d| crate::ids::normalize_pmcid(&d)),
        "isbn" => md.isbn = items.iter().filter_map(|i| crate::ids::normalize_isbn(i)).collect(),
        "journal" | "container-title" => md.journal = first,
        "abstract" | "description" | "summary" => md.abstract_text = first,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ConvertOptions {
        ConvertOptions::default()
    }

    #[test]
    fn decodes_utf8_bom_utf16_and_legacy_encodings() {
        let d = decode(b"\xEF\xBB\xBFhello \xC3\xA9");
        assert_eq!((d.text.as_str(), d.encoding, d.lossy), ("hello é", "utf-8", false));

        let mut le = vec![0xFF, 0xFE];
        le.extend("héllo".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode(&le).text, "héllo");
        assert_eq!(decode(&le).encoding, "utf-16le");
        let mut be = vec![0xFE, 0xFF];
        be.extend("héllo".encode_utf16().flat_map(u16::to_be_bytes));
        assert_eq!(decode(&be).text, "héllo");

        // UTF-16 with no BOM.
        let nobom: Vec<u8> = "plain ascii text here".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode(&nobom).text, "plain ascii text here");

        // Windows-1252: "Café “quoted” – ok" has bytes that are not valid UTF-8.
        let (bytes, _, _) = encoding_rs::WINDOWS_1252
            .encode("Un café très chaud, “merci” – s'il vous plaît, mademoiselle. C'est déjà l'été.");
        let d = decode(&bytes);
        assert!(d.text.contains("café") && d.text.contains("très"), "{} ({})", d.text, d.encoding);
        assert!(!d.lossy);
        assert_ne!(d.encoding, "utf-8");
    }

    #[test]
    fn decodes_japanese_shift_jis() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("これは日本語の文章です。数学の証明について書かれています。");
        let d = decode(&bytes);
        assert!(d.text.contains("日本語"), "{} ({})", d.text, d.encoding);
        assert_eq!(d.encoding, "shift_jis");
    }

    #[test]
    fn invalid_utf8_after_a_good_bom_is_lossy_and_reported() {
        let r = convert_text(b"\xEF\xBB\xBFgood \xFF bad", &opts()).unwrap();
        assert!(r.markdown.contains('\u{FFFD}'));
        assert!(r.warnings.iter().any(|w| w.contains("U+FFFD")));
    }

    #[test]
    fn plain_text_keeps_content_and_normalises_line_endings() {
        let r = convert_text(b"line one\r\nline two\rline three", &opts()).unwrap();
        assert_eq!(r.markdown, "line one\nline two\nline three\n");
        assert_eq!(r.encoding.as_deref(), Some("utf-8"));
        assert!(matches!(convert_text(b"", &opts()), Err(ExtractError::Empty)));
    }

    #[test]
    fn mojibake_in_text_is_repaired_with_a_warning() {
        let (garbled, _, _) =
            encoding_rs::WINDOWS_1252.decode("Café déjà vu — “quoted” naïve façade, über señor".as_bytes());
        let r = convert_text(garbled.as_bytes(), &opts()).unwrap();
        assert_eq!(r.markdown, "Café déjà vu — “quoted” naïve façade, über señor\n");
        assert!(r.warnings.iter().any(|w| w.contains("mojibake")));
        let off = ConvertOptions { repair_mojibake: false };
        assert!(convert_text(garbled.as_bytes(), &off).unwrap().markdown.contains("Ã"));
    }

    #[test]
    fn markdown_structure_is_discovered() {
        let md = "---\ntitle: \"A Study\"\nauthors:\n  - Ann Lee\n  - Bo Chen\nyear: 2021\nlang: en\ntags: [alpha, beta]\ndoi: 10.1000/xyz123\n---\n\n# Heading One\n\nText\n\n```python\n# not a heading\nprint(1)\n```\n\nSetext\n======\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n$$\nx^2\n$$\n\n## Second *level* ##\n";
        let r = convert_markdown(md.as_bytes(), &opts()).unwrap();
        assert_eq!(r.metadata.title.as_deref(), Some("A Study"));
        assert_eq!(r.metadata.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(r.metadata.year, Some(2021));
        assert_eq!(r.metadata.language.as_deref(), Some("en"));
        assert_eq!(r.metadata.keywords, vec!["alpha", "beta"]);
        assert_eq!(r.metadata.doi.as_deref(), Some("10.1000/xyz123"));
        let hs: Vec<_> = r.headings.iter().map(|h| (h.level, h.text.as_str(), h.line)).collect();
        assert_eq!(hs, vec![(1, "Heading One", 12), (1, "Setext", 21), (2, "Second *level*", 32)]);
        let bs: Vec<_> = r.blocks.iter().map(|b| (b.kind, b.start, b.end)).collect();
        assert_eq!(bs, vec![(BlockKind::Code, 16, 19), (BlockKind::Table, 24, 26), (BlockKind::Equation, 28, 30)]);
        // The text is untouched.
        assert_eq!(r.markdown, md);
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert_eq!(lines[11], "# Heading One");
        assert_eq!(lines[20], "Setext");
    }

    #[test]
    fn markdown_title_falls_back_to_first_h1() {
        let r = convert_markdown(b"Intro\n\n# The Real Title\n\nbody\n", &opts()).unwrap();
        assert_eq!(r.metadata.title.as_deref(), Some("The Real Title"));
    }

    #[test]
    fn hash_without_space_and_unclosed_fence() {
        let r = convert_markdown(b"#hashtag line\n\n```\nunclosed fence\nstill code\n", &opts()).unwrap();
        assert!(r.headings.is_empty());
        assert_eq!(r.blocks.len(), 1);
        assert_eq!((r.blocks[0].start, r.blocks[0].end), (3, 5));
    }

    #[test]
    fn horizontal_rule_after_paragraph_is_not_lost_as_heading_text() {
        let r = convert_markdown(b"first para\nsecond line\n---\n\nnext\n", &opts()).unwrap();
        // A rule directly under a multi-line paragraph is not a setext heading in our reading.
        assert!(r.headings.is_empty(), "{:?}", r.headings);
    }
}

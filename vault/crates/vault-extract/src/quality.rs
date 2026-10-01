//! Text quality metrics: how garbled, how empty, how incomplete an extraction looks. Cheap, deterministic,
//! language-agnostic heuristics; the score is for routing and flagging, not a proof of correctness.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::Extracted;

/// A page with fewer non-whitespace characters than this counts as empty.
pub const EMPTY_PAGE_CHARS: usize = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityReport {
    /// 0.0 (unusable) to 1.0 (clean).
    pub score: f32,
    /// Garbled characters over non-whitespace characters.
    pub garbled_ratio: f32,
    pub empty_pages: u32,
    pub empty_page_ratio: f32,
    /// Non-whitespace characters.
    pub chars: usize,
    pub words: usize,
    /// Stable flag names: `replacement_chars`, `private_use`, `control_chars`, `mojibake`, `unmapped_glyphs`,
    /// `empty_pages`, `too_short`, `no_headings`, `missing_sections`, `glued_words`, `spaced_letters`,
    /// `repeated_lines`.
    pub flags: Vec<String>,
}

/// What the caller knows about the document, for the checks that depend on it.
#[derive(Debug, Clone, Copy, Default)]
pub struct QualityContext {
    /// A research paper: expect an abstract, an introduction and references.
    pub expect_paper_sections: bool,
    /// Pages in the source when known (a PDF's page count), for the empty-page ratio when the converter did not
    /// emit page marks for every page.
    pub source_pages: Option<u32>,
}

/// Counts of characters that indicate a broken decode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GarbleCounts {
    pub replacement: usize,
    pub private_use: usize,
    pub control: usize,
    /// Characters taking part in UTF-8-read-as-Latin-1/cp1252 sequences.
    pub mojibake: usize,
    /// Characters inside `(cid:123)` unmapped-glyph markers (PDF fonts without a ToUnicode map).
    pub unmapped_glyphs: usize,
    pub non_whitespace: usize,
}

impl GarbleCounts {
    pub fn garbled(&self) -> usize {
        self.replacement + self.private_use + self.control + self.mojibake + self.unmapped_glyphs
    }

    pub fn ratio(&self) -> f32 {
        if self.non_whitespace == 0 { 0.0 } else { (self.garbled() as f32 / self.non_whitespace as f32).min(1.0) }
    }
}

static CID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(cid:\d+\)").expect("static regex"));

/// The byte a character stands for when UTF-8 bytes were decoded as windows-1252 (which agrees with Latin-1
/// outside 0x80-0x9F).
fn cp1252_byte(c: char) -> Option<u8> {
    let u = c as u32;
    if (0xA0..=0xFF).contains(&u) || (0x80..=0x9F).contains(&u) {
        return Some(u as u8);
    }
    Some(match c {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        _ => return None,
    })
}

/// Find UTF-8 sequences that were read as single-byte characters. Yields `(start_index, char_len, decoded)` in
/// character indices over `chars`.
fn mojibake_sequences(chars: &[char]) -> Vec<(usize, usize, char)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let lead = match cp1252_byte(chars[i]) {
            Some(b) if (0xC2..=0xF4).contains(&b) => b,
            _ => {
                i += 1;
                continue;
            }
        };
        let need = if lead >= 0xF0 {
            3
        } else if lead >= 0xE0 {
            2
        } else {
            1
        };
        if i + need >= chars.len() {
            i += 1;
            continue;
        }
        let mut bytes = vec![lead];
        let mut ok = true;
        for k in 1..=need {
            match cp1252_byte(chars[i + k]) {
                Some(b) if (0x80..=0xBF).contains(&b) => bytes.push(b),
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok
            && let Ok(s) = std::str::from_utf8(&bytes)
            && let Some(c) = s.chars().next()
        {
            out.push((i, need + 1, c));
            i += need + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// Count garbled characters in `text`.
pub fn garble_counts(text: &str) -> GarbleCounts {
    let mut g = GarbleCounts::default();
    let chars: Vec<char> = text.chars().collect();
    for &c in &chars {
        if c.is_whitespace() {
            continue;
        }
        g.non_whitespace += 1;
        let u = c as u32;
        match c {
            '\u{FFFD}' => g.replacement += 1,
            '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}' => g.private_use += 1,
            _ if (0x80..=0x9F).contains(&u)
                || (u < 0x20 && !matches!(c, '\t' | '\n' | '\r' | '\u{c}'))
                || u == 0x7F =>
            {
                g.control += 1
            }
            _ => {}
        }
    }
    g.mojibake = mojibake_sequences(&chars).iter().map(|(_, n, _)| n).sum();
    g.unmapped_glyphs = CID.find_iter(text).map(|m| m.as_str().chars().count()).sum();
    g
}

/// Number of UTF-8-read-as-Latin-1 sequences in `text`.
pub fn mojibake_count(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    mojibake_sequences(&chars).len()
}

/// Undo UTF-8-read-as-Latin-1/cp1252 damage, but only when it is clearly systematic: at least three sequences,
/// and almost every non-ASCII character is part of one. Returns `None` for text that needs no repair, so
/// legitimate text that merely contains an accented capital followed by a symbol is left alone.
pub fn repair_mojibake(text: &str) -> Option<String> {
    let mut current = text.to_string();
    let mut repaired = false;
    for _ in 0..2 {
        let chars: Vec<char> = current.chars().collect();
        let seqs = mojibake_sequences(&chars);
        if seqs.len() < 3 {
            break;
        }
        let non_ascii = chars.iter().filter(|c| !c.is_ascii()).count();
        let covered: usize = seqs.iter().map(|(_, n, _)| n).sum();
        if non_ascii == 0 || (covered as f32) < 0.9 * non_ascii as f32 {
            break;
        }
        let mut out = String::with_capacity(current.len());
        let mut pos = 0;
        for (start, len, decoded) in seqs {
            out.extend(&chars[pos..start]);
            out.push(decoded);
            pos = start + len;
        }
        out.extend(&chars[pos..]);
        current = out;
        repaired = true;
    }
    repaired.then_some(current)
}

fn flag(flags: &mut Vec<String>, name: &str) {
    if !flags.iter().any(|f| f == name) {
        flags.push(name.to_string());
    }
}

/// Assess raw text with no structure.
pub fn assess_text(text: &str) -> QualityReport {
    assess_inner(text, &[], &[], QualityContext::default())
}

/// Assess a converter's output: the text, its pages and its headings.
pub fn assess(ex: &Extracted, ctx: QualityContext) -> QualityReport {
    let lines: Vec<&str> = ex.markdown.lines().collect();
    // Non-whitespace characters per page, from the page marks.
    let mut page_chars: Vec<usize> = Vec::new();
    for (i, p) in ex.pages.iter().enumerate() {
        let start = (p.line as usize).saturating_sub(1).min(lines.len());
        let end = ex
            .pages
            .get(i + 1)
            .map(|n| (n.line as usize).saturating_sub(1))
            .unwrap_or(lines.len())
            .clamp(start, lines.len());
        page_chars.push(lines[start..end].iter().map(|l| l.chars().filter(|c| !c.is_whitespace()).count()).sum());
    }
    let headings: Vec<String> = ex.headings.iter().map(|h| h.text.to_lowercase()).collect();
    assess_inner(&ex.markdown, &page_chars, &headings, ctx)
}

fn assess_inner(text: &str, page_chars: &[usize], headings: &[String], ctx: QualityContext) -> QualityReport {
    let g = garble_counts(text);
    let garbled_ratio = g.ratio();
    let words = text.split_whitespace().count();
    let mut flags = Vec::new();

    if g.replacement > 0 && (g.replacement as f32 / g.non_whitespace.max(1) as f32) > 0.0005 || g.replacement >= 5 {
        flag(&mut flags, "replacement_chars");
    }
    if g.private_use > 0 && g.private_use as f32 / g.non_whitespace.max(1) as f32 > 0.001 {
        flag(&mut flags, "private_use");
    }
    if g.control > 0 {
        flag(&mut flags, "control_chars");
    }
    if g.mojibake >= 3 {
        flag(&mut flags, "mojibake");
    }
    if g.unmapped_glyphs > 0 {
        flag(&mut flags, "unmapped_glyphs");
    }

    let total_pages = ctx.source_pages.unwrap_or(page_chars.len() as u32).max(page_chars.len() as u32);
    let unmarked = if page_chars.is_empty() { 0 } else { total_pages - page_chars.len() as u32 };
    let empty_pages = page_chars.iter().filter(|&&c| c < EMPTY_PAGE_CHARS).count() as u32 + unmarked;
    let empty_page_ratio = if total_pages == 0 { 0.0 } else { empty_pages as f32 / total_pages as f32 };
    if empty_page_ratio > 0.05 && empty_pages >= 1 {
        flag(&mut flags, "empty_pages");
    }

    let min_words = if total_pages > 1 { 50 } else { 5 };
    if words < min_words {
        flag(&mut flags, "too_short");
    }
    let nlines = text.lines().count();
    if headings.is_empty() && nlines > 300 {
        flag(&mut flags, "no_headings");
    }

    let mut missing = 0;
    if ctx.expect_paper_sections {
        let has = |needles: &[&str]| headings.iter().any(|h| needles.iter().any(|n| h.contains(n)));
        let lower_text_has_abstract = text.to_lowercase().contains("abstract");
        if !has(&["abstract"]) && !lower_text_has_abstract {
            missing += 1;
        }
        if !has(&["introduction"]) && !headings.is_empty() {
            missing += 1;
        }
        if !has(&["reference", "bibliograph", "literature cited", "works cited"]) {
            missing += 1;
        }
        if missing > 0 {
            flag(&mut flags, "missing_sections");
        }
    }

    // Word-boundary damage: extraction that dropped spaces or spaced out every letter. Skipped for scripts
    // written without spaces.
    let cjk = cjk_ratio(text);
    if words >= 100 && cjk < 0.2 {
        let toks: Vec<&str> = text.split_whitespace().collect();
        let avg_len = toks.iter().map(|t| t.chars().count()).sum::<usize>() as f32 / toks.len() as f32;
        let single = toks.iter().filter(|t| t.chars().count() == 1 && t.chars().all(char::is_alphabetic)).count()
            as f32
            / toks.len() as f32;
        if avg_len > 14.0 {
            flag(&mut flags, "glued_words");
        }
        if single > 0.45 {
            flag(&mut flags, "spaced_letters");
        }
    }
    if repeated_line_ratio(text) > 0.5 && nlines > 40 {
        flag(&mut flags, "repeated_lines");
    }

    let mut score = 1.0f32;
    score -= (garbled_ratio * 10.0).min(0.7);
    score -= (empty_page_ratio - 0.05).clamp(0.0, 0.5);
    for f in &flags {
        score -= match f.as_str() {
            "too_short" => 0.3,
            "mojibake" => 0.2,
            "glued_words" | "spaced_letters" => 0.3,
            "repeated_lines" => 0.2,
            "missing_sections" => 0.03 * missing as f32,
            "no_headings" => 0.03,
            _ => 0.0,
        };
    }
    QualityReport {
        score: score.clamp(0.0, 1.0),
        garbled_ratio,
        empty_pages,
        empty_page_ratio,
        chars: g.non_whitespace,
        words,
        flags,
    }
}

/// Share of non-whitespace characters that are CJK ideographs, kana or hangul.
pub fn cjk_ratio(text: &str) -> f32 {
    let mut total = 0usize;
    let mut cjk = 0usize;
    for c in text.chars().filter(|c| !c.is_whitespace()) {
        total += 1;
        if is_cjk(c) {
            cjk += 1;
        }
    }
    if total == 0 { 0.0 } else { cjk as f32 / total as f32 }
}

pub(crate) fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x31F0..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x20000..=0x2FA1F | 0xFF66..=0xFF9F)
}

/// Fraction of non-blank lines that repeat an earlier line (OCR loops, running headers extracted as text).
fn repeated_line_ratio(text: &str) -> f32 {
    let mut seen = std::collections::HashSet::new();
    let mut total = 0usize;
    let mut repeats = 0usize;
    for l in text.lines().map(str::trim).filter(|l| l.chars().count() > 3) {
        total += 1;
        if !seen.insert(l) {
            repeats += 1;
        }
    }
    if total == 0 { 0.0 } else { repeats as f32 / total as f32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HeadingMark, PageMark};

    const CLEAN: &str = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. How vexingly quick daft zebras jump. Sphinx of black quartz, judge my vow. The five boxing wizards jump quickly. We promptly judged antique ivory buckles for the next prize. A wizard's job is to vex chumps quickly in fog. Crazy Fredrick bought many very exquisite opal jewels.";

    #[test]
    fn clean_text_scores_high() {
        let r = assess_text(CLEAN);
        assert!(r.score > 0.95, "{r:?}");
        assert_eq!(r.garbled_ratio, 0.0);
        assert!(r.flags.is_empty(), "{:?}", r.flags);
    }

    #[test]
    fn replacement_private_use_and_control_chars_are_counted() {
        let g = garble_counts("ab\u{FFFD}\u{FFFD} cd\u{E001} ef\u{0090}\u{0003}");
        assert_eq!((g.replacement, g.private_use, g.control), (2, 1, 2));
        assert_eq!(g.non_whitespace, 11);
        let bad = format!("{CLEAN} {}", "\u{FFFD}".repeat(40));
        let r = assess_text(&bad);
        assert!(r.flags.contains(&"replacement_chars".to_string()));
        assert!(r.score < 0.9);
    }

    #[test]
    fn mojibake_is_detected_and_repaired() {
        let good = "Café déjà vu — “quoted” text, naïve façade, über, señor, 日本語";
        let bytes = good.as_bytes();
        // Decode the UTF-8 bytes as windows-1252, as a careless reader would.
        let (garbled, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
        assert_ne!(garbled, good);
        assert!(mojibake_count(&garbled) >= 5, "{garbled}");
        let r = assess_text(&format!("{garbled} {CLEAN}"));
        assert!(r.flags.contains(&"mojibake".to_string()), "{:?}", r.flags);
        let fixed = repair_mojibake(&garbled).expect("repairable");
        assert_eq!(fixed, good);
    }

    #[test]
    fn repair_leaves_legitimate_text_alone() {
        assert_eq!(repair_mojibake("Plain ASCII only."), None);
        assert_eq!(repair_mojibake("Café, naïve, über, señor, façade"), None);
        // One accidental sequence is not systematic.
        assert_eq!(repair_mojibake("ÂÃ© once but otherwise “fine” and café"), None);
    }

    #[test]
    fn latin1_mojibake_without_cp1252_specials() {
        // "é" (C3 A9) read as Latin-1 is "Ã©".
        let garbled = "Ã©t\u{c3}\u{a9} caf\u{c3}\u{a9} \u{c3}\u{a0} la cr\u{c3}\u{a8}me";
        assert_eq!(repair_mojibake(garbled).as_deref(), Some("été café à la crème"));
    }

    #[test]
    fn unmapped_glyph_markers_count_as_garbled() {
        let r = assess_text(&format!("{CLEAN} (cid:12)(cid:45)(cid:7) {CLEAN}"));
        assert!(r.flags.contains(&"unmapped_glyphs".to_string()));
        assert!(r.garbled_ratio > 0.0);
    }

    fn page(label: &str, index: u32, line: u32) -> PageMark {
        PageMark { label: Some(label.into()), index, line }
    }

    #[test]
    fn empty_pages_are_measured_from_page_marks() {
        let mut md = String::new();
        let mut pages = Vec::new();
        for i in 1..=10u32 {
            pages.push(page(&i.to_string(), i, md.lines().count() as u32 + 1));
            if i % 2 == 0 {
                md.push('\n');
            } else {
                md.push_str(&format!("{CLEAN}\n"));
            }
        }
        let ex = Extracted { markdown: md, pages, ..Default::default() };
        let r = assess(&ex, QualityContext::default());
        assert_eq!(r.empty_pages, 5);
        assert!((r.empty_page_ratio - 0.5).abs() < 1e-6);
        assert!(r.flags.contains(&"empty_pages".to_string()));
        assert!(r.score < 0.7, "{r:?}");
    }

    #[test]
    fn pages_missing_from_the_markdown_count_as_empty() {
        let ex = Extracted {
            markdown: format!("{CLEAN}\n{CLEAN}\n"),
            pages: vec![page("1", 1, 1), page("2", 2, 2)],
            ..Default::default()
        };
        let r = assess(&ex, QualityContext { source_pages: Some(4), ..Default::default() });
        assert_eq!(r.empty_pages, 2);
    }

    #[test]
    fn missing_sections_for_papers() {
        let ex = Extracted {
            markdown: format!("# Title\n\n{CLEAN}\n\n## Methods\n\n{CLEAN}\n"),
            headings: vec![
                HeadingMark { level: 1, text: "Title".into(), line: 1 },
                HeadingMark { level: 2, text: "Methods".into(), line: 5 },
            ],
            ..Default::default()
        };
        let r = assess(&ex, QualityContext { expect_paper_sections: true, ..Default::default() });
        assert!(r.flags.contains(&"missing_sections".to_string()));
        let ok = Extracted {
            markdown: format!("# T\n\nAbstract\n\n{CLEAN}\n"),
            headings: vec![
                HeadingMark { level: 2, text: "Abstract".into(), line: 3 },
                HeadingMark { level: 2, text: "1 Introduction".into(), line: 4 },
                HeadingMark { level: 2, text: "References".into(), line: 5 },
            ],
            ..Default::default()
        };
        let r = assess(&ok, QualityContext { expect_paper_sections: true, ..Default::default() });
        assert!(!r.flags.contains(&"missing_sections".to_string()), "{:?}", r.flags);
    }

    #[test]
    fn too_short_and_spaced_letters_and_glued_words() {
        assert!(assess_text("hi").flags.contains(&"too_short".to_string()));
        let spaced: String = CLEAN.chars().flat_map(|c| [c, ' ']).collect::<String>().repeat(2);
        assert!(assess_text(&spaced).flags.contains(&"spaced_letters".to_string()));
        let glued: String = CLEAN.split_whitespace().collect::<Vec<_>>().join("").clone();
        let glued = format!("{} {}", glued, glued);
        let words_long = (0..120).map(|_| "internationalizationconsiderations").collect::<Vec<_>>().join(" ");
        assert!(assess_text(&words_long).flags.contains(&"glued_words".to_string()), "{glued}");
    }

    #[test]
    fn cjk_text_is_not_flagged_for_missing_spaces() {
        let zh = "这是一个关于数学证明的段落。".repeat(60);
        let r = assess_text(&zh);
        assert!(!r.flags.contains(&"glued_words".to_string()));
        assert!(cjk_ratio(&zh) > 0.8);
        assert_eq!(cjk_ratio(CLEAN), 0.0);
    }

    #[test]
    fn repeated_lines_are_flagged() {
        let looped = "the same line again and again\n".repeat(80);
        assert!(assess_text(&looped).flags.contains(&"repeated_lines".to_string()));
    }

    #[test]
    fn score_is_clamped() {
        let r = assess_text(&"\u{FFFD}".repeat(500));
        assert_eq!(r.score, 0.0);
    }
}

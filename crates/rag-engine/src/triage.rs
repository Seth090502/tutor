//! Per-page triage of a PDF text layer into four classes.
//!
//! The rules, applied in order, make the classes exhaustive and mutually exclusive:
//! 1. Fewer than [`MIN_TEXT_CHARS_PER_PAGE`] meaningful characters ("little or no text"):
//!    `scanned` if the page has image objects, otherwise `empty`.
//! 2. Otherwise the text layer is usable. The page is `math_heavy` if math signals make up at
//!    least [`MATH_CHAR_RATIO_MIN`] of its meaningful characters, or if at least
//!    [`SHORT_TOKEN_SHARE_MIN`] of its whitespace tokens are 1-2 characters long.
//! 3. Otherwise it is `born_digital`.

use serde::Serialize;

/// Meaningful characters a page needs before its text layer counts as usable (the `N` in
/// "born digital"). A full textbook page has roughly 1,500-3,000; a chapter opener has ~30.
pub const MIN_TEXT_CHARS_PER_PAGE: usize = 100;

/// Share of meaningful characters that are math signals at or above which a text page is
/// math-heavy. English prose sits near 0.5-1% (hyphens, slashes).
pub const MATH_CHAR_RATIO_MIN: f64 = 0.03;

/// Share of whitespace tokens that are 1-2 characters long at or above which a text page is
/// math-heavy. English prose sits near 20% ("a", "of", "to", "in", ...); extracted equations
/// break into single symbols and push it far higher.
pub const SHORT_TOKEN_SHARE_MIN: f64 = 0.40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageClass {
    BornDigital,
    Scanned,
    MathHeavy,
    Empty,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TriageCounts {
    pub born_digital: u64,
    pub scanned: u64,
    pub math_heavy: u64,
    pub empty: u64,
}

impl TriageCounts {
    pub fn add(&mut self, class: PageClass) {
        match class {
            PageClass::BornDigital => self.born_digital += 1,
            PageClass::Scanned => self.scanned += 1,
            PageClass::MathHeavy => self.math_heavy += 1,
            PageClass::Empty => self.empty += 1,
        }
    }

    pub fn merge(&mut self, other: &TriageCounts) {
        self.born_digital += other.born_digital;
        self.scanned += other.scanned;
        self.math_heavy += other.math_heavy;
        self.empty += other.empty;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PageTextStats {
    pub meaningful_chars: usize,
    pub math_chars: usize,
    pub tokens: usize,
    pub short_tokens: usize,
}

/// A character that carries text: not whitespace, not a control or zero-width format
/// character, not U+FFFD (PDFium's "no Unicode mapping"), and not Private Use Area.
pub fn is_meaningful(c: char) -> bool {
    !(c.is_whitespace()
        || c.is_control()
        || matches!(c, '\u{200B}'..='\u{200F}' | '\u{FEFF}' | '\u{FFFD}' | '\u{E000}'..='\u{F8FF}'))
}

/// Math signals: `= + - / ^ _`, Latin-1 `± × ÷ ¹ ² ³`, Greek and Coptic, super/subscripts,
/// letterlike symbols, and the Unicode math-operator and math-symbol blocks.
pub fn is_math_signal(c: char) -> bool {
    matches!(c,
        '=' | '+' | '-' | '/' | '^' | '_'
        | '\u{00B1}' | '\u{00D7}' | '\u{00F7}'  // ± × ÷
        | '\u{00B2}' | '\u{00B3}' | '\u{00B9}'  // ² ³ ¹
        | '\u{0370}'..='\u{03FF}'               // Greek and Coptic
        | '\u{2070}'..='\u{209F}'               // Superscripts and Subscripts
        | '\u{2100}'..='\u{214F}'               // Letterlike Symbols
        | '\u{2200}'..='\u{22FF}'               // Mathematical Operators
        | '\u{27C0}'..='\u{27EF}'               // Miscellaneous Mathematical Symbols-A
        | '\u{2980}'..='\u{29FF}'               // Miscellaneous Mathematical Symbols-B
        | '\u{2A00}'..='\u{2AFF}'               // Supplemental Mathematical Operators
        | '\u{1D400}'..='\u{1D7FF}'             // Mathematical Alphanumeric Symbols
    )
}

pub fn page_text_stats(text: &str) -> PageTextStats {
    let mut stats = PageTextStats::default();
    for c in text.chars().filter(|&c| is_meaningful(c)) {
        stats.meaningful_chars += 1;
        if is_math_signal(c) {
            stats.math_chars += 1;
        }
    }
    for token in text.split_whitespace() {
        stats.tokens += 1;
        if token.chars().count() <= 2 {
            stats.short_tokens += 1;
        }
    }
    stats
}

/// `has_images` is only called for pages with little text, so text-rich pages never pay for
/// walking their page objects.
pub fn classify(stats: &PageTextStats, has_images: impl FnOnce() -> bool) -> PageClass {
    if stats.meaningful_chars < MIN_TEXT_CHARS_PER_PAGE {
        return if has_images() { PageClass::Scanned } else { PageClass::Empty };
    }
    let math_ratio = stats.math_chars as f64 / stats.meaningful_chars as f64;
    let short_share = if stats.tokens == 0 {
        0.0
    } else {
        stats.short_tokens as f64 / stats.tokens as f64
    };
    if math_ratio >= MATH_CHAR_RATIO_MIN || short_share >= SHORT_TOKEN_SHARE_MIN {
        PageClass::MathHeavy
    } else {
        PageClass::BornDigital
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROSE: &str = "The mitochondrion is the organelle in which most of the chemical \
        energy of a cell is released. Glucose is first broken down in the cytoplasm, and the \
        products then enter the mitochondrion, where a series of reactions transfers their \
        energy to carrier molecules. Students often confuse respiration with breathing.";

    #[test]
    fn meaningful_excludes_whitespace_controls_and_unmapped_glyphs() {
        assert!(is_meaningful('a'));
        assert!(is_meaningful('='));
        assert!(is_meaningful('\u{03C0}'));
        for c in [' ', '\n', '\r', '\t', '\u{00A0}', '\u{0002}', '\u{FFFD}', '\u{E001}', '\u{200B}'] {
            assert!(!is_meaningful(c), "{c:?} should not be meaningful");
        }
    }

    #[test]
    fn math_signals_cover_the_documented_sets() {
        for c in "=+-/^_±×÷²³¹".chars() {
            assert!(is_math_signal(c), "{c:?}");
        }
        // Greek, super/subscripts, letterlike, operators, supplemental operators, math alphanumerics.
        for c in ['α', 'Ω', '⁰', '₂', 'ℝ', '∑', '∫', '≤', '⨁', '⟨', '⦃', '𝑥'] {
            assert!(is_math_signal(c), "{c:?}");
        }
        for c in ['a', 'Z', '7', '.', ',', '(', ')', '—', '"'] {
            assert!(!is_math_signal(c), "{c:?}");
        }
    }

    #[test]
    fn stats_count_meaningful_math_and_short_tokens() {
        let s = page_text_stats("x = 2 +\r\n abc \u{FFFD} de");
        // tokens: x = 2 + abc \u{FFFD} de -> 7; short (1-2 chars): x = 2 + \u{FFFD} de -> 6
        assert_eq!(s.tokens, 7);
        assert_eq!(s.short_tokens, 6);
        // meaningful: x = 2 + a b c d e -> 9 (U+FFFD excluded)
        assert_eq!(s.meaningful_chars, 9);
        assert_eq!(s.math_chars, 2);
    }

    #[test]
    fn stats_of_empty_text_are_zero() {
        assert_eq!(page_text_stats(""), PageTextStats::default());
        assert_eq!(page_text_stats(" \r\n\t "), PageTextStats::default());
    }

    #[test]
    fn prose_page_is_born_digital() {
        let stats = page_text_stats(PROSE);
        assert!(stats.meaningful_chars >= MIN_TEXT_CHARS_PER_PAGE);
        assert_eq!(classify(&stats, || panic!("images not needed")), PageClass::BornDigital);
    }

    #[test]
    fn little_text_with_images_is_scanned() {
        assert_eq!(classify(&page_text_stats(""), || true), PageClass::Scanned);
        // A scanner-stamped page number is still "little text".
        assert_eq!(classify(&page_text_stats("Page 12"), || true), PageClass::Scanned);
    }

    #[test]
    fn little_text_without_images_is_empty() {
        assert_eq!(classify(&page_text_stats(""), || false), PageClass::Empty);
        assert_eq!(classify(&page_text_stats("Chapter 3"), || false), PageClass::Empty);
    }

    #[test]
    fn text_threshold_is_inclusive() {
        let at = PageTextStats { meaningful_chars: MIN_TEXT_CHARS_PER_PAGE, tokens: 20, ..Default::default() };
        let below = PageTextStats { meaningful_chars: MIN_TEXT_CHARS_PER_PAGE - 1, ..at };
        assert_eq!(classify(&at, || true), PageClass::BornDigital);
        assert_eq!(classify(&below, || true), PageClass::Scanned);
    }

    #[test]
    fn math_char_ratio_threshold_is_inclusive() {
        let base = PageTextStats { meaningful_chars: 1000, math_chars: 0, tokens: 200, short_tokens: 40 };
        let at = PageTextStats { math_chars: (1000.0 * MATH_CHAR_RATIO_MIN).round() as usize, ..base };
        let below = PageTextStats { math_chars: at.math_chars - 1, ..base };
        assert_eq!(classify(&at, || false), PageClass::MathHeavy);
        assert_eq!(classify(&below, || false), PageClass::BornDigital);
    }

    #[test]
    fn short_token_share_threshold_is_inclusive() {
        let base = PageTextStats { meaningful_chars: 1000, math_chars: 0, tokens: 100, short_tokens: 0 };
        let at = PageTextStats { short_tokens: (100.0 * SHORT_TOKEN_SHARE_MIN).round() as usize, ..base };
        let below = PageTextStats { short_tokens: at.short_tokens - 1, ..base };
        assert_eq!(classify(&at, || false), PageClass::MathHeavy);
        assert_eq!(classify(&below, || false), PageClass::BornDigital);
    }

    #[test]
    fn extracted_equations_are_math_heavy() {
        let page = "f(x) = a x^2 + b x + c , a ≠ 0\n x_1 = ( -b + √Δ ) / 2a , Δ = b^2 - 4ac\n\
                    ∑ k = n ( n + 1 ) / 2 , k = 1 .. n\n ∫ x^n dx = x^(n+1) / (n+1) + C\n\
                    π r^2 = A , 2 π r = C , Ω = 2 π f";
        let stats = page_text_stats(page);
        assert!(stats.meaningful_chars >= MIN_TEXT_CHARS_PER_PAGE);
        assert_eq!(classify(&stats, || false), PageClass::MathHeavy);
    }

    #[test]
    fn scanned_check_is_skipped_for_text_pages() {
        let stats = page_text_stats(PROSE);
        let mut called = false;
        classify(&stats, || {
            called = true;
            true
        });
        assert!(!called);
    }

    #[test]
    fn counts_add_and_merge() {
        let mut a = TriageCounts::default();
        a.add(PageClass::BornDigital);
        a.add(PageClass::BornDigital);
        a.add(PageClass::Scanned);
        let mut b = TriageCounts::default();
        b.add(PageClass::MathHeavy);
        b.add(PageClass::Empty);
        a.merge(&b);
        assert_eq!(a, TriageCounts { born_digital: 2, scanned: 1, math_heavy: 1, empty: 1 });
    }
}

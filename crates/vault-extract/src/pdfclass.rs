//! PDF classification: from cheap signals to a route. Born-digital papers and books go to native extraction;
//! scans and maths-heavy documents wait for the GPU tools (SPEC section 8). The thresholds are fields of
//! [`Thresholds`] so they can be tuned against a labelled sample without touching the logic.

use serde::{Deserialize, Serialize};

use crate::pdfscan::PdfScan;
use crate::quality::is_cjk;

/// What was measured on one page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PageSignals {
    /// Non-whitespace characters in the text layer.
    pub chars: u32,
    /// Share of the page area covered by images, 0 to 1.
    pub image_coverage: f32,
    /// Characters that are mathematical symbols (operators, arrows, math alphanumerics, isolated Greek).
    pub math_chars: u32,
    pub cjk_chars: u32,
}

/// Everything the classifier looks at. `pages` can be filled from the raw scan alone; the per-page measurements
/// need a PDF engine and make the decision sharper.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PdfSignals {
    pub pages: u32,
    pub encrypted: bool,
    pub has_page_labels: bool,
    pub page_signals: Vec<PageSignals>,
    /// All base font names, and the mathematical ones among them.
    pub fonts: Vec<String>,
    pub math_fonts: Vec<String>,
    pub image_objects: u32,
    /// The document metadata or text mentions an ISBN (a book) or a DOI or arXiv id (a paper).
    pub has_isbn: bool,
    pub has_paper_id: bool,
}

impl PdfSignals {
    /// Signals from the byte scan only: no per-page measurements.
    pub fn from_scan(scan: &PdfScan) -> Self {
        Self {
            pages: scan.pages.unwrap_or(0),
            encrypted: scan.encrypted,
            has_page_labels: scan.has_page_labels,
            page_signals: Vec::new(),
            fonts: scan.fonts.clone(),
            math_fonts: scan.math_fonts.clone(),
            image_objects: scan.image_objects,
            has_isbn: false,
            has_paper_id: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfKind {
    Paper,
    Book,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuReason {
    /// Most pages have no text layer: OCR is needed.
    Scanned,
    /// Dense mathematics whose text layer cannot give usable LaTeX.
    MathHeavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "route", content = "reason")]
pub enum PdfRoute {
    /// Extract the text layer directly.
    Native,
    /// Park the document in `waiting_gpu`.
    WaitingGpu(GpuReason),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfClassification {
    pub kind: PdfKind,
    pub route: PdfRoute,
    /// Pages with no usable text layer over all pages.
    pub scanned_page_ratio: f32,
    /// Mathematical characters over all characters.
    pub math_ratio: f32,
    pub cjk_ratio: f32,
    /// Mean characters per page.
    pub chars_per_page: f32,
    /// `partial_scan`, `math_fonts`, `cjk`, `no_text_layer`, `sparse_text`, `encrypted`.
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// A page with fewer text characters than this has no usable text layer.
    pub min_chars_per_page: u32,
    /// Image coverage that makes a text-less page a scan.
    pub scan_image_coverage: f32,
    /// Share of scanned pages above which the document goes to OCR.
    pub scanned_ratio: f32,
    /// Share of scanned pages above which a `partial_scan` flag is raised.
    pub partial_scan_ratio: f32,
    /// Maths share of characters above which the document counts as maths-heavy (with maths fonts present).
    pub math_ratio: f32,
    /// Maths fonts required for the maths-heavy decision.
    pub math_fonts_min: usize,
    /// Pages above which a document is a book.
    pub book_pages: u32,
    /// CJK share above which the `cjk` flag is raised.
    pub cjk_ratio: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_chars_per_page: 40,
            scan_image_coverage: 0.5,
            scanned_ratio: 0.5,
            partial_scan_ratio: 0.1,
            math_ratio: 0.03,
            math_fonts_min: 2,
            book_pages: 120,
            cjk_ratio: 0.3,
        }
    }
}

/// Classify with the default thresholds.
pub fn classify(s: &PdfSignals) -> PdfClassification {
    classify_with(s, &Thresholds::default())
}

pub fn classify_with(s: &PdfSignals, t: &Thresholds) -> PdfClassification {
    let mut flags = Vec::new();
    let pages = if s.page_signals.is_empty() { s.pages } else { s.page_signals.len() as u32 }.max(1);

    // Scanned pages: no text layer. With per-page data, a page is scanned when it has almost no text and an
    // image covers it; without, the raw scan decides all-or-nothing from fonts and image objects.
    let (scanned_pages, total_chars, math_chars, cjk_chars) = if s.page_signals.is_empty() {
        let scanned = if s.fonts.is_empty() && s.image_objects > 0 { pages } else { 0 };
        (scanned, 0u64, 0u64, 0u64)
    } else {
        let scanned = s
            .page_signals
            .iter()
            .filter(|p| p.chars < t.min_chars_per_page && p.image_coverage >= t.scan_image_coverage)
            .count() as u32;
        (
            scanned,
            s.page_signals.iter().map(|p| p.chars as u64).sum(),
            s.page_signals.iter().map(|p| p.math_chars as u64).sum(),
            s.page_signals.iter().map(|p| p.cjk_chars as u64).sum(),
        )
    };
    let scanned_ratio = scanned_pages as f32 / pages as f32;
    let chars_per_page = total_chars as f32 / pages as f32;
    let math_ratio = if total_chars == 0 { 0.0 } else { math_chars as f32 / total_chars as f32 };
    let cjk_ratio = if total_chars == 0 { 0.0 } else { cjk_chars as f32 / total_chars as f32 };

    if s.encrypted {
        flags.push("encrypted".to_string());
    }
    if !s.page_signals.is_empty() && total_chars == 0 {
        flags.push("no_text_layer".to_string());
    } else if !s.page_signals.is_empty() && chars_per_page < t.min_chars_per_page as f32 {
        flags.push("sparse_text".to_string());
    }
    if scanned_ratio >= t.partial_scan_ratio && scanned_ratio < t.scanned_ratio {
        flags.push("partial_scan".to_string());
    }
    if s.math_fonts.len() >= t.math_fonts_min {
        flags.push("math_fonts".to_string());
    }
    if cjk_ratio >= t.cjk_ratio {
        flags.push("cjk".to_string());
    }

    let route = if scanned_ratio >= t.scanned_ratio || (!s.page_signals.is_empty() && total_chars == 0) {
        PdfRoute::WaitingGpu(GpuReason::Scanned)
    } else if math_ratio >= t.math_ratio && s.math_fonts.len() >= t.math_fonts_min {
        PdfRoute::WaitingGpu(GpuReason::MathHeavy)
    } else {
        PdfRoute::Native
    };

    let kind = if pages > t.book_pages || (s.has_isbn && pages > t.book_pages / 2 && !s.has_paper_id) {
        PdfKind::Book
    } else {
        PdfKind::Paper
    };
    PdfClassification { kind, route, scanned_page_ratio: scanned_ratio, math_ratio, cjk_ratio, chars_per_page, flags }
}

/// Characters of `text` that are mathematical symbols: operators, arrows, math alphanumerics, letterlike symbols,
/// super- and subscripts, and Greek letters that stand alone (a Greek word is not mathematics).
pub fn math_char_count(text: &str) -> u32 {
    let chars: Vec<char> = text.chars().collect();
    let is_greek = |c: char| matches!(c as u32, 0x0370..=0x03FF | 0x1F00..=0x1FFF);
    let mut n = 0u32;
    for (i, &c) in chars.iter().enumerate() {
        let u = c as u32;
        let math = matches!(u, 0x2190..=0x22FF | 0x27C0..=0x27EF | 0x2980..=0x2AFF | 0x1D400..=0x1D7FF | 0x2100..=0x214F | 0x2070..=0x209F | 0x00B1 | 0x00D7 | 0x00F7 | 0x2032..=0x2034 | 0x2016 | 0x2308..=0x230B);
        if math {
            n += 1;
        } else if is_greek(c) {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            if !prev.is_some_and(is_greek) && !next.is_some_and(is_greek) {
                n += 1;
            }
        }
    }
    n
}

/// Non-whitespace CJK characters in `text`.
pub fn cjk_char_count(text: &str) -> u32 {
    text.chars().filter(|c| is_cjk(*c)).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(chars: u32, image: f32, math: u32) -> PageSignals {
        PageSignals { chars, image_coverage: image, math_chars: math, cjk_chars: 0 }
    }

    fn signals(pages: Vec<PageSignals>) -> PdfSignals {
        PdfSignals { pages: pages.len() as u32, page_signals: pages, ..Default::default() }
    }

    #[test]
    fn born_digital_paper_is_native() {
        let c = classify(&signals(vec![page(2000, 0.05, 20); 12]));
        assert_eq!(c.route, PdfRoute::Native);
        assert_eq!(c.kind, PdfKind::Paper);
        assert!(c.flags.is_empty(), "{:?}", c.flags);
        assert!((c.chars_per_page - 2000.0).abs() < 1e-3);
    }

    #[test]
    fn scanned_documents_wait_for_the_gpu() {
        let c = classify(&signals(vec![page(0, 0.95, 0); 30]));
        assert_eq!(c.route, PdfRoute::WaitingGpu(GpuReason::Scanned));
        assert!(c.flags.contains(&"no_text_layer".to_string()));
        // A scan that was OCRed already has text and routes native.
        let c = classify(&signals(vec![page(1800, 0.95, 0); 30]));
        assert_eq!(c.route, PdfRoute::Native);
    }

    #[test]
    fn mostly_scanned_goes_to_ocr_and_partially_scanned_is_flagged() {
        let mut pages = vec![page(2000, 0.0, 0); 6];
        pages.extend(vec![page(3, 0.9, 0); 6]);
        assert_eq!(classify(&signals(pages)).route, PdfRoute::WaitingGpu(GpuReason::Scanned));
        let mut pages = vec![page(2000, 0.0, 0); 18];
        pages.extend(vec![page(3, 0.9, 0); 2]);
        let c = classify(&signals(pages));
        assert_eq!(c.route, PdfRoute::Native);
        assert!(c.flags.contains(&"partial_scan".to_string()), "{:?}", c.flags);
        // A mostly white page with no image is a blank page, not a scan.
        let c = classify(&signals(vec![page(2000, 0.0, 0), page(0, 0.0, 0)]));
        assert_eq!(c.scanned_page_ratio, 0.0);
    }

    #[test]
    fn math_heavy_needs_symbols_and_math_fonts() {
        let mut s = signals(vec![page(1000, 0.0, 120); 10]);
        s.math_fonts = vec!["CMMI10".into(), "CMSY10".into()];
        let c = classify(&s);
        assert_eq!(c.route, PdfRoute::WaitingGpu(GpuReason::MathHeavy));
        assert!(c.flags.contains(&"math_fonts".to_string()));
        // The same symbol share without maths fonts (a Greek-language text, say) stays native.
        s.math_fonts.clear();
        assert_eq!(classify(&s).route, PdfRoute::Native);
        // Maths fonts with only a trickle of symbols are an ordinary paper.
        let mut s = signals(vec![page(1000, 0.0, 5); 10]);
        s.math_fonts = vec!["CMMI10".into(), "CMSY10".into()];
        assert_eq!(classify(&s).route, PdfRoute::Native);
    }

    #[test]
    fn books_by_page_count_and_isbn() {
        let c = classify(&signals(vec![page(2500, 0.0, 0); 300]));
        assert_eq!(c.kind, PdfKind::Book);
        assert_eq!(classify(&signals(vec![page(2500, 0.0, 0); 120])).kind, PdfKind::Paper);
        let mut s = signals(vec![page(2500, 0.0, 0); 80]);
        s.has_isbn = true;
        assert_eq!(classify(&s).kind, PdfKind::Book);
        s.has_paper_id = true;
        assert_eq!(classify(&s).kind, PdfKind::Paper);
        let mut s = signals(vec![page(2500, 0.0, 0); 20]);
        s.has_isbn = true;
        assert_eq!(classify(&s).kind, PdfKind::Paper);
    }

    #[test]
    fn cjk_flag() {
        let mut p = page(1000, 0.0, 0);
        p.cjk_chars = 800;
        let c = classify(&signals(vec![p; 5]));
        assert!(c.flags.contains(&"cjk".to_string()));
        assert!((c.cjk_ratio - 0.8).abs() < 1e-5);
    }

    #[test]
    fn scan_only_signals_use_fonts_and_images() {
        let scan = PdfScan { pages: Some(10), fonts: vec![], image_objects: 10, ..Default::default() };
        let c = classify(&PdfSignals::from_scan(&scan));
        assert_eq!(c.route, PdfRoute::WaitingGpu(GpuReason::Scanned));
        let scan =
            PdfScan { pages: Some(10), fonts: vec!["Times-Roman".into()], image_objects: 3, ..Default::default() };
        assert_eq!(classify(&PdfSignals::from_scan(&scan)).route, PdfRoute::Native);
    }

    #[test]
    fn thresholds_are_tunable() {
        let s = signals(vec![page(1000, 0.0, 0); 10]);
        let strict = Thresholds { min_chars_per_page: 5000, ..Thresholds::default() };
        assert!(classify_with(&s, &strict).flags.contains(&"sparse_text".to_string()));
    }

    #[test]
    fn math_character_counting() {
        assert_eq!(math_char_count("x \u{2208} A and \u{03B1} < \u{03B2} \u{2264} \u{1D465}"), 5);
        // A Greek word is not mathematics.
        assert_eq!(math_char_count("\u{03BA}\u{03B1}\u{03BB}\u{03B7}\u{03BC}\u{03AD}\u{03C1}\u{03B1}"), 0);
        assert_eq!(math_char_count("plain ASCII text"), 0);
        assert_eq!(cjk_char_count("abc \u{4E2D}\u{6587} \u{3053}\u{3093}"), 4);
    }
}

//! PDF text through PDFium: positioned characters in, canonical-ready Markdown out, with the printed page labels
//! of `/PageLabels`, columns read in order, headings, paragraphs, lists and references (see `pdflayout`).
//!
//! PDFium is loaded at run time from a DLL (nothing is linked), once per process. It is native code: a malicious
//! PDF can crash or hang it, so a caller that needs isolation runs conversion in a child process with a timeout.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pdfium_render::prelude::*;

use crate::ConvertOptions;
use crate::error::{ExtractError, Result};
use crate::ids;
use crate::pdfclass::{PageSignals, PdfClassification, PdfSignals, cjk_char_count, classify, math_char_count};
use crate::pdflayout::{Glyph, PageInput, PageLines, assemble, infer_labels, layout_page};
use crate::pdfscan;
use crate::types::{Extracted, Metadata, tool_string};

/// Environment variable naming the PDFium library to load.
pub const LIBRARY_ENV: &str = "VAULT_PDFIUM_DLL";

static PDFIUM: OnceLock<std::result::Result<Pdfium, String>> = OnceLock::new();

/// Bind PDFium from `path`. The first call decides: later calls with another path succeed without rebinding.
pub fn init(path: &Path) -> Result<()> {
    let r = PDFIUM.get_or_init(|| {
        Pdfium::bind_to_library(path)
            .map(Pdfium::new)
            .map_err(|e| format!("could not load PDFium from {}: {e:?}", path.display()))
    });
    match r {
        Ok(_) => Ok(()),
        Err(e) => Err(ExtractError::ToolUnavailable(e.clone())),
    }
}

/// Bind PDFium from `VAULT_PDFIUM_DLL`, or `pdfium.dll` / `libpdfium.so` beside the executable.
pub fn init_from_environment() -> Result<()> {
    if let Some(p) = std::env::var_os(LIBRARY_ENV).map(PathBuf::from) {
        return init(&p);
    }
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    for name in ["pdfium.dll", "libpdfium.so", "libpdfium.dylib"] {
        if let Some(p) = exe_dir.as_ref().map(|d| d.join(name)).filter(|p| p.exists()) {
            return init(&p);
        }
    }
    Err(ExtractError::ToolUnavailable(format!("PDFium is not configured: set {LIBRARY_ENV} to the library path")))
}

/// Whether PDFium is bound in this process.
#[doc(hidden)]
pub fn is_bound() -> bool {
    matches!(PDFIUM.get(), Some(Ok(_)))
}

fn pdfium() -> Result<&'static Pdfium> {
    match PDFIUM.get() {
        Some(Ok(p)) => Ok(p),
        Some(Err(e)) => Err(ExtractError::ToolUnavailable(e.clone())),
        None => {
            init_from_environment()?;
            match PDFIUM.get() {
                Some(Ok(p)) => Ok(p),
                _ => Err(ExtractError::ToolUnavailable("PDFium failed to initialise".into())),
            }
        }
    }
}

fn open_error(e: PdfiumError) -> ExtractError {
    match e {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError)
        | PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::SecurityError) => {
            ExtractError::Protected("the PDF needs a password".to_string())
        }
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError) => {
            ExtractError::corrupt("pdf", "PDFium could not parse the file")
        }
        other => ExtractError::convert("pdf", format!("{other:?}")),
    }
}

/// The result of converting a PDF.
#[derive(Debug, Clone)]
pub struct PdfConversion {
    pub extracted: Extracted,
    pub signals: PdfSignals,
    pub classification: PdfClassification,
}

/// Classification signals without building any text: page count, per-page text density, image coverage and
/// maths and CJK character counts, plus the fonts from the byte scan.
pub fn analyze(bytes: &[u8]) -> Result<(PdfSignals, PdfClassification)> {
    let pdfium = pdfium()?;
    let doc = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(open_error)?;
    let mut s = signals_from_scan(bytes);
    let mut pages = Vec::new();
    for page in doc.pages().iter() {
        let text = page.text().map(|t| t.all()).unwrap_or_default();
        pages.push(page_signals(&page, &text));
    }
    s.pages = pages.len() as u32;
    s.page_signals = pages;
    let c = classify(&s);
    Ok((s, c))
}

fn signals_from_scan(bytes: &[u8]) -> PdfSignals {
    let scan = pdfscan::scan(bytes);
    PdfSignals::from_scan(&scan)
}

fn page_signals(page: &PdfPage, text: &str) -> PageSignals {
    let chars = text.chars().filter(|c| !c.is_whitespace()).count() as u32;
    let area = (page.width().value * page.height().value).max(1.0);
    let mut covered = 0.0f32;
    for obj in page.objects().iter() {
        if obj.object_type() == PdfPageObjectType::Image
            && let Ok(b) = obj.bounds()
        {
            covered += (b.width().value * b.height().value).abs();
        }
    }
    PageSignals {
        chars,
        image_coverage: (covered / area).min(1.0),
        math_chars: math_char_count(text),
        cjk_chars: cjk_char_count(text),
    }
}

pub fn convert(bytes: &[u8], _opts: &ConvertOptions) -> Result<PdfConversion> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let pdfium = pdfium()?;
    let doc = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(open_error)?;
    let mut warnings: Vec<String> = Vec::new();

    let mut inputs: Vec<PageInput> = Vec::new();
    let mut psigs: Vec<PageSignals> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for (i, page) in doc.pages().iter().enumerate() {
        let text = match page.text() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(page = i + 1, error = ?e, "PDF page has no readable text layer");
                warnings.push(format!("page {}: no text layer ({e:?})", i + 1));
                inputs.push(PageInput { width: page.width().value, height: page.height().value, ..Default::default() });
                psigs.push(page_signals(&page, ""));
                texts.push(String::new());
                continue;
            }
        };
        let all = text.all();
        psigs.push(page_signals(&page, &all));
        let mut input = glyphs_of(&page, &text);
        input.label = page.label().map(str::to_string).filter(|l| !l.is_empty());
        inputs.push(input);
        texts.push(all);
    }

    // Metadata: document properties, then identifiers found in the text.
    let mut meta = read_metadata(&doc);
    let first_text: String = texts.iter().take(3).cloned().collect::<Vec<_>>().join("\n");
    let rotated: String = inputs.iter().take(2).map(|p| p.rotated_text.as_str()).collect::<Vec<_>>().join(" ");
    meta.arxiv_id = meta.arxiv_id.take().or_else(|| ids::find_arxiv(&rotated)).or_else(|| ids::find_arxiv(&first_text));
    meta.doi = meta.doi.take().or_else(|| ids::find_doi(&first_text));
    let isbns: Vec<String> = ids::find_isbns(&texts.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
    let has_isbn = !isbns.is_empty() && (texts.len() > 60 || !first_text.to_lowercase().contains("arxiv"));
    if has_isbn {
        meta.isbn = isbns;
    }

    let has_labels = inputs.iter().any(|p| p.label.is_some());
    let page_lines: Vec<PageLines> =
        inputs.iter().map(|p| PageLines { label: p.label.clone(), lines: layout_page(p) }).collect();
    let mut assembled = assemble(page_lines, meta);

    // No /PageLabels: look for printed page numbers in the headers and footers.
    if !has_labels {
        let inferred = infer_labels(&assembled.printed_numbers);
        if inferred.iter().any(Option::is_some) {
            for (mark, label) in assembled.extracted.pages.iter_mut().zip(inferred) {
                mark.label = label;
            }
        }
    }

    let mut signals = signals_from_scan(bytes);
    signals.pages = psigs.len() as u32;
    signals.page_signals = psigs;
    signals.has_page_labels = has_labels;
    signals.has_isbn = has_isbn;
    signals.has_paper_id =
        assembled.extracted.metadata.arxiv_id.is_some() || assembled.extracted.metadata.doi.is_some();
    let classification = classify(&signals);

    let mut ex = assembled.extracted;
    ex.tool = tool_string("pdf pdfium");
    ex.encoding = Some("utf-8".to_string());
    ex.warnings.append(&mut warnings);
    if ex.markdown.trim().is_empty() {
        ex.warnings.push("the PDF has no extractable text layer".to_string());
    }
    Ok(PdfConversion { extracted: ex, signals, classification })
}

/// One line per laid-out text line of page `index` (0-based) with its geometry, for tuning the layout rules.
#[doc(hidden)]
pub fn layout_lines_debug(bytes: &[u8], index: usize) -> Result<Vec<String>> {
    let doc = pdfium()?.load_pdf_from_byte_slice(bytes, None).map_err(open_error)?;
    let page = doc.pages().get(index as i32).map_err(|e| ExtractError::convert("pdf", format!("{e:?}")))?;
    let text = page.text().map_err(|e| ExtractError::convert("pdf", format!("{e:?}")))?;
    let input = glyphs_of(&page, &text);
    Ok(layout_page(&input)
        .iter()
        .map(|l| {
            format!(
                "x {:6.1}..{:6.1} y {:6.1}..{:6.1} size {:4.1} bold {} col {} furn {} | {}",
                l.l, l.r, l.bottom, l.top, l.size, l.bold as u8, l.column, l.furniture as u8, l.text
            )
        })
        .collect())
}

fn read_metadata(doc: &PdfDocument) -> Metadata {
    let mut m = Metadata::default();
    let md = doc.metadata();
    let get =
        |t: PdfDocumentMetadataTagType| md.get(t).map(|tag| tag.value().trim().to_string()).filter(|v| !v.is_empty());
    m.title = get(PdfDocumentMetadataTagType::Title).filter(|t| plausible_title(t));
    if let Some(a) = get(PdfDocumentMetadataTagType::Author) {
        m.authors = a.split([';', '\n']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    }
    if let Some(k) = get(PdfDocumentMetadataTagType::Keywords) {
        m.keywords = k.split([',', ';']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
    }
    m.abstract_text = None;
    if let Some(d) = get(PdfDocumentMetadataTagType::CreationDate) {
        // "D:20190402101000+00'00'"
        m.year = d
            .trim_start_matches("D:")
            .get(..4)
            .and_then(|y| y.parse::<i32>().ok())
            .filter(|y| (1900..=2100).contains(y));
    }
    m
}

/// Bold and italic from a PDF font name (`ABCDEF+TimesNewRomanPS-BoldMT`, `CMBX12`, `NimbusRomNo9L-Medi`).
/// Names only: the weight PDFium reports is unreliable for Type 1 fonts without a descriptor. Math italic fonts
/// (`CMMI`) are not text italic.
pub(crate) fn font_style(raw: &str) -> (bool, bool) {
    let name = raw.rsplit('+').next().unwrap_or(raw).to_lowercase();
    let bold = name.contains("bold")
        || name.contains("black")
        || name.contains("heavy")
        || name.contains("demi")
        || name.ends_with("-medi")
        || name.ends_with("-bd")
        || name.ends_with("bd")
        || name.starts_with("cmbx")
        || name.starts_with("cmb")
        || name.starts_with("cmssbx")
        || name.starts_with("cmbsy")
        || name.starts_with("cmmib");
    let italic = (name.contains("italic")
        || name.contains("oblique")
        || name.starts_with("cmti")
        || name.starts_with("cmsl")
        || name.ends_with("-ital")
        || name.ends_with("-it"))
        && !name.starts_with("cmmi");
    (bold, italic)
}

/// Rejects the placeholder titles authoring tools leave behind.
fn plausible_title(t: &str) -> bool {
    let l = t.to_lowercase();
    t.chars().count() >= 4
        && !l.starts_with("microsoft word")
        && !l.starts_with("microsoft powerpoint")
        && !l.starts_with("untitled")
        && !l.ends_with(".pdf")
        && !l.ends_with(".doc")
        && !l.ends_with(".docx")
        && !l.ends_with(".tex")
        && !l.ends_with(".dvi")
        && !l.starts_with("pdf document")
        && !t.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '_')
}

/// Positioned glyphs for a page, in the page's displayed orientation.
fn glyphs_of(page: &PdfPage, text: &PdfPageText) -> PageInput {
    // PDFium reports character boxes in the displayed orientation, so rotated pages need no transform.
    let (w, h) = (page.width().value, page.height().value);
    let mut input = PageInput { width: w, height: h, ..Default::default() };
    let mut last: Option<(f32, f32, f32, f32, f32)> = None; // r, b, t, size of the previous glyph
    for c in text.chars().iter() {
        let Some(ch) = c.unicode_char() else { continue };
        // PDFium synthesises a space where the text layer has a gap; take those as word separators and let the
        // geometry decide only where PDFium did not.
        if c.is_generated().unwrap_or(false) {
            if ch == ' '
                && let Some((r, b, t, size, _)) = last
            {
                input.glyphs.push(Glyph { ch: ' ', l: r, r, b, t, size, bold: false, italic: false });
            }
            continue;
        }
        if matches!(ch, '\r' | '\n' | '\u{fffe}' | '\u{ffff}') {
            continue;
        }
        // PDFium marks a line-end hyphen with U+0002.
        let ch = if ch == '\u{2}' || ch == '\u{ad}' { '-' } else { ch };
        if ch.is_control() && !ch.is_whitespace() {
            continue;
        }
        let Ok(b) = c.loose_bounds() else { continue };
        let size = c.scaled_font_size().value;
        let angle = c.angle_degrees().unwrap_or(0.0);
        let a = ((angle % 360.0) + 360.0) % 360.0;
        if !(a < 5.0 || a > 355.0) {
            // Rotated text (a margin stamp): keep for metadata, leave out of the body.
            if !ch.is_whitespace() || input.rotated_text.ends_with(|x: char| !x.is_whitespace()) {
                input.rotated_text.push(ch);
            }
            continue;
        }
        let (l, r, bt, tp) = (b.left().value, b.right().value, b.bottom().value, b.top().value);
        let (bold, italic) = font_style(&c.font_name());
        input.glyphs.push(Glyph { ch, l, r, b: bt, t: tp, size, bold, italic });
        last = Some((r, bt, tp, size, 0.0));
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdfclass::{GpuReason, PdfRoute};
    use crate::testpdf::*;

    // --- tests -------------------------------------------------------------------------------------------------

    #[test]
    fn page_labels_headings_paragraphs_and_metadata() {
        need_pdfium!();
        let mut ops = vec![
            op("F2", 18.0, 72.0, 740.0, "A Study of Synthetic Things"),
            op("F2", 12.0, 72.0, 700.0, "1 Introduction"),
        ];
        ops.extend(body(
            &["This paragraph has a hyphen-", "ated word and runs to the end.", "It has two sentences."],
            680.0,
        ));
        ops.extend(body(&["A second paragraph starts after a gap."], 620.0));
        let p2 = body(&["Second page text continues here with more words."], 700.0);
        let pdf = build_pdf(
            &[page(ops), page(p2)],
            Some("0 << /S /r >> 1 << /S /D /St 214 /P (A-) >>"),
            Some("Real Paper Title"),
        );
        let r = convert_pdf(&pdf);
        let ex = &r.extracted;
        assert!(ex.markdown.contains("# A Study of Synthetic Things\n\n## 1 Introduction\n\nThis paragraph has a hyphenated word and runs to the end. It has two sentences.\n\nA second paragraph starts after a gap.\n"), "{}", ex.markdown);
        assert!(ex.markdown.contains("Second page text continues here with more words."), "{}", ex.markdown);
        let labels: Vec<_> = ex.pages.iter().map(|p| (p.label.clone().unwrap_or_default(), p.index)).collect();
        assert_eq!(labels, vec![("i".to_string(), 1), ("A-214".to_string(), 2)]);
        let lines: Vec<&str> = ex.markdown.lines().collect();
        assert_eq!(lines[ex.pages[0].line as usize - 1], "# A Study of Synthetic Things");
        assert!(lines[ex.pages[1].line as usize - 1].starts_with("Second page text"));
        assert_eq!(ex.metadata.title.as_deref(), Some("Real Paper Title"));
        assert_eq!(ex.metadata.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(ex.metadata.year, Some(2019));
        assert!(ex.tool.ends_with("pdf pdfium"));
        assert_eq!(r.classification.route, PdfRoute::Native);
        assert!(r.signals.has_page_labels);
    }

    #[test]
    fn two_columns_are_read_in_order_whatever_the_stream_order() {
        need_pdfium!();
        let mut ops = Vec::new();
        for i in 0..6 {
            let y = 700.0 - i as f32 * 12.0;
            ops.push(op("F1", 10.0, 72.0, y, &format!("left column sentence number {i} with words")));
            ops.push(op("F1", 10.0, 320.0, y, &format!("right column sentence number {i} with words")));
        }
        let r = convert_pdf(&build_pdf(&[page(ops)], None, None));
        let md = &r.extracted.markdown;
        let left_last = md.find("left column sentence number 5").unwrap();
        let right_first = md.find("right column sentence number 0").unwrap();
        assert!(left_last < right_first, "{md}");
        assert!(md.find("left column sentence number 0").unwrap() < md.find("left column sentence number 1").unwrap());
    }

    #[test]
    fn two_columns_in_column_by_column_stream_order_stay_in_order() {
        need_pdfium!();
        let mut ops = Vec::new();
        for i in 0..6 {
            ops.push(op(
                "F1",
                10.0,
                72.0,
                700.0 - i as f32 * 12.0,
                &format!("left column sentence number {i} with words"),
            ));
        }
        for i in 0..6 {
            ops.push(op(
                "F1",
                10.0,
                320.0,
                700.0 - i as f32 * 12.0,
                &format!("right column sentence number {i} with words"),
            ));
        }
        let r = convert_pdf(&build_pdf(&[page(ops)], None, None));
        let md = &r.extracted.markdown;
        assert!(
            md.find("left column sentence number 5").unwrap() < md.find("right column sentence number 0").unwrap(),
            "{md}"
        );
    }

    #[test]
    fn superscripts_running_headers_and_printed_page_numbers() {
        need_pdfium!();
        let mut pages = Vec::new();
        for i in 0..8 {
            let mut ops = vec![
                op("F1", 9.0, 72.0, 765.0, "Journal of Synthetic Results"),
                op("F1", 9.0, 300.0, 30.0, &format!("{}", 100 + i)),
            ];
            // The exponent is part of the same text run, raised with a text rise.
            ops.push(op("RAW", 0.0, 0.0, 0.0, &format!("BT /F1 10 Tf 72 600 Td (Page {i} body text about energy E = m c) Tj /F1 6 Tf 4 Ts (2) Tj 0 Ts /F1 10 Tf ( holds.) Tj ET")));
            pages.push(page(ops));
        }
        let r = convert_pdf(&build_pdf(&pages, None, None));
        let ex = &r.extracted;
        assert!(!ex.markdown.contains("Journal of Synthetic"), "{}", ex.markdown);
        assert!(!ex.markdown.contains("\n100\n") && !ex.markdown.contains("\n107\n"), "{}", ex.markdown);
        let labels: Vec<_> = ex.pages.iter().map(|p| p.label.clone().unwrap_or_default()).collect();
        assert_eq!(labels, (100..108).map(|n| n.to_string()).collect::<Vec<_>>());
        assert!(ex.markdown.contains("m c^2 holds."), "{}", ex.markdown);
    }

    #[test]
    fn conversion_is_deterministic() {
        need_pdfium!();
        let mut ops =
            vec![op("F2", 16.0, 72.0, 740.0, "Title Of The Test Paper"), op("F2", 12.0, 72.0, 700.0, "1 Introduction")];
        ops.extend(body(&["Some body text that fills the page.", "A second line of the paragraph."], 680.0));
        let pdf = build_pdf(&[page(ops)], Some("0 << /S /D >>"), Some("A Test Paper"));
        let a = convert_pdf(&pdf).extracted;
        let b = convert_pdf(&pdf).extracted;
        assert_eq!(a, b);
    }

    #[test]
    fn rotated_pages_are_read_upright() {
        need_pdfium!();
        let ops = [
            op("F1", 10.0, 72.0, 700.0, "First line on a landscape page"),
            op("F1", 10.0, 72.0, 686.0, "Second line on a landscape page"),
        ];
        for rotate in [90, 180, 270] {
            let mut p = page(ops.iter().map(|o| op(o.font, o.size, o.x, o.y, &o.text)).collect());
            p.rotate = rotate;
            let r = convert_pdf(&build_pdf(&[p], None, None));
            let md = &r.extracted.markdown;
            assert!(md.contains("First line on a landscape page"), "rotate {rotate}: {md}");
            let a = md.find("First line").unwrap();
            let b = md.find("Second line").unwrap();
            assert!(a < b, "rotate {rotate}: {md}");
        }
    }

    #[test]
    fn scanned_pages_classify_as_waiting_gpu() {
        need_pdfium!();
        let scan = PageSpec { ops: vec![], rotate: 0, full_page_image: true };
        let pdf = build_pdf(&[scan, PageSpec { ops: vec![], rotate: 0, full_page_image: true }], None, None);
        let (signals, class) = analyze(&pdf).unwrap();
        assert_eq!(signals.pages, 2);
        assert!(signals.page_signals.iter().all(|p| p.image_coverage > 0.9 && p.chars == 0));
        assert_eq!(class.route, PdfRoute::WaitingGpu(GpuReason::Scanned));
        let r = convert_pdf(&pdf);
        assert!(r.extracted.warnings.iter().any(|w| w.contains("no extractable text")));
        // The same file with text on the pages is native.
        let text_pdf = build_pdf(
            &[page(body(&["Plenty of ordinary words on this page to count as a text layer for sure."], 700.0))],
            None,
            None,
        );
        assert_eq!(analyze(&text_pdf).unwrap().1.route, PdfRoute::Native);
    }

    #[test]
    fn broken_input_is_reported_not_crashed() {
        need_pdfium!();
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
        assert_eq!(
            convert(b"%PDF-1.4\nnot really a pdf", &ConvertOptions::default()).unwrap_err().reason_code(),
            "corrupt_file"
        );
        let good = build_pdf(&[page(body(&["Some text here."], 700.0))], None, None);
        let truncated = &good[..good.len() / 2];
        assert!(
            convert(truncated, &ConvertOptions::default()).is_err()
                || convert(truncated, &ConvertOptions::default()).is_ok()
        );
    }

    #[test]
    fn font_names_give_bold_and_italic() {
        for n in [
            "ABCDEF+TimesNewRomanPS-BoldMT",
            "CMBX12",
            "Helvetica-Bold",
            "MyriadPro-Semibold",
            "NimbusRomNo9L-Medi",
            "Times-BoldItalic",
        ] {
            assert!(font_style(n).0, "{n}");
        }
        for n in ["CMR10", "Times-Roman", "CMMI10", "Calibri", "Cambria"] {
            assert!(!font_style(n).0, "{n}");
        }
        assert!(font_style("CMTI10").1 && font_style("Times-Italic").1 && font_style("Helvetica-Oblique").1);
        assert!(!font_style("CMMI10").1 && !font_style("CMR10").1);
    }

    #[test]
    fn titles_from_metadata_must_be_plausible() {
        assert!(plausible_title("Attention Is All You Need"));
        for bad in ["Microsoft Word - draft.docx", "untitled", "report.pdf", "12345", "x"] {
            assert!(!plausible_title(bad), "{bad}");
        }
    }
}

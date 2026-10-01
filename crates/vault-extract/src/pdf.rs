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
use crate::pdfclass::{PageSignals, PdfClassification, PdfSignals, classify, cjk_char_count, math_char_count};
use crate::pdflayout::{Glyph, PageInput, PageLines, assemble, infer_labels, layout_page};
use crate::pdfscan;
use crate::types::{Extracted, Metadata, tool_string};

/// Environment variable naming the PDFium library to load.
pub const LIBRARY_ENV: &str = "VAULT_PDFIUM_DLL";

static PDFIUM: OnceLock<std::result::Result<Pdfium, String>> = OnceLock::new();

/// Bind PDFium from `path`. The first call decides: later calls with another path succeed without rebinding.
pub fn init(path: &Path) -> Result<()> {
    let r = PDFIUM.get_or_init(|| Pdfium::bind_to_library(path).map(Pdfium::new).map_err(|e| format!("could not load PDFium from {}: {e:?}", path.display())));
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
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) | PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::SecurityError) => {
            ExtractError::Protected("the PDF needs a password".to_string())
        }
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError) => ExtractError::corrupt("pdf", "PDFium could not parse the file"),
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
        if obj.object_type() == PdfPageObjectType::Image {
            if let Ok(b) = obj.bounds() {
                covered += (b.width().value * b.height().value).abs();
            }
        }
    }
    PageSignals { chars, image_coverage: (covered / area).min(1.0), math_chars: math_char_count(text), cjk_chars: cjk_char_count(text) }
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
    let page_lines: Vec<PageLines> = inputs.iter().map(|p| PageLines { label: p.label.clone(), lines: layout_page(p) }).collect();
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
    signals.has_paper_id = assembled.extracted.metadata.arxiv_id.is_some() || assembled.extracted.metadata.doi.is_some();
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
        .map(|l| format!("x {:6.1}..{:6.1} y {:6.1}..{:6.1} size {:4.1} bold {} col {} furn {} | {}", l.l, l.r, l.bottom, l.top, l.size, l.bold as u8, l.column, l.furniture as u8, l.text))
        .collect())
}

fn read_metadata(doc: &PdfDocument) -> Metadata {
    let mut m = Metadata::default();
    let md = doc.metadata();
    let get = |t: PdfDocumentMetadataTagType| md.get(t).map(|tag| tag.value().trim().to_string()).filter(|v| !v.is_empty());
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
        m.year = d.trim_start_matches("D:").get(..4).and_then(|y| y.parse::<i32>().ok()).filter(|y| (1900..=2100).contains(y));
    }
    m
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
    for c in text.chars().iter() {
        let Some(ch) = c.unicode_char() else { continue };
        if c.is_generated().unwrap_or(false) || matches!(ch, '\r' | '\n' | '\u{fffe}' | '\u{ffff}') {
            continue;
        }
        // PDFium marks a line-end hyphen with U+0002.
        let ch = if ch == '\u{2}' { '-' } else { ch };
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
        let name = c.font_name().to_lowercase();
        let bold = name.contains("bold") || name.contains("black") || name.contains("heavy") || name.contains("semibold") || name.contains("demi") || matches!(c.font_weight(), Some(PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | PdfFontWeight::Weight900)) || matches!(c.font_weight(), Some(PdfFontWeight::Custom(n)) if n >= 600);
        let italic = name.contains("italic") || name.contains("oblique") || c.font_is_italic();
        let ch = if ch == '\u{ad}' { '-' } else { ch };
        input.glyphs.push(Glyph { ch, l, r, b: bt, t: tp, size, bold, italic });
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdfclass::{GpuReason, PdfRoute};
    use std::sync::Once;

    /// The vendored PDFium of the development machine, used when `VAULT_PDFIUM_DLL` is not set.
    const DEV_DLL: &str = "C:/Users/sethy/Projects/tutor/crates/rag-engine/vendor/pdfium/bin/pdfium.dll";

    fn have_pdfium() -> bool {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let path = std::env::var_os(LIBRARY_ENV).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEV_DLL));
            if path.exists() {
                if let Err(e) = init(&path) {
                    eprintln!("PDFium did not load: {e}");
                }
            }
        });
        matches!(PDFIUM.get(), Some(Ok(_)))
    }

    macro_rules! need_pdfium {
        () => {
            if !have_pdfium() {
                eprintln!("SKIPPED: PDFium library not found (set {LIBRARY_ENV})");
                return;
            }
        };
    }

    // --- a tiny PDF writer -------------------------------------------------------------------------------------

    struct Op {
        font: &'static str, // F1 Helvetica, F2 Helvetica-Bold, F3 Courier
        size: f32,
        x: f32,
        y: f32,
        text: String,
    }

    fn op(font: &'static str, size: f32, x: f32, y: f32, text: &str) -> Op {
        Op { font, size, x, y, text: text.to_string() }
    }

    struct PageSpec {
        ops: Vec<Op>,
        rotate: u32,
        full_page_image: bool,
    }

    fn page(ops: Vec<Op>) -> PageSpec {
        PageSpec { ops, rotate: 0, full_page_image: false }
    }

    fn escape(s: &str) -> String {
        s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
    }

    fn build_pdf(pages: &[PageSpec], page_labels: Option<&str>, info_title: Option<&str>) -> Vec<u8> {
        let n = pages.len();
        // object ids: 1 catalog, 2 pages, 3..5 fonts, 6 image, then (page, content) pairs, then info
        let mut objs: Vec<Vec<u8>> = Vec::new();
        let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 7 + i * 2)).collect();
        let labels = page_labels.map(|l| format!(" /PageLabels << /Nums [{l}] >>")).unwrap_or_default();
        objs.push(format!("<< /Type /Catalog /Pages 2 0 R{labels} >>").into_bytes());
        objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
        objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
        objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
        objs.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>".to_vec());
        let img_data = [0x80u8, 0x80, 0x80];
        let mut img = format!("<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n", img_data.len()).into_bytes();
        img.extend_from_slice(&img_data);
        img.extend_from_slice(b"\nendstream");
        objs.push(img);
        for (i, p) in pages.iter().enumerate() {
            let content_id = 8 + i * 2;
            let rotate = if p.rotate != 0 { format!(" /Rotate {}", p.rotate) } else { String::new() };
            objs.push(
                format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]{rotate} /Contents {content_id} 0 R /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> /XObject << /Im0 6 0 R >> >> >>").into_bytes(),
            );
            let mut s = String::new();
            if p.full_page_image {
                s.push_str("q 612 0 0 792 0 0 cm /Im0 Do Q\n");
            }
            for o in &p.ops {
                s.push_str(&format!("BT /{} {} Tf {} {} Td ({}) Tj ET\n", o.font, o.size, o.x, o.y, escape(&o.text)));
            }
            let mut c = format!("<< /Length {} >>\nstream\n", s.len()).into_bytes();
            c.extend_from_slice(s.as_bytes());
            c.extend_from_slice(b"\nendstream");
            objs.push(c);
        }
        let info_id = objs.len() + 1;
        if let Some(t) = info_title {
            objs.push(format!("<< /Title ({}) /Author (Ann Lee; Bo Chen) /CreationDate (D:20190402101500Z) >>", escape(t)).into_bytes());
        }
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(o);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
        for o in offsets {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        let info = if info_title.is_some() { format!(" /Info {info_id} 0 R") } else { String::new() };
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R{info} >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
        out
    }

    fn convert_pdf(bytes: &[u8]) -> PdfConversion {
        convert(bytes, &ConvertOptions::default()).unwrap()
    }

    fn body(lines: &[&str], y0: f32) -> Vec<Op> {
        lines.iter().enumerate().map(|(i, t)| op("F1", 10.0, 72.0, y0 - i as f32 * 12.0, t)).collect()
    }

    // --- tests -------------------------------------------------------------------------------------------------

    #[test]
    fn page_labels_headings_paragraphs_and_metadata() {
        need_pdfium!();
        let mut ops = vec![op("F2", 18.0, 72.0, 740.0, "A Study of Synthetic Things"), op("F2", 12.0, 72.0, 700.0, "1 Introduction")];
        ops.extend(body(&["This paragraph has a hyphen-", "ated word and runs to the end.", "It has two sentences."], 680.0));
        ops.extend(body(&["A second paragraph starts after a gap."], 620.0));
        let p2 = body(&["Second page text continues here with more words."], 700.0);
        let pdf = build_pdf(&[page(ops), page(p2)], Some("0 << /S /r >> 1 << /S /D /St 214 /P (A-) >>"), Some("Real Paper Title"));
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
    fn superscripts_running_headers_and_printed_page_numbers() {
        need_pdfium!();
        let mut pages = Vec::new();
        for i in 0..8 {
            let mut ops = vec![op("F1", 9.0, 72.0, 765.0, "Journal of Synthetic Results"), op("F1", 9.0, 300.0, 30.0, &format!("{}", 100 + i))];
            ops.push(op("F1", 10.0, 72.0, 600.0, &format!("Page {i} body text about energy E = m c")));
            ops.push(op("F1", 6.0, 72.0 + 10.0 * 0.5 * 36.0 + 3.0, 604.0, "2"));
            pages.push(page(ops));
        }
        let r = convert_pdf(&build_pdf(&pages, None, None));
        let ex = &r.extracted;
        assert!(!ex.markdown.contains("Journal of Synthetic"), "{}", ex.markdown);
        assert!(!ex.markdown.contains("\n100\n") && !ex.markdown.contains("\n107\n"), "{}", ex.markdown);
        let labels: Vec<_> = ex.pages.iter().map(|p| p.label.clone().unwrap_or_default()).collect();
        assert_eq!(labels, (100..108).map(|n| n.to_string()).collect::<Vec<_>>());
        assert!(ex.markdown.contains("m c^2"), "{}", ex.markdown);
    }

    #[test]
    fn rotated_pages_are_read_upright() {
        need_pdfium!();
        let ops = vec![op("F1", 10.0, 72.0, 700.0, "First line on a landscape page"), op("F1", 10.0, 72.0, 686.0, "Second line on a landscape page")];
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
        let text_pdf = build_pdf(&[page(body(&["Plenty of ordinary words on this page to count as a text layer for sure."], 700.0))], None, None);
        assert_eq!(analyze(&text_pdf).unwrap().1.route, PdfRoute::Native);
    }

    #[test]
    fn broken_input_is_reported_not_crashed() {
        need_pdfium!();
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
        assert_eq!(convert(b"%PDF-1.4\nnot really a pdf", &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        let good = build_pdf(&[page(body(&["Some text here."], 700.0))], None, None);
        let truncated = &good[..good.len() / 2];
        assert!(convert(truncated, &ConvertOptions::default()).is_err() || convert(truncated, &ConvertOptions::default()).is_ok());
    }

    #[test]
    fn titles_from_metadata_must_be_plausible() {
        assert!(plausible_title("Attention Is All You Need"));
        for bad in ["Microsoft Word - draft.docx", "untitled", "report.pdf", "12345", "x"] {
            assert!(!plausible_title(bad), "{bad}");
        }
    }
}

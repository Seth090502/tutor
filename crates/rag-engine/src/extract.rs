//! Per-format measurement (survey) and text extraction (ingest). Errors are returned as
//! messages for the per-file record.

use crate::text::{self, TextSize};
use crate::triage::{self, PageClass, TriageCounts};
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Run a parser, turning a panic inside it into an error message.
pub fn catch_panics<T>(parse: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(parse)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        Err(format!("parser panicked: {message}"))
    })
}

/// PDFium is not thread-safe. Every call into it (opening, reading, rendering and closing
/// documents) happens while holding this lock, so ingest and survey extraction and page
/// rendering in the service run one at a time. Extraction releases it between pages.
static PDFIUM: Mutex<()> = Mutex::new(());

pub fn pdfium_lock() -> MutexGuard<'static, ()> {
    // A panic inside PDFium work (caught by `catch_panics`) must not disable PDFium for good.
    PDFIUM
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Form XObjects can nest; stop looking for images below this depth.
const MAX_FORM_DEPTH: u32 = 8;

#[derive(Debug, Default, Clone, Copy)]
pub struct Measurement {
    pub pages: Option<u64>,
    pub triage: Option<TriageCounts>,
    pub size: TextSize,
}

/// Measure a PDF's text layer page by page. Password-protected, encrypted (including
/// owner-password-only) and DRM-protected files are reported as errors and never opened
/// further; no password is ever supplied.
pub fn measure_pdf(pdfium: &Pdfium, path: &Path) -> Result<Measurement, String> {
    // Declared before the document, so the document is closed while the lock is still held.
    let mut guard = pdfium_lock();
    let document = open_pdf(pdfium, path)?;
    let mut counts = TriageCounts::default();
    let mut size = TextSize::default();
    let mut pages = 0u64;
    for (index, page) in document.pages().iter().enumerate() {
        let page_text = page_text(&page, index)?;
        counts.add(page_class(&page, &page_text));
        size += text::measure(&page_text);
        pages += 1;
        drop(page);
        guard = yield_pdfium(guard);
    }
    Ok(Measurement {
        pages: Some(pages),
        triage: Some(counts),
        size,
    })
}

/// Let a waiting renderer in between two pages.
fn yield_pdfium(guard: MutexGuard<'static, ()>) -> MutexGuard<'static, ()> {
    drop(guard);
    pdfium_lock()
}

/// Open a PDF that is not encrypted in any way; see [`measure_pdf`]. The caller holds
/// [`pdfium_lock`].
pub fn open_pdf<'a>(pdfium: &'a Pdfium, path: &Path) -> Result<PdfDocument<'a>, String> {
    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| match e {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
            "encrypted PDF: password required; not processed".to_string()
        }
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::SecurityError) => {
            "protected PDF: unsupported security handler (DRM); not processed".to_string()
        }
        other => format!("PDFium could not open the file: {other}"),
    })?;
    match document.permissions().security_handler_revision() {
        Ok(PdfSecurityHandlerRevision::Unprotected) => {}
        Ok(revision) => {
            return Err(format!(
                "encrypted PDF (standard security handler {revision:?}); not processed"
            ));
        }
        Err(_) => {
            return Err("encrypted PDF (unrecognized security handler); not processed".to_string());
        }
    }
    Ok(document)
}

fn page_text(page: &PdfPage, index: usize) -> Result<String, String> {
    Ok(page
        .text()
        .map_err(|e| format!("page {}: cannot read text layer: {e}", index + 1))?
        .all())
}

fn page_class(page: &PdfPage, page_text: &str) -> PageClass {
    let stats = triage::page_text_stats(page_text);
    triage::classify(&stats, || {
        page.objects().iter().any(|object| has_image(&object, 0))
    })
}

pub struct PdfPageText {
    pub text: String,
    pub class: PageClass,
}

pub struct PdfText {
    pub title: Option<String>,
    pub pages: Vec<PdfPageText>,
}

/// Each page's text layer (line breaks normalised to `\n`, other control characters removed)
/// and triage class, plus the document's metadata title. Same refusal rules as [`measure_pdf`].
pub fn pdf_text(pdfium: &Pdfium, path: &Path) -> Result<PdfText, String> {
    // Declared before the document, so the document is closed while the lock is still held.
    let mut guard = pdfium_lock();
    let document = open_pdf(pdfium, path)?;
    let title = document
        .metadata()
        .get(PdfDocumentMetadataTagType::Title)
        .map(|t| t.value().trim().to_string())
        .filter(|t| !t.is_empty());
    let mut pages = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let raw = page_text(&page, index)?;
        let class = page_class(&page, &raw);
        pages.push(PdfPageText {
            text: clean_pdf_text(&raw),
            class,
        });
        drop(page);
        guard = yield_pdfium(guard);
    }
    Ok(PdfText { title, pages })
}

fn clean_pdf_text(raw: &str) -> String {
    raw.replace("\r\n", "\n")
        .chars()
        .map(|c| if c == '\r' { '\n' } else { c })
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect()
}

fn has_image(object: &PdfPageObject, depth: u32) -> bool {
    match object.object_type() {
        PdfPageObjectType::Image => true,
        PdfPageObjectType::XObjectForm if depth < MAX_FORM_DEPTH => {
            object.as_x_object_form_object().is_some_and(|form| {
                (0..form.len())
                    .filter_map(|i| form.get(i).ok())
                    .any(|child| has_image(&child, depth + 1))
            })
        }
        _ => false,
    }
}

/// Sum the text of every spine document, with markup stripped.
pub fn measure_epub(path: &Path) -> Result<Measurement, String> {
    let epub = rbook::Epub::open(path).map_err(|e| format!("cannot open EPUB: {e}"))?;
    let mut size = TextSize::default();
    for content in epub.reader() {
        let content = content.map_err(|e| format!("cannot read EPUB content: {e}"))?;
        size += text::measure(&text::strip_markup(content.content()));
    }
    Ok(Measurement {
        size,
        ..Default::default()
    })
}

/// Markdown and plain text, decoded as UTF-8 with invalid sequences replaced.
pub fn measure_plain(path: &Path) -> Result<Measurement, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read file: {e}"))?;
    Ok(Measurement {
        size: text::measure(&String::from_utf8_lossy(&bytes)),
        ..Default::default()
    })
}

pub struct EpubSection {
    /// Table-of-contents labels leading to this spine document (chapter titles).
    pub heading_path: Vec<String>,
    pub text: String,
}

pub struct EpubText {
    pub title: Option<String>,
    pub sections: Vec<EpubSection>,
}

/// Block-level closing tags after which a paragraph break is kept in the text.
const BLOCK_ENDS: [&str; 14] = [
    "</p>",
    "</div>",
    "</li>",
    "</h1>",
    "</h2>",
    "</h3>",
    "</h4>",
    "</h5>",
    "</h6>",
    "</tr>",
    "</section>",
    "</blockquote>",
    "<br/>",
    "<br />",
];

/// The text of every spine document with markup stripped and paragraph breaks kept. Each
/// document's heading path is the table-of-contents path of the first entry that points to
/// it; a document without an entry continues the previous one's path.
pub fn epub_text(path: &Path) -> Result<EpubText, String> {
    let epub = rbook::Epub::open(path).map_err(|e| format!("cannot open EPUB: {e}"))?;
    let title = epub
        .metadata()
        .title()
        .map(|t| t.value().trim().to_string())
        .filter(|t| !t.is_empty());
    let mut toc_paths: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(root) = epub.toc().contents() {
        let mut stack: Vec<String> = Vec::new();
        for entry in root.flatten().filter(|e| e.depth() > 0) {
            stack.truncate(entry.depth() - 1);
            stack.push(entry.label().trim().to_string());
            if let Some(key) = entry
                .manifest_entry()
                .and_then(|m| m.resource().key().value().map(str::to_string))
            {
                toc_paths.entry(key).or_insert_with(|| stack.clone());
            }
        }
    }
    let mut sections = Vec::new();
    let mut heading_path = Vec::new();
    for content in epub.reader() {
        let content = content.map_err(|e| format!("cannot read EPUB content: {e}"))?;
        let key = content
            .manifest_entry()
            .resource()
            .key()
            .value()
            .map(str::to_string);
        if let Some(path) = key.and_then(|k| toc_paths.get(&k)) {
            heading_path = path.clone();
        }
        let mut xhtml = content.content().to_string();
        for tag in BLOCK_ENDS {
            xhtml = xhtml.replace(tag, &format!("{tag}\n\n"));
        }
        sections.push(EpubSection {
            heading_path: heading_path.clone(),
            text: tidy(&text::strip_markup(&xhtml)),
        });
    }
    Ok(EpubText { title, sections })
}

/// Markdown or plain text, decoded as UTF-8 with invalid sequences replaced and `\r\n` as `\n`.
pub fn plain_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read file: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}

/// Collapse spaces within lines and runs of blank lines to one blank line.
fn tidy(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            blank += 1;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank > 0 { "\n\n" } else { "\n" });
        }
        out.push_str(&line);
        blank = 0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_text_keeps_lines_and_drops_other_control_characters() {
        assert_eq!(clean_pdf_text("a\r\nb\rc\u{2}d\te"), "a\nb\ncd\te");
    }

    #[test]
    fn tidy_keeps_paragraph_breaks_only() {
        assert_eq!(tidy("  a   b \n c\n\n\n\n d  \n"), "a b\nc\n\nd");
        assert_eq!(tidy(" \n "), "");
    }

    #[test]
    fn epub_sections_follow_the_spine_with_toc_titles() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/book.epub");
        let epub = epub_text(&fixture).unwrap();
        assert_eq!(epub.title.as_deref(), Some("Survey Fixture"));
        assert_eq!(epub.sections.len(), 1);
        assert_eq!(epub.sections[0].heading_path, vec!["Chapter 1"]);
        assert!(
            epub.sections[0]
                .text
                .contains("Plants & algae capture light energy."),
            "{}",
            epub.sections[0].text
        );
        assert!(
            epub.sections[0].text.contains("\n\nThey store it"),
            "{}",
            epub.sections[0].text
        );
        assert!(!epub.sections[0].text.contains("margin"));
    }
}

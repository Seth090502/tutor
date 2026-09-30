//! Per-format measurement. Errors are returned as messages for the per-file record.

use crate::text::{self, TextSize};
use crate::triage::{self, TriageCounts};
use pdfium_render::prelude::*;
use std::path::Path;

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

    let mut counts = TriageCounts::default();
    let mut size = TextSize::default();
    let mut pages = 0u64;
    for (index, page) in document.pages().iter().enumerate() {
        let page_text = page
            .text()
            .map_err(|e| format!("page {}: cannot read text layer: {e}", index + 1))?
            .all();
        let stats = triage::page_text_stats(&page_text);
        counts.add(triage::classify(&stats, || {
            page.objects().iter().any(|object| has_image(&object, 0))
        }));
        size += text::measure(&page_text);
        pages += 1;
    }
    Ok(Measurement { pages: Some(pages), triage: Some(counts), size })
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
    Ok(Measurement { size, ..Default::default() })
}

/// Markdown and plain text, decoded as UTF-8 with invalid sequences replaced.
pub fn measure_plain(path: &Path) -> Result<Measurement, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read file: {e}"))?;
    Ok(Measurement { size: text::measure(&String::from_utf8_lossy(&bytes)), ..Default::default() })
}

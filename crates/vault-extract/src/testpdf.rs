//! Test-only helpers: a minimal PDF writer, and a once-per-process PDFium binding for tests that need the real
//! engine. Those tests skip with a message when no library is available (set `VAULT_PDFIUM_DLL`).

use std::path::PathBuf;
use std::sync::Once;

use crate::ConvertOptions;
use crate::pdf::{LIBRARY_ENV, PdfConversion, convert, init, is_bound};

/// The vendored PDFium of the development machine, used when `VAULT_PDFIUM_DLL` is not set.
const DEV_DLL: &str = "C:/Users/sethy/Projects/tutor/crates/rag-engine/vendor/pdfium/bin/pdfium.dll";

pub(crate) fn have_pdfium() -> bool {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let path = std::env::var_os(LIBRARY_ENV).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEV_DLL));
        if path.exists()
            && let Err(e) = init(&path)
        {
            eprintln!("PDFium did not load: {e}");
        }
    });
    is_bound()
}

macro_rules! need_pdfium {
    () => {
        if !crate::testpdf::have_pdfium() {
            eprintln!("SKIPPED: PDFium library not found (set {})", crate::pdf::LIBRARY_ENV);
            return;
        }
    };
}
pub(crate) use need_pdfium;

// --- a tiny PDF writer -------------------------------------------------------------------------------------

pub(crate) struct Op {
    pub font: &'static str, // F1 Helvetica, F2 Helvetica-Bold, F3 Courier
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub text: String,
}

pub(crate) fn op(font: &'static str, size: f32, x: f32, y: f32, text: &str) -> Op {
    Op { font, size, x, y, text: text.to_string() }
}

pub(crate) struct PageSpec {
    pub ops: Vec<Op>,
    pub rotate: u32,
    pub full_page_image: bool,
}

pub(crate) fn page(ops: Vec<Op>) -> PageSpec {
    PageSpec { ops, rotate: 0, full_page_image: false }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

pub(crate) fn build_pdf(pages: &[PageSpec], page_labels: Option<&str>, info_title: Option<&str>) -> Vec<u8> {
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
            if o.font == "RAW" {
                s.push_str(&o.text);
                s.push('\n');
            } else {
                s.push_str(&format!("BT /{} {} Tf {} {} Td ({}) Tj ET\n", o.font, o.size, o.x, o.y, escape(&o.text)));
            }
        }
        let mut c = format!("<< /Length {} >>\nstream\n", s.len()).into_bytes();
        c.extend_from_slice(s.as_bytes());
        c.extend_from_slice(b"\nendstream");
        objs.push(c);
    }
    let info_id = objs.len() + 1;
    if let Some(t) = info_title {
        objs.push(
            format!("<< /Title ({}) /Author (Ann Lee; Bo Chen) /CreationDate (D:20190402101500Z) >>", escape(t))
                .into_bytes(),
        );
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
    out.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R{info} >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes(),
    );
    out
}

pub(crate) fn convert_pdf(bytes: &[u8]) -> PdfConversion {
    convert(bytes, &ConvertOptions::default()).unwrap()
}

pub(crate) fn body(lines: &[&str], y0: f32) -> Vec<Op> {
    lines.iter().enumerate().map(|(i, t)| op("F1", 10.0, 72.0, y0 - i as f32 * 12.0, t)).collect()
}

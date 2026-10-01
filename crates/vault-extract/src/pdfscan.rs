//! A cheap structural scan of a PDF's bytes: page count, fonts, images, encryption. No PDF library: regular
//! expressions over the raw bytes, plus inflating the object streams that hold the fonts and the page tree in
//! PDF 1.5 and later. The numbers are signals for routing, not a parse.

use std::io::Read;
use std::sync::LazyLock;

use flate2::read::ZlibDecoder;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

/// Inflated object-stream bytes scanned per file.
const MAX_INFLATED: usize = 256 * 1024 * 1024;
const MAX_OBJSTMS: usize = 20_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfScan {
    pub version: Option<String>,
    /// Page count from the page tree, when it could be read.
    pub pages: Option<u32>,
    /// An `/Encrypt` dictionary exists. A PDF with an empty user password still opens; only a failed open is
    /// "protected".
    pub encrypted: bool,
    /// Distinct base font names without subset prefixes.
    pub fonts: Vec<String>,
    /// The subset of `fonts` that are mathematical (Computer Modern maths, AMS, STIX Math, Cambria Math...).
    pub math_fonts: Vec<String>,
    /// Image XObjects in the file.
    pub image_objects: u32,
    pub has_page_labels: bool,
    pub producer: Option<String>,
    pub creator: Option<String>,
}

impl PdfScan {
    /// No fonts at all and at least one image: a scan wrapped in a PDF.
    pub fn likely_scanned(&self) -> bool {
        self.fonts.is_empty() && self.image_objects > 0
    }
}

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect("static regex"));
    };
}

re!(ENCRYPT, r"(?-u)/Encrypt\s*(?:\d+\s+\d+\s+R|<<)");
re!(PAGE, r"(?-u)/Type\s*/Page\b");
re!(PAGES, r"(?-u)/Type\s*/Pages\b");
re!(COUNT, r"(?-u)/Count\s+(\d+)");
re!(BASEFONT, r"(?-u)/BaseFont\s*/([^\s/\[\]<>()%]+)");
re!(IMAGE, r"(?-u)/Subtype\s*/Image\b");
re!(LABELS, r"(?-u)/PageLabels\b");
re!(OBJSTM, r"(?-u)/Type\s*/ObjStm\b");
re!(HEADER, r"(?-u)%PDF-(\d\.\d)");
re!(PRODUCER, r"(?-u)/Producer\s*\(((?:[^()\\]|\\.){0,200})\)");
re!(CREATOR, r"(?-u)/Creator\s*\(((?:[^()\\]|\\.){0,200})\)");

static MATH_FONT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)^(?:cm(?:mi|sy|ex|bsy|mib|ss)\d*|msam\d*|msbm\d*|eufm\d*|eusm\d*|euex\d*|rsfs\d*|lasy\d*|wasy\d*|stmary\d*|stix.*math.*|cambria.?math|mathematicalpi.*|mtextra|mt-?extra|mtsy.*|lmmath.*|latinmodernmath.*|xits.?math|asana.?math|texgyre.*math|newtxmi.*|newtxsy.*|txmi.*|txsy.*|mathcomp.*|fouriergut.*|mnsymbol.*)$",
    )
    .expect("static regex")
});

/// Whether a font base name is a mathematical font.
pub fn is_math_font(name: &str) -> bool {
    MATH_FONT.is_match(name)
}

/// Strip a six-capital subset prefix (`ABCDEF+Name`) and any PDF `#xx` escapes.
pub fn clean_font_name(raw: &str) -> String {
    let name = match raw.split_once('+') {
        Some((p, rest)) if p.len() == 6 && p.chars().all(|c| c.is_ascii_uppercase()) => rest,
        _ => raw,
    };
    name.to_string()
}

pub fn is_encrypted(bytes: &[u8]) -> bool {
    ENCRYPT.is_match(bytes)
}

/// Inflate every `/Type /ObjStm` stream, up to the scan limits.
fn object_streams(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut total = 0usize;
    for m in OBJSTM.find_iter(bytes).take(MAX_OBJSTMS) {
        let Some(rel) = find_bytes(&bytes[m.end()..bytes.len().min(m.end() + 4096)], b"stream") else { continue };
        let mut start = m.end() + rel + b"stream".len();
        if bytes.get(start) == Some(&b'\r') {
            start += 1;
        }
        if bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
        let end = find_bytes(&bytes[start..], b"endstream").map(|e| start + e).unwrap_or(bytes.len());
        let mut inflated = Vec::new();
        let remaining = (MAX_INFLATED - total) as u64;
        // A stream with a predictor or another filter fails to inflate cleanly; whatever decoded is still useful.
        let _ = ZlibDecoder::new(&bytes[start..end]).take(remaining).read_to_end(&mut inflated);
        total += inflated.len();
        out.push(inflated);
        if total >= MAX_INFLATED {
            break;
        }
    }
    out
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn pdf_string(raw: &[u8]) -> Option<String> {
    let s: String = String::from_utf8_lossy(raw).replace("\\(", "(").replace("\\)", ")").replace("\\\\", "\\");
    let s = s.trim().to_string();
    (!s.is_empty() && s.chars().all(|c| !c.is_control())).then_some(s)
}

/// Scan `bytes` as a PDF.
pub fn scan(bytes: &[u8]) -> PdfScan {
    let mut s = PdfScan {
        version: HEADER.captures(&bytes[..bytes.len().min(1024)]).map(|c| String::from_utf8_lossy(&c[1]).into_owned()),
        encrypted: is_encrypted(bytes),
        ..PdfScan::default()
    };
    let streams = object_streams(bytes);
    let mut page_objects = 0u32;
    let mut max_count = 0u32;
    let mut fonts: Vec<String> = Vec::new();

    for chunk in std::iter::once(bytes).chain(streams.iter().map(Vec::as_slice)) {
        page_objects += PAGE.find_iter(chunk).count() as u32;
        for m in PAGES.find_iter(chunk) {
            let lo = m.start().saturating_sub(256);
            let hi = chunk.len().min(m.end() + 512);
            if let Some(c) = COUNT.captures(&chunk[lo..hi]) {
                if let Ok(n) = String::from_utf8_lossy(&c[1]).parse::<u32>() {
                    max_count = max_count.max(n);
                }
            }
        }
        for c in BASEFONT.captures_iter(chunk) {
            let name = clean_font_name(&String::from_utf8_lossy(&c[1]));
            if !name.is_empty() && !fonts.contains(&name) {
                fonts.push(name);
            }
        }
        s.image_objects += IMAGE.find_iter(chunk).count() as u32;
        s.has_page_labels |= LABELS.is_match(chunk);
        if s.producer.is_none() {
            s.producer = PRODUCER.captures(chunk).and_then(|c| pdf_string(&c[1]));
        }
        if s.creator.is_none() {
            s.creator = CREATOR.captures(chunk).and_then(|c| pdf_string(&c[1]));
        }
    }
    let pages = page_objects.max(max_count);
    s.pages = (pages > 0).then_some(pages);
    s.math_fonts = fonts.iter().filter(|f| is_math_font(f)).cloned().collect();
    s.fonts = fonts;
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn deflate(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn scans_an_uncompressed_pdf() {
        let pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [0 << /S /r >>] >> >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n4 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n\
5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+CMMI10 >>\nendobj\n\
6 0 obj\n<< /Type /Font /BaseFont /Times-Roman >>\nendobj\n\
7 0 obj\n<< /Type /XObject /Subtype /Image /Width 10 >>\nendobj\n\
8 0 obj\n<< /Producer (TeX \\(pdfTeX\\)) /Creator (LaTeX) >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF";
        let s = scan(pdf);
        assert_eq!(s.version.as_deref(), Some("1.4"));
        assert_eq!(s.pages, Some(2));
        assert_eq!(s.fonts, vec!["CMMI10", "Times-Roman"]);
        assert_eq!(s.math_fonts, vec!["CMMI10"]);
        assert_eq!(s.image_objects, 1);
        assert!(s.has_page_labels && !s.encrypted);
        assert_eq!(s.producer.as_deref(), Some("TeX (pdfTeX)"));
        assert!(!s.likely_scanned());
    }

    #[test]
    fn reads_fonts_and_pages_out_of_object_streams() {
        let inner = b"9 0 10 40 << /Type /Pages /Count 7 /Kids [] >> << /Type /Font /BaseFont /QRSTUV+STIXMath-Regular >>";
        let mut pdf = b"%PDF-1.5\n12 0 obj\n<< /Type /ObjStm /N 2 /First 10 /Filter /FlateDecode /Length 99 >>\nstream\n".to_vec();
        pdf.extend(deflate(inner));
        pdf.extend(b"\nendstream\nendobj\n%%EOF");
        let s = scan(&pdf);
        assert_eq!(s.pages, Some(7));
        assert_eq!(s.fonts, vec!["STIXMath-Regular"]);
        assert_eq!(s.math_fonts, vec!["STIXMath-Regular"]);
    }

    #[test]
    fn encryption_and_scans() {
        let pdf = b"%PDF-1.7\n<< /Type /Page >>\n<< /Type /XObject /Subtype /Image >>\n<< /Type /XObject /Subtype /Image >>\ntrailer << /Encrypt 5 0 R /Root 1 0 R >>";
        let s = scan(pdf);
        assert!(s.encrypted && s.likely_scanned());
        assert!(!is_encrypted(b"%PDF-1.7 /EncryptMetadata false"));
    }

    #[test]
    fn math_font_names() {
        for n in ["CMMI10", "CMSY8", "CMEX10", "MSAM10", "MSBM7", "STIXMathJax_Main", "CambriaMath", "Cambria Math", "LatinModernMath-Regular", "MTExtra", "NewTXMI"] {
            assert!(is_math_font(n), "{n}");
        }
        for n in ["CMR10", "CMBX12", "Times-Roman", "Symbol", "Arial", "STIXGeneral-Regular", "Cambria"] {
            assert!(!is_math_font(n), "{n}");
        }
    }

    #[test]
    fn font_subset_prefix_is_removed_only_when_six_capitals() {
        assert_eq!(clean_font_name("ABCDEF+Font-Bold"), "Font-Bold");
        assert_eq!(clean_font_name("Abc+Font"), "Abc+Font");
        assert_eq!(clean_font_name("Font"), "Font");
    }
}

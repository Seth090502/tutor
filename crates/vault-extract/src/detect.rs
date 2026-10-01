//! Content-based format detection: magic bytes first, then container inspection. A file name, when given, only
//! breaks ties between text formats (Markdown against plain text); it never overrides the bytes.

use std::io::{Cursor, Read};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::text::{self, TextBom};

/// Bytes of a file the sniffers look at for text formats.
const HEAD: usize = 64 * 1024;
/// Decompressed bytes inspected when looking inside a gzip stream.
const GZIP_SNIFF: u64 = 16 * 1024 * 1024;
/// Entries listed when deciding whether a tarball is arXiv source.
const TAR_SNIFF_ENTRIES: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryKind {
    Image,
    Audio,
    Video,
    Executable,
    Font,
    /// 7z, RAR, xz, bzip2, zstd: containers this crate does not unpack.
    CompressedArchive,
    Database,
    /// Legacy binary documents: OLE (doc, xls, ppt), PostScript, DjVu, CHM, MOBI.
    LegacyDocument,
    /// OOXML and ODF documents other than word processing (xlsx, pptx, ods, odp).
    OtherOffice,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Empty,
    Pdf,
    Epub,
    Docx,
    Odt,
    Zip,
    Tar,
    /// gzip of something that is not a tarball; see [`Detected::inner`].
    Gzip,
    /// gzip of a tarball that is not arXiv source.
    TarGz,
    /// A tarball (gzipped or not) or gzipped file holding LaTeX source, as arXiv serves it.
    ArxivSource,
    Jats,
    Tei,
    Html,
    Xhtml,
    /// Any other XML; the root element is in [`Detected::xml_root`].
    Xml,
    Notebook,
    Markdown,
    PlainText,
    Json,
    /// A standalone LaTeX file.
    Latex,
    Rtf,
    Binary(BinaryKind),
}

impl Format {
    /// Short stable name, used in logs and the `extraction.path` of sidecars.
    pub fn name(self) -> &'static str {
        match self {
            Format::Empty => "empty",
            Format::Pdf => "pdf",
            Format::Epub => "epub",
            Format::Docx => "docx",
            Format::Odt => "odt",
            Format::Zip => "zip",
            Format::Tar => "tar",
            Format::Gzip => "gzip",
            Format::TarGz => "tar.gz",
            Format::ArxivSource => "arxiv-source",
            Format::Jats => "jats",
            Format::Tei => "tei",
            Format::Html => "html",
            Format::Xhtml => "xhtml",
            Format::Xml => "xml",
            Format::Notebook => "notebook",
            Format::Markdown => "markdown",
            Format::PlainText => "text",
            Format::Json => "json",
            Format::Latex => "latex",
            Format::Rtf => "rtf",
            Format::Binary(_) => "binary",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Pdf => "application/pdf",
            Format::Epub => "application/epub+zip",
            Format::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Format::Odt => "application/vnd.oasis.opendocument.text",
            Format::Zip => "application/zip",
            Format::Tar => "application/x-tar",
            Format::Gzip | Format::TarGz | Format::ArxivSource => "application/gzip",
            Format::Jats | Format::Tei | Format::Xml => "application/xml",
            Format::Html => "text/html",
            Format::Xhtml => "application/xhtml+xml",
            Format::Notebook => "application/x-ipynb+json",
            Format::Markdown => "text/markdown",
            Format::PlainText | Format::Empty => "text/plain",
            Format::Json => "application/json",
            Format::Latex => "application/x-tex",
            Format::Rtf => "application/rtf",
            Format::Binary(_) => "application/octet-stream",
        }
    }

    /// Whether this is a container whose members are processed individually.
    pub fn is_archive(self) -> bool {
        matches!(self, Format::Zip | Format::Tar | Format::TarGz | Format::Gzip)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detected {
    pub format: Format,
    /// Password-protected or encrypted (PDF with an /Encrypt entry, encrypted OOXML, encrypted ZIP entries).
    pub encrypted: bool,
    /// For [`Format::Gzip`]: what the decompressed head looks like.
    pub inner: Option<Format>,
    /// For [`Format::Xml`]: the root element's local name.
    pub xml_root: Option<String>,
    /// For text formats: the byte-order mark or encoding the head implied.
    pub bom: Option<TextBomName>,
}

/// Serializable name of a detected byte-order mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextBomName {
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl Detected {
    fn new(format: Format) -> Self {
        Self { format, encrypted: false, inner: None, xml_root: None, bom: None }
    }
}

/// Detect the format of `bytes`. `name_hint` is a file name or extension used only to break ties among text
/// formats.
pub fn detect(bytes: &[u8], name_hint: Option<&str>) -> Detected {
    if bytes.is_empty() {
        return Detected::new(Format::Empty);
    }
    if let Some(d) = detect_binary_magic(bytes, name_hint) {
        return d;
    }
    detect_text(bytes, name_hint)
}

fn ext_of(name_hint: Option<&str>) -> Option<String> {
    let name = name_hint?;
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let ext = if base.contains('.') { base.rsplit('.').next()? } else { base };
    Some(ext.to_ascii_lowercase())
}

fn detect_binary_magic(b: &[u8], name_hint: Option<&str>) -> Option<Detected> {
    let bin = |k| Some(Detected::new(Format::Binary(k)));

    // PDF: the header may follow a little junk.
    if b.len() >= 5 && find(&b[..b.len().min(1024)], b"%PDF-").is_some() {
        let mut d = Detected::new(Format::Pdf);
        d.encrypted = crate::pdfscan::is_encrypted(b);
        return Some(d);
    }
    // Containers.
    if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") {
        return Some(detect_zip(b));
    }
    if b.starts_with(&[0x1F, 0x8B]) {
        return Some(detect_gzip(b));
    }
    if b.len() >= 512 && is_tar_header(&b[..512]) {
        return Some(Detected::new(if tar_has_tex(Cursor::new(b)) { Format::ArxivSource } else { Format::Tar }));
    }
    // Compressed archives this crate does not unpack.
    if b.starts_with(b"BZh") && b.len() > 4 && (b'1'..=b'9').contains(&b[3])
        || b.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0])
        || b.starts_with(&[0x28, 0xB5, 0x2F, 0xFD])
        || b.starts_with(&[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C])
        || b.starts_with(b"Rar!\x1A\x07")
    {
        return bin(BinaryKind::CompressedArchive);
    }
    // OLE compound files: legacy Office, MSI, or encrypted OOXML.
    if b.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        let mut d = Detected::new(Format::Binary(BinaryKind::LegacyDocument));
        d.encrypted = contains_utf16le(b, "EncryptedPackage");
        return Some(d);
    }
    // Images.
    if b.starts_with(&[0x89, b'P', b'N', b'G'])
        || b.starts_with(&[0xFF, 0xD8, 0xFF])
        || b.starts_with(b"GIF87a")
        || b.starts_with(b"GIF89a")
        || b.starts_with(&[b'I', b'I', 42, 0])
        || b.starts_with(&[b'M', b'M', 0, 42])
        || b.starts_with(&[0, 0, 1, 0])
        || b.starts_with(&[0, 0, 0, 0x0C, b'j', b'P', b' ', b' '])
        || (b.starts_with(b"BM") && b.len() > 14 && b[6..10] == [0, 0, 0, 0])
        || riff(b, b"WEBP")
        || b.starts_with(b"8BPS")
    {
        return bin(BinaryKind::Image);
    }
    // ISO base media (mp4, mov, m4a, heic).
    if b.len() >= 12 && &b[4..8] == b"ftyp" {
        let brand = &b[8..12];
        return bin(match brand {
            b"heic" | b"heix" | b"hevc" | b"mif1" | b"msf1" | b"avif" | b"avis" => BinaryKind::Image,
            b"M4A " | b"M4B " | b"M4P " => BinaryKind::Audio,
            _ => BinaryKind::Video,
        });
    }
    if b.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) || riff(b, b"AVI ") || b.starts_with(b"FLV\x01") || b.starts_with(&[0, 0, 1, 0xBA]) {
        return bin(BinaryKind::Video);
    }
    if b.starts_with(b"ID3")
        || b.starts_with(b"fLaC")
        || b.starts_with(b"OggS")
        || riff(b, b"WAVE")
        || b.starts_with(b"MThd")
        || b.starts_with(b"FORM") && b.len() > 12 && matches!(&b[8..12], b"AIFF" | b"AIFC")
        || (b.len() > 2 && b[0] == 0xFF && b[1] & 0xE0 == 0xE0 && matches!(ext_of(name_hint).as_deref(), Some("mp3" | "aac" | "mp2")))
    {
        return bin(BinaryKind::Audio);
    }
    // Executables and object code.
    if b.starts_with(b"MZ") || b.starts_with(b"\x7FELF") || b.starts_with(b"\0asm") {
        return bin(BinaryKind::Executable);
    }
    if b.starts_with(&[0xFE, 0xED, 0xFA, 0xCE])
        || b.starts_with(&[0xFE, 0xED, 0xFA, 0xCF])
        || b.starts_with(&[0xCE, 0xFA, 0xED, 0xFE])
        || b.starts_with(&[0xCF, 0xFA, 0xED, 0xFE])
        || b.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE])
    {
        return bin(BinaryKind::Executable);
    }
    // Fonts.
    if b.starts_with(b"wOFF") || b.starts_with(b"wOF2") || b.starts_with(b"OTTO") || b.starts_with(b"ttcf") || b.starts_with(b"true") || looks_like_ttf(b) {
        return bin(BinaryKind::Font);
    }
    if b.starts_with(b"SQLite format 3\0") {
        return bin(BinaryKind::Database);
    }
    if b.starts_with(b"AT&TFORM") || b.starts_with(b"ITSF") || b.starts_with(b"%!PS") || (b.len() > 68 && &b[60..68] == b"BOOKMOBI") {
        return bin(BinaryKind::LegacyDocument);
    }
    None
}

fn riff(b: &[u8], kind: &[u8; 4]) -> bool {
    b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == kind
}

fn looks_like_ttf(b: &[u8]) -> bool {
    b.len() > 28 && b[..4] == [0, 1, 0, 0] && (1..=64).contains(&u16::from_be_bytes([b[4], b[5]])) && b[12..16].iter().all(u8::is_ascii_alphabetic)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn contains_utf16le(b: &[u8], s: &str) -> bool {
    let needle: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
    find(b, &needle).is_some()
}

// -------------------------------------------------------------------------------------------------- ZIP family

fn detect_zip(b: &[u8]) -> Detected {
    let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(b)) else {
        return Detected::new(Format::Zip);
    };
    let names: Vec<String> = zip.file_names().map(str::to_string).collect();
    let has = |n: &str| names.iter().any(|x| x == n);
    let encrypted = (0..zip.len()).any(|i| zip.by_index_raw(i).map(|f| f.encrypted()).unwrap_or(false));

    // The ODF and EPUB `mimetype` entry is stored first and uncompressed.
    let mimetype = if has("mimetype") {
        zip.by_name("mimetype").ok().and_then(|mut f| {
            let mut s = String::new();
            f.by_ref().take(256).read_to_string(&mut s).ok().map(|_| s.trim().to_string())
        })
    } else {
        None
    };
    let format = match mimetype.as_deref() {
        Some("application/epub+zip") => Format::Epub,
        Some("application/vnd.oasis.opendocument.text") | Some("application/vnd.oasis.opendocument.text-template") => Format::Odt,
        Some(m) if m.starts_with("application/vnd.oasis.opendocument.") => Format::Binary(BinaryKind::OtherOffice),
        _ if has("[Content_Types].xml") && has("word/document.xml") => Format::Docx,
        _ if has("[Content_Types].xml") && (has("xl/workbook.xml") || has("ppt/presentation.xml")) => Format::Binary(BinaryKind::OtherOffice),
        _ if has("META-INF/container.xml") && names.iter().any(|n| n.ends_with(".opf")) => Format::Epub,
        _ if has("META-INF/MANIFEST.MF") && names.iter().any(|n| n.ends_with(".class")) => Format::Binary(BinaryKind::Executable),
        _ => Format::Zip,
    };
    let mut d = Detected::new(format);
    d.encrypted = encrypted && matches!(format, Format::Zip | Format::Docx | Format::Odt | Format::Epub);
    d
}

// ------------------------------------------------------------------------------------------- gzip and tar

fn detect_gzip(b: &[u8]) -> Detected {
    let mut head = Vec::new();
    // A truncated or corrupt stream still yields whatever decoded before the error.
    let _ = GzDecoder::new(b).take(HEAD as u64).read_to_end(&mut head);
    if head.len() >= 512 && is_tar_header(&head[..512]) {
        let tex = tar_has_tex(GzDecoder::new(b).take(GZIP_SNIFF));
        return Detected::new(if tex { Format::ArxivSource } else { Format::TarGz });
    }
    let inner = detect(&head, None).format;
    let mut d = Detected::new(if inner == Format::Latex { Format::ArxivSource } else { Format::Gzip });
    d.inner = Some(inner);
    d
}

/// A POSIX ustar or GNU tar header, or a v7 header whose checksum is valid.
fn is_tar_header(h: &[u8]) -> bool {
    if h.len() < 512 {
        return false;
    }
    if &h[257..262] == b"ustar" {
        return true;
    }
    if h[0] == 0 {
        return false;
    }
    let field = &h[148..156];
    let digits: String = field.iter().take_while(|c| (b'0'..=b'7').contains(c)).map(|&c| c as char).collect();
    let Ok(stored) = u32::from_str_radix(digits.trim(), 8) else { return false };
    if digits.is_empty() {
        return false;
    }
    let sum: u32 = h
        .iter()
        .enumerate()
        .map(|(i, &c)| if (148..156).contains(&i) { 32u32 } else { c as u32 })
        .sum();
    sum == stored && h[..100].iter().take_while(|&&c| c != 0).all(|c| c.is_ascii_graphic() || *c == b' ')
}

fn tar_has_tex<R: Read>(r: R) -> bool {
    let mut ar = tar::Archive::new(r);
    let Ok(entries) = ar.entries() else { return false };
    for (n, e) in entries.enumerate() {
        if n >= TAR_SNIFF_ENTRIES {
            break;
        }
        let Ok(e) = e else { break };
        if let Ok(p) = e.path() {
            let name = p.to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".tex") {
                return true;
            }
        }
    }
    false
}

// ------------------------------------------------------------------------------------------------- text family

fn detect_text(b: &[u8], name_hint: Option<&str>) -> Detected {
    let head = &b[..b.len().min(HEAD)];
    let (bom, decoded_head) = match text::sniff_text_head(head) {
        Some(x) => x,
        None => return Detected::new(Format::Binary(BinaryKind::Unknown)),
    };
    let mut d = Detected::new(Format::PlainText);
    d.bom = match bom {
        TextBom::Utf8 => Some(TextBomName::Utf8),
        TextBom::Utf16Le => Some(TextBomName::Utf16Le),
        TextBom::Utf16Be => Some(TextBomName::Utf16Be),
        TextBom::None => None,
    };
    let trimmed = decoded_head.trim_start_matches(['\u{FEFF}', ' ', '\t', '\r', '\n']);
    let lower_head: String = trimmed.chars().take(8192).collect::<String>().to_ascii_lowercase();
    let ext = ext_of(name_hint);

    if trimmed.starts_with('<') {
        match sniff_xml(trimmed, &lower_head) {
            XmlKind::Format(f, root) => {
                d.format = f;
                d.xml_root = root;
                return d;
            }
            XmlKind::NotXml => {}
        }
    }
    if lower_head.contains("<!doctype html") || lower_head.contains("<html") || (lower_head.contains("<body") && lower_head.contains("</")) {
        d.format = Format::Html;
        return d;
    }
    if trimmed.starts_with('<') && html_fragment_score(&lower_head) >= 3 {
        d.format = Format::Html;
        return d;
    }
    if trimmed.starts_with('{') && decoded_head.contains("\"cells\"") && is_notebook(b) {
        d.format = Format::Notebook;
        return d;
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if serde_json::from_slice::<serde::de::IgnoredAny>(b).is_ok() {
            d.format = Format::Json;
            return d;
        }
    }
    if trimmed.starts_with("{\\rtf") {
        d.format = Format::Rtf;
        return d;
    }
    if trimmed.starts_with("%!PS") {
        d.format = Format::Binary(BinaryKind::LegacyDocument);
        return d;
    }
    if is_latex(trimmed) {
        d.format = Format::Latex;
        return d;
    }
    if markdown_score(trimmed, ext.as_deref()) >= 2 {
        d.format = Format::Markdown;
        return d;
    }
    d
}

enum XmlKind {
    Format(Format, Option<String>),
    NotXml,
}

/// Find the root element of an XML-looking head and classify it.
fn sniff_xml(s: &str, lower: &str) -> XmlKind {
    let mut doctype = String::new();
    let mut rest = s;
    let mut saw_decl = false;
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix("<?") {
            saw_decl = true;
            match r.find("?>") {
                Some(i) => rest = &r[i + 2..],
                None => return XmlKind::NotXml,
            }
        } else if let Some(r) = rest.strip_prefix("<!--") {
            match r.find("-->") {
                Some(i) => rest = &r[i + 3..],
                None => return XmlKind::NotXml,
            }
        } else if rest.len() >= 9 && rest[..9].eq_ignore_ascii_case("<!doctype") {
            // The internal subset may contain '>' inside brackets.
            let mut depth = 0i32;
            let mut end = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    '>' if depth <= 0 => {
                        end = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { return XmlKind::NotXml };
            doctype = rest[..end].to_ascii_lowercase();
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    let Some(tag) = rest.strip_prefix('<') else { return XmlKind::NotXml };
    let name_end = tag.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(tag.len());
    let qname = &tag[..name_end];
    if qname.is_empty() || !qname.chars().all(|c| c.is_alphanumeric() || matches!(c, ':' | '_' | '-' | '.')) {
        return XmlKind::NotXml;
    }
    let local = qname.rsplit(':').next().unwrap_or(qname).to_string();
    let local_lc = local.to_ascii_lowercase();
    let tag_end = tag.find('>').unwrap_or(tag.len());
    let root_tag = tag[..tag_end].to_ascii_lowercase();

    let jats_doctype = doctype.contains("jats") || doctype.contains("nlm") || doctype.contains("journalpublishing") || doctype.contains("archivearticle") || doctype.contains("bits");
    let jats_ns = root_tag.contains("jats.nlm.nih.gov") || root_tag.contains("ncbi.nlm.nih.gov/pmc/");
    let jats_shape = lower.contains("<article-meta") || lower.contains("<front>") && lower.contains("<journal-meta");
    let docbook = root_tag.contains("docbook.org") || doctype.contains("docbook");

    match local_lc.as_str() {
        "article" | "book" | "book-part" | "book-part-wrapper" | "pmc-articleset" if !docbook && (jats_doctype || jats_ns || jats_shape || root_tag.contains("article-type")) => {
            XmlKind::Format(Format::Jats, Some(local))
        }
        "tei" | "teicorpus" => XmlKind::Format(Format::Tei, Some(local)),
        _ if root_tag.contains("tei-c.org") && local_lc != "html" => XmlKind::Format(Format::Tei, Some(local)),
        "html" => {
            let xhtml = root_tag.contains("w3.org/1999/xhtml") || (saw_decl && doctype.contains("xhtml"));
            XmlKind::Format(if xhtml { Format::Xhtml } else { Format::Html }, Some(local))
        }
        "svg" => XmlKind::Format(Format::Binary(BinaryKind::Image), Some(local)),
        _ => {
            // HTML snippets and unknown tags without a declaration are not XML documents; give the HTML
            // sniffer a chance when there is no XML prolog and the tag is an HTML one.
            if !saw_decl && doctype.is_empty() && is_html_tag(&local_lc) {
                return XmlKind::NotXml;
            }
            if !tag.contains('>') {
                return XmlKind::NotXml;
            }
            XmlKind::Format(Format::Xml, Some(local))
        }
    }
}

fn is_html_tag(name: &str) -> bool {
    matches!(
        name,
        "head" | "body" | "div" | "p" | "span" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "table" | "section" | "main" | "a" | "br" | "img" | "pre" | "em" | "strong" | "blockquote" | "meta" | "title" | "link" | "script" | "style" | "nav" | "header" | "footer" | "b" | "i"
    )
}

fn html_fragment_score(lower: &str) -> usize {
    ["<p>", "<p ", "<div", "<h1", "<h2", "<h3", "<br", "<a href", "<table", "<span", "<ul", "<li", "<em>", "<strong>", "<pre"]
        .iter()
        .filter(|t| lower.contains(*t))
        .count()
}

/// Cheap but exact: a notebook has a `cells` array and an `nbformat` number.
fn is_notebook(b: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct Nb {
        cells: Vec<serde::de::IgnoredAny>,
        #[serde(default)]
        nbformat: Option<u32>,
    }
    serde_json::from_slice::<Nb>(b).map(|nb| nb.nbformat.is_some() || !nb.cells.is_empty()).unwrap_or(false)
}

fn is_latex(s: &str) -> bool {
    let head: String = s.chars().take(16 * 1024).collect();
    if head.contains("\\documentclass") || head.contains("\\begin{document}") {
        return true;
    }
    let hits = ["\\usepackage", "\\section{", "\\begin{equation}", "\\newcommand", "\\input{", "\\chapter{"]
        .iter()
        .filter(|t| head.contains(*t))
        .count();
    hits >= 2
}

/// Count independent Markdown signals in the head of the text.
fn markdown_score(s: &str, ext: Option<&str>) -> usize {
    let mut score = 0;
    if matches!(ext, Some("md" | "markdown" | "mdown" | "mkd" | "mdx" | "qmd" | "rmd")) {
        score += 1;
    }
    let head: String = s.chars().take(32 * 1024).collect();
    let lines: Vec<&str> = head.lines().collect();
    if lines.first().is_some_and(|l| l.trim_end() == "---") && lines.iter().skip(1).take(60).any(|l| l.trim_end() == "---" || l.trim_end() == "...") {
        score += 2;
    }
    let atx = lines.iter().filter(|l| is_atx_heading(l)).count();
    if atx >= 1 {
        score += 1;
    }
    if atx >= 3 {
        score += 1;
    }
    if lines.iter().any(|l| l.starts_with("```") || l.starts_with("~~~")) {
        score += 1;
    }
    if lines.iter().any(|l| l.trim_start().starts_with("|") && l.contains("---") && l.contains('|')) {
        score += 1;
    }
    let bullets = lines.iter().filter(|l| matches!(l.trim_start().get(..2), Some("- " | "* " | "+ "))).count();
    if bullets >= 2 {
        score += 1;
    }
    if has_md_link(&head) {
        score += 1;
    }
    if head.contains("**") && head.matches("**").count() >= 2 {
        score += 1;
    }
    score
}

fn is_atx_heading(l: &str) -> bool {
    let t = l.trim_end();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') && t[hashes..].trim().len() > 0
}

fn has_md_link(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s[i..].find("](") {
        let at = i + p;
        // Needs an opening bracket earlier on the same line, and a closing parenthesis soon after.
        let line_start = s[..at].rfind('\n').map(|x| x + 1).unwrap_or(0);
        if s[line_start..at].contains('[') && bytes[at..].iter().take(300).any(|&c| c == b')') {
            return true;
        }
        i = at + 2;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (n, c) in entries {
            w.start_file(*n, opts).unwrap();
            w.write_all(c).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn tar_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (n, c) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(c.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, n, *c).unwrap();
        }
        b.into_inner().unwrap()
    }

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn fmt(b: &[u8]) -> Format {
        detect(b, None).format
    }

    #[test]
    fn empty_and_pdf() {
        assert_eq!(fmt(b""), Format::Empty);
        assert_eq!(fmt(b"%PDF-1.7\n1 0 obj\n"), Format::Pdf);
        // Junk before the header is tolerated; the extension is ignored.
        assert_eq!(detect(b"\r\n\r\n%PDF-1.4\n", Some("notes.txt")).format, Format::Pdf);
    }

    #[test]
    fn encrypted_pdf_is_flagged() {
        let d = detect(b"%PDF-1.6\ntrailer\n<< /Root 1 0 R /Encrypt 9 0 R >>\n%%EOF", None);
        assert_eq!(d.format, Format::Pdf);
        assert!(d.encrypted);
        assert!(!detect(b"%PDF-1.6\ntrailer\n<< /Root 1 0 R >>\n%%EOF", None).encrypted);
    }

    #[test]
    fn zip_family_by_container_contents() {
        let epub = zip_bytes(&[("mimetype", b"application/epub+zip"), ("META-INF/container.xml", b"<container/>")]);
        assert_eq!(fmt(&epub), Format::Epub);
        let odt = zip_bytes(&[("mimetype", b"application/vnd.oasis.opendocument.text"), ("content.xml", b"<x/>")]);
        assert_eq!(fmt(&odt), Format::Odt);
        let ods = zip_bytes(&[("mimetype", b"application/vnd.oasis.opendocument.spreadsheet"), ("content.xml", b"<x/>")]);
        assert_eq!(fmt(&ods), Format::Binary(BinaryKind::OtherOffice));
        let docx = zip_bytes(&[("[Content_Types].xml", b"<Types/>"), ("word/document.xml", b"<w:document/>")]);
        assert_eq!(fmt(&docx), Format::Docx);
        let xlsx = zip_bytes(&[("[Content_Types].xml", b"<Types/>"), ("xl/workbook.xml", b"<workbook/>")]);
        assert_eq!(fmt(&xlsx), Format::Binary(BinaryKind::OtherOffice));
        let plain = zip_bytes(&[("a.txt", b"hello"), ("b/c.md", b"# x")]);
        assert_eq!(fmt(&plain), Format::Zip);
        // A docx saved as .zip is still a docx; a plain zip named .docx is still a zip.
        assert_eq!(detect(&docx, Some("thing.zip")).format, Format::Docx);
        assert_eq!(detect(&plain, Some("thing.docx")).format, Format::Zip);
    }

    #[test]
    fn tar_and_gzip_variants() {
        let tar = tar_bytes(&[("a.txt", b"hello"), ("dir/b.txt", b"world")]);
        assert_eq!(fmt(&tar), Format::Tar);
        assert_eq!(fmt(&gz(&tar)), Format::TarGz);
        let src = tar_bytes(&[("main.tex", b"\\documentclass{article}"), ("fig.png", b"x")]);
        assert_eq!(fmt(&src), Format::ArxivSource);
        assert_eq!(fmt(&gz(&src)), Format::ArxivSource);
        // arXiv also serves a single gzipped .tex.
        assert_eq!(fmt(&gz(b"\\documentclass{article}\n\\begin{document}\nHi\n\\end{document}\n")), Format::ArxivSource);
        let d = detect(&gz(b"# Title\n\nSome *text* with [a link](http://x.y).\n"), None);
        assert_eq!(d.format, Format::Gzip);
        assert_eq!(d.inner, Some(Format::Markdown));
    }

    #[test]
    fn known_binaries() {
        use BinaryKind::*;
        let cases: Vec<(&[u8], BinaryKind)> = vec![
            (b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", Image),
            (b"\xFF\xD8\xFF\xE0\0\x10JFIF", Image),
            (b"GIF89a\x01\0\x01\0", Image),
            (b"RIFF\x10\0\0\0WEBPVP8 ", Image),
            (b"ID3\x04\0\0\0\0\0\0", Audio),
            (b"fLaC\0\0\0\"", Audio),
            (b"OggS\0\x02", Audio),
            (b"RIFF\x24\0\0\0WAVEfmt ", Audio),
            (b"\0\0\0\x18ftypmp42\0\0\0\0", Video),
            (b"\0\0\0\x18ftypM4A \0\0\0\0", Audio),
            (b"\x1A\x45\xDF\xA3\x01\0", Video),
            (b"MZ\x90\0\x03\0\0\0\x04\0\0\0", Executable),
            (b"\x7FELF\x02\x01\x01\0", Executable),
            (b"\xCF\xFA\xED\xFE\x07\0\0\x01", Executable),
            (b"wOFF\0\x01\0\0", Font),
            (b"OTTO\0\x0b\x01\0", Font),
            (b"\0\x01\0\0\0\x0e\0\x80\0\x03\0\x30glyf\0\0\0\0\0\0\0\0\0\0\0\0\0\0", Font),
            (b"7z\xBC\xAF\x27\x1C\0\x04", CompressedArchive),
            (b"Rar!\x1A\x07\x01\0", CompressedArchive),
            (b"SQLite format 3\0\x10\0", Database),
            (b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1\0\0", LegacyDocument),
        ];
        for (bytes, kind) in cases {
            assert_eq!(fmt(bytes), Format::Binary(kind), "{:?}", &bytes[..bytes.len().min(8)]);
        }
        // Names lie; bytes decide.
        assert_eq!(detect(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", Some("paper.pdf")).format, Format::Binary(Image));
        assert_eq!(detect(b"MZ\x90\0\x03\0", Some("readme.txt")).format, Format::Binary(Executable));
    }

    #[test]
    fn encrypted_ooxml_is_flagged() {
        let mut b = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        b.extend(std::iter::repeat_n(0u8, 400));
        b.extend("EncryptedPackage".encode_utf16().flat_map(u16::to_le_bytes));
        let d = detect(&b, Some("x.docx"));
        assert_eq!(d.format, Format::Binary(BinaryKind::LegacyDocument));
        assert!(d.encrypted);
    }

    #[test]
    fn jats_with_and_without_doctype() {
        let with_dtd = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD with MathML3 v1.2 20190208//EN" "JATS-archivearticle1-mathml3.dtd">
<article article-type="research-article" xmlns:xlink="http://www.w3.org/1999/xlink"><front/></article>"#;
        assert_eq!(fmt(with_dtd.as_bytes()), Format::Jats);
        let ns = r#"<article xmlns="https://jats.nlm.nih.gov/ns/archiving/1.3/" article-type="x"><front><journal-meta/></front></article>"#;
        assert_eq!(fmt(ns.as_bytes()), Format::Jats);
        let shape = "<article><front><journal-meta/><article-meta/></front><body/></article>";
        assert_eq!(fmt(shape.as_bytes()), Format::Jats);
        // A DocBook <article> is not JATS.
        let docbook = r#"<?xml version="1.0"?><article xmlns="http://docbook.org/ns/docbook"><title>T</title></article>"#;
        let d = detect(docbook.as_bytes(), Some("x.xml"));
        assert_eq!(d.format, Format::Xml);
        assert_eq!(d.xml_root.as_deref(), Some("article"));
    }

    #[test]
    fn tei_root_and_namespace() {
        let tei = r#"<?xml version="1.0" encoding="UTF-8"?><TEI xmlns="http://www.tei-c.org/ns/1.0"><teiHeader/></TEI>"#;
        assert_eq!(fmt(tei.as_bytes()), Format::Tei);
        let nested = r#"<tei:TEI xmlns:tei="http://www.tei-c.org/ns/1.0"><tei:teiHeader/></tei:TEI>"#;
        assert_eq!(fmt(nested.as_bytes()), Format::Tei);
    }

    #[test]
    fn html_and_xhtml() {
        assert_eq!(fmt(b"<!DOCTYPE html>\n<html><body><p>x</p></body></html>"), Format::Html);
        assert_eq!(fmt(b"  \n<HTML><HEAD></HEAD></HTML>"), Format::Html);
        let xhtml = br#"<?xml version="1.0" encoding="utf-8"?><!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "x.dtd"><html xmlns="http://www.w3.org/1999/xhtml"><body/></html>"#;
        assert_eq!(fmt(xhtml), Format::Xhtml);
        assert_eq!(fmt(b"<div><h1>A</h1><p>b</p><p>c</p></div>"), Format::Html);
        // HTML saved with a .md extension is still HTML.
        assert_eq!(detect(b"<!doctype html><html><body>x</body></html>", Some("a.md")).format, Format::Html);
    }

    #[test]
    fn xml_that_is_neither() {
        let d = detect(b"<?xml version=\"1.0\"?><feed><entry/></feed>", None);
        assert_eq!(d.format, Format::Xml);
        assert_eq!(d.xml_root.as_deref(), Some("feed"));
        assert_eq!(fmt(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle/></svg>"), Format::Binary(BinaryKind::Image));
    }

    #[test]
    fn notebook_json_and_plain_json() {
        let nb = br##"{"cells":[{"cell_type":"markdown","source":["# Hi"]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"##;
        assert_eq!(fmt(nb), Format::Notebook);
        assert_eq!(detect(nb, Some("data.txt")).format, Format::Notebook);
        assert_eq!(fmt(br#"{"a":1,"b":[1,2]}"#), Format::Json);
        // A "cells" key without notebook shape is just JSON.
        assert_eq!(fmt(br#"{"cells":"none","x":1}"#), Format::Json);
    }

    #[test]
    fn markdown_versus_plain_text() {
        assert_eq!(fmt(b"# Title\n\nSome text with a [link](http://a.b) and more.\n"), Format::Markdown);
        assert_eq!(fmt(b"---\ntitle: x\n---\n\nBody\n"), Format::Markdown);
        assert_eq!(fmt(b"Just a few plain lines.\nNothing special here.\n"), Format::PlainText);
        // The extension is a tie-break, not an override.
        assert_eq!(detect(b"Just a few plain lines.\n", Some("a.md")).format, Format::PlainText);
        assert_eq!(detect(b"# Heading\nbody\n", Some("a.md")).format, Format::Markdown);
        assert_eq!(detect(b"# Heading\nbody\n", Some("a.txt")).format, Format::PlainText);
        // A shell comment is not a Markdown heading without a space.
        assert_eq!(fmt(b"#!/bin/sh\n#comment\necho hi\n"), Format::PlainText);
    }

    #[test]
    fn latex_source() {
        assert_eq!(fmt(b"\\documentclass{article}\n\\begin{document}\nHi\n\\end{document}\n"), Format::Latex);
        assert_eq!(fmt(b"\\section{A}\ntext\n\\usepackage{x}\n"), Format::Latex);
        assert_eq!(fmt(b"The \\section command is documented elsewhere.\n"), Format::PlainText);
    }

    #[test]
    fn utf16_text_and_binary_garbage() {
        let mut u16le = vec![0xFF, 0xFE];
        u16le.extend("Hello, world\n".encode_utf16().flat_map(u16::to_le_bytes));
        let d = detect(&u16le, None);
        assert_eq!(d.format, Format::PlainText);
        assert_eq!(d.bom, Some(TextBomName::Utf16Le));
        assert_eq!(fmt(&[0x00, 0x9F, 0x01, 0x02, 0x03, 0x00, 0xFE, 0x10, 0x00, 0x00]), Format::Binary(BinaryKind::Unknown));
    }

    #[test]
    fn rtf_and_legacy_text_formats() {
        assert_eq!(fmt(b"{\\rtf1\\ansi Hello}"), Format::Rtf);
        assert_eq!(fmt(b"%!PS-Adobe-3.0\n"), Format::Binary(BinaryKind::LegacyDocument));
    }

    #[test]
    fn latin1_text_is_text() {
        // "café" in Latin-1 is not valid UTF-8 but is still text.
        assert_eq!(fmt(b"Un caf\xe9 tr\xe8s chaud, s'il vous pla\xeet.\n"), Format::PlainText);
    }
}

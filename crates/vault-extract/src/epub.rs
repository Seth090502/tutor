//! EPUB (2 and 3) to canonical-ready Markdown: the spine's XHTML documents in reading order through the HTML
//! walker, with the OPF's metadata and the printed page labels from the page list or inline page-break markers.

use std::collections::HashMap;

use roxmltree::Node;

use crate::ConvertOptions;
use crate::archive::ZipDoc;
use crate::builder::{MdBuilder, collapse_ws};
use crate::error::{ExtractError, Result};
use crate::html::{self, HtmlOptions, Walker};
use crate::ids;
use crate::types::{Extracted, Metadata, tool_string};
use crate::xmlutil::{self, attr, child, children_named, is, name, text_of};

/// Font-obfuscation algorithms are not DRM; anything else listed in encryption.xml is.
const OBFUSCATION: [&str; 2] = ["http://www.idpf.org/2008/embedding", "http://ns.adobe.com/pdf/enc#RC"];

struct Item {
    href: String,
    media_type: String,
    properties: String,
}

pub fn convert(bytes: &[u8], opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut zip = ZipDoc::open(bytes, "epub")?;
    check_drm(&mut zip)?;

    let container = zip
        .read("META-INF/container.xml")?
        .ok_or_else(|| ExtractError::corrupt("epub", "META-INF/container.xml is missing"))?;
    let opf_path = xmlutil::with_document(&container, "epub", |doc, _| {
        let root = doc.root_element();
        let rf = root
            .descendants()
            .find(|n| is(*n, "rootfile"))
            .ok_or_else(|| ExtractError::corrupt("epub", "container.xml names no rootfile"))?;
        Ok(attr(rf, "full-path").unwrap_or("").to_string())
    })?;
    let opf_bytes = zip
        .read(&opf_path)?
        .ok_or_else(|| ExtractError::corrupt("epub", format!("package document {opf_path} is missing")))?;
    let base = opf_path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();

    let (meta, manifest, spine) =
        xmlutil::with_document(&opf_bytes, "epub", |doc, _| Ok(read_package(doc.root_element())))?;
    let mut warnings = Vec::new();

    // Page labels from the navigation document or the NCX: (document path, element id) -> label.
    let mut page_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    for item in manifest.values() {
        let is_nav = item.properties.split_whitespace().any(|p| p == "nav");
        let is_ncx = item.media_type == "application/x-dtbncx+xml";
        if !(is_nav || is_ncx) {
            continue;
        }
        let path = resolve(&base, &item.href);
        let Some(data) = zip.read(&path)? else { continue };
        let dir = path.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
        let found = if is_nav { page_list_from_nav(&data, &dir, opts) } else { page_list_from_ncx(&data, &dir) };
        for (doc_path, id, label) in found {
            page_map.entry(doc_path).or_default().insert(id, label);
        }
    }

    let mut b = MdBuilder::new();
    if let Some(t) = &meta.title {
        b.heading(1, t);
    }
    if !meta.authors.is_empty() {
        b.paragraph(&meta.authors.join(", "));
    }

    let mut refs = Vec::new();
    let mut doc_count = 0usize;
    for idref in &spine {
        let Some(item) = manifest.get(idref) else {
            tracing::warn!(idref = %idref, "spine item is not in the manifest");
            warnings.push(format!("spine item {idref} is not in the manifest"));
            continue;
        };
        if !(item.media_type == "application/xhtml+xml"
            || item.media_type == "text/html"
            || item.media_type == "application/x-dtbook+xml")
        {
            continue;
        }
        if item.properties.split_whitespace().any(|p| p == "nav") {
            continue;
        }
        let path = resolve(&base, &item.href);
        let Some(data) = zip.read(&path)? else {
            tracing::warn!(path = %path, "spine document is missing from the archive");
            warnings.push(format!("spine document {path} is missing from the archive"));
            continue;
        };
        let mut w2 = Vec::new();
        let (src, _enc) = html::decode_html(&data, opts, &mut w2);
        warnings.append(&mut w2);
        let doc = match html::parse_chapter(&src) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(path = %path, error = %e, "spine document skipped");
                warnings.push(format!("{path}: {e}"));
                continue;
            }
        };
        let root = html::chapter_body(&doc);
        // The chapter's top heading sits at level 2, below the book title.
        let offset = html::min_heading_level(root).map(|m| 2 - m as i8).unwrap_or(0);
        let targets = page_map.remove(&path).unwrap_or_default();
        let mut w = Walker::new(&mut b, HtmlOptions { heading_offset: offset }).with_page_targets(targets);
        w.run(root);
        let (mut r, mut warn, _, _) = w.into_parts();
        refs.append(&mut r);
        warnings.append(&mut warn);
        doc_count += 1;
    }
    if doc_count == 0 {
        return Err(ExtractError::convert("epub", "the spine lists no readable XHTML document"));
    }
    let f = b.finish();
    Ok(Extracted {
        markdown: f.markdown,
        headings: f.headings,
        pages: f.pages,
        blocks: f.blocks,
        metadata: meta,
        references: refs,
        warnings,
        tool: tool_string("epub"),
        encoding: Some("utf-8".to_string()),
    })
}

/// DRM: Adobe's rights.xml, or an encryption.xml that encrypts something other than fonts.
fn check_drm(zip: &mut ZipDoc) -> Result<()> {
    if zip.has("META-INF/rights.xml") {
        return Err(ExtractError::Protected("EPUB carries rights.xml (DRM)".to_string()));
    }
    if let Some(enc) = zip.read("META-INF/encryption.xml")? {
        let protected = xmlutil::with_document(&enc, "epub", |doc, _| {
            Ok(doc
                .descendants()
                .filter(|n| is(*n, "EncryptionMethod"))
                .any(|m| !OBFUSCATION.contains(&attr(m, "Algorithm").unwrap_or(""))))
        })?;
        if protected {
            return Err(ExtractError::Protected("EPUB content is encrypted (DRM)".to_string()));
        }
    }
    Ok(())
}

/// Join an href to a directory and resolve `.` and `..`, percent-decoding the result.
fn resolve(dir: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or("");
    let decoded = percent_decode(href);
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for c in decoded.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_package(root: Node) -> (Metadata, HashMap<String, Item>, Vec<String>) {
    let mut m = Metadata::default();
    if let Some(md) = child(root, "metadata") {
        for e in md.children().filter(Node::is_element) {
            let v = text_of(e);
            if v.is_empty() {
                continue;
            }
            match name(e) {
                "title" if m.title.is_none() => m.title = Some(v),
                "creator" => {
                    if !m.authors.contains(&v) {
                        m.authors.push(v);
                    }
                }
                "language" if m.language.is_none() => m.language = Some(v),
                "date" if m.year.is_none() => m.year = ids::year_in(&v),
                "subject" => {
                    if !m.keywords.contains(&v) {
                        m.keywords.push(v);
                    }
                }
                "description" if m.abstract_text.is_none() => m.abstract_text = Some(v),
                "identifier" => {
                    let scheme = attr(e, "scheme").unwrap_or("").to_ascii_lowercase();
                    let lower = v.to_ascii_lowercase();
                    if scheme == "doi"
                        || lower.starts_with("doi:")
                        || lower.contains("doi.org/")
                        || lower.starts_with("urn:doi:")
                    {
                        m.doi = m.doi.take().or_else(|| ids::find_doi(&v));
                    } else if (scheme == "isbn"
                        || lower.starts_with("urn:isbn:")
                        || lower.starts_with("isbn")
                        || ids::normalize_isbn(&v).is_some())
                        && let Some(i) = ids::normalize_isbn(&v).or_else(|| ids::find_isbns(&v).into_iter().next())
                        && !m.isbn.contains(&i)
                    {
                        m.isbn.push(i);
                    }
                }
                _ => {}
            }
        }
    }
    let mut manifest = HashMap::new();
    if let Some(mf) = child(root, "manifest") {
        for it in children_named(mf, "item") {
            if let (Some(id), Some(href)) = (attr(it, "id"), attr(it, "href")) {
                manifest.insert(
                    id.to_string(),
                    Item {
                        href: href.to_string(),
                        media_type: attr(it, "media-type").unwrap_or("").to_string(),
                        properties: attr(it, "properties").unwrap_or("").to_string(),
                    },
                );
            }
        }
    }
    let sp = child(root, "spine");
    let spine = sp
        .map(|s| children_named(s, "itemref").filter_map(|i| attr(i, "idref").map(str::to_string)).collect())
        .unwrap_or_default();
    (m, manifest, spine)
}

/// `(document path, element id, label)` from an EPUB 3 navigation document's `page-list`.
fn page_list_from_nav(data: &[u8], dir: &str, opts: &ConvertOptions) -> Vec<(String, String, String)> {
    let mut w = Vec::new();
    let (src, _) = html::decode_html(data, opts, &mut w);
    let Ok(doc) = html::parse_chapter(&src) else { return Vec::new() };
    let nav_sel = scraper::Selector::parse("nav").expect("static selector");
    let a_sel = scraper::Selector::parse("a").expect("static selector");
    let mut out = Vec::new();
    for nav in doc.select(&nav_sel) {
        let ty = nav.value().attr("epub:type").unwrap_or("");
        if !ty.split_whitespace().any(|t| t == "page-list") {
            continue;
        }
        for a in nav.select(&a_sel) {
            let Some(href) = a.value().attr("href") else { continue };
            let label = collapse_ws(&a.text().collect::<String>());
            if let Some((file, id)) = href.split_once('#')
                && !label.is_empty()
                && !id.is_empty()
            {
                out.push((resolve(dir, file), id.to_string(), label));
            }
        }
    }
    out
}

/// The same from an EPUB 2 NCX `pageList`.
fn page_list_from_ncx(data: &[u8], dir: &str) -> Vec<(String, String, String)> {
    xmlutil::with_document(data, "epub", |doc, _| {
        let mut out = Vec::new();
        for pt in doc.descendants().filter(|n| is(*n, "pageTarget")) {
            let label = child(pt, "navLabel")
                .map(text_of)
                .filter(|l| !l.is_empty())
                .or_else(|| attr(pt, "value").map(str::to_string));
            let src = child(pt, "content").and_then(|c| attr(c, "src"));
            if let (Some(label), Some(src)) = (label, src)
                && let Some((file, id)) = src.split_once('#')
            {
                out.push((resolve(dir, file), id.to_string(), label));
            }
        }
        Ok(out)
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;
    use std::io::{Cursor, Write};

    fn build(files: &[(&str, &str)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.start_file("mimetype", stored).unwrap();
        w.write_all(b"application/epub+zip").unwrap();
        for (n, c) in files {
            w.start_file(*n, deflated).unwrap();
            w.write_all(c.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    const CONTAINER: &str = r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;

    const OPF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>The Synthetic Book</dc:title><dc:creator>Ann Lee</dc:creator><dc:creator>Bo Chen</dc:creator>
<dc:identifier id="id">urn:isbn:9780306406157</dc:identifier><dc:identifier>doi:10.1234/book.1</dc:identifier>
<dc:language>en-GB</dc:language><dc:date>2012-05-01</dc:date><dc:subject>testing</dc:subject>
</metadata>
<manifest>
<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
<item id="c1" href="text/ch%201.xhtml" media-type="application/xhtml+xml"/>
<item id="c2" href="text/ch2.xhtml" media-type="application/xhtml+xml"/>
<item id="img" href="cover.png" media-type="image/png"/>
</manifest>
<spine><itemref idref="nav"/><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#;

    const NAV: &str = r##"<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body>
<nav epub:type="toc"><ol><li><a href="text/ch%201.xhtml">Chapter One</a></li></ol></nav>
<nav epub:type="page-list"><ol><li><a href="text/ch%201.xhtml#pg_iv">iv</a></li><li><a href="text/ch%201.xhtml#pg_1">1</a></li><li><a href="text/ch2.xhtml#pg_2">2</a></li></ol></nav></body></html>"##;

    const CH1: &str = r##"<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>One</title></head><body>
<h1>Chapter One</h1><span id="pg_iv"/><p>Opening paragraph of the first chapter.</p><span id="pg_1"/>
<h2>A Section</h2><p>Section text<a epub:type="noteref" href="#n1">1</a>.</p>
<aside epub:type="footnote" id="n1"><p>The footnote text.</p></aside>
<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table></body></html>"##;

    const CH2: &str = r##"<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml"><body><h2>Chapter Two</h2><p><span id="pg_2"/>Second chapter text.</p><nav><a href="x">skip me</a></nav></body></html>"##;

    fn epub() -> Vec<u8> {
        build(&[
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", OPF),
            ("OEBPS/nav.xhtml", NAV),
            ("OEBPS/text/ch 1.xhtml", CH1),
            ("OEBPS/text/ch2.xhtml", CH2),
        ])
    }

    #[test]
    fn metadata_and_reading_order() {
        let r = convert(&epub(), &ConvertOptions::default()).unwrap();
        let m = &r.metadata;
        assert_eq!(m.title.as_deref(), Some("The Synthetic Book"));
        assert_eq!(m.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(m.isbn, vec!["9780306406157"]);
        assert_eq!(m.doi.as_deref(), Some("10.1234/book.1"));
        assert_eq!(m.language.as_deref(), Some("en-GB"));
        assert_eq!(m.year, Some(2012));
        assert_eq!(m.keywords, vec!["testing"]);
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        // Each chapter's top heading sits at level 2 whatever level the XHTML used.
        assert_eq!(hs, vec![(1, "The Synthetic Book"), (2, "Chapter One"), (3, "A Section"), (2, "Chapter Two")]);
        assert!(!r.markdown.contains("skip me") && !r.markdown.contains("Chapter One\n\n- "), "{}", r.markdown);
        assert!(r.markdown.contains("Ann Lee, Bo Chen\n\n## Chapter One\n\nOpening paragraph"), "{}", r.markdown);
        assert!(r.markdown.contains("Section text[^1]."), "{}", r.markdown);
        assert!(r.markdown.contains("[^1]: The footnote text."), "{}", r.markdown);
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Table).count(), 1);
    }

    #[test]
    fn page_list_labels_become_pages() {
        let r = convert(&epub(), &ConvertOptions::default()).unwrap();
        let labels: Vec<_> = r.pages.iter().map(|p| p.label.clone().unwrap_or_default()).collect();
        assert_eq!(labels, vec!["iv", "1", "2"]);
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert_eq!(lines[r.pages[0].line as usize - 1], "Opening paragraph of the first chapter.");
        assert_eq!(lines[r.pages[1].line as usize - 1], "### A Section");
        assert_eq!(lines[r.pages[2].line as usize - 1], "Second chapter text.");
        assert_eq!(r.pages.iter().map(|p| p.index).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn inline_pagebreak_markers_work_without_a_page_list() {
        let ch = r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body><h1>T</h1><p>One.</p><span epub:type="pagebreak" title="7" id="p7"/><p>Two.</p></body></html>"#;
        let opf = OPF
            .replace(
                "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>",
                "",
            )
            .replace("<itemref idref=\"nav\"/>", "")
            .replace("<itemref idref=\"c2\"/>", "");
        let z =
            build(&[("META-INF/container.xml", CONTAINER), ("OEBPS/content.opf", &opf), ("OEBPS/text/ch 1.xhtml", ch)]);
        let r = convert(&z, &ConvertOptions::default()).unwrap();
        assert_eq!(r.pages.len(), 1);
        assert_eq!(r.pages[0].label.as_deref(), Some("7"));
    }

    #[test]
    fn drm_is_protected_but_font_obfuscation_is_not() {
        let enc = |alg: &str| {
            format!(
                r#"<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container" xmlns:enc="http://www.w3.org/2001/04/xmlenc#"><enc:EncryptedData><enc:EncryptionMethod Algorithm="{alg}"/></enc:EncryptedData></encryption>"#
            )
        };
        let base: Vec<(&str, &str)> = vec![
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", OPF),
            ("OEBPS/nav.xhtml", NAV),
            ("OEBPS/text/ch 1.xhtml", CH1),
            ("OEBPS/text/ch2.xhtml", CH2),
        ];
        let mut drm = base.clone();
        let e = enc("http://www.w3.org/2001/04/xmlenc#aes128-cbc");
        drm.push(("META-INF/encryption.xml", &e));
        let err = convert(&build(&drm), &ConvertOptions::default()).unwrap_err();
        assert_eq!(err.reason_code(), "protected");
        let mut fonts = base.clone();
        let e = enc("http://www.idpf.org/2008/embedding");
        fonts.push(("META-INF/encryption.xml", &e));
        assert!(convert(&build(&fonts), &ConvertOptions::default()).is_ok());
        let mut adobe = base;
        adobe.push(("META-INF/rights.xml", "<rights/>"));
        assert_eq!(convert(&build(&adobe), &ConvertOptions::default()).unwrap_err().reason_code(), "protected");
    }

    #[test]
    fn broken_epubs_report_corruption() {
        let no_container = build(&[("x.txt", "hello")]);
        assert_eq!(convert(&no_container, &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        let no_opf = build(&[("META-INF/container.xml", CONTAINER)]);
        assert_eq!(convert(&no_opf, &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
        assert_eq!(convert(b"not a zip at all", &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
    }

    #[test]
    fn resolve_handles_dots_and_percent_escapes() {
        assert_eq!(resolve("OEBPS/", "text/ch%201.xhtml#a"), "OEBPS/text/ch 1.xhtml");
        assert_eq!(resolve("OEBPS/text/", "../img/a.png"), "OEBPS/img/a.png");
        assert_eq!(resolve("", "./a.xhtml"), "a.xhtml");
    }
}

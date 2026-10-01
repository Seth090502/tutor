//! Detection plus routing: bytes in, an [`Extraction`] out, following SPEC section 8.

use serde::{Deserialize, Serialize};

use crate::ConvertOptions;
use crate::archive::{self, Limits, Unpacked};
use crate::detect::{Detected, Format, detect};
use crate::error::{ExtractError, Result};
use crate::pdfclass::{GpuReason, PdfClassification};
use crate::quality::{QualityContext, QualityReport, assess};
use crate::types::Extracted;
use crate::{docx, epub, html, jats, notebook, odt, tei, text};

/// A converted document with its quality assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Converted {
    pub extracted: Extracted,
    pub quality: QualityReport,
    /// The `extraction.path` of the sidecar: `jats`, `tei`, `html`, `epub`, `docx`, `odt`, `markdown`, `text`,
    /// `notebook` or `pdf-pdfium`.
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    /// Converted to Markdown.
    Converted(Box<Converted>),
    /// An archive: its members are processed one by one (`detect` and `extract` each of them, within the shared
    /// limits).
    Members(Unpacked),
    /// A scanned or maths-heavy PDF. `native` holds the text-layer extraction when there is any worth having
    /// (maths-heavy documents), for use if the GPU tools stay unavailable.
    WaitingGpu { reason: GpuReason, native: Option<Box<Converted>>, classification: Box<PdfClassification> },
    /// The format is known but its converter is not installed (pandoc for LaTeX and RTF).
    WaitingTool { tool: &'static str, format: Format },
}

#[derive(Debug, Clone)]
pub struct Extraction {
    pub detected: Detected,
    pub outcome: Outcome,
}

/// Detect the format of `bytes` and convert it. `name_hint` only breaks ties between text formats.
pub fn extract(bytes: &[u8], name_hint: Option<&str>, opts: &ConvertOptions, limits: &Limits) -> Result<Extraction> {
    let detected = detect(bytes, name_hint);
    tracing::debug!(format = detected.format.name(), bytes = bytes.len(), "detected");
    let outcome = route(bytes, &detected, opts, limits)?;
    Ok(Extraction { detected, outcome })
}

fn route(bytes: &[u8], d: &Detected, opts: &ConvertOptions, limits: &Limits) -> Result<Outcome> {
    let converted = |extracted: Extracted, path: &str, papers: bool| -> Outcome {
        let quality = assess(&extracted, QualityContext { expect_paper_sections: papers, source_pages: None });
        Outcome::Converted(Box::new(Converted { extracted, quality, path: path.to_string() }))
    };
    match d.format {
        Format::Empty => Err(ExtractError::Empty),
        Format::Markdown => Ok(converted(text::convert_markdown(bytes, opts)?, "markdown", false)),
        Format::PlainText | Format::Json => Ok(converted(text::convert_text(bytes, opts)?, "text", false)),
        Format::Jats => Ok(converted(jats::convert(bytes, opts)?, "jats", true)),
        Format::Tei => Ok(converted(tei::convert(bytes, opts)?, "tei", true)),
        Format::Html | Format::Xhtml => Ok(converted(html::convert(bytes, opts)?, "html", false)),
        Format::Notebook => Ok(converted(notebook::convert(bytes, opts)?, "notebook", false)),
        Format::Epub => Ok(converted(epub::convert(bytes, opts)?, "epub", false)),
        Format::Docx => {
            if d.encrypted {
                return Err(ExtractError::Protected("the DOCX has encrypted entries".into()));
            }
            Ok(converted(docx::convert(bytes, opts)?, "docx", false))
        }
        Format::Odt => {
            if d.encrypted {
                return Err(ExtractError::Protected("the ODT has encrypted entries".into()));
            }
            Ok(converted(odt::convert(bytes, opts)?, "odt", false))
        }
        Format::Zip | Format::Tar | Format::TarGz | Format::Gzip => {
            if d.encrypted {
                return Err(ExtractError::Protected("the archive has encrypted entries".into()));
            }
            Ok(Outcome::Members(archive::unpack(bytes, d.format, limits)?))
        }
        Format::ArxivSource | Format::Latex => Ok(Outcome::WaitingTool { tool: "pandoc", format: d.format }),
        Format::Rtf => Ok(Outcome::WaitingTool { tool: "pandoc", format: d.format }),
        Format::Pdf => route_pdf(bytes, opts),
        Format::Xml => Err(ExtractError::Unsupported(format!(
            "XML with root <{}> has no converter",
            d.xml_root.as_deref().unwrap_or("?")
        ))),
        Format::Binary(kind) => {
            if d.encrypted {
                return Err(ExtractError::Protected("an encrypted Office document".into()));
            }
            Err(ExtractError::Unsupported(format!("{kind:?} files are not converted").to_lowercase()))
        }
    }
}

#[cfg(feature = "pdfium")]
fn route_pdf(bytes: &[u8], opts: &ConvertOptions) -> Result<Outcome> {
    use crate::pdfclass::{PdfKind, PdfRoute};
    let c = crate::pdf::convert(bytes, opts)?;
    let papers = c.classification.kind == PdfKind::Paper;
    let quality =
        assess(&c.extracted, QualityContext { expect_paper_sections: papers, source_pages: Some(c.signals.pages) });
    let converted = Converted { extracted: c.extracted, quality, path: "pdf-pdfium".to_string() };
    Ok(match c.classification.route {
        PdfRoute::Native => Outcome::Converted(Box::new(converted)),
        PdfRoute::WaitingGpu(reason) => {
            let native = (reason == GpuReason::MathHeavy).then(|| Box::new(converted));
            Outcome::WaitingGpu { reason, native, classification: Box::new(c.classification) }
        }
    })
}

#[cfg(not(feature = "pdfium"))]
fn route_pdf(_bytes: &[u8], _opts: &ConvertOptions) -> Result<Outcome> {
    Err(ExtractError::ToolUnavailable("vault-extract was built without the pdfium feature".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn run(bytes: &[u8], name: Option<&str>) -> Result<Extraction> {
        extract(bytes, name, &ConvertOptions::default(), &Limits::default())
    }

    fn converted(e: &Extraction) -> &Converted {
        match &e.outcome {
            Outcome::Converted(c) => c,
            other => panic!("not converted: {other:?}"),
        }
    }

    #[test]
    fn text_formats_route_to_their_converters() {
        let md =
            run(b"# Title\n\nSome text with a [link](http://a.b) and enough words to count here.\n", Some("note.txt"))
                .unwrap();
        assert_eq!(md.detected.format, Format::Markdown);
        assert_eq!(converted(&md).path, "markdown");
        let txt = run(b"Just plain words in a plain text file for the test.\n", None).unwrap();
        assert_eq!(converted(&txt).path, "text");
        let html = run(
            b"<!doctype html><html><body><h1>Hi</h1><p>Body words here for the test.</p></body></html>",
            Some("page.md"),
        )
        .unwrap();
        assert_eq!(html.detected.format, Format::Html);
        assert_eq!(converted(&html).extracted.headings.len(), 1);
        let json = run(br#"{"a": 1}"#, None).unwrap();
        assert_eq!(converted(&json).path, "text");
    }

    #[test]
    fn xml_formats_and_notebooks() {
        let jats = run(b"<article><front><article-meta><title-group><article-title>T</article-title></title-group></article-meta></front><body><sec><title>Intro</title><p>Text.</p></sec></body></article>", None).unwrap();
        assert_eq!(jats.detected.format, Format::Jats);
        assert_eq!(converted(&jats).path, "jats");
        let tei = run(br#"<TEI xmlns="http://www.tei-c.org/ns/1.0"><text><body><div><head>H</head><p>Words.</p></div></body></text></TEI>"#, None).unwrap();
        assert_eq!(converted(&tei).path, "tei");
        let nb = run(
            br##"{"cells":[{"cell_type":"markdown","source":["# Nb"]}],"metadata":{},"nbformat":4}"##,
            Some("x.json"),
        )
        .unwrap();
        assert_eq!(nb.detected.format, Format::Notebook);
        assert_eq!(converted(&nb).extracted.metadata.title.as_deref(), Some("Nb"));
        let other = run(b"<?xml version=\"1.0\"?><feed><entry/></feed>", None).unwrap_err();
        assert_eq!(other.reason_code(), "unsupported_format");
    }

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (n, c) in entries {
            w.start_file(*n, o).unwrap();
            w.write_all(c).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn archives_return_members_and_office_zips_convert() {
        let z =
            zip(&[("a.md", b"# A\n\nSome words [here](http://x.y) again.\n"), ("b.txt", b"hello there my friend\n")]);
        let e = run(&z, Some("batch.zip")).unwrap();
        match &e.outcome {
            Outcome::Members(u) => assert_eq!(u.members.len(), 2),
            other => panic!("{other:?}"),
        }
        let docx = zip(&[("[Content_Types].xml", b"<Types/>"), ("word/document.xml", br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello docx</w:t></w:r></w:p></w:body></w:document>"#)]);
        let e = run(&docx, Some("x.zip")).unwrap();
        assert_eq!(e.detected.format, Format::Docx);
        assert_eq!(converted(&e).extracted.markdown, "Hello docx\n");
    }

    #[test]
    fn latex_and_rtf_wait_for_pandoc() {
        let tex = run(b"\\documentclass{article}\n\\begin{document}Hi\\end{document}\n", Some("paper.txt")).unwrap();
        assert!(matches!(tex.outcome, Outcome::WaitingTool { tool: "pandoc", format: Format::Latex }));
        let mut g = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        g.write_all(b"\\documentclass{article}\n\\begin{document}Hi\\end{document}\n").unwrap();
        let e = run(&g.finish().unwrap(), Some("1234.5678")).unwrap();
        assert!(matches!(e.outcome, Outcome::WaitingTool { format: Format::ArxivSource, .. }));
        assert!(matches!(run(b"{\\rtf1\\ansi Hello}", None).unwrap().outcome, Outcome::WaitingTool { .. }));
    }

    #[test]
    fn binaries_and_empty_files_are_refused_with_reason_codes() {
        assert_eq!(run(b"", None).unwrap_err().reason_code(), "empty_file");
        assert_eq!(
            run(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", Some("a.pdf")).unwrap_err().reason_code(),
            "unsupported_format"
        );
        assert_eq!(run(b"MZ\x90\0\x03\0\0\0", None).unwrap_err().reason_code(), "unsupported_format");
        let mut ole = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        ole.extend(std::iter::repeat_n(0u8, 300));
        ole.extend("EncryptedPackage".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(run(&ole, Some("secret.docx")).unwrap_err().reason_code(), "protected");
        assert_eq!(run(&[0, 159, 1, 2, 3, 0, 254], None).unwrap_err().reason_code(), "unsupported_format");
    }

    #[test]
    fn corrupt_files_report_corruption() {
        assert_eq!(
            run(b"<article article-type=\"research-article\"><front></article>", None).unwrap_err().reason_code(),
            "corrupt_file"
        );
        assert_eq!(run(b"PK\x03\x04 not really a zip", None).unwrap_err().reason_code(), "corrupt_file");
    }

    #[cfg(feature = "pdfium")]
    mod pdf_routes {
        use super::*;
        use crate::testpdf::*;

        #[test]
        fn born_digital_pdf_converts_and_scans_wait_for_the_gpu() {
            need_pdfium!();
            let pdf = build_pdf(
                &[page(body(&["Plenty of ordinary words on this page to count as a text layer for sure."], 700.0))],
                None,
                None,
            );
            let e = run(&pdf, Some("paper.pdf")).unwrap();
            assert_eq!(e.detected.format, Format::Pdf);
            let c = converted(&e);
            assert_eq!(c.path, "pdf-pdfium");
            assert!(c.extracted.markdown.contains("Plenty of ordinary words"));
            let scan = build_pdf(&[PageSpec { ops: vec![], rotate: 0, full_page_image: true }], None, None);
            match run(&scan, None).unwrap().outcome {
                Outcome::WaitingGpu { reason, native, .. } => {
                    assert_eq!(reason, GpuReason::Scanned);
                    assert!(native.is_none());
                }
                other => panic!("{other:?}"),
            }
            assert_eq!(run(b"%PDF-1.4\nbroken", None).unwrap_err().reason_code(), "corrupt_file");
        }
    }
}

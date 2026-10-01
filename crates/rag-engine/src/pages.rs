//! `GET /documents/{doc_id}/pages/{n}.png` (contract C11): one PDF page rendered by PDFium as
//! a PNG, with a quote marked.
//!
//! - The width is clamped to [`MIN_WIDTH`]..=[`MAX_WIDTH`] pixels (default [`DEFAULT_WIDTH`]);
//!   the height follows the page's aspect ratio, up to [`MAX_ASPECT`] times the width.
//! - The quote is looked up in the page's text layer after the same normalisation as citation
//!   checking (C11 `/api/library/verify`): whitespace runs are one space, curly quotes are
//!   straight, dashes are `-`, soft hyphens are dropped. Every occurrence is marked: PDFium's
//!   text rectangles for the matched characters are blended with a translucent yellow.
//! - Rendering holds the process-wide PDFium lock (`extract::pdfium_lock`), like extraction.

use crate::extract;
use crate::survey;
use pdfium_render::prelude::*;
use std::path::Path;

pub const DEFAULT_WIDTH: u32 = 1200;
pub const MIN_WIDTH: u32 = 400;
pub const MAX_WIDTH: u32 = 2000;
/// Tallest image, as a multiple of its width (A4 is 1.41, US legal 1.65).
const MAX_ASPECT: u32 = 3;
/// The mark: RGB and opacity.
const MARK_RGB: [u8; 3] = [255, 200, 0];
const MARK_ALPHA: f32 = 0.4;
/// Pixels added around each text rectangle.
const MARK_PAD: i32 = 2;

pub fn clamp_width(w: Option<u32>) -> u32 {
    w.unwrap_or(DEFAULT_WIDTH).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// Page `number` (1-based) of the PDF at `file` as PNG, `width` pixels wide (already clamped),
/// with `quote` marked where the page's text contains it, and how many occurrences were marked.
/// `Ok(None)` when the page does not exist.
pub fn render(
    pdfium_lib: &Path,
    file: &Path,
    number: u32,
    width: u32,
    quote: Option<&str>,
) -> Result<Option<(Vec<u8>, usize)>, String> {
    // Declared first, so PDFium objects are released while the lock is still held.
    let _guard = extract::pdfium_lock();
    let pdfium = survey::bind_pdfium(pdfium_lib)?;
    let document = extract::open_pdf(&pdfium, file)?;
    let index = match i32::try_from(number) {
        Ok(n) if n >= 1 && n <= document.pages().len() => n - 1,
        _ => return Ok(None),
    };
    let page = document
        .pages()
        .get(index)
        .map_err(|e| format!("cannot load page {number}: {e}"))?;
    let width = width as i32;
    let config = PdfRenderConfig::new()
        .set_target_width(width)
        .set_maximum_width(width)
        .set_maximum_height(width * MAX_ASPECT as i32)
        .set_format(PdfBitmapFormat::BGRA)
        .set_clear_color(PdfColor::WHITE);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("cannot render page {number}: {e}"))?;
    let (w, h) = (bitmap.width(), bitmap.height());
    let mut rgba = bitmap.as_rgba_bytes();
    drop(bitmap);

    let mut marked = 0;
    if let Some(quote) = quote
        .map(|q| normalise(q.chars().map(|c| (c, 0))).0)
        .filter(|q| !q.is_empty())
    {
        let text = page
            .text()
            .map_err(|e| format!("page {number}: cannot read text layer: {e}"))?;
        let (chars, origin) = normalise(page_chars(&text).into_iter());
        for start in find_all(&chars, &quote) {
            let span = &origin[start..start + quote.len()];
            let (first, last) = (
                *span.iter().min().expect("quotes are not empty"),
                *span.iter().max().expect("quotes are not empty"),
            );
            for segment in text.segments_subset(first, last - first + 1).iter() {
                let r = segment.bounds();
                let a = page
                    .points_to_pixels(r.left(), r.top(), &config)
                    .map_err(|e| e.to_string())?;
                let b = page
                    .points_to_pixels(r.right(), r.bottom(), &config)
                    .map_err(|e| e.to_string())?;
                blend_rect(
                    &mut rgba,
                    w,
                    h,
                    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)),
                );
            }
            marked += 1;
        }
    }
    Ok(Some((encode_png(&rgba, w as u32, h as u32)?, marked)))
}

/// The page's characters with their PDFium indices. Characters outside the Basic Multilingual
/// Plane may come as two UTF-16 surrogates; they are joined (at the first one's index).
fn page_chars(text: &PdfPageText) -> Vec<(char, usize)> {
    let mut out = Vec::new();
    let mut high: Option<(u32, usize)> = None;
    for c in text.chars().iter() {
        let (value, index) = (c.unicode_value(), c.index());
        if let Some((h, at)) = high.take()
            && (0xDC00..0xE000).contains(&value)
        {
            if let Some(joined) = char::from_u32(0x10000 + ((h - 0xD800) << 10) + (value - 0xDC00))
            {
                out.push((joined, at));
            }
            continue;
        }
        if (0xD800..0xDC00).contains(&value) {
            high = Some((value, index));
        } else if let Some(ch) = char::from_u32(value) {
            out.push((ch, index));
        }
    }
    out
}

/// Normalise for matching, keeping each output character's origin: whitespace runs become one
/// space (none at the start or end), curly quotes become straight, dashes become `-`, soft
/// hyphens are dropped.
fn normalise(chars: impl Iterator<Item = (char, usize)>) -> (Vec<char>, Vec<usize>) {
    let (mut out, mut origin) = (Vec::new(), Vec::new());
    let mut space: Option<usize> = None;
    for (c, at) in chars {
        let c = match c {
            '\u{AD}' => continue,
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' => '"',
            '\u{2010}'..='\u{2015}' | '\u{2212}' => '-',
            c if c.is_whitespace() => {
                space.get_or_insert(at);
                continue;
            }
            c => c,
        };
        if let Some(at) = space.take()
            && !out.is_empty()
        {
            out.push(' ');
            origin.push(at);
        }
        out.push(c);
        origin.push(at);
    }
    (out, origin)
}

/// Start positions of the non-overlapping occurrences of `needle` in `haystack`.
fn find_all(haystack: &[char], needle: &[char]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if haystack[i..i + needle.len()] == *needle {
            found.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    found
}

/// Blend the mark colour over a rectangle `(left, top, right, bottom)` of an RGBA image,
/// padded by [`MARK_PAD`] and clipped to the image.
fn blend_rect(
    rgba: &mut [u8],
    width: i32,
    height: i32,
    (left, top, right, bottom): (i32, i32, i32, i32),
) {
    let (x0, x1) = ((left - MARK_PAD).max(0), (right + MARK_PAD).min(width));
    let (y0, y1) = ((top - MARK_PAD).max(0), (bottom + MARK_PAD).min(height));
    for y in y0..y1 {
        for x in x0..x1 {
            let p = ((y * width + x) * 4) as usize;
            for (channel, mark) in rgba[p..p + 3].iter_mut().zip(MARK_RGB) {
                *channel =
                    (*channel as f32 * (1.0 - MARK_ALPHA) + mark as f32 * MARK_ALPHA).round() as u8;
            }
        }
    }
}

/// An 8-bit RGB PNG of an RGBA image (the page is opaque, so alpha is dropped).
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("cannot encode PNG: {e}"))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| format!("cannot encode PNG: {e}"))?;
    writer
        .finish()
        .map_err(|e| format!("cannot encode PNG: {e}"))?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn norm(s: &str) -> (String, Vec<usize>) {
        let (c, o) = normalise(s.chars().enumerate().map(|(i, c)| (c, i)));
        (c.into_iter().collect(), o)
    }

    #[test]
    fn normalisation_matches_citation_checking() {
        assert_eq!(norm("  a \r\n\t b  ").0, "a b");
        assert_eq!(
            norm("\u{201C}it\u{2019}s\u{201D} \u{2013} x\u{2014}y \u{2212}1").0,
            "\"it's\" - x-y -1"
        );
        assert_eq!(norm("con\u{AD}tinuous").0, "continuous");
        // Each character keeps where it came from; a collapsed run points at its first space.
        assert_eq!(norm("a \n b"), ("a b".to_string(), vec![0, 1, 4]));
    }

    #[test]
    fn find_all_finds_non_overlapping_occurrences() {
        assert_eq!(find_all(&chars("abcabcab"), &chars("abc")), vec![0, 3]);
        assert_eq!(find_all(&chars("aaaa"), &chars("aa")), vec![0, 2]);
        assert!(find_all(&chars("ab"), &chars("abc")).is_empty());
    }

    #[test]
    fn blending_tints_the_padded_rectangle_only() {
        let (w, h) = (20, 10);
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        blend_rect(&mut rgba, w, h, (5, 4, 8, 5));
        let px = |x: i32, y: i32| &rgba[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize];
        assert_eq!(
            px(5, 4),
            [255, 233, 153, 255],
            "white under the mark turns yellow"
        );
        assert_eq!(px(3, 2), [255, 233, 153, 255], "the padding is marked");
        assert_eq!(px(2, 4), [255, 255, 255, 255]);
        assert_eq!(px(12, 4), [255, 255, 255, 255]);
        // Clipped at the image edges without panicking.
        blend_rect(&mut rgba, w, h, (-10, -10, 100, 100));
    }

    #[test]
    fn widths_are_clamped() {
        assert_eq!(
            (
                clamp_width(None),
                clamp_width(Some(10)),
                clamp_width(Some(5000)),
                clamp_width(Some(800))
            ),
            (1200, 400, 2000, 800)
        );
    }

    fn pdfium_lib() -> PathBuf {
        std::env::var_os("PDFIUM_DYNAMIC_LIB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/pdfium/bin"))
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Decoded RGB pixels.
    fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(
            (info.color_type, info.bit_depth),
            (png::ColorType::Rgb, png::BitDepth::Eight)
        );
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }

    /// Pixels tinted by the mark: clearly more red and green than blue.
    fn marked_pixels(rgb: &[u8]) -> usize {
        rgb.as_chunks::<3>()
            .0
            .iter()
            .filter(|&&[r, g, b]| r > 150 && g > 120 && (b as i32) < g as i32 - 60)
            .count()
    }

    #[test]
    fn renders_a_pdf_page_and_marks_the_quote() {
        let lib = pdfium_lib();
        let text = fixture("text.pdf");
        let (png, marked) = render(&lib, &text, 1, 800, None).unwrap().unwrap();
        let (w, h, rgb) = decode(&png);
        // A5 is 419.5 x 595.3 points.
        assert!(
            w == 800 && (h as f32 / 800.0 - 595.28 / 419.53).abs() < 0.01,
            "{w} x {h}"
        );
        assert_eq!((marked, marked_pixels(&rgb)), (0, 0));
        assert!(rgb.iter().any(|&v| v < 100), "the text is drawn");

        // A quote across a line break, with extra whitespace.
        let quote = "Glucose is first broken down in the cytoplasm,   and the products then enter the mitochondrion";
        let (png, marked) = render(&lib, &text, 1, 800, Some(quote)).unwrap().unwrap();
        let (_, _, rgb) = decode(&png);
        // The prose appears twice on page 1.
        assert_eq!(marked, 2, "both occurrences");
        let tinted = marked_pixels(&rgb);
        assert!(tinted > 5_000, "{tinted} marked pixels");

        let (png, marked) = render(&lib, &text, 1, 800, Some("not on this page at all"))
            .unwrap()
            .unwrap();
        assert_eq!((marked, marked_pixels(&decode(&png).2)), (0, 0));
        // Page 2 has its own "Page 2." prefix; the page count bounds the page number.
        assert_eq!(
            render(&lib, &text, 2, 800, Some("Page 2."))
                .unwrap()
                .unwrap()
                .1,
            1
        );
        assert!(render(&lib, &text, 4, 800, None).unwrap().is_none());
        assert!(render(&lib, &text, 0, 800, None).unwrap().is_none());
    }

    #[test]
    fn marks_maths_glyphs_and_refuses_encrypted_files() {
        let lib = pdfium_lib();
        let (png, marked) = render(&lib, &fixture("math.pdf"), 1, 1200, Some("π r^2 = A"))
            .unwrap()
            .unwrap();
        assert_eq!(marked, 1);
        assert!(marked_pixels(&decode(&png).2) > 500);
        let err = render(&lib, &fixture("encrypted.pdf"), 1, 800, None)
            .err()
            .unwrap();
        assert!(err.contains("encrypted"), "{err}");
    }
}

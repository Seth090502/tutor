//! Layout analysis for PDF text: from positioned glyphs to lines in reading order, then to Markdown with
//! headings, paragraphs, lists, references and page marks. Pure geometry, no PDF library, so every rule is
//! testable on synthetic glyphs.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::builder::{MdBuilder, collapse_ws};
use crate::ids;
use crate::jats::script;
use crate::types::{Extracted, Metadata, Reference};

/// One positioned character. Coordinates are PDF points with y growing upward.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Glyph {
    pub ch: char,
    pub l: f32,
    pub r: f32,
    pub b: f32,
    pub t: f32,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PageInput {
    pub width: f32,
    pub height: f32,
    pub glyphs: Vec<Glyph>,
    /// The page's printed label from `/PageLabels`.
    pub label: Option<String>,
    /// Text set at an angle (arXiv's margin stamp), kept for metadata only.
    pub rotated_text: String,
}

/// A line of text in reading order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub text: String,
    pub l: f32,
    pub r: f32,
    pub top: f32,
    pub bottom: f32,
    /// The dominant font size.
    pub size: f32,
    pub bold: bool,
    pub column: usize,
    /// In the top or bottom strip of the page (running header, footer, page number).
    pub furniture: bool,
    /// The line is a bold run-in heading; the body text that followed it on the page line is the next line.
    pub run_in: bool,
}

// ---------------------------------------------------------------------------------------------- page layout

/// A run of glyphs on one baseline with no large gap.
#[derive(Debug, Clone)]
struct Piece {
    text: String,
    l: f32,
    r: f32,
    top: f32,
    bottom: f32,
    size: f32,
    bold: bool,
    /// Vertical centre of the dominant (largest, most numerous) glyphs.
    y: f32,
    /// A bold run-in heading ("2.1. Methods.") at the start of a line of body text.
    run_in: bool,
}

/// Word gaps are about a quarter em; a gap this many ems wide separates pieces (columns, table cells).
const PIECE_GAP_EM: f32 = 1.4;
/// A gap wider than this many ems between glyphs of a piece is a word space.
const SPACE_EM: f32 = 0.18;

pub(crate) fn layout_page(p: &PageInput) -> Vec<Line> {
    let glyphs: Vec<&Glyph> = p.glyphs.iter().filter(|g| g.size > 0.5 && (g.r - g.l).is_finite() && (g.t - g.b).is_finite()).collect();
    if glyphs.is_empty() {
        return Vec::new();
    }
    let pieces = pieces_of(&glyphs);
    let ordered = reading_order(&pieces, p.width);
    let mut lines = merge_into_lines(&ordered);
    mark_furniture(&mut lines, p.height);
    lines
}

/// Group glyphs into baseline clusters, then split each cluster into pieces at wide gaps.
fn pieces_of(glyphs: &[&Glyph]) -> Vec<Piece> {
    let mut order: Vec<usize> = (0..glyphs.len()).collect();
    let ymid = |g: &Glyph| (g.t + g.b) / 2.0;
    order.sort_by(|&a, &b| ymid(glyphs[b]).partial_cmp(&ymid(glyphs[a])).unwrap_or(std::cmp::Ordering::Equal));

    // Sweep top to bottom; a glyph joins the current line when it sits within half the larger size of the
    // line's reference height (the centre of its biggest glyph, so scripts attach to their base line).
    struct Cluster {
        members: Vec<usize>,
        ref_y: f32,
        ref_size: f32,
    }
    let mut clusters: Vec<Cluster> = Vec::new();
    for &i in &order {
        let g = glyphs[i];
        let y = ymid(g);
        match clusters.last_mut() {
            Some(c) if (y - c.ref_y).abs() <= 0.55 * c.ref_size.max(g.size) => {
                c.members.push(i);
                if g.size > c.ref_size {
                    c.ref_size = g.size;
                    c.ref_y = y;
                }
            }
            _ => clusters.push(Cluster { members: vec![i], ref_y: y, ref_size: g.size }),
        }
    }

    let mut pieces = Vec::new();
    for c in clusters {
        let mut m: Vec<&Glyph> = c.members.iter().map(|&i| glyphs[i]).collect();
        m.sort_by(|a, b| a.l.partial_cmp(&b.l).unwrap_or(std::cmp::Ordering::Equal));
        let line_size = dominant_size(&m);
        let split = run_in_boundary(&m);
        let mut cur: Vec<&Glyph> = Vec::new();
        let mut last_r = f32::MIN;
        for (i, g) in m.iter().enumerate() {
            if !cur.is_empty() && (g.l - last_r > PIECE_GAP_EM * line_size || split == Some(i)) {
                let mut piece = build_piece(&cur, line_size);
                // The piece just closed is the run-in head when the split falls here.
                piece.run_in = split == Some(i);
                pieces.push(piece);
                cur.clear();
            }
            cur.push(g);
            last_r = last_r.max(g.r);
        }
        if !cur.is_empty() {
            pieces.push(build_piece(&cur, line_size));
        }
    }
    pieces
}

/// Index of the first glyph after a bold run-in heading ("2.1. Rigged representations of algebras.") that starts
/// the line, when the rest of the line is not bold.
fn run_in_boundary(m: &[&Glyph]) -> Option<usize> {
    let mut seen = 0usize;
    let mut last_visible: Option<char> = None;
    for (i, g) in m.iter().enumerate() {
        if g.ch.is_whitespace() {
            continue;
        }
        if g.bold {
            seen += 1;
            last_visible = Some(g.ch);
            continue;
        }
        // First non-bold glyph: a run-in head needs a sentence-ending bold run of some length before it.
        let ends = matches!(last_visible, Some('.') | Some(':'));
        let has_more_text = m[i..].iter().filter(|x| !x.ch.is_whitespace()).count() >= 8;
        return (seen >= 8 && ends && has_more_text && i > 0).then_some(i);
    }
    None
}

/// The size most characters in the group have.
fn dominant_size(g: &[&Glyph]) -> f32 {
    let mut counts: HashMap<i32, (usize, f32)> = HashMap::new();
    for x in g.iter().filter(|x| !x.ch.is_whitespace()) {
        let e = counts.entry((x.size * 4.0).round() as i32).or_insert((0, x.size));
        e.0 += 1;
    }
    counts.into_values().max_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))).map(|(_, s)| s).unwrap_or_else(|| g.first().map(|x| x.size).unwrap_or(10.0))
}

fn build_piece(g: &[&Glyph], line_size: f32) -> Piece {
    let mut text = String::new();
    let mut last_r = f32::MIN;
    let base_y: f32 = {
        let dom: Vec<f32> = g.iter().filter(|x| (x.size - line_size).abs() < 0.3 && !x.ch.is_whitespace()).map(|x| (x.t + x.b) / 2.0).collect();
        if dom.is_empty() { (g[0].t + g[0].b) / 2.0 } else { dom.iter().sum::<f32>() / dom.len() as f32 }
    };
    let mut pending_script: Option<(char, String)> = None;
    let flush_script = |text: &mut String, p: &mut Option<(char, String)>| {
        if let Some((mark, s)) = p.take() {
            script(mark, &s, text);
        }
    };
    for x in g {
        if x.ch.is_whitespace() {
            flush_script(&mut text, &mut pending_script);
            if !text.ends_with(' ') && !text.is_empty() {
                text.push(' ');
            }
            last_r = last_r.max(x.r);
            continue;
        }
        // Superscripts and subscripts: clearly smaller and shifted off the baseline.
        let shift = ((x.t + x.b) / 2.0 - base_y) / line_size.max(1.0);
        let small = x.size < 0.85 * line_size;
        let kind = if small && shift > 0.18 {
            Some('^')
        } else if small && shift < -0.18 {
            Some('_')
        } else {
            None
        };
        match (kind, &mut pending_script) {
            (Some(k), Some((pk, s))) if *pk == k => s.push(x.ch),
            (Some(k), _) => {
                flush_script(&mut text, &mut pending_script);
                // A script touching its base stays attached to it, even where a space was synthesised.
                if text.ends_with(' ') && last_r > f32::MIN && x.l - last_r <= 0.25 * line_size {
                    text.pop();
                }
                pending_script = Some((k, x.ch.to_string()));
            }
            (None, _) => {
                flush_script(&mut text, &mut pending_script);
                if last_r > f32::MIN && x.l - last_r > SPACE_EM * x.size && !text.ends_with(' ') && !text.is_empty() {
                    text.push(' ');
                }
                text.push(x.ch);
            }
        }
        last_r = last_r.max(x.r);
    }
    flush_script(&mut text, &mut pending_script);
    let top = g.iter().map(|x| x.t).fold(f32::MIN, f32::max);
    let bottom = g.iter().map(|x| x.b).fold(f32::MAX, f32::min);
    let l = g.iter().map(|x| x.l).fold(f32::MAX, f32::min);
    let r = g.iter().map(|x| x.r).fold(f32::MIN, f32::max);
    let letters: Vec<&&Glyph> = g.iter().filter(|x| x.ch.is_alphabetic()).collect();
    let bold = !letters.is_empty() && letters.iter().filter(|x| x.bold).count() * 2 > letters.len();
    Piece { text: text.trim().to_string(), l, r, top, bottom, size: line_size, bold, y: base_y, run_in: false }
}

/// Order pieces: top to bottom, columns left to right between full-width pieces.
fn reading_order(pieces: &[Piece], page_width: f32) -> Vec<(Piece, usize)> {
    let pieces: Vec<&Piece> = pieces.iter().filter(|p| !p.text.is_empty()).collect();
    if pieces.is_empty() {
        return Vec::new();
    }
    let xmin = pieces.iter().map(|p| p.l).fold(f32::MAX, f32::min);
    let xmax = pieces.iter().map(|p| p.r).fold(f32::MIN, f32::max);
    let span = (xmax - xmin).max(1.0);
    let gutters = find_gutters(&pieces, xmin, xmax, page_width);

    let column_of = |p: &Piece| -> Option<usize> {
        if gutters.is_empty() {
            return None;
        }
        let mut col = 0;
        for (i, (gl, gr)) in gutters.iter().enumerate() {
            if p.r <= *gl + 0.5 {
                return Some(col);
            }
            if p.l >= *gr - 0.5 {
                col = i + 1;
                continue;
            }
            return None; // crosses a gutter
        }
        Some(col)
    };
    let _ = span;

    let mut sorted: Vec<&Piece> = pieces.clone();
    sorted.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal).then(a.l.partial_cmp(&b.l).unwrap_or(std::cmp::Ordering::Equal)));

    let ncols = gutters.len() + 1;
    let mut out: Vec<(Piece, usize)> = Vec::new();
    let mut buffers: Vec<Vec<&Piece>> = vec![Vec::new(); ncols];
    let flush = |buffers: &mut Vec<Vec<&Piece>>, out: &mut Vec<(Piece, usize)>| {
        for (c, b) in buffers.iter_mut().enumerate() {
            sort_into_lines(b);
            for p in b.drain(..) {
                out.push((p.clone(), c));
            }
        }
    };
    for p in sorted {
        match column_of(p) {
            Some(c) => buffers[c].push(p),
            None => {
                flush(&mut buffers, &mut out);
                out.push((p.clone(), 0));
            }
        }
    }
    flush(&mut buffers, &mut out);
    out
}

/// `pieces` are sorted from the top down; pieces whose baselines agree within half a size belong to one line and
/// are put left to right, so a label that sits a hair lower than its text still comes first.
fn sort_into_lines(pieces: &mut [&Piece]) {
    let mut start = 0;
    while start < pieces.len() {
        let y0 = pieces[start].y;
        let mut end = start + 1;
        while end < pieces.len() && (pieces[end].y - y0).abs() <= 0.5 * pieces[end].size.max(pieces[start].size) {
            end += 1;
        }
        pieces[start..end].sort_by(|a, b| a.l.partial_cmp(&b.l).unwrap_or(std::cmp::Ordering::Equal));
        start = end;
    }
}

/// Vertical gutters: gaps in horizontal coverage, inside the text span, that no short piece crosses and that
/// have text on both sides.
fn find_gutters(pieces: &[&Piece], xmin: f32, xmax: f32, _page_width: f32) -> Vec<(f32, f32)> {
    let span = xmax - xmin;
    if span < 200.0 {
        return Vec::new();
    }
    // Only pieces narrower than 60% of the span take part: wide ones are titles, headings and full-width text.
    let narrow: Vec<&&Piece> = pieces.iter().filter(|p| p.r - p.l < 0.6 * span).collect();
    if narrow.len() < 8 {
        return Vec::new();
    }
    let mut ivs: Vec<(f32, f32)> = narrow.iter().map(|p| (p.l, p.r)).collect();
    ivs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut gaps: Vec<(f32, f32)> = Vec::new();
    let mut end = ivs[0].1;
    for &(l, r) in &ivs[1..] {
        if l > end {
            gaps.push((end, l));
        }
        end = end.max(r);
    }
    let min_gap = (0.018 * span).max(8.0);
    let total_chars: usize = narrow.iter().map(|p| p.text.chars().count()).sum();
    let mut out = Vec::new();
    for (gl, gr) in gaps {
        if gr - gl < min_gap || gl < xmin + 0.15 * span || gr > xmax - 0.15 * span {
            continue;
        }
        let left: usize = narrow.iter().filter(|p| p.r <= gl + 0.5).map(|p| p.text.chars().count()).sum();
        let right: usize = narrow.iter().filter(|p| p.l >= gr - 0.5).map(|p| p.text.chars().count()).sum();
        if left * 5 >= total_chars && right * 5 >= total_chars {
            out.push((gl, gr));
        }
    }
    out
}

/// Merge pieces on the same baseline in the same column into one line.
fn merge_into_lines(ordered: &[(Piece, usize)]) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut last_y = f32::NAN;
    let mut last_col = usize::MAX;
    for (p, col) in ordered {
        let same_line = lines.last().is_some_and(|l| !l.run_in) && *col == last_col && (p.y - last_y).abs() <= 0.5 * p.size.max(lines.last().map(|l| l.size).unwrap_or(0.0)) && p.l >= lines.last().map(|l| l.l).unwrap_or(0.0);
        if same_line {
            let l = lines.last_mut().expect("checked above");
            l.text.push(' ');
            l.text.push_str(&p.text);
            l.r = l.r.max(p.r);
            l.top = l.top.max(p.top);
            l.bottom = l.bottom.min(p.bottom);
            l.bold = l.bold && p.bold;
            if p.size > l.size {
                l.size = p.size;
            }
        } else {
            lines.push(Line { text: p.text.clone(), l: p.l, r: p.r, top: p.top, bottom: p.bottom, size: p.size, bold: p.bold, column: *col, furniture: false, run_in: p.run_in });
        }
        last_y = p.y;
        last_col = *col;
    }
    lines
}

/// Lines in the top and bottom strips of the page are candidates for running headers, footers and page numbers.
fn mark_furniture(lines: &mut [Line], page_height: f32) {
    if page_height <= 0.0 {
        return;
    }
    let ymax = lines.iter().map(|l| l.top).fold(f32::MIN, f32::max);
    let ymin = lines.iter().map(|l| l.bottom).fold(f32::MAX, f32::min);
    let _ = (ymax, ymin);
    for l in lines.iter_mut() {
        let frac_from_top = 1.0 - l.top / page_height;
        let frac_from_bottom = l.bottom / page_height;
        l.furniture = frac_from_top < 0.12 || frac_from_bottom < 0.12;
    }
}

// ------------------------------------------------------------------------------------------ document assembly

#[derive(Debug, Clone, Default)]
pub(crate) struct PageLines {
    pub label: Option<String>,
    pub lines: Vec<Line>,
}

static NUMBERED_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,2}(?:\.\d{1,2}){0,3})\.?\s+\p{Lu}").expect("static regex"));
static SECTION_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?i)(abstract|introduction|references|bibliography|acknowledg\w*|conclusions?|appendix\b.*|contents|preface|summary|index|glossary)$").expect("static regex"));
static BRACKET_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[(\d{1,4}|[A-Za-z][A-Za-z0-9+\-]{0,15}\d{2,4}[a-z]?)\]\s*").expect("static regex"));
static DOT_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,4})\.\s+\p{Lu}").expect("static regex"));
static PAGE_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?i)(?:page\s+)?(\d{1,4}|[ivxlcdm]{1,7})$").expect("static regex"));
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[•▪◦‣●■□–—]\s+").expect("static regex"));

/// What the assembly step found out besides the text.
#[derive(Debug, Default)]
pub(crate) struct Assembled {
    pub extracted: Extracted,
    /// Per page: the printed page number found in its header or footer, if any.
    pub printed_numbers: Vec<Option<String>>,
}

pub(crate) fn assemble(mut pages: Vec<PageLines>, mut metadata: Metadata) -> Assembled {
    let page_count = pages.len();
    let printed_numbers = strip_furniture(&mut pages);
    let body = body_size(&pages);
    let heading_sizes = heading_size_classes(&pages, body);
    let typical_pitch = median_pitch(&pages);

    let mut b = MdBuilder::new();
    let mut refs: Vec<Reference> = Vec::new();
    let mut in_refs = false;
    let mut abstract_candidate: Option<String> = None;
    let want_abstract = std::cell::Cell::new(false);

    struct Para {
        text: String,
        last_bottom: f32,
        last_l: f32,
        last_r: f32,
        col_l: f32,
        col_r: f32,
        pitch: f32,
        column: usize,
        /// Continues an unfinished paragraph from the previous page.
        cont: bool,
    }
    let mut para: Option<Para> = None;
    let mut carry = false;
    // On the first page, everything above the first section heading is the title block: only its largest line
    // is a heading, the authors, date and affiliations under it are text.
    let title_zone_end = pages
        .first()
        .and_then(|p| p.lines.iter().position(|l| NUMBERED_HEADING.is_match(l.text.trim()) || SECTION_WORD.is_match(l.text.trim())))
        .unwrap_or(0);
    let title_zone_max = pages
        .first()
        .map(|p| p.lines.iter().take(title_zone_end).map(|l| l.size).fold(0.0f32, f32::max))
        .unwrap_or(0.0);
    let mut ref_entry: Option<String> = None;

    macro_rules! flush_para {
        () => {
            if let Some(p) = para.take() {
                let t = collapse_ws(&p.text);
                if want_abstract.get() && abstract_candidate.is_none() && !t.is_empty() {
                    abstract_candidate = Some(t.clone());
                    want_abstract.set(false);
                }
                if p.cont {
                    b.paragraph_cont(&t);
                } else {
                    b.paragraph(&t);
                }
            }
        };
    }
    macro_rules! flush_ref {
        () => {
            if let Some(e) = ref_entry.take() {
                let r = ids::reference_from_raw(&e, None);
                if !r.raw.is_empty() {
                    b.paragraph(&r.raw);
                    refs.push(r);
                }
            }
        };
    }

    for (pi, page) in pages.iter().enumerate() {
        b.page_break_at(page.label.clone(), pi as u32 + 1);
        // Column extents on this page for hanging-indent and paragraph decisions.
        let mut col_l: HashMap<usize, f32> = HashMap::new();
        let mut col_r: HashMap<usize, f32> = HashMap::new();
        // Extents come from body-sized lines only: a title or heading must not widen the column.
        for l in page.lines.iter().filter(|l| (l.size - body).abs() <= 0.1 * body && l.text.chars().count() >= 20) {
            let e = col_l.entry(l.column).or_insert(l.l);
            *e = e.min(l.l);
            let e = col_r.entry(l.column).or_insert(l.r);
            *e = e.max(l.r);
        }
        let mut prev: Option<&Line> = None;
        for (line_index, line) in page.lines.iter().enumerate() {
            let text = line.text.trim();
            if text.is_empty() {
                continue;
            }
            let cl = col_l.get(&line.column).copied().unwrap_or(line.l);
            let cr = col_r.get(&line.column).copied().unwrap_or(line.r);
            let in_title_zone = pi == 0 && line_index < title_zone_end;
            let is_heading = heading_level(line, body, &heading_sizes, prev, cl).is_some() && (!in_title_zone || (line.size - title_zone_max).abs() < 0.6);

            if is_heading {
                carry = false;
                flush_ref!();
                flush_para!();
                let level = heading_level(line, body, &heading_sizes, prev, cl).unwrap_or(2);
                let t = collapse_ws(text);
                in_refs = is_refs_title(&t);
                want_abstract.set(t.eq_ignore_ascii_case("abstract"));
                b.heading(level, &t);
                prev = Some(line);
                continue;
            }

            if in_refs {
                carry = false;
                let labelled = BRACKET_REF.is_match(text);
                let pitch_here = prev.map(|p| p.top - line.top).unwrap_or(0.0);
                let open_unlabelled = ref_entry.as_deref().is_some_and(|e| !BRACKET_REF.is_match(e));
                if labelled && open_unlabelled {
                    // A label set beside the middle of its entry (vertically centred): it belongs to the entry
                    // already open, whose first line came before it.
                    if let (Some(e), Some(m)) = (ref_entry.as_mut(), BRACKET_REF.find(text)) {
                        let label = text[..m.end()].trim();
                        let rest = text[m.end()..].trim();
                        let mut joined = format!("{label} {e}");
                        if !rest.is_empty() {
                            join_line(&mut joined, rest);
                        }
                        *e = joined;
                    }
                    prev = Some(line);
                    continue;
                }
                let starts_entry = labelled
                    || DOT_REF.is_match(text)
                    || (ref_entry.is_some() && line.l <= cl + 0.6 * line.size && prev.is_some_and(|p| p.l > cl + 0.6 * p.size))
                    // Reference lists usually add a little space between entries.
                    || (ref_entry.is_some() && typical_pitch > 0.0 && pitch_here > 1.25 * typical_pitch);
                if starts_entry || ref_entry.is_none() {
                    flush_ref!();
                    ref_entry = Some(text.to_string());
                } else if let Some(e) = ref_entry.as_mut() {
                    join_line(e, text);
                }
                prev = Some(line);
                continue;
            }

            if BULLET.is_match(text) {
                carry = false;
                flush_para!();
                let item = BULLET.replace(text, "").to_string();
                b.list_item(0, "-", &item);
                prev = Some(line);
                continue;
            }

            // Paragraph boundaries: a vertical gap, a first-line indent after a short line, a column change.
            let new_para = match (&para, prev) {
                (None, _) => true,
                (Some(p), Some(pl)) => {
                    let gap = pl.bottom - line.top;
                    let pitch = p.pitch.max(line.size * 1.1);
                    let big_gap = gap > 0.55 * pitch;
                    let indent = line.l > cl + 1.2 * line.size && line.l - pl.l > 0.8 * line.size && pl.r < cr - 2.0 * line.size;
                    // Only a clearly short line (under half the column) ends a paragraph by itself; ragged-right text has
                    // many lines that end early.
                    let short_before = pl.r < cl + 0.5 * (cr - cl).max(1.0) && ends_sentence(&p.text);
                    let col_changed = line.column != p.column;
                    big_gap || indent || short_before || col_changed || (line.size - pl.size).abs() > 0.2 * pl.size.max(1.0)
                }
                (Some(_), None) => true,
            };
            if new_para {
                flush_para!();
                para = Some(Para { text: text.to_string(), last_bottom: line.bottom, last_l: line.l, last_r: line.r, col_l: cl, col_r: cr, pitch: line.top - line.bottom, column: line.column, cont: carry });
                carry = false;
            } else if let Some(p) = para.as_mut() {
                let pitch = prev.map(|pl| pl.top - line.top).unwrap_or(0.0);
                if pitch > 0.0 {
                    p.pitch = pitch;
                }
                join_line(&mut p.text, text);
                p.last_bottom = line.bottom;
                p.last_l = line.l;
                p.last_r = line.r;
                p.col_l = cl;
                p.col_r = cr;
            }
            prev = Some(line);
        }
        // The page ends. An unfinished paragraph is written now and its continuation goes on the next line
        // without a blank line, so the page mark lands where the next page's text starts.
        if let Some(p) = &para {
            carry = !ends_sentence(&p.text);
        }
        flush_para!();
        let _ = page_count;
    }
    flush_ref!();
    flush_para!();

    if metadata.abstract_text.is_none() {
        metadata.abstract_text = abstract_candidate;
    }
    let f = b.finish();
    if metadata.title.is_none() {
        metadata.title = f.headings.iter().find(|h| h.level == 1).map(|h| h.text.clone());
    }
    Assembled {
        extracted: Extracted {
            markdown: f.markdown,
            headings: f.headings,
            pages: f.pages,
            blocks: f.blocks,
            metadata,
            references: refs,
            ..Default::default()
        },
        printed_numbers,
    }
}

fn is_refs_title(t: &str) -> bool {
    let l = t.trim().trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
    let l = l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
    matches!(l, "references" | "reference" | "bibliography" | "works cited" | "literature cited" | "references and notes")
}

fn ends_sentence(t: &str) -> bool {
    t.trim_end().ends_with(['.', '?', '!', ':', '”', '"', ')'])
}

/// Append a continuation line to a paragraph: hyphenated line ends are joined, other breaks become spaces.
fn join_line(acc: &mut String, next: &str) {
    let ends_hyphen = acc.ends_with('-') || acc.ends_with('\u{ad}');
    let before_hyphen = acc.trim_end_matches(['-', '\u{ad}']).chars().last();
    let next_first = next.chars().next();
    if ends_hyphen && before_hyphen.is_some_and(char::is_alphabetic) && next_first.is_some_and(|c| c.is_lowercase()) {
        acc.pop();
        acc.push_str(next);
    } else {
        if !acc.ends_with(' ') {
            acc.push(' ');
        }
        acc.push_str(next);
    }
}

/// The median distance between the tops of consecutive body lines in a column: the normal line pitch.
fn median_pitch(pages: &[PageLines]) -> f32 {
    let mut d: Vec<f32> = Vec::new();
    for p in pages {
        for w in p.lines.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let diff = a.top - b.top;
            if a.column == b.column && diff > 0.5 * a.size && diff < 2.5 * a.size {
                d.push(diff);
            }
        }
    }
    if d.is_empty() {
        return 0.0;
    }
    d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    d[d.len() / 2]
}

/// The most common font size by character count over body-strip lines.
fn body_size(pages: &[PageLines]) -> f32 {
    let mut counts: HashMap<i32, (usize, f32)> = HashMap::new();
    for p in pages {
        for l in &p.lines {
            let e = counts.entry((l.size * 2.0).round() as i32).or_insert((0, l.size));
            e.0 += l.text.chars().count();
        }
    }
    counts.into_values().max_by(|a, b| a.0.cmp(&b.0)).map(|(_, s)| s).unwrap_or(10.0)
}

/// Distinct sizes of lines that look like headings, largest first (rounded to half points).
fn heading_size_classes(pages: &[PageLines], body: f32) -> Vec<f32> {
    // Size classes of heading-like lines with how often each occurs; one-off sizes (a figure label, a pull
    // quote) do not become heading levels, except the largest line on the first page (the title).
    let rounded = |x: f32| (x * 2.0).round() / 2.0;
    let mut counts: Vec<(f32, usize)> = Vec::new();
    for p in pages {
        for l in &p.lines {
            if looks_like_heading(l, body, None, l.l) {
                let s = rounded(l.size);
                match counts.iter_mut().find(|(x, _)| (*x - s).abs() < 0.26) {
                    Some(e) => e.1 += 1,
                    None => counts.push((s, 1)),
                }
            }
        }
    }
    let title_size = pages
        .first()
        .and_then(|p| p.lines.iter().filter(|l| looks_like_heading(l, body, None, l.l)).map(|l| rounded(l.size)).fold(None, |m: Option<f32>, s| Some(m.map_or(s, |x| x.max(s)))));
    let mut sizes: Vec<f32> = counts
        .into_iter()
        .filter(|(s, c)| *c >= 2 || title_size.is_some_and(|t| (t - s).abs() < 0.26 && *s >= body * 1.3))
        .map(|(s, _)| s)
        .collect();
    sizes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    sizes
}

fn looks_like_heading(l: &Line, body: f32, prev: Option<&Line>, col_left: f32) -> bool {
    let t = l.text.trim();
    let n = t.chars().count();
    if n == 0 || n > 140 || t.ends_with([',', ';', ':']) {
        return false;
    }
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    if letters * 2 < n {
        return false;
    }
    let bigger = l.size >= body * 1.15;
    let numbered = NUMBERED_HEADING.is_match(t);
    let section_word = SECTION_WORD.is_match(t);
    // A bold, short, unpunctuated line at body size is a run-in or stand-alone heading.
    let bold_short = l.bold && l.size >= body * 0.95 && n <= 90 && !t.ends_with('.');
    let _ = (prev, col_left);
    (bigger && n <= 120 && !t.ends_with('.')) || (numbered && (l.bold || bigger)) || (section_word && (l.bold || bigger)) || bold_short && (numbered || section_word || n <= 60)
}

fn heading_level(l: &Line, body: f32, classes: &[f32], prev: Option<&Line>, col_left: f32) -> Option<u8> {
    if !looks_like_heading(l, body, prev, col_left) {
        return None;
    }
    // Run-in after body text on the same line cannot be told apart here, so require the line to stand alone.
    let t = l.text.trim();
    if let Some(c) = NUMBERED_HEADING.captures(t) {
        let parts = c[1].split('.').filter(|p| !p.is_empty()).count();
        return Some((parts as u8 + 1).min(6));
    }
    let s = (l.size * 2.0).round() / 2.0;
    match classes.iter().position(|x| (*x - s).abs() < 0.26) {
        Some(rank) => Some((rank as u8 + 1).clamp(1, 4)),
        None if SECTION_WORD.is_match(t) => Some(2),
        None => None,
    }
}

/// Remove running headers and footers and standalone page numbers. Returns the page number each page printed.
fn strip_furniture(pages: &mut [PageLines]) -> Vec<Option<String>> {
    let n = pages.len();
    let normalise = |t: &str| -> String { t.chars().map(|c| if c.is_ascii_digit() { '#' } else { c.to_ascii_lowercase() }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ") };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in pages.iter() {
        let mut seen = std::collections::HashSet::new();
        for l in p.lines.iter().filter(|l| l.furniture) {
            let k = normalise(&l.text);
            if seen.insert(k.clone()) {
                *counts.entry(k).or_default() += 1;
            }
        }
    }
    let threshold = if n >= 6 { (n / 4).max(3) } else { n + 1 };
    let mut printed = vec![None; n];
    for (i, p) in pages.iter_mut().enumerate() {
        let mut keep = Vec::with_capacity(p.lines.len());
        for l in p.lines.drain(..) {
            if l.furniture {
                let t = l.text.trim();
                if let Some(c) = PAGE_NUMBER.captures(t) {
                    printed[i] = Some(c[1].to_string());
                    continue;
                }
                // "Page 3 of 10", "- 3 -" and similar.
                let digits_only: String = t.chars().filter(|c| !c.is_ascii_digit() && !c.is_whitespace() && !matches!(c, '-' | '–' | '|')).collect();
                if digits_only.is_empty() {
                    printed[i] = t.chars().filter(|c| c.is_ascii_digit()).collect::<String>().into();
                    continue;
                }
                if counts.get(&normalise(t)).is_some_and(|&c| c >= threshold) {
                    continue;
                }
            }
            keep.push(l);
        }
        p.lines = keep;
    }
    printed
}

/// Printed page labels inferred from header and footer page numbers when the PDF has no `/PageLabels`.
/// A label is only trusted where at least three pages agree on the offset between number and position, so a
/// stray figure number in a footer cannot produce one.
pub(crate) fn infer_labels(printed: &[Option<String>]) -> Vec<Option<String>> {
    let n = printed.len();
    let mut out: Vec<Option<String>> = vec![None; n];
    // (index, offset, roman?)
    let parsed: Vec<Option<(i64, bool)>> = printed
        .iter()
        .map(|p| {
            let p = p.as_deref()?;
            if let Ok(v) = p.parse::<i64>() {
                Some((v, false))
            } else {
                roman_value(p).map(|v| (v as i64, true))
            }
        })
        .collect();
    let mut i = 0;
    while i < n {
        let Some((v, roman)) = parsed[i] else {
            i += 1;
            continue;
        };
        let offset = v - (i as i64 + 1);
        // Extend while later pages agree (allowing pages with no number in between).
        let mut agree = vec![i];
        let mut j = i + 1;
        let mut misses = 0;
        while j < n && misses <= 2 {
            match parsed[j] {
                Some((v2, r2)) if r2 == roman && v2 - (j as i64 + 1) == offset => {
                    agree.push(j);
                    misses = 0;
                }
                Some(_) => break,
                None => misses += 1,
            }
            j += 1;
        }
        if agree.len() >= 3 {
            let (first, last) = (agree[0], *agree.last().unwrap_or(&agree[0]));
            for (k, slot) in out.iter_mut().enumerate().take(last + 1).skip(first) {
                let val = k as i64 + 1 + offset;
                if val >= 1 {
                    *slot = Some(if roman { to_roman(val as usize) } else { val.to_string() });
                }
            }
            i = last + 1;
        } else {
            i += 1;
        }
    }
    out
}

fn roman_value(s: &str) -> Option<usize> {
    let s = s.to_ascii_lowercase();
    if s.is_empty() || !s.chars().all(|c| "ivxlcdm".contains(c)) {
        return None;
    }
    let val = |c: char| match c {
        'i' => 1,
        'v' => 5,
        'x' => 10,
        'l' => 50,
        'c' => 100,
        'd' => 500,
        _ => 1000,
    };
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0i64;
    for (i, &c) in chars.iter().enumerate() {
        let v = val(c);
        if chars.get(i + 1).is_some_and(|&n| val(n) > v) {
            total -= v;
        } else {
            total += v;
        }
    }
    let total = usize::try_from(total).ok()?;
    // Only canonical numerals count (so "mix" or "dim" do not pass).
    (to_roman(total) == s).then_some(total)
}

fn to_roman(mut n: usize) -> String {
    let table = [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
    let mut s = String::new();
    for (v, r) in table {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// Synthetic glyphs for `text` starting at (x, baseline), with advance 0.5 em and a box 1 em tall.
    pub fn glyphs(text: &str, x: f32, baseline: f32, size: f32, bold: bool) -> Vec<Glyph> {
        let mut out = Vec::new();
        let mut cx = x;
        for ch in text.chars() {
            let adv = if ch == ' ' { 0.3 * size } else { 0.5 * size };
            out.push(Glyph { ch, l: cx, r: cx + adv, b: baseline - 0.2 * size, t: baseline + 0.8 * size, size, bold, italic: false });
            cx += adv;
        }
        out
    }

    pub fn page(glyph_groups: Vec<Vec<Glyph>>) -> PageInput {
        PageInput { width: 612.0, height: 792.0, glyphs: glyph_groups.into_iter().flatten().collect(), label: None, rotated_text: String::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    fn texts(lines: &[Line]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn single_column_reads_top_to_bottom_with_spaces() {
        let p = page(vec![
            glyphs("Second line of text", 72.0, 680.0, 10.0, false),
            glyphs("First line of text", 72.0, 700.0, 10.0, false),
        ]);
        let lines = layout_page(&p);
        assert_eq!(texts(&lines), vec!["First line of text", "Second line of text"]);
    }

    #[test]
    fn two_columns_read_left_then_right_even_when_the_stream_interleaves_them() {
        let mut groups = Vec::new();
        for i in 0..6 {
            let y = 700.0 - i as f32 * 14.0;
            // Stream order: left line, right line, left line, right line...
            groups.push(glyphs(&format!("left column line number {i} text"), 72.0, y, 10.0, false));
            groups.push(glyphs(&format!("right column line number {i} text"), 330.0, y, 10.0, false));
        }
        let lines = layout_page(&page(groups));
        let t = texts(&lines);
        assert_eq!(t.len(), 12);
        assert!(t[..6].iter().all(|l| l.starts_with("left")), "{t:?}");
        assert!(t[6..].iter().all(|l| l.starts_with("right")), "{t:?}");
        assert_eq!(lines[0].column, 0);
        assert_eq!(lines[6].column, 1);
    }

    #[test]
    fn full_width_title_comes_before_the_columns_and_a_wide_figure_caption_splits_them() {
        let mut groups = vec![glyphs("A Very Wide Title Spanning Both Columns Of The Page", 72.0, 740.0, 16.0, true)];
        for i in 0..5 {
            let y = 700.0 - i as f32 * 14.0;
            groups.push(glyphs(&format!("left text {i} aaaaaaaaaaaaaaaaaaaa"), 72.0, y, 10.0, false));
            groups.push(glyphs(&format!("right text {i} bbbbbbbbbbbbbbbbbbbb"), 330.0, y, 10.0, false));
        }
        groups.push(glyphs("Figure 1. A caption that runs across the whole width of the text block here", 72.0, 600.0, 9.0, false));
        for i in 0..5 {
            let y = 580.0 - i as f32 * 14.0;
            groups.push(glyphs(&format!("left again {i} cccccccccccccccccc"), 72.0, y, 10.0, false));
            groups.push(glyphs(&format!("right again {i} dddddddddddddddddd"), 330.0, y, 10.0, false));
        }
        let lines = layout_page(&page(groups));
        let t = texts(&lines);
        assert!(t[0].starts_with("A Very Wide Title"));
        let caption = t.iter().position(|l| l.starts_with("Figure 1")).unwrap();
        // Everything above the caption (both columns) precedes it; everything below follows it.
        assert!(t[1..caption].iter().all(|l| l.starts_with("left text") || l.starts_with("right text")));
        assert!(t[caption + 1..].iter().all(|l| l.contains("again")));
        let first_right = t.iter().position(|l| l.starts_with("right text")).unwrap();
        assert!(t[1..first_right].iter().all(|l| l.starts_with("left text")));
    }

    #[test]
    fn superscripts_and_subscripts_attach_to_their_line() {
        let mut g = glyphs("energy E = m c", 72.0, 700.0, 10.0, false);
        let last = g.last().unwrap().r;
        let mut sup = glyphs("2", last, 704.0, 6.0, false);
        g.append(&mut sup);
        let mut rest = glyphs(" and H", last + 3.0, 700.0, 10.0, false);
        let rr = rest.last().unwrap().r;
        g.append(&mut rest);
        g.append(&mut glyphs("2", rr, 697.0, 6.0, false));
        g.append(&mut glyphs("O", rr + 3.0, 700.0, 10.0, false));
        let lines = layout_page(&page(vec![g]));
        assert_eq!(texts(&lines), vec!["energy E = m c^2 and H_2O"]);
    }

    #[test]
    fn tabular_gaps_split_pieces_but_stay_on_one_line() {
        let g = vec![glyphs("Name", 72.0, 700.0, 10.0, false), glyphs("Value", 200.0, 700.0, 10.0, false), glyphs("Unit", 330.0, 700.0, 10.0, false)];
        let lines = layout_page(&page(g));
        assert_eq!(texts(&lines), vec!["Name Value Unit"]);
    }

    #[test]
    fn page_furniture_is_marked() {
        let g = vec![glyphs("Body text in the middle of the page", 72.0, 400.0, 10.0, false), glyphs("12", 300.0, 30.0, 10.0, false), glyphs("Journal Name Header", 72.0, 770.0, 9.0, false)];
        let lines = layout_page(&page(g));
        let f: Vec<_> = lines.iter().filter(|l| l.furniture).map(|l| l.text.as_str()).collect();
        assert_eq!(f.len(), 2, "{lines:?}");
    }

    fn line(text: &str, l: f32, top: f32, size: f32, bold: bool) -> Line {
        Line { text: text.into(), l, r: l + text.chars().count() as f32 * size * 0.5, top, bottom: top - size, size, bold, column: 0, furniture: false, run_in: false }
    }

    fn body_lines(texts: &[&str], top: f32) -> Vec<Line> {
        texts.iter().enumerate().map(|(i, t)| {
            let mut l = line(t, 72.0, top - i as f32 * 12.0, 10.0, false);
            l.r = 540.0;
            l
        }).collect()
    }

    fn meta() -> Metadata {
        Metadata::default()
    }

    #[test]
    fn headings_paragraphs_and_hyphenation() {
        let mut lines = vec![line("A Study of Things", 72.0, 740.0, 18.0, true), line("1 Introduction", 72.0, 700.0, 12.0, true)];
        lines.extend(body_lines(&["This paragraph has a hyphen-", "ated word and continues here.", "It ends with a sentence."], 680.0));
        // A vertical gap starts a new paragraph.
        lines.extend(body_lines(&["A second paragraph starts after a gap."], 620.0));
        lines.push(line("2.1 Method", 72.0, 590.0, 11.0, true));
        lines.extend(body_lines(&["Method text here."], 570.0));
        // Plenty of body text so that the body size is 10.
        let a = assemble(vec![PageLines { label: Some("5".into()), lines }], meta());
        let md = &a.extracted.markdown;
        assert!(md.contains("# A Study of Things\n\n## 1 Introduction\n\nThis paragraph has a hyphenated word and continues here. It ends with a sentence.\n\nA second paragraph starts after a gap.\n\n### 2.1 Method\n\nMethod text here.\n"), "{md}");
        assert_eq!(a.extracted.pages.len(), 1);
        assert_eq!(a.extracted.pages[0].label.as_deref(), Some("5"));
        assert_eq!(a.extracted.metadata.title.as_deref(), Some("A Study of Things"));
    }

    #[test]
    fn running_headers_footers_and_page_numbers_are_removed_and_numbers_reported() {
        let mut pages = Vec::new();
        for i in 0..8 {
            let mut lines = vec![];
            let mut h = line("Journal of Tests", 72.0, 780.0, 9.0, false);
            h.furniture = true;
            lines.push(h);
            lines.extend(body_lines(&[&format!("Body text of page number {i} goes here and is long enough.")], 600.0));
            let mut n = line(&format!("{}", i + 10), 300.0, 40.0, 9.0, false);
            n.furniture = true;
            lines.push(n);
            pages.push(PageLines { label: None, lines });
        }
        let a = assemble(pages, meta());
        assert!(!a.extracted.markdown.contains("Journal of Tests"), "{}", a.extracted.markdown);
        assert!(!a.extracted.markdown.contains("\n10\n"), "{}", a.extracted.markdown);
        assert_eq!(a.printed_numbers[0].as_deref(), Some("10"));
        assert_eq!(a.printed_numbers[7].as_deref(), Some("17"));
        let labels = infer_labels(&a.printed_numbers);
        assert_eq!(labels[0].as_deref(), Some("10"));
        assert_eq!(labels[7].as_deref(), Some("17"));
    }

    #[test]
    fn label_inference_needs_agreement() {
        let p = |v: &[Option<&str>]| v.iter().map(|x| x.map(str::to_string)).collect::<Vec<_>>();
        // Roman front matter then arabic.
        let printed = p(&[Some("i"), Some("ii"), Some("iii"), Some("iv"), Some("1"), Some("2"), Some("3"), Some("4")]);
        let labels = infer_labels(&printed);
        assert_eq!(labels.iter().map(|l| l.clone().unwrap_or_default()).collect::<Vec<_>>(), vec!["i", "ii", "iii", "iv", "1", "2", "3", "4"]);
        // Gaps (pages with no number) inside a run are filled.
        let printed = p(&[Some("21"), None, Some("23"), Some("24")]);
        assert_eq!(infer_labels(&printed)[1].as_deref(), Some("22"));
        // Two agreeing pages are not enough, and a stray number does not create a run.
        let printed = p(&[Some("5"), Some("6"), None, None]);
        assert!(infer_labels(&printed).iter().all(Option::is_none));
        let printed = p(&[None, Some("17"), None, None, None]);
        assert!(infer_labels(&printed).iter().all(Option::is_none));
    }

    #[test]
    fn roman_numerals() {
        assert_eq!(roman_value("xiv"), Some(14));
        assert_eq!(roman_value("mix"), Some(1009));
        assert_eq!(roman_value("dim"), None);
        assert_eq!(roman_value("iiii"), None);
        assert_eq!(to_roman(1994), "mcmxciv");
    }

    #[test]
    fn references_are_split_by_number_and_hanging_indent() {
        let mut lines = vec![line("References", 72.0, 700.0, 12.0, true)];
        let entries = [("[1] Smith J, Doe A. A study of things. J Stuff 5:1-10 (2019).", 72.0), ("[2] Vaswani A. Attention is all you need. arXiv:1706.03762 (2017).", 72.0)];
        for (i, (t, x)) in entries.iter().enumerate() {
            let mut l = line(t, *x, 680.0 - i as f32 * 12.0, 10.0, false);
            l.r = 540.0;
            lines.push(l);
        }
        // Body lines elsewhere keep the body size at 10.
        let mut long = body_lines(&["Some prose before the references that is plain body text."; 4], 760.0);
        long.extend(lines);
        let a = assemble(vec![PageLines { label: None, lines: long }], meta());
        assert_eq!(a.extracted.references.len(), 2, "{}", a.extracted.markdown);
        assert_eq!(a.extracted.references[1].arxiv_id.as_deref(), Some("1706.03762"));
        assert_eq!(a.extracted.references[0].year, Some(2019));
        assert!(a.extracted.markdown.contains("[1] Smith J, Doe A."), "{}", a.extracted.markdown);

        // Hanging indent: continuation lines are indented, entries start at the margin.
        let mut lines = body_lines(&["Plain body text lines to set the body size."; 4], 760.0);
        lines.push(line("Bibliography", 72.0, 700.0, 12.0, true));
        let mk = |t: &str, x: f32, y: f32| {
            let mut l = line(t, x, y, 10.0, false);
            l.r = 540.0;
            l
        };
        lines.push(mk("Smith, J. (2019). A study of things.", 72.0, 680.0));
        lines.push(mk("Journal of Stuff, 5, 1-10.", 90.0, 668.0));
        lines.push(mk("Doe, A. (2018). Another study.", 72.0, 656.0));
        lines.push(mk("Press.", 90.0, 644.0));
        let a = assemble(vec![PageLines { label: None, lines }], meta());
        assert_eq!(a.extracted.references.len(), 2, "{}", a.extracted.markdown);
        assert_eq!(a.extracted.references[0].raw, "Smith, J. (2019). A study of things. Journal of Stuff, 5, 1-10.");
    }

    #[test]
    fn bullets_become_a_list_and_abstract_is_captured() {
        let mut lines = vec![line("Abstract", 72.0, 720.0, 11.0, true)];
        lines.extend(body_lines(&["We study the thing and report the findings in this abstract text."], 700.0));
        let mut b1 = line("• first point", 72.0, 660.0, 10.0, false);
        b1.r = 300.0;
        let mut b2 = line("• second point", 72.0, 648.0, 10.0, false);
        b2.r = 300.0;
        lines.push(b1);
        lines.push(b2);
        lines.extend(body_lines(&["Closing remark that is body text for sizing purposes."; 3], 560.0));
        let a = assemble(vec![PageLines { label: None, lines }], meta());
        assert!(a.extracted.markdown.contains("- first point\n- second point"), "{}", a.extracted.markdown);
        assert_eq!(a.extracted.metadata.abstract_text.as_deref(), Some("We study the thing and report the findings in this abstract text."));
    }

    #[test]
    fn paragraph_continues_across_pages_when_unfinished() {
        let p1 = PageLines { label: None, lines: body_lines(&["A sentence that has not ended and continues onto"], 700.0) };
        let p2 = PageLines { label: None, lines: body_lines(&["the next page before finishing."], 700.0) };
        let a = assemble(vec![p1, p2], meta());
        // One paragraph on two lines with no blank line between, so the second page's mark has its own line.
        assert_eq!(a.extracted.markdown, "A sentence that has not ended and continues onto
the next page before finishing.
");
        assert_eq!(a.extracted.pages.iter().map(|p| p.line).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn hyphen_join_rules() {
        let mut s = "well-".to_string();
        join_line(&mut s, "known fact");
        assert_eq!(s, "wellknown fact");
        let mut s = "pre-".to_string();
        join_line(&mut s, "Fixed");
        assert_eq!(s, "pre- Fixed");
        let mut s = "ends in a dash -".to_string();
        join_line(&mut s, "next");
        assert_eq!(s, "ends in a dash - next");
    }
}

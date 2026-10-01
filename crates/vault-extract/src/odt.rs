//! ODT (OpenDocument text) to canonical-ready Markdown: outline headings, lists (ordered or bulleted by their list
//! style), tables with spans, footnotes, text boxes and embedded formula objects (MathML to LaTeX).

use std::collections::HashMap;

use roxmltree::Node;

use crate::ConvertOptions;
use crate::archive::ZipDoc;
use crate::builder::{MdBuilder, collapse_ws};
use crate::error::{ExtractError, Result};
use crate::ids;
use crate::mathml;
use crate::tables::{Cell, Grid};
use crate::types::{Extracted, Metadata, tool_string};
use crate::xmlutil::{self, attr, child, descendant, is, name, text_of};

const MAX_DEPTH: usize = 64;
/// Repeated empty rows and cells (spreadsheet-style padding) are dropped; repeated content is expanded up to this.
const MAX_REPEAT: usize = 50;

#[derive(Default, Clone)]
struct PStyle {
    display: String,
    parent: Option<String>,
    mono: bool,
}

#[derive(Default)]
struct Styles {
    paragraph: HashMap<String, PStyle>,
    /// list style name -> level (1-based) -> ordered?
    lists: HashMap<String, HashMap<usize, bool>>,
}

fn decode_style_name(n: &str) -> String {
    n.replace("_20_", " ").replace("_5f_", "_").to_lowercase()
}

fn read_styles(root: Node, out: &mut Styles) {
    for s in root.descendants().filter(|n| is(*n, "style")) {
        if attr(s, "family") != Some("paragraph") {
            continue;
        }
        let Some(nm) = attr(s, "name") else { continue };
        let mono = s.children().filter(|c| is(*c, "text-properties")).any(|tp| {
            let f = attr(tp, "font-name").or_else(|| attr(tp, "font-family")).unwrap_or("").to_lowercase();
            ["courier", "consolas", "monaco", "menlo", "mono", "lucida console", "source code"].iter().any(|m| f.contains(m))
        });
        out.paragraph.insert(
            nm.to_string(),
            PStyle { display: attr(s, "display-name").map(str::to_lowercase).unwrap_or_else(|| decode_style_name(nm)), parent: attr(s, "parent-style-name").map(str::to_string), mono },
        );
    }
    for ls in root.descendants().filter(|n| is(*n, "list-style")) {
        let Some(nm) = attr(ls, "name") else { continue };
        let mut levels = HashMap::new();
        for l in ls.children().filter(Node::is_element) {
            let level = attr(l, "level").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
            match name(l) {
                "list-level-style-number" => {
                    // A number-less level style (num-format empty) is a plain indent.
                    levels.insert(level, attr(l, "num-format").is_none_or(|f| !f.is_empty()));
                }
                "list-level-style-bullet" | "list-level-style-image" => {
                    levels.insert(level, false);
                }
                _ => {}
            }
        }
        out.lists.insert(nm.to_string(), levels);
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Role {
    Title,
    Quote,
    Code,
    Normal,
}

struct Odt<'z, 'a> {
    b: MdBuilder,
    styles: Styles,
    zip: &'z mut ZipDoc<'a>,
    notes: Vec<(usize, String)>,
    code: Vec<String>,
    title_offset: u8,
    warnings: Vec<String>,
}

pub fn convert(bytes: &[u8], _opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut zip = ZipDoc::open(bytes, "odt")?;
    let content = zip.read("content.xml")?.ok_or_else(|| ExtractError::corrupt("odt", "content.xml is missing"))?;
    let styles_xml = zip.read("styles.xml")?;
    let meta_xml = zip.read("meta.xml")?;

    let mut styles = Styles::default();
    if let Some(x) = &styles_xml {
        xmlutil::with_document(x, "odt", |d, _| {
            read_styles(d.root_element(), &mut styles);
            Ok(())
        })?;
    }
    let mut meta = Metadata::default();
    if let Some(x) = &meta_xml {
        meta = xmlutil::with_document(x, "odt", |d, _| Ok(read_meta(d.root_element())))?;
    }

    let mut o = Odt { b: MdBuilder::new(), styles, zip: &mut zip, notes: Vec::new(), code: Vec::new(), title_offset: 0, warnings: Vec::new() };
    xmlutil::with_document(&content, "odt", |doc, _| {
        let root = doc.root_element();
        read_styles(root, &mut o.styles);
        let text = root
            .descendants()
            .find(|n| is(*n, "text") && n.parent().is_some_and(|p| is(p, "body")))
            .ok_or_else(|| ExtractError::corrupt("odt", "content.xml has no office:text body"))?;
        o.title_offset = u8::from(text.descendants().any(|p| is(p, "p") && o.role_of(p) == Role::Title));
        o.flow(text, 0);
        o.flush_code();
        o.write_notes();
        Ok(())
    })?;

    let warnings = std::mem::take(&mut o.warnings);
    let f = o.b.finish();
    if f.markdown.trim().is_empty() {
        return Err(ExtractError::convert("odt", "the document holds no text"));
    }
    if meta.title.is_none() {
        meta.title = f.headings.iter().find(|h| h.level == 1).map(|h| h.text.clone());
    }
    Ok(Extracted {
        markdown: f.markdown,
        headings: f.headings,
        pages: f.pages,
        blocks: f.blocks,
        metadata: meta,
        references: Vec::new(),
        warnings,
        tool: tool_string("odt"),
        encoding: Some("utf-8".to_string()),
    })
}

fn read_meta(root: Node) -> Metadata {
    let mut m = Metadata::default();
    for e in root.descendants().filter(Node::is_element) {
        let v = text_of(e);
        if v.is_empty() {
            continue;
        }
        match name(e) {
            "title" => m.title = Some(v),
            "creator" => m.authors = vec![v],
            "initial-creator" if m.authors.is_empty() => m.authors = vec![v],
            "language" => m.language = Some(v),
            "keyword" => m.keywords.push(v),
            "creation-date" if m.year.is_none() => m.year = ids::year_in(&v),
            "description" | "subject" if m.abstract_text.is_none() => m.abstract_text = Some(v),
            _ => {}
        }
    }
    m
}

impl Odt<'_, '_> {
    fn role_of(&self, p: Node) -> Role {
        let mut cur = attr(p, "style-name").map(str::to_string);
        let mut mono = false;
        for _ in 0..8 {
            let Some(id) = cur else { break };
            let Some(s) = self.styles.paragraph.get(&id) else {
                let d = decode_style_name(&id);
                return classify(&d, mono);
            };
            mono |= s.mono;
            let r = classify(&s.display, mono);
            if r != Role::Normal {
                return r;
            }
            let r = classify(&decode_style_name(&id), mono);
            if r != Role::Normal {
                return r;
            }
            cur = s.parent.clone();
        }
        if mono { Role::Code } else { Role::Normal }
    }

    fn write_notes(&mut self) {
        if self.notes.is_empty() {
            return;
        }
        self.b.heading(2, "Notes");
        for (n, t) in std::mem::take(&mut self.notes) {
            self.b.paragraph(&format!("[^{n}]: {t}"));
        }
    }

    fn flush_code(&mut self) {
        if self.code.is_empty() {
            return;
        }
        let code = std::mem::take(&mut self.code).join("\n");
        self.b.code_block("", &code);
    }

    fn flow(&mut self, parent: Node, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        for n in parent.children().filter(Node::is_element) {
            match name(n) {
                "h" => {
                    self.flush_code();
                    let level = attr(n, "outline-level").and_then(|l| l.parse::<u8>().ok()).unwrap_or(1);
                    let t = self.inline(n, depth);
                    self.b.heading((level + self.title_offset).min(6), &t);
                }
                "p" => self.paragraph(n, depth),
                "list" => {
                    self.flush_code();
                    self.b.break_list();
                    let style = attr(n, "style-name").map(str::to_string);
                    self.list(n, style, 1, depth);
                }
                "table" => {
                    self.flush_code();
                    self.table(n, depth);
                }
                "section" | "text-box" | "index-body" => self.flow(n, depth + 1),
                "table-of-content" | "illustration-index" | "table-index" | "object-index" | "alphabetical-index" | "user-index" | "tracked-changes" | "sequence-decls" | "variable-decls" | "user-field-decls" => {}
                "bibliography" => self.flow(n, depth + 1),
                _ => {}
            }
        }
    }

    fn paragraph(&mut self, p: Node, depth: usize) {
        let role = self.role_of(p);
        // A paragraph that holds only a formula object is a display equation.
        let mut formulas = Vec::new();
        let text = self.inline_collect(p, depth, &mut formulas);
        if role == Role::Code {
            self.code.push(raw_code_text(p));
            return;
        }
        self.flush_code();
        if formulas.len() == 1 && text.trim() == format!("${}$", formulas[0]) {
            self.b.display_math(&formulas[0]);
            return;
        }
        let t = collapse_ws(&text);
        match role {
            Role::Title => self.b.heading(1, &t),
            Role::Quote => self.b.quote(&t),
            _ => self.b.paragraph(&t),
        }
        self.drain_boxes(p, depth);
    }

    /// Paragraphs inside text boxes anchored in `p`.
    fn drain_boxes(&mut self, p: Node, depth: usize) {
        for tb in p.descendants().filter(|n| is(*n, "text-box")) {
            if depth < MAX_DEPTH {
                self.flow(tb, depth + 1);
            }
        }
    }

    fn list(&mut self, l: Node, style: Option<String>, level: usize, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let ordered = style.as_ref().and_then(|s| self.styles.lists.get(s)).and_then(|lv| lv.get(&level)).copied().unwrap_or(false);
        let mut n = attr(l, "start-value").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
        for item in l.children().filter(|c| is(*c, "list-item") || is(*c, "list-header")) {
            let mut text = String::new();
            let mut nested = Vec::new();
            for k in item.children().filter(Node::is_element) {
                match name(k) {
                    "list" => nested.push(k),
                    "p" | "h" => {
                        let t = self.inline(k, depth + 1);
                        if !t.is_empty() {
                            if !text.is_empty() {
                                text.push(' ');
                            }
                            text.push_str(&t);
                        }
                    }
                    _ => {}
                }
            }
            let marker = if ordered { format!("{n}.") } else { "-".to_string() };
            self.b.list_item(level - 1, &marker, &text);
            n += 1;
            for nl in nested {
                let inner_style = attr(nl, "style-name").map(str::to_string).or_else(|| style.clone());
                self.list(nl, inner_style, level + 1, depth + 1);
            }
        }
    }

    fn table(&mut self, t: Node, depth: usize) {
        let mut g = Grid::default();
        let mut header_rows = 0usize;
        let mut add_row = |this: &mut Self, tr: Node, header: bool, g: &mut Grid| {
            let repeat = attr(tr, "number-rows-repeated").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
            let mut cells: Vec<Cell> = Vec::new();
            for tc in tr.children().filter(|c| is(*c, "table-cell")) {
                let span = |a: &str| attr(tc, a).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
                let text = this.cell_text(tc, depth);
                let reps = span("number-columns-repeated");
                if reps > 1 && text.is_empty() {
                    continue;
                }
                for _ in 0..reps.min(MAX_REPEAT) {
                    cells.push(Cell::new(text.clone(), span("number-columns-spanned"), span("number-rows-spanned")));
                }
            }
            if cells.iter().all(|c| c.text.is_empty()) && repeat > 1 {
                return;
            }
            for _ in 0..repeat.min(MAX_REPEAT) {
                if header {
                    header_rows += 1;
                }
                g.rows.push(cells.clone());
            }
        };
        for k in t.children().filter(Node::is_element) {
            match name(k) {
                "table-header-rows" => {
                    for tr in k.children().filter(|c| is(*c, "table-row")) {
                        add_row(self, tr, true, &mut g);
                    }
                }
                "table-rows" | "table-row-group" => {
                    for tr in k.children().filter(|c| is(*c, "table-row")) {
                        add_row(self, tr, false, &mut g);
                    }
                }
                "table-row" => add_row(self, k, false, &mut g),
                _ => {}
            }
        }
        g.header_rows = header_rows;
        g.emit(&mut self.b);
    }

    fn cell_text(&mut self, tc: Node, depth: usize) -> String {
        let mut parts = Vec::new();
        for p in tc.descendants().filter(|n| is(*n, "p") || is(*n, "h")) {
            // Nested tables' paragraphs appear through descendants; each paragraph is visited once.
            let t = self.inline(p, depth + 1);
            if !t.is_empty() {
                parts.push(t);
            }
        }
        parts.join(" ")
    }

    // ------------------------------------------------------------------------------------------------ inline

    fn inline(&mut self, p: Node, depth: usize) -> String {
        let mut formulas = Vec::new();
        collapse_ws(&self.inline_collect(p, depth, &mut formulas))
    }

    fn inline_collect(&mut self, p: Node, depth: usize, formulas: &mut Vec<String>) -> String {
        let mut out = String::new();
        self.inline_into(p, &mut out, depth, formulas);
        out
    }

    fn inline_into(&mut self, n: Node, out: &mut String, depth: usize, formulas: &mut Vec<String>) {
        if depth > MAX_DEPTH {
            return;
        }
        for c in n.children() {
            if c.is_text() {
                out.push_str(c.text().unwrap_or(""));
                continue;
            }
            if !c.is_element() {
                continue;
            }
            match name(c) {
                "s" => {
                    let k = attr(c, "c").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).min(200);
                    out.push_str(&" ".repeat(k));
                }
                "tab" => out.push('\t'),
                "line-break" => out.push('\n'),
                "note" => {
                    let body = child(c, "note-body");
                    let t = body.map(|b| b.children().filter(Node::is_element).map(|p| self.inline(p, depth + 1)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                    if !t.is_empty() {
                        let num = self.notes.len() + 1;
                        self.notes.push((num, t));
                        out.push_str(&format!("[^{num}]"));
                    }
                }
                "frame" => {
                    if let Some(obj) = descendant(c, "object") {
                        if let Some(l) = self.formula(obj) {
                            out.push('$');
                            out.push_str(&l);
                            out.push('$');
                            formulas.push(l);
                        }
                    }
                    // Text boxes are written after the paragraph by the caller.
                }
                "annotation" | "annotation-end" | "tracked-changes" | "change" | "change-start" | "change-end" | "soft-page-break" | "bookmark" | "bookmark-start" | "bookmark-end" | "image" | "text-box" | "ruby-text" | "object" => {}
                "sequence" | "span" | "a" | "ruby" | "ruby-base" | "number" | "date" | "page-number" | "title" | "subject" | "author-name" | "bookmark-ref" | "reference-ref" | "sequence-ref" | "user-defined" | "user-field-get" | "variable-get" | "text-input" | "meta" | "meta-field" | "hidden-text" | "conditional-text" | "initial-creator" | "creator" | "file-name" | "page-count" | "word-count" | "paragraph-count" | "character-count" | "table-count" | "image-count" | "object-count" | "description" | "keywords" | "time" | "chapter" | "name" => {
                    self.inline_into(c, out, depth + 1, formulas)
                }
                _ => self.inline_into(c, out, depth + 1, formulas),
            }
        }
    }

    /// LaTeX from an embedded formula object (a sub-document holding MathML).
    fn formula(&mut self, obj: Node) -> Option<String> {
        let href = attr(obj, "href")?.trim_start_matches("./").trim_end_matches('/').to_string();
        let data = self.zip.read(&format!("{href}/content.xml")).ok().flatten()?;
        let latex = xmlutil::with_document(&data, "odt", |d, _| {
            let math = d.descendants().find(|n| is(*n, "math"));
            Ok(math.and_then(|m| mathml::to_latex(&mathml::from_xml(m))))
        })
        .ok()
        .flatten();
        if latex.is_none() {
            self.warnings.push(format!("formula object {href} could not be converted"));
        }
        latex
    }
}

fn classify(display: &str, mono: bool) -> Role {
    let d = display.trim();
    if d == "title" {
        Role::Title
    } else if d == "quotations" || d == "quote" || d == "block quote" || d == "block quotation" {
        Role::Quote
    } else if mono || d.contains("preformatted") || d.contains("source text") || d.contains("source code") || d == "code" || d.contains("verbatim") {
        Role::Code
    } else {
        Role::Normal
    }
}

/// A code paragraph's text with spaces, tabs and breaks as written.
fn raw_code_text(p: Node) -> String {
    let mut s = String::new();
    for d in p.descendants() {
        if d.is_text() {
            s.push_str(d.text().unwrap_or(""));
        } else if d.is_element() {
            match name(d) {
                "s" => s.push_str(&" ".repeat(attr(d, "c").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).min(200))),
                "tab" => s.push('\t'),
                "line-break" => s.push('\n'),
                _ => {}
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;
    use std::io::{Cursor, Write};

    const NS: &str = r#"xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0""#;

    fn odt(body: &str, auto: &str, extra: &[(&str, String)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.start_file("mimetype", stored).unwrap();
        w.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
        w.start_file("content.xml", o).unwrap();
        write!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content {NS}><office:automatic-styles>{auto}</office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document-content>").unwrap();
        for (n, c) in extra {
            w.start_file(*n, o).unwrap();
            w.write_all(c.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn run(body: &str, auto: &str, extra: &[(&str, String)]) -> Extracted {
        convert(&odt(body, auto, extra), &ConvertOptions::default()).unwrap()
    }

    #[test]
    fn headings_paragraphs_spaces_and_breaks() {
        let r = run(
            r#"<text:h text:outline-level="1">Main <text:span>title</text:span></text:h><text:p>One<text:s text:c="3"/>two<text:tab/>three<text:line-break/>four <text:a xlink:href="http://x.y">linked</text:a>.</text:p><text:h text:outline-level="2">Sub</text:h><text:p>Last.</text:p>"#,
            "",
            &[],
        );
        assert_eq!(r.markdown, "# Main title\n\nOne two three four linked.\n\n## Sub\n\nLast.\n");
        let hs: Vec<(u8, &str, u32)> = r.headings.iter().map(|h| (h.level, h.text.as_str(), h.line)).collect();
        assert_eq!(hs, vec![(1, "Main title", 1), (2, "Sub", 5)]);
    }

    #[test]
    fn title_style_pushes_headings_down() {
        let auto = r#"<style:style style:name="P1" style:family="paragraph" style:parent-style-name="Title"/>"#;
        let r = run(r#"<text:p text:style-name="P1">Doc Title</text:p><text:h text:outline-level="1">Chapter</text:h>"#, auto, &[]);
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(hs, vec![(1, "Doc Title"), (2, "Chapter")]);
    }

    #[test]
    fn lists_follow_their_list_style() {
        let auto = r#"<text:list-style style:name="L1"><text:list-level-style-bullet text:level="1"/><text:list-level-style-bullet text:level="2"/></text:list-style>
<text:list-style style:name="L2"><text:list-level-style-number text:level="1" style:num-format="1"/><text:list-level-style-number text:level="2" style:num-format="a"/></text:list-style>"#;
        let body = r#"<text:list text:style-name="L1"><text:list-item><text:p>bullet</text:p><text:list><text:list-item><text:p>nested bullet</text:p></text:list-item></text:list></text:list-item></text:list>
<text:list text:style-name="L2"><text:list-item><text:p>first</text:p></text:list-item><text:list-item><text:p>second</text:p><text:list><text:list-item><text:p>child</text:p></text:list-item></text:list></text:list-item></text:list>"#;
        let r = run(body, auto, &[]);
        assert_eq!(r.markdown, "- bullet\n  - nested bullet\n\n1. first\n2. second\n  1. child\n");
    }

    #[test]
    fn tables_with_header_rows_and_spans() {
        let body = r#"<table:table><table:table-header-rows><table:table-row><table:table-cell><text:p>Name</text:p></table:table-cell><table:table-cell table:number-columns-spanned="2"><text:p>Score</text:p></table:table-cell><table:covered-table-cell/></table:table-row></table:table-header-rows>
<table:table-row><table:table-cell><text:p>A | B</text:p></table:table-cell><table:table-cell><text:p>1</text:p></table:table-cell><table:table-cell><text:p>2</text:p></table:table-cell><table:table-cell table:number-columns-repeated="1000"/></table:table-row>
<table:table-row table:number-rows-repeated="500"><table:table-cell table:number-columns-repeated="1024"/></table:table-row></table:table>"#;
        let r = run(body, "", &[]);
        assert_eq!(r.markdown, "| Name | Score |  |\n| --- | --- | --- |\n| A \\| B | 1 | 2 |\n");
        assert_eq!(r.blocks[0].kind, BlockKind::Table);
    }

    #[test]
    fn footnotes_quotes_and_code() {
        let auto = r#"<style:style style:name="Q" style:family="paragraph" style:parent-style-name="Quotations"/><style:style style:name="C" style:family="paragraph" style:parent-style-name="Preformatted_20_Text"/>"#;
        let body = r#"<text:p>Claim<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>Source text.</text:p></text:note-body></text:note> made.</text:p>
<text:p text:style-name="Q">A quotation.</text:p><text:p text:style-name="C">if x:<text:s text:c="4"/>run()</text:p><text:p text:style-name="C">done</text:p><text:p>After.</text:p>"#;
        let r = run(body, auto, &[]);
        assert_eq!(r.markdown, "Claim[^1] made.\n\n> A quotation.\n\n```\nif x:    run()\ndone\n```\n\nAfter.\n\n## Notes\n\n[^1]: Source text.\n");
    }

    #[test]
    fn formula_objects_become_latex() {
        let body = r#"<text:p>Value <draw:frame><draw:object xlink:href="./Object 1"/></draw:frame> here.</text:p><text:p><draw:frame><draw:object xlink:href="./Object 2"/></draw:frame></text:p>"#;
        let o1 = r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><semantics><mrow><msup><mi>x</mi><mn>2</mn></msup></mrow><annotation encoding="StarMath 5.0">x^2</annotation></semantics></math>"#;
        let o2 = r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mfrac><mn>1</mn><mn>2</mn></mfrac></math>"#;
        let r = run(body, "", &[("Object 1/content.xml", o1.to_string()), ("Object 2/content.xml", o2.to_string())]);
        assert!(r.markdown.contains("Value $x^2$ here."), "{}", r.markdown);
        assert!(r.markdown.contains("$$\n\\frac{1}{2}\n$$"), "{}", r.markdown);
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Equation).count(), 1);
    }

    #[test]
    fn text_boxes_and_index_are_handled() {
        let body = r#"<text:table-of-content><text:index-body><text:p>1 Intro ....... 3</text:p></text:index-body></text:table-of-content><text:p>Body<draw:frame><draw:text-box><text:p>Box text.</text:p></draw:text-box></draw:frame></text:p>"#;
        let r = run(body, "", &[]);
        assert_eq!(r.markdown, "Body\n\nBox text.\n");
    }

    #[test]
    fn metadata_from_meta_xml() {
        let meta = format!(r#"<office:document-meta {NS}><office:meta><dc:title>ODT Title</dc:title><dc:creator>Ann Lee</dc:creator><dc:language>de-DE</dc:language><meta:keyword>a</meta:keyword><meta:keyword>b</meta:keyword><meta:creation-date>2018-03-01T10:00:00</meta:creation-date></office:meta></office:document-meta>"#);
        let r = run("<text:p>Text.</text:p>", "", &[("meta.xml", meta)]);
        assert_eq!(r.metadata.title.as_deref(), Some("ODT Title"));
        assert_eq!(r.metadata.authors, vec!["Ann Lee"]);
        assert_eq!(r.metadata.language.as_deref(), Some("de-DE"));
        assert_eq!(r.metadata.keywords, vec!["a", "b"]);
        assert_eq!(r.metadata.year, Some(2018));
    }

    #[test]
    fn broken_odt_files() {
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
        assert_eq!(convert(b"not a zip", &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file("x.txt", zip::write::SimpleFileOptions::default()).unwrap();
        let z = w.finish().unwrap().into_inner();
        assert_eq!(convert(&z, &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        assert_eq!(convert(&odt("<text:p/>", "", &[]), &ConvertOptions::default()).unwrap_err().reason_code(), "extraction_failed");
    }
}

//! DOCX (WordprocessingML) to canonical-ready Markdown: styles give headings, quotes and code; numbering gives
//! lists and numbered headings; tables become pipe tables; footnotes become `[^n]` notes; Office Math (OMML)
//! becomes LaTeX.

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
use crate::xmlutil::{self, attr, text_of};

const MAX_DEPTH: usize = 64;

fn ns_is_w(n: Node) -> bool {
    n.tag_name().namespace().is_some_and(|u| u.contains("wordprocessingml"))
}

fn ns_is_m(n: Node) -> bool {
    n.tag_name().namespace().is_some_and(|u| u.ends_with("/math") || u.ends_with("officeDocument/2006/math"))
}

fn w(n: Node, local: &str) -> bool {
    n.is_element() && ns_is_w(n) && n.tag_name().name() == local
}

fn m(n: Node, local: &str) -> bool {
    n.is_element() && ns_is_m(n) && n.tag_name().name() == local
}

fn w_child<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| w(*c, local))
}

fn m_child<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| m(*c, local))
}

/// The `val` attribute of a property child such as `<w:pStyle w:val="Heading1"/>`.
fn w_val<'a>(n: Node<'a, '_>, child: &str) -> Option<&'a str> {
    w_child(n, child).and_then(|c| attr(c, "val"))
}

// ------------------------------------------------------------------------------------------------ styles

#[derive(Default, Clone)]
struct Style {
    name: String,
    based_on: Option<String>,
    outline: Option<u8>,
    num: Option<(String, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Role {
    Title,
    Heading(u8),
    Quote,
    Code,
    Normal,
}

#[derive(Default)]
struct Numbering {
    /// numId -> abstractNumId
    num_to_abs: HashMap<String, String>,
    /// (abstractNumId, ilvl) -> level definition
    levels: HashMap<(String, u8), Level>,
}

#[derive(Clone, Default)]
struct Level {
    fmt: String,
    text: String,
    start: usize,
}

fn read_styles(root: Node) -> HashMap<String, Style> {
    let mut out = HashMap::new();
    for s in root.children().filter(|c| w(*c, "style")) {
        let Some(id) = attr(s, "styleId") else { continue };
        let ppr = w_child(s, "pPr");
        let num = ppr.and_then(|p| w_child(p, "numPr")).and_then(|np| {
            let id = w_val(np, "numId")?.to_string();
            let lvl = w_val(np, "ilvl").and_then(|l| l.parse().ok()).unwrap_or(0);
            Some((id, lvl))
        });
        out.insert(
            id.to_string(),
            Style {
                name: w_val(s, "name").unwrap_or("").to_lowercase(),
                based_on: w_val(s, "basedOn").map(str::to_string),
                outline: ppr.and_then(|p| w_val(p, "outlineLvl")).and_then(|v| v.parse::<u8>().ok()).filter(|l| *l < 9),
                num,
            },
        );
    }
    out
}

fn read_numbering(root: Node) -> Numbering {
    let mut n = Numbering::default();
    for a in root.children().filter(|c| w(*c, "abstractNum")) {
        let Some(id) = attr(a, "abstractNumId") else { continue };
        for l in a.children().filter(|c| w(*c, "lvl")) {
            let ilvl: u8 = attr(l, "ilvl").and_then(|v| v.parse().ok()).unwrap_or(0);
            n.levels.insert(
                (id.to_string(), ilvl),
                Level {
                    fmt: w_val(l, "numFmt").unwrap_or("decimal").to_string(),
                    text: w_val(l, "lvlText").unwrap_or("").to_string(),
                    start: w_val(l, "start").and_then(|v| v.parse().ok()).unwrap_or(1),
                },
            );
        }
    }
    for num in root.children().filter(|c| w(*c, "num")) {
        if let (Some(id), Some(abs)) = (attr(num, "numId"), w_val(num, "abstractNumId")) {
            n.num_to_abs.insert(id.to_string(), abs.to_string());
        }
    }
    n
}

fn format_number(fmt: &str, n: usize) -> String {
    let letters = |n: usize, base: u8| -> String {
        let mut n = n.max(1);
        let mut s = Vec::new();
        while n > 0 {
            n -= 1;
            s.push(base + (n % 26) as u8);
            n /= 26;
        }
        s.reverse();
        String::from_utf8_lossy(&s).into_owned()
    };
    let roman = |mut n: usize| -> String {
        let table = [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
        let mut s = String::new();
        for (v, r) in table {
            while n >= v {
                s.push_str(r);
                n -= v;
            }
        }
        s
    };
    match fmt {
        "lowerLetter" => letters(n, b'a'),
        "upperLetter" => letters(n, b'A'),
        "lowerRoman" => roman(n),
        "upperRoman" => roman(n).to_uppercase(),
        "none" => String::new(),
        _ => n.to_string(),
    }
}

// ------------------------------------------------------------------------------------------------ converter

/// What a paragraph's content turned into.
#[derive(Default)]
struct ParaContent {
    segments: Vec<Seg>,
    text: String,
    runs: usize,
    mono_runs: usize,
    boxes: Vec<String>,
    math_only: Option<String>,
}

enum Seg {
    Text(String),
    Display(String),
}

struct Docx {
    b: MdBuilder,
    styles: HashMap<String, Style>,
    numbering: Numbering,
    footnotes: HashMap<String, String>,
    endnotes: HashMap<String, String>,
    /// numId -> counters per level
    counters: HashMap<String, Vec<usize>>,
    /// Notes in the order they are referenced: (display number, text).
    used_notes: Vec<(usize, String)>,
    code: Vec<String>,
    title_offset: u8,
    warnings: Vec<String>,
    /// The list (numId) the previous paragraph belonged to, so a new list starts a new block.
    current_list: Option<String>,
}

pub fn convert(bytes: &[u8], _opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut zip = ZipDoc::open(bytes, "docx")?;
    let main = main_part(&mut zip)?;
    let dir = main.rsplit_once('/').map(|(d, _)| format!("{d}/")).unwrap_or_default();
    let doc_bytes = zip.read(&main)?.ok_or_else(|| ExtractError::corrupt("docx", format!("{main} is missing")))?;
    let rels = relationships(&mut zip, &main)?;
    let part = |zip: &mut ZipDoc, kind: &str, default: &str| -> Result<Option<Vec<u8>>> {
        let path = rels.get(kind).map(|t| resolve_target(&dir, t)).unwrap_or_else(|| default.to_string());
        zip.read(&path)
    };
    let styles_xml = part(&mut zip, "styles", "word/styles.xml")?;
    let numbering_xml = part(&mut zip, "numbering", "word/numbering.xml")?;
    let footnotes_xml = part(&mut zip, "footnotes", "word/footnotes.xml")?;
    let endnotes_xml = part(&mut zip, "endnotes", "word/endnotes.xml")?;
    let core_xml = zip.read("docProps/core.xml")?;

    let styles = match &styles_xml {
        Some(x) => xmlutil::with_document(x, "docx", |d, _| Ok(read_styles(d.root_element())))?,
        None => HashMap::new(),
    };
    let numbering = match &numbering_xml {
        Some(x) => xmlutil::with_document(x, "docx", |d, _| Ok(read_numbering(d.root_element())))?,
        None => Numbering::default(),
    };
    let mut c = Docx {
        b: MdBuilder::new(),
        styles,
        numbering,
        footnotes: HashMap::new(),
        endnotes: HashMap::new(),
        counters: HashMap::new(),
        used_notes: Vec::new(),
        code: Vec::new(),
        title_offset: 0,
        warnings: Vec::new(),
        current_list: None,
    };
    if let Some(x) = &footnotes_xml {
        c.footnotes = xmlutil::with_document(x, "docx", |d, _| Ok(c.read_notes(d.root_element(), "footnote")))?;
    }
    if let Some(x) = &endnotes_xml {
        c.endnotes = xmlutil::with_document(x, "docx", |d, _| Ok(c.read_notes(d.root_element(), "endnote")))?;
    }
    let mut meta = Metadata::default();
    if let Some(x) = &core_xml {
        meta = xmlutil::with_document(x, "docx", |d, _| Ok(read_core(d.root_element())))?;
    }

    xmlutil::with_document(&doc_bytes, "docx", |doc, enc| {
        let root = doc.root_element();
        let body = root.children().find(|n| w(*n, "body")).ok_or_else(|| ExtractError::corrupt("docx", "document.xml has no <w:body>"))?;
        // A Title paragraph pushes the headings one level down so the outline has a single root.
        c.title_offset = u8::from(body.descendants().any(|p| w(p, "p") && c.role_of(p) == Role::Title));
        c.blocks(body, 0);
        c.flush_code();
        c.write_notes();
        let _ = enc;
        Ok(())
    })?;

    let f = c.b.finish();
    if f.markdown.trim().is_empty() {
        return Err(ExtractError::convert("docx", "the document holds no text"));
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
        warnings: c.warnings,
        tool: tool_string("docx"),
        encoding: Some("utf-8".to_string()),
    })
}

/// The main document part from `_rels/.rels`, defaulting to `word/document.xml`.
fn main_part(zip: &mut ZipDoc) -> Result<String> {
    if let Some(rels) = zip.read("_rels/.rels")? {
        let found = xmlutil::with_document(&rels, "docx", |d, _| {
            Ok(d.descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "Relationship")
                .find(|n| attr(*n, "Type").is_some_and(|t| t.ends_with("/officeDocument")))
                .and_then(|n| attr(n, "Target").map(|t| t.trim_start_matches('/').to_string())))
        })?;
        if let Some(t) = found {
            return Ok(t);
        }
    }
    Ok("word/document.xml".to_string())
}

/// Relationship type suffix (`styles`, `footnotes`, `numbering`...) to target, for the main part.
fn relationships(zip: &mut ZipDoc, main: &str) -> Result<HashMap<String, String>> {
    let (dir, file) = main.rsplit_once('/').map(|(d, f)| (format!("{d}/"), f.to_string())).unwrap_or((String::new(), main.to_string()));
    let Some(data) = zip.read(&format!("{dir}_rels/{file}.rels"))? else { return Ok(HashMap::new()) };
    xmlutil::with_document(&data, "docx", |d, _| {
        let mut out = HashMap::new();
        for r in d.descendants().filter(|n| n.is_element() && n.tag_name().name() == "Relationship") {
            if let (Some(t), Some(target)) = (attr(r, "Type"), attr(r, "Target"))
                && attr(r, "TargetMode") != Some("External") {
                    out.insert(t.rsplit('/').next().unwrap_or(t).to_string(), target.to_string());
                }
        }
        Ok(out)
    })
}

fn resolve_target(dir: &str, target: &str) -> String {
    if let Some(abs) = target.strip_prefix('/') {
        return abs.to_string();
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for c in target.split('/') {
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

fn read_core(root: Node) -> Metadata {
    let mut m = Metadata::default();
    for e in root.children().filter(Node::is_element) {
        let v = text_of(e);
        if v.is_empty() {
            continue;
        }
        match e.tag_name().name() {
            "title" => m.title = Some(v).filter(|t| !t.to_lowercase().starts_with("microsoft word -")),
            "creator" => m.authors = v.split(';').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect(),
            "keywords" => m.keywords = v.split([',', ';']).map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect(),
            "language" => m.language = Some(v),
            "created" => m.year = ids::year_in(&v),
            "description" => m.abstract_text = Some(v),
            _ => {}
        }
    }
    m
}

impl Docx {
    fn read_notes(&mut self, root: Node, kind: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for n in root.children().filter(|c| w(*c, kind)) {
            if matches!(attr(n, "type"), Some("separator" | "continuationSeparator" | "continuationNotice")) {
                continue;
            }
            let Some(id) = attr(n, "id") else { continue };
            let mut parts = Vec::new();
            for p in n.descendants().filter(|d| w(*d, "p")) {
                let c = self.paragraph_content(p, 0);
                let t = collapse_ws(&c.text);
                if !t.is_empty() {
                    parts.push(t);
                }
            }
            out.insert(id.to_string(), parts.join(" "));
        }
        out
    }

    fn write_notes(&mut self) {
        if self.used_notes.is_empty() {
            return;
        }
        self.b.heading(2, "Notes");
        for (n, text) in std::mem::take(&mut self.used_notes) {
            self.b.paragraph(&format!("[^{n}]: {text}"));
        }
    }

    // ------------------------------------------------------------------------------------------------ blocks

    fn blocks(&mut self, parent: Node, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        for n in parent.children().filter(Node::is_element) {
            if w(n, "p") {
                self.paragraph(n);
            } else if w(n, "tbl") {
                self.flush_code();
                self.table(n);
            } else if w(n, "sdt") {
                if let Some(content) = w_child(n, "sdtContent") {
                    self.blocks(content, depth + 1);
                }
            } else if w(n, "customXml") || w(n, "ins") || w(n, "moveTo") {
                self.blocks(n, depth + 1);
            } else if m(n, "oMathPara") {
                self.flush_code();
                if let Some(l) = omml_latex(n, 0) {
                    self.b.display_math(&l);
                }
            }
        }
    }

    fn style_chain(&self, id: &str) -> Vec<&Style> {
        let mut out = Vec::new();
        let mut cur = Some(id.to_string());
        while let Some(i) = cur {
            let Some(s) = self.styles.get(&i) else { break };
            out.push(s);
            if out.len() > 8 {
                break;
            }
            cur = s.based_on.clone();
        }
        out
    }

    fn role_of(&self, p: Node) -> Role {
        let ppr = w_child(p, "pPr");
        let style_id = ppr.and_then(|pp| w_val(pp, "pStyle")).unwrap_or("");
        let chain = self.style_chain(style_id);
        let names: Vec<&str> = chain.iter().map(|s| s.name.as_str()).collect();
        let id_lower = style_id.to_lowercase();
        for n in names.iter().copied().chain(std::iter::once(id_lower.as_str())) {
            let t = n.trim();
            if t == "title" {
                return Role::Title;
            }
            if let Some(rest) = t.strip_prefix("heading")
                && let Ok(l) = rest.trim().parse::<u8>()
                    && (1..=9).contains(&l) {
                        return Role::Heading(l);
                    }
        }
        // Outline level on the paragraph or its style.
        let outline = ppr.and_then(|pp| w_val(pp, "outlineLvl")).and_then(|v| v.parse::<u8>().ok()).filter(|l| *l < 9).or_else(|| chain.iter().find_map(|s| s.outline));
        if let Some(l) = outline {
            return Role::Heading(l + 1);
        }
        let first = names.first().copied().unwrap_or("");
        if first == "quote" || first == "intense quote" || first == "block text" || first == "block quote" {
            return Role::Quote;
        }
        if names.iter().any(|n| n.contains("code") || n.contains("preformatted") || n.contains("verbatim") || n.contains("source")) && !names.iter().any(|n| n.contains("source of") || n.contains("bibliography")) {
            return Role::Code;
        }
        Role::Normal
    }

    fn numbering_of(&self, p: Node) -> Option<(String, u8)> {
        let ppr = w_child(p, "pPr")?;
        let own = w_child(ppr, "numPr").map(|np| {
            let id = w_val(np, "numId").map(str::to_string);
            let lvl = w_val(np, "ilvl").and_then(|l| l.parse::<u8>().ok());
            (id, lvl)
        });
        let style_id = w_val(ppr, "pStyle").unwrap_or("");
        let from_style = self.style_chain(style_id).iter().find_map(|s| s.num.clone());
        match own {
            Some((Some(id), lvl)) => (id != "0").then(|| (id, lvl.unwrap_or(from_style.map(|s| s.1).unwrap_or(0)))),
            Some((None, Some(lvl))) => from_style.map(|(id, _)| (id, lvl)),
            _ => from_style,
        }
        .filter(|(id, _)| id != "0")
    }

    /// Advance the counters for a numbered paragraph and return its label (`1.`, `a)`, `2.1`) or `None` for bullets.
    fn number_label(&mut self, num_id: &str, ilvl: u8) -> Option<String> {
        let abs = self.numbering.num_to_abs.get(num_id)?.clone();
        let lvl = self.numbering.levels.get(&(abs.clone(), ilvl)).cloned().unwrap_or(Level { fmt: "decimal".into(), text: "%1.".into(), start: 1 });
        let counters = self.counters.entry(num_id.to_string()).or_default();
        let il = ilvl as usize;
        if counters.len() <= il {
            for i in counters.len()..=il {
                let start = self.numbering.levels.get(&(abs.clone(), i as u8)).map(|l| l.start).unwrap_or(1);
                counters.push(start.saturating_sub(1));
            }
        }
        counters[il] += 1;
        counters.truncate(il + 1);
        if lvl.fmt == "bullet" {
            return None;
        }
        let mut label = lvl.text.clone();
        for i in 0..=il {
            let level_fmt = self.numbering.levels.get(&(abs.clone(), i as u8)).map(|l| l.fmt.clone()).unwrap_or_else(|| "decimal".into());
            let counters = self.counters.get(num_id).map(|c| c.as_slice()).unwrap_or(&[]);
            label = label.replace(&format!("%{}", i + 1), &format_number(&level_fmt, counters.get(i).copied().unwrap_or(1)));
        }
        let label = label.trim().to_string();
        if label.is_empty() || label.contains('%') { Some(format_number(&lvl.fmt, self.counters.get(num_id).and_then(|c| c.get(il)).copied().unwrap_or(1)) + ".") } else { Some(label) }
    }

    fn paragraph(&mut self, p: Node) {
        let role = self.role_of(p);
        let c = self.paragraph_content(p, 0);
        let numbering = self.numbering_of(p);
        let code_by_font = c.runs > 0 && c.mono_runs == c.runs && role == Role::Normal && numbering.is_none();
        if role == Role::Code || code_by_font {
            // Keep the raw text, tabs and indentation included.
            let raw: String = c.segments.iter().filter_map(|s| if let Seg::Text(t) = s { Some(t.as_str()) } else { None }).collect();
            self.code.push(raw.replace('\u{a0}', " "));
            return;
        }
        self.flush_code();
        if numbering.is_none() {
            self.current_list = None;
        }
        // A paragraph that is only an equation is a display equation.
        if let Some(l) = c.math_only {
            self.b.display_math(&l);
            return;
        }
        for seg in &c.segments {
            match seg {
                Seg::Display(l) => self.b.display_math(l),
                Seg::Text(t) => {
                    let text = collapse_ws(t);
                    if text.is_empty() {
                        continue;
                    }
                    match (&role, &numbering) {
                        (Role::Title, _) => self.b.heading(1, &text),
                        (Role::Heading(l), num) => {
                            let label = num.as_ref().and_then(|(id, lvl)| self.number_label(id, *lvl));
                            let level = (*l + self.title_offset).min(6);
                            self.b.heading(level, &match label {
                                Some(lab) if !text.starts_with(&lab) => format!("{lab} {text}"),
                                _ => text,
                            });
                        }
                        (_, Some((id, lvl))) => {
                            if self.current_list.as_deref() != Some(id.as_str()) {
                                self.b.break_list();
                            }
                            self.current_list = Some(id.clone());
                            let label = self.number_label(id, *lvl);
                            match label {
                                None => self.b.list_item(*lvl as usize, "-", &text),
                                Some(l) => {
                                    let numeric = l.trim_end_matches(['.', ')']).chars().all(|c| c.is_ascii_digit()) && !l.trim_end_matches(['.', ')']).is_empty() && !l.trim_end_matches(['.', ')']).contains('.');
                                    if numeric {
                                        self.b.list_item(*lvl as usize, &l, &text);
                                    } else {
                                        self.b.list_item(*lvl as usize, "-", &format!("{l} {text}"));
                                    }
                                }
                            }
                        }
                        (Role::Quote, None) => self.b.quote(&text),
                        _ => self.b.paragraph(&text),
                    }
                }
            }
        }
        for t in c.boxes {
            self.b.paragraph(&t);
        }
    }

    fn flush_code(&mut self) {
        if self.code.is_empty() {
            return;
        }
        let code = std::mem::take(&mut self.code).join("\n");
        self.b.code_block("", &code);
    }

    // ----------------------------------------------------------------------------------------- paragraph content

    fn paragraph_content(&mut self, p: Node, depth: usize) -> ParaContent {
        let mut c = ParaContent::default();
        let mut cur = String::new();
        let mut maths = 0usize;
        let mut display_in_text = 0usize;
        self.runs(p, &mut c, &mut cur, depth, &mut maths, &mut display_in_text);
        if !cur.is_empty() {
            c.segments.push(Seg::Text(std::mem::take(&mut cur)));
        }
        c.text = c.segments.iter().filter_map(|s| if let Seg::Text(t) = s { Some(t.as_str()) } else { None }).collect::<Vec<_>>().join(" ");
        // A paragraph holding one equation and no other text is a display equation.
        if maths == 1 && display_in_text == 0 && c.segments.len() == 1
            && let Seg::Text(t) = &c.segments[0]
                && let Some(inner) = t.trim().strip_prefix('$').and_then(|x| x.strip_suffix('$'))
                    && !inner.contains('$') {
                        c.math_only = Some(inner.to_string());
                    }
        c
    }

    fn runs(&mut self, parent: Node, c: &mut ParaContent, cur: &mut String, depth: usize, maths: &mut usize, display_in_text: &mut usize) {
        if depth > MAX_DEPTH {
            return;
        }
        for n in parent.children().filter(Node::is_element) {
            if w(n, "r") {
                c.runs += 1;
                if run_is_monospace(n) {
                    c.mono_runs += 1;
                }
                self.run(n, c, cur, depth);
            } else if w(n, "hyperlink") || w(n, "smartTag") || w(n, "ins") || w(n, "moveTo") || w(n, "fldSimple") || w(n, "customXml") || w(n, "bdo") || w(n, "dir") {
                self.runs(n, c, cur, depth + 1, maths, display_in_text);
            } else if w(n, "sdt") {
                if let Some(content) = w_child(n, "sdtContent") {
                    self.runs(content, c, cur, depth + 1, maths, display_in_text);
                }
            } else if m(n, "oMathPara") {
                if let Some(l) = omml_latex(n, 0) {
                    if !cur.is_empty() {
                        c.segments.push(Seg::Text(std::mem::take(cur)));
                    }
                    c.segments.push(Seg::Display(l));
                    *display_in_text += 1;
                }
            } else if m(n, "oMath") {
                if let Some(l) = omml_latex(n, 0) {
                    cur.push('$');
                    cur.push_str(&l);
                    cur.push('$');
                    *maths += 1;
                }
            } else if n.tag_name().name() == "AlternateContent" {
                // The Choice branch holds the modern markup; the Fallback repeats it for old readers.
                let branch = n.children().find(|b| b.is_element() && b.tag_name().name() == "Choice").or_else(|| n.children().find(|b| b.is_element() && b.tag_name().name() == "Fallback"));
                if let Some(b) = branch {
                    self.runs(b, c, cur, depth + 1, maths, display_in_text);
                }
            }
        }
    }

    fn run(&mut self, r: Node, c: &mut ParaContent, cur: &mut String, depth: usize) {
        for n in r.children().filter(Node::is_element) {
            if !ns_is_w(n) && n.tag_name().name() != "AlternateContent"
                && m(n, "oMath") {
                    continue;
                }
                // Drawing and picture containers (wps, v, mc) may hold text boxes.
            match n.tag_name().name() {
                "t" if ns_is_w(n) => cur.push_str(&xmlutil::raw_text(n)),
                "tab" | "ptab" if ns_is_w(n) => cur.push('\t'),
                "br" | "cr" if ns_is_w(n) => {
                    if attr(n, "type") != Some("page") {
                        cur.push('\n');
                    }
                }
                "noBreakHyphen" if ns_is_w(n) => cur.push('-'),
                "softHyphen" if ns_is_w(n) => {}
                "sym" if ns_is_w(n) => {
                    if let Some(ch) = attr(n, "char").and_then(|h| u32::from_str_radix(h.trim_start_matches("F0"), 16).ok()).and_then(char::from_u32) {
                        // Symbol-font code points are private use; keep only printable ASCII-range stand-ins.
                        if ch.is_ascii_graphic() {
                            cur.push(ch);
                        }
                    }
                }
                "footnoteReference" if ns_is_w(n) => {
                    if let Some(id) = attr(n, "id")
                        && let Some(text) = self.footnotes.get(id).cloned() {
                            let num = self.used_notes.len() + 1;
                            self.used_notes.push((num, text));
                            cur.push_str(&format!("[^{num}]"));
                        }
                }
                "endnoteReference" if ns_is_w(n) => {
                    if let Some(id) = attr(n, "id")
                        && let Some(text) = self.endnotes.get(id).cloned() {
                            let num = self.used_notes.len() + 1;
                            self.used_notes.push((num, text));
                            cur.push_str(&format!("[^{num}]"));
                        }
                }
                "txbxContent" => {
                    for p in n.children().filter(|x| w(*x, "p")) {
                        let inner = self.paragraph_content(p, depth + 1);
                        let t = collapse_ws(&inner.text);
                        if !t.is_empty() {
                            c.boxes.push(t);
                        }
                    }
                }
                "AlternateContent" => {
                    let branch = n.children().find(|b| b.is_element() && b.tag_name().name() == "Choice").or_else(|| n.children().find(|b| b.is_element() && b.tag_name().name() == "Fallback"));
                    if let Some(b) = branch {
                        self.run(b, c, cur, depth + 1);
                    }
                }
                "drawing" | "pict" | "object" | "Choice" | "Fallback" | "inline" | "anchor" | "graphic" | "graphicData" | "wsp" | "txbx" | "textbox" | "sdtContent"
                    if depth < MAX_DEPTH => {
                        self.run(n, c, cur, depth + 1);
                    }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------------------------------------------- tables

    fn table(&mut self, tbl: Node) {
        let mut g = Grid::default();
        let mut header_rows = 0usize;
        let mut leading_header = true;
        // Column index -> (row, cell) of the cell a vertical merge continues.
        let mut open: HashMap<usize, (usize, usize)> = HashMap::new();
        for tr in tbl.children().filter(|c| w(*c, "tr")) {
            let is_header = w_child(tr, "trPr").is_some_and(|pr| w_child(pr, "tblHeader").is_some());
            if is_header && leading_header {
                header_rows += 1;
            } else {
                leading_header = false;
            }
            let mut row: Vec<Cell> = Vec::new();
            let row_idx = g.rows.len();
            let mut col = 0usize;
            for tc in tr.children().filter(|c| w(*c, "tc")) {
                let pr = w_child(tc, "tcPr");
                let span = pr.and_then(|p| w_val(p, "gridSpan")).and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
                let vmerge = pr.and_then(|p| w_child(p, "vMerge")).map(|v| attr(v, "val").unwrap_or("continue").to_string());
                match vmerge.as_deref() {
                    Some("continue") => {
                        if let Some(&(r, ci)) = open.get(&col)
                            && let Some(cell) = g.rows.get_mut(r).and_then(|rw| rw.get_mut(ci)) {
                                cell.rowspan += 1;
                            }
                    }
                    _ => {
                        let mut parts = Vec::new();
                        for p in tc.descendants().filter(|d| w(*d, "p")) {
                            let t = collapse_ws(&self.paragraph_content(p, 0).text);
                            if !t.is_empty() {
                                parts.push(t);
                            }
                        }
                        row.push(Cell::new(parts.join(" "), span, 1));
                        if vmerge.as_deref() == Some("restart") {
                            open.insert(col, (row_idx, row.len() - 1));
                        } else {
                            open.remove(&col);
                        }
                    }
                }
                col += span;
            }
            g.rows.push(row);
        }
        g.header_rows = header_rows;
        g.emit(&mut self.b);
    }
}

fn run_is_monospace(r: Node) -> bool {
    let Some(rpr) = w_child(r, "rPr") else { return false };
    let Some(f) = w_child(rpr, "rFonts") else { return false };
    let font = attr(f, "ascii").or_else(|| attr(f, "hAnsi")).unwrap_or("").to_lowercase();
    ["courier", "consolas", "monaco", "menlo", "lucida console", "source code", "fira code", "dejavu sans mono", "cascadia", "monospace", "liberation mono", "andale mono"].iter().any(|m| font.contains(m))
}

// ---------------------------------------------------------------------------------------------------- OMML

/// LaTeX for an `m:oMath` or `m:oMathPara` element.
fn omml_latex(n: Node, depth: usize) -> Option<String> {
    let s = omml(n, depth);
    let s = collapse_ws(&s);
    (!s.is_empty()).then_some(s)
}

fn mchild_latex(n: Node, name: &str, depth: usize) -> String {
    m_child(n, name).map(|c| omml(c, depth + 1)).unwrap_or_default()
}

fn m_prop<'a>(n: Node<'a, '_>, props: &str, key: &str) -> Option<&'a str> {
    m_child(n, props).and_then(|p| m_child(p, key)).and_then(|k| attr(k, "val"))
}

fn flag(v: Option<&str>) -> bool {
    matches!(v, Some("1" | "on" | "true"))
}

fn omml(n: Node, depth: usize) -> String {
    if depth > 40 {
        return mathml::escape_text(&text_of(n));
    }
    let d = depth + 1;
    let kids = |n: Node| -> String {
        n.children().filter(|c| c.is_element() && ns_is_m(*c)).map(|c| omml(c, d)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
    };
    let name = n.tag_name().name();
    match name {
        "r" => {
            let text: String = n.children().filter(|c| m(*c, "t") || w(*c, "t")).map(xmlutil::raw_text).collect();
            let plain = m_prop(n, "rPr", "sty") == Some("p") || flag(m_prop(n, "rPr", "nor"));
            let literal = flag(m_prop(n, "rPr", "lit")) || flag(m_prop(n, "rPr", "nor"));
            if literal {
                format!("\\text{{{}}}", mathml::escape_text(text.trim()))
            } else {
                let l = mathml::latex_text(&text);
                if plain && l.chars().all(|c| c.is_ascii_alphabetic()) && l.chars().count() > 1 { format!("\\mathrm{{{l}}}") } else { l }
            }
        }
        "f" => {
            let num = mchild_latex(n, "num", d);
            let den = mchild_latex(n, "den", d);
            match m_prop(n, "fPr", "type") {
                Some("noBar") => format!("\\binom{{{num}}}{{{den}}}"),
                Some("lin") => format!("{num} / {den}"),
                _ => format!("\\frac{{{num}}}{{{den}}}"),
            }
        }
        "rad" => {
            let e = mchild_latex(n, "e", d);
            let deg = mchild_latex(n, "deg", d);
            if deg.is_empty() || flag(m_prop(n, "radPr", "degHide")) { format!("\\sqrt{{{e}}}") } else { format!("\\sqrt[{deg}]{{{e}}}") }
        }
        "sSup" => format!("{}^{}", script_base(n, d), mathml::group(&mchild_latex(n, "sup", d))),
        "sSub" => format!("{}_{}", script_base(n, d), mathml::group(&mchild_latex(n, "sub", d))),
        "sSubSup" => format!("{}_{}^{}", script_base(n, d), mathml::group(&mchild_latex(n, "sub", d)), mathml::group(&mchild_latex(n, "sup", d))),
        "sPre" => {
            let sub = mchild_latex(n, "sub", d);
            let sup = mchild_latex(n, "sup", d);
            format!("{{}}_{}^{} {}", mathml::group(&sub), mathml::group(&sup), mchild_latex(n, "e", d))
        }
        "nary" => {
            let chr = m_prop(n, "naryPr", "chr").unwrap_or("∫");
            let sym = chr.chars().next().and_then(mathml::symbol).map(str::to_string).unwrap_or_else(|| mathml::op(chr));
            let sub = if flag(m_prop(n, "naryPr", "subHide")) { String::new() } else { mchild_latex(n, "sub", d) };
            let sup = if flag(m_prop(n, "naryPr", "supHide")) { String::new() } else { mchild_latex(n, "sup", d) };
            let mut s = sym;
            if !sub.is_empty() {
                s.push_str(&format!("_{}", mathml::group(&sub)));
            }
            if !sup.is_empty() {
                s.push_str(&format!("^{}", mathml::group(&sup)));
            }
            format!("{s} {}", mchild_latex(n, "e", d))
        }
        "d" => {
            let beg = m_prop(n, "dPr", "begChr").unwrap_or("(");
            let end = m_prop(n, "dPr", "endChr").unwrap_or(")");
            let sep = m_prop(n, "dPr", "sepChr").unwrap_or("|");
            let inner = n.children().filter(|c| m(*c, "e")).map(|e| omml(e, d)).collect::<Vec<_>>().join(&format!(" {} ", mathml::op(sep)));
            let delim = |s: &str| if s.is_empty() { ".".to_string() } else { mathml::op(s) };
            format!("\\left{} {inner} \\right{}", delim(beg), delim(end))
        }
        "func" => {
            let name = collapse_ws(&mchild_latex(n, "fName", d));
            let bare = name.trim_start_matches("\\mathrm{").trim_end_matches('}');
            let f = if matches!(bare, "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "sinh" | "cosh" | "tanh" | "log" | "ln" | "lg" | "exp" | "lim" | "max" | "min" | "det" | "dim" | "ker" | "arg" | "sup" | "inf") {
                format!("\\{bare}")
            } else {
                name
            };
            format!("{f} {}", mchild_latex(n, "e", d))
        }
        "acc" => {
            let chr = m_prop(n, "accPr", "chr").unwrap_or("\u{302}");
            let e = mchild_latex(n, "e", d);
            match mathml::accent_command(chr, e.chars().count() == 1) {
                Some(cmd) => format!("{cmd}{{{e}}}"),
                None => e,
            }
        }
        "bar" => {
            let e = mchild_latex(n, "e", d);
            if m_prop(n, "barPr", "pos") == Some("bot") { format!("\\underline{{{e}}}") } else { format!("\\overline{{{e}}}") }
        }
        "limLow" => {
            let e = mchild_latex(n, "e", d);
            let lim = mchild_latex(n, "lim", d);
            if matches!(e.trim_start_matches("\\mathrm{").trim_end_matches('}'), "lim" | "max" | "min" | "sup" | "inf") {
                format!("\\{}_{}", e.trim_start_matches("\\mathrm{").trim_end_matches('}'), mathml::group(&lim))
            } else {
                format!("\\underset{{{lim}}}{{{e}}}")
            }
        }
        "limUpp" => format!("\\overset{{{}}}{{{}}}", mchild_latex(n, "lim", d), mchild_latex(n, "e", d)),
        "m" => {
            let rows: Vec<String> = n.children().filter(|c| m(*c, "mr")).map(|r| r.children().filter(|e| m(*e, "e")).map(|e| omml(e, d)).collect::<Vec<_>>().join(" & ")).collect();
            format!("\\begin{{matrix}} {} \\end{{matrix}}", rows.join(" \\\\ "))
        }
        "eqArr" => {
            let rows: Vec<String> = n.children().filter(|c| m(*c, "e")).map(|e| omml(e, d)).collect();
            format!("\\begin{{aligned}} {} \\end{{aligned}}", rows.join(" \\\\ "))
        }
        "rPr" | "ctrlPr" | "fPr" | "radPr" | "naryPr" | "dPr" | "accPr" | "barPr" | "limLowPr" | "limUppPr" | "mPr" | "eqArrPr" | "sSupPr" | "sSubPr" | "sSubSupPr" | "sPrePr" | "funcPr" | "boxPr" | "borderBoxPr" | "phantPr" | "groupChrPr" | "oMathParaPr" | "t" => String::new(),
        _ => kids(n),
    }
}

fn script_base(n: Node, d: usize) -> String {
    let e = mchild_latex(n, "e", d);
    let nested = m_child(n, "e").is_some_and(|e| e.children().any(|c| m(c, "sSub") || m(c, "sSup") || m(c, "sSubSup")));
    if nested || (e.contains(' ') && !e.ends_with(')') && !e.ends_with("\\right)")) { format!("{{{e}}}") } else { e }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;
    use std::io::{Cursor, Write};

    const W: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape""#;

    fn docx(document: &str, extra: &[(String, String)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut put = |n: &str, c: &str| {
            w.start_file(n, o).unwrap();
            w.write_all(c.as_bytes()).unwrap();
        };
        put("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>");
        put("word/document.xml", &format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document {W}><w:body>{document}</w:body></w:document>"));
        for (n, c) in extra {
            put(n.as_str(), c.as_str());
        }
        w.finish().unwrap().into_inner()
    }

    fn run(document: &str, extra: &[(String, String)]) -> Extracted {
        convert(&docx(document, extra), &ConvertOptions::default()).unwrap()
    }

    fn p(style: &str, text: &str) -> String {
        let st = if style.is_empty() { String::new() } else { format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>") };
        format!("<w:p>{st}<w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>")
    }

    fn styles() -> (String, String) {
        let s = format!(
            r#"<w:styles {W}>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Heading1"/><w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Odd"><w:name w:val="My Chapter"/><w:pPr><w:outlineLvl w:val="2"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Quote"><w:name w:val="Quote"/></w:style>
<w:style w:type="paragraph" w:styleId="SourceCode"><w:name w:val="Source Code"/></w:style>
</w:styles>"#
        );
        ("word/styles.xml".to_string(), s)
    }

    fn numbering() -> (String, String) {
        let s = format!(
            r#"<w:numbering {W}>
<w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1"/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1.%2"/></w:lvl></w:abstractNum>
<w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="bullet"/><w:lvlText w:val="o"/></w:lvl></w:abstractNum>
<w:abstractNum w:abstractNumId="2"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="(%2)"/></w:lvl></w:abstractNum>
<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num><w:num w:numId="3"><w:abstractNumId w:val="2"/></w:num>
</w:numbering>"#
        );
        ("word/numbering.xml".to_string(), s)
    }

    fn li(num: u32, lvl: u32, text: &str) -> String {
        format!("<w:p><w:pPr><w:pStyle w:val=\"ListParagraph\"/><w:numPr><w:ilvl w:val=\"{lvl}\"/><w:numId w:val=\"{num}\"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>")
    }

    #[test]
    fn headings_titles_numbering_and_outline_levels() {
        let doc = format!("{}{}{}{}{}{}", p("Title", "The Document"), p("Heading1", "Introduction"), p("Heading2", "Background"), p("Heading1", "Methods"), p("Odd", "Odd chapter"), p("", "Plain paragraph."));
        let r = run(&doc, &[styles(), numbering()]);
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(hs, vec![(1, "The Document"), (2, "1 Introduction"), (3, "1.1 Background"), (2, "2 Methods"), (4, "Odd chapter")]);
        assert!(r.markdown.ends_with("Plain paragraph.\n"));
        let lines: Vec<&str> = r.markdown.lines().collect();
        for h in &r.headings {
            assert_eq!(lines[h.line as usize - 1], format!("{} {}", "#".repeat(h.level as usize), h.text));
        }
        assert_eq!(r.metadata.title.as_deref(), Some("The Document"));
    }

    #[test]
    fn lists_bullets_and_numbers() {
        let doc = format!("{}{}{}{}{}{}", li(2, 0, "bullet one"), li(2, 1, "sub bullet"), li(3, 0, "first"), li(3, 1, "child a"), li(3, 1, "child b"), li(3, 0, "second"));
        let r = run(&doc, &[styles(), numbering()]);
        assert_eq!(r.markdown, "- bullet one\n  - sub bullet\n\n1. first\n  - (a) child a\n  - (b) child b\n2. second\n");
    }

    #[test]
    fn runs_tabs_breaks_and_hyperlinks() {
        let doc = r#"<w:p><w:r><w:t>Alpha</w:t></w:r><w:r><w:tab/><w:t>beta</w:t></w:r><w:r><w:br/><w:t>gamma</w:t></w:r><w:hyperlink r:id="rId5" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:r><w:t xml:space="preserve"> link text</w:t></w:r></w:hyperlink><w:del><w:r><w:delText>deleted</w:delText></w:r></w:del><w:ins><w:r><w:t> inserted</w:t></w:r></w:ins><w:r><w:instrText>HYPERLINK "x"</w:instrText></w:r></w:p>"#;
        let r = run(doc, &[]);
        assert_eq!(r.markdown, "Alpha beta gamma link text inserted\n");
    }

    #[test]
    fn tables_with_merges_and_headers() {
        let doc = r#"<w:tbl>
<w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>Scores</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:p><w:r><w:t>A | B</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p><w:p><w:r><w:t>extra</w:t></w:r></w:p></w:tc></w:tr>
</w:tbl>"#;
        let r = run(doc, &[]);
        assert_eq!(r.markdown, "| Name | Scores |  |\n| --- | --- | --- |\n| A \\| B | 1 | 2 extra |\n");
        assert_eq!(r.blocks[0].kind, BlockKind::Table);
        // A vertical merge in the body becomes an HTML table that keeps the rowspan.
        let doc = r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>y</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>m</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#;
        let r = run(doc, &[]);
        assert!(r.markdown.contains("<tr><td rowspan=\"2\">m</td><td>1</td></tr>"), "{}", r.markdown);
    }

    #[test]
    fn footnotes_become_numbered_notes() {
        let doc = r#"<w:p><w:r><w:t>Claim.</w:t></w:r><w:r><w:footnoteReference w:id="2"/></w:r><w:r><w:t xml:space="preserve"> More.</w:t></w:r><w:r><w:footnoteReference w:id="3"/></w:r></w:p>"#;
        let notes = format!(
            r#"<w:footnotes {W}><w:footnote w:type="separator" w:id="0"><w:p><w:r><w:separator/></w:r></w:p></w:footnote><w:footnote w:id="2"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:t xml:space="preserve"> Source one.</w:t></w:r></w:p></w:footnote><w:footnote w:id="3"><w:p><w:r><w:t>Source two.</w:t></w:r></w:p></w:footnote></w:footnotes>"#
        );
        let r = run(doc, &[("word/footnotes.xml".to_string(), notes)]);
        assert_eq!(r.markdown, "Claim.[^1] More.[^2]\n\n## Notes\n\n[^1]: Source one.\n\n[^2]: Source two.\n");
    }

    #[test]
    fn quote_and_code_styles_and_monospace_runs() {
        let mono = |t: &str| format!("<w:p><w:r><w:rPr><w:rFonts w:ascii=\"Consolas\"/></w:rPr><w:t xml:space=\"preserve\">{t}</w:t></w:r></w:p>");
        let doc = format!("{}{}{}{}{}{}", p("Quote", "Quoted words."), p("SourceCode", "fn main() {"), p("SourceCode", "    run();"), p("SourceCode", "}"), p("", "After."), mono("x = 1"));
        let r = run(&doc, &[styles()]);
        assert_eq!(r.markdown, "> Quoted words.\n\n```\nfn main() {\n    run();\n}\n```\n\nAfter.\n\n```\nx = 1\n```\n");
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Code).count(), 2);
    }

    #[test]
    fn office_math_becomes_latex() {
        let doc = r#"<w:p><w:r><w:t xml:space="preserve">Inline </w:t></w:r><m:oMath><m:sSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath><w:r><w:t xml:space="preserve"> done.</w:t></w:r></w:p>
<w:p><m:oMathPara><m:oMath><m:f><m:num><m:r><m:t>a+b</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f><m:r><m:t>=</m:t></m:r><m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr><m:sub><m:r><m:t>i=1</m:t></m:r></m:sub><m:sup><m:r><m:t>n</m:t></m:r></m:sup><m:e><m:sSub><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sub><m:r><m:t>i</m:t></m:r></m:sub></m:sSub></m:e></m:nary></m:oMath></m:oMathPara></w:p>
<w:p><m:oMath><m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/><m:e><m:r><m:t>α</m:t></m:r></m:e></m:rad></m:oMath></w:p>"#.to_string();
        let r = run(&doc, &[]);
        assert!(r.markdown.contains("Inline $x^2$ done."), "{}", r.markdown);
        assert!(r.markdown.contains("$$\n\\frac{a+b}{2} = \\sum_{i=1}^n x_i\n$$"), "{}", r.markdown);
        assert!(r.markdown.contains("$$\n\\sqrt{\\alpha}\n$$"), "{}", r.markdown);
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Equation).count(), 2);
    }

    #[test]
    fn text_boxes_and_alternate_content_are_not_duplicated() {
        let doc = r#"<w:p><w:r><w:t>Body.</w:t></w:r><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>Box text</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></w:drawing></mc:Choice><mc:Fallback><w:pict><w:txbxContent><w:p><w:r><w:t>Box text</w:t></w:r></w:p></w:txbxContent></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>"#;
        let r = run(doc, &[]);
        assert_eq!(r.markdown.matches("Box text").count(), 1, "{}", r.markdown);
        assert!(r.markdown.starts_with("Body."));
    }

    #[test]
    fn core_properties_supply_metadata() {
        let core = r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/"><dc:title>Core Title</dc:title><dc:creator>Ann Lee; Bo Chen</dc:creator><cp:keywords>a, b</cp:keywords><dc:language>en-US</dc:language><dcterms:created>2019-04-02T10:00:00Z</dcterms:created></cp:coreProperties>"#;
        let r = run(&p("", "Body text."), &[("docProps/core.xml".to_string(), core.to_string())]);
        assert_eq!(r.metadata.title.as_deref(), Some("Core Title"));
        assert_eq!(r.metadata.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(r.metadata.keywords, vec!["a", "b"]);
        assert_eq!(r.metadata.language.as_deref(), Some("en-US"));
        assert_eq!(r.metadata.year, Some(2019));
    }

    #[test]
    fn format_number_styles() {
        assert_eq!(format_number("decimal", 3), "3");
        assert_eq!(format_number("lowerLetter", 28), "ab");
        assert_eq!(format_number("upperLetter", 3), "C");
        assert_eq!(format_number("lowerRoman", 14), "xiv");
        assert_eq!(format_number("upperRoman", 9), "IX");
        assert_eq!(format_number("none", 1), "");
    }

    #[test]
    fn broken_docx_files() {
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
        assert_eq!(convert(b"PK not a zip", &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"/>").unwrap();
        let z = w.finish().unwrap().into_inner();
        assert_eq!(convert(&z, &ConvertOptions::default()).unwrap_err().reason_code(), "corrupt_file");
        let empty = docx("<w:p/>", &[]);
        assert_eq!(convert(&empty, &ConvertOptions::default()).unwrap_err().reason_code(), "extraction_failed");
    }
}

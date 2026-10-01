//! TEI to canonical-ready Markdown: GROBID's header and body output, and general TEI texts (books, verse, plays,
//! `<pb>` page breaks). GROBID's TEI uses the same shapes, so its output goes through this converter.

use roxmltree::Node;

use crate::ConvertOptions;
use crate::builder::{MdBuilder, collapse_ws};
use crate::error::{ExtractError, Result};
use crate::ids;
use crate::jats::{formula_latex, script};
use crate::tables::{Cell, Grid};
use crate::types::{Extracted, Metadata, Reference, tool_string};
use crate::xmlutil::{self, attr, child, children_named, descendant, is, name, text_of};

pub fn convert(bytes: &[u8], _opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    xmlutil::with_document(bytes, "tei", |doc, enc| {
        let root = doc.root_element();
        let tei = match name(root) {
            "TEI" | "tei" => root,
            "teiCorpus" => child(root, "TEI").ok_or_else(|| ExtractError::convert("tei", "teiCorpus holds no <TEI>"))?,
            other => return Err(ExtractError::convert("tei", format!("unexpected root element <{other}>"))),
        };
        let mut c = Tei::default();
        c.run(tei);
        Ok(c.finish(enc))
    })
}

#[derive(Default)]
struct Tei {
    b: MdBuilder,
    meta: Metadata,
    refs: Vec<Reference>,
    warnings: Vec<String>,
    title_emitted: bool,
    /// Notes found inside the current paragraph, written after it.
    pending_notes: Vec<String>,
}

impl Tei {
    fn finish(self, encoding: &str) -> Extracted {
        let f = self.b.finish();
        Extracted {
            markdown: f.markdown,
            headings: f.headings,
            pages: f.pages,
            blocks: f.blocks,
            metadata: self.meta,
            references: self.refs,
            warnings: self.warnings,
            tool: tool_string("tei"),
            encoding: Some(encoding.to_string()),
        }
    }

    fn run(&mut self, tei: Node) {
        self.meta.language = attr(tei, "lang").map(str::to_string).filter(|l| !l.is_empty());
        if let Some(h) = child(tei, "teiHeader") {
            self.header(h);
        }
        let text = child(tei, "text");
        if self.meta.language.is_none() {
            self.meta.language = text.and_then(|t| attr(t, "lang")).map(str::to_string).filter(|l| !l.is_empty());
        }
        if let Some(title) = self.meta.title.clone() {
            self.b.heading(1, &title);
            self.title_emitted = true;
            if !self.meta.authors.is_empty() {
                let line = self.meta.authors.join(", ");
                self.b.paragraph(&line);
            }
        }
        if let Some(h) = child(tei, "teiHeader") {
            self.abstract_and_keywords(h);
        }
        if let Some(text) = text {
            for k in text.children().filter(Node::is_element) {
                match name(k) {
                    "front" => self.container(k, 1),
                    "body" => self.container(k, 1),
                    "back" => self.container(k, 1),
                    "group" => {
                        for t in k.children().filter(|c| is(*c, "text")) {
                            for part in t.children().filter(Node::is_element) {
                                self.container(part, 1);
                            }
                        }
                    }
                    _ => self.block(k, 1),
                }
            }
        }
    }

    // ----------------------------------------------------------------------------------------------- header

    fn header(&mut self, h: Node) {
        let file_desc = child(h, "fileDesc");
        let title_stmt = file_desc.and_then(|f| child(f, "titleStmt"));
        let bibl = file_desc.and_then(|f| child(f, "sourceDesc")).and_then(|s| child(s, "biblStruct"));
        let analytic = bibl.and_then(|b| child(b, "analytic"));
        let monogr = bibl.and_then(|b| child(b, "monogr"));

        // Title: the main title of the header, else the analytic title, else the monograph title.
        let pick_title = |scope: Option<Node>| -> Option<String> {
            let scope = scope?;
            let titles: Vec<Node> = children_named(scope, "title").collect();
            titles
                .iter()
                .find(|t| attr(**t, "type") == Some("main"))
                .or_else(|| titles.iter().find(|t| matches!(attr(**t, "level"), Some("a") | Some("m"))))
                .or_else(|| titles.first())
                .map(|t| text_of(*t))
                .filter(|t| !t.is_empty())
        };
        self.meta.title = pick_title(title_stmt).or_else(|| pick_title(analytic)).or_else(|| pick_title(monogr));

        for s in [analytic, title_stmt, monogr].into_iter().flatten() {
            let authors: Vec<String> = children_named(s, "author").filter_map(|a| person_name(a)).collect();
            if !authors.is_empty() {
                self.meta.authors = authors;
                break;
            }
        }
        if let Some(b) = bibl {
            for id in b.descendants().filter(|d| is(*d, "idno")) {
                self.take_idno(id);
            }
        }
        if let Some(pub_stmt) = file_desc.and_then(|f| child(f, "publicationStmt")) {
            for id in pub_stmt.descendants().filter(|d| is(*d, "idno")) {
                self.take_idno(id);
            }
        }
        // Publication year.
        let date_in = |scope: Option<Node>| -> Option<i32> {
            let scope = scope?;
            scope
                .descendants()
                .filter(|d| is(*d, "date"))
                .find_map(|d| attr(d, "when").and_then(ids::year_in).or_else(|| ids::year_in(&text_of(d))))
        };
        self.meta.year = date_in(monogr).or_else(|| date_in(file_desc.and_then(|f| child(f, "publicationStmt")))).or_else(|| date_in(bibl));
        self.meta.journal = monogr
            .and_then(|m| children_named(m, "title").find(|t| attr(*t, "level") == Some("j")).or_else(|| child(m, "title")))
            .map(text_of)
            .filter(|t| !t.is_empty() && Some(t) != self.meta.title.as_ref());
        let profile = child(h, "profileDesc");
        if let Some(l) = profile.and_then(|p| child(p, "langUsage")).and_then(|l| child(l, "language")).and_then(|l| attr(l, "ident"))
            && self.meta.language.is_none() {
                self.meta.language = Some(l.to_string());
            }
        if let Some(tc) = profile.and_then(|p| child(p, "textClass")) {
            for kw in tc.descendants().filter(|d| is(*d, "term")) {
                let t = text_of(kw);
                if !t.is_empty() && !self.meta.keywords.contains(&t) {
                    self.meta.keywords.push(t);
                }
            }
        }
        if let Some(abs) = profile.and_then(|p| child(p, "abstract")) {
            let t: Vec<String> = abs.descendants().filter(|d| is(*d, "p")).map(|p| self.inline_text(p)).filter(|t| !t.is_empty()).collect();
            let joined = if t.is_empty() { text_of(abs) } else { t.join(" ") };
            if !joined.is_empty() {
                self.meta.abstract_text = Some(joined);
            }
        }
    }

    fn take_idno(&mut self, id: Node) {
        let v = text_of(id);
        match attr(id, "type").unwrap_or("").to_ascii_lowercase().as_str() {
            "doi" => self.meta.doi = self.meta.doi.take().or_else(|| ids::find_doi(&v)),
            "arxiv" => self.meta.arxiv_id = self.meta.arxiv_id.take().or_else(|| ids::bare_arxiv(&v).or_else(|| ids::find_arxiv(&v))),
            "pmc" | "pmcid" => self.meta.pmcid = self.meta.pmcid.take().or_else(|| ids::normalize_pmcid(&v)),
            "isbn" | "isbn-10" | "isbn-13" | "isbn10" | "isbn13" => {
                if let Some(i) = ids::normalize_isbn(&v)
                    && !self.meta.isbn.contains(&i) {
                        self.meta.isbn.push(i);
                    }
            }
            _ => {}
        }
    }

    fn abstract_and_keywords(&mut self, h: Node) {
        let profile = child(h, "profileDesc");
        if let Some(abs) = profile.and_then(|p| child(p, "abstract")) {
            self.b.heading(2, "Abstract");
            let paras: Vec<Node> = abs.descendants().filter(|d| is(*d, "p")).collect();
            if paras.is_empty() {
                let t = self.inline_text(abs);
                self.b.paragraph(&t);
            }
            for p in paras {
                self.paragraph(p);
            }
        }
        if !self.meta.keywords.is_empty() {
            let line = format!("Keywords: {}", self.meta.keywords.join("; "));
            self.b.paragraph(&line);
        }
    }

    // ----------------------------------------------------------------------------------------------- body

    fn container(&mut self, n: Node, depth: usize) {
        for k in n.children().filter(Node::is_element) {
            self.block(k, depth);
        }
    }

    fn block(&mut self, n: Node, depth: usize) {
        match name(n) {
            "div" | "div1" | "div2" | "div3" | "div4" | "div5" | "div6" | "div7" => self.div(n, depth),
            "head" => {
                // A heading outside a div: treat as a section heading at the current depth.
                let t = self.inline_text(n);
                self.b.heading((depth + 1) as u8, &t);
            }
            "p" | "ab" | "trailer" | "byline" | "dateline" | "salute" | "signed" | "docDate" | "docAuthor" | "docEdition" | "docImprint" | "epigraph" | "argument" | "opener" | "closer" | "postscript" | "note" | "bibl" => {
                if matches!(name(n), "epigraph" | "argument" | "opener" | "closer" | "postscript") && n.children().any(|c| c.is_element() && matches!(name(c), "p" | "quote" | "lg" | "l" | "bibl" | "cit" | "signed" | "dateline" | "salute")) {
                    self.container(n, depth);
                } else if name(n) == "note" {
                    let t = self.inline_text(n);
                    if !t.is_empty() {
                        self.b.paragraph(&format!("[note] {t}"));
                    }
                } else {
                    self.paragraph(n);
                }
            }
            "pb" => self.page_break(n),
            "lg" => self.verse(n),
            "l" => {
                let t = self.inline_text(n);
                self.b.paragraph(&t);
            }
            "list" | "listWit" | "listPerson" | "listPlace" | "listEvent" => self.list(n, 0),
            "listBibl" => self.list_bibl(n),
            "biblStruct" => self.bibl_struct(n),
            "quote" | "cit" | "q" => {
                for k in n.children().filter(Node::is_element) {
                    if matches!(name(k), "p" | "l" | "lg" | "quote" | "bibl" | "ab") {
                        let t = self.inline_text(k);
                        self.b.quote(&t);
                    }
                }
                if !n.children().any(|c| c.is_element() && matches!(name(c), "p" | "l" | "lg" | "quote" | "bibl" | "ab")) {
                    let t = self.inline_text(n);
                    self.b.quote(&t);
                }
            }
            "sp" => self.speech(n),
            "formula" => self.display_formula(n),
            "figure" => self.figure(n),
            "table" => self.table(n),
            "titlePage" | "docTitle" => {
                for k in n.children().filter(Node::is_element) {
                    if name(k) == "titlePart" || name(k) == "docTitle" {
                        let t = self.inline_text(k);
                        if !self.title_emitted && !t.is_empty() {
                            self.b.heading(1, &t);
                            self.title_emitted = true;
                            self.meta.title.get_or_insert(t);
                        } else {
                            self.b.paragraph(&t);
                        }
                    } else {
                        self.block(k, depth);
                    }
                }
            }
            "titlePart" => {
                let t = self.inline_text(n);
                self.b.paragraph(&t);
            }
            "ref" | "ptr" | "milestone" | "cb" | "lb" | "fw" | "gap" | "space" | "anchor" | "index" => {}
            "front" | "body" | "back" | "text" | "group" | "floatingText" => self.container(n, depth),
            "teiHeader" => {}
            _ => {
                if n.children().any(|c| c.is_element() && is_block(name(c))) {
                    self.container(n, depth);
                } else {
                    self.paragraph(n);
                }
            }
        }
    }

    fn div(&mut self, d: Node, depth: usize) {
        let kind = attr(d, "type").unwrap_or("");
        let head = child(d, "head");
        let mut level = depth + 1;
        let mut heading_text = String::new();
        if let Some(h) = head {
            heading_text = self.inline_text(h);
            // GROBID writes flat divs whose number carries the depth: "2.1" is a level-3 heading.
            if let Some(n) = attr(h, "n").map(str::trim).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.')) {
                let parts = n.trim_end_matches('.').split('.').filter(|p| !p.is_empty()).count();
                if parts > 0 {
                    level = parts + 1;
                    if !heading_text.starts_with(n) {
                        heading_text = format!("{n} {heading_text}");
                    }
                }
            }
        } else if !d.children().any(|c| is(c, "div") && child(c, "head").is_some()) {
            // A typed division with no heading of its own gets the conventional one, unless a nested
            // division already carries it (GROBID wraps an acknowledgement in a typed outer div).
            heading_text = match kind {
                "references" | "bibliography" => "References",
                "acknowledgement" | "acknowledgements" => "Acknowledgements",
                _ => "",
            }
            .to_string();
        }
        self.b.heading(level.min(6) as u8, &heading_text);
        // Only divisions that carry a heading deepen the outline.
        let inner = if heading_text.trim().is_empty() { depth } else { depth + 1 };
        for k in d.children().filter(Node::is_element) {
            if name(k) != "head" {
                self.block(k, inner);
            }
        }
    }

    fn page_break(&mut self, pb: Node) {
        let label = attr(pb, "n").map(str::trim).filter(|l| !l.is_empty()).map(str::to_string);
        self.b.page_break(label);
    }

    /// A paragraph: inline content, with page breaks, block formulas and notes handled in order.
    fn paragraph(&mut self, p: Node) {
        let mut buf = String::new();
        for c in p.children() {
            if c.is_element() {
                match name(c) {
                    "pb" => {
                        self.b.paragraph(&std::mem::take(&mut buf));
                        self.page_break(c);
                        continue;
                    }
                    "formula" if attr(c, "type") == Some("display") || c.parent().is_some_and(|pp| is(pp, "div")) => {
                        self.b.paragraph(&std::mem::take(&mut buf));
                        self.display_formula(c);
                        continue;
                    }
                    "figure" | "table" | "list" | "quote" | "lg" | "listBibl" => {
                        self.b.paragraph(&std::mem::take(&mut buf));
                        self.block(c, 1);
                        continue;
                    }
                    _ => {}
                }
            }
            self.inline(c, &mut buf);
        }
        self.b.paragraph(&buf);
        for note in std::mem::take(&mut self.pending_notes) {
            self.b.paragraph(&format!("[note] {note}"));
        }
    }

    fn inline_text(&mut self, n: Node) -> String {
        let mut s = String::new();
        for c in n.children() {
            self.inline(c, &mut s);
        }
        collapse_ws(&s)
    }

    fn inline(&mut self, n: Node, out: &mut String) {
        if n.is_text() {
            out.push_str(n.text().unwrap_or(""));
            return;
        }
        if !n.is_element() {
            return;
        }
        match name(n) {
            "hi" => {
                let rend = attr(n, "rend").unwrap_or("");
                let t = self.inline_text(n);
                if rend.contains("sup") {
                    script('^', &t, out);
                } else if rend.contains("sub") {
                    script('_', &t, out);
                } else {
                    out.push_str(&t);
                }
            }
            "formula" | "math" => {
                if let Some(l) = formula_latex(n) {
                    out.push('$');
                    out.push_str(&l);
                    out.push('$');
                } else {
                    out.push_str(&text_of(n));
                }
            }
            "note" => {
                let t = self.inline_text(n);
                if !t.is_empty() {
                    self.pending_notes.push(t);
                }
            }
            "lb" | "br" => out.push(' '),
            "pb" | "cb" | "milestone" | "anchor" | "ptr" | "gap" | "index" | "fw" => {}
            "label" if n.parent().is_some_and(|p| is(p, "formula")) => {}
            "figure" | "table" => {
                let cap = child(n, "head").map(|h| self.inline_text(h)).unwrap_or_default();
                out.push(' ');
                out.push_str(&cap);
                out.push(' ');
            }
            "lg" => {
                let t = n.children().filter(|c| is(*c, "l")).map(|l| self.inline_text(l)).collect::<Vec<_>>().join(" / ");
                out.push_str(&t);
            }
            "del" | "surplus" => {}
            "choice" => {
                // Prefer the corrected or regularised reading.
                let pick = ["corr", "reg", "expan", "ex"].iter().find_map(|t| child(n, t)).or_else(|| n.children().find(Node::is_element));
                if let Some(p) = pick {
                    self.inline(p, out);
                }
            }
            _ => {
                for c in n.children() {
                    self.inline(c, out);
                }
            }
        }
    }

    fn verse(&mut self, lg: Node) {
        // Nested line groups (stanzas in a poem) are separate stanzas.
        let nested: Vec<Node> = lg.children().filter(|c| is(*c, "lg")).collect();
        if !nested.is_empty() {
            if let Some(h) = child(lg, "head") {
                let t = self.inline_text(h);
                self.b.paragraph(&t);
            }
            for s in nested {
                self.verse(s);
            }
            return;
        }
        let lines: Vec<String> = lg.children().filter(|c| is(*c, "l")).map(|l| self.inline_text(l)).filter(|t| !t.is_empty()).collect();
        self.b.paragraph(&lines.join(" / "));
    }

    fn speech(&mut self, sp: Node) {
        let who = child(sp, "speaker").map(|s| self.inline_text(s)).unwrap_or_default();
        let mut lines: Vec<String> = Vec::new();
        for k in sp.children().filter(Node::is_element) {
            match name(k) {
                "speaker" => {}
                "stage" => {
                    let t = self.inline_text(k);
                    lines.push(format!("({t})"));
                }
                _ => {
                    let t = self.inline_text(k);
                    if !t.is_empty() {
                        lines.push(t);
                    }
                }
            }
        }
        let body = lines.join(" ");
        self.b.paragraph(&if who.is_empty() { body } else { format!("{who}: {body}") });
    }

    fn list(&mut self, l: Node, depth: usize) {
        let ordered = matches!(attr(l, "type"), Some("ordered")) || attr(l, "rend").is_some_and(|r| r.contains("numbered"));
        if depth == 0 {
            self.b.break_list();
        }
        if let Some(h) = child(l, "head") {
            let t = self.inline_text(h);
            self.b.paragraph(&t);
        }
        let mut n = 0;
        for item in l.children().filter(|c| is(*c, "item") || is(*c, "witness") || is(*c, "person") || is(*c, "place") || is(*c, "event")) {
            n += 1;
            let label = child(item, "label").map(|x| self.inline_text(x)).unwrap_or_default();
            let mut text = String::new();
            let mut nested: Vec<Node> = Vec::new();
            for c in item.children() {
                if c.is_element() && name(c) == "list" {
                    nested.push(c);
                } else if c.is_element() && name(c) == "label" {
                } else {
                    self.inline(c, &mut text);
                }
            }
            let text = collapse_ws(&text);
            let (marker, line) = if ordered && label.is_empty() {
                (format!("{n}."), text)
            } else if !label.is_empty() {
                ("-".to_string(), format!("{label} {text}"))
            } else {
                ("-".to_string(), text)
            };
            self.b.list_item(depth, &marker, &line);
            for nl in nested {
                self.list(nl, depth + 1);
            }
        }
    }

    fn display_formula(&mut self, f: Node) {
        match formula_latex(f) {
            Some(l) => self.b.display_math(&l),
            None => {
                // GROBID linearises formulas to plain text, which is not LaTeX; keep it as text.
                let mut s = String::new();
                for c in f.children() {
                    if !(c.is_element() && name(c) == "label") {
                        self.inline(c, &mut s);
                    }
                }
                let label = child(f, "label").map(text_of).unwrap_or_default();
                let t = collapse_ws(&s);
                if !t.is_empty() {
                    self.b.paragraph(&if label.is_empty() { t } else { format!("{t} {label}") });
                }
            }
        }
    }

    fn figure(&mut self, f: Node) {
        // GROBID wraps tables in <figure type="table">.
        if let Some(t) = child(f, "table") {
            let cap = [child(f, "head"), child(f, "label")]
                .iter()
                .flatten()
                .map(|h| self.inline_text(*h))
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let desc = child(f, "figDesc").map(|d| self.inline_text(d)).unwrap_or_default();
            let caption = collapse_ws(&format!("{cap} {desc}"));
            self.b.paragraph(&caption);
            self.table(t);
            return;
        }
        let mut parts: Vec<String> = Vec::new();
        for tag in ["head", "label", "figDesc"] {
            for k in children_named(f, tag) {
                let t = self.inline_text(k);
                if !t.is_empty() {
                    parts.push(t);
                }
            }
        }
        let text = parts.join(" ");
        self.b.paragraph(&text);
    }

    fn table(&mut self, t: Node) {
        let mut g = Grid::default();
        if let Some(h) = child(t, "head") {
            let cap = self.inline_text(h);
            self.b.paragraph(&cap);
        }
        let mut header_rows = 0;
        let mut in_header = true;
        for row in children_named(t, "row") {
            let is_label = attr(row, "role") == Some("label");
            if is_label && in_header {
                header_rows += 1;
            } else {
                in_header = false;
            }
            let cells = children_named(row, "cell")
                .map(|c| {
                    let span = |a: &str| attr(c, a).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1);
                    Cell::new(self.inline_text(c), span("cols"), span("rows"))
                })
                .collect();
            g.rows.push(cells);
        }
        g.header_rows = header_rows;
        g.emit(&mut self.b);
    }

    // -------------------------------------------------------------------------------------------- references

    fn list_bibl(&mut self, lb: Node) {
        for k in lb.children().filter(Node::is_element) {
            match name(k) {
                "biblStruct" => self.bibl_struct(k),
                "bibl" => {
                    let t = self.inline_text(k);
                    if !t.is_empty() {
                        let r = ids::reference_from_raw(&t, attr(k, "id").map(str::to_string).filter(|_| false));
                        self.b.paragraph(&r.raw);
                        self.refs.push(r);
                    }
                }
                "head" => {
                    let t = self.inline_text(k);
                    self.b.paragraph(&t);
                }
                _ => {}
            }
        }
    }

    fn bibl_struct(&mut self, bs: Node) {
        let analytic = child(bs, "analytic");
        let monogr = child(bs, "monogr");
        let raw_note = bs.descendants().find(|d| is(*d, "note") && attr(*d, "type") == Some("raw_reference"));
        let mut entry = Reference::default();

        let title_of = |scope: Option<Node>| -> Option<String> {
            let s = scope?;
            children_named(s, "title").find(|t| attr(*t, "type") == Some("main") || attr(*t, "level") == Some("a")).or_else(|| child(s, "title")).map(text_of).filter(|t| !t.is_empty())
        };
        let art_title = title_of(analytic);
        let source = monogr.and_then(|m| child(m, "title")).map(text_of).filter(|t| !t.is_empty());
        entry.title = art_title.clone().or_else(|| source.clone());

        let authors: Vec<String> = analytic
            .into_iter()
            .chain(monogr)
            .flat_map(|s| children_named(s, "author").filter_map(|a| person_name_short(a)).collect::<Vec<_>>())
            .collect();
        let imprint = monogr.and_then(|m| child(m, "imprint"));
        let year = imprint.and_then(|i| descendant(i, "date")).and_then(|d| attr(d, "when").and_then(ids::year_in).or_else(|| ids::year_in(&text_of(d))));
        let scope = |unit: &str| -> Option<String> {
            imprint.and_then(|i| children_named(i, "biblScope").find(|b| attr(*b, "unit") == Some(unit))).map(|b| {
                let from = attr(b, "from");
                let to = attr(b, "to");
                match (from, to) {
                    (Some(f), Some(t)) => format!("{f}-{t}"),
                    (Some(f), None) => f.to_string(),
                    _ => text_of(b),
                }
            })
        };
        let raw = match raw_note {
            Some(n) => text_of(n),
            None => {
                let mut s = authors.join(", ");
                if let Some(y) = year {
                    if !s.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(&format!("({y})"));
                }
                let push = |s: &mut String, part: String| {
                    if part.is_empty() {
                        return;
                    }
                    if !s.is_empty() {
                        if !s.ends_with(['.', ')', '?', '!']) {
                            s.push('.');
                        }
                        s.push(' ');
                    }
                    s.push_str(&part);
                };
                push(&mut s, art_title.clone().unwrap_or_default());
                let mut src = if art_title.is_some() { source.clone().unwrap_or_default() } else { String::new() };
                if let Some(v) = scope("volume") {
                    src = format!("{src} {v}").trim().to_string();
                }
                if let Some(i) = scope("issue") {
                    src.push_str(&format!("({i})"));
                }
                if let Some(p) = scope("page") {
                    src.push_str(&format!(":{p}"));
                }
                push(&mut s, src);
                s
            }
        };
        let mut r = ids::reference_from_raw(&raw, attr(bs, "id").map(|i| i.trim_start_matches('b').to_string()).filter(|i| i.chars().all(|c| c.is_ascii_digit())));
        for id in bs.descendants().filter(|d| is(*d, "idno")) {
            let v = text_of(id);
            match attr(id, "type").unwrap_or("").to_ascii_lowercase().as_str() {
                "doi" => r.doi = ids::find_doi(&v).or(r.doi.take()),
                "arxiv" => r.arxiv_id = ids::bare_arxiv(&v).or_else(|| ids::find_arxiv(&v)).or(r.arxiv_id.take()),
                "pmcid" | "pmc" => r.pmcid = ids::normalize_pmcid(&v).or(r.pmcid.take()),
                "pmid" => r.pmid = Some(v).filter(|v| v.chars().all(|c| c.is_ascii_digit())).or(r.pmid.take()),
                "isbn" => r.isbn = ids::normalize_isbn(&v).or(r.isbn.take()),
                _ => {}
            }
        }
        r.title = entry.title;
        r.year = year.or(r.year);
        if r.raw.is_empty() {
            return;
        }
        let line = match &r.label {
            Some(l) => format!("[{l}] {}", r.raw),
            None => r.raw.clone(),
        };
        self.b.paragraph(&line);
        self.refs.push(r);
    }
}

fn is_block(n: &str) -> bool {
    matches!(n, "div" | "p" | "list" | "lg" | "l" | "table" | "figure" | "quote" | "sp" | "head" | "listBibl" | "formula" | "ab" | "titlePage")
}

/// "Given Middle Surname" from a `persName` (or a plain `name`).
fn person_name(author: Node) -> Option<String> {
    let pn = child(author, "persName").unwrap_or(author);
    let forenames: Vec<String> = children_named(pn, "forename").map(text_of).filter(|s| !s.is_empty()).collect();
    let surname: Vec<String> = children_named(pn, "surname").map(text_of).filter(|s| !s.is_empty()).collect();
    let mut parts = forenames;
    parts.extend(surname);
    if parts.is_empty() {
        let t = text_of(pn);
        return Some(t).filter(|t| !t.is_empty());
    }
    Some(parts.join(" "))
}

/// "Surname G M" from a `persName`, the style used in printed reference lists.
fn person_name_short(author: Node) -> Option<String> {
    let pn = child(author, "persName").unwrap_or(author);
    let sur = children_named(pn, "surname").map(text_of).collect::<Vec<_>>().join(" ");
    let initials: String = children_named(pn, "forename").map(text_of).filter_map(|f| f.chars().next()).collect();
    if sur.is_empty() {
        return Some(text_of(pn)).filter(|t| !t.is_empty());
    }
    Some(if initials.is_empty() { sur } else { format!("{sur} {initials}") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;

    const GROBID: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<TEI xml:space="preserve" xmlns="http://www.tei-c.org/ns/1.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<teiHeader xml:lang="en"><fileDesc>
<titleStmt><title level="a" type="main">Synthetic Findings in Test Data</title></titleStmt>
<publicationStmt><publisher/><availability status="unknown"><licence/></availability></publicationStmt>
<sourceDesc><biblStruct>
<analytic>
<author><persName><forename type="first">Ann</forename><forename type="middle">B</forename><surname>Lee</surname></persName></author>
<author><persName><forename type="first">Bo</forename><surname>Chen</surname></persName></author>
<idno type="DOI">10.1234/syn.2021.7</idno>
<title level="a" type="main">Synthetic Findings in Test Data</title>
</analytic>
<monogr><title level="j">Journal of Tests</title><imprint><date type="published" when="2021-03-04"/></imprint></monogr>
<idno type="arXiv">arXiv:2103.01234v1</idno>
</biblStruct></sourceDesc></fileDesc>
<profileDesc><textClass><keywords><term>testing</term><term>synthetic data</term></keywords></textClass>
<abstract><div><p>We test the converter on a synthetic article.</p></div></abstract></profileDesc>
</teiHeader>
<text xml:lang="en"><body>
<div><head n="1">Introduction</head><p>First paragraph with a citation <ref type="bibr" target="#b0">(Smith and Doe, 2019)</ref> and a figure <ref type="figure" target="#fig_0">1</ref>.</p>
<p>A second paragraph<note place="foot" n="1">A footnote about it.</note> continues here.</p></div>
<div><head n="2">Methods</head>
<div><head n="2.1">Setup</head><p>Setup text.</p>
<formula xml:id="formula_0">E = mc 2<label>(1)</label></formula>
<p>After the formula.</p></div>
<figure xml:id="fig_0"><head>Figure 1</head><label>1</label><figDesc>The experimental layout.</figDesc></figure>
<figure type="table" xml:id="tab_0"><head>Table 1</head><figDesc>Results by group.</figDesc>
<table><row role="label"><cell>Group</cell><cell>Mean</cell></row><row><cell>A</cell><cell>1.5</cell></row><row><cell>B</cell><cell>2.5</cell></row></table></figure>
</div>
</body>
<back>
<div type="acknowledgement"><div><head>Acknowledgements</head><p>Thanks.</p></div></div>
<div type="references"><listBibl>
<biblStruct xml:id="b0"><analytic><title level="a" type="main">A study of things</title>
<author><persName><forename type="first">J</forename><surname>Smith</surname></persName></author>
<author><persName><forename type="first">A</forename><surname>Doe</surname></persName></author><idno type="DOI">10.5555/abc.123</idno></analytic>
<monogr><title level="j">Journal of Stuff</title><imprint><biblScope unit="volume">5</biblScope><biblScope unit="page" from="1" to="10"/><date type="published" when="2019"/></imprint></monogr></biblStruct>
<biblStruct xml:id="b1"><monogr><title level="m">A Book of Things</title><author><persName><forename>K</forename><surname>Jones</surname></persName></author><imprint><date type="published" when="2001"/><idno type="ISBN">0-306-40615-2</idno></imprint></monogr><note type="raw_reference">Jones K. A Book of Things. Press; 2001.</note></biblStruct>
</listBibl></div>
</back></text></TEI>"##;

    fn run(src: &str) -> Extracted {
        convert(src.as_bytes(), &ConvertOptions::default()).unwrap()
    }

    #[test]
    fn grobid_metadata() {
        let r = run(GROBID);
        let m = &r.metadata;
        assert_eq!(m.title.as_deref(), Some("Synthetic Findings in Test Data"));
        assert_eq!(m.authors, vec!["Ann B Lee", "Bo Chen"]);
        assert_eq!(m.doi.as_deref(), Some("10.1234/syn.2021.7"));
        assert_eq!(m.arxiv_id.as_deref(), Some("2103.01234"));
        assert_eq!(m.year, Some(2021));
        assert_eq!(m.journal.as_deref(), Some("Journal of Tests"));
        assert_eq!(m.keywords, vec!["testing", "synthetic data"]);
        assert_eq!(m.language.as_deref(), Some("en"));
        assert_eq!(m.abstract_text.as_deref(), Some("We test the converter on a synthetic article."));
        assert!(r.tool.ends_with(" tei"));
    }

    #[test]
    fn grobid_numbered_headings_set_levels() {
        let r = run(GROBID);
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(
            hs,
            vec![
                (1, "Synthetic Findings in Test Data"),
                (2, "Abstract"),
                (2, "1 Introduction"),
                (2, "2 Methods"),
                (3, "2.1 Setup"),
                (2, "Acknowledgements"),
                (2, "References"),
            ]
        );
        let lines: Vec<&str> = r.markdown.lines().collect();
        for h in &r.headings {
            assert_eq!(lines[h.line as usize - 1], format!("{} {}", "#".repeat(h.level as usize), h.text));
        }
    }

    #[test]
    fn body_text_notes_formulas_figures_and_tables() {
        let r = run(GROBID);
        let md = &r.markdown;
        assert!(md.contains("a citation (Smith and Doe, 2019) and a figure 1."), "{md}");
        assert!(md.contains("A second paragraph continues here.\n\n[note] A footnote about it."), "{md}");
        // GROBID formulas are plain text, not LaTeX: kept as a paragraph, not wrapped in $$.
        assert!(md.contains("E = mc 2 (1)"), "{md}");
        assert!(!md.contains("$$"), "{md}");
        assert!(md.contains("Figure 1 1 The experimental layout."), "{md}");
        assert!(md.contains("Table 1 Results by group.\n\n| Group | Mean |\n| --- | --- |\n| A | 1.5 |\n| B | 2.5 |"), "{md}");
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Table).count(), 1);
    }

    #[test]
    fn references_with_ids_and_raw_text() {
        let r = run(GROBID);
        assert_eq!(r.references.len(), 2);
        let a = &r.references[0];
        assert_eq!(a.doi.as_deref(), Some("10.5555/abc.123"));
        assert_eq!(a.year, Some(2019));
        assert_eq!(a.label.as_deref(), Some("0"));
        assert_eq!(a.title.as_deref(), Some("A study of things"));
        assert!(a.raw.starts_with("Smith J, Doe A (2019) A study of things. Journal of Stuff"), "{}", a.raw);
        let b = &r.references[1];
        assert_eq!(b.raw, "Jones K. A Book of Things. Press; 2001.");
        assert_eq!(b.isbn.as_deref(), Some("0306406152"));
        let refs = r.headings.iter().find(|h| h.text == "References").unwrap().line as usize;
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert!(lines[refs + 1].starts_with("[0] Smith J, Doe A (2019)"), "{:?}", &lines[refs..]);
    }

    #[test]
    fn page_breaks_verse_and_drama() {
        let src = r#"<TEI xmlns="http://www.tei-c.org/ns/1.0"><teiHeader><fileDesc><titleStmt><title>A Play</title></titleStmt></fileDesc></teiHeader>
<text><body><div type="chapter"><head>Act One</head>
<pb n="12"/><p>Opening words.</p>
<lg><l>Roses are red</l><l>Violets are blue</l></lg>
<sp><speaker>HAMLET</speaker><p>To be, or not to be.</p><stage>aside</stage></sp>
<p>Page turns<pb n="13"/> mid paragraph.</p>
<list type="ordered"><item>one</item><item>two</item></list>
</div></body></text></TEI>"#;
        let r = run(src);
        let md = &r.markdown;
        assert!(md.contains("Roses are red / Violets are blue"), "{md}");
        assert!(md.contains("HAMLET: To be, or not to be. (aside)"), "{md}");
        assert!(md.contains("1. one\n2. two"), "{md}");
        let labels: Vec<_> = r.pages.iter().map(|p| (p.label.clone().unwrap(), p.index)).collect();
        assert_eq!(labels, vec![("12".to_string(), 1), ("13".to_string(), 2)]);
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert_eq!(lines[r.pages[0].line as usize - 1], "Opening words.");
        assert_eq!(lines[r.pages[1].line as usize - 1], "mid paragraph.");
    }

    #[test]
    fn mathml_inside_tei_becomes_latex() {
        let src = r#"<TEI xmlns="http://www.tei-c.org/ns/1.0"><text><body><div><head>Maths</head>
<p>Inline <formula notation="mathml"><math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi><mo>+</mo><mn>1</mn></math></formula> text.</p>
<formula type="display"><math xmlns="http://www.w3.org/1998/Math/MathML"><mfrac><mn>1</mn><mn>2</mn></mfrac></math></formula></div></body></text></TEI>"#;
        let r = run(src);
        assert!(r.markdown.contains("Inline $x + 1$ text."), "{}", r.markdown);
        assert!(r.markdown.contains("$$\n\\frac{1}{2}\n$$"), "{}", r.markdown);
    }

    #[test]
    fn unexpected_root_and_empty() {
        assert!(convert(b"<html/>", &ConvertOptions::default()).is_err());
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
    }
}

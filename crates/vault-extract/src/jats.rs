//! JATS (and light BITS) article XML to canonical-ready Markdown.
//!
//! Sections become headings, MathML and `tex-math` become LaTeX, tables become pipe tables, captions become
//! paragraphs, and the reference list becomes structured entries. Front matter supplies the metadata.

use roxmltree::Node;

use crate::ConvertOptions;
use crate::builder::{MdBuilder, collapse_ws};
use crate::error::{ExtractError, Result};
use crate::ids;
use crate::mathml;
use crate::tables::{Cell, Grid};
use crate::types::{Extracted, Metadata, Reference, tool_string};
use crate::xmlutil::{self, attr, child, children_named, descendant, is, name, raw_text, text_of, text_of_child};

/// Elements that are blocks even when they sit inside a paragraph.
const BLOCKS_IN_P: [&str; 18] = [
    "disp-formula",
    "disp-formula-group",
    "fig",
    "fig-group",
    "table-wrap",
    "table-wrap-group",
    "list",
    "def-list",
    "disp-quote",
    "boxed-text",
    "preformat",
    "code",
    "statement",
    "array",
    "media",
    "supplementary-material",
    "verse-group",
    "chem-struct-wrap",
];

pub fn convert(bytes: &[u8], _opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    xmlutil::with_document(bytes, "jats", |doc, enc| {
        let root = doc.root_element();
        let mut c = Jats::default();
        match name(root) {
            "article" | "sub-article" | "response" => c.article(root),
            "pmc-articleset" | "articles" | "article-set" => {
                let articles: Vec<Node> = root.children().filter(|n| is(*n, "article")).collect();
                let Some(first) = articles.first() else {
                    return Err(ExtractError::convert("jats", "no <article> in the article set"));
                };
                if articles.len() > 1 {
                    c.warnings.push(format!("article set holds {} articles; converted the first", articles.len()));
                }
                c.article(*first);
            }
            "book" | "book-part-wrapper" | "book-part" => c.book(root),
            other => return Err(ExtractError::convert("jats", format!("unexpected root element <{other}>"))),
        }
        Ok(c.finish(enc))
    })
}

#[derive(Default)]
struct Jats {
    b: MdBuilder,
    meta: Metadata,
    refs: Vec<Reference>,
    warnings: Vec<String>,
}

impl Jats {
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
            tool: tool_string("jats"),
            encoding: Some(encoding.to_string()),
        }
    }

    // ------------------------------------------------------------------------------------------ articles

    fn article(&mut self, article: Node) {
        if let Some(l) = attr(article, "lang").filter(|l| !l.is_empty()) {
            self.meta.language = Some(l.to_string());
        }
        let front = child(article, "front").or_else(|| child(article, "front-stub"));
        if let Some(front) = front {
            self.front(front);
        }
        if let Some(body) = child(article, "body") {
            self.blocks_of(body, 1);
        }
        if let Some(back) = child(article, "back") {
            self.back(back);
        }
        if let Some(floats) = child(article, "floats-group") {
            self.blocks_of(floats, 1);
        }
        for sub in children_named(article, "sub-article").chain(children_named(article, "response")) {
            let title = descendant(sub, "article-title").map(text_of).unwrap_or_default();
            let kind = attr(sub, "article-type").unwrap_or("Sub-article");
            self.b.heading(2, &if title.is_empty() { kind.to_string() } else { format!("{kind}: {title}") });
            if let Some(body) = child(sub, "body") {
                self.blocks_of(body, 2);
            }
            if let Some(back) = child(sub, "back") {
                self.back(back);
            }
        }
    }

    fn book(&mut self, root: Node) {
        if let Some(meta) = descendant(root, "book-meta") {
            self.meta.title = descendant(meta, "book-title").map(text_of).filter(|t| !t.is_empty());
            self.meta.authors = self.contributors(meta);
            self.meta.isbn = children_named(meta, "isbn").filter_map(|i| ids::normalize_isbn(&text_of(i))).collect();
            self.meta.year = descendant(meta, "year").and_then(|y| text_of(y).parse().ok());
            if let Some(t) = &self.meta.title {
                self.b.heading(1, t);
            }
        }
        for n in root.children().filter(Node::is_element) {
            match name(n) {
                "book-body" | "front-matter" | "book-back" => {
                    for part in n.children().filter(Node::is_element) {
                        self.book_part_or_block(part, 1);
                    }
                }
                "book-part" => self.book_part_or_block(n, 1),
                _ => {}
            }
        }
    }

    fn book_part_or_block(&mut self, n: Node, depth: usize) {
        match name(n) {
            "book-part" | "front-matter-part" | "book-part-wrapper" => {
                let meta = child(n, "book-part-meta");
                let title = meta.and_then(|m| descendant(m, "title")).map(|t| self.inline_text(t)).unwrap_or_default();
                let label = meta.and_then(|m| descendant(m, "label")).map(text_of).unwrap_or_default();
                self.b.heading((depth + 1) as u8, &join_label(&label, &title));
                for k in n.children().filter(Node::is_element) {
                    match name(k) {
                        "body" | "named-book-part-body" => self.blocks_of(k, depth + 1),
                        "back" => self.back(k),
                        "book-part" | "front-matter-part" => self.book_part_or_block(k, depth + 1),
                        _ => {}
                    }
                }
            }
            _ => self.block(n, depth),
        }
    }

    // ----------------------------------------------------------------------------------------------- front

    fn front(&mut self, front: Node) {
        if let Some(jm) = child(front, "journal-meta") {
            self.meta.journal = descendant(jm, "journal-title")
                .map(text_of)
                .filter(|t| !t.is_empty())
                .or_else(|| descendant(jm, "abbrev-journal-title").map(text_of));
        }
        let Some(am) = child(front, "article-meta").or(Some(front)) else { return };
        for id in children_named(am, "article-id") {
            let v = text_of(id);
            match attr(id, "pub-id-type").unwrap_or("") {
                "doi" => self.meta.doi = ids::find_doi(&v).or(self.meta.doi.take()),
                "pmc" | "pmcid" => self.meta.pmcid = ids::normalize_pmcid(&v).or(self.meta.pmcid.take()),
                "arxiv" => {
                    self.meta.arxiv_id =
                        ids::bare_arxiv(&v).or_else(|| ids::find_arxiv(&v)).or(self.meta.arxiv_id.take())
                }
                _ => {}
            }
        }
        let title_group = child(am, "title-group");
        let title =
            title_group.and_then(|t| child(t, "article-title")).map(|t| self.inline_text(t)).unwrap_or_default();
        let subtitle = title_group.and_then(|t| child(t, "subtitle")).map(|t| self.inline_text(t)).unwrap_or_default();
        let full_title = match (title.is_empty(), subtitle.is_empty()) {
            (false, false) => format!("{title}: {subtitle}"),
            (false, true) => title,
            (true, false) => subtitle,
            (true, true) => String::new(),
        };
        if !full_title.is_empty() {
            self.meta.title = Some(full_title.clone());
            self.b.heading(1, &full_title);
        }
        self.meta.authors = self.contributors(am);
        if !self.meta.authors.is_empty() {
            let line = self.meta.authors.join(", ");
            self.b.paragraph(&line);
        }
        // Publication year: prefer the electronic or print publication dates over history dates.
        let mut years: Vec<(u8, i32)> = Vec::new();
        for d in children_named(am, "pub-date") {
            if let Some(y) = child(d, "year").and_then(|y| text_of(y).parse::<i32>().ok()) {
                let rank = match attr(d, "pub-type").or(attr(d, "date-type")).unwrap_or("") {
                    "epub" | "pub" => 0,
                    "ppub" | "collection" | "epub-ppub" => 1,
                    _ => 2,
                };
                years.push((rank, y));
            }
        }
        years.sort_by_key(|(r, _)| *r);
        self.meta.year = years.first().map(|(_, y)| *y);

        for kg in children_named(am, "kwd-group") {
            for k in children_named(kg, "kwd") {
                let t = text_of(k);
                if !t.is_empty() && !self.meta.keywords.contains(&t) {
                    self.meta.keywords.push(t);
                }
            }
        }
        for abs in children_named(am, "abstract").chain(children_named(am, "trans-abstract")) {
            self.abstract_block(abs);
        }
        if !self.meta.keywords.is_empty() {
            let line = format!("Keywords: {}", self.meta.keywords.join("; "));
            self.b.paragraph(&line);
        }
    }

    fn contributors(&self, scope: Node) -> Vec<String> {
        let mut authors = Vec::new();
        let mut others = Vec::new();
        for cg in children_named(scope, "contrib-group") {
            for c in children_named(cg, "contrib") {
                let Some(n) = self.contrib_name(c) else { continue };
                match attr(c, "contrib-type") {
                    Some("author") | None => authors.push(n),
                    _ => others.push(n),
                }
            }
        }
        if authors.is_empty() { others } else { authors }
    }

    fn contrib_name(&self, c: Node) -> Option<String> {
        let n = child(c, "name").or_else(|| child(c, "name-alternatives").and_then(|a| child(a, "name")));
        if let Some(n) = n {
            let given = text_of_child(n, "given-names").unwrap_or_default();
            let sur = text_of_child(n, "surname").unwrap_or_default();
            let prefix = text_of_child(n, "prefix");
            let suffix = text_of_child(n, "suffix");
            let mut parts = Vec::new();
            parts.extend(prefix);
            if !given.is_empty() {
                parts.push(given);
            }
            if !sur.is_empty() {
                parts.push(sur);
            }
            let mut s = parts.join(" ");
            if let Some(suf) = suffix {
                s.push(' ');
                s.push_str(&suf);
            }
            return (!s.trim().is_empty()).then_some(s);
        }
        if let Some(sn) = child(c, "string-name") {
            return Some(text_of(sn)).filter(|s| !s.is_empty());
        }
        child(c, "collab").map(text_of).filter(|s| !s.is_empty())
    }

    fn abstract_block(&mut self, abs: Node) {
        let kind = name(abs);
        let title = child(abs, "title").map(|t| self.inline_text(t)).filter(|t| !t.is_empty());
        let default = if kind == "trans-abstract" {
            match attr(abs, "lang") {
                Some(l) => format!("Abstract ({l})"),
                None => "Abstract (translation)".to_string(),
            }
        } else {
            "Abstract".to_string()
        };
        self.b.heading(2, &title.unwrap_or(default));
        let mut plain = Vec::new();
        for k in abs.children().filter(Node::is_element) {
            if name(k) == "title" {
                continue;
            }
            if name(k) == "sec" {
                let t = child(k, "title").map(|t| self.inline_text(t)).unwrap_or_default();
                self.b.heading(3, &t);
                for p in k.children().filter(|n| is(*n, "p")) {
                    plain.push(self.inline_text(p));
                    self.para(p);
                }
            } else {
                if name(k) == "p" {
                    plain.push(self.inline_text(k));
                }
                self.block(k, 2);
            }
        }
        if kind == "abstract" && self.meta.abstract_text.is_none() && attr(abs, "abstract-type").is_none() {
            let joined = plain.join(" ");
            if !joined.is_empty() {
                self.meta.abstract_text = Some(joined);
            }
        }
    }

    // --------------------------------------------------------------------------------------------- blocks

    fn blocks_of(&mut self, n: Node, depth: usize) {
        for k in n.children().filter(Node::is_element) {
            self.block(k, depth);
        }
    }

    fn block(&mut self, n: Node, depth: usize) {
        match name(n) {
            "sec" | "app" | "notes" | "glossary" | "bio" | "ack" | "ref-list" | "fn-group" | "app-group" => {
                self.section(n, depth)
            }
            "p" => self.para(n),
            "title" | "label" | "sec-meta" | "object-id" | "kwd-group" | "permissions" => {}
            "list" => self.list(n, 0),
            "def-list" => self.def_list(n),
            "disp-quote" => {
                for p in n.children().filter(Node::is_element) {
                    if name(p) == "attrib" {
                        let t = self.inline_text(p);
                        self.b.quote(&format!("-- {t}"));
                    } else {
                        let t = self.inline_text(p);
                        self.b.quote(&t);
                    }
                }
            }
            "disp-formula" | "chem-struct-wrap" => self.display_formula(n),
            "disp-formula-group" => {
                for f in n.children().filter(|c| is(*c, "disp-formula")) {
                    self.display_formula(f);
                }
            }
            "fig" => self.figure(n),
            "fig-group" => {
                for f in n.children().filter(Node::is_element) {
                    if name(f) == "fig" {
                        self.figure(f);
                    } else {
                        self.block(f, depth);
                    }
                }
            }
            "table-wrap" => self.table_wrap(n),
            "table-wrap-group" => {
                for t in n.children().filter(Node::is_element) {
                    if name(t) == "table-wrap" {
                        self.table_wrap(t);
                    } else {
                        self.block(t, depth);
                    }
                }
            }
            "table" => self.table(n),
            "preformat" | "code" => {
                let lang = attr(n, "language").or(attr(n, "code-type")).unwrap_or("");
                self.b.code_block(lang, &raw_text(n));
            }
            "boxed-text" | "statement" | "verse-group" | "speech" | "alternatives" => {
                let label = text_of_child(n, "label").unwrap_or_default();
                let title = child(n, "title").map(|t| self.inline_text(t)).unwrap_or_default();
                let head = join_label(&label, &title);
                if !head.is_empty() {
                    self.b.paragraph(&head);
                }
                for k in n.children().filter(Node::is_element) {
                    if !matches!(name(k), "label" | "title" | "caption") {
                        self.block(k, depth);
                    }
                }
                if let Some(cap) = child(n, "caption") {
                    self.blocks_of(cap, depth);
                }
            }
            "supplementary-material" | "media" => {
                if let Some(cap) = child(n, "caption") {
                    let label = text_of_child(n, "label");
                    let text = self.caption_text(label.as_deref(), Some(cap));
                    self.b.paragraph(&text);
                }
            }
            "array" => {
                if let Some(t) = child(n, "tbody").or_else(|| child(n, "table")) {
                    let g = self.grid(n, t);
                    g.emit(&mut self.b);
                }
            }
            "graphic" | "inline-graphic" | "ref" | "fn" | "journal-meta" | "article-meta" => {
                if name(n) == "fn" {
                    self.footnote(n);
                }
            }
            "attrib" | "sig-block" | "sig" | "address" | "related-article" | "related-object" => {
                let t = self.inline_text(n);
                self.b.paragraph(&t);
            }
            "body" | "back" | "front" => self.blocks_of(n, depth),
            _ => {
                // Unknown container or stray inline element: keep its text.
                if n.children().any(|c| c.is_element() && is_block_name(name(c))) {
                    self.blocks_of(n, depth);
                } else {
                    let t = self.inline_text(n);
                    self.b.paragraph(&t);
                }
            }
        }
    }

    fn section(&mut self, n: Node, depth: usize) {
        let kind = name(n);
        let label = text_of_child(n, "label").unwrap_or_default();
        let title = child(n, "title").map(|t| self.inline_text(t)).unwrap_or_default();
        let level = (depth + 1) as u8;
        match kind {
            "ref-list" => {
                self.b.heading(2, if title.is_empty() { "References" } else { &title });
                self.ref_list(n);
                return;
            }
            "ack" => self.b.heading(2, if title.is_empty() { "Acknowledgements" } else { &title }),
            "fn-group" => self.b.heading(2, if title.is_empty() { "Notes" } else { &title }),
            "app-group" => {
                if !title.is_empty() {
                    self.b.heading(2, &title);
                }
            }
            "bio" => {
                if !title.is_empty() {
                    self.b.heading(level, &title);
                }
            }
            _ => {
                let head = join_label(&label, &title);
                if !head.is_empty() {
                    self.b.heading(level, &head);
                } else if kind == "app" {
                    self.b.heading(2, "Appendix");
                } else if kind == "glossary" {
                    self.b.heading(2, "Glossary");
                } else if kind == "notes" {
                    self.b.heading(2, "Notes");
                }
            }
        }
        let next_depth = if kind == "app-group" || kind == "ack" || kind == "fn-group" { depth } else { depth + 1 };
        for k in n.children().filter(Node::is_element) {
            match name(k) {
                "title" | "label" | "sec-meta" => {}
                "ref-list" => self.section(k, 1),
                _ => self.block(k, next_depth),
            }
        }
    }

    fn back(&mut self, back: Node) {
        for k in back.children().filter(Node::is_element) {
            match name(k) {
                "ref-list" => self.section(k, 1),
                "sec" | "app-group" | "app" | "ack" | "fn-group" | "glossary" | "notes" | "bio" => self.section(k, 1),
                _ => self.block(k, 1),
            }
        }
    }

    // ------------------------------------------------------------------------------------------ paragraphs

    /// A paragraph; block elements inside it end the paragraph and are emitted in order.
    fn para(&mut self, p: Node) {
        let mut buf = String::new();
        for c in p.children() {
            if c.is_element() && BLOCKS_IN_P.contains(&name(c)) {
                let t = std::mem::take(&mut buf);
                self.b.paragraph(&t);
                self.block(c, 1);
            } else {
                self.inline(c, &mut buf);
            }
        }
        self.b.paragraph(&buf);
    }

    /// Everything inside `n` as one line of inline text.
    fn inline_text(&self, n: Node) -> String {
        let mut s = String::new();
        for c in n.children() {
            self.inline(c, &mut s);
        }
        collapse_ws(&s)
    }

    fn inline(&self, n: Node, out: &mut String) {
        if n.is_text() {
            out.push_str(n.text().unwrap_or(""));
            return;
        }
        if !n.is_element() {
            return;
        }
        match name(n) {
            "sup" => script('^', &self.inline_text(n), out),
            "sub" => script('_', &self.inline_text(n), out),
            "inline-formula"
            | "tex-math"
            | "math"
            | "disp-formula"
            | "chem-struct"
            | "inline-supplementary-material" => {
                if let Some(l) = formula_latex(n) {
                    out.push('$');
                    out.push_str(&l);
                    out.push('$');
                } else {
                    out.push_str(&text_of(n));
                }
            }
            "alternatives" => {
                if let Some(l) = formula_latex(n) {
                    out.push('$');
                    out.push_str(&l);
                    out.push('$');
                } else if let Some(first) =
                    n.children().find(|c| c.is_element() && name(*c) != "graphic" && name(*c) != "inline-graphic")
                {
                    self.inline(first, out);
                }
            }
            "break" | "br" => out.push(' '),
            "ext-link" | "uri" | "email" => {
                let t = self.inline_text(n);
                if t.is_empty() {
                    out.push_str(attr(n, "href").unwrap_or(""));
                } else {
                    out.push_str(&t);
                }
            }
            "fig" | "fig-group" | "table-wrap" => {
                let cap = child(n, "caption");
                let label = text_of_child(n, "label");
                out.push(' ');
                out.push_str(&self.caption_text(label.as_deref(), cap));
                out.push(' ');
            }
            "target" | "index-term" | "object-id" | "inline-graphic" | "graphic" => {}
            _ => {
                for c in n.children() {
                    self.inline(c, out);
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------------ constructs

    fn display_formula(&mut self, n: Node) {
        match formula_latex(n) {
            Some(l) => self.b.display_math(&l),
            None => {
                let t = text_of(n);
                if !t.is_empty() {
                    self.b.paragraph(&t);
                } else {
                    self.warnings.push("a display formula had no MathML or TeX content".to_string());
                }
            }
        }
    }

    fn caption_text(&self, label: Option<&str>, caption: Option<Node>) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(l) = label.map(collapse_ws).filter(|l| !l.is_empty()) {
            parts.push(if l.ends_with(['.', ':']) { l } else { format!("{l}.") });
        }
        if let Some(cap) = caption {
            if let Some(t) = child(cap, "title") {
                let t = self.inline_text(t);
                if !t.is_empty() {
                    parts.push(if t.ends_with(['.', '?', '!', ':']) { t } else { format!("{t}.") });
                }
            }
            for p in cap.children().filter(|c| is(*c, "p")) {
                let t = self.inline_text(p);
                if !t.is_empty() {
                    parts.push(t);
                }
            }
            if child(cap, "title").is_none() && !cap.children().any(|c| is(c, "p")) {
                let t = self.inline_text(cap);
                if !t.is_empty() {
                    parts.push(t);
                }
            }
        }
        parts.join(" ")
    }

    fn figure(&mut self, f: Node) {
        let label = text_of_child(f, "label");
        let text = self.caption_text(label.as_deref(), child(f, "caption"));
        self.b.paragraph(&text);
    }

    fn table_wrap(&mut self, tw: Node) {
        let label = text_of_child(tw, "label");
        let text = self.caption_text(label.as_deref(), child(tw, "caption"));
        self.b.paragraph(&text);
        let table = child(tw, "table")
            .or_else(|| child(tw, "alternatives").and_then(|a| child(a, "table")))
            .or_else(|| descendant(tw, "table"));
        match table {
            Some(t) => self.table(t),
            None => {
                self.warnings.push(format!("table {} has no <table> (image only)", label.as_deref().unwrap_or("?")))
            }
        }
        if let Some(foot) = child(tw, "table-wrap-foot") {
            for k in foot.children().filter(Node::is_element) {
                match name(k) {
                    "fn" => self.footnote(k),
                    "title" | "label" => {}
                    _ => self.block(k, 1),
                }
            }
        }
    }

    fn table(&mut self, t: Node) {
        let g = self.grid(t, t);
        g.emit(&mut self.b);
    }

    fn grid(&self, _owner: Node, table: Node) -> Grid {
        let mut g = Grid::default();
        let mut header_rows = 0usize;
        let mut push_rows = |section: Node, is_head: bool, g: &mut Grid| {
            for tr in children_named(section, "tr") {
                let mut row = Vec::new();
                for c in tr.children().filter(|c| is(*c, "td") || is(*c, "th")) {
                    let span = |a: &str| attr(c, a).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1);
                    row.push(Cell::new(self.inline_text(c), span("colspan"), span("rowspan")));
                }
                if is_head {
                    header_rows += 1;
                }
                g.rows.push(row);
            }
        };
        let sections: Vec<Node> = table.children().filter(Node::is_element).collect();
        let has_sections = sections.iter().any(|s| matches!(name(*s), "thead" | "tbody" | "tfoot"));
        if has_sections {
            for s in &sections {
                match name(*s) {
                    "thead" => push_rows(*s, true, &mut g),
                    "tbody" | "tfoot" => push_rows(*s, false, &mut g),
                    _ => {}
                }
            }
        } else {
            push_rows(table, false, &mut g);
            // No <thead>: a first row made only of <th> is the header.
            if let Some(first) = children_named(table, "tr").next()
                && first.children().filter(Node::is_element).all(|c| is(c, "th"))
                && first.children().any(|c| c.is_element())
            {
                header_rows = 1;
            }
        }
        g.header_rows = header_rows;
        g
    }

    fn list(&mut self, l: Node, depth: usize) {
        let ordered = matches!(
            attr(l, "list-type"),
            Some("order" | "alpha-lower" | "alpha-upper" | "roman-lower" | "roman-upper")
        );
        if depth == 0 {
            self.b.break_list();
        }
        let mut n = 0;
        for item in children_named(l, "list-item") {
            n += 1;
            let label = text_of_child(item, "label").unwrap_or_default();
            let mut text = String::new();
            let mut nested: Vec<Node> = Vec::new();
            for k in item.children().filter(Node::is_element) {
                match name(k) {
                    "label" => {}
                    "list" => nested.push(k),
                    _ => {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&self.inline_text(k));
                    }
                }
            }
            let bullet_label = label.is_empty() || label.chars().all(|c| !c.is_alphanumeric());
            let (marker, text) = if ordered && bullet_label {
                (format!("{n}."), text)
            } else if !bullet_label {
                ("-".to_string(), format!("{label} {text}"))
            } else {
                ("-".to_string(), text)
            };
            self.b.list_item(depth, &marker, &text);
            for nl in nested {
                self.list(nl, depth + 1);
            }
        }
    }

    fn def_list(&mut self, dl: Node) {
        self.b.break_list();
        if let Some(t) = child(dl, "title") {
            let t = self.inline_text(t);
            self.b.paragraph(&t);
        }
        for item in children_named(dl, "def-item") {
            let term = child(item, "term").map(|t| self.inline_text(t)).unwrap_or_default();
            let def = child(item, "def").map(|d| self.inline_text(d)).unwrap_or_default();
            let line = if term.is_empty() { def } else { format!("{term}: {def}") };
            self.b.list_item(0, "-", &line);
        }
    }

    fn footnote(&mut self, f: Node) {
        let label = text_of_child(f, "label").unwrap_or_default();
        let mut text = String::new();
        for k in f.children().filter(Node::is_element) {
            if name(k) != "label" {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&self.inline_text(k));
            }
        }
        if text.is_empty() {
            return;
        }
        self.b.paragraph(&if label.is_empty() { text } else { format!("[{label}] {text}") });
    }

    // ---------------------------------------------------------------------------------------- references

    fn ref_list(&mut self, rl: Node) {
        for k in rl.children().filter(Node::is_element) {
            match name(k) {
                "ref" => self.reference(k),
                "ref-list" => {
                    if let Some(t) = child(k, "title") {
                        let t = self.inline_text(t);
                        self.b.heading(3, &t);
                    }
                    self.ref_list(k);
                }
                "title" | "label" => {}
                _ => {}
            }
        }
    }

    fn reference(&mut self, r: Node) {
        let label = text_of_child(r, "label");
        let cit = ["mixed-citation", "element-citation", "nlm-citation", "citation"].iter().find_map(|n| child(r, n));
        let (raw, title) = match cit {
            Some(c) if name(c) == "mixed-citation" => (self.inline_text(c), cite_title(c).map(|t| self.inline_text(t))),
            Some(c) => (self.format_element_citation(c), cite_title(c).map(|t| self.inline_text(t))),
            None => (self.inline_text(r), None),
        };
        let raw = strip_leading_label(&raw, label.as_deref());
        if raw.is_empty() {
            return;
        }
        let mut entry = ids::reference_from_raw(&raw, label.clone());
        entry.title = title.filter(|t| !t.is_empty());
        if let Some(c) = cit {
            for id in c.descendants().filter(|d| is(*d, "pub-id") || is(*d, "ext-link")) {
                let v = text_of(id);
                let kind = attr(id, "pub-id-type").or(attr(id, "ext-link-type")).unwrap_or("");
                match kind {
                    "doi" => entry.doi = ids::find_doi(&v).or(entry.doi.take()),
                    "pmid" => {
                        entry.pmid = Some(v).filter(|v| v.chars().all(|c| c.is_ascii_digit())).or(entry.pmid.take())
                    }
                    "pmcid" | "pmc" => entry.pmcid = ids::normalize_pmcid(&v).or(entry.pmcid.take()),
                    "arxiv" => {
                        entry.arxiv_id = ids::bare_arxiv(&v).or_else(|| ids::find_arxiv(&v)).or(entry.arxiv_id.take())
                    }
                    _ => {}
                }
            }
        }
        let line = match &label {
            Some(l) => format!("[{l}] {raw}"),
            None => raw,
        };
        self.b.paragraph(&line);
        self.refs.push(entry);
    }

    /// A readable citation from the parts of an `element-citation`.
    fn format_element_citation(&self, c: Node) -> String {
        let mut names: Vec<String> = Vec::new();
        for pg in children_named(c, "person-group") {
            if matches!(attr(pg, "person-group-type"), Some("editor" | "translator"))
                && children_named(c, "person-group").count() > 1
            {
                continue;
            }
            for n in pg.children().filter(Node::is_element) {
                match name(n) {
                    "name" => {
                        let sur = text_of_child(n, "surname").unwrap_or_default();
                        let given = text_of_child(n, "given-names").unwrap_or_default();
                        names.push(format!("{sur} {given}").trim().to_string());
                    }
                    "string-name" | "collab" => names.push(text_of(n)),
                    "etal" => names.push("et al.".to_string()),
                    _ => {}
                }
            }
        }
        let mut s = String::new();
        let join = |names: &[String]| -> String { names.join(", ") };
        if !names.is_empty() {
            s.push_str(&join(&names));
        }
        if let Some(y) = text_of_child(c, "year") {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&format!("({y})"));
        }
        let sentence = |s: &mut String, part: String| {
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
        let title = cite_title(c).map(|t| self.inline_text(t)).unwrap_or_default();
        sentence(&mut s, title);
        let source = text_of_child(c, "source").unwrap_or_default();
        let mut vol = String::new();
        if let Some(v) = text_of_child(c, "volume") {
            vol.push_str(&v);
        }
        if let Some(i) = text_of_child(c, "issue") {
            vol.push_str(&format!("({i})"));
        }
        let pages = match (text_of_child(c, "fpage"), text_of_child(c, "lpage")) {
            (Some(a), Some(b)) => format!("{a}-{b}"),
            (Some(a), None) => a,
            _ => text_of_child(c, "elocation-id").unwrap_or_default(),
        };
        let mut src = source;
        if !vol.is_empty() {
            src = format!("{src} {vol}").trim().to_string();
        }
        if !pages.is_empty() {
            src = if vol.is_empty() { format!("{src} {pages}") } else { format!("{src}:{pages}") }.trim().to_string();
        }
        sentence(&mut s, src);
        for tag in ["publisher-loc", "publisher-name", "edition"] {
            if let Some(v) = text_of_child(c, tag) {
                sentence(&mut s, v);
            }
        }
        for id in children_named(c, "pub-id") {
            let v = text_of(id);
            match attr(id, "pub-id-type") {
                Some("doi") => sentence(&mut s, format!("doi:{v}")),
                Some("pmid") => sentence(&mut s, format!("PMID {v}")),
                Some("pmcid") | Some("pmc") => sentence(&mut s, v),
                _ => {}
            }
        }
        for l in children_named(c, "ext-link") {
            sentence(&mut s, text_of(l));
        }
        if let Some(cm) = text_of_child(c, "comment") {
            sentence(&mut s, cm);
        }
        s
    }
}

fn cite_title<'a, 'i>(c: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    ["article-title", "chapter-title", "data-title", "source"].iter().find_map(|n| child(c, n))
}

fn strip_leading_label(raw: &str, label: Option<&str>) -> String {
    let Some(l) = label else { return raw.to_string() };
    let bracket = format!("[{l}]");
    for p in [bracket.as_str(), l, &format!("{l}.")] {
        if let Some(rest) = raw.strip_prefix(p)
            && (rest.starts_with(' ') || p == bracket)
        {
            return rest.trim_start().to_string();
        }
    }
    raw.to_string()
}

fn is_block_name(n: &str) -> bool {
    matches!(
        n,
        "p" | "sec"
            | "list"
            | "fig"
            | "table-wrap"
            | "disp-formula"
            | "def-list"
            | "disp-quote"
            | "boxed-text"
            | "preformat"
            | "code"
            | "statement"
            | "fn"
    )
}

fn join_label(label: &str, title: &str) -> String {
    match (label.is_empty(), title.is_empty()) {
        (true, _) => title.to_string(),
        (false, true) => label.to_string(),
        (false, false) if title.starts_with(label) => title.to_string(),
        (false, false) => format!("{label} {title}"),
    }
}

/// `^2`, `_i` for short plain scripts and `^{...}`/`_{...}` for anything else.
pub(crate) fn script(mark: char, text: &str, out: &mut String) {
    let t = text.trim();
    if t.is_empty() {
        return;
    }
    let simple = t.chars().count() <= 6
        && t.chars().all(|c| c.is_alphanumeric() || matches!(c, '*' | '†' | '‡' | '§' | '¶' | '+' | '-' | '−' | ','));
    out.push(mark);
    if simple {
        out.push_str(t);
    } else {
        out.push('{');
        out.push_str(t);
        out.push('}');
    }
}

/// LaTeX for a formula element: its `tex-math`, else its MathML, preferring the author's TeX.
pub(crate) fn formula_latex(n: Node) -> Option<String> {
    if name(n) == "math" {
        return mathml::to_latex(&mathml::from_xml(n));
    }
    if name(n) == "tex-math" {
        let t = mathml::clean_tex(&raw_text(n));
        return (!t.is_empty()).then_some(t);
    }
    if let Some(t) = n.descendants().find(|d| is(*d, "tex-math")) {
        let t = mathml::clean_tex(&raw_text(t));
        if !t.is_empty() {
            return Some(t);
        }
    }
    n.descendants().find(|d| is(*d, "math")).and_then(|m| mathml::to_latex(&mathml::from_xml(m)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;

    const ARTICLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE article PUBLIC "-//NLM//DTD JATS (Z39.96) Journal Archiving and Interchange DTD with MathML3 v1.2 20190208//EN" "JATS-archivearticle1-mathml3.dtd">
<article xmlns:mml="http://www.w3.org/1998/Math/MathML" xmlns:xlink="http://www.w3.org/1999/xlink" article-type="research-article" xml:lang="en">
<front>
<journal-meta><journal-title-group><journal-title>Journal of Synthetic Results</journal-title></journal-title-group></journal-meta>
<article-meta>
<article-id pub-id-type="doi">10.1234/JSR.2020.001</article-id>
<article-id pub-id-type="pmc">7654321</article-id>
<article-id pub-id-type="arxiv">2001.01234v2</article-id>
<title-group><article-title>On the <italic>Mean</italic> of Things</article-title><subtitle>A synthetic study</subtitle></title-group>
<contrib-group>
<contrib contrib-type="author"><name><surname>Lee</surname><given-names>Ann B.</given-names></name></contrib>
<contrib contrib-type="author"><name><surname>Chen</surname><given-names>Bo</given-names></name></contrib>
<contrib contrib-type="author"><collab>The Test Consortium</collab></contrib>
<contrib contrib-type="editor"><name><surname>Ed</surname><given-names>Itor</given-names></name></contrib>
</contrib-group>
<pub-date pub-type="ppub"><year>2021</year></pub-date>
<pub-date pub-type="epub"><year>2020</year></pub-date>
<abstract><p>We study the mean <inline-formula><mml:math><mml:mover><mml:mi>x</mml:mi><mml:mo>¯</mml:mo></mml:mover></mml:math></inline-formula> of samples.</p></abstract>
<kwd-group><kwd>mean</kwd><kwd>variance</kwd></kwd-group>
</article-meta>
</front>
<body>
<sec id="s1"><label>1</label><title>Introduction</title>
<p>Water is H<sub>2</sub>O and energy scales as m c<sup>2</sup>. See <xref ref-type="bibr" rid="B1">[1]</xref> and <ext-link ext-link-type="uri" xlink:href="https://example.org/x">this site</ext-link>.</p>
<p>Before the formula.
<disp-formula id="e1"><label>(1)</label><alternatives><mml:math display="block"><mml:mi>E</mml:mi><mml:mo>=</mml:mo><mml:mi>m</mml:mi><mml:msup><mml:mi>c</mml:mi><mml:mn>2</mml:mn></mml:msup></mml:math><tex-math id="t1">\begin{document}$$E=mc^{2}$$\end{document}</tex-math></alternatives></disp-formula>
After the formula we continue.</p>
<sec><title>Methods</title>
<p>Inline math: <inline-formula><tex-math id="t2">$\alpha_i$</tex-math></inline-formula> and a list:</p>
<list list-type="order"><list-item><p>first</p></list-item><list-item><p>second</p><list list-type="bullet"><list-item><p>nested</p></list-item></list></list-item></list>
<table-wrap id="t3"><label>Table 1</label><caption><title>Results</title><p>All values are means.</p></caption>
<table><thead><tr><th>Group</th><th>Mean</th></tr></thead><tbody><tr><td>A | B</td><td>1.5</td></tr><tr><td>C</td><td>2.5</td></tr></tbody></table>
<table-wrap-foot><fn><p>Note about the table.</p></fn></table-wrap-foot></table-wrap>
<fig id="f1"><label>Figure 1</label><caption><title>The setup</title><p>A schematic.</p></caption><graphic xlink:href="f1.png"/></fig>
<code language="python">for i in range(3):
    print(i)</code>
</sec>
</sec>
<sec><title>Results &amp; Discussion</title><p>Done.</p></sec>
</body>
<back>
<ack><title>Acknowledgements</title><p>Thanks to all.</p></ack>
<ref-list><title>References</title>
<ref id="B1"><label>1</label><mixed-citation publication-type="journal">Smith J, Doe A. A study of things. J Stuff. 2019;5:1-10. doi:10.5555/abc.123 <pub-id pub-id-type="pmid">12345678</pub-id></mixed-citation></ref>
<ref id="B2"><label>2</label><element-citation publication-type="journal"><person-group person-group-type="author"><name><surname>Vaswani</surname><given-names>A</given-names></name><etal/></person-group><year>2017</year><article-title>Attention is all you need</article-title><source>NeurIPS</source><volume>30</volume><fpage>5998</fpage><lpage>6008</lpage><pub-id pub-id-type="arxiv">1706.03762</pub-id></element-citation></ref>
</ref-list>
</back>
</article>"#;

    fn run() -> Extracted {
        convert(ARTICLE.as_bytes(), &ConvertOptions::default()).unwrap()
    }

    #[test]
    fn metadata_from_front_matter() {
        let r = run();
        let m = &r.metadata;
        assert_eq!(m.title.as_deref(), Some("On the Mean of Things: A synthetic study"));
        assert_eq!(m.authors, vec!["Ann B. Lee", "Bo Chen", "The Test Consortium"]);
        assert_eq!(m.doi.as_deref(), Some("10.1234/jsr.2020.001"));
        assert_eq!(m.pmcid.as_deref(), Some("PMC7654321"));
        assert_eq!(m.arxiv_id.as_deref(), Some("2001.01234"));
        assert_eq!(m.year, Some(2020));
        assert_eq!(m.language.as_deref(), Some("en"));
        assert_eq!(m.keywords, vec!["mean", "variance"]);
        assert_eq!(m.journal.as_deref(), Some("Journal of Synthetic Results"));
        assert!(m.abstract_text.as_deref().unwrap().starts_with("We study the mean $\\bar{x}$"));
        assert!(r.tool.starts_with("vault-extract ") && r.tool.ends_with(" jats"));
        assert_eq!(r.encoding.as_deref(), Some("utf-8"));
    }

    #[test]
    fn sections_become_headings_with_lines_that_match() {
        let r = run();
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(
            hs,
            vec![
                (1, "On the Mean of Things: A synthetic study"),
                (2, "Abstract"),
                (2, "1 Introduction"),
                (3, "Methods"),
                (2, "Results & Discussion"),
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
    fn inline_formatting_and_math() {
        let md = run().markdown;
        assert!(md.contains("Water is H_2O and energy scales as m c^2. See [1] and this site."), "{md}");
        assert!(md.contains("Inline math: $\\alpha_i$ and a list:"), "{md}");
    }

    #[test]
    fn display_formula_splits_the_paragraph_and_prefers_tex() {
        let r = run();
        assert!(
            r.markdown.contains("Before the formula.\n\n$$\nE=mc^{2}\n$$\n\nAfter the formula we continue."),
            "{}",
            r.markdown
        );
        let eq = r.blocks.iter().find(|b| b.kind == BlockKind::Equation).unwrap();
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert_eq!(lines[eq.start as usize - 1], "$$");
        assert_eq!(lines[eq.end as usize - 1], "$$");
    }

    #[test]
    fn lists_tables_captions_and_code() {
        let r = run();
        let md = &r.markdown;
        assert!(md.contains("\n1. first\n2. second\n  - nested\n"), "{md}");
        assert!(md.contains("Table 1. Results. All values are means.\n\n| Group | Mean |\n| --- | --- |\n| A \\| B | 1.5 |\n| C | 2.5 |\n"), "{md}");
        assert!(md.contains("[?] Note about the table.") || md.contains("Note about the table."), "{md}");
        assert!(md.contains("Figure 1. The setup. A schematic."), "{md}");
        assert!(md.contains("```python\nfor i in range(3):\n    print(i)\n```"), "{md}");
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Table).count(), 1);
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Code).count(), 1);
    }

    #[test]
    fn references_are_structured_and_one_per_line() {
        let r = run();
        assert_eq!(r.references.len(), 2);
        let a = &r.references[0];
        assert_eq!(a.label.as_deref(), Some("1"));
        assert_eq!(a.doi.as_deref(), Some("10.5555/abc.123"));
        assert_eq!(a.pmid.as_deref(), Some("12345678"));
        assert_eq!(a.year, Some(2019));
        let b = &r.references[1];
        assert_eq!(b.arxiv_id.as_deref(), Some("1706.03762"));
        assert_eq!(b.year, Some(2017));
        assert_eq!(b.title.as_deref(), Some("Attention is all you need"));
        assert!(
            b.raw.starts_with("Vaswani A, et al. (2017) Attention is all you need. NeurIPS 30:5998-6008"),
            "{}",
            b.raw
        );
        let refs_line = r.headings.iter().find(|h| h.text == "References").unwrap().line as usize;
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert!(lines[refs_line + 1].starts_with("[1] Smith J, Doe A."), "{:?}", &lines[refs_line..]);
    }

    #[test]
    fn named_entities_without_a_dtd_file_still_parse() {
        let src = "<article><front><article-meta><title-group><article-title>A&nbsp;B &mdash; C</article-title></title-group></article-meta></front><body><p>x&nbsp;y</p></body></article>";
        let r = convert(src.as_bytes(), &ConvertOptions::default()).unwrap();
        assert_eq!(r.metadata.title.as_deref(), Some("A B — C"));
    }

    #[test]
    fn body_table_with_spans_falls_back_to_html() {
        let src = r#"<article><body><table-wrap><label>Table 2</label><table><tr><th>a</th><th>b</th></tr><tr><td rowspan="2">x</td><td>1</td></tr><tr><td>2</td></tr></table></table-wrap></body></article>"#;
        let r = convert(src.as_bytes(), &ConvertOptions::default()).unwrap();
        assert!(r.markdown.contains("```html\n<table>"), "{}", r.markdown);
        assert_eq!(r.blocks[0].kind, BlockKind::Table);
    }

    #[test]
    fn article_set_and_unexpected_roots() {
        let set = "<pmc-articleset><article><front><article-meta><title-group><article-title>One</article-title></title-group></article-meta></front></article><article><front/></article></pmc-articleset>";
        let r = convert(set.as_bytes(), &ConvertOptions::default()).unwrap();
        assert_eq!(r.metadata.title.as_deref(), Some("One"));
        assert!(r.warnings.iter().any(|w| w.contains("2 articles")));
        assert!(convert(b"<other/>", &ConvertOptions::default()).is_err());
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
    }

    #[test]
    fn bits_book_parts() {
        let src = r#"<book><book-meta><book-title-group><book-title>The Book</book-title></book-title-group><contrib-group><contrib contrib-type="author"><name><surname>Writer</surname><given-names>W</given-names></name></contrib></contrib-group><isbn>978-0-306-40615-7</isbn><pub-date><year>1999</year></pub-date></book-meta>
<book-body><book-part book-part-type="chapter"><book-part-meta><title-group><label>1</label><title>Start</title></title-group></book-part-meta><body><p>Chapter text.</p></body></book-part></book-body></book>"#;
        let r = convert(src.as_bytes(), &ConvertOptions::default()).unwrap();
        assert_eq!(r.metadata.title.as_deref(), Some("The Book"));
        assert_eq!(r.metadata.isbn, vec!["9780306406157"]);
        assert_eq!(r.metadata.year, Some(1999));
        assert!(r.markdown.contains("## 1 Start\n\nChapter text."), "{}", r.markdown);
    }

    #[test]
    fn footnote_group_definition_list_and_quote() {
        let src = r#"<article><body><p>Text.</p>
<def-list><def-item><term>Alpha</term><def><p>First letter.</p></def></def-item></def-list>
<disp-quote><p>To be or not to be.</p><attrib>Anonymous</attrib></disp-quote></body>
<back><fn-group><fn id="fn1"><label>1</label><p>A footnote.</p></fn></fn-group></back></article>"#;
        let r = convert(src.as_bytes(), &ConvertOptions::default()).unwrap();
        assert!(r.markdown.contains("- Alpha: First letter."), "{}", r.markdown);
        assert!(r.markdown.contains("> To be or not to be.\n\n> -- Anonymous"), "{}", r.markdown);
        assert!(r.markdown.contains("## Notes\n\n[1] A footnote."), "{}", r.markdown);
    }

    #[test]
    fn script_helper() {
        let mut s = String::new();
        script('^', "2", &mut s);
        script('_', "i,j", &mut s);
        script('^', "long text here", &mut s);
        assert_eq!(s, "^2_i,j^{long text here}");
    }
}

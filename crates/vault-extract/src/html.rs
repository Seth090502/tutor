//! HTML and XHTML to canonical-ready Markdown, including arXiv's LaTeXML output (MathML `alttext` becomes LaTeX,
//! equation tables become display maths, the bibliography becomes structured references) and EPUB chapters
//! (page-break markers become pages).

use std::collections::HashMap;
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Node, Selector};

use crate::ConvertOptions;
use crate::builder::{MdBuilder, collapse_ws};
use crate::error::{ExtractError, Result};
use crate::ids;
use crate::jats::script;
use crate::mathml;
use crate::tables::{Cell, Grid};
use crate::text;
use crate::types::{Extracted, Metadata, Reference, tool_string};

/// Elements nested deeper than this are flattened to text, so a hostile page cannot exhaust the stack.
const MAX_DEPTH: usize = 64;

static SEL_META: LazyLock<Selector> = LazyLock::new(|| Selector::parse("meta").expect("static selector"));
static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("title").expect("static selector"));
static SEL_HTML: LazyLock<Selector> = LazyLock::new(|| Selector::parse("html").expect("static selector"));
static SEL_BODY: LazyLock<Selector> = LazyLock::new(|| Selector::parse("body").expect("static selector"));
static SEL_ARTICLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("article, main, [role=main], .ltx_page_content").expect("static selector"));
static SEL_H1: LazyLock<Selector> = LazyLock::new(|| Selector::parse("h1").expect("static selector"));

/// Settings for one HTML document.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HtmlOptions {
    /// Added to every heading level (an EPUB chapter's `h1` sits below the book title).
    pub heading_offset: i8,
}

pub fn convert(bytes: &[u8], opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut warnings = Vec::new();
    let (src, encoding) = decode_html(bytes, opts, &mut warnings);
    check_depth(&src)?;
    let src = if looks_like_xhtml(&src) { strip_mathml_prefix(&expand_self_closing(&src)) } else { src };
    let doc = Html::parse_document(&src);
    let mut meta = read_metadata(&doc);
    let root = content_root(&doc);

    let mut b = MdBuilder::new();
    let has_h1 = root.select(&SEL_H1).next().is_some();
    if !has_h1
        && let Some(t) = &meta.title {
            b.heading(1, t);
        }
    let mut w = Walker::new(&mut b, HtmlOptions::default());
    w.run(root);
    let (refs, mut warn, authors, abstract_text) = w.into_parts();
    warnings.append(&mut warn);
    if meta.authors.is_empty() {
        meta.authors = authors;
    }
    if meta.abstract_text.is_none() {
        meta.abstract_text = abstract_text;
    }
    let f = b.finish();
    if meta.title.is_none() {
        meta.title = f.headings.iter().find(|h| h.level == 1).map(|h| h.text.clone());
    }
    Ok(Extracted {
        markdown: f.markdown,
        headings: f.headings,
        pages: f.pages,
        blocks: f.blocks,
        metadata: meta,
        references: refs,
        warnings,
        tool: tool_string("html"),
        encoding: Some(encoding),
    })
}

/// XHTML (an XML declaration or an XHTML or EPUB namespace). In it `<span/>` is an empty element, but an HTML
/// parser ignores the slash and would swallow everything after it.
pub(crate) fn looks_like_xhtml(src: &str) -> bool {
    let head: String = src.chars().take(2048).collect();
    head.trim_start().starts_with("<?xml") || head.contains("w3.org/1999/xhtml") || head.contains("idpf.org/2007/ops")
}

static SELF_CLOSING: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)<([A-Za-z][A-Za-z0-9:\-]*)(\s[^<>]*?)?/>").expect("static regex"));

/// XHTML that binds MathML to a prefix (`<m:math xmlns:m="...">`) loses its maths in an HTML parser, which does
/// not know prefixes. Drop the prefix from the tags of every prefix bound to the MathML namespace.
pub(crate) fn strip_mathml_prefix(src: &str) -> String {
    static DECL: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"xmlns:([A-Za-z_][\w.\-]*)\s*=\s*["']http://www\.w3\.org/1998/Math/MathML["']"#).expect("static regex"));
    let mut out = src.to_string();
    let prefixes: Vec<String> = DECL.captures_iter(src).map(|c| c[1].to_string()).collect();
    for p in prefixes {
        out = out.replace(&format!("<{p}:"), "<").replace(&format!("</{p}:"), "</");
        out = out.replace(&format!("xmlns:{p}="), "data-stripped-xmlns=");
    }
    out
}

/// Turn `<name attrs/>` into `<name attrs></name>` for elements that are not void in HTML.
pub(crate) fn expand_self_closing(src: &str) -> String {
    SELF_CLOSING
        .replace_all(src, |c: &regex::Captures| {
            let name = &c[1];
            let attrs = c.get(2).map_or("", |m| m.as_str());
            if matches!(name.to_ascii_lowercase().as_str(), "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta" | "param" | "source" | "track" | "wbr") {
                c[0].to_string()
            } else {
                format!("<{name}{attrs}></{name}>")
            }
        })
        .into_owned()
}

/// Deepest element nesting accepted before parsing. Real pages stay under about 60; the HTML parser slows
/// quadratically on deeper input.
pub(crate) const MAX_HTML_DEPTH: usize = 1500;

/// Reject hostile nesting before the parser sees it. The estimate counts only elements that need an end tag, so
/// optional end tags (`li`, `p`, `td`...) cannot inflate it.
pub(crate) fn check_depth(src: &str) -> Result<()> {
    let depth = nesting_depth(src);
    if depth > MAX_HTML_DEPTH {
        tracing::warn!(depth, "refusing hostile HTML nesting");
        return Err(ExtractError::Limits(format!("html element nesting is about {depth} levels deep (limit {MAX_HTML_DEPTH})")));
    }
    Ok(())
}

fn nesting_depth(src: &str) -> usize {
    let b = src.as_bytes();
    let (mut depth, mut max, mut i) = (0usize, 0usize, 0usize);
    while let Some(p) = src[i..].find('<') {
        let at = i + p;
        i = at + 1;
        let rest = &b[i..];
        if rest.starts_with(b"!--") {
            i = src[i..].find("-->").map(|e| i + e + 3).unwrap_or(b.len());
            continue;
        }
        let closing = rest.first() == Some(&b'/');
        let name_start = if closing { i + 1 } else { i };
        let name: String = src[name_start.min(src.len())..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let nests = matches!(
            name.as_str(),
            "div" | "span" | "section" | "article" | "aside" | "nav" | "header" | "footer" | "main" | "ul" | "ol" | "table" | "blockquote" | "a" | "b" | "i" | "em" | "strong" | "font" | "center" | "form" | "figure" | "pre" | "code" | "details" | "dl" | "fieldset" | "small" | "big" | "u" | "s" | "sub" | "sup" | "label" | "mark" | "cite"
        );
        if !nests {
            continue;
        }
        if closing {
            depth = depth.saturating_sub(1);
        } else {
            let end = src[i..].find('>').map(|e| i + e).unwrap_or(b.len());
            if !(end > 0 && b.get(end - 1) == Some(&b'/')) {
                depth += 1;
                max = max.max(depth);
            }
        }
    }
    max
}

/// Decode: valid UTF-8 first, then a `<meta charset>` declaration, then detection.
pub(crate) fn decode_html(bytes: &[u8], opts: &ConvertOptions, warnings: &mut Vec<String>) -> (String, String) {
    let bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]) || bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    if !bom && std::str::from_utf8(bytes).is_err()
        && let Some(enc) = meta_charset(bytes) {
            let (t, had) = enc.decode_without_bom_handling(bytes);
            if had {
                warnings.push(format!("some bytes were not valid {} and became U+FFFD", enc.name()));
            }
            return (t.into_owned(), enc.name().to_ascii_lowercase());
        }
    let d = text::decode_for_conversion(bytes, opts, warnings);
    (d.text, d.encoding.to_string())
}

fn meta_charset(bytes: &[u8]) -> Option<&'static encoding_rs::Encoding> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_ascii_lowercase();
    let i = head.find("charset")?;
    let rest = head[i + 7..].trim_start().strip_prefix('=')?.trim_start().trim_start_matches(['"', '\'']);
    let label: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':')).collect();
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// The element holding the document's content: an `article`, `main` or LaTeXML content region when there is
/// one, else the `body`, else the whole document.
fn content_root(doc: &Html) -> ElementRef<'_> {
    let mut best: Option<(usize, ElementRef)> = None;
    for el in doc.select(&SEL_ARTICLE) {
        let len: usize = el.text().map(|t| t.trim().len()).sum();
        if best.is_none_or(|(l, _)| len > l) {
            best = Some((len, el));
        }
    }
    let body_len: usize = doc.select(&SEL_BODY).next().map(|b| b.text().map(|t| t.trim().len()).sum()).unwrap_or(0);
    match best {
        // An article that holds most of the page text is the content; a small one is a teaser inside a larger page.
        Some((len, el)) if len * 2 >= body_len => el,
        _ => doc.select(&SEL_BODY).next().or_else(|| doc.select(&SEL_HTML).next()).unwrap_or_else(|| doc.root_element()),
    }
}

/// The smallest heading level used in `root`, if any.
pub(crate) fn min_heading_level(root: ElementRef) -> Option<u8> {
    root.descendants().filter_map(ElementRef::wrap).filter_map(|e| {
        let n = tag(e).as_bytes();
        (n.len() == 2 && n[0] == b'h' && (b'1'..=b'6').contains(&n[1])).then(|| n[1] - b'0')
    }).min()
}

/// Parse one XHTML chapter: self-closing tags expanded, nesting checked.
pub(crate) fn parse_chapter(src: &str) -> Result<Html> {
    check_depth(src)?;
    let src = if looks_like_xhtml(src) { strip_mathml_prefix(&expand_self_closing(src)) } else { src.to_string() };
    Ok(Html::parse_document(&src))
}

/// The element a chapter's text lives in.
pub(crate) fn chapter_body(doc: &Html) -> ElementRef<'_> {
    doc.select(&SEL_BODY).next().unwrap_or_else(|| doc.root_element())
}

fn read_metadata(doc: &Html) -> Metadata {
    let mut m = Metadata::default();
    let mut dc_title = None;
    let mut og_title = None;
    for el in doc.select(&SEL_META) {
        let v = el.value();
        let name = v.attr("name").or_else(|| v.attr("property")).unwrap_or("").to_ascii_lowercase();
        let content = collapse_ws(v.attr("content").unwrap_or(""));
        if content.is_empty() {
            continue;
        }
        match name.as_str() {
            "citation_title" => m.title = Some(content),
            "dc.title" | "dcterms.title" => dc_title = Some(content),
            "og:title" => og_title = Some(content),
            "citation_author" | "dc.creator" | "dcterms.creator" | "author" | "article:author" => {
                if !m.authors.contains(&content) {
                    m.authors.push(content);
                }
            }
            "citation_doi" | "prism.doi" => m.doi = ids::find_doi(&content).or(m.doi.take()),
            "dc.identifier" | "dcterms.identifier" => {
                m.doi = m.doi.take().or_else(|| ids::find_doi(&content));
                if let Some(a) = ids::find_arxiv(&content) {
                    m.arxiv_id.get_or_insert(a);
                }
            }
            "citation_arxiv_id" => m.arxiv_id = ids::bare_arxiv(&content).or_else(|| ids::find_arxiv(&content)).or(m.arxiv_id.take()),
            "citation_pmid" => {}
            "citation_isbn" | "dc.relation.ispartof" if name == "citation_isbn" => {
                if let Some(i) = ids::normalize_isbn(&content) {
                    m.isbn.push(i);
                }
            }
            "citation_publication_date" | "citation_date" | "citation_online_date" | "dc.date" | "dcterms.date" | "article:published_time" | "prism.publicationdate" => {
                if m.year.is_none() {
                    m.year = ids::year_in(&content);
                }
            }
            "citation_journal_title" | "prism.publicationname" => m.journal = Some(content),
            "keywords" | "citation_keywords" => {
                for k in content.split([',', ';']).map(str::trim).filter(|k| !k.is_empty()) {
                    if !m.keywords.iter().any(|x| x == k) {
                        m.keywords.push(k.to_string());
                    }
                }
            }
            "citation_language" | "dc.language" | "dcterms.language" => m.language = Some(content),
            "citation_abstract" => m.abstract_text = Some(content),
            _ => {}
        }
    }
    if m.title.is_none() {
        m.title = dc_title.or(og_title).or_else(|| doc.select(&SEL_TITLE).next().map(|t| collapse_ws(&t.text().collect::<String>())).filter(|t| !t.is_empty()));
    }
    if m.language.is_none() {
        m.language = doc.select(&SEL_HTML).next().and_then(|h| h.value().attr("lang").or_else(|| h.value().attr("xml:lang"))).map(str::to_string).filter(|l| !l.is_empty());
    }
    m
}

// ---------------------------------------------------------------------------------------------------- walker

/// Converts an element tree into the builder. Also used by the EPUB converter for each spine document.
pub(crate) struct Walker<'b> {
    b: &'b mut MdBuilder,
    opts: HtmlOptions,
    refs: Vec<Reference>,
    warnings: Vec<String>,
    authors: Vec<String>,
    abstract_text: Option<String>,
    /// Level of the heading that opened a references section, while inside one.
    in_refs: Option<u8>,
    pending_notes: Vec<String>,
    /// Element ids that start a page, with the printed label (from an EPUB page list).
    page_targets: HashMap<String, String>,
    /// Footnote and endnote ids that a note reference points at, with the reference's label.
    noterefs: HashMap<String, String>,
}

fn tag<'a>(el: ElementRef<'a>) -> &'a str {
    el.value().name()
}

fn has_class(el: ElementRef, c: &str) -> bool {
    el.value().classes().any(|x| x == c)
}

fn class_contains(el: ElementRef, frag: &str) -> bool {
    el.value().classes().any(|x| x.contains(frag))
}

fn is_page_break(el: ElementRef) -> bool {
    let v = el.value();
    v.attr("epub:type").is_some_and(|t| t.split_whitespace().any(|x| x == "pagebreak")) || v.attr("role") == Some("doc-pagebreak")
}

fn page_label(el: ElementRef) -> Option<String> {
    let v = el.value();
    let from_attr = v.attr("title").or_else(|| v.attr("aria-label")).map(collapse_ws).filter(|s| !s.is_empty());
    from_attr
        .or_else(|| Some(collapse_ws(&el.text().collect::<String>())).filter(|s| !s.is_empty()))
        .or_else(|| {
            v.attr("id").map(|i| i.trim_start_matches(|c: char| c.is_alphabetic() || c == '_' || c == '-').to_string()).filter(|s| !s.is_empty())
        })
}

fn is_skipped(el: ElementRef) -> bool {
    let t = tag(el);
    if t == "aside" {
        // Notes written as asides (EPUB 3 footnotes) are content; other asides are page furniture.
        let v = el.value();
        let note = v.attr("epub:type").is_some_and(|t| t.contains("note")) || v.attr("role").is_some_and(|r| r.starts_with("doc-") && r.contains("note"));
        return !note;
    }
    if matches!(
        t,
        "script" | "style" | "noscript" | "template" | "head" | "meta" | "link" | "title" | "nav" | "footer" | "form" | "iframe" | "svg" | "canvas" | "object" | "embed" | "audio" | "video" | "select" | "button" | "input" | "textarea" | "dialog" | "base"
    ) {
        return !(t == "script" && matches!(el.value().attr("type"), Some(ty) if ty.starts_with("math/tex")));
    }
    let v = el.value();
    if v.attr("hidden").is_some() || v.attr("role") == Some("navigation") || v.attr("role") == Some("doc-toc") {
        return true;
    }
    if v.attr("style").is_some_and(|s| {
        let s: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
        s.contains("display:none")
    }) {
        return true;
    }
    if t.starts_with("mjx-") && !matches!(t, "mjx-container" | "mjx-assistive-mml") {
        return true;
    }
    has_class(el, "katex-html")
        || has_class(el, "MathJax_Preview")
        || class_contains(el, "ltx_page_header")
        || class_contains(el, "ltx_page_footer")
        || has_class(el, "ltx_pagination")
        || has_class(el, "ltx_rdf")
        || has_class(el, "ltx_role_affiliation")
        || has_class(el, "ltx_role_email")
        || has_class(el, "ltx_role_address")
}

fn is_block_tag(t: &str) -> bool {
    matches!(
        t,
        "address" | "article" | "blockquote" | "details" | "dd" | "div" | "dl" | "dt" | "fieldset" | "figcaption" | "figure" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "header" | "hgroup" | "hr" | "li" | "main" | "ol" | "p" | "pre" | "section" | "table" | "ul" | "summary" | "center" | "body" | "html" | "caption" | "menu" | "dir" | "tbody" | "thead" | "tfoot" | "tr"
    )
}

impl<'b> Walker<'b> {
    pub(crate) fn new(b: &'b mut MdBuilder, opts: HtmlOptions) -> Self {
        Self { b, opts, refs: Vec::new(), warnings: Vec::new(), authors: Vec::new(), abstract_text: None, in_refs: None, pending_notes: Vec::new(), page_targets: HashMap::new(), noterefs: HashMap::new() }
    }

    pub(crate) fn with_page_targets(mut self, targets: HashMap<String, String>) -> Self {
        self.page_targets = targets;
        self
    }

    fn level(&self, h: u8) -> u8 {
        (h as i16 + self.opts.heading_offset as i16).clamp(1, 6) as u8
    }

    pub(crate) fn into_parts(self) -> (Vec<Reference>, Vec<String>, Vec<String>, Option<String>) {
        (self.refs, self.warnings, self.authors, self.abstract_text)
    }

    pub(crate) fn run(&mut self, root: ElementRef) {
        self.noterefs = note_references(root);
        self.flow(root, 0);
    }

    /// Children of a container: inline runs become paragraphs, block children are dispatched.
    fn flow(&mut self, el: ElementRef, depth: usize) {
        if depth > MAX_DEPTH {
            let t = collapse_ws(&el.text().collect::<String>());
            self.b.paragraph(&t);
            return;
        }
        let mut buf = String::new();
        let mut prev_br = false;
        for c in el.children() {
            match c.value() {
                Node::Text(t) => {
                    if !t.trim().is_empty() {
                        prev_br = false;
                    }
                    buf.push_str(t);
                }
                Node::Element(_) => {
                    let Some(ce) = ElementRef::wrap(c) else { continue };
                    if is_skipped(ce) {
                        continue;
                    }
                    if is_page_break(ce) {
                        self.flush(&mut buf);
                        self.b.page_break(page_label(ce));
                        continue;
                    }
                    if let Some(label) = ce.value().attr("id").and_then(|id| self.page_targets.get(id)).cloned() {
                        self.flush(&mut buf);
                        self.b.page_break(Some(label));
                    }
                    // A footnote body that a note reference points at: "[^label]: text".
                    if let Some(label) = ce.value().attr("id").and_then(|id| self.noterefs.get(id)).cloned()
                        && is_note_body(ce) {
                            self.flush(&mut buf);
                            let t = self.inline_text(ce, depth + 1);
                            self.b.paragraph(&format!("[^{label}]: {t}"));
                            continue;
                        }
                    let t = tag(ce);
                    if t == "br" {
                        if prev_br {
                            self.flush(&mut buf);
                        } else {
                            buf.push(' ');
                        }
                        prev_br = true;
                        continue;
                    }
                    prev_br = false;
                    if is_block_tag(t) || is_display_math(ce) {
                        self.flush(&mut buf);
                        self.block(ce, depth + 1);
                    } else {
                        self.inline(ce, &mut buf, depth + 1);
                    }
                }
                _ => {}
            }
        }
        self.flush(&mut buf);
    }

    /// Write the buffered inline text as a paragraph (or a reference entry inside a references section).
    fn flush(&mut self, buf: &mut String) {
        let text = collapse_ws(buf);
        buf.clear();
        if !text.is_empty() {
            if self.in_refs.is_some() {
                self.reference(&text, None);
            } else {
                self.b.paragraph(&text);
            }
        }
        for n in std::mem::take(&mut self.pending_notes) {
            self.b.paragraph(&format!("[note] {n}"));
        }
    }

    fn reference(&mut self, text: &str, label: Option<String>) {
        let raw = strip_label(text, label.as_deref());
        let r = ids::reference_from_raw(&raw, label.clone());
        if r.raw.is_empty() {
            return;
        }
        let line = match &label {
            Some(l) => format!("[{l}] {}", r.raw),
            None => r.raw.clone(),
        };
        self.b.paragraph(&line);
        self.refs.push(r);
    }

    fn block(&mut self, el: ElementRef, depth: usize) {
        let t = tag(el);
        match t {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.heading(el),
            "p" => self.flow(el, depth),
            "address" | "summary" | "dt" | "figcaption" | "caption" | "hgroup" => {
                let s = self.inline_text(el, depth);
                if !s.is_empty() {
                    self.b.paragraph(&s);
                }
                self.drain_notes();
            }
            "hr" => {}
            "pre" => self.pre(el),
            "ul" | "ol" | "menu" | "dir" => self.list(el, 0, depth),
            "dl" => self.def_list(el, depth),
            "blockquote" => self.quote(el, depth),
            "table" => self.table(el, depth),
            "figure" => self.figure(el, depth),
            "math" => self.display_math(el),
            "details" => self.flow(el, depth),
            _ => {
                if has_class(el, "ltx_abstract") {
                    self.abstract_block(el, depth);
                } else if has_class(el, "ltx_authors") || has_class(el, "ltx_creator") && has_class(el, "ltx_role_author") {
                    self.authors_block(el);
                } else if class_contains(el, "ltx_equation") || has_class(el, "ltx_eqn_table") {
                    self.equation_container(el);
                } else if has_class(el, "ltx_bibliography") {
                    self.flow(el, depth);
                    self.in_refs = None;
                } else if has_class(el, "ltx_biblist") {
                    self.biblist(el);
                } else {
                    self.flow(el, depth);
                }
            }
        }
    }

    fn drain_notes(&mut self) {
        for n in std::mem::take(&mut self.pending_notes) {
            self.b.paragraph(&format!("[note] {n}"));
        }
    }

    fn heading(&mut self, el: ElementRef) {
        let level = tag(el).as_bytes()[1] - b'0';
        let text = self.inline_text(el, 0);
        // A new heading at the same or a higher level ends a references section.
        let eff = self.level(level);
        if let Some(open) = self.in_refs
            && eff <= open {
                self.in_refs = None;
            }
        if is_references_title(&text) {
            self.in_refs = Some(eff);
        }
        self.b.heading(eff, &text);
    }

    fn pre(&mut self, el: ElementRef) {
        let code = el.children().filter_map(ElementRef::wrap).find(|c| tag(*c) == "code");
        let lang = code
            .into_iter()
            .chain(std::iter::once(el))
            .flat_map(|e| e.value().classes().collect::<Vec<_>>())
            .find_map(|c| c.strip_prefix("language-").or_else(|| c.strip_prefix("lang-")).or_else(|| c.strip_prefix("brush:")))
            .unwrap_or("")
            .to_string();
        let raw: String = el.text().collect();
        self.b.code_block(&lang, &raw);
    }

    fn list(&mut self, el: ElementRef, depth: usize, hdepth: usize) {
        if hdepth > MAX_DEPTH {
            return;
        }
        if has_class(el, "ltx_biblist") {
            return self.biblist(el);
        }
        let ordered = tag(el) == "ol";
        if depth == 0 {
            self.b.break_list();
        }
        let start: usize = el.value().attr("start").and_then(|s| s.trim().parse().ok()).unwrap_or(1);
        let mut n = start;
        for li in el.children().filter_map(ElementRef::wrap).filter(|c| tag(*c) == "li") {
            let mut text = String::new();
            let mut nested: Vec<ElementRef> = Vec::new();
            for c in li.children() {
                match c.value() {
                    Node::Text(t) => text.push_str(t),
                    Node::Element(_) => {
                        let Some(ce) = ElementRef::wrap(c) else { continue };
                        if is_skipped(ce) {
                            continue;
                        }
                        if matches!(tag(ce), "ul" | "ol") {
                            nested.push(ce);
                        } else if is_page_break(ce) {
                            self.b.page_break(page_label(ce));
                        } else if is_block_tag(tag(ce)) && !text.is_empty() {
                            text.push(' ');
                            self.inline(ce, &mut text, hdepth + 1);
                            text.push(' ');
                        } else {
                            self.inline(ce, &mut text, hdepth + 1);
                            if is_block_tag(tag(ce)) {
                                text.push(' ');
                            }
                        }
                    }
                    _ => {}
                }
            }
            let text = collapse_ws(&text);
            if self.in_refs.is_some() {
                self.reference(&text, ordered.then(|| n.to_string()));
            } else {
                let marker = if ordered { format!("{n}.") } else { "-".to_string() };
                self.b.list_item(depth, &marker, &text);
            }
            n += 1;
            for nl in nested {
                self.list(nl, depth + 1, hdepth + 1);
            }
            self.drain_notes();
        }
    }

    fn def_list(&mut self, el: ElementRef, depth: usize) {
        self.b.break_list();
        let mut term = String::new();
        for c in el.children().filter_map(ElementRef::wrap) {
            match tag(c) {
                "dt" => term = self.inline_text(c, depth + 1),
                "dd" => {
                    let def = self.inline_text(c, depth + 1);
                    let line = if term.is_empty() { def } else { format!("{term}: {def}") };
                    self.b.list_item(0, "-", &line);
                    term.clear();
                }
                _ => {}
            }
        }
        if !term.is_empty() {
            self.b.list_item(0, "-", &term);
        }
    }

    fn quote(&mut self, el: ElementRef, depth: usize) {
        let blocks: Vec<ElementRef> = el.children().filter_map(ElementRef::wrap).filter(|c| matches!(tag(*c), "p" | "div" | "ul" | "ol")).collect();
        if blocks.is_empty() {
            let t = self.inline_text(el, depth + 1);
            self.b.quote(&t);
        } else {
            for p in blocks {
                let t = self.inline_text(p, depth + 1);
                self.b.quote(&t);
            }
        }
    }

    fn figure(&mut self, el: ElementRef, depth: usize) {
        for c in el.children().filter_map(ElementRef::wrap) {
            if tag(c) == "figcaption" {
                let t = self.inline_text(c, depth + 1);
                self.b.paragraph(&t);
            }
        }
        for c in el.children().filter_map(ElementRef::wrap) {
            if tag(c) != "figcaption" && !is_skipped(c) {
                if is_block_tag(tag(c)) {
                    self.block(c, depth + 1);
                } else if tag(c) == "math" {
                    self.display_math(c);
                }
            }
        }
    }

    fn display_math(&mut self, el: ElementRef) {
        match mathml::to_latex(&mathml::from_scraper(el)) {
            Some(l) => self.b.display_math(&l),
            None => {
                let t = collapse_ws(&el.text().collect::<String>());
                self.b.paragraph(&t);
            }
        }
    }

    /// A LaTeXML equation table or group: every `math` becomes a display equation; equation numbers are dropped.
    fn equation_container(&mut self, el: ElementRef) {
        let maths: Vec<ElementRef> = el.descendants().filter_map(ElementRef::wrap).filter(|e| tag(*e) == "math").collect();
        if maths.is_empty() {
            self.flow(el, 1);
            return;
        }
        for m in maths {
            self.display_math(m);
        }
    }

    fn table(&mut self, el: ElementRef, depth: usize) {
        if class_contains(el, "ltx_eqn") || class_contains(el, "ltx_equation") {
            return self.equation_container(el);
        }
        let mut g = Grid::default();
        let mut header_rows = 0usize;
        let mut caption = None;
        let mut rows: Vec<(ElementRef, bool)> = Vec::new();
        for c in el.children().filter_map(ElementRef::wrap) {
            match tag(c) {
                "caption" => caption = Some(self.inline_text(c, depth + 1)),
                "thead" => rows.extend(c.children().filter_map(ElementRef::wrap).filter(|r| tag(*r) == "tr").map(|r| (r, true))),
                "tbody" | "tfoot" => rows.extend(c.children().filter_map(ElementRef::wrap).filter(|r| tag(*r) == "tr").map(|r| (r, false))),
                "tr" => rows.push((c, false)),
                _ => {}
            }
        }
        for (i, (tr, in_head)) in rows.iter().enumerate() {
            let cells: Vec<ElementRef> = tr.children().filter_map(ElementRef::wrap).filter(|c| matches!(tag(*c), "td" | "th")).collect();
            let all_th = !cells.is_empty() && cells.iter().all(|c| tag(*c) == "th");
            if *in_head || (all_th && i == header_rows) {
                header_rows += 1;
            }
            let row = cells
                .iter()
                .map(|c| {
                    let span = |a: &str| c.value().attr(a).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1);
                    Cell::new(self.inline_text(*c, depth + 1), span("colspan"), span("rowspan"))
                })
                .collect();
            g.rows.push(row);
        }
        g.header_rows = header_rows;
        if let Some(c) = caption.filter(|c| !c.is_empty()) {
            self.b.paragraph(&c);
        }
        g.emit(self.b);
    }

    // ------------------------------------------------------------------------------------------- LaTeXML

    fn abstract_block(&mut self, el: ElementRef, depth: usize) {
        let mut paras = Vec::new();
        self.b.heading(self.level(2), "Abstract");
        for c in el.children().filter_map(ElementRef::wrap) {
            if has_class(c, "ltx_title") || matches!(tag(c), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                continue;
            }
            let t = self.inline_text(c, depth + 1);
            if !t.is_empty() {
                paras.push(t.clone());
                self.b.paragraph(&t);
            }
        }
        if self.abstract_text.is_none() && !paras.is_empty() {
            self.abstract_text = Some(paras.join(" "));
        }
    }

    fn authors_block(&mut self, el: ElementRef) {
        let names: Vec<String> = el
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|e| has_class(*e, "ltx_personname"))
            .map(|e| collapse_ws(&e.text().collect::<String>()))
            .filter(|n| !n.is_empty())
            .collect();
        if !names.is_empty() {
            let line = names.join(", ");
            self.b.paragraph(&line);
            self.authors = names;
        }
    }

    fn biblist(&mut self, el: ElementRef) {
        for li in el.children().filter_map(ElementRef::wrap).filter(|c| tag(*c) == "li") {
            let label = li.descendants().filter_map(ElementRef::wrap).find(|e| class_contains(*e, "ltx_tag_bibitem")).map(|e| collapse_ws(&e.text().collect::<String>()));
            let mut parts: Vec<String> = Vec::new();
            for c in li.children().filter_map(ElementRef::wrap) {
                if class_contains(c, "ltx_tag_bibitem") {
                    continue;
                }
                let t = self.inline_text(c, 1);
                if !t.is_empty() {
                    parts.push(t);
                }
            }
            let text = if parts.is_empty() { collapse_ws(&li.text().collect::<String>()) } else { parts.join(" ") };
            let label = label.map(|l| l.trim_matches(['[', ']', '.', ' ']).to_string()).filter(|l| !l.is_empty());
            self.reference(&text, label);
        }
    }

    // ------------------------------------------------------------------------------------------------ inline

    fn inline_text(&mut self, el: ElementRef, depth: usize) -> String {
        let mut s = String::new();
        for c in el.children() {
            self.inline_node(c, &mut s, depth);
        }
        collapse_ws(&s)
    }

    fn inline_node(&mut self, c: ego_tree::NodeRef<Node>, out: &mut String, depth: usize) {
        match c.value() {
            Node::Text(t) => out.push_str(t),
            Node::Element(_) => {
                if let Some(ce) = ElementRef::wrap(c)
                    && !is_skipped(ce) {
                        if is_page_break(ce) {
                            // Inline page break: keep position by splitting later; record immediately.
                            self.b.page_break(page_label(ce));
                        } else {
                            self.inline(ce, out, depth + 1);
                        }
                    }
            }
            _ => {}
        }
    }

    fn inline(&mut self, el: ElementRef, out: &mut String, depth: usize) {
        if depth > MAX_DEPTH {
            out.push_str(&el.text().collect::<String>());
            return;
        }
        let t = tag(el);
        match t {
            "br" => out.push(' '),
            "a" if is_noteref(el) => {
                let label = collapse_ws(&el.text().collect::<String>());
                if !label.is_empty() {
                    out.push_str(&format!("[^{}]", label.trim_matches(['[', ']', '(', ')'])));
                }
            }
            "sup" => {
                if class_contains(el, "ltx_note_mark") {
                    return;
                }
                let s = self.inline_text(el, depth);
                script('^', &s, out);
            }
            "sub" => {
                let s = self.inline_text(el, depth);
                script('_', &s, out);
            }
            "code" | "kbd" | "samp" | "tt" => {
                let s = collapse_ws(&el.text().collect::<String>());
                if s.is_empty() {
                    return;
                }
                if s.contains('`') || s.len() > 200 {
                    out.push_str(&s);
                } else {
                    out.push('`');
                    out.push_str(&s);
                    out.push('`');
                }
            }
            "math" => match mathml::to_latex(&mathml::from_scraper(el)) {
                Some(l) => {
                    out.push('$');
                    out.push_str(&l);
                    out.push('$');
                }
                None => out.push_str(&el.text().collect::<String>()),
            },
            "script" => {
                // MathJax v2 source: <script type="math/tex">
                let ty = el.value().attr("type").unwrap_or("");
                let tex = mathml::clean_tex(&el.text().collect::<String>());
                if !tex.is_empty() {
                    if ty.contains("mode=display") {
                        out.push_str(&format!(" $${tex}$$ ").replace("$$", "$"));
                    } else {
                        out.push('$');
                        out.push_str(&tex);
                        out.push('$');
                    }
                }
            }
            "img" | "picture" | "source" | "map" | "area" => {}
            "span" | "div" | "a" | "b" | "i" | "em" | "strong" | "u" | "small" | "mark" | "abbr" | "cite" | "q" | "dfn" | "label" | "time" | "bdi" | "bdo" | "font" | "s" | "strike" | "del" | "ins" | "big" | "var" | "data" | "wbr" | "ruby" | "rb" | "section" | "article" | "p" | "li" | "ul" | "ol" | "table" | "figure" | "blockquote" => {
                if class_contains(el, "ltx_note") && !class_contains(el, "ltx_note_") {
                    // LaTeXML footnote: the text goes after the paragraph, the mark stays out.
                    let content = el.descendants().filter_map(ElementRef::wrap).find(|e| has_class(*e, "ltx_note_content"));
                    if let Some(c) = content {
                        let mut s = String::new();
                        for ch in c.children() {
                            if let Some(ce) = ElementRef::wrap(ch)
                                && (class_contains(ce, "ltx_note_mark") || class_contains(ce, "ltx_tag_note")) {
                                    continue;
                                }
                            self.inline_node(ch, &mut s, depth);
                        }
                        let s = collapse_ws(&s);
                        if !s.is_empty() {
                            self.pending_notes.push(s);
                        }
                    }
                    return;
                }
                if has_class(el, "ltx_tag") {
                    // Section and caption numbers ("Figure 1: ") keep their text.
                    for ch in el.children() {
                        self.inline_node(ch, out, depth);
                    }
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                    return;
                }
                if matches!(t, "p" | "li" | "div" | "section" | "article" | "blockquote" | "figure" | "ul" | "ol" | "table") {
                    out.push(' ');
                }
                for ch in el.children() {
                    self.inline_node(ch, out, depth);
                }
                if matches!(t, "p" | "li" | "div" | "section" | "article" | "blockquote" | "figure" | "ul" | "ol" | "table") {
                    out.push(' ');
                }
            }
            _ => {
                for ch in el.children() {
                    self.inline_node(ch, out, depth);
                }
            }
        }
    }
}

fn is_noteref(el: ElementRef) -> bool {
    let v = el.value();
    v.attr("epub:type").is_some_and(|t| t.split_whitespace().any(|x| x == "noteref")) || v.attr("role") == Some("doc-noteref")
}

fn is_note_body(el: ElementRef) -> bool {
    let v = el.value();
    v.attr("epub:type").is_some_and(|t| t.split_whitespace().any(|x| matches!(x, "footnote" | "endnote" | "rearnote" | "note")))
        || v.attr("role").is_some_and(|r| matches!(r, "doc-footnote" | "doc-endnote"))
}

/// Note references in `root`: target id to the reference's label.
fn note_references(root: ElementRef) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for a in root.descendants().filter_map(ElementRef::wrap).filter(|e| tag(*e) == "a" && is_noteref(*e)) {
        let label = collapse_ws(&a.text().collect::<String>());
        let label = label.trim_matches(['[', ']', '(', ')']).to_string();
        if let (Some(href), false) = (a.value().attr("href"), label.is_empty())
            && let Some((_, id)) = href.split_once('#') {
                m.entry(id.to_string()).or_insert(label);
            }
    }
    m
}

/// A `display="block"` MathML element standing on its own in a flow of text.
fn is_display_math(el: ElementRef) -> bool {
    tag(el) == "math" && el.value().attr("display") == Some("block")
}

fn is_references_title(t: &str) -> bool {
    let l = t.trim().trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
    // Drop a leading section number ("7 References") already handled by trim; compare the words.
    let l = l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
    matches!(l, "references" | "reference" | "bibliography" | "works cited" | "literature cited" | "references and notes" | "sources" | "citations")
}

fn strip_label(raw: &str, label: Option<&str>) -> String {
    let Some(l) = label else { return raw.to_string() };
    for p in [format!("[{l}]"), format!("{l}."), l.to_string()] {
        if let Some(rest) = raw.strip_prefix(&p)
            && (rest.starts_with(' ') || p.starts_with('[')) {
                return rest.trim_start().to_string();
            }
    }
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;

    fn run(src: &str) -> Extracted {
        convert(src.as_bytes(), &ConvertOptions::default()).unwrap()
    }

    #[test]
    fn plain_html_structure() {
        let r = run(
            r#"<!DOCTYPE html><html lang="fr"><head><meta charset="utf-8"><title>Page Title</title>
<meta name="citation_author" content="Ann Lee"><meta name="citation_author" content="Bo Chen">
<meta name="citation_doi" content="doi:10.1234/ABC.5"><meta name="citation_publication_date" content="2020/05/01">
<meta name="keywords" content="alpha, beta"><script>var x = 1;</script><style>p{}</style></head>
<body><nav><a href="/">Home</a></nav>
<h1>Main Title</h1><p>First <em>emphasised</em> and <strong>bold</strong> text with <a href="http://x.y">a link</a>.</p>
<h2>Section</h2><ul><li>one</li><li>two<ul><li>nested</li></ul></li></ul><ol start="3"><li>third</li><li>fourth</li></ol>
<p>H<sub>2</sub>O and x<sup>2</sup>; <code>let a = 1;</code></p>
<pre><code class="language-rust">fn main() {
    println!("hi");
}</code></pre>
<blockquote><p>Quoted words.</p></blockquote>
<dl><dt>Term</dt><dd>Definition</dd></dl>
<footer>Copyright</footer></body></html>"#,
        );
        let m = &r.metadata;
        assert_eq!(m.title.as_deref(), Some("Page Title"));
        assert_eq!(m.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(m.doi.as_deref(), Some("10.1234/abc.5"));
        assert_eq!(m.year, Some(2020));
        assert_eq!(m.keywords, vec!["alpha", "beta"]);
        assert_eq!(m.language.as_deref(), Some("fr"));
        let md = &r.markdown;
        assert!(md.starts_with("# Main Title\n\nFirst emphasised and bold text with a link.\n\n## Section\n\n- one\n- two\n  - nested\n\n3. third\n4. fourth\n"), "{md}");
        assert!(md.contains("H_2O and x^2; `let a = 1;`"), "{md}");
        assert!(md.contains("```rust\nfn main() {\n    println!(\"hi\");\n}\n```"), "{md}");
        assert!(md.contains("> Quoted words."), "{md}");
        assert!(md.contains("- Term: Definition"), "{md}");
        assert!(!md.contains("Home") && !md.contains("Copyright") && !md.contains("var x"), "{md}");
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(hs, vec![(1, "Main Title"), (2, "Section")]);
    }

    #[test]
    fn title_becomes_heading_when_there_is_no_h1() {
        let r = run("<html><head><title>Only Title</title></head><body><p>Body text.</p></body></html>");
        assert!(r.markdown.starts_with("# Only Title\n\nBody text.\n"), "{}", r.markdown);
    }

    #[test]
    fn tables_with_header_caption_and_spans() {
        let r = run(
            r#"<body><table><caption>Scores</caption><thead><tr><th>Name</th><th colspan="2">Score</th></tr></thead>
<tbody><tr><td>A</td><td>1</td><td>2</td></tr><tr><td>B | C</td><td>3</td><td>4</td></tr></tbody></table>
<table><tr><td rowspan="2">x</td><td>y</td></tr><tr><td>z</td></tr></table></body>"#,
        );
        let md = &r.markdown;
        assert!(md.contains("Scores\n\n| Name | Score |  |\n| --- | --- | --- |\n| A | 1 | 2 |\n| B \\| C | 3 | 4 |\n"), "{md}");
        assert!(md.contains("```html\n<table>\n<tr><td rowspan=\"2\">x</td><td>y</td></tr>"), "{md}");
        assert_eq!(r.blocks.iter().filter(|b| b.kind == BlockKind::Table).count(), 2);
    }

    #[test]
    fn double_br_splits_paragraphs() {
        let r = run("<body>line one<br>line two<br><br>next paragraph</body>");
        assert_eq!(r.markdown, "line one line two\n\nnext paragraph\n");
    }

    #[test]
    fn mathjax_and_katex_sources() {
        let r = run(
            r#"<body><p>Euler: <script type="math/tex">e^{i\pi}+1=0</script> and
<span class="katex"><span class="katex-mathml"><math><semantics><mrow><mi>x</mi></mrow><annotation encoding="application/x-tex">x^2</annotation></semantics></math></span><span class="katex-html" aria-hidden="true">x2 duplicated</span></span> done.</p></body>"#,
        );
        assert!(r.markdown.contains("Euler: $e^{i\\pi}+1=0$ and $x^2$ done."), "{}", r.markdown);
        assert!(!r.markdown.contains("duplicated"));
    }

    const LATEXML: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>[2301.01234] A LaTeXML Paper</title>
<meta name="citation_arxiv_id" content="2301.01234v2"></head><body>
<header class="ltx_page_header"><a href="/">arXiv header</a></header>
<div class="ltx_page_main"><div class="ltx_page_content"><article class="ltx_document">
<h1 class="ltx_title ltx_title_document">A LaTeXML Paper</h1>
<div class="ltx_authors"><span class="ltx_creator ltx_role_author"><span class="ltx_personname">Ann Lee</span><span class="ltx_author_notes"><span class="ltx_role_affiliation">University X</span></span></span>
<span class="ltx_creator ltx_role_author"><span class="ltx_personname">Bo Chen</span></span></div>
<div class="ltx_abstract"><h6 class="ltx_title ltx_title_abstract">Abstract</h6><p class="ltx_p">We prove a bound.</p></div>
<section id="S1" class="ltx_section"><h2 class="ltx_title ltx_title_section"><span class="ltx_tag ltx_tag_section">1 </span>Introduction</h2>
<div id="S1.p1" class="ltx_para"><p class="ltx_p">Let <math alttext="x\in\mathbb{R}" class="ltx_Math" display="inline"><semantics><mrow><mi>x</mi><mo>∈</mo><mi>ℝ</mi></mrow><annotation-xml encoding="MathML-Content"><ci>x</ci></annotation-xml><annotation encoding="application/x-tex">x\in\mathbb{R}</annotation></semantics></math> be given <cite class="ltx_cite ltx_citemacro_citep">(<a href="#bib.bib1" class="ltx_ref">Smith et&nbsp;al., 2020</a>)</cite>.<span class="ltx_note ltx_role_footnote"><sup class="ltx_note_mark">1</sup><span class="ltx_note_outer"><span class="ltx_note_content"><sup class="ltx_note_mark">1</sup><span class="ltx_tag ltx_tag_note">1</span>A footnote.</span></span></span></p></div>
<table class="ltx_equation ltx_eqn_table"><tbody><tr class="ltx_equation ltx_eqn_row ltx_align_baseline"><td class="ltx_eqn_cell ltx_eqn_eqno ltx_align_middle ltx_align_left" rowspan="1"><span class="ltx_tag ltx_tag_equation ltx_align_left">(1)</span></td><td class="ltx_eqn_cell ltx_eqn_center_padleft"></td><td class="ltx_eqn_cell ltx_align_center"><math alttext="E=mc^{2}" class="ltx_Math" display="block"><semantics><mrow><mi>E</mi><mo>=</mo><mrow><mi>m</mi><msup><mi>c</mi><mn>2</mn></msup></mrow></mrow><annotation encoding="application/x-tex">E=mc^{2}</annotation></semantics></math></td><td class="ltx_eqn_cell ltx_eqn_center_padright"></td></tr></tbody></table>
<figure id="S1.T1" class="ltx_table"><figcaption class="ltx_caption ltx_centering"><span class="ltx_tag ltx_tag_table">Table 1: </span>Results by method.</figcaption>
<table class="ltx_tabular ltx_guessed_headers ltx_align_middle"><thead class="ltx_thead"><tr class="ltx_tr"><th class="ltx_td ltx_th ltx_th_column">Method</th><th class="ltx_td ltx_th ltx_th_column">Score</th></tr></thead><tbody class="ltx_tbody"><tr class="ltx_tr"><td class="ltx_td">Ours</td><td class="ltx_td">0.9</td></tr></tbody></table></figure>
</section>
<section id="bib" class="ltx_bibliography"><h2 class="ltx_title ltx_title_bibliography">References</h2>
<ul class="ltx_biblist"><li id="bib.bib1" class="ltx_bibitem"><span class="ltx_tag ltx_tag_bibitem">[1]</span><span class="ltx_bibblock">Smith, J., Doe, A.: A study of things. J. Stuff 5, 1–10 (2020). doi:10.5555/abc.123</span></li>
<li id="bib.bib2" class="ltx_bibitem"><span class="ltx_tag ltx_tag_bibitem">[2]</span><span class="ltx_bibblock">Vaswani, A.: Attention is all you need. arXiv:1706.03762 (2017)</span></li></ul></section>
</article></div></div><footer class="ltx_page_footer">Generated by LaTeXML</footer></body></html>"##;

    #[test]
    fn latexml_paper() {
        let r = run(LATEXML);
        let md = &r.markdown;
        assert!(!md.contains("arXiv header") && !md.contains("Generated by") && !md.contains("University X"), "{md}");
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(hs, vec![(1, "A LaTeXML Paper"), (2, "Abstract"), (2, "1 Introduction"), (2, "References")]);
        assert_eq!(r.metadata.authors, vec!["Ann Lee", "Bo Chen"]);
        assert_eq!(r.metadata.arxiv_id.as_deref(), Some("2301.01234"));
        assert_eq!(r.metadata.abstract_text.as_deref(), Some("We prove a bound."));
        // MathML alttext becomes LaTeX; citations and notes stay readable.
        assert!(md.contains("Let $x\\in\\mathbb{R}$ be given (Smith et al., 2020)."), "{md}");
        assert!(md.contains("\n\n[note] A footnote."), "{md}");
        // The equation table is one display equation without the number.
        assert!(md.contains("$$\nE=mc^{2}\n$$"), "{md}");
        assert!(!md.contains("(1)"), "{md}");
        assert!(md.contains("Table 1: Results by method.\n\n| Method | Score |\n| --- | --- |\n| Ours | 0.9 |"), "{md}");
        // Bibliography entries are structured and one per line.
        assert_eq!(r.references.len(), 2);
        assert_eq!(r.references[0].label.as_deref(), Some("1"));
        assert_eq!(r.references[0].doi.as_deref(), Some("10.5555/abc.123"));
        assert_eq!(r.references[1].arxiv_id.as_deref(), Some("1706.03762"));
        assert!(md.contains("[1] Smith, J., Doe, A.: A study of things."), "{md}");
        let lines: Vec<&str> = md.lines().collect();
        for h in &r.headings {
            assert_eq!(lines[h.line as usize - 1], format!("{} {}", "#".repeat(h.level as usize), h.text));
        }
    }

    #[test]
    fn mathml_without_alttext_is_converted() {
        let r = run(r#"<body><p>Sum <math><mrow><munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>a</mi><mi>i</mi></msub></mrow></math>.</p><math display="block"><mfrac><mn>1</mn><mn>2</mn></mfrac></math></body>"#);
        assert!(r.markdown.contains("Sum $\\sum\\limits_{i = 1}^n a_i$."), "{}", r.markdown);
        assert!(r.markdown.contains("$$\n\\frac{1}{2}\n$$"), "{}", r.markdown);
    }

    #[test]
    fn generic_reference_lists_become_entries() {
        let r = run(
            r#"<body><h1>Paper</h1><h2>Intro</h2><p>Text.</p><h2>References</h2>
<ol><li>Smith J. (2019) Things. doi:10.5555/xyz</li><li>Doe A. (2018) More things.</li></ol><h2>Appendix</h2><ol><li>item</li></ol></body>"#,
        );
        assert_eq!(r.references.len(), 2);
        assert_eq!(r.references[0].doi.as_deref(), Some("10.5555/xyz"));
        assert!(r.markdown.contains("[1] Smith J. (2019) Things."), "{}", r.markdown);
        assert!(r.markdown.contains("## Appendix\n\n1. item"), "{}", r.markdown);
    }

    #[test]
    fn epub_style_page_breaks() {
        let r = run(
            r#"<html xmlns:epub="http://www.idpf.org/2007/ops"><body><p>First page text.<span epub:type="pagebreak" id="p12" title="12"/> Continues here.</p>
<div role="doc-pagebreak" aria-label="13"></div><p>Next page.</p></body></html>"#,
        );
        let labels: Vec<_> = r.pages.iter().map(|p| p.label.clone().unwrap()).collect();
        assert_eq!(labels, vec!["12", "13"]);
        let lines: Vec<&str> = r.markdown.lines().collect();
        assert_eq!(lines[r.pages[0].line as usize - 1], "Continues here.");
        assert_eq!(lines[r.pages[1].line as usize - 1], "Next page.");
    }

    #[test]
    fn xhtml_and_declared_charset() {
        let r = run(r#"<?xml version="1.0" encoding="UTF-8"?><html xmlns="http://www.w3.org/1999/xhtml"><body><h1>Titre</h1><p>Caf&#233; et cr&egrave;me.<br/>Fin.</p></body></html>"#);
        assert!(r.markdown.contains("Café et crème. Fin."), "{}", r.markdown);
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode("<html><head><meta charset=\"windows-1252\"></head><body><p>Un café très chaud, “merci” – s'il vous plaît.</p></body></html>");
        let r = convert(&bytes, &ConvertOptions::default()).unwrap();
        assert!(r.markdown.contains("café très chaud"), "{}", r.markdown);
        assert_eq!(r.encoding.as_deref(), Some("windows-1252"));
    }

    #[test]
    fn hostile_nesting_is_refused_and_deep_but_sane_nesting_is_flattened() {
        let bomb = format!("<body>{}core text{}</body>", "<div>".repeat(20_000), "</div>".repeat(20_000));
        let err = convert(bomb.as_bytes(), &ConvertOptions::default()).unwrap_err();
        assert_eq!(err.reason_code(), "archive_limits");
        let deep = format!("<body>{}core text{}</body>", "<div>".repeat(900), "</div>".repeat(900));
        let r = run(&deep);
        assert!(r.markdown.contains("core text"), "{}", &r.markdown[..r.markdown.len().min(200)]);
        // Unclosed optional-end-tag elements do not count as nesting.
        let items = format!("<body><ul>{}</ul></body>", "<li>item".repeat(5000));
        assert!(run(&items).markdown.contains("- item"));
    }

    #[test]
    fn article_is_preferred_over_page_chrome() {
        let r = run("<body><div>menu junk here</div><article><h1>Real</h1><p>Real article text that is long enough to be the main content of the page.</p></article></body>");
        assert!(r.markdown.contains("Real article text"), "{}", r.markdown);
        assert!(!r.markdown.contains("menu junk"), "{}", r.markdown);
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
    }
}

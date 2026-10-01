//! MathML to LaTeX. The converter works on a small neutral tree so the same code serves roxmltree documents (JATS,
//! TEI, EPUB, ODT) and scraper trees (HTML). When the source already carries LaTeX (`alttext` from LaTeXML, a
//! `application/x-tex` annotation) that text is used verbatim.

/// Elements nested deeper than this are flattened to their text.
const MAX_DEPTH: usize = 40;

#[derive(Debug, Clone, Default)]
pub(crate) struct MNode {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<MKid>,
}

#[derive(Debug, Clone)]
pub(crate) enum MKid {
    Text(String),
    El(MNode),
}

impl MNode {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn elems(&self) -> Vec<&MNode> {
        self.kids
            .iter()
            .filter_map(|k| match k {
                MKid::El(e) => Some(e),
                MKid::Text(_) => None,
            })
            .collect()
    }

    fn text(&self) -> String {
        let mut s = String::new();
        for k in &self.kids {
            match k {
                MKid::Text(t) => s.push_str(t),
                MKid::El(e) => s.push_str(&e.text()),
            }
        }
        s
    }
}

pub(crate) fn from_xml(n: roxmltree::Node) -> MNode {
    fn go(n: roxmltree::Node, depth: usize) -> MNode {
        let mut m = MNode {
            name: n.tag_name().name().to_string(),
            attrs: n.attributes().map(|a| (a.name().to_string(), a.value().to_string())).collect(),
            kids: Vec::new(),
        };
        for c in n.children() {
            if c.is_element() {
                if depth < MAX_DEPTH {
                    m.kids.push(MKid::El(go(c, depth + 1)));
                } else {
                    let flat: String = c.descendants().filter(|d| d.is_text()).filter_map(|d| d.text()).collect();
                    m.kids.push(MKid::El(MNode { name: "mtext".into(), attrs: Vec::new(), kids: vec![MKid::Text(flat)] }));
                }
            } else if c.is_text() {
                m.kids.push(MKid::Text(c.text().unwrap_or("").to_string()));
            }
        }
        m
    }
    go(n, 0)
}

pub(crate) fn from_scraper(e: scraper::ElementRef) -> MNode {
    fn go(e: scraper::ElementRef, depth: usize) -> MNode {
        let mut m = MNode {
            name: e.value().name().to_string(),
            attrs: e.value().attrs().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            kids: Vec::new(),
        };
        for c in e.children() {
            if let Some(ce) = scraper::ElementRef::wrap(c) {
                if depth < MAX_DEPTH {
                    m.kids.push(MKid::El(go(ce, depth + 1)));
                } else {
                    let flat: String = ce.text().collect();
                    m.kids.push(MKid::El(MNode { name: "mtext".into(), attrs: Vec::new(), kids: vec![MKid::Text(flat)] }));
                }
            } else if let Some(t) = c.value().as_text() {
                m.kids.push(MKid::Text(t.to_string()));
            }
        }
        m
    }
    go(e, 0)
}

/// Whether the text reads as TeX rather than a natural-language description.
fn looks_like_tex(alt: &str) -> bool {
    let t = alt.trim();
    !t.is_empty() && (t.contains(['\\', '^', '_', '{', '}']) || t.chars().count() <= 2)
}

/// LaTeX for a `math` element, or `None` when it is empty.
pub(crate) fn to_latex(m: &MNode) -> Option<String> {
    let found = find_tex_annotation(m);
    let out = if let Some(tex) = found {
        clean_tex(&tex)
    } else if let Some(alt) = m.attr("alttext").filter(|a| looks_like_tex(a)) {
        clean_tex(alt)
    } else {
        tidy(&conv(m, 0))
    };
    (!out.trim().is_empty()).then_some(out)
}

fn find_tex_annotation(m: &MNode) -> Option<String> {
    for e in m.elems() {
        if e.name == "annotation" {
            let enc = e.attr("encoding").unwrap_or("").to_ascii_lowercase();
            if enc.contains("tex") && !enc.contains("html") {
                let t = e.text();
                if !t.trim().is_empty() {
                    return Some(t);
                }
            }
        }
        if e.name == "semantics" {
            if let Some(t) = find_tex_annotation(e) {
                return Some(t);
            }
        }
    }
    None
}

/// Convert a MathML fragment given as XML text (a `math` element, namespaced or not).
pub fn mathml_to_latex(xml: &str) -> Option<String> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    to_latex(&from_xml(doc.root_element()))
}

/// Strip the wrappers TeX sources put around a formula: `\begin{document}`, `$`, `$$`, `\[ \]`, `\( \)`, and the
/// preamble, and collapse whitespace.
pub(crate) fn clean_tex(src: &str) -> String {
    let mut s = src.trim();
    if let Some(i) = s.find("\\begin{document}") {
        s = &s[i + "\\begin{document}".len()..];
        if let Some(j) = s.rfind("\\end{document}") {
            s = &s[..j];
        }
    }
    let mut s = s.trim();
    loop {
        let before = s;
        for (open, close) in [("$$", "$$"), ("$", "$"), ("\\[", "\\]"), ("\\(", "\\)")] {
            if s.len() >= open.len() + close.len() && s.starts_with(open) && s.ends_with(close) {
                s = s[open.len()..s.len() - close.len()].trim();
            }
        }
        if std::ptr::eq(before, s) || before == s {
            break;
        }
    }
    tidy(s)
}

/// Collapse whitespace and drop blank lines inside a formula.
fn tidy(s: &str) -> String {
    let joined: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.replace("$$", "")
}

// ---------------------------------------------------------------------------------------------- conversion

fn conv(n: &MNode, depth: usize) -> String {
    if depth > MAX_DEPTH {
        return escape_text(&n.text());
    }
    let kids = n.elems();
    let d = depth + 1;
    match n.name.as_str() {
        "math" | "mstyle" | "mpadded" | "merror" | "mrow" | "none" => row(&kids, d),
        "mphantom" => String::new(),
        "semantics" => kids.iter().find(|k| !matches!(k.name.as_str(), "annotation" | "annotation-xml")).map(|k| conv(k, d)).unwrap_or_default(),
        "annotation" | "annotation-xml" | "mglyph" | "mprescripts" => String::new(),
        "maction" => kids.first().map(|k| conv(k, d)).unwrap_or_default(),
        "mi" => ident(n),
        "mn" => n.text().trim().to_string(),
        "mo" => op(&n.text()),
        "mtext" => text_run(&n.text()),
        "ms" => text_run(&format!("\"{}\"", n.text())),
        "mspace" => space(n),
        "mfrac" if kids.len() >= 2 => {
            let (a, b) = (conv(kids[0], d), conv(kids[1], d));
            let no_rule = n.attr("linethickness").is_some_and(|t| t.trim().trim_end_matches(|c: char| c.is_alphabetic()).parse::<f32>() == Ok(0.0));
            if no_rule {
                format!("\\binom{{{a}}}{{{b}}}")
            } else {
                format!("\\frac{{{a}}}{{{b}}}")
            }
        }
        "msqrt" => format!("\\sqrt{{{}}}", row(&kids, d)),
        "mroot" if kids.len() >= 2 => format!("\\sqrt[{}]{{{}}}", conv(kids[1], d), conv(kids[0], d)),
        "msub" if kids.len() >= 2 => format!("{}_{}", base(kids[0], d), group(&conv(kids[1], d))),
        "msup" if kids.len() >= 2 => format!("{}^{}", base(kids[0], d), group(&conv(kids[1], d))),
        "msubsup" if kids.len() >= 3 => format!("{}_{}^{}", base(kids[0], d), group(&conv(kids[1], d)), group(&conv(kids[2], d))),
        "mover" if kids.len() >= 2 => over(kids[0], kids[1], d),
        "munder" if kids.len() >= 2 => under(kids[0], kids[1], d),
        "munderover" if kids.len() >= 3 => {
            let b = conv(kids[0], d);
            if is_big_operator(kids[0]) {
                format!("{b}\\limits_{}^{}", group(&conv(kids[1], d)), group(&conv(kids[2], d)))
            } else {
                format!("\\overset{{{}}}{{\\underset{{{}}}{{{b}}}}}", conv(kids[2], d), conv(kids[1], d))
            }
        }
        "mmultiscripts" => multiscripts(&kids, d),
        "mfenced" => fenced(n, &kids, d),
        "mtable" => table(n, &kids, d),
        "mtr" | "mlabeledtr" => kids.iter().map(|c| conv(c, d)).collect::<Vec<_>>().join(" & "),
        "mtd" => row(&kids, d),
        "menclose" => {
            let inner = row(&kids, d);
            if n.attr("notation").is_some_and(|x| x.contains("box")) { format!("\\boxed{{{inner}}}") } else { inner }
        }
        _ => {
            if kids.is_empty() { escape_text(n.text().trim()) } else { row(&kids, d) }
        }
    }
}

/// A row of siblings, with matrix and cases environments recognised by their fences.
fn row(kids: &[&MNode], d: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < kids.len() {
        // fence, mtable, optional closing fence
        if kids[i].name == "mo" && i + 1 < kids.len() && kids[i + 1].name == "mtable" {
            let open = kids[i].text();
            let open = open.trim();
            let close_node = kids.get(i + 2).filter(|k| k.name == "mo");
            let close = close_node.map(|k| k.text()).unwrap_or_default();
            let close = close.trim();
            let env = match (open, close) {
                ("(", ")") => Some("pmatrix"),
                ("[", "]") => Some("bmatrix"),
                ("|", "|") => Some("vmatrix"),
                ("‖", "‖") | ("\u{2016}", "\u{2016}") => Some("Vmatrix"),
                ("{", "") => Some("cases"),
                _ => None,
            };
            if let Some(env) = env {
                let body = table_body(kids[i + 1], d);
                parts.push(format!("\\begin{{{env}}} {body} \\end{{{env}}}"));
                i += if close_node.is_some() { 3 } else { 2 };
                continue;
            }
        }
        let p = conv(kids[i], d);
        if !p.is_empty() {
            parts.push(p);
        }
        i += 1;
    }
    parts.join(" ")
}

fn table_body(t: &MNode, d: usize) -> String {
    t.elems().iter().map(|r| conv(r, d)).collect::<Vec<_>>().join(" \\\\ ")
}

fn table(n: &MNode, _kids: &[&MNode], d: usize) -> String {
    let body = table_body(n, d);
    let aligned = n.attr("columnalign").is_some_and(|a| a.trim_start().starts_with("right"));
    let env = if aligned { "aligned" } else { "matrix" };
    format!("\\begin{{{env}}} {body} \\end{{{env}}}")
}

fn ident(n: &MNode) -> String {
    let t: String = n.text().chars().filter(|c| !('\u{2061}'..='\u{2064}').contains(c)).collect();
    let t = t.trim();
    if t.is_empty() {
        return String::new();
    }
    let variant = n.attr("mathvariant").unwrap_or("");
    let chars: Vec<char> = t.chars().collect();
    if chars.len() == 1 {
        let c = chars[0];
        if let Some((style, base)) = math_alnum(c) {
            return styled(style, &base.to_string());
        }
        if let Some(s) = symbol(c) {
            return s.to_string();
        }
        let plain = escape_char(c);
        return match variant {
            "normal" if c.is_alphabetic() => format!("\\mathrm{{{plain}}}"),
            "bold" | "bold-italic" => format!("\\mathbf{{{plain}}}"),
            "double-struck" => format!("\\mathbb{{{plain}}}"),
            "script" | "bold-script" => format!("\\mathcal{{{plain}}}"),
            "fraktur" | "bold-fraktur" => format!("\\mathfrak{{{plain}}}"),
            "sans-serif" => format!("\\mathsf{{{plain}}}"),
            "monospace" => format!("\\mathtt{{{plain}}}"),
            _ => plain,
        };
    }
    if matches!(
        t,
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "sinh" | "cosh" | "tanh" | "coth" | "arcsin" | "arccos" | "arctan" | "log" | "ln" | "lg" | "exp" | "lim" | "limsup" | "liminf" | "max" | "min" | "sup" | "inf" | "det" | "dim" | "ker" | "deg" | "gcd" | "arg" | "Pr" | "hom" | "mod"
    ) {
        return match t {
            "limsup" => "\\limsup".into(),
            "liminf" => "\\liminf".into(),
            "mod" => "\\bmod".into(),
            _ => format!("\\{t}"),
        };
    }
    // Multi-character identifiers may mix Greek letters and Latin ones.
    if chars.iter().all(|c| c.is_ascii_alphanumeric()) {
        return match variant {
            "italic" | "" if chars.len() <= 2 => t.to_string(),
            "bold" | "bold-italic" => format!("\\mathbf{{{t}}}"),
            "italic" => format!("\\mathit{{{t}}}"),
            _ => format!("\\operatorname{{{t}}}"),
        };
    }
    chars
        .iter()
        .map(|&c| symbol(c).map(str::to_string).or_else(|| math_alnum(c).map(|(s, b)| styled(s, &b.to_string()))).unwrap_or_else(|| escape_char(c)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn op(raw: &str) -> String {
    let t: String = raw.chars().filter(|c| !('\u{2061}'..='\u{2064}').contains(c)).collect();
    let t = t.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut it = t.chars();
    let (first, rest) = (it.next().unwrap_or(' '), it.as_str());
    if rest.is_empty() {
        return match first {
            '{' => "\\{".into(),
            '}' => "\\}".into(),
            '|' => "|".into(),
            '\u{2016}' => "\\|".into(),
            '\u{2212}' => "-".into(),
            c => symbol(c).map(str::to_string).unwrap_or_else(|| escape_char(c)),
        };
    }
    match t {
        "<=" => "\\leq".into(),
        ">=" => "\\geq".into(),
        "!=" => "\\neq".into(),
        "->" => "\\to".into(),
        _ => t.chars().map(|c| symbol(c).map(str::to_string).unwrap_or_else(|| escape_char(c))).collect::<Vec<_>>().join(" "),
    }
}

fn escape_char(c: char) -> String {
    match c {
        '#' | '$' | '%' | '&' | '_' => format!("\\{c}"),
        '{' => "\\{".into(),
        '}' => "\\}".into(),
        '~' => "\\sim".into(),
        '\\' => "\\backslash".into(),
        '^' => "\\hat{}".into(),
        c => c.to_string(),
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' | '}' | '$' | '%' | '&' | '#' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            c => out.push(c),
        }
    }
    out
}

fn text_run(t: &str) -> String {
    if t.trim().is_empty() {
        return if t.is_empty() { String::new() } else { "\\ ".into() };
    }
    format!("\\text{{{}}}", escape_text(t.trim()))
}

fn space(n: &MNode) -> String {
    let w = n.attr("width").unwrap_or("");
    let num: f32 = w.trim_end_matches(|c: char| c.is_alphabetic()).parse().unwrap_or(0.0);
    if w.ends_with("em") && num >= 1.5 {
        "\\qquad".into()
    } else if w.ends_with("em") && num >= 0.9 {
        "\\quad".into()
    } else if num == 0.0 && !w.is_empty() {
        String::new()
    } else {
        "\\,".into()
    }
}

/// A subscript or superscript argument: braces unless a single token.
fn group(s: &str) -> String {
    let single = s.chars().count() == 1 && s.chars().all(|c| c.is_ascii_alphanumeric());
    if single { s.to_string() } else { format!("{{{s}}}") }
}

/// The base of a script: braced when it already carries a script of its own.
fn base(n: &MNode, d: usize) -> String {
    let s = conv(n, d);
    if matches!(n.name.as_str(), "msub" | "msup" | "msubsup") || (s.contains(' ') && n.name == "mrow") {
        format!("{{{s}}}")
    } else {
        s
    }
}

fn is_big_operator(n: &MNode) -> bool {
    let t = n.text();
    let t = t.trim();
    matches!(t, "∑" | "∏" | "∐" | "⋃" | "⋂" | "⋁" | "⋀" | "⨁" | "⨂" | "⨀" | "lim" | "max" | "min" | "sup" | "inf" | "limsup" | "liminf" | "argmax" | "argmin")
        || (n.name == "mo" && matches!(n.attr("largeop"), Some("true")))
}

fn accent_command(accent: &str, single_letter_base: bool) -> Option<&'static str> {
    Some(match accent.trim() {
        "^" | "ˆ" | "\u{302}" => "\\hat",
        "ˇ" | "\u{30C}" => "\\check",
        "~" | "˜" | "\u{303}" => "\\tilde",
        "¯" | "‾" | "―" | "\u{305}" | "\u{304}" | "_" if single_letter_base => "\\bar",
        "¯" | "‾" | "―" | "\u{305}" | "\u{304}" | "_" => "\\overline",
        "→" | "\u{20D7}" => "\\vec",
        "˙" | "\u{307}" => "\\dot",
        "¨" | "\u{308}" => "\\ddot",
        "⏞" => "\\overbrace",
        "←" => "\\overleftarrow",
        "↔" => "\\overleftrightarrow",
        "´" | "\u{301}" => "\\acute",
        "`" | "\u{300}" => "\\grave",
        "˘" | "\u{306}" => "\\breve",
        _ => return None,
    })
}

fn over(b: &MNode, o: &MNode, d: usize) -> String {
    let bs = conv(b, d);
    if o.name == "mo" {
        let single = bs.chars().count() == 1;
        if let Some(cmd) = accent_command(&o.text(), single) {
            return format!("{cmd}{{{bs}}}");
        }
    }
    if is_big_operator(b) {
        return format!("{bs}\\limits^{}", group(&conv(o, d)));
    }
    format!("\\overset{{{}}}{{{bs}}}", conv(o, d))
}

fn under(b: &MNode, u: &MNode, d: usize) -> String {
    let bs = conv(b, d);
    if u.name == "mo" {
        match u.text().trim() {
            "¯" | "_" | "‾" | "―" => return format!("\\underline{{{bs}}}"),
            "⏟" => return format!("\\underbrace{{{bs}}}"),
            _ => {}
        }
    }
    if is_big_operator(b) {
        return format!("{bs}\\limits_{}", group(&conv(u, d)));
    }
    format!("\\underset{{{}}}{{{bs}}}", conv(u, d))
}

fn multiscripts(kids: &[&MNode], d: usize) -> String {
    let Some((b, rest)) = kids.split_first() else { return String::new() };
    let split = rest.iter().position(|k| k.name == "mprescripts");
    let (post, pre) = match split {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, &rest[rest.len()..]),
    };
    let pair = |s: &[&MNode]| -> String {
        let mut out = String::new();
        for c in s.chunks(2) {
            if let Some(sub) = c.first().filter(|x| x.name != "none") {
                out.push_str(&format!("_{}", group(&conv(sub, d))));
            }
            if let Some(sup) = c.get(1).filter(|x| x.name != "none") {
                out.push_str(&format!("^{}", group(&conv(sup, d))));
            }
        }
        out
    };
    let pre_s = pair(pre);
    let prefix = if pre_s.is_empty() { String::new() } else { format!("{{}}{pre_s}") };
    format!("{prefix}{}{}", base(b, d), pair(post))
}

fn fenced(n: &MNode, kids: &[&MNode], d: usize) -> String {
    let open = n.attr("open").unwrap_or("(");
    let close = n.attr("close").unwrap_or(")");
    let seps: Vec<char> = n.attr("separators").unwrap_or(",").chars().filter(|c| !c.is_whitespace()).collect();
    let mut inner = String::new();
    for (i, k) in kids.iter().enumerate() {
        if i > 0 {
            if let Some(s) = seps.get(i - 1).or(seps.last()) {
                inner.push_str(&escape_char(*s));
                inner.push(' ');
            }
        }
        inner.push_str(&conv(k, d));
    }
    let delim = |s: &str| if s.is_empty() { ".".to_string() } else { op(s) };
    format!("\\left{} {inner} \\right{}", delim(open), delim(close))
}

// ------------------------------------------------------------------------------------------ symbol tables

fn styled(style: &str, s: &str) -> String {
    match style {
        "" => s.to_string(),
        cmd => format!("\\{cmd}{{{s}}}"),
    }
}

/// Mathematical alphanumerics (U+1D400 block) and the letterlike holes: the LaTeX font command and the plain
/// character they stand for.
fn math_alnum(c: char) -> Option<(&'static str, char)> {
    let u = c as u32;
    let letterlike: Option<(&'static str, char)> = match c {
        'ℂ' => Some(("mathbb", 'C')),
        'ℍ' => Some(("mathbb", 'H')),
        'ℕ' => Some(("mathbb", 'N')),
        'ℙ' => Some(("mathbb", 'P')),
        'ℚ' => Some(("mathbb", 'Q')),
        'ℝ' => Some(("mathbb", 'R')),
        'ℤ' => Some(("mathbb", 'Z')),
        'ℬ' => Some(("mathcal", 'B')),
        'ℰ' => Some(("mathcal", 'E')),
        'ℱ' => Some(("mathcal", 'F')),
        'ℋ' => Some(("mathcal", 'H')),
        'ℐ' => Some(("mathcal", 'I')),
        'ℒ' => Some(("mathcal", 'L')),
        'ℳ' => Some(("mathcal", 'M')),
        'ℛ' => Some(("mathcal", 'R')),
        'ℭ' => Some(("mathfrak", 'C')),
        'ℌ' => Some(("mathfrak", 'H')),
        'ℨ' => Some(("mathfrak", 'Z')),
        'ℎ' => Some(("", 'h')),
        _ => None,
    };
    if letterlike.is_some() {
        return letterlike;
    }
    // (start, font command, has 26 upper + 26 lower)
    const BLOCKS: [(u32, &str); 14] = [
        (0x1D400, "mathbf"),
        (0x1D434, ""),
        (0x1D468, "boldsymbol"),
        (0x1D49C, "mathcal"),
        (0x1D4D0, "mathcal"),
        (0x1D504, "mathfrak"),
        (0x1D538, "mathbb"),
        (0x1D56C, "mathfrak"),
        (0x1D5A0, "mathsf"),
        (0x1D5D4, "mathsf"),
        (0x1D608, "mathsf"),
        (0x1D63C, "mathsf"),
        (0x1D670, "mathtt"),
        (0x1D7CE, "mathbf"),
    ];
    for (start, cmd) in BLOCKS {
        if start == 0x1D7CE {
            if (0x1D7CE..=0x1D7FF).contains(&u) {
                let digit = char::from_digit((u - 0x1D7CE) % 10, 10)?;
                return Some((cmd, digit));
            }
            continue;
        }
        if (start..start + 52).contains(&u) {
            let i = u - start;
            let ch = if i < 26 { (b'A' + i as u8) as char } else { (b'a' + (i - 26) as u8) as char };
            return Some((cmd, ch));
        }
    }
    None
}

/// LaTeX for a Greek letter or a symbol.
fn symbol(c: char) -> Option<&'static str> {
    Some(match c {
        'α' => "\\alpha",
        'β' => "\\beta",
        'γ' => "\\gamma",
        'δ' => "\\delta",
        'ε' => "\\varepsilon",
        'ϵ' => "\\epsilon",
        'ζ' => "\\zeta",
        'η' => "\\eta",
        'θ' => "\\theta",
        'ϑ' => "\\vartheta",
        'ι' => "\\iota",
        'κ' => "\\kappa",
        'λ' => "\\lambda",
        'μ' | 'µ' => "\\mu",
        'ν' => "\\nu",
        'ξ' => "\\xi",
        'π' => "\\pi",
        'ϖ' => "\\varpi",
        'ρ' => "\\rho",
        'ϱ' => "\\varrho",
        'σ' => "\\sigma",
        'ς' => "\\varsigma",
        'τ' => "\\tau",
        'υ' => "\\upsilon",
        'φ' => "\\varphi",
        'ϕ' => "\\phi",
        'χ' => "\\chi",
        'ψ' => "\\psi",
        'ω' => "\\omega",
        'Γ' => "\\Gamma",
        'Δ' => "\\Delta",
        'Θ' => "\\Theta",
        'Λ' => "\\Lambda",
        'Ξ' => "\\Xi",
        'Π' => "\\Pi",
        'Σ' => "\\Sigma",
        'Υ' => "\\Upsilon",
        'Φ' => "\\Phi",
        'Ψ' => "\\Psi",
        'Ω' => "\\Omega",
        '±' => "\\pm",
        '∓' => "\\mp",
        '×' => "\\times",
        '÷' => "\\div",
        '·' | '⋅' => "\\cdot",
        '∗' | '*' => "\\ast",
        '∘' => "\\circ",
        '∙' | '•' => "\\bullet",
        '⊕' => "\\oplus",
        '⊗' => "\\otimes",
        '⊙' => "\\odot",
        '⊖' => "\\ominus",
        '∧' => "\\wedge",
        '∨' => "\\vee",
        '¬' => "\\neg",
        '∩' => "\\cap",
        '∪' => "\\cup",
        '⊂' => "\\subset",
        '⊃' => "\\supset",
        '⊆' => "\\subseteq",
        '⊇' => "\\supseteq",
        '⊄' => "\\not\\subset",
        '⊏' => "\\sqsubset",
        '∈' => "\\in",
        '∉' => "\\notin",
        '∋' => "\\ni",
        '∅' => "\\emptyset",
        '∀' => "\\forall",
        '∃' => "\\exists",
        '∄' => "\\nexists",
        '∇' => "\\nabla",
        '∂' => "\\partial",
        '∞' => "\\infty",
        'ℓ' => "\\ell",
        'ℏ' => "\\hbar",
        'ℜ' => "\\Re",
        'ℑ' => "\\Im",
        'ℵ' => "\\aleph",
        '≤' | '⩽' => "\\leq",
        '≥' | '⩾' => "\\geq",
        '≪' => "\\ll",
        '≫' => "\\gg",
        '≠' => "\\neq",
        '≈' => "\\approx",
        '≃' => "\\simeq",
        '≅' => "\\cong",
        '≡' => "\\equiv",
        '∼' => "\\sim",
        '∝' => "\\propto",
        '⊥' | '⟂' => "\\perp",
        '∥' => "\\parallel",
        '∣' => "\\mid",
        '⊢' => "\\vdash",
        '⊨' => "\\models",
        '→' => "\\to",
        '←' => "\\leftarrow",
        '↔' => "\\leftrightarrow",
        '⇒' => "\\Rightarrow",
        '⇐' => "\\Leftarrow",
        '⇔' => "\\Leftrightarrow",
        '↦' => "\\mapsto",
        '⟶' => "\\longrightarrow",
        '⟵' => "\\longleftarrow",
        '⟹' => "\\Longrightarrow",
        '⟺' => "\\Longleftrightarrow",
        '↑' => "\\uparrow",
        '↓' => "\\downarrow",
        '∑' => "\\sum",
        '∏' => "\\prod",
        '∐' => "\\coprod",
        '∫' => "\\int",
        '∬' => "\\iint",
        '∭' => "\\iiint",
        '∮' => "\\oint",
        '⋃' => "\\bigcup",
        '⋂' => "\\bigcap",
        '⋁' => "\\bigvee",
        '⋀' => "\\bigwedge",
        '⌊' => "\\lfloor",
        '⌋' => "\\rfloor",
        '⌈' => "\\lceil",
        '⌉' => "\\rceil",
        '⟨' | '〈' | '〈' => "\\langle",
        '⟩' | '〉' | '〉' => "\\rangle",
        '…' => "\\ldots",
        '⋯' => "\\cdots",
        '⋮' => "\\vdots",
        '⋱' => "\\ddots",
        '′' => "'",
        '″' => "''",
        '°' => "^\\circ",
        '∠' => "\\angle",
        '△' => "\\triangle",
        '□' => "\\square",
        '∴' => "\\therefore",
        '∵' => "\\because",
        '†' => "\\dagger",
        '‡' => "\\ddagger",
        '√' => "\\sqrt{}",
        '≺' => "\\prec",
        '≻' => "\\succ",
        '⪯' => "\\preceq",
        '⪰' => "\\succeq",
        '⊤' => "\\top",
        '≔' => ":=",
        '\u{2212}' => "-",
        '\u{2061}' | '\u{2062}' | '\u{2063}' | '\u{2064}' => "",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(src: &str) -> String {
        mathml_to_latex(src).unwrap_or_default()
    }

    #[test]
    fn identifiers_numbers_operators() {
        assert_eq!(m("<math><mi>x</mi><mo>+</mo><mn>2</mn><mo>=</mo><mi>y</mi></math>"), "x + 2 = y");
        assert_eq!(m("<math><mi>α</mi><mo>≤</mo><mi>β</mi></math>"), "\\alpha \\leq \\beta");
        assert_eq!(m("<math><mi>sin</mi><mo>&#x2061;</mo><mi>x</mi></math>"), "\\sin x");
    }

    #[test]
    fn fractions_roots_scripts() {
        assert_eq!(m("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>"), "\\frac{1}{2}");
        assert_eq!(m("<math><msqrt><mi>x</mi></msqrt></math>"), "\\sqrt{x}");
        assert_eq!(m("<math><mroot><mi>x</mi><mn>3</mn></mroot></math>"), "\\sqrt[3]{x}");
        assert_eq!(m("<math><msup><mi>x</mi><mn>2</mn></msup></math>"), "x^2");
        assert_eq!(m("<math><msub><mi>a</mi><mrow><mi>i</mi><mi>j</mi></mrow></msub></math>"), "a_{i j}");
        assert_eq!(m("<math><msubsup><mi>x</mi><mi>i</mi><mn>2</mn></msubsup></math>"), "x_i^2");
        // A script on a script needs braces.
        assert_eq!(m("<math><msup><msub><mi>x</mi><mi>i</mi></msub><mn>2</mn></msup></math>"), "{x_i}^2");
    }

    #[test]
    fn big_operators_with_limits() {
        let s = m("<math><munderover><mo>∑</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow><mi>n</mi></munderover><msub><mi>a</mi><mi>i</mi></msub></math>");
        assert_eq!(s, "\\sum\\limits_{i = 1}^n a_i");
        assert_eq!(m("<math><munder><mi>lim</mi><mrow><mi>n</mi><mo>→</mo><mi>∞</mi></mrow></munder><mi>f</mi></math>"), "\\lim\\limits_{n \\to \\infty} f");
    }

    #[test]
    fn accents_and_decorations() {
        assert_eq!(m("<math><mover><mi>x</mi><mo>^</mo></mover></math>"), "\\hat{x}");
        assert_eq!(m("<math><mover><mi>x</mi><mo>¯</mo></mover></math>"), "\\bar{x}");
        assert_eq!(m("<math><mover><mrow><mi>a</mi><mi>b</mi></mrow><mo>¯</mo></mover></math>"), "\\overline{a b}");
        assert_eq!(m("<math><mover><mi>v</mi><mo>→</mo></mover></math>"), "\\vec{v}");
        assert_eq!(m("<math><munder><mrow><mi>a</mi><mi>b</mi></mrow><mo>⏟</mo></munder></math>"), "\\underbrace{a b}");
    }

    #[test]
    fn fenced_expressions_and_matrices() {
        assert_eq!(m("<math><mfenced><mi>a</mi><mi>b</mi></mfenced></math>"), "\\left( a, b \\right)");
        assert_eq!(m("<math><mfenced open='[' close=']'><mi>x</mi></mfenced></math>"), "\\left[ x \\right]");
        let mat = "<math><mrow><mo>(</mo><mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>0</mn></mtd></mtr><mtr><mtd><mn>0</mn></mtd><mtd><mn>1</mn></mtd></mtr></mtable><mo>)</mo></mrow></math>";
        assert_eq!(m(mat), "\\begin{pmatrix} 1 & 0 \\\\ 0 & 1 \\end{pmatrix}");
        let cases = "<math><mrow><mo>{</mo><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mtext>if</mtext></mtd></mtr></mtable></mrow></math>";
        assert_eq!(m(cases), "\\begin{cases} x & \\text{if} \\end{cases}");
    }

    #[test]
    fn text_and_special_characters() {
        assert_eq!(m("<math><mtext>for all</mtext><mi>x</mi></math>"), "\\text{for all} x");
        assert_eq!(m("<math><mtext>50% of {x}</mtext></math>"), "\\text{50\\% of \\{x\\}}");
        assert_eq!(m("<math><mo>{</mo><mi>x</mi><mo>}</mo></math>"), "\\{ x \\}");
    }

    #[test]
    fn math_alphanumerics_and_blackboard() {
        assert_eq!(m("<math><mi>ℝ</mi></math>"), "\\mathbb{R}");
        assert_eq!(m("<math><mi>𝐱</mi></math>"), "\\mathbf{x}");
        assert_eq!(m("<math><mi mathvariant='double-struck'>E</mi></math>"), "\\mathbb{E}");
        assert_eq!(m("<math><mi mathvariant='normal'>d</mi><mi>x</mi></math>"), "\\mathrm{d} x");
    }

    #[test]
    fn namespaced_mathml_and_semantics() {
        let src = "<mml:math xmlns:mml='http://www.w3.org/1998/Math/MathML'><mml:semantics><mml:mrow><mml:mi>a</mml:mi></mml:mrow><mml:annotation encoding='StarMath 5.0'>a</mml:annotation></mml:semantics></mml:math>";
        assert_eq!(m(src), "a");
    }

    #[test]
    fn tex_annotation_and_alttext_win_over_conversion() {
        let ann = "<math><semantics><mrow><mi>x</mi></mrow><annotation encoding='application/x-tex'>\\frac{x}{y}</annotation></semantics></math>";
        assert_eq!(m(ann), "\\frac{x}{y}");
        let alt = "<math alttext='\\alpha^{2}'><mi>wrong</mi></math>";
        assert_eq!(m(alt), "\\alpha^{2}");
        // A natural-language alttext (Word, screen readers) is not TeX; the structure is converted instead.
        let words = "<math alttext='x squared plus one'><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><mn>1</mn></math>";
        assert_eq!(m(words), "x^2 + 1");
    }

    #[test]
    fn empty_math_is_none() {
        assert_eq!(mathml_to_latex("<math/>"), None);
    }

    #[test]
    fn clean_tex_strips_wrappers() {
        let pmc = "\\documentclass[12pt]{minimal}\n\\usepackage{amsmath}\n\\begin{document}$$\\alpha+\\beta$$\\end{document}";
        assert_eq!(clean_tex(pmc), "\\alpha+\\beta");
        assert_eq!(clean_tex("$x^2$"), "x^2");
        assert_eq!(clean_tex("\\[ a \n =\n b \\]"), "a = b");
        assert_eq!(clean_tex("\\begin{equation} a=b \\end{equation}"), "\\begin{equation} a=b \\end{equation}");
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let mut s = String::from("<math>");
        for _ in 0..100 {
            s.push_str("<mrow>");
        }
        s.push_str("<mi>x</mi>");
        for _ in 0..100 {
            s.push_str("</mrow>");
        }
        s.push_str("</math>");
        let doc = roxmltree::Document::parse_with_options(&s, roxmltree::ParsingOptions::default());
        // roxmltree itself may refuse very deep documents; either way converting must not crash.
        if let Ok(doc) = doc {
            let out = to_latex(&from_xml(doc.root_element())).unwrap_or_default();
            assert!(out.contains('x'));
        }
    }
}

//! XML parsing and DOM helpers shared by the JATS, TEI, DOCX, ODT and EPUB converters.

use std::sync::LazyLock;

use regex::Regex;
use roxmltree::{Document, Node, ParsingOptions};

use crate::error::{ExtractError, Result};
use crate::text;

/// Upper bound on DOM nodes: a 100 MB article has a few million; more is a bomb.
pub(crate) const NODES_LIMIT: u32 = 8_000_000;

/// Decode XML bytes, honouring a BOM or the declared encoding, then falling back to detection.
pub(crate) fn decode_xml(bytes: &[u8]) -> (String, &'static str) {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).to_ascii_lowercase();
    if !bytes.starts_with(&[0xEF, 0xBB, 0xBF]) && !bytes.starts_with(&[0xFF, 0xFE]) && !bytes.starts_with(&[0xFE, 0xFF])
        && let Some(i) = head.find("encoding=") {
            let rest = &head[i + 9..];
            let q = rest.chars().next();
            if let Some(q) = q.filter(|c| *c == '"' || *c == '\'')
                && let Some(end) = rest[1..].find(q) {
                    let label = &rest[1..1 + end];
                    if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes())
                        && enc != encoding_rs::UTF_8 && enc != encoding_rs::UTF_16LE && enc != encoding_rs::UTF_16BE {
                            let (t, _) = enc.decode_without_bom_handling(bytes);
                            return (t.into_owned(), enc.name());
                        }
                }
        }
    let d = text::decode(bytes);
    (d.text, d.encoding)
}

static ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&([A-Za-z][A-Za-z0-9]*);").expect("static regex"));

/// Replace named HTML entities that XML does not define (JATS and TEI documents normally rely on their DTD for
/// them) by numeric references; any other unknown name keeps its ampersand as text.
fn expand_named_entities(src: &str) -> String {
    ENTITY
        .replace_all(src, |c: &regex::Captures| {
            let name = &c[1];
            if matches!(name, "amp" | "lt" | "gt" | "quot" | "apos") {
                return c[0].to_string();
            }
            match named_entity(name) {
                Some(ch) => format!("&#{};", ch as u32),
                None => format!("&amp;{name};"),
            }
        })
        .into_owned()
}

/// Deepest element nesting accepted. Real documents stay under about 40; the parser recurses per level.
pub(crate) const MAX_XML_DEPTH: usize = 100;

/// The deepest element nesting in `src`, from a linear scan that skips comments, CDATA, processing instructions
/// and declarations. It does not validate; it only has to be an upper bound on what the parser will recurse into.
pub(crate) fn max_depth(src: &str) -> usize {
    let b = src.as_bytes();
    let (mut depth, mut max, mut i) = (0usize, 0usize, 0usize);
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i += src[i..].find("-->").map(|p| p + 3).unwrap_or(b.len() - i);
        } else if rest.starts_with(b"<![CDATA[") {
            i += src[i..].find("]]>").map(|p| p + 3).unwrap_or(b.len() - i);
        } else if rest.starts_with(b"<?") {
            i += src[i..].find("?>").map(|p| p + 2).unwrap_or(b.len() - i);
        } else if rest.starts_with(b"<!") {
            // DOCTYPE: skip to the matching '>' honouring an internal subset in brackets.
            let mut brackets = 0i32;
            let mut j = i + 2;
            while j < b.len() {
                match b[j] {
                    b'[' => brackets += 1,
                    b']' => brackets -= 1,
                    b'>' if brackets <= 0 => break,
                    _ => {}
                }
                j += 1;
            }
            i = j + 1;
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i += src[i..].find('>').map(|p| p + 1).unwrap_or(b.len() - i);
        } else {
            // A start tag; quotes may hide a '>' inside attribute values.
            let mut j = i + 1;
            let mut quote = 0u8;
            while j < b.len() {
                let c = b[j];
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == 0x27 {
                    quote = c;
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            let self_closing = j > i && b.get(j - 1) == Some(&b'/');
            if !self_closing {
                depth += 1;
                max = max.max(depth);
            }
            i = j + 1;
        }
    }
    max
}

fn parse_options<'a>() -> ParsingOptions<'a> {
    ParsingOptions { allow_dtd: true, nodes_limit: NODES_LIMIT, ..Default::default() }
}

/// Decode `bytes` and run `f` on the parsed document. The DTD is allowed and the node count is limited; when the
/// first parse hits an unknown named entity, the common HTML entities are expanded and parsing is retried once.
/// `f` also receives the source encoding.
pub(crate) fn with_document<T>(
    bytes: &[u8],
    format: &'static str,
    f: impl FnOnce(&Document, &'static str) -> Result<T>,
) -> Result<T> {
    let (src, encoding) = decode_xml(bytes);
    let depth = max_depth(&src);
    if depth > MAX_XML_DEPTH {
        tracing::warn!(depth, format, "refusing hostile XML nesting");
        return Err(ExtractError::Limits(format!("{format} element nesting is {depth} levels deep (limit {MAX_XML_DEPTH})")));
    }
    match Document::parse_with_options(&src, parse_options()) {
        Ok(doc) => f(&doc, encoding),
        Err(roxmltree::Error::UnknownEntityReference(..)) => {
            let fixed = expand_named_entities(&src);
            let doc = Document::parse_with_options(&fixed, parse_options()).map_err(|e| ExtractError::corrupt(format, e.to_string()))?;
            f(&doc, encoding)
        }
        Err(e) => Err(ExtractError::corrupt(format, e.to_string())),
    }
}

// --------------------------------------------------------------------------------------------- node helpers

pub(crate) fn name<'a>(n: Node<'a, '_>) -> &'a str {
    n.tag_name().name()
}

pub(crate) fn is(n: Node, local: &str) -> bool {
    n.is_element() && n.tag_name().name() == local
}

pub(crate) fn child<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| is(*c, local))
}

pub(crate) fn children_named<'a, 'i>(n: Node<'a, 'i>, local: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| is(*c, local))
}

pub(crate) fn descendant<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.descendants().skip(1).find(|c| is(*c, local))
}

/// An attribute by local name, ignoring its namespace (`xlink:href`, `xml:lang`, plain `href`).
pub(crate) fn attr<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.name() == local).map(|a| a.value())
}

/// All descendant text, unmodified.
pub(crate) fn raw_text(n: Node) -> String {
    let mut s = String::new();
    for d in n.descendants() {
        if d.is_text() {
            s.push_str(d.text().unwrap_or(""));
        }
    }
    s
}

/// All descendant text with whitespace collapsed.
pub(crate) fn text_of(n: Node) -> String {
    crate::builder::collapse_ws(&raw_text(n))
}

pub(crate) fn text_of_child(n: Node, local: &str) -> Option<String> {
    child(n, local).map(text_of).filter(|s| !s.is_empty())
}

// ------------------------------------------------------------------------------------------ HTML entity table

/// The HTML named entities that turn up in scholarly XML written against a DTD.
pub(crate) fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "nbsp" => '\u{a0}',
        "iexcl" => '¡',
        "cent" => '¢',
        "pound" => '£',
        "curren" => '¤',
        "yen" => '¥',
        "brvbar" => '¦',
        "sect" => '§',
        "uml" => '¨',
        "copy" => '©',
        "ordf" => 'ª',
        "laquo" => '«',
        "not" => '¬',
        "shy" => '\u{ad}',
        "reg" => '®',
        "macr" => '¯',
        "deg" => '°',
        "plusmn" => '±',
        "sup2" => '²',
        "sup3" => '³',
        "acute" => '´',
        "micro" => 'µ',
        "para" => '¶',
        "middot" => '·',
        "cedil" => '¸',
        "sup1" => '¹',
        "ordm" => 'º',
        "raquo" => '»',
        "frac14" => '¼',
        "frac12" => '½',
        "frac34" => '¾',
        "iquest" => '¿',
        "Agrave" => 'À',
        "Aacute" => 'Á',
        "Acirc" => 'Â',
        "Atilde" => 'Ã',
        "Auml" => 'Ä',
        "Aring" => 'Å',
        "AElig" => 'Æ',
        "Ccedil" => 'Ç',
        "Egrave" => 'È',
        "Eacute" => 'É',
        "Ecirc" => 'Ê',
        "Euml" => 'Ë',
        "Igrave" => 'Ì',
        "Iacute" => 'Í',
        "Icirc" => 'Î',
        "Iuml" => 'Ï',
        "ETH" => 'Ð',
        "Ntilde" => 'Ñ',
        "Ograve" => 'Ò',
        "Oacute" => 'Ó',
        "Ocirc" => 'Ô',
        "Otilde" => 'Õ',
        "Ouml" => 'Ö',
        "times" => '×',
        "Oslash" => 'Ø',
        "Ugrave" => 'Ù',
        "Uacute" => 'Ú',
        "Ucirc" => 'Û',
        "Uuml" => 'Ü',
        "Yacute" => 'Ý',
        "THORN" => 'Þ',
        "szlig" => 'ß',
        "agrave" => 'à',
        "aacute" => 'á',
        "acirc" => 'â',
        "atilde" => 'ã',
        "auml" => 'ä',
        "aring" => 'å',
        "aelig" => 'æ',
        "ccedil" => 'ç',
        "egrave" => 'è',
        "eacute" => 'é',
        "ecirc" => 'ê',
        "euml" => 'ë',
        "igrave" => 'ì',
        "iacute" => 'í',
        "icirc" => 'î',
        "iuml" => 'ï',
        "eth" => 'ð',
        "ntilde" => 'ñ',
        "ograve" => 'ò',
        "oacute" => 'ó',
        "ocirc" => 'ô',
        "otilde" => 'õ',
        "ouml" => 'ö',
        "divide" => '÷',
        "oslash" => 'ø',
        "ugrave" => 'ù',
        "uacute" => 'ú',
        "ucirc" => 'û',
        "uuml" => 'ü',
        "yacute" => 'ý',
        "thorn" => 'þ',
        "yuml" => 'ÿ',
        "OElig" => 'Œ',
        "oelig" => 'œ',
        "Scaron" => 'Š',
        "scaron" => 'š',
        "Yuml" => 'Ÿ',
        "fnof" => 'ƒ',
        "circ" => 'ˆ',
        "tilde" => '˜',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "zwnj" => '\u{200c}',
        "zwj" => '\u{200d}',
        "lrm" => '\u{200e}',
        "rlm" => '\u{200f}',
        "ndash" => '–',
        "mdash" => '—',
        "lsquo" => '‘',
        "rsquo" => '’',
        "sbquo" => '‚',
        "ldquo" => '“',
        "rdquo" => '”',
        "bdquo" => '„',
        "dagger" => '†',
        "Dagger" => '‡',
        "bull" => '•',
        "hellip" => '…',
        "permil" => '‰',
        "prime" => '′',
        "Prime" => '″',
        "lsaquo" => '‹',
        "rsaquo" => '›',
        "oline" => '‾',
        "frasl" => '⁄',
        "euro" => '€',
        "trade" => '™',
        "alefsym" => 'ℵ',
        "larr" => '←',
        "uarr" => '↑',
        "rarr" => '→',
        "darr" => '↓',
        "harr" => '↔',
        "lArr" => '⇐',
        "rArr" => '⇒',
        "hArr" => '⇔',
        "forall" => '∀',
        "part" => '∂',
        "exist" => '∃',
        "empty" => '∅',
        "nabla" => '∇',
        "isin" => '∈',
        "notin" => '∉',
        "ni" => '∋',
        "prod" => '∏',
        "sum" => '∑',
        "minus" => '−',
        "lowast" => '∗',
        "radic" => '√',
        "prop" => '∝',
        "infin" => '∞',
        "ang" => '∠',
        "and" => '∧',
        "or" => '∨',
        "cap" => '∩',
        "cup" => '∪',
        "int" => '∫',
        "there4" => '∴',
        "sim" => '∼',
        "cong" => '≅',
        "asymp" => '≈',
        "ne" => '≠',
        "equiv" => '≡',
        "le" => '≤',
        "ge" => '≥',
        "sub" => '⊂',
        "sup" => '⊃',
        "nsub" => '⊄',
        "sube" => '⊆',
        "supe" => '⊇',
        "oplus" => '⊕',
        "otimes" => '⊗',
        "perp" => '⊥',
        "sdot" => '⋅',
        "lceil" => '⌈',
        "rceil" => '⌉',
        "lfloor" => '⌊',
        "rfloor" => '⌋',
        "lang" => '〈',
        "rang" => '〉',
        "loz" => '◊',
        "Alpha" => 'Α',
        "Beta" => 'Β',
        "Gamma" => 'Γ',
        "Delta" => 'Δ',
        "Epsilon" => 'Ε',
        "Zeta" => 'Ζ',
        "Eta" => 'Η',
        "Theta" => 'Θ',
        "Iota" => 'Ι',
        "Kappa" => 'Κ',
        "Lambda" => 'Λ',
        "Mu" => 'Μ',
        "Nu" => 'Ν',
        "Xi" => 'Ξ',
        "Omicron" => 'Ο',
        "Pi" => 'Π',
        "Rho" => 'Ρ',
        "Sigma" => 'Σ',
        "Tau" => 'Τ',
        "Upsilon" => 'Υ',
        "Phi" => 'Φ',
        "Chi" => 'Χ',
        "Psi" => 'Ψ',
        "Omega" => 'Ω',
        "alpha" => 'α',
        "beta" => 'β',
        "gamma" => 'γ',
        "delta" => 'δ',
        "epsilon" => 'ε',
        "zeta" => 'ζ',
        "eta" => 'η',
        "theta" => 'θ',
        "iota" => 'ι',
        "kappa" => 'κ',
        "lambda" => 'λ',
        "mu" => 'μ',
        "nu" => 'ν',
        "xi" => 'ξ',
        "omicron" => 'ο',
        "pi" => 'π',
        "rho" => 'ρ',
        "sigmaf" => 'ς',
        "sigma" => 'σ',
        "tau" => 'τ',
        "upsilon" => 'υ',
        "phi" => 'φ',
        "chi" => 'χ',
        "psi" => 'ψ',
        "omega" => 'ω',
        "thetasym" => 'ϑ',
        "upsih" => 'ϒ',
        "piv" => 'ϖ',
        "hearts" => '♥',
        "spades" => '♠',
        "clubs" => '♣',
        "diams" => '♦',
        "check" => '✓',
        "star" => '☆',
        "hairsp" => '\u{200a}',
        "numsp" => '\u{2007}',
        "ThinSpace" => '\u{2009}',
        "InvisibleTimes" => '\u{2062}',
        "ApplyFunction" => '\u{2061}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_a_doctype() {
        let src = r#"<?xml version="1.0"?><!DOCTYPE article PUBLIC "-//NLM//DTD JATS//EN" "x.dtd"><article><p>hi</p></article>"#;
        let t = with_document(src.as_bytes(), "jats", |d, _| Ok(text_of(d.root_element()))).unwrap();
        assert_eq!(t, "hi");
    }

    #[test]
    fn unknown_named_entities_are_expanded_on_retry() {
        let src = "<article><p>a&nbsp;b &mdash; c &alpha; &madeup; d</p></article>";
        let t = with_document(src.as_bytes(), "jats", |d, _| Ok(raw_text(d.root_element()))).unwrap();
        assert_eq!(t, "a\u{a0}b — c α &madeup; d");
    }

    #[test]
    fn nesting_depth_is_measured_and_limited() {
        assert_eq!(max_depth("<?xml version='1.0'?><!DOCTYPE a [<!ENTITY x '<b>'>]><a><b/><c><d>t</d><!-- <e><f> --></c><![CDATA[<g><h>]]></a>"), 3);
        assert_eq!(max_depth("<a x=\"1>2\"><b y='<z>'/></a>"), 1);
        let deep = format!("{}x{}", "<a>".repeat(5000), "</a>".repeat(5000));
        let err = with_document(deep.as_bytes(), "jats", |_, _| Ok(())).unwrap_err();
        assert_eq!(err.reason_code(), "archive_limits");
    }

    #[test]
    fn malformed_xml_is_corrupt() {
        let err = with_document(b"<a><b></a>", "jats", |_, _| Ok(())).unwrap_err();
        assert_eq!(err.reason_code(), "corrupt_file");
    }

    #[test]
    fn declared_encoding_is_honoured() {
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a>caf\u{e9}</a>");
        let (s, enc) = decode_xml(&bytes);
        assert!(s.contains("café"), "{s} {enc}");
        let (s, enc) = decode_xml("<?xml version=\"1.0\" encoding=\"UTF-8\"?><a>café</a>".as_bytes());
        assert!(s.contains("café") && enc == "utf-8");
    }

    #[test]
    fn attribute_lookup_ignores_namespaces() {
        let doc = Document::parse(r#"<a xmlns:xlink="http://www.w3.org/1999/xlink" xlink:href="u" xml:lang="fr"><b/></a>"#).unwrap();
        let a = doc.root_element();
        assert_eq!(attr(a, "href"), Some("u"));
    }
}

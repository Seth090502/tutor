//! Format-independent text sizing, and markup stripping for EPUB (X)HTML.

use serde::Serialize;
use std::ops::AddAssign;

/// Characters per token for the rough `tokens_est = characters / 4` estimate.
pub const CHARS_PER_TOKEN: u64 = 4;

/// Longest entity name (between `&` and `;`) that is decoded.
const MAX_ENTITY_LEN: usize = 32;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TextSize {
    /// Length of the text after collapsing each whitespace run to one space and trimming, so
    /// PDF line breaks and XHTML indentation do not inflate the count. Control characters are
    /// not counted.
    pub characters: u64,
    pub whitespace_tokens: u64,
}

impl TextSize {
    pub fn tokens_est(&self) -> u64 {
        self.characters / CHARS_PER_TOKEN
    }
}

impl AddAssign for TextSize {
    fn add_assign(&mut self, rhs: Self) {
        self.characters += rhs.characters;
        self.whitespace_tokens += rhs.whitespace_tokens;
    }
}

pub fn measure(text: &str) -> TextSize {
    let mut size = TextSize::default();
    for token in text.split_whitespace() {
        size.whitespace_tokens += 1;
        size.characters += token.chars().filter(|c| !c.is_control()).count() as u64;
    }
    // One separating space between consecutive tokens.
    size.characters += size.whitespace_tokens.saturating_sub(1);
    size
}

/// Reduce (X)HTML to its text: tags and comments become a space, `script` and `style`
/// bodies are dropped, CDATA is kept, and XML entities plus `&nbsp;` and numeric references
/// are decoded. Only used for counting, so it favours simplicity over full HTML parsing.
pub fn strip_markup(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(lt) = rest.find('<') {
        push_decoded(&mut out, &rest[..lt]);
        out.push(' ');
        rest = &rest[lt..];
        rest = if let Some(body) = rest.strip_prefix("<!--") {
            after(body, "-->")
        } else if let Some(body) = rest.strip_prefix("<![CDATA[") {
            out.push_str(&body[..body.find("]]>").unwrap_or(body.len())]);
            after(body, "]]>")
        } else if let Some(name) = raw_text_element(rest) {
            let open_end = rest.find('>').unwrap_or(rest.len());
            if rest[..open_end].ends_with('/') {
                after(rest, ">")
            } else {
                let body = &rest[open_end..];
                find_ascii_ci(body, &format!("</{name}")).map_or("", |i| after(&body[i..], ">"))
            }
        } else {
            after(rest, ">")
        };
    }
    push_decoded(&mut out, rest);
    out
}

/// The rest of `s` after the first `pat`, or "" if `pat` does not occur.
fn after<'a>(s: &'a str, pat: &str) -> &'a str {
    s.find(pat).map_or("", |i| &s[i + pat.len()..])
}

/// `script` or `style` if `tag` opens one of those elements (any case).
fn raw_text_element(tag: &str) -> Option<&'static str> {
    let b = tag.as_bytes();
    ["script", "style"].into_iter().find(|name| {
        let n = name.len();
        b.len() > n + 1
            && b[1..=n].eq_ignore_ascii_case(name.as_bytes())
            && matches!(b[n + 1], b'>' | b'/' | b' ' | b'\t' | b'\r' | b'\n')
    })
}

/// Byte offset of an ASCII `needle` in `s`, ignoring ASCII case. The offset is always a char
/// boundary because ASCII bytes never occur inside multi-byte UTF-8 sequences.
fn find_ascii_ci(s: &str, needle: &str) -> Option<usize> {
    s.as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

fn push_decoded(out: &mut String, text: &str) {
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        // Entity names are short; bounding the search keeps text full of bare '&' linear.
        let decoded = rest.as_bytes()[1..]
            .iter()
            .take(MAX_ENTITY_LEN)
            .position(|&b| b == b';')
            .and_then(|semi| decode_entity(&rest[1..1 + semi]).map(|c| (c, semi + 2)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
}

fn decode_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_collapses_whitespace_runs() {
        assert_eq!(measure("  a  bc\r\n\r\n d\t"), TextSize { characters: 6, whitespace_tokens: 3 });
    }

    #[test]
    fn measure_counts_unicode_scalars_not_bytes() {
        assert_eq!(measure("π ≤ ∑"), TextSize { characters: 5, whitespace_tokens: 3 });
    }

    #[test]
    fn measure_ignores_control_characters() {
        assert_eq!(measure("ab\u{2}c"), TextSize { characters: 3, whitespace_tokens: 1 });
    }

    #[test]
    fn measure_of_blank_text_is_zero() {
        assert_eq!(measure(""), TextSize::default());
        assert_eq!(measure(" \n\t "), TextSize::default());
    }

    #[test]
    fn tokens_est_is_characters_over_four() {
        assert_eq!(TextSize { characters: 4001, whitespace_tokens: 0 }.tokens_est(), 1000);
    }

    #[test]
    fn sizes_add() {
        let mut a = TextSize { characters: 3, whitespace_tokens: 1 };
        a += TextSize { characters: 4, whitespace_tokens: 2 };
        assert_eq!(a, TextSize { characters: 7, whitespace_tokens: 3 });
    }

    #[test]
    fn strip_removes_tags_comments_script_and_style() {
        let html = "<?xml version=\"1.0\"?><html><head><title>T</title>\
                    <style>p { color: red; }</style><script>var x = '<p>';</script></head>\
                    <body><!-- note <b>hidden</b> --><h1>Chapter&#160;1</h1>\
                    <p>Plants &amp; algae</p><p>use &lt;light&gt; &#x3C0;</p></body></html>";
        let text = strip_markup(html);
        assert_eq!(measure(&text), measure("T Chapter 1 Plants & algae use <light> π"));
        assert!(!text.contains("color"));
        assert!(!text.contains("var x"));
        assert!(!text.contains("hidden"));
    }

    #[test]
    fn strip_keeps_cdata_and_unknown_entities() {
        let text = strip_markup("<p><![CDATA[a < b]]> &copy; &bogus</p>");
        assert_eq!(measure(&text), measure("a < b &copy; &bogus"));
    }

    #[test]
    fn strip_handles_uppercase_tags_and_unterminated_markup() {
        assert_eq!(measure(&strip_markup("<SCRIPT>x()</SCRIPT>ok")), measure("ok"));
        assert_eq!(measure(&strip_markup("text <p unterminated")), measure("text"));
        assert_eq!(measure(&strip_markup("a <!-- open comment")), measure("a"));
    }
}

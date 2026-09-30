//! The BM25 analyzer: tantivy's `en_stem` chain (simple tokenizer, tokens over 40 bytes dropped,
//! lowercase, English stemmer) behind a Unicode maths normalisation, applied to indexed text and
//! queries alike.
//!
//! PDFium emits the glyphs a PDF shows, not LaTeX, so the same formula reaches the index in
//! several spellings. Before tokenising, text is put in Unicode NFKC form, which maps
//! mathematical alphanumerics (`𝑥`, `𝐯`, `ℝ`), super- and subscripts (`x²` becomes `x2`, as
//! pdfium already emits positioned superscripts), ligatures and full-width forms to plain
//! characters. Then Greek letters and common operators become their LaTeX command names as
//! separate words (`2π` becomes `2 pi`, `∑` becomes `sum`, `≤` becomes `leq`), so glyph text,
//! LaTeX queries (`\sum`, `\alpha`) and spelled-out Greek meet on the same terms. Token offsets
//! refer to the normalised text; the index stores no positions or offsets.

use tantivy::tokenizer::{
    Language, LowerCaser, RemoveLongFilter, SimpleTokenStream, SimpleTokenizer, Stemmer, TextAnalyzer, Tokenizer,
};
use unicode_normalization::UnicodeNormalization;

/// The analyzer's name in the index schema. A new name is a new schema (see `bm25::SCHEMA_VERSION`).
pub const ANALYZER: &str = "en_stem_math";

pub fn analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(MathTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(Stemmer::new(Language::English))
        .build()
}

/// [`SimpleTokenizer`] over [`normalise`]d text.
#[derive(Clone, Default)]
pub struct MathTokenizer {
    text: String,
    inner: SimpleTokenizer,
}

impl Tokenizer for MathTokenizer {
    type TokenStream<'a> = SimpleTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> SimpleTokenStream<'a> {
        self.text.clear();
        normalise_into(text, &mut self.text);
        self.inner.token_stream(&self.text)
    }
}

/// Greek letters in alphabet order from alpha (U+03B1 and U+0391); final sigma sits before sigma.
const GREEK: [&str; 25] = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa", "lambda", "mu", "nu", "xi",
    "omicron", "pi", "rho", "sigma", "sigma", "tau", "upsilon", "phi", "chi", "psi", "omega",
];

/// The LaTeX name of a Greek letter or maths symbol, after NFKC (which has already turned
/// variants such as `ϑ`, `ϕ`, `µ`, `Ω` (ohm) and the mathematical alphanumerics into these).
fn latex_name(c: char) -> Option<&'static str> {
    let greek = |base: u32| GREEK.get((c as u32 - base) as usize).copied();
    match c {
        'α'..='ω' => greek(0x3B1),
        'Α'..='Ω' => greek(0x391),
        '∆' => Some("delta"), // U+2206 INCREMENT, which some fonts use for Delta
        '∑' => Some("sum"),
        '∏' => Some("prod"),
        '∐' => Some("coprod"),
        '∫' => Some("int"),
        '∬' => Some("iint"),
        '∭' => Some("iiint"),
        '∮' => Some("oint"),
        '∂' => Some("partial"),
        '∇' => Some("nabla"),
        '∞' => Some("infty"),
        '√' => Some("sqrt"),
        '∈' => Some("in"),
        '∉' => Some("notin"),
        '∋' => Some("ni"),
        '⊂' => Some("subset"),
        '⊃' => Some("supset"),
        '⊆' => Some("subseteq"),
        '⊇' => Some("supseteq"),
        '∪' => Some("cup"),
        '∩' => Some("cap"),
        '∅' => Some("emptyset"),
        '∀' => Some("forall"),
        '∃' => Some("exists"),
        '∄' => Some("nexists"),
        '¬' => Some("neg"),
        '∧' => Some("wedge"),
        '∨' => Some("vee"),
        '⊕' => Some("oplus"),
        '⊗' => Some("otimes"),
        '⊥' => Some("perp"),
        '∥' => Some("parallel"),
        '∠' => Some("angle"),
        '≤' | '⩽' => Some("leq"),
        '≥' | '⩾' => Some("geq"),
        '≠' => Some("neq"),
        '≈' => Some("approx"),
        '≡' => Some("equiv"),
        '≅' => Some("cong"),
        '∼' => Some("sim"),
        '≃' => Some("simeq"),
        '∝' => Some("propto"),
        '≪' => Some("ll"),
        '≫' => Some("gg"),
        '±' => Some("pm"),
        '∓' => Some("mp"),
        '×' => Some("times"),
        '÷' => Some("div"),
        '⋅' => Some("cdot"),
        '∘' => Some("circ"),
        '→' => Some("to"),
        '←' => Some("gets"),
        '↔' => Some("leftrightarrow"),
        '↦' => Some("mapsto"),
        '⇒' => Some("implies"),
        '⇐' => Some("impliedby"),
        '⇔' => Some("iff"),
        'ħ' => Some("hbar"),
        'ℵ' => Some("aleph"),
        _ => None,
    }
}

fn normalise_into(text: &str, out: &mut String) {
    if text.is_ascii() {
        out.push_str(text);
        return;
    }
    for c in text.nfkc() {
        match latex_name(c) {
            Some(name) => {
                out.push(' ');
                out.push_str(name);
                out.push(' ');
            }
            None => out.push(c),
        }
    }
}

/// The text the tokenizer splits (see the module comment).
#[cfg(test)]
pub fn normalise(text: &str) -> String {
    let mut out = String::new();
    normalise_into(text, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(text: &str) -> Vec<String> {
        let mut analyzer = analyzer();
        let mut stream = analyzer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        out
    }

    #[test]
    fn greek_letters_become_their_names() {
        assert_eq!(terms("2π r"), ["2", "pi", "r"]);
        assert_eq!(terms("Δx and δ"), ["delta", "x", "and", "delta"]);
        assert_eq!(terms("αβ"), ["alpha", "beta"]);
        assert_eq!(terms("ς σ Σ ω Ω"), ["sigma", "sigma", "sigma", "omega", "omega"]);
        // Variant forms, the micro and ohm signs, and the increment sign some fonts use for Delta.
        assert_eq!(terms("ϑ ϕ ϵ µ Ω ∆"), ["theta", "phi", "epsilon", "mu", "omega", "delta"]);
    }

    #[test]
    fn operators_become_latex_names() {
        assert_eq!(terms("∑ k"), ["sum", "k"]);
        assert_eq!(terms("∫ x dx"), ["int", "x", "dx"]);
        assert_eq!(terms("∂u/∂t"), ["partial", "u", "partial", "t"]);
        assert_eq!(terms("|x| ≤ 1, a ≠ 0, x ∈ A ⊆ B"), ["x", "leq", "1", "a", "neq", "0", "x", "in", "a", "subseteq", "b"]);
        assert_eq!(terms("√∆ → ∞"), ["sqrt", "delta", "to", "infti"]); // "infty" stems to "infti"
        assert_eq!(terms("ℏω"), ["hbar", "omega"]);
    }

    #[test]
    fn mathematical_alphanumerics_and_scripts_become_plain_characters() {
        // Italic x, bold v, double-struck R, italic alpha and bold digits.
        assert_eq!(terms("𝑥 𝐯 ℝ 𝛼 𝟐"), ["x", "v", "r", "alpha", "2"]);
        // Superscripts and subscripts read like positioned text: x² as x2, H₂O as H2O.
        assert_eq!(terms("x² + H₂O"), ["x2", "h2o"]);
        assert_eq!(terms("ﬁnite"), ["finit"]);
    }

    #[test]
    fn plain_text_and_latex_queries_meet_glyph_text() {
        assert_eq!(normalise("plain ASCII stays as it is"), "plain ASCII stays as it is");
        assert_eq!(terms(r"\alpha + \sum_{k} \leq \int"), terms("α + ∑_{k} ≤ ∫"));
        assert_eq!(terms("alpha particles"), terms("α particles"));
        // Accented letters stay composed, so words keep their accents.
        assert_eq!(terms("café"), ["café"]);
    }
}

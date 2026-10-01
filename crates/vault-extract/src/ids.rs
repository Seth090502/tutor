//! Identifier extraction from free text: DOI, arXiv id, PMCID, PMID, ISBN and years.

use std::sync::LazyLock;

use regex::Regex;

use crate::types::Reference;

static DOI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\b10\.\d{4,9}/[^\s"<>]+"#).expect("static regex"));
static ARXIV_NEW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:arxiv[:\s/]*(?:abs/|pdf/)?|\b)(\d{4}\.\d{4,5})(?:v\d+)?\b").expect("static regex")
});
static ARXIV_PREFIXED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)arxiv[:\s]*(?:abs/|pdf/)?((?:\d{4}\.\d{4,5})|(?:[a-z\-]+(?:\.[a-z]{2})?/\d{7}))(?:v\d+)?")
        .expect("static regex")
});
static ARXIV_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)arxiv\.org/(?:abs|pdf)/((?:\d{4}\.\d{4,5})|(?:[a-z\-]+(?:\.[a-z]{2})?/\d{7}))(?:v\d+)?")
        .expect("static regex")
});
static PMCID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bPMC\s?(\d{4,9})\b").expect("static regex"));
static PMID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bPMID:?\s*(\d{4,9})\b").expect("static regex"));
static YEAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(1[5-9]\d\d|20\d\d)\b").expect("static regex"));
static ISBN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:isbn(?:-1[03])?:?\s*)?((?:97[89][\- ]?)?\d(?:[\- ]?\d){8}[\- ]?[\dxX])\b")
        .expect("static regex")
});

/// The first DOI in `s`, lower-cased, without a resolver URL prefix or trailing punctuation.
pub fn find_doi(s: &str) -> Option<String> {
    let m = DOI.find(s)?;
    let mut d = m.as_str().to_string();
    while d.ends_with(['.', ',', ';', ')', ']', '}', '>', '"', '\'']) {
        // Keep a closing parenthesis that balances an opening one inside the DOI.
        if d.ends_with(')') && d.matches('(').count() >= d.matches(')').count() {
            break;
        }
        d.pop();
    }
    Some(d.to_lowercase())
}

/// The first arXiv identifier in `s` (without version), from `arXiv:...` or an arxiv.org URL.
pub fn find_arxiv(s: &str) -> Option<String> {
    if let Some(c) = ARXIV_URL.captures(s).or_else(|| ARXIV_PREFIXED.captures(s)) {
        return Some(c[1].to_string());
    }
    None
}

/// An arXiv identifier written bare, such as `2301.01234v2`.
pub fn bare_arxiv(s: &str) -> Option<String> {
    let t = s.trim();
    let c = ARXIV_NEW.captures(t)?;
    (c.get(0)?.as_str().len() == t.len() || t.to_ascii_lowercase().starts_with("arxiv")).then(|| c[1].to_string())
}

pub fn normalize_pmcid(s: &str) -> Option<String> {
    let t = s.trim();
    if let Some(c) = PMCID.captures(t) {
        return Some(format!("PMC{}", &c[1]));
    }
    (t.len() >= 4 && t.len() <= 9 && t.chars().all(|c| c.is_ascii_digit())).then(|| format!("PMC{t}"))
}

pub fn find_pmid(s: &str) -> Option<String> {
    PMID.captures(s).map(|c| c[1].to_string())
}

/// The first plausible publication year in `s`.
pub fn year_in(s: &str) -> Option<i32> {
    YEAR.captures(s).and_then(|c| c[1].parse().ok())
}

/// ISBN-10 or ISBN-13 digits without separators, only when the check digit is valid.
pub fn normalize_isbn(s: &str) -> Option<String> {
    let digits: String =
        s.chars().filter(|c| c.is_ascii_digit() || matches!(c, 'x' | 'X')).collect::<String>().to_ascii_uppercase();
    // Strip a leading "ISBN" artefact: letters were already filtered, so only digits remain.
    match digits.len() {
        10 => {
            let mut sum = 0;
            for (i, c) in digits.chars().enumerate() {
                let v = if c == 'X' { if i == 9 { 10 } else { return None } } else { c.to_digit(10)? };
                sum += v * (10 - i as u32);
            }
            (sum % 11 == 0).then_some(digits)
        }
        13 if digits.starts_with("978") || digits.starts_with("979") => {
            let mut sum = 0;
            for (i, c) in digits.chars().enumerate() {
                let v = c.to_digit(10)?;
                sum += if i % 2 == 0 { v } else { 3 * v };
            }
            (sum % 10 == 0).then_some(digits)
        }
        _ => None,
    }
}

/// Every valid ISBN found in free text.
pub fn find_isbns(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in ISBN.captures_iter(s) {
        if let Some(i) = normalize_isbn(&c[1])
            && !out.contains(&i)
        {
            out.push(i);
        }
    }
    out
}

/// A [`Reference`] with the identifiers it mentions filled in.
pub fn reference_from_raw(raw: &str, label: Option<String>) -> Reference {
    let raw = crate::builder::collapse_ws(raw);
    Reference {
        doi: find_doi(&raw),
        pmid: find_pmid(&raw),
        pmcid: PMCID.captures(&raw).map(|c| format!("PMC{}", &c[1])),
        arxiv_id: find_arxiv(&raw),
        isbn: find_isbns(&raw).into_iter().next(),
        year: year_in(&raw),
        label,
        raw,
        title: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dois() {
        assert_eq!(
            find_doi("see https://doi.org/10.1038/S41586-020-2649-2."),
            Some("10.1038/s41586-020-2649-2".into())
        );
        assert_eq!(
            find_doi("doi:10.1002/(SICI)1097-4571(199806)49:8 next"),
            Some("10.1002/(sici)1097-4571(199806)49:8".into())
        );
        assert_eq!(find_doi("no identifier here 10.5 / x"), None);
        assert_eq!(find_doi("(doi: 10.1000/abc123)"), Some("10.1000/abc123".into()));
    }

    #[test]
    fn arxiv_ids() {
        assert_eq!(find_arxiv("arXiv:2301.01234v2 [cs.LG]"), Some("2301.01234".into()));
        assert_eq!(find_arxiv("https://arxiv.org/abs/1706.03762v5"), Some("1706.03762".into()));
        assert_eq!(find_arxiv("arXiv:hep-th/9901001"), Some("hep-th/9901001".into()));
        assert_eq!(find_arxiv("Published in 2019.12345 pages"), None);
        assert_eq!(bare_arxiv("2301.01234v2"), Some("2301.01234".into()));
        assert_eq!(bare_arxiv("arXiv:2301.01234"), Some("2301.01234".into()));
    }

    #[test]
    fn pmc_pmid_year() {
        assert_eq!(normalize_pmcid("PMC1234567"), Some("PMC1234567".into()));
        assert_eq!(normalize_pmcid("1234567"), Some("PMC1234567".into()));
        assert_eq!(normalize_pmcid("not an id"), None);
        assert_eq!(find_pmid("J Biol. 2001. PMID: 11234567."), Some("11234567".into()));
        assert_eq!(year_in("Smith, J. (2019). Title"), Some(2019));
        assert_eq!(year_in("no year 12345"), None);
    }

    #[test]
    fn isbn_checksums() {
        assert_eq!(normalize_isbn("978-0-306-40615-7"), Some("9780306406157".into()));
        assert_eq!(normalize_isbn("0-306-40615-2"), Some("0306406152".into()));
        assert_eq!(normalize_isbn("0-8044-2957-X"), Some("080442957X".into()));
        assert_eq!(normalize_isbn("978-0-306-40615-8"), None);
        assert_eq!(
            find_isbns("ISBN 978-0-306-40615-7 and ISBN-10: 0-306-40615-2"),
            vec!["9780306406157", "0306406152"]
        );
    }

    #[test]
    fn reference_ids_are_extracted() {
        let r = reference_from_raw(
            "Vaswani A. et al. (2017) Attention is all you need. arXiv:1706.03762. doi:10.5555/3295222.3295349",
            Some("3".into()),
        );
        assert_eq!(r.arxiv_id.as_deref(), Some("1706.03762"));
        assert_eq!(r.doi.as_deref(), Some("10.5555/3295222.3295349"));
        assert_eq!(r.year, Some(2017));
        assert_eq!(r.label.as_deref(), Some("3"));
    }
}

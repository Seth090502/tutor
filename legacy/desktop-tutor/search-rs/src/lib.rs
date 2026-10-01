//! BM25 search over the Tutor corpus.
//!
//! Ranking is the `bm25` crate (MIT, github.com/Michael-JB/bm25): English
//! stemming, NLTK stopwords, unicode normalization, k1=1.2, b=0.75. This crate
//! adds corpus loading, a deterministic tie order, and snippets.

use bm25::{DefaultTokenizer, Document, Language, SearchEngine, SearchEngineBuilder, Tokenizer};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One searchable record: a note file, or a user paper's metadata.
pub struct Doc {
    pub path: String,
    pub text: String,
    /// Fixed snippet (user papers); notes get a snippet around the first match.
    pub label: Option<String>,
}

pub struct Hit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

/// Every .md/.txt under `<root>/corpus` (sorted by path), then the entries of
/// `corpus/papers_index.json` if it exists. Unreadable note files are skipped
/// with a warning; a malformed papers index is an error.
pub fn load_corpus(root: &Path) -> Result<Vec<Doc>, String> {
    let corpus = root.join("corpus");
    let mut files = Vec::new();
    walk(&corpus, &mut files);
    files.sort();
    let mut docs = Vec::new();
    for f in files {
        match fs::read_to_string(&f) {
            Ok(text) => {
                let rel = f.strip_prefix(root).unwrap_or(&f);
                docs.push(Doc { path: rel.to_string_lossy().replace('\\', "/"), text, label: None });
            }
            Err(e) => eprintln!("tutor-search: skipping {}: {e}", f.display()),
        }
    }
    docs.extend(load_papers(&corpus.join("papers_index.json"))?);
    Ok(docs)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out);
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("md" | "txt")) {
            out.push(p);
        }
    }
}

/// Metadata-only records written by scripts/ingest_papers.py (never reads PDFs).
fn load_papers(index: &Path) -> Result<Vec<Doc>, String> {
    let Ok(raw) = fs::read_to_string(index) else { return Ok(Vec::new()) };
    let v: Value = serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", index.display()))?;
    let field = |d: &Value, k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let docs = v.get("documents").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(docs
        .iter()
        .map(|d| {
            let (title, file) = (field(d, "title_hint"), field(d, "filename"));
            let path = [field(d, "path"), field(d, "id")].into_iter().find(|s| !s.is_empty());
            let label: String = format!("[user paper] {title} ({file})").chars().take(400).collect();
            Doc { path: path.unwrap_or_else(|| "?".into()), text: format!("{title} {file}"), label: Some(label) }
        })
        .collect())
}

pub struct Index {
    docs: Vec<Doc>,
    engine: SearchEngine<usize>,
    tokenizer: DefaultTokenizer,
}

impl Index {
    pub fn new(docs: Vec<Doc>) -> Index {
        let records = docs.iter().enumerate().map(|(i, d)| Document::new(i, d.text.clone()));
        let engine = SearchEngineBuilder::<usize>::with_documents(Language::English, records).build();
        Index { docs, engine, tokenizer: DefaultTokenizer::new(Language::English) }
    }

    /// Top `limit` hits, score descending; ties keep corpus order so output is stable.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let mut found = self.engine.search(query, None);
        found.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.document.id.cmp(&b.document.id)));
        found.truncate(limit);
        let wanted: HashSet<String> = self.tokens(query).into_iter().collect();
        found
            .into_iter()
            .map(|r| {
                let d = &self.docs[r.document.id];
                let snippet = d.label.clone().unwrap_or_else(|| self.snippet(&d.text, &wanted));
                Hit { path: d.path.clone(), score: r.score, snippet }
            })
            .collect()
    }

    fn tokens(&self, text: &str) -> Vec<String> {
        <DefaultTokenizer as Tokenizer>::tokenize(&self.tokenizer, text)
    }

    /// Up to 400 chars from 160 before the first word whose token is in the query.
    fn snippet(&self, text: &str, wanted: &HashSet<String>) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut first = 0;
        let mut start = None;
        for i in 0..=chars.len() {
            if i < chars.len() && chars[i].is_alphanumeric() {
                start.get_or_insert(i);
            } else if let Some(s) = start.take() {
                let word: String = chars[s..i].iter().collect();
                if self.tokens(&word).iter().any(|t| wanted.contains(t)) {
                    first = s;
                    break;
                }
            }
        }
        let lo = first.saturating_sub(160);
        let hi = (first + 320).min(chars.len());
        let window: String = chars[lo..hi].iter().collect();
        window.replace('\n', " ").trim().chars().take(400).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(path: &str, text: &str) -> Doc {
        Doc { path: path.into(), text: text.into(), label: None }
    }

    fn paths(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.path.as_str()).collect()
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tutor-search-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("corpus").join("sub")).unwrap();
        root
    }

    #[test]
    fn stopwords_do_not_match_inside_other_words() {
        let idx = Index::new(vec![doc("a.md", "status later metadata format")]);
        assert!(idx.search("at of to", 3).is_empty());
    }

    #[test]
    fn stemming_matches_word_forms() {
        let idx = Index::new(vec![
            doc("math.md", "The discriminant decides root count."),
            doc("other.md", "Sources and licenses for the corpus."),
        ]);
        assert_eq!(paths(&idx.search("Roots of the equation?", 3)), ["math.md"]);
    }

    #[test]
    fn ties_break_by_corpus_order_and_limit_truncates() {
        let idx = Index::new(vec![doc("a.md", "cell wall"), doc("b.md", "cell wall"), doc("c.md", "cell wall")]);
        for _ in 0..5 {
            assert_eq!(paths(&idx.search("cell", 2)), ["a.md", "b.md"]);
        }
    }

    #[test]
    fn snippet_centers_on_first_match() {
        let text = format!("{}\nPhotosynthesis stores light energy.", "filler ".repeat(100));
        let idx = Index::new(vec![doc("s.md", &text)]);
        let hit = &idx.search("photosynthesis", 1)[0];
        assert!(hit.snippet.contains("Photosynthesis stores light energy."));
        assert!(hit.snippet.chars().count() <= 400 && !hit.snippet.contains('\n'));
    }

    #[test]
    fn load_corpus_reads_notes_and_paper_metadata() {
        let root = temp_root("load");
        fs::write(root.join("corpus").join("b.md"), "newton force").unwrap();
        fs::write(root.join("corpus").join("sub").join("a.txt"), "cells").unwrap();
        fs::write(root.join("corpus").join("index.json"), "{}").unwrap();
        fs::write(
            root.join("corpus").join("papers_index.json"),
            r#"{"documents": [{"id": "paper-1", "filename": "newton_laws.pdf", "title_hint": "newton laws", "path": "D:/p/newton_laws.pdf"}]}"#,
        )
        .unwrap();
        let docs = load_corpus(&root).unwrap();
        let got: Vec<&str> = docs.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(got, ["corpus/b.md", "corpus/sub/a.txt", "D:/p/newton_laws.pdf"]);
        let hits = Index::new(docs).search("newton", 5);
        let paper = hits.iter().find(|h| h.path == "D:/p/newton_laws.pdf").unwrap();
        assert_eq!(paper.snippet, "[user paper] newton laws (newton_laws.pdf)");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn malformed_papers_index_is_an_error() {
        let root = temp_root("bad");
        fs::write(root.join("corpus").join("papers_index.json"), "{not json").unwrap();
        assert!(load_corpus(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}

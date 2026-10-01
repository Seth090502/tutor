//! tutor-search: reads one query per stdin line, writes one JSON array of
//! {path, score, snippet} hits per stdout line. One process serves a whole
//! batch, because process launches are the expensive part on this machine.
//!
//! Usage: tutor-search --root <workspace> [--limit N]

use serde_json::json;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use tutor_search::{Index, load_corpus};

fn main() -> ExitCode {
    let mut root = PathBuf::from(".");
    let mut limit = 5usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let value = args.next();
        match (a.as_str(), value) {
            ("--root", Some(v)) => root = PathBuf::from(v),
            ("--limit", Some(v)) => match v.parse() {
                Ok(n) => limit = n,
                Err(_) => return fail(&format!("--limit wants a number, got {v:?}")),
            },
            _ => return fail(&format!("unexpected argument {a:?}; usage: tutor-search --root <dir> [--limit N]")),
        }
    }
    let index = match load_corpus(&root) {
        Ok(docs) => Index::new(docs),
        Err(e) => return fail(&e),
    };
    let mut out = BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let query = match line {
            Ok(q) => q,
            Err(e) => return fail(&format!("reading stdin: {e}")),
        };
        let hits: Vec<_> = index
            .search(&query, limit)
            .into_iter()
            .map(|h| json!({"path": h.path, "score": (f64::from(h.score) * 1000.0).round() / 1000.0, "snippet": h.snippet}))
            .collect();
        if writeln!(out, "{}", json!(hits)).is_err() {
            return fail("writing stdout");
        }
    }
    if out.flush().is_err() {
        return fail("writing stdout");
    }
    ExitCode::SUCCESS
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("tutor-search: {msg}");
    ExitCode::from(2)
}

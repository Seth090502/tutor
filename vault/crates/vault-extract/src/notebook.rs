//! Jupyter notebooks (nbformat 4, and the older nbformat 3 layout) to Markdown: markdown cells as written, code
//! cells fenced with the kernel language, short text outputs kept, everything else dropped.

use serde::Deserialize;
use serde_json::Value;

use crate::ConvertOptions;
use crate::builder::fenced;
use crate::error::{ExtractError, Result};
use crate::text::{self, analyze_markdown};
use crate::types::{Extracted, tool_string};

/// Text outputs longer than this many characters or lines are dropped.
const MAX_OUTPUT_CHARS: usize = 600;
const MAX_OUTPUT_LINES: usize = 20;

#[derive(Deserialize)]
struct Notebook {
    #[serde(default)]
    cells: Vec<Cell>,
    /// nbformat 3 keeps its cells in worksheets.
    #[serde(default)]
    worksheets: Vec<Worksheet>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Deserialize)]
struct Worksheet {
    #[serde(default)]
    cells: Vec<Cell>,
}

#[derive(Deserialize, Default)]
struct Cell {
    #[serde(default)]
    cell_type: String,
    #[serde(default)]
    source: Value,
    /// nbformat 3 code cells.
    #[serde(default)]
    input: Value,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    level: Option<u8>,
    #[serde(default)]
    outputs: Vec<Value>,
}

/// A notebook field that is either a string or a list of lines.
fn joined(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(lines) => lines.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(""),
        _ => String::new(),
    }
}

pub fn convert(bytes: &[u8], opts: &ConvertOptions) -> Result<Extracted> {
    if bytes.is_empty() {
        return Err(ExtractError::Empty);
    }
    let mut warnings = Vec::new();
    let d = text::decode_for_conversion(bytes, opts, &mut warnings);
    let nb: Notebook = serde_json::from_str(&d.text).map_err(|e| ExtractError::corrupt("notebook", e.to_string()))?;
    let cells: Vec<&Cell> = if nb.cells.is_empty() {
        nb.worksheets.iter().flat_map(|w| w.cells.iter()).collect()
    } else {
        nb.cells.iter().collect()
    };

    let kernel_lang = nb
        .metadata
        .pointer("/language_info/name")
        .or_else(|| nb.metadata.pointer("/kernelspec/language"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            nb.metadata
                .pointer("/kernelspec/name")
                .and_then(Value::as_str)
                .map(|n| n.trim_end_matches(|c: char| c.is_ascii_digit()).to_string())
        })
        .unwrap_or_default()
        .to_lowercase();

    let mut parts: Vec<String> = Vec::new();
    let mut dropped_outputs = 0usize;
    for cell in cells {
        match cell.cell_type.as_str() {
            "markdown" => {
                let src = joined(&cell.source);
                let t = src.trim_end();
                if !t.trim().is_empty() {
                    parts.push(t.trim_start_matches(['\n', '\r']).to_string());
                }
            }
            "heading" => {
                let t = joined(&cell.source);
                let t = t.trim();
                if !t.is_empty() {
                    parts.push(format!("{} {t}", "#".repeat(cell.level.unwrap_or(1).clamp(1, 6) as usize)));
                }
            }
            "raw" => {
                let src = joined(&cell.source);
                if !src.trim().is_empty() {
                    parts.push(src.trim().to_string());
                }
            }
            "code" => {
                let src = if cell.source.is_null() { joined(&cell.input) } else { joined(&cell.source) };
                let lang = cell.language.clone().unwrap_or_else(|| kernel_lang.clone());
                let lines = fenced(&lang, &src);
                if !lines.is_empty() {
                    parts.push(lines.join("\n"));
                }
                for out in &cell.outputs {
                    match short_text_output(out) {
                        Some(t) => {
                            let lines = fenced("output", &t);
                            if !lines.is_empty() {
                                parts.push(lines.join("\n"));
                            }
                        }
                        None => dropped_outputs += 1,
                    }
                }
            }
            _ => {}
        }
    }
    if dropped_outputs > 0 {
        warnings.push(format!("dropped {dropped_outputs} non-text or long outputs"));
    }
    let mut markdown = parts.join("\n\n");
    if markdown.is_empty() {
        return Err(ExtractError::convert("notebook", "no cell had any content"));
    }
    markdown.push('\n');
    let a = analyze_markdown(&markdown);
    let mut metadata = a.metadata;
    metadata.title = nb
        .metadata
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| a.headings.iter().find(|h| h.level == 1).map(|h| h.text.clone()));
    if let Some(authors) = nb.metadata.get("authors").and_then(Value::as_array) {
        metadata.authors = authors
            .iter()
            .filter_map(|a| a.get("name").and_then(Value::as_str).or_else(|| a.as_str()))
            .map(str::to_string)
            .collect();
    }
    Ok(Extracted {
        markdown,
        headings: a.headings,
        blocks: a.blocks,
        metadata,
        warnings,
        tool: tool_string("notebook"),
        encoding: Some(d.encoding.to_string()),
        ..Default::default()
    })
}

/// The text of a `stream`, `execute_result` or `display_data` output when it is short, else `None`.
fn short_text_output(out: &Value) -> Option<String> {
    let ty = out.get("output_type").and_then(Value::as_str).or_else(|| out.get("output_type").map(|_| ""))?;
    let text = match ty {
        "stream" => joined(out.get("text")?),
        "execute_result" | "display_data" | "pyout" => {
            let data = out.get("data").unwrap_or(out);
            joined(data.get("text/plain").or_else(|| data.get("text"))?)
        }
        "error" | "pyerr" => return None,
        _ => return None,
    };
    let t = text.trim_end().to_string();
    // Terminal control sequences are noise.
    let t = strip_ansi(&t);
    if t.trim().is_empty() || t.chars().count() > MAX_OUTPUT_CHARS || t.lines().count() > MAX_OUTPUT_LINES {
        return None;
    }
    Some(t)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BlockKind;

    fn run(json: &str) -> Extracted {
        convert(json.as_bytes(), &ConvertOptions::default()).unwrap()
    }

    const NB: &str = r###"{
 "cells": [
  {"cell_type": "markdown", "metadata": {}, "source": ["# Analysis of Things\n", "\n", "Some *intro* text with $x^2$.\n"]},
  {"cell_type": "code", "execution_count": 1, "metadata": {}, "source": ["import math\n", "print(math.pi)"],
   "outputs": [{"output_type": "stream", "name": "stdout", "text": ["3.141592653589793\n"]}]},
  {"cell_type": "code", "execution_count": 2, "metadata": {}, "source": "1 + 1",
   "outputs": [{"output_type": "execute_result", "execution_count": 2, "metadata": {}, "data": {"text/plain": ["2"], "image/png": "AAAA"}}]},
  {"cell_type": "code", "execution_count": 3, "metadata": {}, "source": ["plot()"],
   "outputs": [{"output_type": "display_data", "metadata": {}, "data": {"image/png": "iVBORw0KGgo=", "text/plain": ["<Figure size 640x480 with 1 Axes>"]}},
               {"output_type": "error", "ename": "ValueError", "evalue": "bad", "traceback": ["\u001b[0;31mValueError\u001b[0m"]}]},
  {"cell_type": "markdown", "metadata": {}, "source": "## Next section\n\nA code sample in markdown:\n\n```python\n# not a heading\n```\n"},
  {"cell_type": "raw", "metadata": {}, "source": "raw text cell"},
  {"cell_type": "code", "metadata": {}, "source": "", "outputs": [], "execution_count": null}
 ],
 "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}, "language_info": {"name": "python"}, "authors": [{"name": "Ann Lee"}]},
 "nbformat": 4, "nbformat_minor": 5
}"###;

    #[test]
    fn cells_become_markdown_and_fences() {
        let r = run(NB);
        let md = &r.markdown;
        assert!(md.starts_with("# Analysis of Things\n\nSome *intro* text with $x^2$.\n\n```python\nimport math\nprint(math.pi)\n```\n\n```output\n3.141592653589793\n```\n"), "{md}");
        assert!(md.contains("```python\n1 + 1\n```\n\n```output\n2\n```"), "{md}");
        assert!(md.contains("<Figure size 640x480 with 1 Axes>"), "{md}");
        assert!(md.contains("raw text cell"), "{md}");
        assert!(!md.contains("ValueError") && !md.contains("iVBOR"), "{md}");
        assert_eq!(r.metadata.title.as_deref(), Some("Analysis of Things"));
        assert_eq!(r.metadata.authors, vec!["Ann Lee"]);
    }

    #[test]
    fn headings_and_blocks_follow_the_final_text() {
        let r = run(NB);
        let hs: Vec<(u8, &str)> = r.headings.iter().map(|h| (h.level, h.text.as_str())).collect();
        assert_eq!(hs, vec![(1, "Analysis of Things"), (2, "Next section")]);
        let lines: Vec<&str> = r.markdown.lines().collect();
        for h in &r.headings {
            assert_eq!(lines[h.line as usize - 1], format!("{} {}", "#".repeat(h.level as usize), h.text));
        }
        let code: Vec<_> = r.blocks.iter().filter(|b| b.kind == BlockKind::Code).collect();
        assert!(code.len() >= 5, "{code:?}");
        for b in code {
            assert!(lines[b.start as usize - 1].starts_with("```") && lines[b.end as usize - 1].starts_with("```"));
        }
        assert!(r.warnings.iter().any(|w| w.contains("dropped")));
    }

    #[test]
    fn long_outputs_are_dropped() {
        let big = "x".repeat(MAX_OUTPUT_CHARS + 1);
        let many = (0..30).map(|i| i.to_string()).collect::<Vec<_>>().join("\\n");
        let nb = format!(
            r#"{{"cells":[{{"cell_type":"code","source":"run()","outputs":[{{"output_type":"stream","name":"stdout","text":"{big}"}},{{"output_type":"stream","name":"stdout","text":"{many}"}}]}}],"metadata":{{}},"nbformat":4}}"#
        );
        let r = run(&nb);
        assert_eq!(r.markdown, "```\nrun()\n```\n");
    }

    #[test]
    fn ansi_sequences_are_removed_from_outputs() {
        let nb = r#"{"cells":[{"cell_type":"code","source":"x","outputs":[{"output_type":"stream","name":"stdout","text":"\u001b[1mbold\u001b[0m done"}]}],"metadata":{},"nbformat":4}"#;
        assert!(run(nb).markdown.contains("```output\nbold done\n```"));
    }

    #[test]
    fn nbformat3_worksheets() {
        let nb = r#"{"worksheets":[{"cells":[{"cell_type":"heading","level":2,"source":["Old Heading"],"metadata":{}},{"cell_type":"code","language":"python","input":["print(1)"],"outputs":[{"output_type":"stream","stream":"stdout","text":["1\n"]}]}]}],"metadata":{},"nbformat":3}"#;
        let r = run(nb);
        assert_eq!(r.markdown, "## Old Heading\n\n```python\nprint(1)\n```\n\n```output\n1\n```\n");
    }

    #[test]
    fn broken_and_empty_notebooks_are_errors() {
        assert!(matches!(convert(b"{not json", &ConvertOptions::default()), Err(ExtractError::Corrupt { .. })));
        assert!(matches!(
            convert(br#"{"cells":[],"metadata":{},"nbformat":4}"#, &ConvertOptions::default()),
            Err(ExtractError::Convert { .. })
        ));
        assert!(matches!(convert(b"", &ConvertOptions::default()), Err(ExtractError::Empty)));
    }

    #[test]
    fn code_containing_backticks_gets_a_longer_fence() {
        let nb = r#"{"cells":[{"cell_type":"code","source":"s = '```'","outputs":[]}],"metadata":{"language_info":{"name":"python"}},"nbformat":4}"#;
        assert_eq!(run(nb).markdown, "````python\ns = '```'\n````\n");
    }
}

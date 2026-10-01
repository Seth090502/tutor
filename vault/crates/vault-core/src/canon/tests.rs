use super::*;
use proptest::prelude::*;
use std::collections::BTreeMap;

fn prose(input: &str) -> String {
    normalize_prose(input).text
}

/// Python's `str.splitlines()` (no `keepends`), reimplemented exactly.
fn py_splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut iter = s.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if matches!(
            c,
            '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{1C}' | '\u{1D}' | '\u{1E}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        ) {
            out.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' && matches!(iter.peek(), Some((_, '\n'))) {
                iter.next();
                end += 1;
            }
            start = end;
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn removed(c: char) -> bool {
    matches!(c, '\u{FEFF}' | '\u{200B}' | '\u{AD}')
        || (c < ' ' && !matches!(c, '\t' | '\n' | '\r' | '\u{0B}' | '\u{0C}'))
        || (('\u{80}'..='\u{9F}').contains(&c) && c != '\u{85}')
}

/// Non-whitespace characters, counted, after NFC and the step-4 removals; optionally ignoring `>`.
fn ink(s: &str, ignore_quote_markers: bool) -> BTreeMap<char, usize> {
    let nfc: String = s.nfc().collect();
    let kept: String = nfc.chars().filter(|&c| !removed(c)).collect();
    let mut m = BTreeMap::new();
    for c in kept.nfc() {
        if !c.is_whitespace() && !(ignore_quote_markers && c == '>') {
            *m.entry(c).or_default() += 1;
        }
    }
    m
}

// ------------------------------------------------------------------------------------------- unit cases

#[test]
fn splits_sentences_one_per_line() {
    assert_eq!(
        prose("Hello world. This is a test.\nAnother line here."),
        "Hello world.\nThis is a test.\nAnother line here.\n"
    );
    assert_eq!(prose("Is it? Yes! Done."), "Is it?\nYes!\nDone.\n");
}

#[test]
fn abbreviations_do_not_split() {
    assert_eq!(
        prose("See e.g. Smith et al. 2019 for details. Then Fig. 3 shows it. Compare cf. Jones. Dr. Who arrived."),
        "See e.g. Smith et al. 2019 for details.\nThen Fig. 3 shows it.\nCompare cf. Jones.\nDr. Who arrived.\n"
    );
    assert_eq!(prose("J. R. R. Tolkien wrote it. Next one."), "J. R. R. Tolkien wrote it.\nNext one.\n");
    assert_eq!(prose("Made in the U.S. Army camps. Then home."), "Made in the U.S. Army camps.\nThen home.\n");
    assert_eq!(prose("See pp. 12-14 and Vol. II of it. End."), "See pp. 12-14 and Vol. II of it.\nEnd.\n");
    assert_eq!(prose("He said Mrs. Smith left. Prof. Lee agreed."), "He said Mrs. Smith left.\nProf. Lee agreed.\n");
}

#[test]
fn inline_maths_and_code_spans_are_not_split() {
    assert_eq!(prose("We have $x = 1. Y$ here. Next sentence."), "We have $x = 1. Y$ here.\nNext sentence.\n");
    assert_eq!(prose("Run `a. B` now. Done."), "Run `a. B` now.\nDone.\n");
    assert_eq!(prose("Use ``x `y. Z` w`` here. Ok."), "Use ``x `y. Z` w`` here.\nOk.\n");
    assert_eq!(prose("Display $$a. B$$ inline. Ok."), "Display $$a. B$$ inline.\nOk.\n");
    // Dollar amounts are not maths: a closing `$` must follow a non-space.
    assert_eq!(prose("It costs $5. Then $10 more."), "It costs $5.\nThen $10 more.\n");
    assert_eq!(prose(r"Escaped \$x. Y\$ splits. Ok."), "Escaped \\$x.\nY\\$ splits.\nOk.\n");
}

#[test]
fn cjk_splits_after_full_stops() {
    assert_eq!(prose("你好。世界很大！真的吗？是的。"), "你好。\n世界很大！\n真的吗？\n是的。\n");
    assert_eq!(prose("「こんにちは。」と言った。次の文。"), "「こんにちは。」\nと言った。\n次の文。\n");
}

#[test]
fn no_split_without_whitespace_outside_cjk() {
    assert_eq!(prose("Version 1.World ends.Next part"), "Version 1.World ends.Next part\n");
}

#[test]
fn lists_keep_their_content_column() {
    assert_eq!(
        prose("- First sentence. Second sentence.\n- Another item. With two."),
        "- First sentence.\n  Second sentence.\n- Another item.\n  With two.\n"
    );
    assert_eq!(prose("10. Alpha one. Beta two."), "10. Alpha one.\n    Beta two.\n");
    assert_eq!(prose("-   Wide gap. Kept."), "-   Wide gap.\n    Kept.\n");
    assert_eq!(prose("- outer\n  - inner one. Inner two."), "- outer\n  - inner one.\n    Inner two.\n");
    assert_eq!(prose("* Lazy item\ncontinues here. Then more."), "* Lazy item continues here.\n  Then more.\n");
    assert_eq!(prose("1. Item\n\n   Second paragraph. Of item."), "1. Item\n\n   Second paragraph.\n   Of item.\n");
}

#[test]
fn quotes_keep_their_prefix() {
    assert_eq!(prose("> Quoted one. Quoted two.\n> Still quoted."), "> Quoted one.\n> Quoted two.\n> Still quoted.\n");
    assert_eq!(prose("> - Item one. Item two."), "> - Item one.\n>   Item two.\n");
    assert_eq!(prose("> > Deep one. Deep two."), "> > Deep one.\n> > Deep two.\n");
    assert_eq!(prose("> First para.\n>\n> Second. Para."), "> First para.\n>\n> Second.\n> Para.\n");
}

#[test]
fn verbatim_blocks_are_untouched() {
    let fence = "```python\nx  =  1   \n\tif x:  \n\n\n   y. Z\n```";
    assert_eq!(
        prose(&format!("Intro. Two.\n\n{fence}\n\nOutro. Two.")),
        format!("Intro.\nTwo.\n\n{fence}\n\nOutro.\nTwo.\n")
    );
    let unclosed = "~~~\nno  close.  Here.\n\n\nstill code";
    assert_eq!(prose(unclosed), format!("{unclosed}\n"));
    let maths = "$$\na. B  \\\\\n\nc\n$$";
    assert_eq!(prose("Text.\n\n$$\na. B  \\\\\nc\n$$"), "Text.\n\n$$\na. B  \\\\\nc\n$$\n");
    assert!(prose(maths).contains("$$\n"));
    assert_eq!(prose("$$x = 1.  Y$$\nAfter. Two."), "$$x = 1.  Y$$\nAfter.\nTwo.\n");
    let table = "| a.  B | c |\n|---|---|\n| d.  E | f |";
    assert_eq!(prose(&format!("{table}\nNext. Two.")), format!("{table}\nNext.\nTwo.\n"));
    let html = "<div class=\"x\">\nSome   text. More.\n</div>";
    assert_eq!(prose(&format!("{html}\n\nAfter. Two.")), format!("{html}\n\nAfter.\nTwo.\n"));
    let comment = "<!-- a. B\n\nc. D -->";
    assert_eq!(prose(&format!("{comment}\nAfter.")), format!("{comment}\nAfter.\n"));
}

#[test]
fn headings_and_rules_are_verbatim() {
    let c = normalize_prose("#  Title   with   spaces  \nBody one. Body two.\n## Sub ##\nText.\n\n---\n# Next\nEnd.");
    assert_eq!(c.text, "#  Title   with   spaces  \nBody one.\nBody two.\n## Sub ##\nText.\n\n---\n# Next\nEnd.\n");
    assert_eq!(c.headings.len(), 3);
    assert_eq!(
        (c.headings[0].level, c.headings[0].text.as_str(), c.headings[0].start, c.headings[0].end),
        (1, "Title   with   spaces", 1, 7)
    );
    assert_eq!(
        (c.headings[1].level, c.headings[1].text.as_str(), c.headings[1].start, c.headings[1].end),
        (2, "Sub", 4, 7)
    );
    assert_eq!((c.headings[2].start, c.headings[2].end), (8, 9));
    assert_eq!(prose("Title\n=====\nText. Two."), "Title\n=====\nText.\nTwo.\n");
    assert_eq!(prose("#hashtag is not a heading. Ok."), "#hashtag is not a heading.\nOk.\n");
}

#[test]
fn reference_sections_keep_one_entry_per_line() {
    let input = "# Intro\nText. More.\n## 7. References\nSmith, J. (2019). A paper. Journal 3.\n\
                 Doe, A. (2020).  Another.  Journal.\n\n### Sub\nLee, K. A b. C.\n# Next\nText. More.";
    let c = normalize_prose(input);
    assert_eq!(
        c.text,
        "# Intro\nText.\nMore.\n## 7. References\nSmith, J. (2019). A paper. Journal 3.\nDoe, A. (2020).  Another.  Journal.\n\n### Sub\nLee, K. A b. C.\n# Next\nText.\nMore.\n"
    );
    assert_eq!(c.blocks, vec![BlockSpan { kind: "references".into(), start: 5, end: 9 }]);
    assert!(is_reference_heading("**Bibliography**"));
    assert!(is_reference_heading("VII. Index:"));
    assert!(!is_reference_heading("References and further reading"));
}

#[test]
fn line_breaks_and_removals() {
    assert_eq!(prose("a\r\nb\rc\u{2028}d\u{85}e\u{0B}f\u{2029}g"), "a b c d e f g\n");
    assert_eq!(
        prose("soft\u{AD}hyphen zero\u{200B}width\u{FEFF} bom \u{7} bell\u{9F}"),
        "softhyphen zerowidth bom bell\n"
    );
    assert_eq!(prose("keep\u{200D}zwj and\u{200C}zwnj"), "keep\u{200D}zwj and\u{200C}zwnj\n");
    assert_eq!(prose("\u{FEFF}Start."), "Start.\n");
    assert_eq!(prose("e\u{301}"), "\u{E9}\n");
    assert_eq!(prose("e\u{AD}\u{301}"), "\u{E9}\n", "NFC must run again after removals");
    assert_eq!(prose("a\t\tb   c   "), "a b c\n");
    assert_eq!(prose("\n\n\nA.\n\n\n\nB.\n\n"), "A.\n\nB.\n");
    assert_eq!(prose(""), "");
    assert_eq!(prose(" \n\t\n"), "");
}

#[test]
fn form_feeds_record_page_starts() {
    let c = normalize_prose("Page one text.\u{C}Page two starts. More.\n\nPara.\u{C}\u{C}After two breaks.\u{C}");
    // A form feed is a line break, so two in a row leave an empty line: a paragraph break.
    assert_eq!(c.text, "Page one text.\nPage two starts.\nMore.\n\nPara.\n\nAfter two breaks.\n");
    assert_eq!(c.page_breaks, vec![2, 7, 7]);
    // The form feed falls inside a sentence that started on page one: the page starts on the line holding it.
    let c = normalize_prose("The sentence starts on one page\u{C}and ends on the next. Then more.");
    assert_eq!(c.text, "The sentence starts on one page and ends on the next.\nThen more.\n");
    assert_eq!(c.page_breaks, vec![1]);
    let c = normalize_prose("One.\n\u{C}\nTwo.");
    assert_eq!((c.text.as_str(), c.page_breaks.as_slice()), ("One.\n\nTwo.\n", &[3u32][..]));
}

#[test]
fn continuation_lines_never_start_a_block() {
    assert_eq!(prose("Step 2. 3. Then go."), "Step 2. 3.\nThen go.\n");
    assert_eq!(prose("It is item # 5. # Not a heading."), "It is item # 5. # Not a heading.\n");
    assert_eq!(
        prose("Done. - Not a list. > Not a quote. | Not a table."),
        "Done. - Not a list. > Not a quote. | Not a table.\n"
    );
    // `*** Fine.` is not a thematic break and a fence needs no backticks after it, so these may start lines.
    assert_eq!(prose("Ok. *** Fine. Next."), "Ok.\n*** Fine.\nNext.\n");
    // (UAX #29 rule SB8 already keeps `Ok. ```code` together because a lowercase letter follows.)
    assert_eq!(prose("Ok. ```Code``` here. <B>Bold</B> too."), "Ok.\n```Code``` here. <B>Bold</B> too.\n");
}

#[test]
fn indented_paragraphs_keep_their_indent() {
    assert_eq!(prose("    Indented one. Indented two."), "    Indented one.\n    Indented two.\n");
    assert_eq!(prose("\tTabbed one. Tabbed two."), "    Tabbed one.\n    Tabbed two.\n");
}

#[test]
fn long_lines_split_at_the_last_space_that_fits() {
    let words: Vec<String> = (0..1500).map(|i| format!("w{i}")).collect();
    let sentence = format!("{}.", words.join(" "));
    let out = prose(&sentence);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() >= 2);
    assert!(lines.iter().all(|l| l.chars().count() <= MAX_LINE_CHARS));
    assert_eq!(lines.join(" "), sentence);
    assert_eq!(prose(&out), out);
    // No whitespace to split at: the line stays whole rather than breaking a word.
    let token = "x".repeat(MAX_LINE_CHARS + 10);
    assert_eq!(prose(&token), format!("{token}\n"));
    // A list item's continuation indent counts toward the limit.
    let item = format!("- {sentence}");
    assert!(prose(&item).lines().all(|l| l.chars().count() <= MAX_LINE_CHARS));
    // Table rows are verbatim and never split.
    let row = format!("| {} |", "y ".repeat(3000));
    assert_eq!(prose(&row), format!("{row}\n"));
}

#[test]
fn a_tricky_document_is_stable() {
    let doc = "\u{FEFF}# On Memory\r\n\r\nThe magical number is 7 ± 2 (Miller, 1956). Cowan et al. argued for 4; cf. Fig. 2.\r\n\
               Inline $a_1 = b. C$ maths and `x. Y` code.\u{C}Next page here.\r\n\r\n\
               > Quoted text. Two sentences.\r\n> - with a list. In it.\r\n\r\n\
               1. First. Item.\r\n   - nested. Item.\r\n\r\n\
               | col. A | col. B |\r\n|---|---|\r\n\r\n\
               ```rust\r\nfn main() {  }\t\r\n```\r\n\r\n\
               中文句子。第二句！Mixed English. 日本語の文。\r\n\r\n## References\r\n[1] A. Author. Title. 2020.\r\n";
    let c = normalize_prose(doc);
    assert_eq!(normalize_prose(&c.text).text, c.text);
    assert!(c.text.contains("fn main() {  }\t\n"), "code kept verbatim except CRLF");
    assert!(c.text.contains("[1] A. Author. Title. 2020.\n"));
    assert!(c.text.contains("> Quoted text.\n> Two sentences.\n"));
    assert!(c.text.contains("中文句子。\n第二句！\nMixed English.\n日本語の文。\n"));
    assert_eq!(c.page_breaks.len(), 1);
    assert_eq!(c.headings.iter().map(|h| h.text.as_str()).collect::<Vec<_>>(), vec!["On Memory", "References"]);
    assert!(c.blocks.iter().any(|b| b.kind == "table") && c.blocks.iter().any(|b| b.kind == "code"));
}

/// The repository's own Markdown (tables, nested lists, fences, links) holds every property.
#[test]
fn real_markdown_documents_hold_the_properties() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    let docs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let mut files = Vec::new();
    walk(&docs, &mut files);
    assert!(files.len() >= 10, "expected the repository docs at {}", docs.display());
    for f in files {
        let (_, c) = canonical_prose(&std::fs::read(&f).unwrap());
        let raw = decode(&std::fs::read(&f).unwrap()).text;
        assert_eq!(normalize_prose(&c.text).text, c.text, "{} is not stable", f.display());
        assert_eq!(ink(&c.text, true), ink(&raw, true), "{} lost text", f.display());
        assert_eq!(py_splitlines(&c.text), c.text.lines().collect::<Vec<_>>(), "{}", f.display());
    }
}

// -------------------------------------------------------------------------------------------- decoding

#[test]
fn decodes_boms_utf16_and_legacy_encodings() {
    let mut le = vec![0xFF, 0xFE];
    le.extend("héllo\n".encode_utf16().flat_map(u16::to_le_bytes));
    assert_eq!(decode(&le), Decoded { text: "héllo\n".into(), encoding: "utf-16le".into(), had_errors: false });

    let be: Vec<u8> = "plain ascii text here\n".encode_utf16().flat_map(u16::to_be_bytes).collect();
    let d = decode(&be);
    assert_eq!((d.text.as_str(), d.encoding.as_str()), ("plain ascii text here\n", "utf-16be"));

    let mut u32le = vec![0xFF, 0xFE, 0, 0];
    u32le.extend("x€".chars().flat_map(|c| (c as u32).to_le_bytes()));
    assert_eq!(decode(&u32le).text, "x€");
    assert_eq!(decode(&u32le).encoding, "utf-32le");

    let utf8_bom = [&[0xEF, 0xBB, 0xBF][..], "café".as_bytes()].concat();
    assert_eq!(decode(&utf8_bom).text, "café");

    let latin = b"Le caf\xe9 \xe9tait ferm\xe9 pendant l'\xe9t\xe9, et la client\xe8le \xe9tait d\xe9\xe7ue.";
    let d = decode(latin);
    assert_eq!(d.text, "Le café était fermé pendant l'été, et la clientèle était déçue.");
    assert_eq!(d.encoding, "windows-1252");
    assert!(!d.had_errors);
}

#[test]
fn code_changes_only_line_endings_and_bom() {
    assert_eq!(normalize_code("\u{FEFF}a\r\nb\rc\t \n\u{C}d  "), "a\nb\nc\t \n\u{C}d  ");
    assert_eq!(normalize_code("no newline"), "no newline");
    let (d, t) = canonical_code(b"\xEF\xBB\xBFx = 1\r\n");
    assert_eq!((d.encoding.as_str(), t.as_str()), ("utf-8", "x = 1\n"));
    assert_eq!(split_lines("a\nb"), vec!["a", "b"]);
    assert_eq!(split_lines("a\nb\n"), vec!["a", "b"]);
    assert_eq!(split_lines("a\n\n"), vec!["a", ""]);
    assert!(split_lines("").is_empty());
}

// ------------------------------------------------------------------------------------------ properties

fn token(with_quotes: bool) -> BoxedStrategy<String> {
    let mut specials = vec![
        ".",
        ",",
        "!",
        "?",
        ";",
        ":",
        "\"",
        "'",
        "(",
        ")",
        "e.g.",
        "i.e.",
        "et al.",
        "Fig.",
        "U.S.",
        "A.",
        "p.",
        "No.",
        "$",
        "$$",
        "`",
        "``",
        "*",
        "-",
        "+",
        "#",
        "##",
        "|",
        "<div>",
        "<",
        "1.",
        "2)",
        "10.",
        "~~~",
        "```",
        "===",
        "---",
        "***",
        "。",
        "！",
        "？",
        "你好",
        "世界",
        "「",
        "」",
        "\u{a0}",
        "\u{3000}",
        "\t",
        "\u{ad}",
        "\u{200b}",
        "\u{feff}",
        "\u{200d}",
        "\u{301}",
        "e\u{301}",
        "\u{7}",
        "\u{85}",
        "\u{2028}",
        "\u{c}",
        "\r",
        "\r\n",
        "References",
        "Index",
        "\\",
        "[1]",
        "–",
        "…",
        "<!--",
        "-->",
        "    ",
    ];
    if with_quotes {
        specials.extend([">", "> ", ">>"]);
    }
    let specials: Vec<String> = specials.into_iter().map(String::from).collect();
    prop_oneof![
        8 => "[a-zA-Z]{1,8}",
        2 => "[0-9]{1,4}",
        4 => prop::sample::select(specials),
    ]
    .boxed()
}

fn separator(with_quotes: bool) -> BoxedStrategy<&'static str> {
    let mut seps = vec![" ", " ", " ", " ", "", "\n", "\n\n", "  ", "\t", "\n- ", "\n# ", "\n    ", "\r\n", "\n1. "];
    if with_quotes {
        seps.extend(["\n> ", "\n> > ", "\n>\n"]);
    }
    prop::sample::select(seps).boxed()
}

fn document(with_quotes: bool) -> impl Strategy<Value = String> {
    prop::collection::vec((token(with_quotes), separator(with_quotes)), 0..120)
        .prop_map(|parts| parts.into_iter().map(|(t, s)| t + s).collect())
}

proptest! {
    // PROPTEST_CASES raises the count for a deeper run.
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(2000),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prose_is_idempotent(doc in document(true), limit in prop::sample::select(vec![12usize, 40, 4000])) {
        let once = normalize_prose_with_limit(&doc, limit).text;
        let twice = normalize_prose_with_limit(&once, limit).text;
        prop_assert_eq!(&twice, &once, "input {:?}", doc);
    }

    #[test]
    fn prose_loses_no_text(doc in document(false), limit in prop::sample::select(vec![12usize, 4000])) {
        let out = normalize_prose_with_limit(&doc, limit).text;
        prop_assert_eq!(ink(&out, false), ink(&doc, false), "input {:?}\noutput {:?}", doc, out);
    }

    #[test]
    fn prose_with_quotes_loses_no_text_but_quote_markers(doc in document(true)) {
        let out = normalize_prose_with_limit(&doc, 40).text;
        prop_assert_eq!(ink(&out, true), ink(&doc, true), "input {:?}\noutput {:?}", doc, out);
    }

    #[test]
    fn python_and_rust_agree_on_lines(doc in document(true)) {
        let c = normalize_prose_with_limit(&doc, 40);
        let rust: Vec<&str> = c.text.lines().collect();
        prop_assert_eq!(py_splitlines(&c.text), rust.clone());
        prop_assert_eq!(c.lines as usize, rust.len());
        prop_assert!(c.text.is_empty() || (c.text.ends_with('\n') && !c.text.ends_with("\n\n") && !c.text.starts_with('\n')));
        for &p in &c.page_breaks {
            prop_assert!(p >= 1 && p <= c.lines);
        }
        for h in &c.headings {
            prop_assert!(h.start >= 1 && h.start <= h.end && h.end <= c.lines);
        }
        for b in &c.blocks {
            prop_assert!(b.start >= 1 && b.start <= b.end && b.end <= c.lines);
        }
    }

    #[test]
    fn fenced_blocks_survive_verbatim(
        before in "[A-Za-z .,]{0,200}",
        after in "[A-Za-z .,]{0,200}",
        body in prop::collection::vec("[ -_a-~\t\u{e9}\u{4e2d}]{0,40}", 0..8),
    ) {
        let body: Vec<String> = body.into_iter().filter(|l| fence_open(l.trim_start()).is_none() && !l.trim_start().starts_with("```")).collect();
        let block = format!("```text\n{}\n```", body.join("\n"));
        let doc = format!("{before}\n\n{block}\n\n{after}");
        let out = normalize_prose(&doc).text;
        prop_assert!(out.contains(&format!("{block}\n")), "block {:?} not verbatim in {:?}", block, out);
    }

    #[test]
    fn code_is_idempotent_and_keeps_every_line(text in "[a-z \t\r\n\u{feff}\u{c}\u{2028}é]{0,200}") {
        let once = normalize_code(&text);
        prop_assert_eq!(normalize_code(&once), once.clone());
        let original_lines = text.trim_start_matches('\u{FEFF}').replace("\r\n", "\n").replace('\r', "\n");
        prop_assert_eq!(&once, &original_lines);
        prop_assert!(!once.contains('\r'));
    }
}

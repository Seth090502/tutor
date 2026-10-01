use super::*;
use crate::sidecar::{Categories, CodeInfo};
use crate::testkit::{PAPER, TestVault, block_on, prose_request};
use crate::writer::{PublishOutcome, PublishRequest, Repo, Supersede, Writer};

struct Fixture {
    v: TestVault,
    paper: PublishOutcome,
    text: String,
}

fn fixture() -> (Fixture, Vault) {
    let v = TestVault::new();
    let w = v.writer();
    let mut req = prose_request(PAPER, "Working memory capacity");
    req.metadata.authors = vec!["Nelson Cowan".into()];
    req.metadata.year = Some(2001);
    req.metadata.language = Some("en".into());
    req.metadata.categories = Some(Categories {
        primary: Some("psych-cogsci".into()),
        areas: vec!["memory".into()],
        openalex_topic: Some("T10001".into()),
        ..Categories::default()
    });
    req.abstract_text = Some("Short-term memory holds about four chunks.".into());
    let text = req.text.clone();
    let paper = w.publish(req).unwrap();
    drop(w);
    let vault = Vault::open(v.config()).unwrap();
    (Fixture { v, paper, text }, vault)
}

fn read(vault: &Vault, addresses: &[String], expand: Option<Expand>, max_tokens: Option<u32>) -> ReadResponse {
    block_on(vault.read(ReadRequest { addresses: addresses.to_vec(), expand, max_tokens })).unwrap()
}

fn lines(text: &str) -> Vec<&str> {
    split_lines(text)
}

#[test]
fn reads_every_line_exactly_by_any_form_of_the_id() {
    let (f, vault) = fixture();
    let all = lines(&f.text);
    for (i, line) in all.iter().enumerate() {
        let n = i + 1;
        let r = read(&vault, &[format!("{}#L{n}", f.paper.doc_id)], None, None);
        assert_eq!(r.passages[0].text, *line, "line {n}");
    }
    let hash_only = format!("{}#L1-L{}", f.paper.doc_id.hash(), all.len());
    let wrong_slug = format!("{}-wrong-slug#L1-L{}", f.paper.doc_id.hash(), all.len());
    for a in [hash_only, wrong_slug] {
        let r = read(&vault, &[a], None, None);
        assert_eq!(r.passages[0].text, f.text.trim_end_matches('\n'));
        assert_eq!(r.passages[0].address.doc, f.paper.doc_id, "responses carry the canonical id");
        assert_eq!(r.passages[0].title.as_deref(), Some("Working memory capacity"));
    }
}

#[test]
fn per_item_errors_do_not_fail_the_batch() {
    let (f, vault) = fixture();
    let n = lines(&f.text).len();
    let r = read(
        &vault,
        &[
            "not an address".into(),
            "0123456789abcdef#L1".into(),
            format!("{}#L{}", f.paper.doc_id, n + 1),
            format!("{}#L2", f.paper.doc_id),
            r"C:\vault\hot\text\x.md".into(),
        ],
        None,
        None,
    );
    assert_eq!(r.passages.len(), 1);
    assert_eq!(r.errors.iter().map(|e| e.index).collect::<Vec<_>>(), vec![0, 1, 2, 4]);
    assert!(r.errors[1].message.contains("unknown document"));
    assert!(r.errors[2].message.contains(&format!("lines 1-{n}")));
    assert!(block_on(vault.read(ReadRequest { addresses: vec![], expand: None, max_tokens: None })).is_err());
    assert!(
        block_on(vault.read(ReadRequest { addresses: vec!["x".into()], expand: None, max_tokens: Some(0) })).is_err()
    );
}

#[test]
fn expands_by_lines_and_by_section_with_pages_and_section_paths() {
    let (f, vault) = fixture();
    let all = lines(&f.text);
    let find = |needle: &str| all.iter().position(|l| l.contains(needle)).unwrap() as u32 + 1;
    let detail = find("A detail sentence.");
    let r =
        read(&vault, &[format!("{}#L{detail}", f.paper.doc_id)], Some(Expand::Lines { before: 1, after: 100 }), None);
    let p = &r.passages[0];
    assert_eq!((p.address.start, p.address.end), (detail - 1, all.len() as u32), "clamped to the document");
    // The first line is the blank line under `### Details`, so it is already in that section.
    assert_eq!(p.section.as_deref(), Some("Working memory capacity > Evidence > Details"), "section of the first line");

    let r = read(&vault, &[format!("{}#L{detail}", f.paper.doc_id)], Some(Expand::Section), None);
    let p = &r.passages[0];
    let details_heading = find("### Details");
    assert_eq!(p.address.start, details_heading);
    assert!(p.text.starts_with("### Details") && p.text.contains("Another detail."));
    assert_eq!(p.section.as_deref(), Some("Working memory capacity > Evidence > Details"));
    assert_eq!(p.page.as_deref(), Some("2"), "after the form feed");

    let first = read(&vault, &[format!("{}#L1", f.paper.doc_id)], Some(Expand::Lines { before: 5, after: 0 }), None);
    assert_eq!((first.passages[0].address.start, first.passages[0].page.as_deref()), (1, Some("1")));
}

#[test]
fn budgets_truncate_with_a_way_to_get_the_rest() {
    let (f, vault) = fixture();
    let n = lines(&f.text).len() as u32;
    let whole = format!("{}#L1-L{n}", f.paper.doc_id);
    let r = read(&vault, &[whole.clone(), format!("{}#L1", f.paper.doc_id)], None, Some(120));
    let t = r.truncated.as_ref().expect("truncated");
    let first = &r.passages[0];
    assert!(first.address.end < n);
    assert!(t.next.contains(&format!("{}#L{}", f.paper.doc_id, first.address.end + 1)), "{}", t.next);
    assert!(t.next.contains(&format!("{}#L1\"", f.paper.doc_id)), "unserved addresses are listed: {}", t.next);
    // Following the instructions reassembles the exact text.
    let mut got = vec![first.text.clone()];
    let mut start = first.address.end + 1;
    while start <= n {
        let r = read(&vault, &[format!("{}#L{start}-L{n}", f.paper.doc_id)], None, Some(120));
        let p = &r.passages[0];
        got.push(p.text.clone());
        start = p.address.end + 1;
    }
    assert_eq!(got.join("\n"), f.text.trim_end_matches('\n'));
    // A budget too small for even one line says so instead of returning a partial line.
    let r = read(&vault, &[whole], None, Some(1));
    assert!(r.passages.is_empty() && r.errors[0].message.contains("max_tokens"));
    // max_tokens above the hard cap is clamped, not rejected.
    assert!(read(&vault, &[format!("{}#L1", f.paper.doc_id)], None, Some(1_000_000)).truncated.is_none());
}

#[test]
fn superseded_documents_still_read_and_point_forward() {
    let (f, vault) = fixture();
    drop(vault);
    let w = f.v.writer();
    let mut req = prose_request(&PAPER.replace("four", "four or so"), "Working memory capacity");
    req.supersedes = Some(Supersede { doc: f.paper.doc_id.to_string(), reason: "re-extraction".into() });
    let new = w.publish(req).unwrap();
    drop(w);
    let vault = Vault::open(f.v.config()).unwrap();
    let r = read(&vault, &[format!("{}#L1-L3", f.paper.doc_id)], None, None);
    assert_eq!(r.passages[0].text, lines(&f.text)[..3].join("\n"));
    assert_eq!(r.passages[0].superseded_by.as_ref(), Some(&new.doc_id));
}

#[test]
fn damaged_text_is_reported_not_served() {
    let (f, vault) = fixture();
    let path = f.v.config().layout.hot.resolve_existing(&f.paper.rel_path).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[3] ^= 0x20;
    std::fs::write(&path, &bytes).unwrap();
    let r = read(&vault, &[format!("{}#L1", f.paper.doc_id)], None, None);
    assert!(r.passages.is_empty());
    assert!(r.errors[0].message.contains("integrity"));
    assert!(!r.errors[0].message.contains(":\\"), "no paths in tool output");
}

#[test]
fn outlines_show_headings_pages_and_symbols() {
    let (f, vault) = fixture();
    drop(vault);
    let w = f.v.writer();
    w.upsert_repo(&Repo { repo_id: "demo@1".into(), name: "demo".into(), ..Repo::default() }).unwrap();
    let mut code = PublishRequest::new("def f():\n    return 1\n\nclass C:\n    pass\n".into(), DocKind::Code);
    code.code = Some(CodeInfo {
        repo: Some("demo@1".into()),
        path: "pkg/mod.py".into(),
        language: Some("python".into()),
        swhid: None,
        symbols: vec![
            Symbol { kind: "function".into(), name: "f".into(), start: 1, end: 2, parent: None },
            Symbol { kind: "class".into(), name: "C".into(), start: 4, end: 5, parent: None },
        ],
    });
    let code_out = w.publish(code).unwrap();
    drop(w);
    let vault = Vault::open(f.v.config()).unwrap();
    let r = block_on(vault.outline(OutlineRequest {
        doc_ids: vec![f.paper.doc_id.hash().into(), code_out.doc_id.to_string(), "bad".into()],
        max_depth: Some(2),
        max_tokens: None,
    }))
    .unwrap();
    assert_eq!(r.errors.len(), 1);
    let paper = &r.outlines[0];
    assert!(paper.headings.iter().all(|h| h.level <= 2));
    assert_eq!(
        paper.headings.iter().map(|h| h.text.as_str()).collect::<Vec<_>>(),
        vec!["Working memory capacity", "Evidence", "References"]
    );
    assert_eq!(paper.pages.len(), 2);
    let c = &r.outlines[1];
    assert_eq!((c.doc.repo.as_deref(), c.doc.path.as_deref()), (Some("demo"), Some("pkg/mod.py")));
    assert_eq!(c.symbols.len(), 2);
    // Section expansion for code uses the innermost symbol.
    let rr = read(&vault, &[format!("{}#L2", code_out.doc_id)], Some(Expand::Section), None);
    assert_eq!(
        (rr.passages[0].address.start, rr.passages[0].address.end, rr.passages[0].section.as_deref()),
        (1, 2, Some("f"))
    );
    // A tiny budget shrinks the first outline and says what was cut.
    let small = block_on(vault.outline(OutlineRequest {
        doc_ids: vec![f.paper.doc_id.to_string()],
        max_depth: None,
        max_tokens: Some(120),
    }))
    .unwrap();
    assert!(small.truncated.is_some());
}

#[test]
fn find_docs_ranks_metadata_and_applies_filters() {
    let (f, vault) = fixture();
    let q = |queries: &[&str], filters: Filters| {
        block_on(vault.find_docs(FindDocsRequest {
            queries: queries.iter().map(|s| s.to_string()).collect(),
            filters,
            k: None,
            max_tokens: None,
        }))
    };
    let r = q(&["working memory", "cowan", "chunks", "Evidence", "!!!", "nothing-matches-zzz"], Filters::default())
        .unwrap();
    for i in 0..4 {
        assert_eq!(r.results[i].len(), 1, "query {i}");
        assert_eq!(r.results[i][0].doc_id, f.paper.doc_id);
        assert!(r.results[i][0].score.unwrap() > 0.0);
    }
    assert_eq!((r.errors.len(), r.errors[0].index), (1, 4));
    assert!(r.results[5].is_empty());
    let hit = |filters: Filters| q(&["memory"], filters).unwrap().results[0].len();
    assert_eq!(hit(Filters { category: Some("psych-cogsci".into()), ..Filters::default() }), 1);
    assert_eq!(hit(Filters { category: Some("medicine".into()), ..Filters::default() }), 0);
    assert_eq!(hit(Filters { area: Some("memory".into()), ..Filters::default() }), 1);
    assert_eq!(hit(Filters { kind: Some(DocKind::Book), ..Filters::default() }), 0);
    assert_eq!(hit(Filters { year_from: Some(2000), year_to: Some(2001), ..Filters::default() }), 1);
    assert_eq!(hit(Filters { year_from: Some(2002), ..Filters::default() }), 0);
    assert_eq!(hit(Filters { language: Some("zh".into()), ..Filters::default() }), 0);
    assert_eq!(hit(Filters { openalex_topic: Some("T10001".into()), ..Filters::default() }), 1);
    assert_eq!(hit(Filters { batch: Some("nope".into()), ..Filters::default() }), 0);
    assert_eq!(hit(Filters { repo: Some("nope".into()), ..Filters::default() }), 0);
    assert!(
        q(
            &["memory"],
            Filters { category: Some("medicine".into()), area: Some("memory".into()), ..Filters::default() }
        )
        .is_err()
    );
}

#[test]
fn vault_info_counts_and_reports_disks() {
    let (f, vault) = fixture();
    let taxonomy = r#"{"format":1,"version":"2026-10-01.1","categories":[
        {"slug":"psych-cogsci","name":"Psychology and cognitive science","targets":{"pilot":{"papers":3150,"books":350}}},
        {"slug":"engineering","name":"Engineering","targets":{"pilot":{"papers":630,"books":70}}}]}"#;
    let dir = f.v.root.join("hot").join("meta").join("taxonomy");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("categories.json"), taxonomy).unwrap();
    let info = block_on(vault.vault_info()).unwrap();
    assert_eq!((info.vault_id.as_str(), info.format_version, info.stage.as_str()), ("62171af3-test", 1, "local"));
    assert_eq!((info.documents, info.quarantined, info.repositories), (1, 0, 0));
    assert_eq!(info.by_kind, vec![(DocKind::Paper, 1)]);
    assert_eq!(info.coverage.len(), 2);
    assert_eq!((info.coverage[0].papers, info.coverage[0].target_papers), (1, 3150));
    assert_eq!(info.taxonomy_version, "2026-10-01.1");
    assert_eq!(info.index_version, "none");
    assert_eq!(info.tools_available, vec!["read", "outline", "find_docs", "vault_info"]);
    assert_eq!(info.disk.iter().map(|d| d.tier.as_str()).collect::<Vec<_>>(), vec!["hot", "cold", "inbox"]);
    assert!(info.disk.iter().all(|d| d.total_bytes > 0 && d.used_bytes <= d.total_bytes));
}

#[test]
fn later_phase_tools_say_when_they_arrive() {
    let (_f, vault) = fixture();
    let filters = Filters::default();
    let not_yet =
        |r: std::result::Result<(), ApiError>| matches!(r, Err(ApiError::NotYetAvailable(m)) if m.contains("Phase"));
    assert!(not_yet(
        block_on(vault.grep(GrepRequest {
            patterns: vec!["x".into()],
            scope: Scope::Vault,
            context_lines: 0,
            max_hits: None,
            case_insensitive: false,
            max_tokens: None
        }))
        .map(|_| ())
    ));
    assert!(not_yet(
        block_on(vault.search(SearchRequest { queries: vec![], filters: filters.clone(), k: None, max_tokens: None }))
            .map(|_| ())
    ));
    assert!(not_yet(
        block_on(vault.workspace(WorkspaceRequest { action: WorkspaceAction::List, doc_ids: vec![], workspace: None }))
            .map(|_| ())
    ));
    assert!(not_yet(
        block_on(vault.citations(CitationsRequest {
            doc_ids: vec![],
            direction: Direction::Both,
            k: None,
            max_tokens: None
        }))
        .map(|_| ())
    ));
    assert!(not_yet(
        block_on(vault.define(DefineRequest { terms: vec![], filters, k: None, max_tokens: None })).map(|_| ())
    ));
}

#[test]
fn concurrent_calls_each_get_exact_answers() {
    let (f, vault) = fixture();
    let vault = std::sync::Arc::new(vault);
    let n = lines(&f.text).len();
    let handles: Vec<_> = (0..8)
        .map(|t| {
            let vault = std::sync::Arc::clone(&vault);
            let id = f.paper.doc_id.to_string();
            std::thread::spawn(move || {
                for k in 0..20 {
                    let line = (t * 7 + k) % n + 1;
                    let r = block_on(vault.read(ReadRequest {
                        addresses: vec![format!("{id}#L{line}")],
                        expand: None,
                        max_tokens: None,
                    }))
                    .unwrap();
                    assert_eq!(r.passages.len(), 1);
                    if k % 5 == 0 {
                        block_on(vault.vault_info()).unwrap();
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert!(vault.idle.lock().unwrap().len() <= POOL_IDLE_MAX);
}

#[test]
fn opening_requires_an_initialised_vault() {
    let v = TestVault::new();
    assert!(matches!(Vault::open(v.config()), Err(CoreError::Config(m)) if m.contains("run intake")));
    drop(Writer::open(v.config()).unwrap());
    assert!(Vault::open(v.config()).is_ok());
}

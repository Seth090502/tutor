//! End to end through the public API: publish, read every address exactly, idempotent re-publish, supersede,
//! verify and tamper detection, then move the vault to another root and read everything again with no rebuild.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use vault_api::{Address, DocKind, PageMark, ReadRequest, ReadResponse, Symbol, VaultApi};
use vault_core::canon;
use vault_core::ids::blake3_hex;
use vault_core::sidecar::{CodeInfo, Extraction, Provenance, SourceRef};
use vault_core::store;
use vault_core::writer::{PublishOutcome, PublishRequest, PublishStatus, Repo, StoreOriginal, Supersede, Writer};
use vault_core::{Vault, VaultConfig};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn make_vault(root: &Path) {
    for tier in ["hot", "cold", "inbox"] {
        std::fs::create_dir_all(root.join(tier)).unwrap();
    }
    std::fs::write(
        root.join("vault.toml"),
        "format_version = 1\nvault_id = \"integration-vault\"\n[tiers]\nhot = \"hot\"\ncold = \"cold\"\ninbox = \"inbox\"\n",
    )
    .unwrap();
}

fn config_for(root: &Path, config_dir: &Path, name: &str) -> VaultConfig {
    let path = config_dir.join(name);
    std::fs::write(&path, format!("stage = \"local\"\nmachine = \"{name}\"\n[vault]\nroot = '{}'\n", root.display()))
        .unwrap();
    VaultConfig::from_file(&path).unwrap()
}

const DOCS: [(&str, &str); 3] = [
    (
        "Working Memory",
        "# Working memory\r\n\r\nThe magical number is four. Cowan et al. argued so; cf. Fig. 2.\r\n\
         Inline $x = 1. Y$ maths stays whole.\u{C}A new page starts here. It ends.\r\n\r\n\
         - First item. Second sentence.\r\n- Another item.\r\n\r\n> Quoted. Twice.\r\n\r\n\
         | a | b |\r\n|---|---|\r\n\r\n## References\r\nCowan, N. (2001). The magical number 4. BBS.\r\n",
    ),
    ("認知心理学の基礎", "# 第一章\n\n記憶は重要である。注意も重要だ！本当か？そうだ。\n\n```python\nx  =  1\t\n```\n"),
    (
        "A plain note",
        "\u{FEFF}Plain text without headings.  Two   spaces collapse.\n\n\n\nA second paragraph, e.g. this one. Done.",
    ),
];

fn prose(writer: &Writer, dir: &Path, i: usize, title: &str, raw: &str, batch: &str) -> PublishRequest {
    let source = dir.join(format!("doc{i}.md"));
    std::fs::write(&source, raw).unwrap();
    let b3 = blake3_hex(raw.as_bytes());
    let stored = writer
        .store_original(&StoreOriginal {
            source: &source,
            blake3: b3.clone(),
            ext: "md".into(),
            format: Some("markdown".into()),
            batch: Some(batch.into()),
        })
        .unwrap();
    let (decoded, c) = canon::canonical_prose(raw.as_bytes());
    let mut req = PublishRequest::new(c.text, DocKind::Paper);
    req.metadata.title = Some(title.into());
    req.metadata.provenance = vec![Provenance {
        batch: batch.into(),
        received: "2026-10-02".into(),
        path: format!("doc{i}.md"),
        manifest: serde_json::json!({}),
    }];
    req.headings = c.headings;
    req.blocks = c.blocks;
    req.pages = std::iter::once(1)
        .chain(c.page_breaks.iter().copied())
        .enumerate()
        .map(|(k, line)| PageMark { label: (k + 1).to_string(), index: (k + 1) as u32, line })
        .collect();
    req.extraction = Extraction {
        path: "markdown".into(),
        tool: "integration".into(),
        extract_fp: "fp1".into(),
        encoding: decoded.encoding,
    };
    req.sources = vec![SourceRef { original: b3, ext: "md".into(), bytes: stored.bytes }];
    req
}

fn code(writer: &Writer) -> PublishRequest {
    writer.upsert_repo(&Repo { repo_id: "demo@abc".into(), name: "demo".into(), ..Repo::default() }).unwrap();
    let (_, text) = canon::canonical_code(b"\xEF\xBB\xBFdef main():\r\n\treturn 1  \r\n\r\n# no final newline");
    let mut req = PublishRequest::new(text, DocKind::Code);
    req.code = Some(CodeInfo {
        repo: Some("demo@abc".into()),
        path: "src/main.py".into(),
        language: Some("python".into()),
        swhid: None,
        symbols: vec![Symbol { kind: "function".into(), name: "main".into(), start: 1, end: 2, parent: None }],
    });
    req.extraction = Extraction {
        path: "code".into(),
        tool: "integration".into(),
        extract_fp: "fp1".into(),
        encoding: "utf-8".into(),
    };
    req
}

fn read(vault: &Vault, addresses: Vec<String>) -> ReadResponse {
    block_on(vault.read(ReadRequest { addresses, expand: None, max_tokens: Some(20_000) })).unwrap()
}

/// Every single-line address and the whole-document range of every document read back exactly.
fn assert_reads_exactly(vault: &Vault, expected: &BTreeMap<String, String>) {
    for (doc_id, text) in expected {
        let lines = canon::split_lines(text);
        let addresses: Vec<String> = (1..=lines.len()).map(|n| format!("{doc_id}#L{n}")).collect();
        let r = read(vault, addresses);
        assert!(r.errors.is_empty() && r.truncated.is_none(), "{doc_id}: {:?} {:?}", r.errors, r.truncated);
        assert_eq!(r.passages.len(), lines.len());
        for (p, line) in r.passages.iter().zip(&lines) {
            assert_eq!(&p.text, line, "{}", p.address);
            let again: Address = p.address.to_string().parse().unwrap();
            assert_eq!(again, p.address, "every returned address round-trips");
        }
        let whole = read(vault, vec![format!("{doc_id}#L1-L{}", lines.len())]);
        assert_eq!(whole.passages[0].text, lines.join("\n"));
    }
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() { walk_files(&p, out) } else { out.push(p) }
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), &target).unwrap();
        }
    }
}

#[test]
fn publish_read_verify_and_move_the_vault() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path().join("vault-a");
    make_vault(&root);
    let config = config_for(&root, work.path(), "machine-a.toml");

    let writer = Writer::open(config.clone()).unwrap();
    let mut expected: BTreeMap<String, String> = BTreeMap::new();
    let mut outcomes: Vec<PublishOutcome> = Vec::new();
    for (i, (title, raw)) in DOCS.iter().enumerate() {
        let req = prose(&writer, work.path(), i, title, raw, "batch-1");
        let text = req.text.clone();
        let out = writer.publish(req).unwrap();
        assert_eq!(out.status, PublishStatus::Published);
        expected.insert(out.doc_id.to_string(), text);
        outcomes.push(out);
    }
    let code_req = code(&writer);
    let code_text = code_req.text.clone();
    assert!(
        !code_text.ends_with('\n') && code_text.contains("\treturn 1  \n"),
        "code keeps tabs, trailing spaces, no final LF"
    );
    let code_out = writer.publish(code_req).unwrap();
    assert!(code_out.rel_path.as_str().ends_with(".py"));
    expected.insert(code_out.doc_id.to_string(), code_text);
    writer.sync().unwrap();

    // Idempotence: the same inputs again add no rows and no manifest lines.
    let counts = writer.table_counts().unwrap();
    let manifest = std::fs::read(root.join("hot").join("manifests").join("text.jsonl")).unwrap();
    for (i, (title, raw)) in DOCS.iter().enumerate() {
        let out = writer.publish(prose(&writer, work.path(), i, title, raw, "batch-1")).unwrap();
        assert_eq!((out.status, &out.doc_id), (PublishStatus::AlreadyPublished, &outcomes[i].doc_id));
    }
    assert_eq!(writer.publish(code(&writer)).unwrap().status, PublishStatus::AlreadyPublished);
    assert_eq!(writer.table_counts().unwrap(), counts, "re-publishing adds no rows");
    assert_eq!(std::fs::read(root.join("hot").join("manifests").join("text.jsonl")).unwrap(), manifest);

    // A changed extraction is a new document; the old address keeps its text and points forward.
    let (title, raw) = DOCS[0];
    let mut changed = prose(&writer, work.path(), 0, title, &raw.replace("four", "four, give or take"), "batch-2");
    changed.supersedes = Some(Supersede { doc: outcomes[0].doc_id.to_string(), reason: "re-extraction".into() });
    let changed_text = changed.text.clone();
    let newer = writer.publish(changed).unwrap();
    assert_ne!(newer.doc_id, outcomes[0].doc_id);
    expected.insert(newer.doc_id.to_string(), changed_text);
    writer.sync().unwrap();

    let report = writer.verify().unwrap();
    assert!(report.is_clean(), "{report:?}");
    // Texts, plus one original per distinct source (the re-run's sources were duplicates; the change adds one).
    assert_eq!(report.checked as usize, expected.len() + DOCS.len() + 1, "texts plus originals");
    drop(writer);

    let vault = Vault::open(config.clone()).unwrap();
    assert_reads_exactly(&vault, &expected);
    let old = read(&vault, vec![format!("{}#L1-L3", outcomes[0].doc_id)]);
    assert_eq!(old.passages[0].text, canon::split_lines(&expected[&outcomes[0].doc_id.to_string()])[..3].join("\n"));
    assert_eq!(old.passages[0].superseded_by.as_ref(), Some(&newer.doc_id));

    // One flipped byte in a stored text: verify fails and read refuses to serve it.
    let stored = config.layout.hot.resolve_existing(&outcomes[1].rel_path).unwrap();
    let original = std::fs::read(&stored).unwrap();
    let mut flipped = original.clone();
    flipped[0] ^= 0x01;
    std::fs::write(&stored, &flipped).unwrap();
    let report = store::verify(&config.layout).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.mismatched.len(), 1);
    assert!(report.mismatched[0].path.ends_with(outcomes[1].rel_path.as_str()));
    let r = read(&vault, vec![format!("{}#L1", outcomes[1].doc_id)]);
    assert!(r.passages.is_empty() && r.errors[0].message.contains("integrity"));
    std::fs::write(&stored, &original).unwrap();
    assert!(store::verify(&config.layout).unwrap().is_clean());
    drop(vault);

    // Portability: nothing inside the vault names its location; a copy elsewhere works by config alone.
    let marker = work.path().file_name().unwrap().to_string_lossy().to_string();
    let mut files = Vec::new();
    walk_files(&root, &mut files);
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(&marker), "{} records the vault's location", f.display());
    }
    let elsewhere = tempfile::tempdir().unwrap();
    let root_b = elsewhere.path().join("moved").join("vault-b");
    copy_tree(&root, &root_b);
    let config_b = config_for(&root_b, elsewhere.path(), "machine-b.toml");
    assert!(store::verify(&config_b.layout).unwrap().is_clean());
    let vault_b = Vault::open(config_b).unwrap();
    assert_reads_exactly(&vault_b, &expected);
    let info = block_on(vault_b.vault_info()).unwrap();
    assert_eq!((info.vault_id.as_str(), info.documents), ("integration-vault", expected.len() as u64 - 1));
}

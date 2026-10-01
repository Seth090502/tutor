//! A temporary vault with published documents, built through vault-core's public API the way intake will.

use std::path::PathBuf;
use vault_api::{DocId, DocKind, PageMark, Symbol};
use vault_core::VaultConfig;
use vault_core::canon;
use vault_core::sidecar::{CodeInfo, Extraction, Provenance};
use vault_core::writer::{PublishRequest, PublishStatus, Repo, Writer};

pub const VAULT_ID: &str = "wire-test-vault";

pub struct RealVault {
    _dir: tempfile::TempDir,
    /// A machine config with every section of the SPEC example. `[server]` binds ephemeral ports.
    pub config_path: PathBuf,
    pub token_file: PathBuf,
    pub config: VaultConfig,
    /// Every published document with the canonical text vault-core stored for it.
    pub docs: Vec<(DocId, String)>,
}

const PROSE: [(&str, &str); 3] = [
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

/// A vault with its three tiers and `vault.toml`, but no database: intake has not run yet.
pub fn empty() -> RealVault {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("vault");
    for tier in ["hot", "cold", "inbox"] {
        std::fs::create_dir_all(root.join(tier)).unwrap();
    }
    std::fs::write(
        root.join("vault.toml"),
        format!("format_version = 1\nvault_id = \"{VAULT_ID}\"\n[tiers]\nhot = \"hot\"\ncold = \"cold\"\ninbox = \"inbox\"\n"),
    )
    .unwrap();
    let token_file = dir.path().join("token");
    let config_path = dir.path().join("libraryd.toml");
    // vault-core rejects two identical bind strings even at port 0, so gRPC gets another loopback address.
    std::fs::write(
        &config_path,
        format!(
            "stage = \"local\"\nmachine = \"wire-test\"\n\
             [vault]\nroot = '{}'\nhot = \"\"\ncold = \"\"\ninbox = \"\"\n\
             [server]\nmcp_bind = \"127.0.0.1:0\"\ngrpc_bind = \"127.0.0.2:0\"\ntoken_file = '{}'\n\
             allowed_hosts = [\"127.0.0.1\", \"localhost\"]\n\
             [intake]\noriginals = \"copy\"\nstability_seconds = 30\nloose_quiet_minutes = 15\nworkers = 6\n\
             [sidecars]\ngrobid_url = \"\"\nmineru = \"\"\npandoc = \"\"\nopenalex_key_env = \"OPENALEX_API_KEY\"\n",
            root.display(),
            token_file.display()
        ),
    )
    .unwrap();
    let config = VaultConfig::from_file(&config_path).unwrap();
    RealVault { _dir: dir, config_path, token_file, config, docs: Vec::new() }
}

fn prose(title: &str, raw: &str, index: usize) -> PublishRequest {
    let c = canon::normalize_prose(raw);
    let mut req = PublishRequest::new(c.text, DocKind::Paper);
    req.metadata.title = Some(title.into());
    req.metadata.provenance = vec![Provenance {
        batch: "wire".into(),
        received: "2026-10-02".into(),
        path: format!("doc{index}.md"),
        manifest: serde_json::json!({}),
    }];
    req.headings = c.headings;
    req.blocks = c.blocks;
    req.pages = std::iter::once(1)
        .chain(c.page_breaks.iter().copied())
        .enumerate()
        .map(|(k, line)| PageMark { label: (k + 1).to_string(), index: (k + 1) as u32, line })
        .collect();
    req.extraction =
        Extraction { path: "markdown".into(), tool: "wire".into(), extract_fp: "fp1".into(), encoding: "utf-8".into() };
    req
}

fn code(writer: &Writer) -> PublishRequest {
    writer.upsert_repo(&Repo { repo_id: "demo@abc".into(), name: "demo".into(), ..Repo::default() }).unwrap();
    let (_, text) = canon::canonical_code(b"\xEF\xBB\xBFdef main():\r\n\treturn 1  \r\n\r\n# no final newline");
    let mut req = PublishRequest::new(text, DocKind::Code);
    req.metadata.title = Some("demo main.py".into());
    req.code = Some(CodeInfo {
        repo: Some("demo@abc".into()),
        path: "src/main.py".into(),
        language: Some("python".into()),
        swhid: None,
        symbols: vec![Symbol { kind: "function".into(), name: "main".into(), start: 1, end: 2, parent: None }],
    });
    req.extraction =
        Extraction { path: "code".into(), tool: "wire".into(), extract_fp: "fp1".into(), encoding: "utf-8".into() };
    req
}

/// A vault holding three prose documents (one with pages, headings, a table and references; one Japanese; one
/// plain) and one Python file, published and closed.
pub fn build() -> RealVault {
    let mut vault = empty();
    let writer = Writer::open(vault.config.clone()).unwrap();
    let mut requests: Vec<PublishRequest> =
        PROSE.iter().enumerate().map(|(i, (title, raw))| prose(title, raw, i)).collect();
    requests.push(code(&writer));
    for req in requests {
        let text = req.text.clone();
        let out = writer.publish(req).unwrap();
        assert_eq!(out.status, PublishStatus::Published);
        vault.docs.push((out.doc_id, text));
    }
    writer.sync().unwrap();
    vault
}

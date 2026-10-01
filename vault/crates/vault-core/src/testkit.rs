//! Helpers for unit tests: a temporary vault, canonical publish requests, stored sources.

use crate::canon;
use crate::config::VaultConfig;
use crate::config::tests::{config_text, make_vault};
use crate::ids::blake3_hex;
use crate::sidecar::{Extraction, Provenance, SourceRef};
use crate::writer::{PublishRequest, StoreOriginal, Writer};
use std::path::{Path, PathBuf};
use vault_api::{DocKind, PageMark};

pub(crate) struct TestVault {
    pub dir: tempfile::TempDir,
    pub root: PathBuf,
}

impl TestVault {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = make_vault(dir.path());
        TestVault { dir, root }
    }

    pub fn config(&self) -> VaultConfig {
        VaultConfig::from_toml(&config_text(&self.root)).unwrap()
    }

    pub fn config_with(&self, extra: &str) -> VaultConfig {
        VaultConfig::from_toml(&format!("{}{extra}", config_text(&self.root))).unwrap()
    }

    pub fn writer(&self) -> Writer {
        Writer::open(self.config()).unwrap()
    }

    /// Writes `bytes` as a delivered file and stores it as an original.
    pub fn source(&self, writer: &Writer, name: &str, bytes: &[u8]) -> SourceRef {
        let path = self.dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let b3 = blake3_hex(bytes);
        let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("bin").to_string();
        let stored = writer
            .store_original(&StoreOriginal {
                source: &path,
                blake3: b3.clone(),
                ext: ext.clone(),
                format: None,
                batch: Some("b1".into()),
            })
            .unwrap();
        SourceRef { original: b3, ext, bytes: stored.bytes }
    }
}

/// A prose publish request built the way intake will: normalise, then take the structure canon found.
pub(crate) fn prose_request(raw: &str, title: &str) -> PublishRequest {
    let c = canon::normalize_prose(raw);
    let mut req = PublishRequest::new(c.text, DocKind::Paper);
    req.metadata.title = Some(title.to_string());
    req.headings = c.headings;
    req.blocks = c.blocks;
    let mut pages = vec![PageMark { label: "1".into(), index: 1, line: 1 }];
    for (i, line) in c.page_breaks.iter().enumerate() {
        let n = u32::try_from(i + 2).unwrap();
        pages.push(PageMark { label: n.to_string(), index: n, line: *line });
    }
    req.pages = pages;
    req.extraction =
        Extraction { path: "markdown".into(), tool: "test".into(), extract_fp: "fp1".into(), encoding: "utf-8".into() };
    req.metadata.provenance = vec![Provenance {
        batch: "b1".into(),
        received: "2026-10-02".into(),
        path: "papers/x.md".into(),
        manifest: serde_json::json!({}),
    }];
    req
}

pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

pub(crate) const PAPER: &str = "# Working memory capacity\n\nThe magical number is four. Cowan et al. argued this in 2001.\n\n\
## Evidence\n\nChunking experiments show limits. Span tasks agree.\u{C}Page two begins here. It continues.\n\n\
### Details\n\nA detail sentence. Another detail.\n\n## References\nCowan, N. (2001). The magical number 4. BBS.\n";

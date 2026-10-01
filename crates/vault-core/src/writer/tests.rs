use super::*;
use crate::sidecar::{Categories, Provenance};
use crate::testkit::{PAPER, TestVault, prose_request};
use std::sync::Arc;

fn counts(w: &Writer) -> Vec<(String, i64)> {
    w.table_counts().unwrap()
}

fn manifest_lines(v: &TestVault) -> usize {
    std::fs::read_to_string(v.root.join("hot").join("manifests").join("text.jsonl")).unwrap_or_default().lines().count()
}

#[test]
fn publish_writes_text_sidecar_rows_and_manifest() {
    let v = TestVault::new();
    let w = v.writer();
    let src = v.source(&w, "cowan.md", PAPER.as_bytes());
    let mut req = prose_request(PAPER, "The Magical Number Four");
    req.sources = vec![src.clone()];
    let text = req.text.clone();
    let out = w.publish(req).unwrap();
    assert_eq!(out.status, PublishStatus::Published);
    assert_eq!(out.doc_id.hash(), &ids::hash16(text.as_bytes()));
    assert_eq!(out.doc_id.slug(), Some("the-magical-number-four"));
    assert_eq!(out.rel_path.as_str(), format!("text/{}/{}.md", out.doc_id.shard(), out.doc_id));
    let stored = store::read(&w.layout().hot, &out.rel_path).unwrap();
    assert_eq!(stored, text.as_bytes());
    let sc = Sidecar::from_json(&store::read(&w.layout().hot, &ids::sidecar_rel_path(&out.doc_id).unwrap()).unwrap())
        .unwrap();
    assert_eq!(
        (sc.lines, sc.bytes, sc.doc_id.as_str()),
        (out.lines, text.len() as u64, out.doc_id.to_string().as_str())
    );
    assert_eq!(sc.sources, vec![src]);
    assert!(sc.headings.iter().any(|h| h.text == "Evidence"));
    assert_eq!(sc.pages.len(), 2);
    assert_eq!(manifest_lines(&v), 1);
    let c = counts(&w);
    let get = |t: &str| c.iter().find(|(n, _)| n == t).unwrap().1;
    assert_eq!((get("documents"), get("doc_originals"), get("doc_fts")), (1, 1, 1));
    assert!(w.verify().unwrap().is_clean());
}

#[test]
fn publish_is_idempotent_and_only_adds_provenance() {
    let v = TestVault::new();
    let w = v.writer();
    let first = w.publish(prose_request(PAPER, "Title one")).unwrap();
    let before = counts(&w);
    let sidecar_path = w.layout().hot.resolve_existing(&ids::sidecar_rel_path(&first.doc_id).unwrap()).unwrap();
    let sidecar_written = std::fs::metadata(&sidecar_path).unwrap().modified().unwrap();
    let again = w.publish(prose_request(PAPER, "Title one")).unwrap();
    assert_eq!((again.status, &again.doc_id), (PublishStatus::AlreadyPublished, &first.doc_id));
    assert_eq!(counts(&w), before, "no new rows");
    assert_eq!(manifest_lines(&v), 1);
    assert_eq!(
        std::fs::metadata(&sidecar_path).unwrap().modified().unwrap(),
        sidecar_written,
        "an unchanged sidecar is not rewritten"
    );

    // Same text from another delivery, with a different title: the slug stays frozen; provenance grows.
    let mut other = prose_request(PAPER, "A different title");
    other.metadata.provenance = vec![Provenance {
        batch: "b2".into(),
        received: "2026-10-03".into(),
        path: "x/y.md".into(),
        manifest: serde_json::json!({"k": 1}),
    }];
    let third = w.publish(other).unwrap();
    assert_eq!(third.doc_id, first.doc_id);
    assert_eq!(counts(&w), before);
    let sc = Sidecar::from_json(&store::read(&w.layout().hot, &ids::sidecar_rel_path(&first.doc_id).unwrap()).unwrap())
        .unwrap();
    assert_eq!(sc.provenance.len(), 2);
    assert_eq!(sc.title.as_deref(), Some("Title one"));
}

#[test]
fn concurrent_publishes_of_the_same_text_agree() {
    let v = TestVault::new();
    let w = Arc::new(v.writer());
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let w = Arc::clone(&w);
            std::thread::spawn(move || w.publish(prose_request(PAPER, "Same")).unwrap())
        })
        .collect();
    let outcomes: Vec<PublishOutcome> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|o| o.status == PublishStatus::Published).count(), 1);
    assert!(outcomes.iter().all(|o| o.doc_id == outcomes[0].doc_id));
    assert_eq!(manifest_lines(&v), 1);
}

#[test]
fn refuses_non_canonical_text_and_bad_structure() {
    let v = TestVault::new();
    let w = v.writer();
    let mut req = prose_request(PAPER, "t");
    req.text = "Two sentences. On one line.\n".into();
    assert!(matches!(w.publish(req), Err(CoreError::NotCanonical(_))));
    let mut req = prose_request(PAPER, "t");
    req.text.clear();
    assert!(matches!(w.publish(req), Err(CoreError::NotCanonical(_))));
    let mut req = prose_request(PAPER, "t");
    req.headings[0].end = 10_000;
    assert!(matches!(w.publish(req), Err(CoreError::InvalidStructure(_))));
    let mut req = prose_request(PAPER, "t");
    req.pages.reverse();
    assert!(matches!(w.publish(req), Err(CoreError::InvalidStructure(_))));
    let mut req = prose_request(PAPER, "t");
    req.kind = DocKind::Code;
    assert!(matches!(w.publish(req), Err(CoreError::InvalidStructure(_))));
    let mut code = PublishRequest::new("a\r\nb".into(), DocKind::Code);
    code.code = Some(CodeInfo { path: "a.py".into(), ..CodeInfo::default() });
    assert!(matches!(w.publish(code), Err(CoreError::NotCanonical(_))));
    let mut req = prose_request(PAPER, "t");
    req.sources = vec![SourceRef { original: ids::blake3_hex(b"never stored"), ext: "pdf".into(), bytes: 1 }];
    assert!(matches!(w.publish(req), Err(CoreError::InvalidArgument(_))));
    assert!(counts(&w).iter().all(|(t, n)| *n == 0 || t == "meta"), "nothing was written");
}

#[test]
fn a_hash16_collision_is_refused() {
    let v = TestVault::new();
    let w = v.writer();
    let out = w.publish(prose_request(PAPER, "x")).unwrap();
    lock(&w.conn)
        .execute("UPDATE documents SET text_blake3 = 'different' WHERE doc_id = ?1", [out.doc_id.to_string()])
        .unwrap();
    assert!(matches!(w.publish(prose_request(PAPER, "x")), Err(CoreError::IdCollision { .. })));

    // Same hash16 at the file level before any row exists (different bytes already at the target name).
    let v = TestVault::new();
    let w = v.writer();
    let req = prose_request("Other text. Here.", "y");
    let id = ids::doc_id(&ids::hash16(req.text.as_bytes()), "y").unwrap();
    store::write_once(&w.layout().hot, &ids::text_rel_path(&id, DocKind::Paper, None).unwrap(), b"not the same\n")
        .unwrap();
    assert!(matches!(w.publish(req), Err(CoreError::IdCollision { .. })));
}

#[test]
fn a_new_extraction_supersedes_the_old_one() {
    let v = TestVault::new();
    let w = v.writer();
    let old = w.publish(prose_request(PAPER, "Paper")).unwrap();
    let mut req = prose_request(&PAPER.replace("four", "four, plus or minus one"), "Paper");
    req.supersedes = Some(Supersede { doc: old.doc_id.hash().to_string(), reason: "better extraction".into() });
    let new = w.publish(req.clone()).unwrap();
    assert_ne!(new.doc_id, old.doc_id);
    let conn = lock(&w.conn);
    let row = db::doc_by_hash(&conn, old.doc_id.hash()).unwrap().unwrap();
    assert_eq!(row.superseded_by.as_deref(), Some(new.doc_id.to_string().as_str()));
    let (reason, n): (String, i64) =
        conn.query_row("SELECT reason, count(*) FROM lineage", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!((reason.as_str(), n), ("better extraction", 1));
    drop(conn);
    let sc = Sidecar::from_json(&store::read(&w.layout().hot, &ids::sidecar_rel_path(&new.doc_id).unwrap()).unwrap())
        .unwrap();
    assert_eq!(sc.supersedes.as_deref(), Some(old.doc_id.to_string().as_str()));
    let before = counts(&w);
    assert_eq!(w.publish(req).unwrap().status, PublishStatus::AlreadyPublished);
    assert_eq!(counts(&w), before);
    let mut bad = prose_request("Third. Text.", "p");
    bad.supersedes = Some(Supersede { doc: "0000000000000000".into(), reason: "r".into() });
    assert!(matches!(w.publish(bad), Err(CoreError::UnknownDocument(_))));
}

#[test]
fn an_interrupted_publish_is_completed_by_the_rerun() {
    let v = TestVault::new();
    // Crash after the files, before the commit: the rerun adopts the files.
    {
        let w = v.writer();
        let req = prose_request(PAPER, "Crash test");
        let id = ids::doc_id(&ids::hash16(req.text.as_bytes()), "crash-test").unwrap();
        store::write_once(
            &w.layout().hot,
            &ids::text_rel_path(&id, DocKind::Paper, None).unwrap(),
            req.text.as_bytes(),
        )
        .unwrap();
        let out = w.publish(req).unwrap();
        assert_eq!((out.status, out.doc_id), (PublishStatus::Published, id));
    }
    // Crash after the commit, before the manifest line: verify notices, the rerun repairs.
    std::fs::write(v.root.join("hot").join("manifests").join("text.jsonl"), b"").unwrap();
    let w = v.writer();
    assert_eq!(w.verify().unwrap().extra.len(), 1);
    assert_eq!(w.publish(prose_request(PAPER, "Crash test")).unwrap().status, PublishStatus::AlreadyPublished);
    assert!(w.verify().unwrap().is_clean());
}

#[test]
fn code_files_keep_their_extension_and_repository() {
    let v = TestVault::new();
    let w = v.writer();
    w.upsert_repo(&Repo { repo_id: "ripgrep@abc123".into(), name: "ripgrep".into(), ..Repo::default() }).unwrap();
    let text = canon::normalize_code("fn main() {\r\n\tprintln!(\"hi\");  \r\n}");
    let mut req = PublishRequest::new(text.clone(), DocKind::Code);
    req.code = Some(CodeInfo {
        repo: Some("ripgrep@abc123".into()),
        path: "crates/core/main.rs".into(),
        language: Some("rust".into()),
        swhid: None,
        symbols: vec![vault_api::Symbol {
            kind: "function".into(),
            name: "main".into(),
            start: 1,
            end: 3,
            parent: None,
        }],
    });
    let out = w.publish(req).unwrap();
    assert_eq!(out.lines, 3);
    assert_eq!(out.doc_id.slug(), Some("ripgrep-crates-core-main-rs"));
    assert!(out.rel_path.as_str().ends_with(".rs"));
    assert_eq!(store::read(&w.layout().hot, &out.rel_path).unwrap(), text.as_bytes());
    let conn = lock(&w.conn);
    assert_eq!(
        db::repo_of(&conn, &out.doc_id.to_string()).unwrap(),
        Some(("ripgrep".into(), "crates/core/main.rs".into()))
    );
    drop(conn);
    let mut unknown = PublishRequest::new("x\n".into(), DocKind::Code);
    unknown.code = Some(CodeInfo { repo: Some("nope".into()), path: "x.rs".into(), ..CodeInfo::default() });
    assert!(matches!(w.publish(unknown), Err(CoreError::InvalidArgument(_))));
}

#[test]
fn metadata_updates_rewrite_rows_and_sidecar_but_not_structure() {
    let v = TestVault::new();
    let w = v.writer();
    let out = w.publish(prose_request(PAPER, "Old title")).unwrap();
    let rel = ids::sidecar_rel_path(&out.doc_id).unwrap();
    let before = Sidecar::from_json(&store::read(&w.layout().hot, &rel).unwrap()).unwrap();
    let mut meta = before.metadata();
    meta.title = Some("New title".into());
    meta.authors = vec!["Nelson Cowan".into()];
    meta.categories = Some(Categories {
        primary: Some("psych-cogsci".into()),
        secondary: vec!["medicine".into()],
        openalex_topic: Some("T10001".into()),
        areas: vec!["memory".into()],
        source: Some("openalex".into()),
        confidence: Some(0.9),
    });
    w.update_metadata(out.doc_id.hash(), meta, Some("landmark".into()), None).unwrap();
    let after = Sidecar::from_json(&store::read(&w.layout().hot, &rel).unwrap()).unwrap();
    assert_eq!(after.title.as_deref(), Some("New title"));
    assert!(before.check_replacement(&after).is_ok());
    let conn = lock(&w.conn);
    let row = db::doc_by_hash(&conn, out.doc_id.hash()).unwrap().unwrap();
    assert_eq!((row.title.as_deref(), row.primary_category.as_deref()), (Some("New title"), Some("psych-cogsci")));
    assert_eq!(row.doc_id, out.doc_id.to_string(), "the address never changes");
    let cats: i64 = conn.query_row("SELECT count(*) FROM doc_categories", [], |r| r.get(0)).unwrap();
    assert_eq!(cats, 3);
    let hits: i64 =
        conn.query_row("SELECT count(*) FROM doc_fts WHERE doc_fts MATCH 'cowan'", [], |r| r.get(0)).unwrap();
    assert_eq!(hits, 1);
}

#[test]
fn originals_are_stored_once_by_content() {
    let v = TestVault::new();
    let w = v.writer();
    let a = v.source(&w, "paper.PDF", b"%PDF-1.7 bytes");
    assert_eq!(a.ext, "PDF");
    let path = v.dir.path().join("same-bytes.bin");
    std::fs::write(&path, b"%PDF-1.7 bytes").unwrap();
    let again = w
        .store_original(&StoreOriginal {
            source: &path,
            blake3: a.original.clone(),
            ext: "bin".into(),
            format: None,
            batch: None,
        })
        .unwrap();
    assert!(again.duplicate);
    assert!(again.rel_path.as_str().ends_with(".pdf"), "the first stored name is kept");
    let wrong = w.store_original(&StoreOriginal {
        source: &path,
        blake3: ids::blake3_hex(b"something else"),
        ext: "pdf".into(),
        format: None,
        batch: None,
    });
    assert!(matches!(wrong, Err(CoreError::HashMismatch { .. })));
    assert!(path.exists(), "copy mode never removes the delivery");
    let r = w.verify().unwrap();
    assert!(r.is_clean() && r.checked == 1, "{r:?}");
}

#[test]
fn move_mode_moves_the_original() {
    let v = TestVault::new();
    let w = Writer::open(v.config_with("[intake]\noriginals = \"move\"\n")).unwrap();
    let path = v.dir.path().join("book.epub");
    std::fs::write(&path, b"epub").unwrap();
    let b3 = ids::blake3_hex(b"epub");
    let s = w
        .store_original(&StoreOriginal { source: &path, blake3: b3, ext: "epub".into(), format: None, batch: None })
        .unwrap();
    assert!(!s.duplicate && !path.exists());
    assert!(w.verify().unwrap().is_clean());
}

#[test]
fn quarantine_copies_and_records_but_never_copies_secrets() {
    let v = TestVault::new();
    let w = v.writer();
    let path = v.dir.path().join("broken.pdf");
    std::fs::write(&path, b"broken").unwrap();
    let b3 = ids::blake3_hex(b"broken");
    let attempt = Attempt { tool: "pdfium".into(), error: "no text layer".into(), at: "t".into() };
    let rec = w
        .quarantine(&QuarantineRequest {
            source: Some(&path),
            blake3: b3.clone(),
            ext: "pdf".into(),
            reason: ReasonCode::ExtractionFailed,
            detail: "every extractor failed".into(),
            batch: Some("b1".into()),
            original_path: Some("papers/broken.pdf".into()),
            attempts: vec![attempt.clone()],
            delivery_id: None,
        })
        .unwrap();
    let cold = &w.layout().cold;
    assert_eq!(store::read(cold, rec.stored_rel_path.as_ref().unwrap()).unwrap(), b"broken");
    let reason: ReasonFile = serde_json::from_slice(&store::read(cold, &rec.reason_rel_path).unwrap()).unwrap();
    assert_eq!((reason.reason, reason.attempts.len(), reason.copied), (ReasonCode::ExtractionFailed, 1, true));
    let json: serde_json::Value = serde_json::from_slice(&store::read(cold, &rec.reason_rel_path).unwrap()).unwrap();
    assert_eq!(json["reason"], "extraction_failed");

    let secret = v.dir.path().join(".env");
    std::fs::write(&secret, b"API_KEY=abc").unwrap();
    let sb3 = ids::blake3_hex(b"API_KEY=abc");
    let rec = w
        .quarantine(&QuarantineRequest {
            source: Some(&secret),
            blake3: sb3.clone(),
            ext: "env".into(),
            reason: ReasonCode::SuspectedSecret,
            detail: "rule dotenv".into(),
            batch: None,
            original_path: Some("repo/.env".into()),
            attempts: vec![],
            delivery_id: None,
        })
        .unwrap();
    assert!(rec.stored_rel_path.is_none());
    let (file_rel, _) = ids::quarantine_rel_paths(&sb3, "env").unwrap();
    assert!(cold.resolve_if_exists(&file_rel).unwrap().is_none(), "a suspected secret is never copied");
    assert!(!String::from_utf8(store::read(cold, &rec.reason_rel_path).unwrap()).unwrap().contains("abc"));
    assert!(secret.exists());
}

#[test]
fn batches_and_deliveries_reach_one_outcome_each() {
    let v = TestVault::new();
    let w = v.writer();
    let batch = Batch {
        batch_id: "2026-10-02-psych".into(),
        kind: BatchKind::Batch,
        inbox_name: "2026-10-02-psych".into(),
        detected_at: "t0".into(),
        ready_at: None,
        completed_at: None,
        state: "detected".into(),
        files_expected: Some(2),
        report_path: None,
    };
    w.upsert_batch(&batch).unwrap();
    w.upsert_batch(&Batch { state: "done".into(), completed_at: Some("t1".into()), ..batch.clone() }).unwrap();
    let d = NewDelivery {
        batch_id: batch.batch_id.clone(),
        rel_path: "a.pdf".into(),
        received_at: "t".into(),
        ..NewDelivery::default()
    };
    let id1 = w.record_delivery(&d).unwrap();
    let id1b = w.record_delivery(&NewDelivery { blake3: Some("ab".repeat(32)), ..d.clone() }).unwrap();
    assert_eq!(id1, id1b);
    let id2 = w.record_delivery(&NewDelivery { rel_path: "b.pdf".into(), ..d }).unwrap();
    w.set_delivery_outcome(
        id1,
        &DeliveryOutcome {
            outcome: Outcome::Published,
            reason: None,
            rule: None,
            detail: None,
            doc_id: Some("x".into()),
        },
    )
    .unwrap();
    w.set_delivery_outcome(
        id2,
        &DeliveryOutcome {
            outcome: Outcome::Quarantined,
            reason: Some("corrupt_file".into()),
            rule: None,
            detail: None,
            doc_id: None,
        },
    )
    .unwrap();
    assert_eq!(
        w.delivery_outcomes(&batch.batch_id).unwrap(),
        vec![("a.pdf".to_string(), Outcome::Published), ("b.pdf".to_string(), Outcome::Quarantined)]
    );
    assert!(
        w.set_delivery_outcome(
            999,
            &DeliveryOutcome { outcome: Outcome::Excluded, reason: None, rule: None, detail: None, doc_id: None }
        )
        .is_err()
    );
    let state: String = lock(&w.conn)
        .query_row("SELECT state FROM batches WHERE batch_id = ?1", [&batch.batch_id], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "done");
}

#[test]
fn only_one_writer_and_only_its_own_vault() {
    let v = TestVault::new();
    let w = v.writer();
    assert!(matches!(Writer::open(v.config()), Err(CoreError::WriterLocked(_))));
    w.enqueue_job("extract", "k", "fp").unwrap();
    let claimed = w.claim_jobs(None, 5).unwrap();
    assert_eq!(claimed.len(), 1);
    drop(w);
    // A restart returns the interrupted job to the queue.
    let w = v.writer();
    assert_eq!(w.job_counts().unwrap(), vec![("extract".to_string(), JobState::Queued, 1)]);
    drop(w);
    std::fs::write(v.root.join("vault.toml"), "format_version = 1\nvault_id = \"another\"\n").unwrap();
    assert!(matches!(Writer::open(v.config()), Err(CoreError::Config(m)) if m.contains("belongs to vault")));
}

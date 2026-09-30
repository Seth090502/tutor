//! Incremental ingest of the corpus and notes roots.
//!
//! 1. Walk the roots (or one folder inside a root). Files whose size and modification time
//!    match the `files` table keep their recorded BLAKE3 hash; the rest are hashed.
//! 2. `doc_id` is the content hash, so identical copies are one document; its canonical path
//!    (which decides domain and source type) is the first of its paths in sorted order.
//! 3. Documents no file points to any more are removed from the BM25 index, the vector store
//!    and the database. Nothing is removed when the walk reported errors, so an unreadable or
//!    unplugged folder never empties the index.
//! 4. Documents whose canonical path changed keep their chunks; their domain and source type
//!    are updated in both indexes.
//! 5. New documents are extracted, chunked and indexed one at a time, committed in batches.
//!    A document is `indexing` until its batch is committed to BM25, then `indexed`; a crash in
//!    between is repaired at the start of the next ingest.
//! 6. With an OCR provider: queued pages are recognised and their documents re-indexed.
//! 7. With an embedding provider: chunks not yet in the vector store are embedded in batches.
//!
//! Memory stays bounded by one document's text, one commit batch in the BM25 writer, and one
//! buffer of vectors; the file list (one entry per file) is the only corpus-sized structure.

use crate::bm25::Bm25Writer;
use crate::chunk::{self, DocText};
use crate::dedup;
use crate::discover::{self, DocKind};
use crate::embed::{self, Embedder, InputType};
use crate::engine::{Engine, Roots};
use crate::error::{Error, Result};
use crate::extract::{self, PdfText};
use crate::headings::{self, Section};
use crate::jobs::Progress;
use crate::meta::{self, DocMeta, Root};
use crate::ocr;
use crate::store::{self, SpendKind};
use crate::survey;
use crate::triage::{PageClass, TriageCounts};
use crate::vector::{VecRow, Vectors};
use pdfium_render::prelude::Pdfium;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};
use tokio::runtime::Handle;

/// Commit the BM25 index after this many documents or chunks, whichever comes first.
const COMMIT_DOCS: usize = 64;
const COMMIT_CHUNKS: usize = 20_000;
/// Vectors buffered before one LanceDB append (each append is a new data fragment).
const VECTOR_FLUSH_ROWS: usize = 1024;
const LOG_EVERY: u64 = 25;

pub const EMBED_PROVIDER_KEY: &str = "embedding.provider";
pub const EMBED_MODEL_KEY: &str = "embedding.model";
pub const EMBED_DIMS_KEY: &str = "embedding.dims";

#[derive(Debug, Default, Serialize)]
pub struct IngestReport {
    /// Document files found by the walk.
    pub files: u64,
    /// Distinct documents (by content) after the ingest.
    pub documents: u64,
    pub indexed: u64,
    pub unchanged: u64,
    pub moved: u64,
    pub removed: u64,
    /// Documents that could not be read (encrypted, damaged, ...).
    pub errors: u64,
    pub chunks_added: u64,
    pub ocr_pages: u64,
    pub embedded_chunks: u64,
    /// Walk and hashing errors; while there are any, nothing is removed.
    pub walk_errors: Vec<String>,
    pub seconds: f64,
}

/// A folder to walk: a whole root, or a folder inside one.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub root: Root,
    pub dir: PathBuf,
    /// Path of `dir` inside the root (`/`-separated, empty for the root itself).
    pub rel_prefix: String,
}

impl Target {
    /// Stored paths under this target are `>= lo` and `< hi`.
    fn path_range(&self) -> (String, String) {
        let lo = join_path(&[self.root.prefix(), &self.rel_prefix]) + "/";
        let hi = format!("{}0", &lo[..lo.len() - 1]); // '0' follows '/'
        (lo, hi)
    }
}

fn join_path(parts: &[&str]) -> String {
    parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join("/")
}

/// Folders to walk: both roots, or only `scope`, which must be inside one of them.
pub fn targets(roots: &Roots, scope: Option<&Path>) -> Result<Vec<Target>> {
    let mut all = vec![(Root::Corpus, roots.corpus.clone())];
    if let Some(notes) = &roots.notes {
        all.push((Root::Notes, notes.clone()));
    }
    let Some(scope) = scope else {
        return Ok(all.into_iter().map(|(root, dir)| Target { root, dir, rel_prefix: String::new() }).collect());
    };
    if !scope.is_dir() {
        return Err(format!("root {} is not a directory", scope.display()).into());
    }
    let scope_c = scope.canonicalize()?;
    for (root, dir) in all {
        if let Ok(rel) = scope_c.strip_prefix(dir.canonicalize()?) {
            let rel_prefix = rel.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>().join("/");
            return Ok(vec![Target { root, dir: scope.to_path_buf(), rel_prefix }]);
        }
    }
    Err(format!("root {} is not inside the corpus or notes folder", scope.display()).into())
}

struct Found {
    path: String,
    abs: PathBuf,
    bytes: u64,
    mtime_ns: i64,
}

fn mtime_ns(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i64)
}

/// A live document: its id, canonical stored path and size.
struct LiveDoc {
    doc_id: String,
    path: String,
    bytes: u64,
}

struct ExistingDoc {
    path: String,
    domain: String,
    source_type: String,
    state: String,
}

pub fn ingest(
    engine: &Engine,
    roots: &Roots,
    pdfium_lib: &Path,
    scope: Option<&Path>,
    rebuild: bool,
    rt: &Handle,
    progress: &Progress,
) -> Result<IngestReport> {
    let started = Instant::now();
    let targets = targets(roots, scope)?;
    for t in &targets {
        if !t.dir.is_dir() {
            return Err(format!("{} is not a directory", t.dir.display()).into());
        }
    }
    let mut conn = store::open(&engine.data_dir)?;
    let mut writer = engine.bm25.writer()?;
    let mut report = IngestReport::default();
    let mut vectors_changed = false;

    if rebuild {
        eprintln!("ingest: rebuild requested; clearing indexes (finished OCR text is kept)");
        writer.delete_all()?;
        writer.commit()?;
        engine.bm25.reload()?;
        if let Some(v) = &engine.vectors {
            rt.block_on(v.drop_all())?;
        }
        conn.execute_batch(
            "DELETE FROM chunks; DELETE FROM documents; DELETE FROM files; DELETE FROM ocr_queue WHERE state <> 'done';",
        )?;
        for key in [EMBED_PROVIDER_KEY, EMBED_MODEL_KEY, EMBED_DIMS_KEY] {
            store::meta_delete(&conn, key)?;
        }
    }
    recover_interrupted(&mut conn, &mut writer, engine, rt)?;

    // 1. Walk.
    let mut found = Vec::new();
    for t in &targets {
        let walked = discover::discover(&t.dir);
        report.walk_errors.extend(walked.walk_errors);
        for doc in walked.docs {
            found.push(Found {
                path: join_path(&[t.root.prefix(), &t.rel_prefix, &doc.rel_path]),
                mtime_ns: mtime_ns(&doc.path),
                abs: doc.path,
                bytes: doc.bytes,
            });
        }
    }
    let walk_complete = report.walk_errors.is_empty();
    report.files = found.len() as u64;
    eprintln!("ingest: {} files found under {} folder(s)", found.len(), targets.len());

    // 2. Hash what changed and record the file table.
    let known = known_files(&conn, &targets)?;
    let changed: Vec<&Found> = found
        .iter()
        .filter(|f| known.get(&f.path).is_none_or(|(bytes, mtime, _)| *bytes != f.bytes || *mtime != f.mtime_ns))
        .collect();
    let hashes = dedup::hash_all(&changed.iter().map(|f| f.abs.as_path()).collect::<Vec<_>>(), |_| {});
    let tx = conn.transaction()?;
    {
        let mut upsert = tx.prepare_cached(
            "INSERT INTO files(path, bytes, mtime_ns, doc_id) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(path) DO UPDATE SET bytes = excluded.bytes, mtime_ns = excluded.mtime_ns, doc_id = excluded.doc_id",
        )?;
        for (f, hash) in changed.iter().zip(&hashes) {
            match hash {
                Ok(hash) => {
                    upsert.execute(params![f.path, f.bytes as i64, f.mtime_ns, hash])?;
                }
                Err(e) => report.walk_errors.push(format!("{}: {e}", f.path)),
            }
        }
        if walk_complete {
            let present: HashSet<&str> = found.iter().map(|f| f.path.as_str()).collect();
            let mut delete = tx.prepare_cached("DELETE FROM files WHERE path = ?1")?;
            for path in known.keys().filter(|p| !present.contains(p.as_str())) {
                delete.execute([path])?;
            }
        } else {
            eprintln!("ingest: {} walk error(s); nothing is removed this time", report.walk_errors.len());
        }
    }
    tx.commit()?;

    // 3. Live documents, and removal of the ones nothing points to.
    let live = live_docs(&conn)?;
    let existing = existing_docs(&conn)?;
    let live_ids: HashSet<&str> = live.iter().map(|d| d.doc_id.as_str()).collect();
    let removed: Vec<String> = existing.keys().filter(|id| !live_ids.contains(id.as_str())).cloned().collect();
    if !removed.is_empty() {
        for id in &removed {
            writer.delete_doc(id);
        }
        writer.commit()?;
        engine.bm25.reload()?;
        if let Some(v) = &engine.vectors {
            rt.block_on(v.delete_docs(&removed))?;
            vectors_changed = true;
        }
        let tx = conn.transaction()?;
        for id in &removed {
            for table in ["chunks", "ocr_queue", "documents"] {
                tx.execute(&format!("DELETE FROM {table} WHERE doc_id = ?1"), [id])?;
            }
        }
        tx.commit()?;
        report.removed = removed.len() as u64;
    }
    progress.add_total(live.len() as u64);

    // 4. Documents whose canonical path moved.
    let mut moved = Vec::new();
    for doc in &live {
        if let Some(old) = existing.get(&doc.doc_id)
            && old.path != doc.path
        {
            moved.push((doc, doc_meta(&doc.path)?, old));
        }
    }
    let mut refiled = Vec::new();
    for (doc, m, old) in &moved {
        if old.state == "indexed" && (old.domain.as_str(), old.source_type.as_str()) != (m.domain.as_str(), m.source_type) {
            writer.delete_doc(&doc.doc_id);
            for (chunk_id, text) in chunk_texts(&conn, &doc.doc_id)? {
                writer.add(&chunk_id, &doc.doc_id, &m.domain, m.source_type, &text)?;
            }
            refiled.push((doc.doc_id.clone(), m.domain.clone(), m.source_type));
        }
    }
    if !moved.is_empty() {
        writer.commit()?;
        engine.bm25.reload()?;
        if let Some(v) = &engine.vectors {
            for (doc_id, domain, source_type) in &refiled {
                rt.block_on(v.set_doc_meta(doc_id, domain, source_type))?;
                vectors_changed = true;
            }
        }
        let tx = conn.transaction()?;
        for (doc, m, _) in &moved {
            tx.execute(
                "UPDATE documents SET path = ?2, domain = ?3, source_type = ?4 WHERE doc_id = ?1",
                params![doc.doc_id, doc.path, m.domain, m.source_type],
            )?;
        }
        tx.commit()?;
        report.moved = moved.len() as u64;
    }

    // 5. New documents.
    let mut pdfium: Option<Pdfium> = None;
    let (mut batch_docs, mut batch_chunks) = (0, 0);
    let mut seen = 0u64;
    for doc in &live {
        match existing.get(&doc.doc_id) {
            Some(old) if old.path == doc.path => report.unchanged += 1,
            Some(_) => {}
            None => {
                let m = doc_meta(&doc.path)?;
                let abs = roots.resolve(&doc.path).ok_or(format!("no root for {}", doc.path))?;
                match index_doc(&mut conn, &writer, &mut pdfium, pdfium_lib, doc, &abs, &m, false)? {
                    Some(n) => {
                        report.indexed += 1;
                        report.chunks_added += n;
                        batch_chunks += n as usize;
                    }
                    None => report.errors += 1,
                }
                batch_docs += 1;
                if batch_docs >= COMMIT_DOCS || batch_chunks >= COMMIT_CHUNKS {
                    commit(&conn, &mut writer, engine)?;
                    (batch_docs, batch_chunks) = (0, 0);
                }
            }
        }
        progress.add_done(1);
        seen += 1;
        if seen.is_multiple_of(LOG_EVERY) || seen == live.len() as u64 {
            eprintln!("ingest: documents {seen}/{} (last: {})", live.len(), doc.path);
        }
    }
    commit(&conn, &mut writer, engine)?;
    report.documents = live.len() as u64;

    // 6. OCR. A failure is reported after the rest of the ingest has run.
    let mut failure: Option<Error> = None;
    if let Some(provider) = &engine.ocr {
        let (pending, _) = ocr::counts(&conn)?;
        progress.add_total(pending);
        let pdf_path = |doc_id: &str| -> Option<PathBuf> {
            let path: String =
                conn.query_row("SELECT path FROM documents WHERE doc_id = ?1", [doc_id], |r| r.get(0)).ok()?;
            roots.resolve(&path)
        };
        match ocr::process(&conn, provider.as_ref(), pdf_path, || progress.add_done(1)) {
            Ok(n) => report.ocr_pages = n,
            Err(e) => failure = Some(e),
        }
        let reindex = ocr::docs_to_reindex(&conn)?;
        for doc_id in &reindex {
            let (path, bytes): (String, i64) = conn.query_row(
                "SELECT path, bytes FROM documents WHERE doc_id = ?1",
                [doc_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let doc = LiveDoc { doc_id: doc_id.clone(), path, bytes: bytes as u64 };
            let abs = roots.resolve(&doc.path).ok_or(format!("no root for {}", doc.path))?;
            if let Some(v) = &engine.vectors {
                rt.block_on(v.delete_docs(std::slice::from_ref(doc_id)))?;
                vectors_changed = true;
            }
            index_doc(&mut conn, &writer, &mut pdfium, pdfium_lib, &doc, &abs, &doc_meta(&doc.path)?, true)?;
        }
        if !reindex.is_empty() {
            commit(&conn, &mut writer, engine)?;
            eprintln!("ingest: re-indexed {} document(s) with OCR text", reindex.len());
        }
    }
    writer.finish()?;

    // 7. Embeddings.
    if let (Some(embedder), Some(vectors)) = (&engine.embedder, &engine.vectors) {
        match embed_pending(&conn, embedder, vectors, rt, progress) {
            Ok(n) => {
                report.embedded_chunks = n;
                vectors_changed |= n > 0;
            }
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    if vectors_changed && let Some(v) = &engine.vectors {
        rt.block_on(v.maintain())?;
    }

    report.seconds = started.elapsed().as_secs_f64();
    eprintln!(
        "ingest: {} documents ({} new, {} unchanged, {} moved, {} removed, {} errors), {} chunks added, {} embedded in {:.1}s",
        report.documents,
        report.indexed,
        report.unchanged,
        report.moved,
        report.removed,
        report.errors,
        report.chunks_added,
        report.embedded_chunks,
        report.seconds
    );
    match failure {
        Some(e) => Err(e),
        None => Ok(report),
    }
}

fn doc_meta(stored: &str) -> Result<DocMeta> {
    let (root, rel) = meta::split_stored(stored).ok_or(format!("stored path without a root: {stored}"))?;
    Ok(meta::classify(root, rel))
}

fn known_files(conn: &Connection, targets: &[Target]) -> Result<HashMap<String, (u64, i64, String)>> {
    let mut known = HashMap::new();
    let mut stmt = conn.prepare("SELECT path, bytes, mtime_ns, doc_id FROM files WHERE path >= ?1 AND path < ?2")?;
    for t in targets {
        let (lo, hi) = t.path_range();
        let rows = stmt.query_map([lo, hi], |r| {
            Ok((r.get::<_, String>(0)?, (r.get::<_, i64>(1)? as u64, r.get::<_, i64>(2)?, r.get::<_, String>(3)?)))
        })?;
        for row in rows {
            let (path, value) = row?;
            known.insert(path, value);
        }
    }
    Ok(known)
}

fn live_docs(conn: &Connection) -> Result<Vec<LiveDoc>> {
    let mut stmt = conn.prepare(
        "SELECT f.doc_id, f.path, f.bytes FROM files f \
         JOIN (SELECT doc_id, MIN(path) AS path FROM files GROUP BY doc_id) c ON c.doc_id = f.doc_id AND c.path = f.path \
         ORDER BY f.path",
    )?;
    let rows = stmt.query_map([], |r| Ok(LiveDoc { doc_id: r.get(0)?, path: r.get(1)?, bytes: r.get::<_, i64>(2)? as u64 }))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn existing_docs(conn: &Connection) -> Result<HashMap<String, ExistingDoc>> {
    let mut stmt = conn.prepare("SELECT doc_id, path, domain, source_type, state FROM documents")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, ExistingDoc { path: r.get(1)?, domain: r.get(2)?, source_type: r.get(3)?, state: r.get(4)? }))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

fn chunk_texts(conn: &Connection, doc_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT chunk_id, text FROM chunks WHERE doc_id = ?1 ORDER BY ordinal")?;
    let rows = stmt.query_map([doc_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Commit the BM25 batch, make it searchable, then mark its documents indexed.
fn commit(conn: &Connection, writer: &mut Bm25Writer, engine: &Engine) -> Result<()> {
    writer.commit()?;
    engine.bm25.reload()?;
    conn.execute("UPDATE documents SET state = 'indexed' WHERE state = 'indexing'", [])?;
    Ok(())
}

/// Undo documents left `indexing` by an interrupted ingest, so they are indexed again.
fn recover_interrupted(conn: &mut Connection, writer: &mut Bm25Writer, engine: &Engine, rt: &Handle) -> Result<()> {
    let stale: Vec<String> = conn
        .prepare("SELECT doc_id FROM documents WHERE state = 'indexing'")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    if stale.is_empty() {
        return Ok(());
    }
    eprintln!("ingest: repairing {} document(s) from an interrupted ingest", stale.len());
    for id in &stale {
        writer.delete_doc(id);
    }
    writer.commit()?;
    engine.bm25.reload()?;
    if let Some(v) = &engine.vectors {
        rt.block_on(v.delete_docs(&stale))?;
    }
    let tx = conn.transaction()?;
    for id in &stale {
        tx.execute("DELETE FROM chunks WHERE doc_id = ?1", [id])?;
        tx.execute("DELETE FROM documents WHERE doc_id = ?1", [id])?;
    }
    tx.commit()?;
    Ok(())
}

struct Extracted {
    title: Option<String>,
    text: DocText,
    page_classes: Vec<(u32, PageClass)>,
}

/// Page texts (OCR text replacing the text layer where a page has it), page classes, and the
/// PDF heading heuristic.
fn pdf_extracted(pdf: PdfText, ocr_texts: &HashMap<u32, String>) -> Extracted {
    let page_classes: Vec<(u32, PageClass)> = pdf.pages.iter().enumerate().map(|(i, p)| (i as u32 + 1, p.class)).collect();
    let mut text = DocText::from_pages(pdf.pages.iter().enumerate().map(|(i, p)| {
        let number = i as u32 + 1;
        (number, ocr_texts.get(&number).map_or(p.text.as_str(), String::as_str))
    }));
    text.sections = headings::pdf(&text.page_slices());
    Extracted { title: pdf.title, text, page_classes }
}

fn epub_extracted(path: &Path) -> std::result::Result<Extracted, String> {
    let epub = extract::epub_text(path)?;
    let mut text = DocText::default();
    for section in epub.sections.into_iter().filter(|s| !s.text.trim().is_empty()) {
        if !text.text.is_empty() {
            text.text.push_str(chunk::PAGE_SEPARATOR);
        }
        text.sections.push(Section { offset: text.text.len(), path: section.heading_path });
        text.text.push_str(&section.text);
    }
    Ok(Extracted { title: epub.title, text, page_classes: Vec::new() })
}

fn plain_extracted(path: &Path, markdown: bool) -> std::result::Result<Extracted, String> {
    let body = extract::plain_text(path)?;
    let title = markdown
        .then(|| body.lines().find_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string())))
        .flatten()
        .filter(|t| !t.is_empty());
    let sections = if markdown { headings::markdown(&body) } else { Vec::new() };
    Ok(Extracted { title, text: DocText { text: body, pages: Vec::new(), sections, markdown }, page_classes: Vec::new() })
}

/// Extract, chunk and store one document, and add its chunks to the BM25 batch. Returns the
/// chunk count, or `None` when the file could not be read (stored as an `error` document).
#[allow(clippy::too_many_arguments)]
fn index_doc(
    conn: &mut Connection,
    writer: &Bm25Writer,
    pdfium: &mut Option<Pdfium>,
    pdfium_lib: &Path,
    doc: &LiveDoc,
    abs: &Path,
    m: &DocMeta,
    reindex: bool,
) -> Result<Option<u64>> {
    let kind = discover::doc_kind(abs).ok_or(format!("{} is not a document type", doc.path))?;
    let ocr_texts = ocr::done_texts(conn, &doc.doc_id)?;
    if kind == DocKind::Pdf && pdfium.is_none() {
        *pdfium = Some(survey::bind_pdfium(pdfium_lib)?);
    }
    let extracted = extract::catch_panics(|| match kind {
        DocKind::Pdf => Ok(pdf_extracted(extract::pdf_text(pdfium.as_ref().expect("bound above"), abs)?, &ocr_texts)),
        DocKind::Epub => epub_extracted(abs),
        DocKind::Md => plain_extracted(abs, true),
        DocKind::Txt => plain_extracted(abs, false),
    });
    let stem = Path::new(&doc.path).file_stem().map_or_else(|| doc.path.clone(), |s| s.to_string_lossy().into_owned());
    let kind_name = serde_json::to_value(kind)?.as_str().unwrap_or_default().to_string();

    let tx = conn.transaction()?;
    tx.execute("DELETE FROM chunks WHERE doc_id = ?1", [&doc.doc_id])?;
    let upsert_doc = |state: &str, error: Option<&str>, title: &str, counts: &TriageCounts, pages: usize, chunks: usize| {
        tx.execute(
            "INSERT INTO documents(doc_id, path, kind, title, domain, source_type, bytes, state, error, pages, \
             born_digital, scanned, math_heavy, empty, chunks, ocr_pages, indexed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17) \
             ON CONFLICT(doc_id) DO UPDATE SET path = excluded.path, kind = excluded.kind, title = excluded.title, \
             domain = excluded.domain, source_type = excluded.source_type, bytes = excluded.bytes, state = excluded.state, \
             error = excluded.error, pages = excluded.pages, born_digital = excluded.born_digital, scanned = excluded.scanned, \
             math_heavy = excluded.math_heavy, empty = excluded.empty, chunks = excluded.chunks, \
             ocr_pages = excluded.ocr_pages, indexed_at = excluded.indexed_at",
            params![
                doc.doc_id,
                doc.path,
                kind_name,
                title,
                m.domain,
                m.source_type,
                doc.bytes as i64,
                state,
                error,
                pages as i64,
                counts.born_digital as i64,
                counts.scanned as i64,
                counts.math_heavy as i64,
                counts.empty as i64,
                chunks as i64,
                ocr_texts.len() as i64,
                store::now()
            ],
        )
    };
    let chunks = match extracted {
        Err(error) => {
            eprintln!("ingest: {}: {error}", doc.path);
            upsert_doc("error", Some(&error), &stem, &TriageCounts::default(), 0, 0)?;
            None
        }
        Ok(x) => {
            let chunks = chunk::chunk(&x.text);
            let mut counts = TriageCounts::default();
            for (_, class) in &x.page_classes {
                counts.add(*class);
            }
            let title = x.title.as_deref().unwrap_or(&stem);
            upsert_doc("indexing", None, title, &counts, x.page_classes.len(), chunks.len())?;
            let mut insert = tx.prepare_cached(
                "INSERT INTO chunks(chunk_id, doc_id, ordinal, page_start, page_end, heading_path, text, tokens) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for c in &chunks {
                insert.execute(params![
                    chunk_id(&doc.doc_id, c.ordinal),
                    doc.doc_id,
                    c.ordinal,
                    c.page_start,
                    c.page_end,
                    serde_json::to_string(&c.heading_path)?,
                    c.text,
                    c.tokens
                ])?;
            }
            drop(insert);
            ocr::enqueue(&tx, &doc.doc_id, &x.page_classes)?;
            Some(chunks)
        }
    };
    tx.commit()?;
    if reindex {
        writer.delete_doc(&doc.doc_id);
    }
    let Some(chunks) = chunks else { return Ok(None) };
    for c in &chunks {
        writer.add(&chunk_id(&doc.doc_id, c.ordinal), &doc.doc_id, &m.domain, m.source_type, &c.text)?;
    }
    Ok(Some(chunks.len() as u64))
}

pub fn chunk_id(doc_id: &str, ordinal: u32) -> String {
    format!("{doc_id}:{ordinal}")
}

/// Embed every chunk not yet in the vector store. Chunks are marked `2` while their vectors
/// are being written; a run interrupted there removes and redoes them next time.
fn embed_pending(conn: &Connection, embedder: &Embedder, vectors: &Vectors, rt: &Handle, progress: &Progress) -> Result<u64> {
    let stored_provider = store::meta_get(conn, EMBED_PROVIDER_KEY)?;
    let stored_model = store::meta_get(conn, EMBED_MODEL_KEY)?;
    if let (Some(provider), Some(model)) = (&stored_provider, &stored_model)
        && (provider != embedder.provider_name() || model != embedder.model())
    {
        return Err(format!(
            "the vector store holds {provider}/{model} embeddings but {}/{} is configured; \
             run an ingest with rebuild=true to re-embed everything with the new model",
            embedder.provider_name(),
            embedder.model()
        )
        .into());
    }
    let stuck: Vec<String> = conn
        .prepare("SELECT chunk_id FROM chunks WHERE embedded = 2")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    if !stuck.is_empty() {
        rt.block_on(vectors.delete_chunks(&stuck))?;
        conn.execute("UPDATE chunks SET embedded = 0 WHERE embedded = 2", [])?;
    }
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM chunks c JOIN documents d ON d.doc_id = c.doc_id WHERE c.embedded = 0 AND d.state = 'indexed'",
        [],
        |r| r.get(0),
    )?;
    if pending == 0 {
        return Ok(0);
    }
    if !embedder.is_priced() {
        eprintln!("embed: RAG_EMBED_PRICE_PER_MTOK is not set; tokens are recorded but spend shows $0");
    }
    eprintln!("embed: {pending} chunks with {}/{}", embedder.provider_name(), embedder.model());
    progress.add_total(pending as u64);

    let mut select = conn.prepare(
        "SELECT c.chunk_id, c.doc_id, c.text, d.domain, d.source_type FROM chunks c JOIN documents d ON d.doc_id = c.doc_id \
         WHERE c.embedded = 0 AND d.state = 'indexed' LIMIT ?1",
    )?;
    let mut buffer: Vec<VecRow> = Vec::new();
    let mut done = 0u64;
    loop {
        let batch: Vec<(String, String, String, String, String)> = select
            .query_map([embed::BATCH_SIZE as i64], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<std::result::Result<_, _>>()?;
        if batch.is_empty() {
            break;
        }
        set_embedded(conn, batch.iter().map(|b| b.0.as_str()), 2)?;
        let texts: Vec<String> = batch.iter().map(|b| b.2.clone()).collect();
        let out = match rt.block_on(embedder.embed_batch(&texts, InputType::Document)) {
            Ok(out) => out,
            Err(e) => {
                flush(conn, vectors, &mut buffer, embedder, rt)?;
                set_embedded(conn, batch.iter().map(|b| b.0.as_str()), 0)?;
                return Err(e);
            }
        };
        store::record_spend(conn, SpendKind::Embedding, embedder.provider_name(), embedder.model(), out.tokens, out.usd)?;
        for ((chunk_id, doc_id, _, domain, source_type), vector) in batch.into_iter().zip(out.vectors) {
            buffer.push(VecRow { chunk_id, doc_id, domain, source_type, vector });
        }
        done += texts.len() as u64;
        progress.add_done(texts.len() as u64);
        if buffer.len() >= VECTOR_FLUSH_ROWS {
            flush(conn, vectors, &mut buffer, embedder, rt)?;
            eprintln!("embed: {done}/{pending} chunks");
        }
    }
    flush(conn, vectors, &mut buffer, embedder, rt)?;
    eprintln!("embed: {done}/{pending} chunks");
    Ok(done)
}

fn set_embedded<'a>(conn: &Connection, chunk_ids: impl Iterator<Item = &'a str>, state: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut update = tx.prepare_cached("UPDATE chunks SET embedded = ?2 WHERE chunk_id = ?1")?;
        for id in chunk_ids {
            update.execute(params![id, state])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn flush(conn: &Connection, vectors: &Vectors, buffer: &mut Vec<VecRow>, embedder: &Embedder, rt: &Handle) -> Result<()> {
    let Some(first) = buffer.first() else { return Ok(()) };
    if store::meta_get(conn, EMBED_MODEL_KEY)?.is_none() {
        store::meta_set(conn, EMBED_PROVIDER_KEY, embedder.provider_name())?;
        store::meta_set(conn, EMBED_MODEL_KEY, embedder.model())?;
        store::meta_set(conn, EMBED_DIMS_KEY, &first.vector.len().to_string())?;
    }
    rt.block_on(vectors.add(buffer))?;
    set_embedded(conn, buffer.iter().map(|r| r.chunk_id.as_str()), 1)?;
    buffer.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bm25::Filter;
    use crate::extract::PdfPageText;
    use crate::store::temp_dir;
    use std::fs;

    struct Fixture {
        dir: PathBuf,
        roots: Roots,
        engine: Engine,
        rt: tokio::runtime::Runtime,
    }

    impl Fixture {
        fn new(name: &str, embedder: Option<Embedder>) -> Fixture {
            let dir = temp_dir(name);
            let roots = Roots { corpus: dir.join("corpus"), notes: Some(dir.join("notes")) };
            fs::create_dir_all(&roots.corpus).unwrap();
            fs::create_dir_all(roots.notes.as_ref().unwrap()).unwrap();
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
            let engine = rt.block_on(Engine::open(&dir.join("data"), embedder, None)).unwrap();
            Fixture { dir, roots, engine, rt }
        }

        fn write(&self, rel: &str, text: &str) {
            let path = self.dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn ingest(&self) -> IngestReport {
            self.ingest_scoped(None)
        }

        fn ingest_scoped(&self, scope: Option<&Path>) -> IngestReport {
            let progress = Progress::default();
            let report = ingest(
                &self.engine,
                &self.roots,
                Path::new("unused-no-pdfs"),
                scope,
                false,
                self.rt.handle(),
                &progress,
            )
            .unwrap();
            let (done, total) = progress.get();
            assert_eq!(done, total, "progress ends complete");
            report
        }

        fn search(&self, q: &str, domains: &[&str]) -> Vec<String> {
            let filter = Filter { domains: domains.iter().map(|s| s.to_string()).collect(), source_types: vec![] };
            self.engine.bm25.search(q, 10, &filter).unwrap().into_iter().map(|(id, _)| id).collect()
        }

        fn docs(&self) -> Vec<(String, String, String, String)> {
            self.engine
                .conn()
                .prepare("SELECT path, domain, source_type, state FROM documents ORDER BY path")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
                .unwrap()
                .collect::<std::result::Result<_, _>>()
                .unwrap()
        }

        fn count(&self, sql: &str) -> i64 {
            self.engine.conn().query_row(sql, [], |r| r.get(0)).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn doc(path: &str, domain: &str, source_type: &str) -> (String, String, String, String) {
        (path.into(), domain.into(), source_type.into(), "indexed".into())
    }

    #[test]
    fn scoped_targets_must_be_inside_a_root() {
        let f = Fixture::new("ingest-targets", None);
        fs::create_dir_all(f.roots.corpus.join("math/books")).unwrap();
        let t = targets(&f.roots, Some(&f.roots.corpus.join("math/books"))).unwrap();
        assert_eq!((t[0].root, t[0].rel_prefix.as_str()), (Root::Corpus, "math/books"));
        assert_eq!(t[0].path_range(), ("corpus/math/books/".into(), "corpus/math/books0".into()));
        assert_eq!(targets(&f.roots, None).unwrap().len(), 2);
        assert_eq!(targets(&f.roots, None).unwrap()[0].path_range(), ("corpus/".into(), "corpus0".into()));
        let err = targets(&f.roots, Some(&f.dir)).unwrap_err().0;
        assert!(err.contains("not inside"), "{err}");
    }

    #[test]
    fn incremental_ingest_handles_unchanged_changed_moved_and_deleted_documents() {
        let f = Fixture::new("ingest-incremental", None);
        f.write("corpus/math/books/limits.md", "# Limits\n\nA limit describes where a function is heading.\n");
        f.write("corpus/inbox/redox.txt", "Oxidation is loss of electrons.");
        f.write("corpus/science/papers/cells.md", "Mitochondria release energy in cells.");
        f.write("notes/chemistry/acids.md", "# Acids\n\nAcids donate protons.");
        // A byte-identical copy is the same document; the first sorted path is canonical.
        f.write("corpus/science/papers/copy-of-cells.md", "Mitochondria release energy in cells.");

        let first = f.ingest();
        assert_eq!((first.files, first.documents, first.indexed, first.unchanged), (5, 4, 4, 0));
        assert_eq!(
            f.docs(),
            vec![
                doc("corpus/inbox/redox.txt", "unsorted", "unknown"),
                doc("corpus/math/books/limits.md", "math", "book"),
                doc("corpus/science/papers/cells.md", "science", "paper"),
                doc("notes/chemistry/acids.md", "chemistry", "note"),
            ]
        );
        let redox_chunks = f.search("oxidation electrons", &[]);
        assert_eq!(redox_chunks.len(), 1);

        // Nothing changed: nothing is re-indexed.
        let second = f.ingest();
        assert_eq!((second.indexed, second.unchanged, second.moved, second.removed), (0, 4, 0, 0));
        assert_eq!(f.search("oxidation electrons", &[]), redox_chunks);

        // Changed content is a new document; the old one and its chunks are gone.
        f.write("corpus/math/books/limits.md", "# Limits\n\nContinuity means no jumps.\n");
        // Moved out of the inbox: same chunks, new metadata in both stores.
        fs::create_dir_all(f.dir.join("corpus/chemistry/books")).unwrap();
        fs::rename(f.dir.join("corpus/inbox/redox.txt"), f.dir.join("corpus/chemistry/books/redox.txt")).unwrap();
        // Deleted.
        fs::remove_file(f.dir.join("notes/chemistry/acids.md")).unwrap();

        let third = f.ingest();
        assert_eq!((third.indexed, third.unchanged, third.moved, third.removed), (1, 1, 1, 2));
        assert_eq!(
            f.docs(),
            vec![
                doc("corpus/chemistry/books/redox.txt", "chemistry", "book"),
                doc("corpus/math/books/limits.md", "math", "book"),
                doc("corpus/science/papers/cells.md", "science", "paper"),
            ]
        );
        assert!(f.search("function heading", &[]).is_empty(), "old version of limits.md is still indexed");
        assert_eq!(f.search("continuity jumps", &["math"]).len(), 1);
        assert_eq!(f.search("oxidation electrons", &["chemistry"]), redox_chunks);
        assert!(f.search("oxidation electrons", &["unsorted"]).is_empty());
        assert!(f.search("acids protons", &[]).is_empty());
        assert_eq!(f.count("SELECT COUNT(*) FROM chunks WHERE doc_id NOT IN (SELECT doc_id FROM documents)"), 0);

        // Removing the canonical copy makes the other copy canonical.
        fs::remove_file(f.dir.join("corpus/science/papers/cells.md")).unwrap();
        let fourth = f.ingest();
        assert_eq!((fourth.moved, fourth.removed, fourth.indexed), (1, 0, 0));
        assert!(f.docs().contains(&doc("corpus/science/papers/copy-of-cells.md", "science", "paper")));
    }

    #[test]
    fn scoped_ingest_only_touches_its_folder() {
        let f = Fixture::new("ingest-scope", None);
        f.write("corpus/math/books/a.md", "Alpha particles.");
        f.write("corpus/science/books/b.md", "Beta decay.");
        f.ingest();
        fs::remove_file(f.dir.join("corpus/science/books/b.md")).unwrap();
        f.write("corpus/math/books/c.md", "Gamma rays.");
        let report = f.ingest_scoped(Some(&f.dir.join("corpus/math")));
        assert_eq!((report.files, report.indexed, report.removed), (2, 1, 0));
        assert_eq!(f.search("beta decay", &[]).len(), 1, "a deletion outside the scope is not applied yet");
        let full = f.ingest();
        assert_eq!(full.removed, 1);
    }

    #[test]
    fn walk_errors_prevent_removal() {
        let f = Fixture::new("ingest-walk-error", None);
        f.write("corpus/math/books/a.md", "Alpha particles.");
        f.ingest();
        // A missing notes root is a walk error: nothing may be removed because of it.
        fs::remove_dir_all(f.roots.notes.as_ref().unwrap()).unwrap();
        fs::remove_file(f.dir.join("corpus/math/books/a.md")).unwrap();
        let progress = Progress::default();
        let result = ingest(&f.engine, &f.roots, Path::new("x"), None, false, f.rt.handle(), &progress);
        assert!(result.unwrap_err().0.contains("not a directory"));
        assert_eq!(f.search("alpha", &[]).len(), 1);
    }

    #[test]
    fn an_interrupted_batch_is_redone() {
        let f = Fixture::new("ingest-interrupted", None);
        f.write("corpus/math/books/a.md", "Alpha particles.");
        f.ingest();
        // Simulate a crash after the database write but before the BM25 commit.
        f.engine.conn().execute("UPDATE documents SET state = 'indexing'", []).unwrap();
        let report = f.ingest();
        assert_eq!((report.indexed, report.unchanged), (1, 0));
        assert_eq!(f.docs(), vec![doc("corpus/math/books/a.md", "math", "book")]);
        assert_eq!(f.search("alpha", &[]).len(), 1, "exactly one copy in BM25");
    }

    #[test]
    fn rebuild_starts_over() {
        let f = Fixture::new("ingest-rebuild", None);
        f.write("corpus/math/books/a.md", "Alpha particles.");
        f.ingest();
        let progress = Progress::default();
        let report = ingest(&f.engine, &f.roots, Path::new("x"), None, true, f.rt.handle(), &progress).unwrap();
        assert_eq!((report.indexed, report.unchanged), (1, 0));
        assert_eq!(f.search("alpha", &[]).len(), 1);
    }

    #[test]
    fn unreadable_documents_are_recorded_as_errors() {
        let f = Fixture::new("ingest-errors", None);
        f.write("corpus/math/books/broken.epub", "not a zip");
        let report = f.ingest();
        assert_eq!((report.documents, report.errors, report.indexed), (1, 1, 0));
        let (state, error): (String, String) =
            f.engine.conn().query_row("SELECT state, error FROM documents", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(state, "error");
        assert!(error.contains("EPUB"), "{error}");
        // Unchanged errors are not retried.
        assert_eq!(f.ingest().unchanged, 1);
    }

    #[test]
    fn ocr_text_replaces_the_text_layer_of_its_pages() {
        let pdf = PdfText {
            title: Some("T".into()),
            pages: vec![
                PdfPageText { text: "Born digital page text.".into(), class: PageClass::BornDigital },
                PdfPageText { text: "x = y".into(), class: PageClass::MathHeavy },
                PdfPageText { text: String::new(), class: PageClass::Scanned },
            ],
        };
        let ocr = HashMap::from([(3, "Recognised scan text.".to_string())]);
        let x = pdf_extracted(pdf, &ocr);
        assert_eq!(x.text.text, "Born digital page text.\n\nx = y\n\nRecognised scan text.");
        assert_eq!(x.text.pages.iter().map(|p| p.1).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(x.page_classes, vec![(1, PageClass::BornDigital), (2, PageClass::MathHeavy), (3, PageClass::Scanned)]);
        assert_eq!(x.title.as_deref(), Some("T"));
    }

    #[test]
    fn embeddings_are_added_incrementally_and_follow_moves_and_deletes() {
        let f = Fixture::new("ingest-embed", Some(Embedder::mock(16, Some(1.0), vec![])));
        f.write("corpus/math/books/a.md", "Alpha particles are helium nuclei.");
        f.write("corpus/inbox/b.md", "Beta decay emits electrons.");
        let first = f.ingest();
        assert_eq!(first.embedded_chunks, 2);
        let vectors = f.engine.vectors.as_ref().unwrap();
        assert_eq!(f.rt.block_on(vectors.count()).unwrap(), 2);
        assert_eq!(f.count("SELECT COUNT(*) FROM chunks WHERE embedded <> 1"), 0);
        // Spend: the mock bills one token per word (5 + 4) at $1 per million tokens.
        let usd = store::spend_usd(&f.engine.conn(), SpendKind::Embedding).unwrap();
        assert!((usd - 9.0 / 1e6).abs() < 1e-12, "{usd}");
        assert_eq!(store::meta_get(&f.engine.conn(), EMBED_DIMS_KEY).unwrap().as_deref(), Some("16"));

        assert_eq!(f.ingest().embedded_chunks, 0, "unchanged chunks are not embedded again");

        fs::create_dir_all(f.dir.join("corpus/science/books")).unwrap();
        fs::rename(f.dir.join("corpus/inbox/b.md"), f.dir.join("corpus/science/books/b.md")).unwrap();
        fs::remove_file(f.dir.join("corpus/math/books/a.md")).unwrap();
        let third = f.ingest();
        assert_eq!((third.moved, third.removed, third.embedded_chunks), (1, 1, 0));
        let science = Filter { domains: vec!["science".into()], source_types: vec!["book".into()] };
        let query = embed::mock_vector("Beta decay emits electrons.", 16);
        let hits = f.rt.block_on(vectors.search(&query, 5, &science)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(f.rt.block_on(vectors.count()).unwrap(), 1);
    }

    #[test]
    fn a_failed_embedding_batch_is_retried_by_the_next_ingest() {
        let fatal = embed::ProviderError::Fatal("HTTP 401".into());
        let f = Fixture::new("ingest-embed-fail", Some(Embedder::mock(8, None, vec![fatal])));
        f.write("corpus/math/books/a.md", "Alpha particles.");
        let progress = Progress::default();
        let err = ingest(&f.engine, &f.roots, Path::new("x"), None, false, f.rt.handle(), &progress).unwrap_err().0;
        assert!(err.contains("401"), "{err}");
        assert_eq!(f.search("alpha", &[]).len(), 1, "text indexing is kept");
        assert_eq!(f.count("SELECT COUNT(*) FROM chunks WHERE embedded = 0"), 1);
        assert_eq!(f.ingest().embedded_chunks, 1);
    }

    #[test]
    fn a_different_embedding_model_needs_a_rebuild() {
        let f = Fixture::new("ingest-embed-model", Some(Embedder::mock(8, None, vec![])));
        f.write("corpus/math/books/a.md", "Alpha particles.");
        store::meta_set(&f.engine.conn(), EMBED_PROVIDER_KEY, "voyage").unwrap();
        store::meta_set(&f.engine.conn(), EMBED_MODEL_KEY, "voyage-3").unwrap();
        let progress = Progress::default();
        let err = ingest(&f.engine, &f.roots, Path::new("x"), None, false, f.rt.handle(), &progress).unwrap_err().0;
        assert!(err.contains("rebuild=true"), "{err}");
        let report = ingest(&f.engine, &f.roots, Path::new("x"), None, true, f.rt.handle(), &progress).unwrap();
        assert_eq!(report.embedded_chunks, 1);
    }
}

//! OCR queue for PDF pages whose text layer is missing or unreliable (`scanned` and
//! `math_heavy`), behind a provider trait.
//!
//! Ingest queues those pages as `pending` and still indexes whatever text layer they have.
//! When `RAG_OCR_PROVIDER` names a provider, the ingest job sends pending pages to it, stores
//! the text (`done`), records spend, and re-indexes each changed document with the OCR text in
//! place of those pages' text layer. A page whose OCR fails stays `pending` with the error and
//! is retried by the next ingest. No provider is implemented yet (P3 pilots choose one), so any
//! value of `RAG_OCR_PROVIDER` is rejected at startup instead of being silently ignored.

use crate::error::Result;
use crate::store::{self, SpendKind};
use crate::triage::PageClass;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pages fetched from the queue per database read.
const QUEUE_BATCH: usize = 256;

pub struct OcrOutput {
    pub text: String,
    pub usd: f64,
}

pub trait OcrProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    /// Recognise page `page` (1-based) of the PDF at `pdf`.
    fn recognize(&self, pdf: &Path, page: u32) -> Result<OcrOutput>;
}

pub fn provider_from_env() -> Result<Option<Box<dyn OcrProvider>>> {
    match std::env::var("RAG_OCR_PROVIDER").ok().filter(|v| !v.trim().is_empty()) {
        None => Ok(None),
        Some(name) => Err(format!(
            "RAG_OCR_PROVIDER={name:?} is not supported: no OCR provider is implemented yet. \
             Unset it to keep scanned and math-heavy pages queued."
        )
        .into()),
    }
}

pub fn needs_ocr(class: PageClass) -> bool {
    matches!(class, PageClass::Scanned | PageClass::MathHeavy)
}

fn class_name(class: PageClass) -> &'static str {
    match class {
        PageClass::BornDigital => "born_digital",
        PageClass::Scanned => "scanned",
        PageClass::MathHeavy => "math_heavy",
        PageClass::Empty => "empty",
    }
}

/// Queue the pages of `doc_id` that need OCR. Pages already queued (including finished ones)
/// are left as they are.
pub fn enqueue(conn: &Connection, doc_id: &str, pages: &[(u32, PageClass)]) -> Result<()> {
    let mut insert = conn.prepare_cached(
        "INSERT OR IGNORE INTO ocr_queue(doc_id, page, class, state, updated_at) VALUES (?1, ?2, ?3, 'pending', ?4)",
    )?;
    for &(page, class) in pages.iter().filter(|(_, c)| needs_ocr(*c)) {
        insert.execute(params![doc_id, page, class_name(class), store::now()])?;
    }
    Ok(())
}

/// OCR text of the finished pages of `doc_id`, by page number.
pub fn done_texts(conn: &Connection, doc_id: &str) -> Result<HashMap<u32, String>> {
    let mut stmt = conn.prepare_cached("SELECT page, text FROM ocr_queue WHERE doc_id = ?1 AND state = 'done'")?;
    let rows = stmt.query_map([doc_id], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// `(pending, done)` page counts.
pub fn counts(conn: &Connection) -> Result<(u64, u64)> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(state = 'pending'), 0), COALESCE(SUM(state = 'done'), 0) FROM ocr_queue",
        [],
        |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
    )?)
}

/// Send every pending page to `provider`. `pdf_path` resolves a document to its file. Returns
/// the number of pages recognised. Stops at the first failure, which is recorded on the page
/// and returned; pages finished before it are kept. Documents are re-indexed with their OCR
/// text afterwards (see [`docs_to_reindex`]).
pub fn process(
    conn: &Connection,
    provider: &dyn OcrProvider,
    pdf_path: impl Fn(&str) -> Option<PathBuf>,
    mut on_page: impl FnMut(),
) -> Result<u64> {
    let mut done = 0;
    let mut after = (String::new(), 0u32);
    loop {
        let batch: Vec<(String, u32)> = conn
            .prepare_cached(
                "SELECT doc_id, page FROM ocr_queue WHERE state = 'pending' AND (doc_id, page) > (?1, ?2) \
                 ORDER BY doc_id, page LIMIT ?3",
            )?
            .query_map(params![after.0, after.1, QUEUE_BATCH as i64], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        let Some(last) = batch.last() else { break };
        after = last.clone();
        for (doc_id, page) in batch {
            let result = match pdf_path(&doc_id) {
                Some(path) => provider.recognize(&path, page),
                None => Err(format!("document {doc_id} has no file").into()),
            };
            match result {
                Ok(out) => {
                    conn.execute(
                        "UPDATE ocr_queue SET state = 'done', text = ?3, error = NULL, provider = ?4, updated_at = ?5 \
                         WHERE doc_id = ?1 AND page = ?2",
                        params![doc_id, page, out.text, provider.name(), store::now()],
                    )?;
                    store::record_spend(conn, SpendKind::Ocr, provider.name(), provider.model(), 0, out.usd)?;
                    done += 1;
                    on_page();
                }
                Err(e) => {
                    conn.execute(
                        "UPDATE ocr_queue SET error = ?3, provider = ?4, updated_at = ?5 WHERE doc_id = ?1 AND page = ?2",
                        params![doc_id, page, e.0, provider.name(), store::now()],
                    )?;
                    return Err(format!("OCR of {doc_id} page {page} failed: {e}").into());
                }
            }
        }
    }
    Ok(done)
}

/// Indexed documents with more finished OCR pages than they were last indexed with.
pub fn docs_to_reindex(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT d.doc_id FROM documents d \
         JOIN (SELECT doc_id, COUNT(*) AS n FROM ocr_queue WHERE state = 'done' GROUP BY doc_id) q ON q.doc_id = d.doc_id \
         WHERE d.state = 'indexed' AND q.n > d.ocr_pages ORDER BY d.doc_id",
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::temp_dir;
    use std::sync::Mutex;

    struct MockOcr {
        fail_page: Option<u32>,
        seen: Mutex<Vec<(PathBuf, u32)>>,
    }

    impl OcrProvider for MockOcr {
        fn name(&self) -> &str {
            "mock-ocr"
        }
        fn model(&self) -> &str {
            "m"
        }
        fn recognize(&self, pdf: &Path, page: u32) -> Result<OcrOutput> {
            self.seen.lock().unwrap().push((pdf.to_path_buf(), page));
            if Some(page) == self.fail_page {
                return Err("provider down".into());
            }
            Ok(OcrOutput { text: format!("ocr text of page {page}"), usd: 0.001 })
        }
    }

    fn state(conn: &Connection, doc: &str, page: u32) -> (String, Option<String>, Option<String>) {
        conn.query_row(
            "SELECT state, text, error FROM ocr_queue WHERE doc_id = ?1 AND page = ?2",
            params![doc, page],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    const PAGES: [(u32, PageClass); 4] =
        [(1, PageClass::BornDigital), (2, PageClass::Scanned), (3, PageClass::MathHeavy), (4, PageClass::Empty)];

    #[test]
    fn only_scanned_and_math_heavy_pages_are_queued_and_stay_pending() {
        let dir = temp_dir("ocr-queue");
        let conn = store::open(&dir).unwrap();
        enqueue(&conn, "d", &PAGES).unwrap();
        assert_eq!(counts(&conn).unwrap(), (2, 0));
        assert_eq!(state(&conn, "d", 2).0, "pending");
        let classes: Vec<String> = conn
            .prepare("SELECT class FROM ocr_queue ORDER BY page")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(classes, ["scanned", "math_heavy"]);
        assert!(done_texts(&conn, "d").unwrap().is_empty());
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_provider_fills_the_queue_records_spend_and_reports_changed_docs() {
        let dir = temp_dir("ocr-process");
        let conn = store::open(&dir).unwrap();
        enqueue(&conn, "d1", &PAGES).unwrap();
        enqueue(&conn, "d2", &[(7, PageClass::Scanned)]).unwrap();
        let ocr = MockOcr { fail_page: None, seen: Mutex::new(Vec::new()) };
        let mut pages = 0;
        let done = process(&conn, &ocr, |d| Some(PathBuf::from(format!("{d}.pdf"))), || pages += 1).unwrap();
        assert_eq!((done, pages), (3, 3));
        assert_eq!(counts(&conn).unwrap(), (0, 3));
        assert_eq!(done_texts(&conn, "d1").unwrap(), HashMap::from([(2, "ocr text of page 2".into()), (3, "ocr text of page 3".into())]));
        assert_eq!(ocr.seen.lock().unwrap()[2], (PathBuf::from("d2.pdf"), 7));
        assert!((store::spend_usd(&conn, SpendKind::Ocr).unwrap() - 0.003).abs() < 1e-12);

        // Re-queueing a finished document keeps its OCR text.
        enqueue(&conn, "d1", &PAGES).unwrap();
        assert_eq!(counts(&conn).unwrap(), (0, 3));

        // Indexed documents that have not used their OCR text yet need re-indexing.
        for (doc, ocr_pages) in [("d1", 0), ("d2", 1)] {
            conn.execute(
                "INSERT INTO documents(doc_id, path, kind, title, domain, source_type, bytes, state, ocr_pages) \
                 VALUES (?1, ?1, 'pdf', 't', 'math', 'book', 1, 'indexed', ?2)",
                params![doc, ocr_pages],
            )
            .unwrap();
        }
        assert_eq!(docs_to_reindex(&conn).unwrap(), ["d1"]);
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failure_is_recorded_and_the_page_stays_pending() {
        let dir = temp_dir("ocr-fail");
        let conn = store::open(&dir).unwrap();
        enqueue(&conn, "d", &PAGES).unwrap();
        let ocr = MockOcr { fail_page: Some(3), seen: Mutex::new(Vec::new()) };
        let err = process(&conn, &ocr, |_| Some(PathBuf::from("d.pdf")), || {}).unwrap_err().0;
        assert!(err.contains("page 3") && err.contains("provider down"), "{err}");
        assert_eq!(state(&conn, "d", 2).0, "done");
        let (s, text, error) = state(&conn, "d", 3);
        assert_eq!((s.as_str(), text, error.as_deref()), ("pending", None, Some("provider down")));
        assert_eq!(counts(&conn).unwrap(), (1, 1));
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

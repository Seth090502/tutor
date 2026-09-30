//! Library browsing (contract C9): the indexed documents, one document's outline, and its chunks
//! in reading order. Only `indexed` documents are listed; `error` documents are counted by
//! `/status` and documents mid-ingest appear once their batch is committed.

use crate::error::Result;
use crate::meta::SOURCE_TYPES;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};
use serde::{Deserialize, Serialize};

pub const DEFAULT_LIMIT: u32 = 50;
pub const DEFAULT_CHUNK_LIMIT: u32 = 20;
pub const MAX_LIMIT: u32 = 200;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Doc {
    pub doc_id: String,
    pub path: String,
    pub title: String,
    pub domain: String,
    pub source_type: String,
    pub pages: u64,
    pub chunks: u64,
    pub bytes: u64,
    /// When the document's current content was indexed (epoch seconds).
    pub added_at: Option<f64>,
    /// Pages of this document waiting for OCR.
    pub ocr_pending: u64,
}

#[derive(Debug, Serialize)]
pub struct DocList {
    pub total: u64,
    pub documents: Vec<Doc>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineEntry {
    pub heading_path: Vec<String>,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
    /// The first chunk under this heading path.
    pub chunk_id: String,
}

#[derive(Debug, Serialize)]
pub struct DocDetail {
    #[serde(flatten)]
    pub doc: Doc,
    pub outline: Vec<OutlineEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChunkView {
    pub chunk_id: String,
    pub ordinal: u32,
    pub text: String,
    pub page_start: Option<u32>,
    pub page_end: Option<u32>,
    pub heading_path: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChunkPage {
    pub chunks: Vec<ChunkView>,
    /// The ordinal to pass as `from` for the next page, or null after the last chunk.
    pub next: Option<u32>,
}

/// `GET /documents` query parameters. Empty values mean "not given".
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    pub domain: Option<String>,
    pub source_type: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u64>,
}

/// `GET /documents/{doc_id}/chunks` query parameters.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunksQuery {
    pub from: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// A to Z, ignoring ASCII case.
    Title,
    /// Newest first.
    Added,
    /// Most pages first.
    Pages,
}

/// Validated list parameters.
#[derive(Debug)]
pub struct ListParams {
    domain: Option<String>,
    source_type: Option<String>,
    q: Option<String>,
    sort: Sort,
    limit: u32,
    offset: u64,
}

fn given(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

fn check_limit(limit: Option<u32>, default: u32) -> std::result::Result<u32, String> {
    let limit = limit.unwrap_or(default);
    if (1..=MAX_LIMIT).contains(&limit) { Ok(limit) } else { Err(format!("limit must be between 1 and {MAX_LIMIT}")) }
}

impl ListParams {
    /// The parameters, or the message for a 400.
    pub fn parse(query: ListQuery) -> std::result::Result<ListParams, String> {
        let source_type = given(query.source_type);
        if let Some(bad) = source_type.as_deref().filter(|s| !SOURCE_TYPES.contains(s)) {
            return Err(format!("unknown source_type {bad:?}; expected one of {SOURCE_TYPES:?}"));
        }
        let sort = match given(query.sort).as_deref() {
            None | Some("title") => Sort::Title,
            Some("added") => Sort::Added,
            Some("pages") => Sort::Pages,
            Some(other) => return Err(format!("unknown sort {other:?}; expected title, added or pages")),
        };
        Ok(ListParams {
            domain: given(query.domain),
            source_type,
            q: given(query.q),
            sort,
            limit: check_limit(query.limit, DEFAULT_LIMIT)?,
            offset: query.offset.unwrap_or(0),
        })
    }
}

/// `(from, limit)` for a chunk page, or the message for a 400.
pub fn chunk_params(query: ChunksQuery) -> std::result::Result<(u32, u32), String> {
    Ok((query.from.unwrap_or(0), check_limit(query.limit, DEFAULT_CHUNK_LIMIT)?))
}

const DOC_COLUMNS: &str = "d.doc_id, d.path, d.title, d.domain, d.source_type, d.pages, d.chunks, d.bytes, d.indexed_at, \
     (SELECT COUNT(*) FROM ocr_queue q WHERE q.doc_id = d.doc_id AND q.state = 'pending')";

fn doc_row(r: &Row) -> rusqlite::Result<Doc> {
    Ok(Doc {
        doc_id: r.get(0)?,
        path: r.get(1)?,
        title: r.get(2)?,
        domain: r.get(3)?,
        source_type: r.get(4)?,
        pages: r.get::<_, i64>(5)? as u64,
        chunks: r.get::<_, i64>(6)? as u64,
        bytes: r.get::<_, i64>(7)? as u64,
        added_at: r.get(8)?,
        ocr_pending: r.get::<_, i64>(9)? as u64,
    })
}

/// `q` as a LIKE pattern matching it anywhere, with `%`, `_` and `\` taken literally.
fn like_pattern(q: &str) -> String {
    let mut pattern = String::from("%");
    for c in q.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

pub fn list(conn: &Connection, p: &ListParams) -> Result<DocList> {
    let mut filter = String::from("d.state = 'indexed'");
    let mut args: Vec<SqlValue> = Vec::new();
    if let Some(domain) = &p.domain {
        filter.push_str(" AND d.domain = ?");
        args.push(SqlValue::Text(domain.to_lowercase()));
    }
    if let Some(source_type) = &p.source_type {
        filter.push_str(" AND d.source_type = ?");
        args.push(SqlValue::Text(source_type.clone()));
    }
    if let Some(q) = &p.q {
        filter.push_str(" AND (d.title LIKE ? ESCAPE '\\' OR d.path LIKE ? ESCAPE '\\')");
        args.extend([SqlValue::Text(like_pattern(q)), SqlValue::Text(like_pattern(q))]);
    }
    let total: i64 =
        conn.query_row(&format!("SELECT COUNT(*) FROM documents d WHERE {filter}"), params_from_iter(&args), |r| r.get(0))?;
    let order = match p.sort {
        Sort::Title => "d.title COLLATE NOCASE, d.doc_id",
        Sort::Added => "d.indexed_at DESC, d.doc_id",
        Sort::Pages => "d.pages DESC, d.title COLLATE NOCASE, d.doc_id",
    };
    args.extend([SqlValue::Integer(p.limit.into()), SqlValue::Integer(p.offset.min(i64::MAX as u64) as i64)]);
    let mut stmt =
        conn.prepare(&format!("SELECT {DOC_COLUMNS} FROM documents d WHERE {filter} ORDER BY {order} LIMIT ? OFFSET ?"))?;
    let documents = stmt.query_map(params_from_iter(&args), doc_row)?.collect::<std::result::Result<_, _>>()?;
    Ok(DocList { total: total as u64, documents })
}

fn find(conn: &Connection, doc_id: &str) -> Result<Option<Doc>> {
    Ok(conn
        .query_row(&format!("SELECT {DOC_COLUMNS} FROM documents d WHERE d.doc_id = ?1 AND d.state = 'indexed'"), [doc_id], doc_row)
        .optional()?)
}

fn heading_path(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

/// The document and its outline: each distinct, non-empty heading path of its chunks, in reading
/// order, with the first chunk under it and the pages from that chunk to the end of the run.
pub fn document(conn: &Connection, doc_id: &str) -> Result<Option<DocDetail>> {
    let Some(doc) = find(conn, doc_id)? else { return Ok(None) };
    let mut stmt = conn.prepare("SELECT chunk_id, page_start, page_end, heading_path FROM chunks WHERE doc_id = ?1 ORDER BY ordinal")?;
    let mut rows = stmt.query([doc_id])?;
    let mut outline: Vec<OutlineEntry> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // The previous chunk's heading path (JSON), and whether its run is the last outline entry.
    let (mut previous, mut extending) = (None::<String>, false);
    while let Some(r) = rows.next()? {
        let path: String = r.get(3)?;
        let (page_start, page_end): (Option<u32>, Option<u32>) = (r.get(1)?, r.get(2)?);
        if previous.as_deref() == Some(path.as_str()) {
            if let Some(last) = outline.last_mut().filter(|_| extending) {
                last.page_end = page_end.or(last.page_end);
            }
            continue;
        }
        previous = Some(path.clone());
        let heading_path = heading_path(&path);
        extending = !heading_path.is_empty() && seen.insert(path);
        if extending {
            outline.push(OutlineEntry { heading_path, page_start, page_end, chunk_id: r.get(0)? });
        }
    }
    Ok(Some(DocDetail { doc, outline }))
}

/// Up to `limit` chunks of the document from ordinal `from`, in reading order.
pub fn chunks(conn: &Connection, doc_id: &str, from: u32, limit: u32) -> Result<Option<ChunkPage>> {
    if find(conn, doc_id)?.is_none() {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT chunk_id, ordinal, text, page_start, page_end, heading_path FROM chunks \
         WHERE doc_id = ?1 AND ordinal >= ?2 ORDER BY ordinal LIMIT ?3",
    )?;
    let mut chunks: Vec<ChunkView> = stmt
        .query_map(params![doc_id, from, limit + 1], |r| {
            Ok(ChunkView {
                chunk_id: r.get(0)?,
                ordinal: r.get(1)?,
                text: r.get(2)?,
                page_start: r.get(3)?,
                page_end: r.get(4)?,
                heading_path: heading_path(&r.get::<_, String>(5)?),
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    let next = if chunks.len() > limit as usize { chunks.pop().map(|c| c.ordinal) } else { None };
    Ok(Some(ChunkPage { chunks, next }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{self, temp_dir};

    fn insert_doc(conn: &Connection, id: &str, title: &str, (domain, source_type): (&str, &str), pages: i64, at: f64, state: &str) {
        conn.execute(
            "INSERT INTO documents(doc_id, path, kind, title, domain, source_type, bytes, state, pages, chunks, indexed_at) \
             VALUES (?1, ?2, 'pdf', ?3, ?4, ?5, 100, ?6, ?7, 0, ?8)",
            params![id, format!("corpus/{domain}/books/{id}.pdf"), title, domain, source_type, state, pages, at],
        )
        .unwrap();
    }

    fn insert_chunk(conn: &Connection, doc: &str, ordinal: u32, pages: (Option<u32>, Option<u32>), path: &[&str]) {
        conn.execute(
            "INSERT INTO chunks(chunk_id, doc_id, ordinal, page_start, page_end, heading_path, text, tokens) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            params![format!("{doc}:{ordinal}"), doc, ordinal, pages.0, pages.1, serde_json::to_string(path).unwrap(), format!("text {ordinal}")],
        )
        .unwrap();
    }

    fn query(pairs: &[(&str, &str)]) -> ListParams {
        let q: ListQuery = serde_json::from_value(serde_json::Value::Object(
            pairs
                .iter()
                .map(|(k, v)| {
                    let v = match *k {
                        "limit" | "offset" => serde_json::json!(v.parse::<u64>().unwrap()),
                        _ => serde_json::json!(v),
                    };
                    (k.to_string(), v)
                })
                .collect(),
        ))
        .unwrap();
        ListParams::parse(q).unwrap()
    }

    fn ids(list: &DocList) -> Vec<&str> {
        list.documents.iter().map(|d| d.doc_id.as_str()).collect()
    }

    #[test]
    fn lists_filter_sort_and_page_indexed_documents() {
        let dir = temp_dir("library-list");
        let conn = store::open(&dir).unwrap();
        insert_doc(&conn, "a", "calculus", ("math", "book"), 300, 3.0, "indexed");
        insert_doc(&conn, "b", "Algebra 100%", ("math", "paper"), 20, 1.0, "indexed");
        insert_doc(&conn, "c", "Cells", ("science", "book"), 120, 2.0, "indexed");
        insert_doc(&conn, "d", "Broken", ("math", "book"), 0, 4.0, "error");
        insert_doc(&conn, "e", "Pending", ("math", "book"), 0, 5.0, "indexing");
        conn.execute("INSERT INTO ocr_queue(doc_id, page, class, state) VALUES ('a', 1, 'scanned', 'pending'), ('a', 2, 'scanned', 'done')", [])
            .unwrap();

        let all = list(&conn, &query(&[])).unwrap();
        assert_eq!((all.total, ids(&all)), (3, vec!["b", "a", "c"]), "title order ignores case; only indexed documents");
        let a = &all.documents[1];
        assert_eq!((a.path.as_str(), a.pages, a.bytes, a.added_at, a.ocr_pending), ("corpus/math/books/a.pdf", 300, 100, Some(3.0), 1));
        assert_eq!(ids(&list(&conn, &query(&[("sort", "added")])).unwrap()), ["a", "c", "b"]);
        assert_eq!(ids(&list(&conn, &query(&[("sort", "pages")])).unwrap()), ["a", "c", "b"]);
        assert_eq!(ids(&list(&conn, &query(&[("domain", "MATH")])).unwrap()), ["b", "a"]);
        assert_eq!(ids(&list(&conn, &query(&[("domain", "math"), ("source_type", "book")])).unwrap()), ["a"]);
        assert_eq!(ids(&list(&conn, &query(&[("q", "CELL")])).unwrap()), ["c"], "title substring, any case");
        assert_eq!(ids(&list(&conn, &query(&[("q", "science/books")])).unwrap()), ["c"], "path substring");
        assert_eq!(ids(&list(&conn, &query(&[("q", "100%")])).unwrap()), ["b"]);
        assert!(list(&conn, &query(&[("q", "_")])).unwrap().documents.is_empty(), "wildcards are literal");
        assert_eq!(ids(&list(&conn, &query(&[("domain", ""), ("q", "")])).unwrap()).len(), 3, "empty values are no filter");
        let page = list(&conn, &query(&[("limit", "1"), ("offset", "1")])).unwrap();
        assert_eq!((page.total, ids(&page)), (3, vec!["a"]));
        assert!(list(&conn, &query(&[("offset", "10")])).unwrap().documents.is_empty());

        for bad in [
            ListQuery { limit: Some(0), ..Default::default() },
            ListQuery { limit: Some(201), ..Default::default() },
            ListQuery { sort: Some("size".into()), ..Default::default() },
            ListQuery { source_type: Some("books".into()), ..Default::default() },
        ] {
            assert!(ListParams::parse(bad).is_err());
        }
        assert_eq!(ListParams::parse(ListQuery { limit: Some(200), ..Default::default() }).unwrap().limit, 200);
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn outline_and_chunk_pages_follow_reading_order() {
        let dir = temp_dir("library-doc");
        let conn = store::open(&dir).unwrap();
        insert_doc(&conn, "a", "Calculus", ("math", "book"), 9, 1.0, "indexed");
        insert_doc(&conn, "x", "Broken", ("math", "book"), 0, 1.0, "error");
        // Inserted out of order: reading order is by ordinal.
        insert_chunk(&conn, "a", 3, (Some(4), Some(5)), &["Ch 1", "1.2 Limits"]);
        insert_chunk(&conn, "a", 0, (Some(1), Some(1)), &[]);
        insert_chunk(&conn, "a", 1, (Some(1), Some(2)), &["Ch 1"]);
        insert_chunk(&conn, "a", 2, (Some(2), Some(3)), &["Ch 1"]);
        insert_chunk(&conn, "a", 4, (Some(5), Some(6)), &["Ch 1", "1.2 Limits"]);
        insert_chunk(&conn, "a", 5, (Some(7), Some(7)), &["Ch 1"]);
        insert_chunk(&conn, "a", 6, (Some(8), Some(9)), &["Ch 2"]);

        let detail = document(&conn, "a").unwrap().unwrap();
        assert_eq!(detail.doc.title, "Calculus");
        let entry = |path: &[&str], start, end, chunk: &str| OutlineEntry {
            heading_path: path.iter().map(|s| s.to_string()).collect(),
            page_start: Some(start),
            page_end: Some(end),
            chunk_id: chunk.into(),
        };
        assert_eq!(
            detail.outline,
            vec![entry(&["Ch 1"], 1, 3, "a:1"), entry(&["Ch 1", "1.2 Limits"], 4, 6, "a:3"), entry(&["Ch 2"], 8, 9, "a:6")],
            "distinct heading paths, first run only; the preamble without a heading is left out"
        );
        let json = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["doc_id"], "a", "Doc fields sit beside the outline");
        assert!(json["outline"].is_array());
        assert!(document(&conn, "x").unwrap().is_none(), "error documents are not browsable");
        assert!(document(&conn, "missing").unwrap().is_none());

        let first = chunks(&conn, "a", 0, 3).unwrap().unwrap();
        assert_eq!(first.chunks.iter().map(|c| c.ordinal).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(first.next, Some(3));
        assert_eq!(first.chunks[1].heading_path, ["Ch 1"]);
        assert_eq!((first.chunks[1].page_start, first.chunks[1].page_end, first.chunks[1].text.as_str()), (Some(1), Some(2), "text 1"));
        let last = chunks(&conn, "a", 3, 4).unwrap().unwrap();
        assert_eq!((last.chunks.len(), last.next), (4, None));
        let past = chunks(&conn, "a", 50, 20).unwrap().unwrap();
        assert_eq!((past.chunks.len(), past.next), (0, None));
        assert!(chunks(&conn, "missing", 0, 20).unwrap().is_none());
        assert_eq!(chunk_params(ChunksQuery::default()), Ok((0, DEFAULT_CHUNK_LIMIT)));
        assert!(chunk_params(ChunksQuery { from: None, limit: Some(0) }).is_err());
        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

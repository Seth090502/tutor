//! Chunk vectors in LanceDB: `--data/lancedb`, table `chunks`, cosine distance.
//!
//! The table is created by the first write, so nothing exists until an embedding provider has
//! produced vectors. It carries `domain` and `source_type` so filters are applied inside the
//! vector search (LanceDB pre-filters by default). An IVF-PQ index is built once the table
//! has [`INDEX_MIN_ROWS`] rows; below that a flat scan is exact and fast enough.

use crate::bm25::Filter;
use crate::error::Result;
use futures::TryStreamExt;
use lancedb::DistanceType;
use lancedb::arrow::arrow_array::types::Float32Type;
use lancedb::arrow::arrow_array::{Array, FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use lancedb::arrow::arrow_schema::{DataType, Field, Schema};
use lancedb::index::Index;
use lancedb::index::vector::IvfPqIndexBuilder;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use lancedb::table::OptimizeAction;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const TABLE: &str = "chunks";

/// Rows at which an IVF-PQ index is created (LanceDB's guidance: somewhere in 100K-1M).
pub const INDEX_MIN_ROWS: usize = 100_000;

/// Values per `IN (...)` list in deletes.
const DELETE_BATCH: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct VecRow {
    pub chunk_id: String,
    pub doc_id: String,
    pub domain: String,
    pub source_type: String,
    pub vector: Vec<f32>,
}

pub struct Vectors {
    db: lancedb::Connection,
    table: Mutex<Option<lancedb::Table>>,
}

/// A SQL string literal.
pub fn sql_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql_in(column: &str, values: &[String]) -> String {
    let list: Vec<String> = values.iter().map(|v| sql_str(v)).collect();
    format!("{column} IN ({})", list.join(", "))
}

/// The filter as a SQL predicate, or `None` when it does not restrict anything.
pub fn filter_sql(filter: &Filter) -> Option<String> {
    let mut parts = Vec::new();
    if !filter.domains.is_empty() {
        parts.push(sql_in("domain", &filter.domains));
    }
    if !filter.source_types.is_empty() {
        parts.push(sql_in("source_type", &filter.source_types));
    }
    (!parts.is_empty()).then(|| parts.join(" AND "))
}

impl Vectors {
    /// Connect to (creating) the LanceDB directory and open the table if it exists.
    pub async fn open(dir: &Path) -> Result<Vectors> {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let uri = dir.to_str().ok_or("the LanceDB path is not valid UTF-8")?.replace('\\', "/");
        // Zero: every read sees the latest committed version, including writes by the ingest job.
        let db = lancedb::connect(&uri).read_consistency_interval(Duration::ZERO).execute().await?;
        let table = if db.table_names().execute().await?.iter().any(|t| t == TABLE) {
            Some(db.open_table(TABLE).execute().await?)
        } else {
            None
        };
        Ok(Vectors { db, table: Mutex::new(table) })
    }

    fn table(&self) -> Option<lancedb::Table> {
        self.table.lock().expect("vector table lock").clone()
    }

    pub async fn count(&self) -> Result<usize> {
        match self.table() {
            Some(table) => Ok(table.count_rows(None).await?),
            None => Ok(0),
        }
    }

    pub async fn add(&self, rows: &[VecRow]) -> Result<()> {
        let Some(first) = rows.first() else { return Ok(()) };
        let dims = first.vector.len();
        if dims == 0 || rows.iter().any(|r| r.vector.len() != dims) {
            return Err("vectors in one batch must share a non-zero dimension".into());
        }
        let schema = Arc::new(Schema::new(vec![
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("domain", DataType::Utf8, false),
            Field::new("source_type", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), dims as i32),
                true,
            ),
        ]));
        let strings = |f: fn(&VecRow) -> &str| Arc::new(StringArray::from_iter_values(rows.iter().map(f)));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                strings(|r| &r.chunk_id),
                strings(|r| &r.doc_id),
                strings(|r| &r.domain),
                strings(|r| &r.source_type),
                Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    rows.iter().map(|r| Some(r.vector.iter().map(|v| Some(*v)).collect::<Vec<_>>())),
                    dims as i32,
                )),
            ],
        )?;
        match self.table() {
            Some(table) => {
                table.add(batch).execute().await?;
            }
            None => {
                let table = self.db.create_table(TABLE, batch).execute().await?;
                *self.table.lock().expect("vector table lock") = Some(table);
            }
        }
        Ok(())
    }

    async fn delete_in(&self, column: &str, values: &[String]) -> Result<()> {
        let Some(table) = self.table() else { return Ok(()) };
        for batch in values.chunks(DELETE_BATCH) {
            table.delete(sql_in(column, batch).as_str()).await?;
        }
        Ok(())
    }

    pub async fn delete_docs(&self, doc_ids: &[String]) -> Result<()> {
        self.delete_in("doc_id", doc_ids).await
    }

    pub async fn delete_chunks(&self, chunk_ids: &[String]) -> Result<()> {
        self.delete_in("chunk_id", chunk_ids).await
    }

    /// Update the filter columns of a document's vectors after it moved in the corpus.
    pub async fn set_doc_meta(&self, doc_id: &str, domain: &str, source_type: &str) -> Result<()> {
        let Some(table) = self.table() else { return Ok(()) };
        table
            .update()
            .only_if(format!("doc_id = {}", sql_str(doc_id)))
            .column("domain", sql_str(domain))
            .column("source_type", sql_str(source_type))
            .execute()
            .await?;
        Ok(())
    }

    /// Nearest `limit` chunks to `vector`, as `(chunk_id, cosine distance)`, closest first.
    pub async fn search(&self, vector: &[f32], limit: usize, filter: &Filter) -> Result<Vec<(String, f32)>> {
        let Some(table) = self.table() else { return Ok(Vec::new()) };
        let mut query = table
            .query()
            .nearest_to(vector)?
            .distance_type(DistanceType::Cosine)
            .limit(limit)
            .select(Select::columns(&["chunk_id"]));
        if let Some(predicate) = filter_sql(filter) {
            query = query.only_if(predicate);
        }
        let batches: Vec<RecordBatch> = query.execute().await?.try_collect().await?;
        let mut hits = Vec::new();
        for batch in &batches {
            let ids = batch
                .column_by_name("chunk_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or("vector search result without chunk_id")?;
            let distances = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                .ok_or("vector search result without _distance")?;
            for i in 0..batch.num_rows() {
                hits.push((ids.value(i).to_string(), distances.value(i)));
            }
        }
        hits.sort_by(|a, b| a.1.total_cmp(&b.1));
        Ok(hits)
    }

    /// Compact small fragments, prune old versions (LanceDB's default age), update indexes, and
    /// create the IVF-PQ index once the table is large enough.
    pub async fn maintain(&self) -> Result<()> {
        let Some(table) = self.table() else { return Ok(()) };
        let rows = table.count_rows(None).await?;
        let indexed = table.list_indices().await?.iter().any(|i| i.columns.iter().any(|c| c == "vector"));
        if !indexed && rows >= INDEX_MIN_ROWS {
            eprintln!("vectors: building the IVF-PQ index over {rows} rows");
            table
                .create_index(&["vector"], Index::IvfPq(IvfPqIndexBuilder::default().distance_type(DistanceType::Cosine)))
                .execute()
                .await?;
        }
        table.optimize(OptimizeAction::All).await?;
        Ok(())
    }

    pub async fn drop_all(&self) -> Result<()> {
        let existing = self.table.lock().expect("vector table lock").take();
        if existing.is_some() {
            self.db.drop_table(TABLE, &[]).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::temp_dir;

    fn row(chunk_id: &str, domain: &str, source_type: &str, vector: [f32; 3]) -> VecRow {
        VecRow {
            chunk_id: chunk_id.into(),
            doc_id: chunk_id.split(':').next().unwrap().into(),
            domain: domain.into(),
            source_type: source_type.into(),
            vector: vector.to_vec(),
        }
    }

    fn ids(hits: &[(String, f32)]) -> Vec<&str> {
        hits.iter().map(|(id, _)| id.as_str()).collect()
    }

    fn filter(domains: &[&str], source_types: &[&str]) -> Filter {
        Filter {
            domains: domains.iter().map(|s| s.to_string()).collect(),
            source_types: source_types.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn filter_sql_quotes_values() {
        assert_eq!(filter_sql(&Filter::default()), None);
        assert_eq!(
            filter_sql(&filter(&["math", "o'brien"], &["book"])).unwrap(),
            "domain IN ('math', 'o''brien') AND source_type IN ('book')"
        );
    }

    #[tokio::test]
    async fn adds_searches_filters_updates_and_deletes() {
        let dir = temp_dir("vectors");
        let v = Vectors::open(&dir).await.unwrap();
        assert_eq!(v.count().await.unwrap(), 0);
        assert!(v.search(&[1.0, 0.0, 0.0], 5, &Filter::default()).await.unwrap().is_empty());

        v.add(&[row("a:0", "math", "book", [1.0, 0.0, 0.0]), row("a:1", "math", "book", [0.9, 0.1, 0.0])])
            .await
            .unwrap();
        v.add(&[row("b:0", "chemistry", "paper", [0.0, 1.0, 0.0]), row("c:0", "math", "note", [0.7, 0.7, 0.0])])
            .await
            .unwrap();
        assert_eq!(v.count().await.unwrap(), 4);

        let hits = v.search(&[1.0, 0.0, 0.0], 10, &Filter::default()).await.unwrap();
        assert_eq!(ids(&hits), vec!["a:0", "a:1", "c:0", "b:0"]);
        assert!(hits[0].1.abs() < 1e-5, "identical direction has distance 0: {hits:?}");
        assert_eq!(ids(&v.search(&[1.0, 0.0, 0.0], 2, &Filter::default()).await.unwrap()), vec!["a:0", "a:1"]);
        assert_eq!(ids(&v.search(&[1.0, 0.0, 0.0], 10, &filter(&["chemistry"], &[])).await.unwrap()), vec!["b:0"]);
        assert_eq!(ids(&v.search(&[1.0, 0.0, 0.0], 10, &filter(&["math"], &["note"])).await.unwrap()), vec!["c:0"]);

        v.set_doc_meta("b", "science", "book").await.unwrap();
        assert_eq!(ids(&v.search(&[0.0, 1.0, 0.0], 10, &filter(&["science"], &["book"])).await.unwrap()), vec!["b:0"]);

        v.delete_docs(&["a".into()]).await.unwrap();
        v.delete_chunks(&["c:0".into()]).await.unwrap();
        assert_eq!(v.count().await.unwrap(), 1);
        v.maintain().await.unwrap();

        // A new handle on the same directory sees the table.
        let reopened = Vectors::open(&dir).await.unwrap();
        assert_eq!(reopened.count().await.unwrap(), 1);
        reopened.drop_all().await.unwrap();
        assert_eq!(reopened.count().await.unwrap(), 0);
        drop((v, reopened));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mismatched_dimensions_are_rejected() {
        let dir = temp_dir("vectors-dims");
        let v = Vectors::open(&dir).await.unwrap();
        let mut bad = row("a:1", "math", "book", [1.0, 0.0, 0.0]);
        bad.vector.push(1.0);
        assert!(v.add(&[row("a:0", "math", "book", [1.0, 0.0, 0.0]), bad]).await.is_err());
        drop(v);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

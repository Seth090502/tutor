//! BM25 over chunk text with tantivy's English stemming analyzer (`en_stem`: simple
//! tokenizer, tokens over 40 bytes dropped, lowercase, Porter/Snowball English stemmer).
//!
//! Queries are not parsed as tantivy query syntax: the query text goes through the same
//! analyzer and each distinct term becomes an optional (`SHOULD`) clause, which is plain BM25
//! over the query's terms. Arbitrary text such as `f(x) = a:b` therefore never fails to parse,
//! and term positions are not indexed (phrase queries are not needed), which keeps the index
//! smaller.

use crate::error::Result;
use std::collections::HashSet;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TantivyDocument, TextFieldIndexing, TextOptions, Value,
};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term, doc};

pub const ANALYZER: &str = "en_stem";

/// Indexing memory for the writer, shared by its threads (tantivy needs 15 MB per thread).
const WRITER_MEMORY_BYTES: usize = 256 << 20;

#[derive(Clone, Copy)]
struct Fields {
    chunk_id: Field,
    doc_id: Field,
    domain: Field,
    source_type: Field,
    text: Field,
}

fn schema() -> (Schema, Fields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default().set_tokenizer(ANALYZER).set_index_option(IndexRecordOption::WithFreqs),
    );
    let fields = Fields {
        chunk_id: builder.add_text_field("chunk_id", STRING | STORED),
        doc_id: builder.add_text_field("doc_id", STRING),
        domain: builder.add_text_field("domain", STRING),
        source_type: builder.add_text_field("source_type", STRING),
        text: builder.add_text_field("text", text_options),
    };
    (builder.build(), fields)
}

/// Restrict results to any of these values. An empty list means no restriction.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub domains: Vec<String>,
    pub source_types: Vec<String>,
}

pub struct Bm25 {
    index: Index,
    reader: IndexReader,
    fields: Fields,
}

impl Bm25 {
    pub fn open(dir: &Path) -> Result<Bm25> {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let (schema, fields) = schema();
        let index = Index::open_or_create(MmapDirectory::open(dir)?, schema)
            .map_err(|e| format!("cannot open the BM25 index in {}: {e}", dir.display()))?;
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Bm25 { index, reader, fields })
    }

    /// The single writer. tantivy's lock file makes a second writer on the same index fail,
    /// including one in another process.
    pub fn writer(&self) -> Result<Bm25Writer> {
        let writer: IndexWriter = self
            .index
            .writer(WRITER_MEMORY_BYTES)
            .map_err(|e| format!("cannot open the BM25 index for writing (is another ingest running?): {e}"))?;
        Ok(Bm25Writer { writer, fields: self.fields })
    }

    /// Make the last commit visible to searches.
    pub fn reload(&self) -> Result<()> {
        Ok(self.reader.reload()?)
    }

    /// Best `limit` chunks for `query`, as `(chunk_id, score)` in descending score order.
    pub fn search(&self, query: &str, limit: usize, filter: &Filter) -> Result<Vec<(String, f32)>> {
        let f = self.fields;
        let mut analyzer = self.index.tokenizer_for_field(f.text)?;
        let mut stream = analyzer.token_stream(query);
        let mut seen = HashSet::new();
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        while stream.advance() {
            let token = stream.token().text.clone();
            if seen.insert(token.clone()) {
                let term = Term::from_field_text(f.text, &token);
                clauses.push((Occur::Should, Box::new(TermQuery::new(term, IndexRecordOption::WithFreqs))));
            }
        }
        if clauses.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut must: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, Box::new(BooleanQuery::new(clauses)))];
        for (field, values) in [(f.domain, &filter.domains), (f.source_type, &filter.source_types)] {
            if !values.is_empty() {
                must.push((Occur::Must, Box::new(any_of(field, values))));
            }
        }
        let searcher = self.reader.searcher();
        let top = searcher.search(&BooleanQuery::new(must), &TopDocs::with_limit(limit).order_by_score())?;
        top.into_iter()
            .map(|(score, address)| {
                let stored: TantivyDocument = searcher.doc(address)?;
                let chunk_id = stored
                    .get_first(f.chunk_id)
                    .and_then(|v| v.as_str())
                    .ok_or("BM25 index entry without a chunk_id")?
                    .to_string();
                Ok((chunk_id, score))
            })
            .collect()
    }
}

fn any_of(field: Field, values: &[String]) -> BooleanQuery {
    BooleanQuery::new(
        values
            .iter()
            .map(|v| -> (Occur, Box<dyn Query>) {
                (Occur::Should, Box::new(TermQuery::new(Term::from_field_text(field, v), IndexRecordOption::Basic)))
            })
            .collect(),
    )
}

pub struct Bm25Writer {
    writer: IndexWriter,
    fields: Fields,
}

impl Bm25Writer {
    pub fn add(&self, chunk_id: &str, doc_id: &str, domain: &str, source_type: &str, text: &str) -> Result<()> {
        let f = self.fields;
        self.writer.add_document(doc!(
            f.chunk_id => chunk_id,
            f.doc_id => doc_id,
            f.domain => domain,
            f.source_type => source_type,
            f.text => text,
        ))?;
        Ok(())
    }

    /// Remove every chunk of `doc_id` (applied at the next commit).
    pub fn delete_doc(&self, doc_id: &str) {
        self.writer.delete_term(Term::from_field_text(self.fields.doc_id, doc_id));
    }

    pub fn delete_all(&self) -> Result<()> {
        self.writer.delete_all_documents()?;
        Ok(())
    }

    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        Ok(())
    }

    /// Wait for background segment merges, then release the writer lock.
    pub fn finish(self) -> Result<()> {
        Ok(self.writer.wait_merging_threads()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::temp_dir;

    fn filter(domains: &[&str], source_types: &[&str]) -> Filter {
        Filter {
            domains: domains.iter().map(|s| s.to_string()).collect(),
            source_types: source_types.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn ids(hits: &[(String, f32)]) -> Vec<&str> {
        hits.iter().map(|(id, _)| id.as_str()).collect()
    }

    #[test]
    fn stems_ranks_filters_and_deletes() {
        let dir = temp_dir("bm25");
        let bm25 = Bm25::open(&dir).unwrap();
        let mut w = bm25.writer().unwrap();
        w.add("a:0", "a", "chemistry", "book", "Oxidation reactions lose electrons; reduction gains them.").unwrap();
        w.add("a:1", "a", "chemistry", "book", "Acids donate protons in water.").unwrap();
        w.add("b:0", "b", "math", "paper", "A limit describes the value a function approaches.").unwrap();
        w.add("c:0", "c", "math", "note", "Reaction rates and limits of sequences.").unwrap();
        w.commit().unwrap();
        bm25.reload().unwrap();

        // Stemming: "electron" matches "electrons", "reaction" matches "reactions" and "Reaction".
        assert_eq!(ids(&bm25.search("electron reaction", 10, &Filter::default()).unwrap()), vec!["a:0", "c:0"]);
        // Both contain "limit" once; BM25 length normalisation ranks the shorter chunk first.
        assert_eq!(ids(&bm25.search("limits", 10, &filter(&["math"], &[])).unwrap()), vec!["c:0", "b:0"]);
        assert_eq!(ids(&bm25.search("limits", 10, &filter(&["math"], &["note"])).unwrap()), vec!["c:0"]);
        assert_eq!(ids(&bm25.search("limits", 10, &filter(&["chemistry"], &[])).unwrap()), Vec::<&str>::new());
        assert_eq!(bm25.search("limits", 1, &Filter::default()).unwrap().len(), 1);

        // Query syntax characters are just text; no terms means no hits.
        assert_eq!(ids(&bm25.search("f(x) = limit:value AND \"approaches", 10, &Filter::default()).unwrap())[0], "b:0");
        assert!(bm25.search("?? !! ==", 10, &Filter::default()).unwrap().is_empty());

        w.delete_doc("a");
        w.commit().unwrap();
        bm25.reload().unwrap();
        assert!(bm25.search("oxidation acids", 10, &Filter::default()).unwrap().is_empty());
        w.delete_all().unwrap();
        w.commit().unwrap();
        w.finish().unwrap();
        bm25.reload().unwrap();
        assert!(bm25.search("limits", 10, &Filter::default()).unwrap().is_empty());
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_writer_is_refused() {
        let dir = temp_dir("bm25-lock");
        let bm25 = Bm25::open(&dir).unwrap();
        let w = bm25.writer().unwrap();
        let second = bm25.writer();
        assert!(second.is_err());
        assert!(second.err().unwrap().0.contains("another ingest"));
        w.finish().unwrap();
        assert!(bm25.writer().is_ok());
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

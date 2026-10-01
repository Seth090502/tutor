//! BM25 over chunk text, book title and heading path, with the maths-aware English analyzer of
//! [`crate::tokenize`].
//!
//! Queries are not parsed as tantivy query syntax: the query text goes through the same
//! analyzer and each distinct term becomes optional (`SHOULD`) clauses on the text field and,
//! boosted, on the title and heading fields. The score is the sum of the per-field BM25 scores.
//! Arbitrary text such as `f(x) = a:b` therefore never fails to parse, and term positions are
//! not indexed (phrase queries are not needed), which keeps the index smaller.

use crate::error::{Error, Result};
use crate::tokenize::{self, ANALYZER};
use std::collections::HashSet;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::indexer::LogMergePolicy;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TantivyDocument, TextFieldIndexing,
    TextOptions, Value,
};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyError, Term, doc};

/// The version of the index's fields and analyzer. Bump it whenever either changes: an ingest
/// that finds another version in the database (`meta`) re-indexes BM25 from the stored chunks.
/// Version 1 (no `meta` entry) indexed chunk text only, with tantivy's `en_stem`.
pub const SCHEMA_VERSION: &str = "2";
/// The `meta` key holding the [`SCHEMA_VERSION`] the index was last built with.
pub const SCHEMA_KEY: &str = "bm25.schema_version";

/// Merge a segment once this share of its documents is deleted (tantivy's default, 1.0, never
/// merges for deletes alone). Re-indexed and removed documents then free their space.
const DEL_DOCS_RATIO_BEFORE_MERGE: f32 = 0.2;

/// Index tuning, from the environment (README: Index tuning).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Query-time weight of a term matching the book title (`RAG_BM25_TITLE_BOOST`).
    pub title_boost: f32,
    /// Query-time weight of a term matching the heading path (`RAG_BM25_HEADING_BOOST`).
    pub heading_boost: f32,
    /// Indexing memory for the writer, shared by its threads (`RAG_BM25_WRITER_HEAP_MB`).
    pub writer_heap_bytes: usize,
    /// Commit the index after this many added chunks (`RAG_BM25_COMMIT_CHUNKS`).
    pub commit_chunks: usize,
}

impl Default for Tuning {
    /// Boosts that rank a title or heading match above a body-only match (tested below), and
    /// the growth settings of docs/tutor/CORPUS.md (heap 1-2 GB, commits every 50-200k chunks).
    fn default() -> Tuning {
        Tuning {
            title_boost: 2.0,
            heading_boost: 2.0,
            writer_heap_bytes: 1 << 30,
            commit_chunks: 100_000,
        }
    }
}

const WRITER_HEAP_MB: std::ops::RangeInclusive<usize> = 128..=32_768;

impl Tuning {
    pub fn from_env() -> Result<Tuning> {
        Tuning::from_vars(|name| std::env::var(name).ok())
    }

    /// The defaults, overridden by the variables `var` returns. Empty values mean unset.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Tuning> {
        fn parse<T: std::str::FromStr>(
            var: &impl Fn(&str) -> Option<String>,
            name: &str,
        ) -> Result<Option<T>> {
            match var(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                None => Ok(None),
                Some(v) => v
                    .parse()
                    .map(Some)
                    .map_err(|_| Error(format!("{name} must be a number, got {v:?}"))),
            }
        }
        let mut tuning = Tuning::default();
        for (name, boost) in [
            ("RAG_BM25_TITLE_BOOST", &mut tuning.title_boost),
            ("RAG_BM25_HEADING_BOOST", &mut tuning.heading_boost),
        ] {
            if let Some(value) = parse::<f32>(&var, name)? {
                if !(value.is_finite() && value >= 0.0) {
                    return Err(format!("{name} must be 0 or more, got {value}").into());
                }
                *boost = value;
            }
        }
        if let Some(mb) = parse::<usize>(&var, "RAG_BM25_WRITER_HEAP_MB")? {
            if !WRITER_HEAP_MB.contains(&mb) {
                return Err(format!(
                    "RAG_BM25_WRITER_HEAP_MB must be between {} and {}, got {mb}",
                    WRITER_HEAP_MB.start(),
                    WRITER_HEAP_MB.end()
                )
                .into());
            }
            tuning.writer_heap_bytes = mb << 20;
        }
        if let Some(chunks) = parse::<usize>(&var, "RAG_BM25_COMMIT_CHUNKS")? {
            if chunks == 0 {
                return Err("RAG_BM25_COMMIT_CHUNKS must be at least 1".into());
            }
            tuning.commit_chunks = chunks;
        }
        Ok(tuning)
    }
}

#[derive(Clone, Copy)]
struct Fields {
    chunk_id: Field,
    doc_id: Field,
    domain: Field,
    source_type: Field,
    text: Field,
    title: Field,
    heading: Field,
}

fn schema() -> (Schema, Fields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(ANALYZER)
            .set_index_option(IndexRecordOption::WithFreqs),
    );
    let fields = Fields {
        chunk_id: builder.add_text_field("chunk_id", STRING | STORED),
        doc_id: builder.add_text_field("doc_id", STRING),
        domain: builder.add_text_field("domain", STRING),
        source_type: builder.add_text_field("source_type", STRING),
        text: builder.add_text_field("text", text_options.clone()),
        title: builder.add_text_field("title", text_options.clone()),
        heading: builder.add_text_field("heading", text_options),
    };
    (builder.build(), fields)
}

/// One chunk as indexed.
pub struct Entry<'a> {
    pub chunk_id: &'a str,
    pub doc_id: &'a str,
    pub domain: &'a str,
    pub source_type: &'a str,
    /// The document's title.
    pub title: &'a str,
    /// The chunk's heading path.
    pub heading_path: &'a [String],
    pub text: &'a str,
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
    tuning: Tuning,
    /// The index was created by this open (none existed, or it had an older schema), so it
    /// holds nothing yet.
    pub created: bool,
}

impl Bm25 {
    /// Open the index in `dir`, or create it. An index with other fields or another analyzer
    /// (an older [`SCHEMA_VERSION`]) is deleted and created empty; the next ingest re-indexes it.
    pub fn open(dir: &Path, tuning: Tuning) -> Result<Bm25> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let (schema, fields) = schema();
        let created = !Index::exists(&MmapDirectory::open(dir)?).map_err(|e| e.to_string())?;
        let (index, created) = match Index::open_or_create(
            MmapDirectory::open(dir)?,
            schema.clone(),
        ) {
            Ok(index) => (index, created),
            Err(TantivyError::SchemaError(_)) => {
                eprintln!(
                    "rag-engine: the BM25 index in {} was built with an older schema; it is cleared now and \
                     re-indexed from the database by the next ingest (until then BM25 search finds nothing)",
                    dir.display()
                );
                std::fs::remove_dir_all(dir).map_err(|e| {
                    format!("cannot clear the old BM25 index in {}: {e}", dir.display())
                })?;
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
                (Index::create_in_dir(dir, schema)?, true)
            }
            Err(e) => {
                return Err(format!("cannot open the BM25 index in {}: {e}", dir.display()).into());
            }
        };
        index.tokenizers().register(ANALYZER, tokenize::analyzer());
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        Ok(Bm25 {
            index,
            reader,
            fields,
            tuning,
            created,
        })
    }

    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    /// The single writer. tantivy's lock file makes a second writer on the same index fail,
    /// including one in another process.
    pub fn writer(&self) -> Result<Bm25Writer> {
        let writer: IndexWriter =
            self.index
                .writer(self.tuning.writer_heap_bytes)
                .map_err(|e| {
                    format!(
                        "cannot open the BM25 index for writing (is another ingest running?): {e}"
                    )
                })?;
        let mut merge_policy = LogMergePolicy::default();
        merge_policy.set_del_docs_ratio_before_merge(DEL_DOCS_RATIO_BEFORE_MERGE);
        writer.set_merge_policy(Box::new(merge_policy));
        Ok(Bm25Writer {
            writer,
            fields: self.fields,
        })
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
        let fields = [
            (f.text, 1.0),
            (f.title, self.tuning.title_boost),
            (f.heading, self.tuning.heading_boost),
        ];
        while stream.advance() {
            let token = stream.token().text.clone();
            if seen.insert(token.clone()) {
                for (field, boost) in fields.iter().filter(|(_, boost)| *boost > 0.0) {
                    let term = TermQuery::new(
                        Term::from_field_text(*field, &token),
                        IndexRecordOption::WithFreqs,
                    );
                    clauses.push((
                        Occur::Should,
                        Box::new(BoostQuery::new(Box::new(term), *boost)),
                    ));
                }
            }
        }
        if clauses.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut must: Vec<(Occur, Box<dyn Query>)> =
            vec![(Occur::Must, Box::new(BooleanQuery::new(clauses)))];
        for (field, values) in [
            (f.domain, &filter.domains),
            (f.source_type, &filter.source_types),
        ] {
            if !values.is_empty() {
                must.push((Occur::Must, Box::new(any_of(field, values))));
            }
        }
        let searcher = self.reader.searcher();
        let top = searcher.search(
            &BooleanQuery::new(must),
            &TopDocs::with_limit(limit).order_by_score(),
        )?;
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
                (
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_text(field, v),
                        IndexRecordOption::Basic,
                    )),
                )
            })
            .collect(),
    )
}

pub struct Bm25Writer {
    writer: IndexWriter,
    fields: Fields,
}

impl Bm25Writer {
    pub fn add(&self, e: &Entry) -> Result<()> {
        let f = self.fields;
        self.writer.add_document(doc!(
            f.chunk_id => e.chunk_id,
            f.doc_id => e.doc_id,
            f.domain => e.domain,
            f.source_type => e.source_type,
            f.text => e.text,
            f.title => e.title,
            f.heading => e.heading_path.join("\n"),
        ))?;
        Ok(())
    }

    /// Remove every chunk of `doc_id` (applied at the next commit).
    pub fn delete_doc(&self, doc_id: &str) {
        self.writer
            .delete_term(Term::from_field_text(self.fields.doc_id, doc_id));
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

    /// Add a chunk without title or headings.
    fn add(
        w: &Bm25Writer,
        chunk_id: &str,
        doc_id: &str,
        domain: &str,
        source_type: &str,
        text: &str,
    ) {
        w.add(&Entry {
            chunk_id,
            doc_id,
            domain,
            source_type,
            title: "",
            heading_path: &[],
            text,
        })
        .unwrap();
    }

    #[test]
    fn stems_ranks_filters_and_deletes() {
        let dir = temp_dir("bm25");
        let bm25 = Bm25::open(&dir, Tuning::default()).unwrap();
        let mut w = bm25.writer().unwrap();
        add(
            &w,
            "a:0",
            "a",
            "chemistry",
            "book",
            "Oxidation reactions lose electrons; reduction gains them.",
        );
        add(
            &w,
            "a:1",
            "a",
            "chemistry",
            "book",
            "Acids donate protons in water.",
        );
        add(
            &w,
            "b:0",
            "b",
            "math",
            "paper",
            "A limit describes the value a function approaches.",
        );
        add(
            &w,
            "c:0",
            "c",
            "math",
            "note",
            "Reaction rates and limits of sequences.",
        );
        w.commit().unwrap();
        bm25.reload().unwrap();

        // Stemming: "electron" matches "electrons", "reaction" matches "reactions" and "Reaction".
        assert_eq!(
            ids(&bm25
                .search("electron reaction", 10, &Filter::default())
                .unwrap()),
            vec!["a:0", "c:0"]
        );
        // Both contain "limit" once; BM25 length normalisation ranks the shorter chunk first.
        assert_eq!(
            ids(&bm25.search("limits", 10, &filter(&["math"], &[])).unwrap()),
            vec!["c:0", "b:0"]
        );
        assert_eq!(
            ids(&bm25
                .search("limits", 10, &filter(&["math"], &["note"]))
                .unwrap()),
            vec!["c:0"]
        );
        assert_eq!(
            ids(&bm25
                .search("limits", 10, &filter(&["chemistry"], &[]))
                .unwrap()),
            Vec::<&str>::new()
        );
        assert_eq!(
            bm25.search("limits", 1, &Filter::default()).unwrap().len(),
            1
        );

        // Query syntax characters are just text; no terms means no hits.
        assert_eq!(
            ids(&bm25
                .search(
                    "f(x) = limit:value AND \"approaches",
                    10,
                    &Filter::default()
                )
                .unwrap())[0],
            "b:0"
        );
        assert!(
            bm25.search("?? !! ==", 10, &Filter::default())
                .unwrap()
                .is_empty()
        );

        w.delete_doc("a");
        w.commit().unwrap();
        bm25.reload().unwrap();
        assert!(
            bm25.search("oxidation acids", 10, &Filter::default())
                .unwrap()
                .is_empty()
        );
        w.delete_all().unwrap();
        w.commit().unwrap();
        w.finish().unwrap();
        bm25.reload().unwrap();
        assert!(
            bm25.search("limits", 10, &Filter::default())
                .unwrap()
                .is_empty()
        );
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// "Entropy" in one chunk's title (t), in another's heading (h), and twice in a third's body
    /// (b). Titles, headings and bodies have even lengths, as in a real book.
    fn boost_fixture(name: &str, tuning: Tuning) -> (std::path::PathBuf, Bm25) {
        let dir = temp_dir(name);
        let bm25 = Bm25::open(&dir, tuning).unwrap();
        let mut w = bm25.writer().unwrap();
        let heading = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
        let entries = [
            (
                "t:0",
                "Entropy and Information",
                heading("Chapter 1", "Surprise"),
                "Bits measure how surprising a message is to the receiver who reads it carefully.",
            ),
            (
                "h:0",
                "Thermal Physics Today",
                heading("Chapter 2", "Entropy"),
                "Heat flows from hot bodies to cold ones until the two of them reach thermal equilibrium.",
            ),
            (
                "b:0",
                "Physics Notes Volume",
                heading("Chapter 3", "Gases"),
                "Entropy rises when a gas expands freely, and entropy counts the microstates of the system.",
            ),
            (
                "x:0",
                "Organic Chemistry Basics",
                heading("Chapter 4", "Alkenes"),
                "Double bonds add hydrogen over a metal catalyst at modest pressures and room temperature.",
            ),
        ];
        for (chunk_id, title, heading_path, text) in &entries {
            let doc_id = &chunk_id[..1];
            w.add(&Entry {
                chunk_id,
                doc_id,
                domain: "science",
                source_type: "book",
                title,
                heading_path,
                text,
            })
            .unwrap();
        }
        w.commit().unwrap();
        w.finish().unwrap();
        bm25.reload().unwrap();
        (dir, bm25)
    }

    #[test]
    fn title_and_heading_matches_rank_above_a_body_only_match() {
        let (dir, bm25) = boost_fixture("bm25-boost", Tuning::default());
        let hits = bm25.search("entropy", 10, &Filter::default()).unwrap();
        let order = ids(&hits);
        assert_eq!(order.len(), 3, "{hits:?}");
        assert_eq!(
            order[2], "b:0",
            "the default boosts rank title and heading matches first: {hits:?}"
        );
        // Title and heading words are searchable through the maths analyzer too.
        assert_eq!(
            ids(&bm25.search("thermal", 10, &Filter::default()).unwrap()),
            vec!["h:0"]
        );
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();

        // Unboosted, the body with the word twice ranks first; with the fields off, only the body
        // matches.
        let flat = Tuning {
            title_boost: 1.0,
            heading_boost: 1.0,
            ..Tuning::default()
        };
        let (dir, bm25) = boost_fixture("bm25-boost-flat", flat);
        assert_eq!(
            ids(&bm25.search("entropy", 10, &Filter::default()).unwrap())[0],
            "b:0"
        );
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
        let off = Tuning {
            title_boost: 0.0,
            heading_boost: 0.0,
            ..Tuning::default()
        };
        let (dir, bm25) = boost_fixture("bm25-boost-off", off);
        assert_eq!(
            ids(&bm25.search("entropy", 10, &Filter::default()).unwrap()),
            vec!["b:0"]
        );
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn maths_glyphs_and_latex_queries_match() {
        let dir = temp_dir("bm25-maths");
        let bm25 = Bm25::open(&dir, Tuning::default()).unwrap();
        let mut w = bm25.writer().unwrap();
        add(
            &w,
            "m:0",
            "m",
            "math",
            "book",
            "∑ 1/n² = π²/6 and ∂u/∂t = k ∇²u for 𝑥 ∈ ℝ",
        );
        add(
            &w,
            "p:0",
            "p",
            "math",
            "book",
            "The sum of the parts is a whole.",
        );
        w.commit().unwrap();
        w.finish().unwrap();
        bm25.reload().unwrap();
        assert_eq!(
            ids(&bm25.search(r"\pi", 10, &Filter::default()).unwrap()),
            vec!["m:0"]
        );
        assert_eq!(
            ids(&bm25
                .search("partial nabla", 10, &Filter::default())
                .unwrap()),
            vec!["m:0"]
        );
        assert_eq!(
            ids(&bm25.search("π ∂", 10, &Filter::default()).unwrap()),
            vec!["m:0"]
        );
        assert_eq!(
            ids(&bm25
                .search(r"x \in \mathbb{R}", 10, &Filter::default())
                .unwrap())[0],
            "m:0"
        );
        assert_eq!(
            ids(&bm25.search(r"\sum", 10, &Filter::default()).unwrap()).len(),
            2,
            "∑ and the word sum"
        );
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tuning_comes_from_the_environment() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(Tuning::from_vars(vars(&[])).unwrap(), Tuning::default());
        assert_eq!(
            Tuning::from_vars(vars(&[("RAG_BM25_TITLE_BOOST", " ")])).unwrap(),
            Tuning::default()
        );
        let t = Tuning::from_vars(vars(&[
            ("RAG_BM25_TITLE_BOOST", "1.5"),
            ("RAG_BM25_HEADING_BOOST", "0"),
            ("RAG_BM25_WRITER_HEAP_MB", "2048"),
            ("RAG_BM25_COMMIT_CHUNKS", "50000"),
        ]))
        .unwrap();
        assert_eq!(
            t,
            Tuning {
                title_boost: 1.5,
                heading_boost: 0.0,
                writer_heap_bytes: 2048 << 20,
                commit_chunks: 50_000
            }
        );
        for bad in [
            &[("RAG_BM25_TITLE_BOOST", "-1")][..],
            &[("RAG_BM25_HEADING_BOOST", "NaN")],
            &[("RAG_BM25_HEADING_BOOST", "high")],
            &[("RAG_BM25_WRITER_HEAP_MB", "64")],
            &[("RAG_BM25_COMMIT_CHUNKS", "0")],
        ] {
            assert!(Tuning::from_vars(vars(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_writer_merges_segments_with_a_fifth_deleted() {
        let dir = temp_dir("bm25-merge");
        let bm25 = Bm25::open(&dir, Tuning::default()).unwrap();
        let w = bm25.writer().unwrap();
        let policy = format!("{:?}", w.writer.get_merge_policy());
        assert!(
            policy.contains("del_docs_ratio_before_merge: 0.2"),
            "{policy}"
        );
        w.finish().unwrap();
        drop(bm25);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_index_with_an_older_schema_is_recreated() {
        let dir = temp_dir("bm25-old-schema");
        // Version 1: chunk text only, with tantivy's en_stem analyzer.
        let mut builder = Schema::builder();
        builder.add_text_field("chunk_id", STRING | STORED);
        builder.add_text_field("text", tantivy::schema::TEXT);
        let old = Index::create_in_dir(&dir, builder.build()).unwrap();
        let mut w: IndexWriter = old.writer_with_num_threads(1, 50 << 20).unwrap();
        w.add_document(doc!(old.schema().get_field("chunk_id").unwrap() => "a:0"))
            .unwrap();
        w.commit().unwrap();
        drop(w);
        drop(old);

        let bm25 = Bm25::open(&dir, Tuning::default()).unwrap();
        assert!(bm25.created, "the old index is replaced by an empty one");
        assert_eq!(bm25.reader.searcher().num_docs(), 0);
        let mut w = bm25.writer().unwrap();
        add(&w, "b:0", "b", "math", "book", "Limits of sequences.");
        w.commit().unwrap();
        w.finish().unwrap();
        drop(bm25);
        let reopened = Bm25::open(&dir, Tuning::default()).unwrap();
        assert!(!reopened.created, "a current index is kept");
        assert_eq!(
            ids(&reopened.search("limit", 10, &Filter::default()).unwrap()),
            vec!["b:0"]
        );
        drop(reopened);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_second_writer_is_refused() {
        let dir = temp_dir("bm25-lock");
        let bm25 = Bm25::open(&dir, Tuning::default()).unwrap();
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

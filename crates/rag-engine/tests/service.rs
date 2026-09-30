//! Runs `rag-engine serve` on a free port against `tests/library` (a corpus-shaped tree built
//! from the survey fixtures) and exercises contract C1 over HTTP. Needs PDFium in
//! vendor/pdfium/ or PDFIUM_DYNAMIC_LIB_PATH (see README).
//!
//! Library contents and what they test:
//! - corpus/math/books/limits.md: Markdown headings, several chunks
//! - corpus/math/books/algebra.pdf: one math-heavy page (OCR queue)
//! - corpus/math/papers/series.txt: plain text paper
//! - corpus/science/books/biology.pdf and corpus/science/papers/biology-copy.pdf: identical
//!   copies (one document; the books path is canonical), 2 text pages and 1 empty page
//! - corpus/english/books/plants.epub: EPUB with a table of contents
//! - corpus/chemistry/papers/locked.pdf: encrypted, recorded as an error
//! - corpus/inbox/scan.pdf: one scanned page without text (OCR queue), unsorted
//! - notes/chemistry/redox.md: a note

use axum::Json;
use axum::extract::State;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Variables the engine reads. They are removed from the child's environment so a test can
/// never reach a real (paid) provider, then set explicitly where a test needs them.
const ENGINE_VARS: [&str; 8] = [
    "RAG_EMBED_PROVIDER",
    "RAG_EMBED_MODEL",
    "RAG_EMBED_API_KEY",
    "VOYAGE_API_KEY",
    "RAG_EMBED_BASE_URL",
    "RAG_EMBED_PRICE_PER_MTOK",
    "RAG_OCR_PROVIDER",
    "RAG_ENGINE_TOKEN",
];

const ALLOWED: [&str; 4] = ["http://localhost:1420", "http://127.0.0.1:1420", "http://tauri.localhost", "tauri://localhost"];

fn library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/library")
}

struct Server {
    child: Child,
    base: String,
    log: Arc<Mutex<String>>,
    data: PathBuf,
    /// Sent as the bearer token by `call` when the server was started with one.
    token: Option<String>,
}

/// `rag-engine serve` on the test library with a fresh data directory and a free port.
fn serve_command(data: &Path, env: &[(&str, String)]) -> Command {
    let lib = library();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rag-engine"));
    cmd.arg("serve")
        .arg("--corpus")
        .arg(lib.join("corpus"))
        .arg("--notes")
        .arg(lib.join("notes"))
        .arg("--data")
        .arg(data)
        .args(["--port", "0"]);
    for var in ENGINE_VARS {
        cmd.env_remove(var);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd
}

impl Server {
    fn start(name: &str, env: &[(&str, String)]) -> Server {
        let data = std::env::temp_dir().join(format!("rag-engine-service-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data);
        let mut cmd = serve_command(&data, env);
        let token = env.iter().find(|(key, _)| *key == "RAG_ENGINE_TOKEN").map(|(_, value)| value.clone());
        let mut child = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
        let stderr = child.stderr.take().unwrap();
        let log = Arc::new(Mutex::new(String::new()));
        let (tx, rx) = std::sync::mpsc::channel();
        let sink = log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                if let Some(address) = line.strip_prefix("rag-engine: listening on ") {
                    let _ = tx.send(address.to_string());
                }
                let mut log = sink.lock().unwrap();
                log.push_str(&line);
                log.push('\n');
            }
        });
        let base = rx
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("the server did not start:\n{}", log.lock().unwrap()));
        assert!(base.starts_with("http://127.0.0.1:"), "{base}");
        Server { child, base, log, data, token }
    }

    fn log(&self) -> String {
        self.log.lock().unwrap().clone()
    }

    async fn call(&self, method: Method, path: &str, body: Option<&Value>) -> (StatusCode, Value) {
        let mut request = reqwest::Client::new().request(method, format!("{}{path}", self.base));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let text = response.text().await.unwrap();
        let value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: not JSON ({e}): {text:?}"));
        (status, value)
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(&body)).await
    }

    async fn search(&self, body: Value) -> Value {
        let (status, value) = self.post("/search", body.clone()).await;
        assert_eq!(status, StatusCode::OK, "{body} -> {value}");
        value
    }

    async fn start_job(&self, path: &str, body: Value) -> String {
        let (status, value) = self.post(path, body).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{value}");
        value["job_id"].as_str().unwrap().to_string()
    }

    async fn wait_job(&self, job_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            let (status, job) = self.get(&format!("/jobs/{job_id}")).await;
            assert_eq!(status, StatusCode::OK, "{job}");
            if job["state"] == "done" || job["state"] == "failed" {
                return job;
            }
            assert!(Instant::now() < deadline, "job {job_id} did not finish: {job}\n{}", self.log());
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.data);
    }
}

/// Every file under `dir` with its size and modification time.
fn snapshot(dir: &Path) -> Vec<(PathBuf, u64, SystemTime)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                out.push((entry.path(), meta.len(), meta.modified().unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// About 30 words from the middle of `text`: a query "built from the chunk's own text".
fn query_from(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let start = words.len().saturating_sub(30) / 2;
    words[start..(start + 30).min(words.len())].join(" ")
}

fn hits(response: &Value) -> &Vec<Value> {
    response["hits"].as_array().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn service_contract_with_bm25_only() {
    let before = snapshot(&library());
    let server = Server::start("bm25", &[]);

    let (status, health) = server.get("/health").await;
    assert_eq!((status, health), (StatusCode::OK, json!({"ok": true, "version": env!("CARGO_PKG_VERSION")})));

    let (status, empty) = server.get("/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["documents"]["total"], 0);
    assert_eq!(empty["documents"]["by_source_type"], json!({"book": 0, "paper": 0, "note": 0, "unknown": 0}));
    assert_eq!(empty["active_job"], Value::Null);
    assert_eq!(empty["embedding"], json!({"provider": null, "model": null, "dims": null}));

    // Survey job and the latest report.
    let (status, missing) = server.get("/survey/latest").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(missing["error"].is_string());
    let survey = server.wait_job(&server.start_job("/survey", json!({})).await).await;
    assert_eq!(survey["state"], "done", "{survey}");
    assert_eq!(survey["kind"], "survey");
    let (status, report) = server.get("/survey/latest").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (&report["totals"]["files"], &report["totals"]["unique_files"], &report["totals"]["errors"]),
        (&json!(8), &json!(7), &json!(1))
    );

    // Ingest job lifecycle.
    let job = server.wait_job(&server.start_job("/ingest", json!({"rebuild": false})).await).await;
    assert_eq!(job["state"], "done", "{job}\n{}", server.log());
    assert_eq!(job["kind"], "ingest");
    assert_eq!(job["error"], Value::Null);
    assert!(job["progress"]["total"].as_u64().unwrap() > 0);
    assert_eq!(job["progress"]["done"], job["progress"]["total"]);
    assert!(job["started_at"].as_f64().unwrap() <= job["finished_at"].as_f64().unwrap());

    let (_, stats) = server.get("/status").await;
    assert!(stats["corpus_root"].as_str().unwrap().ends_with("tests/library/corpus"), "{stats}");
    assert_eq!(
        stats["documents"],
        json!({
            "total": 7,
            "by_domain": {"chemistry": 1, "english": 1, "math": 3, "science": 1, "unsorted": 1},
            "by_source_type": {"book": 4, "paper": 1, "note": 1, "unknown": 1}
        })
    );
    assert_eq!(
        stats["pages"],
        json!({"total": 5, "by_class": {"born_digital": 2, "scanned": 1, "math_heavy": 1, "empty": 1}})
    );
    assert_eq!(stats["ocr"], json!({"pending_pages": 2, "done_pages": 0}));
    assert_eq!(stats["errors"], 1);
    assert_eq!(stats["chunks"]["embedded"], 0);
    let chunk_total = stats["chunks"]["total"].as_u64().unwrap();
    assert!(chunk_total >= 6, "{stats}");
    assert_eq!(stats["spend_usd"], json!({"embedding": 0.0, "ocr": 0.0}));
    assert_eq!(stats["active_job"], Value::Null);

    // Library browsing (C9): indexed documents only, title order by default.
    let (status, listing) = server.get("/documents").await;
    assert_eq!(status, StatusCode::OK, "{listing}");
    assert_eq!(listing["total"], 7, "{listing}");
    let docs = listing["documents"].as_array().unwrap();
    let titles: Vec<String> = docs.iter().map(|d| d["title"].as_str().unwrap().to_lowercase()).collect();
    assert!(titles.windows(2).all(|w| w[0] <= w[1]), "{titles:?}");
    assert!(docs.iter().all(|d| d["path"] != "corpus/chemistry/papers/locked.pdf"), "error documents are not listed");
    let by_path = |path: &str| docs.iter().find(|d| d["path"] == path).unwrap_or_else(|| panic!("{path} missing: {listing}")).clone();
    let limits_doc = by_path("corpus/math/books/limits.md");
    let mut keys: Vec<&str> = limits_doc.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["added_at", "bytes", "chunks", "doc_id", "domain", "ocr_pending", "pages", "path", "source_type", "title"]);
    assert_eq!(
        (&limits_doc["title"], &limits_doc["domain"], &limits_doc["source_type"], &limits_doc["pages"], &limits_doc["bytes"]),
        (&json!("Chapter 3 Limits"), &json!("math"), &json!("book"), &json!(0), &json!(fs::metadata(library().join("corpus/math/books/limits.md")).unwrap().len()))
    );
    assert!(limits_doc["added_at"].as_f64().unwrap() > 1.7e9);
    assert_eq!((&by_path("corpus/inbox/scan.pdf")["ocr_pending"], &by_path("corpus/math/books/algebra.pdf")["ocr_pending"]), (&json!(1), &json!(1)));
    assert_eq!(by_path("corpus/science/books/biology.pdf")["pages"], 3);
    let (_, math) = server.get("/documents?domain=math&sort=pages&limit=2&offset=0").await;
    assert_eq!(math["total"], 3, "{math}");
    assert_eq!(math["documents"].as_array().unwrap().len(), 2);
    assert_eq!(math["documents"][0]["path"], "corpus/math/books/algebra.pdf", "most pages first: {math}");
    let (_, found) = server.get("/documents?q=BIOLOGY&domain=&source_type=").await;
    assert_eq!((&found["total"], &found["documents"][0]["path"]), (&json!(1), &json!("corpus/science/books/biology.pdf")));
    let (_, notes) = server.get("/documents?source_type=note").await;
    assert_eq!((&notes["total"], &notes["documents"][0]["path"]), (&json!(1), &json!("notes/chemistry/redox.md")));
    for bad in ["/documents?limit=201", "/documents?limit=0", "/documents?sort=size", "/documents?source_type=books", "/documents?page=2", "/documents?offset=-1"] {
        let (status, body) = server.get(bad).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad} -> {body}");
        assert!(body["error"].is_string(), "{bad} -> {body}");
    }

    // One document with its outline, then its chunks page by page in reading order.
    let limits_id = limits_doc["doc_id"].as_str().unwrap();
    let (status, detail) = server.get(&format!("/documents/{limits_id}")).await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["title"], "Chapter 3 Limits");
    let outline = detail["outline"].as_array().unwrap();
    assert!(!outline.is_empty(), "{detail}");
    assert_eq!(outline[0]["heading_path"][0], "Chapter 3 Limits", "{detail}");
    assert!(outline.iter().all(|e| e["chunk_id"].as_str().unwrap().starts_with(&format!("{limits_id}:")) && e["page_start"].is_null()));
    let paths: Vec<&Value> = outline.iter().map(|e| &e["heading_path"]).collect();
    assert!(paths.iter().enumerate().all(|(i, p)| !paths[..i].contains(p)), "distinct heading paths: {detail}");
    let mut ordinals = Vec::new();
    let mut texts = Vec::new();
    let mut from = Some(0);
    while let Some(start) = from {
        let (status, page) = server.get(&format!("/documents/{limits_id}/chunks?from={start}&limit=1")).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        for c in page["chunks"].as_array().unwrap() {
            ordinals.push(c["ordinal"].as_u64().unwrap());
            texts.push(c["text"].as_str().unwrap().to_string());
            assert_eq!(c["chunk_id"], format!("{limits_id}:{}", c["ordinal"]));
            assert!(c["heading_path"].is_array() && c["page_start"].is_null() && c["page_end"].is_null());
        }
        from = page["next"].as_u64();
    }
    assert_eq!(ordinals, (0..limits_doc["chunks"].as_u64().unwrap()).collect::<Vec<_>>());
    assert!(ordinals.len() >= 2, "limits.md has several chunks");
    let (_, all) = server.get(&format!("/documents/{limits_id}/chunks")).await;
    assert_eq!(all["next"], Value::Null);
    let all_texts: Vec<&str> = all["chunks"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap()).collect();
    assert_eq!(all_texts, texts);
    let (_, biology) = server.get(&format!("/documents/{}", by_path("corpus/science/books/biology.pdf")["doc_id"].as_str().unwrap())).await;
    assert_eq!(biology["pages"], 3);
    for missing in ["/documents/nope", "/documents/nope/chunks"] {
        let (status, body) = server.get(missing).await;
        assert_eq!((status, body["error"].is_string()), (StatusCode::NOT_FOUND, true), "{missing}");
    }
    let (status, _) = server.get(&format!("/documents/{limits_id}/chunks?limit=500")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Every chunk of the distinct-text documents ranks first for a query from its own text.
    let math = server.search(json!({"query": "limit limits function series", "domains": ["math"], "mode": "bm25", "k": 100})).await;
    let own: Vec<&Value> = hits(&math)
        .iter()
        .filter(|h| h["path"] == "corpus/math/books/limits.md" || h["path"] == "corpus/math/papers/series.txt")
        .collect();
    assert!(own.iter().filter(|h| h["path"] == "corpus/math/books/limits.md").count() >= 2, "{math}");
    for hit in &own {
        let response = server.search(json!({"query": query_from(hit["text"].as_str().unwrap()), "k": 5, "mode": "bm25"})).await;
        assert_eq!(hits(&response)[0]["chunk_id"], hit["chunk_id"], "query from {hit}\n-> {response}");
        assert_eq!(hits(&response)[0]["ranks"], json!({"bm25": 1, "vector": null}));
    }

    // Hit fields: ids, metadata, pages and heading paths.
    for hit in hits(&math) {
        assert_eq!(hit["domain"], "math");
        let chunk_id = hit["chunk_id"].as_str().unwrap();
        assert!(chunk_id.starts_with(&format!("{}:", hit["doc_id"].as_str().unwrap())), "{hit}");
        assert_eq!(hit["doc_id"].as_str().unwrap().len(), 64, "doc_id is a BLAKE3 hex digest");
    }
    let limits: Vec<&Value> = own.iter().copied().filter(|h| h["path"] == "corpus/math/books/limits.md").collect();
    assert!(limits.iter().all(|h| h["heading_path"][0] == "Chapter 3 Limits" && h["page_start"].is_null()), "{limits:?}");
    assert!(limits.iter().all(|h| h["source_type"] == "book" && h["title"] == "Chapter 3 Limits"));
    // A chunk's heading path is the one in force where the chunk starts.
    let infinity = server.search(json!({"query": "horizontal asymptotes numerator denominator degrees", "k": 1, "mode": "bm25"})).await;
    assert_eq!(hits(&infinity)[0]["heading_path"], json!(["Chapter 3 Limits", "3.3 Limits at Infinity"]), "{infinity}");

    let biology = server.search(json!({"query": "mitochondrion organelle chemical energy", "mode": "bm25", "k": 10})).await;
    let first = &hits(&biology)[0];
    assert_eq!(first["path"], "corpus/science/books/biology.pdf", "the books copy is canonical: {biology}");
    assert_eq!((&first["domain"], &first["source_type"]), (&json!("science"), &json!("book")));
    assert!(hits(&biology).iter().all(|h| h["path"] != "corpus/science/papers/biology-copy.pdf"));
    let (start, end) = (first["page_start"].as_u64().unwrap(), first["page_end"].as_u64().unwrap());
    assert!(1 <= start && start <= end && end <= 2, "{first}");

    let epub = server.search(json!({"query": "plants algae capture light", "k": 1})).await;
    assert_eq!(epub["mode_used"], "bm25", "hybrid falls back to BM25 without embeddings");
    let hit = &hits(&epub)[0];
    assert_eq!(
        (&hit["path"], &hit["title"], &hit["heading_path"], &hit["page_start"], &hit["domain"]),
        (&json!("corpus/english/books/plants.epub"), &json!("Survey Fixture"), &json!(["Chapter 1"]), &Value::Null, &json!("english"))
    );

    // A math-heavy page waits for OCR, but its text layer is already searchable.
    let algebra = server.search(json!({"query": "dx", "domains": ["math"], "mode": "bm25"})).await;
    let hit = &hits(&algebra)[0];
    assert_eq!((&hit["path"], &hit["page_start"], &hit["page_end"]), (&json!("corpus/math/books/algebra.pdf"), &json!(1), &json!(1)));

    // Filters.
    let notes = server.search(json!({"query": "oxidation electrons reduction", "source_types": ["note"]})).await;
    assert_eq!(hits(&notes).len(), 1);
    assert_eq!(
        (&hits(&notes)[0]["path"], &hits(&notes)[0]["domain"], &hits(&notes)[0]["source_type"]),
        (&json!("notes/chemistry/redox.md"), &json!("chemistry"), &json!("note"))
    );
    let papers = server.search(json!({"query": "series limits energy", "source_types": ["paper"], "k": 50})).await;
    assert!(!hits(&papers).is_empty() && hits(&papers).iter().all(|h| h["source_type"] == "paper"), "{papers}");
    let science = server.search(json!({"query": "limits energy series", "domains": ["science", "english"], "k": 50})).await;
    assert!(!hits(&science).is_empty());
    assert!(hits(&science).iter().all(|h| h["domain"] == "science" || h["domain"] == "english"), "{science}");
    let none = server.search(json!({"query": "limits", "domains": ["math"], "source_types": ["note"]})).await;
    assert!(hits(&none).is_empty());
    let k2 = server.search(json!({"query": "limit", "k": 2})).await;
    assert_eq!(hits(&k2).len(), 2);

    // Errors are JSON with 4xx statuses.
    let (status, body) = server.post("/search", json!({"query": "limits", "mode": "vector"})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body["error"].as_str().unwrap().contains("vector search is unavailable"));
    for bad in [json!({"query": ""}), json!({"query": "x", "k": 0}), json!({"query": "x", "source_types": ["books"]}), json!({"q": "x"})] {
        let (status, body) = server.post("/search", bad.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad} -> {body}");
        assert!(body["error"].is_string());
    }
    let response = reqwest::Client::new().post(format!("{}/search", server.base)).body("{not json").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.json::<Value>().await.unwrap()["error"].is_string());
    let (status, body) = server.get("/jobs/does-not-exist").await;
    assert_eq!((status, body["error"].is_string()), (StatusCode::NOT_FOUND, true));
    let (status, body) = server.get("/nowhere").await;
    assert_eq!((status, body["error"].is_string()), (StatusCode::NOT_FOUND, true));
    let (status, body) = server.post("/ingest", json!({"root": std::env::temp_dir()})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body["error"].as_str().unwrap().contains("not inside"));
    let (status, _) = server.post("/survey", json!({"root": "Z:/definitely/missing"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A second ingest changes nothing; a scoped one is accepted.
    let again = server.wait_job(&server.start_job("/ingest", json!({})).await).await;
    assert_eq!(again["state"], "done");
    let scoped = server.wait_job(&server.start_job("/ingest", json!({"root": library().join("corpus/math")})).await).await;
    assert_eq!(scoped["state"], "done", "{scoped}");
    let (_, after) = server.get("/status").await;
    assert_eq!((&after["documents"], &after["chunks"]["total"]), (&stats["documents"], &json!(chunk_total)));
    // Finished jobs stay readable.
    let (status, old) = server.get(&format!("/jobs/{}", job["job_id"].as_str().unwrap())).await;
    assert_eq!((status, &old), (StatusCode::OK, &job));

    // CORS: exactly the four app origins.
    let client = reqwest::Client::new();
    for origin in ALLOWED {
        let preflight = client
            .request(Method::OPTIONS, format!("{}/search", server.base))
            .header("Origin", origin)
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type")
            .send()
            .await
            .unwrap();
        let headers = preflight.headers();
        assert_eq!(headers.get("access-control-allow-origin").map(|v| v.to_str().unwrap()), Some(origin));
        assert!(headers.get("access-control-allow-headers").unwrap().to_str().unwrap().contains("content-type"));
        let get = client.get(format!("{}/health", server.base)).header("Origin", origin).send().await.unwrap();
        assert_eq!(get.headers().get("access-control-allow-origin").map(|v| v.to_str().unwrap()), Some(origin));
    }
    for origin in ["http://evil.example", "http://localhost:3000", "https://localhost:1420", "null"] {
        let preflight = client
            .request(Method::OPTIONS, format!("{}/search", server.base))
            .header("Origin", origin)
            .header("Access-Control-Request-Method", "POST")
            .send()
            .await
            .unwrap();
        assert!(preflight.headers().get("access-control-allow-origin").is_none(), "{origin} was allowed");
        let get = client.get(format!("{}/health", server.base)).header("Origin", origin).send().await.unwrap();
        assert!(get.headers().get("access-control-allow-origin").is_none(), "{origin} was allowed");
    }

    // The corpus and notes were only read.
    assert_eq!(snapshot(&library()), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn bearer_token_guards_every_route_but_health() {
    const TOKEN: &str = "test-token-3f9a7c1e5b";
    let server = Server::start("auth", &[("RAG_ENGINE_TOKEN", TOKEN.into())]);
    let client = reqwest::Client::new();
    let url = |path: &str| format!("{}{path}", server.base);

    let health = client.get(url("/health")).send().await.unwrap();
    assert_eq!(health.status(), StatusCode::OK, "health needs no token");
    for (method, path) in [
        (Method::GET, "/status"),
        (Method::GET, "/documents"),
        (Method::GET, "/documents/x"),
        (Method::GET, "/documents/x/chunks"),
        (Method::GET, "/jobs/x"),
        (Method::GET, "/survey/latest"),
        (Method::POST, "/search"),
        (Method::POST, "/ingest"),
        (Method::POST, "/survey"),
        (Method::GET, "/nowhere"),
        (Method::POST, "/health"),
    ] {
        for auth in [None, Some("Bearer wrong-token"), Some(&*format!("Basic {TOKEN}")), Some(TOKEN), Some("Bearer")] {
            let mut request = client.request(method.clone(), url(path));
            if let Some(auth) = auth {
                request = request.header("Authorization", auth);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{method} {path} with {auth:?}");
            assert!(response.json::<Value>().await.unwrap()["error"].is_string());
        }
    }
    let (status, body) = server.get("/status").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = server.get("/documents").await;
    assert_eq!((status, &body), (StatusCode::OK, &json!({"total": 0, "documents": []})));
    let lower = client.get(url("/status")).header("Authorization", format!("bearer {TOKEN}")).send().await.unwrap();
    assert_eq!(lower.status(), StatusCode::OK, "the scheme is case-insensitive");
    let (status, body) = server.post("/search", json!({"query": "limits"})).await;
    assert_eq!((status, &body["hits"]), (StatusCode::OK, &json!([])));

    // Browsers preflight without the token; the Authorization header is allowed, and a 401 still
    // carries CORS headers so the app can read it.
    let preflight = client
        .request(Method::OPTIONS, url("/documents"))
        .header("Origin", ALLOWED[0])
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "authorization")
        .send()
        .await
        .unwrap();
    assert!(preflight.status().is_success(), "{}", preflight.status());
    assert!(preflight.headers()["access-control-allow-headers"].to_str().unwrap().contains("authorization"));
    let refused = client.get(url("/status")).header("Origin", ALLOWED[0]).send().await.unwrap();
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(refused.headers()["access-control-allow-origin"], ALLOWED[0]);

    assert!(!server.log().contains(TOKEN), "the token was logged:\n{}", server.log());
}

#[test]
fn a_non_loopback_bind_needs_a_token() {
    let data = std::env::temp_dir().join(format!("rag-engine-service-bind-{}", std::process::id()));
    for bind in ["0.0.0.0", "192.168.1.20", "::"] {
        let out = serve_command(&data, &[("RAG_ENGINE_TOKEN", "  ".into())]).args(["--bind", bind]).output().unwrap();
        assert!(!out.status.success(), "{bind}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(&format!("refusing to listen on {bind} without RAG_ENGINE_TOKEN")), "{stderr}");
    }
    assert!(!data.exists(), "nothing is created before the bind is checked");
}

const DIMS: usize = 64;

/// Bag-of-words vectors: each lowercase word adds 1 to a hashed dimension; L2-normalised.
fn mock_vector(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; DIMS];
    for word in text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let hash = word.to_lowercase().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
        v[(hash % DIMS as u64) as usize] += 1.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(f32::MIN_POSITIVE);
    v.iter().map(|x| x / norm).collect()
}

/// An OpenAI-compatible embeddings endpoint that holds every request until `gate` opens.
#[derive(Clone)]
struct FakeEmbeddings {
    gate: tokio::sync::watch::Receiver<bool>,
    requests: Arc<AtomicUsize>,
    tokens: Arc<AtomicU64>,
    models: Arc<Mutex<Vec<String>>>,
}

async fn fake_embeddings(State(fake): State<FakeEmbeddings>, Json(body): Json<Value>) -> Json<Value> {
    fake.requests.fetch_add(1, Ordering::SeqCst);
    fake.gate.clone().wait_for(|open| *open).await.unwrap();
    fake.models.lock().unwrap().push(body["model"].as_str().unwrap().to_string());
    let inputs: Vec<String> = serde_json::from_value(body["input"].clone()).unwrap();
    let tokens: u64 = inputs.iter().map(|t| t.split_whitespace().count() as u64).sum();
    fake.tokens.fetch_add(tokens, Ordering::SeqCst);
    let data: Vec<Value> =
        inputs.iter().enumerate().map(|(i, t)| json!({"object": "embedding", "index": i, "embedding": mock_vector(t)})).collect();
    Json(json!({"object": "list", "data": data, "usage": {"prompt_tokens": tokens, "total_tokens": tokens}}))
}

#[tokio::test(flavor = "multi_thread")]
async fn hybrid_search_with_mock_embeddings_and_one_job_at_a_time() {
    let (open_gate, gate) = tokio::sync::watch::channel(false);
    let fake = FakeEmbeddings { gate, requests: Default::default(), tokens: Default::default(), models: Default::default() };
    let app = axum::Router::new().route("/v1/embeddings", axum::routing::post(fake_embeddings)).with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fake_url = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let server = Server::start(
        "hybrid",
        &[
            ("RAG_EMBED_PROVIDER", "openai_compat".into()),
            ("RAG_EMBED_MODEL", "mock-embed".into()),
            ("RAG_EMBED_BASE_URL", fake_url),
            ("RAG_EMBED_PRICE_PER_MTOK", "0.5".into()),
        ],
    );
    let (_, status) = server.get("/status").await;
    assert_eq!(status["embedding"], json!({"provider": "openai_compat", "model": "mock-embed", "dims": null}));

    // The ingest blocks in its first embedding request, so it is certainly still running.
    let job_id = server.start_job("/ingest", json!({})).await;
    let deadline = Instant::now() + Duration::from_secs(120);
    while fake.requests.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "no embedding request arrived:\n{}", server.log());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let (_, running) = server.get(&format!("/jobs/{job_id}")).await;
    assert_eq!(running["state"], "running", "{running}");
    assert!(running["started_at"].is_f64() && running["finished_at"].is_null());
    assert!(running["progress"]["done"].as_u64() < running["progress"]["total"].as_u64(), "{running}");
    for (path, body) in [("/ingest", json!({})), ("/survey", json!({}))] {
        let (status, body) = server.post(path, body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{path}: {body}");
        assert!(body["error"].as_str().unwrap().contains(&job_id), "{body}");
    }
    let (_, status) = server.get("/status").await;
    assert_eq!((&status["active_job"]["job_id"], &status["active_job"]["state"]), (&json!(job_id), &json!("running")));

    open_gate.send(true).unwrap();
    let done = server.wait_job(&job_id).await;
    assert_eq!(done["state"], "done", "{done}\n{}", server.log());

    let (_, status) = server.get("/status").await;
    let total = status["chunks"]["total"].as_u64().unwrap();
    assert!(total > 0);
    assert_eq!(status["chunks"]["embedded"], total, "{status}");
    assert_eq!(status["embedding"], json!({"provider": "openai_compat", "model": "mock-embed", "dims": DIMS}));
    let billed = fake.tokens.load(Ordering::SeqCst) as f64;
    let spend = status["spend_usd"]["embedding"].as_f64().unwrap();
    assert!((spend - billed * 0.5 / 1e6).abs() < 1e-12, "spend {spend} for {billed} tokens");
    assert!(fake.models.lock().unwrap().iter().all(|m| m == "mock-embed"));

    // Hybrid: the chunk whose own text is the query is first in both lists.
    let series = server.search(json!({"query": "harmonic series geometric ratio", "mode": "bm25", "k": 1})).await;
    let target = hits(&series)[0].clone();
    assert_eq!(target["path"], "corpus/math/papers/series.txt");
    let query = query_from(target["text"].as_str().unwrap());
    let hybrid = server.search(json!({"query": query, "k": 5})).await;
    assert_eq!(hybrid["mode_used"], "hybrid");
    assert_eq!(hits(&hybrid)[0]["chunk_id"], target["chunk_id"], "{hybrid}");
    assert_eq!(hits(&hybrid)[0]["ranks"], json!({"bm25": 1, "vector": 1}));
    let expected = 2.0 / 61.0;
    assert!((hits(&hybrid)[0]["score"].as_f64().unwrap() - expected).abs() < 1e-9, "{hybrid}");

    let vector = server.search(json!({"query": query, "k": 5, "mode": "vector"})).await;
    assert_eq!(vector["mode_used"], "vector");
    assert_eq!(hits(&vector)[0]["chunk_id"], target["chunk_id"]);
    assert_eq!(hits(&vector)[0]["ranks"], json!({"bm25": null, "vector": 1}));
    let similarity = hits(&vector)[0]["score"].as_f64().unwrap();
    assert!(similarity > 0.5 && similarity <= 1.0 + 1e-6, "{vector}");

    // Filters apply inside the vector search too.
    for (filter, key, allowed) in [
        (json!({"domains": ["science"]}), "domain", vec!["science"]),
        (json!({"source_types": ["note", "unknown"]}), "source_type", vec!["note", "unknown"]),
        (json!({"domains": ["english", "chemistry"]}), "domain", vec!["english", "chemistry"]),
    ] {
        for mode in ["vector", "hybrid"] {
            let mut body = json!({"query": query, "k": 10, "mode": mode});
            body.as_object_mut().unwrap().extend(filter.as_object().unwrap().clone());
            let response = server.search(body).await;
            assert!(!hits(&response).is_empty(), "{filter} {mode}");
            assert!(hits(&response).iter().all(|h| allowed.contains(&h[key].as_str().unwrap())), "{response}");
        }
    }

    // Query embeddings are billed too.
    let (_, later) = server.get("/status").await;
    assert!(later["spend_usd"]["embedding"].as_f64().unwrap() > spend);

    // Re-ingesting embeds nothing new.
    let requests = fake.requests.load(Ordering::SeqCst);
    let again = server.wait_job(&server.start_job("/ingest", json!({})).await).await;
    assert_eq!(again["state"], "done");
    assert_eq!(fake.requests.load(Ordering::SeqCst), requests);
}

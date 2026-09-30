//! The local HTTP service (contract C1), bound to 127.0.0.1 only.

use crate::engine::{self, Engine, Roots, SURVEY_FILE};
use crate::error::Result;
use crate::ingest;
use crate::jobs::{self, JobKind, Jobs};
use crate::meta::SOURCE_TYPES;
use crate::ocr;
use crate::search::{self, SearchError, SearchRequest};
use crate::store::{self, SpendKind};
use crate::survey;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Handle;
use tower_http::cors::CorsLayer;

pub const DEFAULT_PORT: u16 = 8811;

/// The browser origins the desktop app uses (docs/tutor/CONTRACTS.md). No others are allowed.
pub const ALLOWED_ORIGINS: [&str; 4] =
    ["http://localhost:1420", "http://127.0.0.1:1420", "http://tauri.localhost", "tauri://localhost"];

pub struct AppState {
    pub engine: Arc<Engine>,
    pub roots: Roots,
    pub pdfium: PathBuf,
    pub jobs: Jobs,
    pub rt: Handle,
}

pub struct ApiError(StatusCode, String);

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> ApiError {
        ApiError(status, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, axum::Json(json!({"error": self.1}))).into_response()
    }
}

impl From<crate::error::Error> for ApiError {
    fn from(e: crate::error::Error) -> Self {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.0)
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

/// A JSON body; an empty body means the type's defaults.
fn parse_body<T: DeserializeOwned + Default>(body: &Bytes) -> ApiResult<T> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(T::default());
    }
    serde_json::from_slice(body).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, format!("invalid JSON body: {e}")))
}

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(ALLOWED_ORIGINS.map(HeaderValue::from_static))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/survey", post(start_survey))
        .route("/survey/latest", get(survey_latest))
        .route("/ingest", post(start_ingest))
        .route("/jobs/{job_id}", get(job))
        .route("/search", post(run_search))
        .fallback(|| async { ApiError::new(StatusCode::NOT_FOUND, "not found") })
        .method_not_allowed_fallback(|| async { ApiError::new(StatusCode::METHOD_NOT_ALLOWED, "method not allowed") })
        .layer(cors)
        .with_state(state)
}

/// Serve on 127.0.0.1:`port` (0 picks a free port) until Ctrl+C.
pub async fn serve(state: Arc<AppState>, port: u16) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("cannot listen on 127.0.0.1:{port}: {e}"))?;
    eprintln!("rag-engine: listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("rag-engine: shutting down");
        })
        .await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    axum::Json(json!({"ok": true, "version": env!("CARGO_PKG_VERSION")}))
}

async fn status(State(state): State<Arc<AppState>>) -> ApiResult<axum::Json<Value>> {
    let dims_ready = search::vectors_ready(&state.engine).await?;
    let state2 = state.clone();
    let value = tokio::task::spawn_blocking(move || status_json(&state2, dims_ready))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok(axum::Json(value))
}

fn status_json(state: &AppState, vectors_ready: bool) -> Result<Value> {
    let engine = &state.engine;
    let conn = engine.conn();
    let mut by_domain: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_source_type: BTreeMap<String, u64> = SOURCE_TYPES.iter().map(|s| (s.to_string(), 0)).collect();
    let (mut documents, mut pages, mut born_digital, mut scanned, mut math_heavy, mut empty) = (0, 0, 0, 0, 0, 0);
    let mut stmt = conn.prepare(
        "SELECT domain, source_type, COUNT(*), SUM(pages), SUM(born_digital), SUM(scanned), SUM(math_heavy), SUM(empty) \
         FROM documents WHERE state = 'indexed' GROUP BY domain, source_type",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            [r.get::<_, i64>(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?].map(|n| n as u64),
        ))
    })?;
    for row in rows {
        let (domain, source_type, [n, p, bd, sc, mh, em]) = row?;
        *by_domain.entry(domain).or_default() += n;
        *by_source_type.entry(source_type).or_default() += n;
        documents += n;
        pages += p;
        born_digital += bd;
        scanned += sc;
        math_heavy += mh;
        empty += em;
    }
    drop(stmt);
    let chunks: i64 = conn.query_row("SELECT COALESCE(SUM(chunks), 0) FROM documents WHERE state <> 'error'", [], |r| r.get(0))?;
    let unembedded: i64 = conn.query_row("SELECT COUNT(*) FROM chunks WHERE embedded <> 1", [], |r| r.get(0))?;
    let errors: i64 = conn.query_row("SELECT COUNT(*) FROM documents WHERE state = 'error'", [], |r| r.get(0))?;
    let (ocr_pending, ocr_done) = ocr::counts(&conn)?;
    let embedding = match &engine.embedder {
        Some(e) => {
            let dims = if vectors_ready {
                store::meta_get(&conn, ingest::EMBED_DIMS_KEY)?.and_then(|d| d.parse::<u64>().ok())
            } else {
                None
            };
            json!({"provider": e.provider_name(), "model": e.model(), "dims": dims})
        }
        None => json!({"provider": null, "model": null, "dims": null}),
    };
    Ok(json!({
        "corpus_root": engine::display_path(&state.roots.corpus),
        "data_dir": engine::display_path(&engine.data_dir),
        "documents": {"total": documents, "by_domain": by_domain, "by_source_type": by_source_type},
        "pages": {"total": pages, "by_class": {
            "born_digital": born_digital, "scanned": scanned, "math_heavy": math_heavy, "empty": empty}},
        "chunks": {"total": chunks, "embedded": (chunks - unembedded).max(0)},
        "ocr": {"pending_pages": ocr_pending, "done_pages": ocr_done},
        "errors": errors,
        "embedding": embedding,
        "active_job": state.jobs.active().map(|j| j.view()),
        "spend_usd": {
            "embedding": store::spend_usd(&conn, SpendKind::Embedding)?,
            "ocr": store::spend_usd(&conn, SpendKind::Ocr)?,
        },
    }))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurveyRequest {
    root: Option<PathBuf>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IngestRequest {
    root: Option<PathBuf>,
    #[serde(default)]
    rebuild: bool,
}

fn started(result: std::result::Result<String, String>) -> ApiResult<Response> {
    match result {
        Ok(job_id) => Ok((StatusCode::ACCEPTED, axum::Json(json!({"job_id": job_id}))).into_response()),
        Err(running) => Err(ApiError::new(StatusCode::CONFLICT, format!("job {running} is still running"))),
    }
}

async fn start_survey(State(state): State<Arc<AppState>>, body: Bytes) -> ApiResult<Response> {
    let req: SurveyRequest = parse_body(&body)?;
    let root = req.root.unwrap_or_else(|| state.roots.corpus.clone());
    if !root.is_dir() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, format!("root {} is not a directory", root.display())));
    }
    let (pdfium, out) = (state.pdfium.clone(), state.engine.data_dir.join(SURVEY_FILE));
    started(state.jobs.start(JobKind::Survey, state.engine.shared_conn(), move |progress| {
        let report = survey::build(&root, &pdfium, |done, total| progress.set(done as u64, total as u64))?;
        write_atomically(&out, &serde_json::to_string_pretty(&report)?)
    }))
}

fn write_atomically(path: &Path, text: &str) -> Result<()> {
    let dir = path.parent().ok_or("survey path has no parent")?;
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

async fn survey_latest(State(state): State<Arc<AppState>>) -> ApiResult<Response> {
    let path = state.engine.data_dir.join(SURVEY_FILE);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(([(header::CONTENT_TYPE, "application/json")], bytes).into_response()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ApiError::new(StatusCode::NOT_FOUND, "no survey has been run yet")),
        Err(e) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("cannot read {}: {e}", path.display()))),
    }
}

async fn start_ingest(State(state): State<Arc<AppState>>, body: Bytes) -> ApiResult<Response> {
    let req: IngestRequest = parse_body(&body)?;
    if let Some(root) = &req.root {
        ingest::targets(&state.roots, Some(root)).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.0))?;
    }
    let job_state = state.clone();
    started(state.jobs.start(JobKind::Ingest, state.engine.shared_conn(), move |progress| {
        let s = &job_state;
        ingest::ingest(&s.engine, &s.roots, &s.pdfium, req.root.as_deref(), req.rebuild, &s.rt, progress).map(|_| ())
    }))
}

async fn job(State(state): State<Arc<AppState>>, UrlPath(job_id): UrlPath<String>) -> ApiResult<Response> {
    match state.jobs.get(&state.engine.shared_conn(), &job_id)? {
        Some(view) => Ok(axum::Json(view).into_response()),
        None => Err(ApiError::new(StatusCode::NOT_FOUND, format!("no job {job_id}"))),
    }
}

async fn run_search(State(state): State<Arc<AppState>>, body: Bytes) -> ApiResult<Response> {
    let req: SearchRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, format!("invalid search request: {e}")))?;
    match search::search(state.engine.clone(), req).await {
        Ok(response) => Ok(axum::Json(response).into_response()),
        Err(SearchError::BadRequest(m)) => Err(ApiError::new(StatusCode::BAD_REQUEST, m)),
        Err(SearchError::Unavailable(m)) => Err(ApiError::new(StatusCode::CONFLICT, m)),
        Err(SearchError::Provider(m)) => Err(ApiError::new(StatusCode::BAD_GATEWAY, m)),
        Err(SearchError::Internal(m)) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, m)),
    }
}

/// Mark jobs a previous process left running as failed.
pub fn fail_interrupted_jobs(engine: &Engine) -> Result<()> {
    let n = jobs::fail_interrupted(&engine.conn())?;
    if n > 0 {
        eprintln!("rag-engine: {n} job(s) from a previous run were interrupted");
    }
    Ok(())
}

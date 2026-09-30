//! The HTTP service (contracts C1 and C9). It binds 127.0.0.1 unless `--bind` names another
//! address, which needs `RAG_ENGINE_TOKEN`. With a token, every route except `GET /health`
//! requires `Authorization: Bearer <token>`.

use crate::engine::{self, Engine, Roots, SURVEY_FILE};
use crate::error::Result;
use crate::ingest;
use crate::jobs::{self, JobKind, Jobs};
use crate::library::{self, ChunksQuery, ListParams, ListQuery};
use crate::meta::SOURCE_TYPES;
use crate::ocr;
use crate::search::{self, SearchError, SearchRequest};
use crate::store::{self, SpendKind};
use crate::survey;
use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path as UrlPath, Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::net::IpAddr;
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
    /// `RAG_ENGINE_TOKEN`; never logged.
    pub token: Option<String>,
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
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/documents", get(documents))
        .route("/documents/{doc_id}", get(document))
        .route("/documents/{doc_id}/chunks", get(document_chunks))
        .route("/survey", post(start_survey))
        .route("/survey/latest", get(survey_latest))
        .route("/ingest", post(start_ingest))
        .route("/jobs/{job_id}", get(job))
        .route("/search", post(run_search))
        .fallback(|| async { ApiError::new(StatusCode::NOT_FOUND, "not found") })
        .method_not_allowed_fallback(|| async { ApiError::new(StatusCode::METHOD_NOT_ALLOWED, "method not allowed") })
        // CORS is outside the token check, so browser preflights (which carry no token) succeed
        // and 401 responses still carry CORS headers.
        .layer(middleware::from_fn_with_state(state.clone(), require_token))
        .layer(cors)
        .with_state(state)
}

/// With a token configured, refuse every request but `GET /health` that does not carry it.
async fn require_token(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    let Some(token) = &state.token else { return next.run(request).await };
    if request.method() == Method::GET && request.uri().path() == "/health" {
        return next.run(request).await;
    }
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, credentials)| credentials.trim());
    // Comparing BLAKE3 digests is constant-time (blake3::Hash's equality) and hides the length.
    match presented {
        Some(p) if blake3::hash(p.as_bytes()) == blake3::hash(token.as_bytes()) => next.run(request).await,
        Some(_) => ApiError::new(StatusCode::UNAUTHORIZED, "invalid bearer token").into_response(),
        None => ApiError::new(StatusCode::UNAUTHORIZED, "missing bearer token: send Authorization: Bearer <RAG_ENGINE_TOKEN>")
            .into_response(),
    }
}

/// A non-loopback address is only allowed with a token.
pub fn check_bind(bind: IpAddr, token: Option<&str>) -> Result<()> {
    if !bind.is_loopback() && token.is_none() {
        return Err(format!(
            "refusing to listen on {bind} without RAG_ENGINE_TOKEN: set it to a long random secret, \
             or bind 127.0.0.1 (the default)"
        )
        .into());
    }
    Ok(())
}

/// Serve on `bind`:`port` (0 picks a free port) until Ctrl+C. `main` has checked the bind with [`check_bind`].
pub async fn serve(state: Arc<AppState>, bind: IpAddr, port: u16) -> Result<()> {
    let listener = tokio::net::TcpListener::bind((bind, port))
        .await
        .map_err(|e| format!("cannot listen on {bind}:{port}: {e}"))?;
    eprintln!(
        "rag-engine: {}",
        if state.token.is_some() {
            "RAG_ENGINE_TOKEN is set: every route except GET /health needs the bearer token"
        } else {
            "RAG_ENGINE_TOKEN is not set: requests are not authenticated"
        }
    );
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

/// Run a database read off the async threads.
async fn read_db<T: Send + 'static>(
    state: &Arc<AppState>,
    read: impl FnOnce(&rusqlite::Connection) -> Result<T> + Send + 'static,
) -> ApiResult<T> {
    let state = state.clone();
    Ok(tokio::task::spawn_blocking(move || read(&state.engine.conn()))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??)
}

fn query_params<T>(query: std::result::Result<Query<T>, QueryRejection>) -> ApiResult<T> {
    query.map(|Query(q)| q).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, format!("invalid query: {}", e.body_text())))
}

async fn documents(
    State(state): State<Arc<AppState>>,
    query: std::result::Result<Query<ListQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let params = ListParams::parse(query_params(query)?).map_err(|m| ApiError::new(StatusCode::BAD_REQUEST, m))?;
    let list = read_db(&state, move |conn| library::list(conn, &params)).await?;
    Ok(axum::Json(list).into_response())
}

async fn document(State(state): State<Arc<AppState>>, UrlPath(doc_id): UrlPath<String>) -> ApiResult<Response> {
    let id = doc_id.clone();
    match read_db(&state, move |conn| library::document(conn, &id)).await? {
        Some(detail) => Ok(axum::Json(detail).into_response()),
        None => Err(ApiError::new(StatusCode::NOT_FOUND, format!("no document {doc_id}"))),
    }
}

async fn document_chunks(
    State(state): State<Arc<AppState>>,
    UrlPath(doc_id): UrlPath<String>,
    query: std::result::Result<Query<ChunksQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let (from, limit) = library::chunk_params(query_params(query)?).map_err(|m| ApiError::new(StatusCode::BAD_REQUEST, m))?;
    let id = doc_id.clone();
    match read_db(&state, move |conn| library::chunks(conn, &id, from, limit)).await? {
        Some(page) => Ok(axum::Json(page).into_response()),
        None => Err(ApiError::new(StatusCode::NOT_FOUND, format!("no document {doc_id}"))),
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

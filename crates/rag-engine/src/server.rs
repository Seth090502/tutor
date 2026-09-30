//! The HTTP service (contracts C1 and C9). It binds 127.0.0.1 unless `--bind` names another
//! address, which needs `RAG_ENGINE_TOKEN`. With a token, every route except `GET /health`
//! requires `Authorization: Bearer <token>`. Requests a foreign web page could have sent are
//! refused by their `Host` and `Origin` (contract C3).

use crate::engine::{self, Engine, Roots, SURVEY_FILE};
use crate::error::Result;
use crate::ingest;
use crate::jobs::{self, JobKind, Jobs};
use crate::library::{self, ChunksQuery, ListParams, ListQuery};
use crate::meta::SOURCE_TYPES;
use crate::ocr;
use crate::pages;
use crate::search::{self, SearchError, SearchRequest};
use crate::store::{self, SpendKind};
use crate::survey;
use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path as UrlPath, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Handle;
use tower_http::cors::CorsLayer;

pub const DEFAULT_PORT: u16 = 8811;

/// The browser origins the desktop app uses (docs/tutor/CONTRACTS.md, C3): the built app's, and the
/// Vite dev server's in debug builds only. No others are allowed.
pub const ALLOWED_ORIGINS: &[&str] = allowed_origins(cfg!(debug_assertions));

/// The app origins to trust: `RAG_APP_ORIGINS` (comma-separated) when set, else [`ALLOWED_ORIGINS`].
/// The desktop shell sets it to its own origins, so a debug shell can run a release rag-engine.
fn app_origins() -> Vec<String> {
    std::env::var("RAG_APP_ORIGINS")
        .ok()
        .map(|v| v.split(',').map(|o| o.trim().to_string()).filter(|o| !o.is_empty()).collect::<Vec<_>>())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| ALLOWED_ORIGINS.iter().map(|o| o.to_string()).collect())
}

const fn allowed_origins(debug: bool) -> &'static [&'static str] {
    if debug {
        &["http://localhost:1420", "http://127.0.0.1:1420", "http://tauri.localhost", "tauri://localhost"]
    } else {
        &["http://tauri.localhost", "tauri://localhost"]
    }
}

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

pub fn router(state: Arc<AppState>, hosts: Option<Vec<String>>) -> Router {
    let origins = app_origins();
    let cors = CorsLayer::new()
        .allow_origin(origins.iter().filter_map(|origin| HeaderValue::from_str(origin).ok()).collect::<Vec<_>>())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/documents", get(documents))
        .route("/documents/{doc_id}", get(document))
        .route("/documents/{doc_id}/chunks", get(document_chunks))
        .route("/documents/{doc_id}/pages/{page}", get(page_image))
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
        // Between the two: CORS still answers preflights, and a foreign page is refused before
        // the token check or any route sees its request.
        .layer(middleware::from_fn_with_state(Arc::new(RequestGuard { hosts, origins }), guard_request))
        .layer(cors)
        .with_state(state)
}

/// Refuses what a web page other than the app could have sent (contract C3): a `Host` that is not
/// an address of this service (DNS rebinding), and a state-changing request whose `Origin` is not
/// an app origin, or that has none and is marked `Sec-Fetch-Site: cross-site`. GET is not
/// origin-checked: the app loads page images with `<img>`, which sends no `Origin`.
struct RequestGuard {
    /// `Host` values to accept ([`allowed_hosts`]); `None` accepts any.
    hosts: Option<Vec<String>>,
    /// App origins ([`app_origins`]).
    origins: Vec<String>,
}

impl RequestGuard {
    fn refusal(&self, method: &Method, headers: &HeaderMap) -> Option<&'static str> {
        if let Some(hosts) = &self.hosts {
            let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or_default().trim();
            if !hosts.iter().any(|allowed| allowed.eq_ignore_ascii_case(host)) {
                return Some("the Host header is not an address of this service");
            }
        }
        if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
            return None;
        }
        match headers.get(header::ORIGIN) {
            Some(origin) if self.origins.iter().any(|o| origin.as_bytes() == o.as_bytes()) => None,
            Some(_) => Some("the Origin is not one of the app's origins"),
            None if headers.get("sec-fetch-site").is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"cross-site")) => {
                Some("a cross-site request without an app Origin")
            }
            None => None,
        }
    }
}

async fn guard_request(State(guard): State<Arc<RequestGuard>>, request: Request, next: Next) -> Response {
    match guard.refusal(request.method(), request.headers()) {
        Some(reason) => ApiError::new(StatusCode::FORBIDDEN, format!("request refused: {reason}")).into_response(),
        None => next.run(request).await,
    }
}

/// `Host` values the service answers to: `127.0.0.1`, `localhost` and `[::1]` on its port, plus
/// its `--bind` address. `None` when it listens on every address (`0.0.0.0`, `::`): clients then
/// name the machine however they reach it, and the token such a bind requires ([`check_bind`])
/// already refuses a foreign page.
fn allowed_hosts(bind: IpAddr, port: u16) -> Option<Vec<String>> {
    if bind.is_unspecified() {
        return None;
    }
    let mut hosts: Vec<String> = ["127.0.0.1", "localhost", "[::1]"].iter().map(|name| format!("{name}:{port}")).collect();
    let bound = SocketAddr::new(bind, port).to_string();
    if !hosts.contains(&bound) {
        hosts.push(bound);
    }
    Some(hosts)
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
    let local = listener.local_addr()?;
    eprintln!("rag-engine: listening on http://{local}");
    axum::serve(listener, router(state, allowed_hosts(bind, local.port())))
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
    // A survey reads every file under its root, so only a folder of the library may be named.
    if let Some(root) = &req.root {
        ingest::targets(&state.roots, Some(root)).map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.0))?;
    }
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

/// `GET /documents/{doc_id}/pages/{n}.png` query parameters. An empty `highlight` means none.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    w: Option<u32>,
    highlight: Option<String>,
}

/// A PDF page as PNG (C11), rendered by PDFium with `highlight` marked (`src/pages.rs`).
async fn page_image(
    State(state): State<Arc<AppState>>,
    UrlPath((doc_id, page)): UrlPath<(String, String)>,
    query: std::result::Result<Query<PageQuery>, QueryRejection>,
) -> ApiResult<Response> {
    let not_found = |message: String| ApiError::new(StatusCode::NOT_FOUND, message);
    let number: u32 =
        page.strip_suffix(".png").and_then(|n| n.parse().ok()).ok_or_else(|| not_found("not found".into()))?;
    let query = query_params(query)?;
    let (width, highlight) = (pages::clamp_width(query.w), query.highlight.filter(|h| !h.trim().is_empty()));
    let id = doc_id.clone();
    let doc = read_db(&state, move |conn| {
        Ok(conn
            .query_row("SELECT path, kind, pages FROM documents WHERE doc_id = ?1 AND state = 'indexed'", [&id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })
            .optional()?)
    })
    .await?;
    let Some((path, kind, page_count)) = doc else { return Err(not_found(format!("no document {doc_id}"))) };
    if kind != "pdf" {
        return Err(not_found(format!("document {doc_id} is not a PDF")));
    }
    if number == 0 || i64::from(number) > page_count {
        return Err(not_found(format!("document {doc_id} has no page {number} (pages 1-{page_count})")));
    }
    let file = state.roots.resolve(&path).filter(|f| f.is_file()).ok_or_else(|| not_found(format!("{path} is no longer in the library")))?;
    let lib = state.pdfium.clone();
    let rendered = tokio::task::spawn_blocking(move || pages::render(&lib, &file, number, width, highlight.as_deref()))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match rendered {
        // The document id is a content hash, so a page image never changes.
        Ok(Some((png, _marked))) => {
            Ok(([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "private, max-age=86400")], png).into_response())
        }
        Ok(None) => Err(not_found(format!("document {doc_id} has no page {number}"))),
        Err(e) => Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{path}: {e}"))),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs.iter().map(|&(name, value)| (header::HeaderName::from_static(name), HeaderValue::from_static(value))).collect()
    }

    #[test]
    fn release_builds_trust_only_the_built_app() {
        assert_eq!(allowed_origins(false), ["http://tauri.localhost", "tauri://localhost"]);
        assert_eq!(
            allowed_origins(true),
            ["http://localhost:1420", "http://127.0.0.1:1420", "http://tauri.localhost", "tauri://localhost"]
        );
        assert_eq!(ALLOWED_ORIGINS, allowed_origins(cfg!(debug_assertions)));
    }

    #[test]
    fn hosts_are_the_loopback_names_on_the_port_plus_the_bind_address() {
        let loopback = ["127.0.0.1:8811", "localhost:8811", "[::1]:8811"].map(String::from).to_vec();
        assert_eq!(allowed_hosts("127.0.0.1".parse().unwrap(), 8811), Some(loopback.clone()));
        assert_eq!(allowed_hosts("::1".parse().unwrap(), 8811), Some(loopback.clone()));
        let lan = allowed_hosts("192.168.1.20".parse().unwrap(), 8811).unwrap();
        assert_eq!(lan, [loopback.clone(), vec!["192.168.1.20:8811".into()]].concat());
        assert_eq!(allowed_hosts("fe80::1".parse().unwrap(), 9000).unwrap().last().unwrap(), "[fe80::1]:9000");
        assert_eq!(allowed_hosts("0.0.0.0".parse().unwrap(), 8811), None);
        assert_eq!(allowed_hosts("::".parse().unwrap(), 8811), None);
    }

    #[test]
    fn guard_refuses_foreign_hosts_and_foreign_state_changes() {
        let origins: Vec<String> = ALLOWED_ORIGINS.iter().map(|o| o.to_string()).collect();
        let guard = RequestGuard { hosts: allowed_hosts("127.0.0.1".parse().unwrap(), 8811), origins: origins.clone() };
        let app = "tauri://localhost";
        let refused = |method: Method, extra: &[(&'static str, &'static str)]| {
            let mut all = headers(&[("host", "127.0.0.1:8811")]);
            all.extend(headers(extra));
            guard.refusal(&method, &all).is_some()
        };
        // Host: another name (DNS rebinding) or port is refused; case does not matter.
        for host in ["evil.example:8811", "127.0.0.1:8812", "127.0.0.1", "0.0.0.0:8811"] {
            assert!(refused(Method::GET, &[("host", host)]), "{host}");
        }
        assert!(!refused(Method::GET, &[("host", "LOCALHOST:8811")]));
        assert!(guard.refusal(&Method::GET, &HeaderMap::new()).is_some(), "no Host");
        // A state-changing request from a foreign page, with or without its Origin.
        assert!(refused(Method::POST, &[("origin", "http://evil.example")]));
        assert!(refused(Method::POST, &[("origin", "null")]));
        assert!(refused(Method::POST, &[("sec-fetch-site", "cross-site")]));
        // The app (cross-site to 127.0.0.1, with its Origin), native clients, and reads pass.
        assert!(!refused(Method::POST, &[("origin", app), ("sec-fetch-site", "cross-site")]));
        assert!(!refused(Method::POST, &[]));
        assert!(!refused(Method::GET, &[("origin", "http://evil.example"), ("sec-fetch-site", "cross-site")]));
        // Listening on every address: any Host, same Origin rules.
        let open = RequestGuard { hosts: None, origins };
        assert!(open.refusal(&Method::GET, &headers(&[("host", "library-pc:8811")])).is_none());
        assert!(open.refusal(&Method::POST, &headers(&[("origin", "http://evil.example")])).is_some());
    }

    #[test]
    fn the_shell_can_name_the_app_origins() {
        // One test owns the variable, so no other test sees it set.
        unsafe { std::env::set_var("RAG_APP_ORIGINS", " http://localhost:1420 ,tauri://localhost,") };
        let named = app_origins();
        unsafe { std::env::remove_var("RAG_APP_ORIGINS") };
        assert_eq!(named, ["http://localhost:1420", "tauri://localhost"]);
        assert_eq!(app_origins(), ALLOWED_ORIGINS.iter().map(|o| o.to_string()).collect::<Vec<_>>());
    }
}

//! Optional embeddings over HTTP, configured by environment variables (contract C1):
//!
//! | Variable | Meaning |
//! |---|---|
//! | `RAG_EMBED_PROVIDER` | `voyage` or `openai_compat`; unset means no embeddings |
//! | `RAG_EMBED_MODEL` | model name (required with a provider) |
//! | `RAG_EMBED_API_KEY` | API key; `voyage` also reads `VOYAGE_API_KEY` |
//! | `RAG_EMBED_BASE_URL` | API base; required for `openai_compat`, default `https://api.voyageai.com/v1` for `voyage` |
//! | `RAG_EMBED_PRICE_PER_MTOK` | USD per million tokens, for spend accounting (unset: spend is recorded as $0) |
//!
//! Requests go to `{base}/embeddings`. Voyage gets `input_type` `document` or `query`.
//! Throttling (429) and server errors (5xx), including timeouts and connection failures, are
//! retried with exponential backoff (or the server's `Retry-After`); other errors fail at once.

use crate::error::Result;
use futures::future::BoxFuture;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

pub const VOYAGE_BASE_URL: &str = "https://api.voyageai.com/v1";

/// Texts per request: 128 chunks of ~400 tokens is ~51K tokens, under Voyage's 120K-token and
/// OpenAI's 300K-token request limits.
pub const BATCH_SIZE: usize = 128;

const MAX_ATTEMPTS: u32 = 6;
const BASE_DELAY: Duration = Duration::from_secs(1);
const MAX_DELAY: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    Document,
    Query,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Embeddings {
    pub vectors: Vec<Vec<f32>>,
    /// Tokens billed, as reported by the provider.
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderError {
    Retryable { message: String, retry_after: Option<Duration> },
    Fatal(String),
}

/// One request to an embedding API. [`Embedder`] adds retries, validation and pricing.
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn embed<'a>(&'a self, texts: &'a [String], input: InputType) -> BoxFuture<'a, std::result::Result<Embeddings, ProviderError>>;
}

#[derive(Deserialize)]
struct ApiResponse {
    data: Vec<ApiItem>,
    usage: Option<ApiUsage>,
}

#[derive(Deserialize)]
struct ApiItem {
    embedding: Vec<f32>,
    index: Option<usize>,
}

#[derive(Deserialize)]
struct ApiUsage {
    total_tokens: Option<u64>,
    prompt_tokens: Option<u64>,
}

/// The `{base}/embeddings` call shared by both providers.
struct HttpEmbeddings {
    client: reqwest::Client,
    url: String,
    key: Option<String>,
    model: String,
}

impl HttpEmbeddings {
    fn new(base_url: &str, key: Option<String>, model: String) -> Result<HttpEmbeddings> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        Ok(HttpEmbeddings { client, url: format!("{}/embeddings", base_url.trim_end_matches('/')), key, model })
    }

    async fn post(&self, body: serde_json::Value, texts: &[String]) -> std::result::Result<Embeddings, ProviderError> {
        let mut request = self.client.post(&self.url).json(&body);
        if let Some(key) = &self.key {
            request = request.bearer_auth(key);
        }
        let response = request.send().await.map_err(|e| ProviderError::Retryable {
            message: format!("request to {} failed: {e}", self.url),
            retry_after: None,
        })?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(Duration::from_secs);
            let body: String = response.text().await.unwrap_or_default().chars().take(300).collect();
            let message = format!("{} returned HTTP {status}: {body}", self.url);
            return Err(if status.as_u16() == 429 || status.is_server_error() {
                ProviderError::Retryable { message, retry_after }
            } else {
                ProviderError::Fatal(message)
            });
        }
        let parsed: ApiResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Fatal(format!("unexpected response from {}: {e}", self.url)))?;
        let mut items: Vec<(usize, Vec<f32>)> =
            parsed.data.into_iter().enumerate().map(|(i, item)| (item.index.unwrap_or(i), item.embedding)).collect();
        items.sort_by_key(|(i, _)| *i);
        // Providers report billed tokens; if one does not, cl100k gives an estimate.
        let tokens = parsed.usage.and_then(|u| u.total_tokens.or(u.prompt_tokens)).unwrap_or_else(|| {
            texts.iter().map(|t| crate::chunk::tokenizer().encode_ordinary(t).len() as u64).sum()
        });
        Ok(Embeddings { vectors: items.into_iter().map(|(_, v)| v).collect(), tokens })
    }
}

pub struct Voyage(HttpEmbeddings);

impl Provider for Voyage {
    fn name(&self) -> &'static str {
        "voyage"
    }
    fn model(&self) -> &str {
        &self.0.model
    }
    fn embed<'a>(&'a self, texts: &'a [String], input: InputType) -> BoxFuture<'a, std::result::Result<Embeddings, ProviderError>> {
        let input_type = match input {
            InputType::Document => "document",
            InputType::Query => "query",
        };
        Box::pin(self.0.post(json!({"input": texts, "model": self.0.model, "input_type": input_type}), texts))
    }
}

pub struct OpenAiCompat(HttpEmbeddings);

impl Provider for OpenAiCompat {
    fn name(&self) -> &'static str {
        "openai_compat"
    }
    fn model(&self) -> &str {
        &self.0.model
    }
    fn embed<'a>(&'a self, texts: &'a [String], _input: InputType) -> BoxFuture<'a, std::result::Result<Embeddings, ProviderError>> {
        Box::pin(self.0.post(json!({"input": texts, "model": self.0.model}), texts))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmbedOutput {
    pub vectors: Vec<Vec<f32>>,
    pub tokens: u64,
    pub usd: f64,
}

pub struct Embedder {
    provider: Box<dyn Provider>,
    price_per_mtok: Option<f64>,
    base_delay: Duration,
}

impl Embedder {
    pub fn new(provider: Box<dyn Provider>, price_per_mtok: Option<f64>) -> Embedder {
        Embedder { provider, price_per_mtok, base_delay: BASE_DELAY }
    }

    /// The embedder described by the environment, or `None` when `RAG_EMBED_PROVIDER` is unset.
    pub fn from_env() -> Result<Option<Embedder>> {
        Self::from_vars(|name| std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
    }

    fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Option<Embedder>> {
        let Some(provider) = var("RAG_EMBED_PROVIDER") else { return Ok(None) };
        let model = var("RAG_EMBED_MODEL").ok_or("RAG_EMBED_MODEL is required when RAG_EMBED_PROVIDER is set")?;
        let price = match var("RAG_EMBED_PRICE_PER_MTOK") {
            None => None,
            Some(v) => Some(
                v.parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p >= 0.0)
                    .ok_or(format!("RAG_EMBED_PRICE_PER_MTOK must be a non-negative number, got {v:?}"))?,
            ),
        };
        let provider: Box<dyn Provider> = match provider.as_str() {
            "voyage" => {
                let key = var("RAG_EMBED_API_KEY")
                    .or_else(|| var("VOYAGE_API_KEY"))
                    .ok_or("voyage needs RAG_EMBED_API_KEY or VOYAGE_API_KEY")?;
                let base = var("RAG_EMBED_BASE_URL").unwrap_or_else(|| VOYAGE_BASE_URL.to_string());
                Box::new(Voyage(HttpEmbeddings::new(&base, Some(key), model)?))
            }
            "openai_compat" => {
                let base = var("RAG_EMBED_BASE_URL").ok_or("openai_compat needs RAG_EMBED_BASE_URL")?;
                Box::new(OpenAiCompat(HttpEmbeddings::new(&base, var("RAG_EMBED_API_KEY"), model)?))
            }
            other => return Err(format!("RAG_EMBED_PROVIDER must be voyage or openai_compat, got {other:?}").into()),
        };
        Ok(Some(Embedder::new(provider, price)))
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub fn model(&self) -> &str {
        self.provider.model()
    }

    pub fn is_priced(&self) -> bool {
        self.price_per_mtok.is_some()
    }

    pub fn usd(&self, tokens: u64) -> f64 {
        tokens as f64 * self.price_per_mtok.unwrap_or(0.0) / 1e6
    }

    /// One request for up to [`BATCH_SIZE`] texts, retried on throttling and server errors.
    pub async fn embed_batch(&self, texts: &[String], input: InputType) -> Result<EmbedOutput> {
        assert!(texts.len() <= BATCH_SIZE, "embed_batch takes at most {BATCH_SIZE} texts");
        if texts.is_empty() {
            return Ok(EmbedOutput { vectors: Vec::new(), tokens: 0, usd: 0.0 });
        }
        let name = self.provider.name();
        let mut attempt = 0;
        let embeddings = loop {
            match self.provider.embed(texts, input).await {
                Ok(e) => break e,
                Err(ProviderError::Retryable { message, retry_after }) if attempt + 1 < MAX_ATTEMPTS => {
                    let delay = retry_after.unwrap_or(self.base_delay * 2u32.pow(attempt)).min(MAX_DELAY);
                    attempt += 1;
                    eprintln!("embed: {message}; retry {attempt}/{} in {delay:?}", MAX_ATTEMPTS - 1);
                    tokio::time::sleep(delay).await;
                }
                Err(ProviderError::Retryable { message, .. }) => {
                    return Err(format!("{name} embeddings failed after {MAX_ATTEMPTS} attempts: {message}").into());
                }
                Err(ProviderError::Fatal(message)) => return Err(format!("{name} embeddings failed: {message}").into()),
            }
        };
        let dims = embeddings.vectors.first().map_or(0, Vec::len);
        if embeddings.vectors.len() != texts.len() || dims == 0 || embeddings.vectors.iter().any(|v| v.len() != dims) {
            return Err(format!(
                "{name} returned {} vectors (dimensions {dims}) for {} texts",
                embeddings.vectors.len(),
                texts.len()
            )
            .into());
        }
        Ok(EmbedOutput { usd: self.usd(embeddings.tokens), vectors: embeddings.vectors, tokens: embeddings.tokens })
    }
}

/// Deterministic bag-of-words vectors for tests: each lowercase word adds 1 to a hashed
/// dimension, and the result is L2-normalised.
#[cfg(test)]
pub fn mock_vector(text: &str, dims: usize) -> Vec<f32> {
    let mut v = vec![0f32; dims];
    for word in text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let hash = word.to_lowercase().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
        v[(hash % dims as u64) as usize] += 1.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    } else {
        v[0] = 1.0;
    }
    v
}

/// A provider for tests: scripted failures first, then [`mock_vector`]s; one token per word.
#[cfg(test)]
pub struct MockProvider {
    pub dims: usize,
    pub failures: std::sync::Mutex<std::collections::VecDeque<ProviderError>>,
    pub calls: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl MockProvider {
    pub fn new(dims: usize, failures: Vec<ProviderError>) -> MockProvider {
        MockProvider { dims, failures: std::sync::Mutex::new(failures.into()), calls: Default::default() }
    }
}

#[cfg(test)]
impl Provider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }
    fn model(&self) -> &str {
        "mock-model"
    }
    fn embed<'a>(&'a self, texts: &'a [String], _input: InputType) -> BoxFuture<'a, std::result::Result<Embeddings, ProviderError>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let failure = self.failures.lock().unwrap().pop_front();
        Box::pin(async move {
            if let Some(failure) = failure {
                return Err(failure);
            }
            Ok(Embeddings {
                vectors: texts.iter().map(|t| mock_vector(t, self.dims)).collect(),
                tokens: texts.iter().map(|t| t.split_whitespace().count() as u64).sum(),
            })
        })
    }
}

#[cfg(test)]
impl Embedder {
    pub fn mock(dims: usize, price_per_mtok: Option<f64>, failures: Vec<ProviderError>) -> Embedder {
        Embedder {
            provider: Box::new(MockProvider::new(dims, failures)),
            price_per_mtok,
            base_delay: Duration::from_millis(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::{IntoResponse, Response};
    use axum::routing::post;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn texts(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn no_provider_means_no_embedder() {
        assert!(Embedder::from_vars(vars(&[])).unwrap().is_none());
    }

    #[test]
    fn env_configuration_is_validated() {
        let e = Embedder::from_vars(vars(&[("RAG_EMBED_PROVIDER", "voyage"), ("RAG_EMBED_MODEL", "voyage-4"), ("VOYAGE_API_KEY", "k")]))
            .unwrap()
            .unwrap();
        assert_eq!((e.provider_name(), e.model(), e.is_priced()), ("voyage", "voyage-4", false));

        let e = Embedder::from_vars(vars(&[
            ("RAG_EMBED_PROVIDER", "openai_compat"),
            ("RAG_EMBED_MODEL", "m"),
            ("RAG_EMBED_BASE_URL", "http://127.0.0.1:9"),
            ("RAG_EMBED_PRICE_PER_MTOK", "0.12"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!((e.provider_name(), e.is_priced()), ("openai_compat", true));

        for (pairs, needle) in [
            (vec![("RAG_EMBED_PROVIDER", "voyage")], "RAG_EMBED_MODEL"),
            (vec![("RAG_EMBED_PROVIDER", "voyage"), ("RAG_EMBED_MODEL", "m")], "VOYAGE_API_KEY"),
            (vec![("RAG_EMBED_PROVIDER", "openai_compat"), ("RAG_EMBED_MODEL", "m")], "RAG_EMBED_BASE_URL"),
            (vec![("RAG_EMBED_PROVIDER", "cohere"), ("RAG_EMBED_MODEL", "m")], "voyage or openai_compat"),
            (
                vec![("RAG_EMBED_PROVIDER", "voyage"), ("RAG_EMBED_MODEL", "m"), ("VOYAGE_API_KEY", "k"), ("RAG_EMBED_PRICE_PER_MTOK", "-1")],
                "non-negative",
            ),
        ] {
            let err = Embedder::from_vars(vars(&pairs)).err().expect("should fail").0;
            assert!(err.contains(needle), "{err}");
        }
    }

    #[test]
    fn spend_is_tokens_times_price_per_million() {
        let e = Embedder::mock(4, Some(0.18), vec![]);
        assert!((e.usd(1_000_000) - 0.18).abs() < 1e-12);
        assert!((e.usd(2_500) - 0.00045).abs() < 1e-12);
        assert_eq!(Embedder::mock(4, None, vec![]).usd(1_000_000), 0.0);
    }

    #[tokio::test]
    async fn batches_report_tokens_and_usd() {
        let e = Embedder::mock(8, Some(2.0), vec![]);
        let out = e.embed_batch(&texts(&["one two three", "four five"]), InputType::Document).await.unwrap();
        assert_eq!(out.vectors.len(), 2);
        assert_eq!(out.vectors[0].len(), 8);
        assert_eq!(out.tokens, 5);
        assert!((out.usd - 5.0 * 2.0 / 1e6).abs() < 1e-15);
    }

    #[tokio::test]
    async fn throttling_and_server_errors_are_retried() {
        let retry = |m: &str| ProviderError::Retryable { message: m.into(), retry_after: None };
        let e = Embedder::mock(4, None, vec![retry("429"), retry("503")]);
        let out = e.embed_batch(&texts(&["a"]), InputType::Query).await.unwrap();
        assert_eq!(out.vectors.len(), 1);

        let e = Embedder::mock(4, None, (0..MAX_ATTEMPTS).map(|_| retry("503")).collect());
        let err = e.embed_batch(&texts(&["a"]), InputType::Query).await.unwrap_err().0;
        assert!(err.contains("after 6 attempts"), "{err}");
    }

    #[tokio::test]
    async fn other_errors_fail_without_retry() {
        let e = Embedder::mock(4, None, vec![ProviderError::Fatal("HTTP 401".into())]);
        let err = e.embed_batch(&texts(&["a"]), InputType::Query).await.unwrap_err().0;
        assert!(err.contains("401"), "{err}");
    }

    /// Each request's Authorization header and JSON body.
    type Requests = Arc<Mutex<Vec<(Option<String>, serde_json::Value)>>>;

    /// A fake embeddings API: records requests, answers with scripted statuses, then vectors
    /// in reverse index order (clients must sort by `index`).
    #[derive(Clone, Default)]
    struct Fake {
        requests: Requests,
        statuses: Arc<Mutex<Vec<u16>>>,
    }

    async fn fake_embeddings(State(fake): State<Fake>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Response {
        let auth = headers.get("authorization").map(|v| v.to_str().unwrap().to_string());
        fake.requests.lock().unwrap().push((auth, body.clone()));
        let status = fake.statuses.lock().unwrap().pop();
        if let Some(code) = status {
            return (StatusCode::from_u16(code).unwrap(), "try later").into_response();
        }
        let inputs: Vec<String> = serde_json::from_value(body["input"].clone()).unwrap();
        let data: Vec<_> = inputs
            .iter()
            .enumerate()
            .rev()
            .map(|(i, t)| json!({"object": "embedding", "index": i, "embedding": mock_vector(t, 6)}))
            .collect();
        Json(json!({"object": "list", "data": data, "model": body["model"], "usage": {"total_tokens": 42}})).into_response()
    }

    async fn start_fake(fake: Fake) -> String {
        let app = axum::Router::new().route("/v1/embeddings", post(fake_embeddings)).with_state(fake);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        url
    }

    #[tokio::test]
    async fn voyage_sends_input_type_and_key_and_sorts_by_index() {
        let fake = Fake::default();
        let base = start_fake(fake.clone()).await;
        let e = Embedder::from_vars(vars(&[
            ("RAG_EMBED_PROVIDER", "voyage"),
            ("RAG_EMBED_MODEL", "voyage-test"),
            ("RAG_EMBED_API_KEY", "secret-key"),
            ("RAG_EMBED_BASE_URL", &base),
            ("RAG_EMBED_PRICE_PER_MTOK", "1000000"),
        ]))
        .unwrap()
        .unwrap();
        let docs = texts(&["alpha beta", "gamma"]);
        let out = e.embed_batch(&docs, InputType::Document).await.unwrap();
        assert_eq!(out.vectors, vec![mock_vector("alpha beta", 6), mock_vector("gamma", 6)]);
        assert_eq!((out.tokens, out.usd), (42, 42.0));
        e.embed_batch(&texts(&["q"]), InputType::Query).await.unwrap();

        let requests = fake.requests.lock().unwrap().clone();
        assert_eq!(requests[0].0.as_deref(), Some("Bearer secret-key"));
        assert_eq!(requests[0].1, json!({"input": ["alpha beta", "gamma"], "model": "voyage-test", "input_type": "document"}));
        assert_eq!(requests[1].1["input_type"], "query");
    }

    #[tokio::test]
    async fn openai_compat_retries_http_503_and_omits_missing_key() {
        let fake = Fake::default();
        fake.statuses.lock().unwrap().extend([503, 429]);
        let base = start_fake(fake.clone()).await;
        let mut e = Embedder::from_vars(vars(&[
            ("RAG_EMBED_PROVIDER", "openai_compat"),
            ("RAG_EMBED_MODEL", "m"),
            ("RAG_EMBED_BASE_URL", &format!("{base}/")),
        ]))
        .unwrap()
        .unwrap();
        e.base_delay = Duration::from_millis(1);
        let out = e.embed_batch(&texts(&["x"]), InputType::Query).await.unwrap();
        assert_eq!(out.vectors.len(), 1);
        let requests = fake.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2].0, None);
        assert_eq!(requests[2].1, json!({"input": ["x"], "model": "m"}));
    }

    #[tokio::test]
    async fn http_400_is_not_retried() {
        let fake = Fake::default();
        fake.statuses.lock().unwrap().push(400);
        let base = start_fake(fake.clone()).await;
        let e = Embedder::from_vars(vars(&[("RAG_EMBED_PROVIDER", "openai_compat"), ("RAG_EMBED_MODEL", "m"), ("RAG_EMBED_BASE_URL", &base)]))
            .unwrap()
            .unwrap();
        let err = e.embed_batch(&texts(&["x"]), InputType::Query).await.unwrap_err().0;
        assert!(err.contains("400"), "{err}");
        assert_eq!(fake.requests.lock().unwrap().len(), 1);
    }
}

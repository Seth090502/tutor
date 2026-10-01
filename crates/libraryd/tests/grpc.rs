//! The gRPC interface and client: streaming, compression, connection reuse, retries, deadlines, and limits.

mod common;

use common::*;
use futures::StreamExt;
use libraryd::mock::{CODE, COWAN, MockVault};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tonic::codec::CompressionEncoding;
use tonic::codegen::http;
use tonic::transport::{Channel, Endpoint};
use tower::Service;
use vault_api::*;
use vault_client::proto as pb;
use vault_client::{ClientError, ClientOptions, Compression, merge_stream};

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

// ------------------------------------------------------------------------------------------- streaming

#[tokio::test(flavor = "multi_thread")]
async fn every_stream_merges_back_into_the_unary_answer() {
    let api = Arc::new(MockVault::new());
    let h = Harness::start(api.clone()).await;
    let c = h.grpc();

    let read = ReadRequest {
        addresses: strings(&[
            "3f9a1c07b2e4d55e#L12",
            "bad",
            "9c1d2e3f4a5b6c7d#L5-L6",
            "3f9a1c07b2e4d55e#L999",
            "00000000000000c0#L3",
        ]),
        expand: Some(Expand::Lines { before: 1, after: 1 }),
        max_tokens: None,
    };
    let chunks: Vec<ReadResponse> = c.read_stream(read.clone()).await.unwrap().map(|r| r.unwrap()).collect().await;
    assert_eq!(chunks.len(), 5, "one response per address");
    assert_eq!(chunks[1].errors[0].index, 1, "item errors point into the original request");
    assert_eq!(chunks[3].errors[0].index, 3);
    let merged = merge_stream::<ReadRequest>(c.read_stream(read.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, c.read(read).await.unwrap());

    let outline = OutlineRequest { doc_ids: strings(&[COWAN, "bad", CODE]), max_depth: None, max_tokens: None };
    let merged = merge_stream::<OutlineRequest>(c.outline_stream(outline.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, api.outline(outline).await.unwrap());

    let grep = GrepRequest {
        patterns: strings(&["magical number", "(", "fn \\w+", "Working memory"]),
        scope: Scope::Vault,
        context_lines: 1,
        max_hits: None,
        case_insensitive: false,
        max_tokens: None,
    };
    let chunks: Vec<GrepResponse> = c.grep_stream(grep.clone()).await.unwrap().map(|r| r.unwrap()).collect().await;
    assert_eq!(chunks.len(), 4);
    assert!(chunks[2].hits.iter().all(|h| h.pattern == 2), "hit pattern indexes are rebased");
    assert_eq!(chunks[1].errors[0].index, 1);
    let merged = merge_stream::<GrepRequest>(c.grep_stream(grep.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, api.grep(grep).await.unwrap());

    let find = FindDocsRequest {
        queries: strings(&["cowan", "textbook", "zzz"]),
        filters: Filters::default(),
        k: None,
        max_tokens: None,
    };
    let merged = merge_stream::<FindDocsRequest>(c.find_docs_stream(find.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, api.find_docs(find).await.unwrap());

    let search = SearchRequest {
        queries: strings(&["working memory", "chunking"]),
        filters: Filters::default(),
        k: Some(3),
        max_tokens: None,
    };
    let merged = merge_stream::<SearchRequest>(c.search_stream(search.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, api.search(search).await.unwrap());

    let define = DefineRequest {
        terms: strings(&["chunking", "unobtainium", "working memory"]),
        filters: Filters::default(),
        k: None,
        max_tokens: None,
    };
    let chunks: Vec<DefineResponse> =
        c.define_stream(define.clone()).await.unwrap().map(|r| r.unwrap()).collect().await;
    assert_eq!(chunks[2].definitions[0].term, 2, "definition term indexes are rebased");
    let merged = merge_stream::<DefineRequest>(c.define_stream(define.clone()).await.unwrap()).await.unwrap();
    assert_eq!(merged, api.define(define).await.unwrap());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_large_batch_streams_one_chunk_per_item() {
    let h = Harness::mock().await;
    let addresses: Vec<String> = (1..=300).map(|i| format!("9c1d2e3f4a5b6c7d#L{}", 1 + i % 80)).collect();
    let request = ReadRequest { addresses, expand: None, max_tokens: None };
    let mut stream = h.grpc().read_stream(request).await.unwrap();
    let mut count = 0;
    while let Some(chunk) = stream.next().await {
        assert_eq!(chunk.unwrap().passages.len(), 1);
        count += 1;
    }
    assert_eq!(count, 300);
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn stream_chunks_run_concurrently_and_arrive_in_request_order() {
    // Address Ln sleeps (9 - n) x 40 ms: the first address is the slowest. Run one after another, the eight
    // would take 40 x (8 + 7 + ... + 1) = 1,440 ms.
    let api = Scripted { read_delay_unit: Duration::from_millis(40), ..Scripted::default() };
    let h = Harness::start(Arc::new(api)).await;
    let request = ReadRequest {
        addresses: (1..=8).map(|n| format!("3f9a1c07b2e4d55e#L{n}")).collect(),
        expand: None,
        max_tokens: None,
    };
    let started = Instant::now();
    let chunks: Vec<ReadResponse> = h.grpc().read_stream(request).await.unwrap().map(|r| r.unwrap()).collect().await;
    let elapsed = started.elapsed();
    let lines: Vec<u32> = chunks.iter().map(|c| c.passages[0].address.start).collect();
    assert_eq!(lines, (1..=8).collect::<Vec<u32>>(), "results stay in request order");
    assert!(elapsed < Duration::from_millis(900), "chunks did not overlap: {elapsed:?}");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn separate_unary_calls_run_concurrently_on_one_connection() {
    let api = Scripted { flat_delay: Duration::from_millis(200), ..Scripted::default() };
    let h = Harness::start(Arc::new(api)).await;
    let client = h.grpc();
    let started = Instant::now();
    let calls = (0..8).map(|_| {
        let c = client.clone();
        tokio::spawn(async move { c.vault_info().await })
    });
    for r in futures::future::join_all(calls).await {
        r.unwrap().unwrap();
    }
    assert!(started.elapsed() < Duration::from_millis(1_000), "{:?}", started.elapsed());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_call_ends_the_stream_with_its_error() {
    let h = Harness::start(Arc::new(Scripted::failing(ApiError::Internal("disk gone".into()), usize::MAX))).await;
    let request = ReadRequest {
        addresses: strings(&["3f9a1c07b2e4d55e#L1", "3f9a1c07b2e4d55e#L2"]),
        expand: None,
        max_tokens: None,
    };
    let mut stream = h.grpc().read_stream(request).await.unwrap();
    match stream.next().await {
        Some(Err(ClientError::Api(ApiError::Internal(m)))) => assert_eq!(m, "disk gone"),
        other => panic!("{other:?}"),
    }
    h.server.shutdown().await.unwrap();

    let h = Harness::mock().await;
    let empty = ReadRequest { addresses: vec![], expand: None, max_tokens: None };
    let mut stream = h.grpc().read_stream(empty).await.unwrap();
    assert!(
        matches!(stream.next().await, Some(Err(ClientError::Api(ApiError::InvalidArgument(_))))),
        "an empty batch is the tool's own error"
    );
    h.server.shutdown().await.unwrap();
}

// -------------------------------------------------------------------------------------------- compression

/// The `grpc-encoding` of one request and of its response.
type Encodings = (Option<String>, Option<String>);

/// Wraps a channel and records the `grpc-encoding` of each request and response.
#[derive(Clone)]
struct Spy {
    inner: Channel,
    seen: Arc<Mutex<Vec<Encodings>>>,
}

impl Service<http::Request<tonic::body::Body>> for Spy {
    type Response = <Channel as Service<http::Request<tonic::body::Body>>>::Response;
    type Error = <Channel as Service<http::Request<tonic::body::Body>>>::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<tonic::body::Body>) -> Self::Future {
        let encoding = |h: &http::HeaderMap| h.get("grpc-encoding").and_then(|v| v.to_str().ok()).map(String::from);
        let sent = encoding(request.headers());
        let future = self.inner.call(request);
        let seen = self.seen.clone();
        Box::pin(async move {
            let response = future.await?;
            seen.lock().unwrap().push((sent, encoding(response.headers())));
            Ok(response)
        })
    }
}

async fn spy_client(
    h: &Harness,
    send: Option<CompressionEncoding>,
    accept: &[CompressionEncoding],
) -> (pb::vault_client::VaultClient<Spy>, Arc<Mutex<Vec<Encodings>>>) {
    let channel = Endpoint::from_shared(h.grpc_url()).unwrap().connect().await.unwrap();
    let seen = Arc::default();
    let mut client = pb::vault_client::VaultClient::new(Spy { inner: channel, seen: Arc::clone(&seen) });
    if let Some(enc) = send {
        client = client.send_compressed(enc);
    }
    for enc in accept {
        client = client.accept_compressed(*enc);
    }
    (client, seen)
}

fn authorised<T>(h: &Harness, message: T) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    request.metadata_mut().insert("authorization", format!("Bearer {}", h.token.expose()).parse().unwrap());
    request
}

#[tokio::test(flavor = "multi_thread")]
async fn grpc_compresses_with_zstd_and_gzip_when_the_client_asks() {
    let h = Harness::mock().await;
    // A request big enough to be worth compressing, answering with a long passage.
    let read = pb::ReadRequest {
        addresses: (0..200).map(|_| "3f9a1c07b2e4d55e#L1-L60".to_string()).collect(),
        expand: None,
        max_tokens: Some(20_000),
    };
    for (name, enc) in [("zstd", CompressionEncoding::Zstd), ("gzip", CompressionEncoding::Gzip)] {
        let (mut client, seen) = spy_client(&h, Some(enc), &[enc]).await;
        let response = client.read(authorised(&h, read.clone())).await.unwrap().into_inner();
        assert!(!response.passages.is_empty());
        let seen = seen.lock().unwrap().clone();
        assert_eq!(seen[0].0.as_deref(), Some(name), "request compressed with {name}");
        assert_eq!(seen[0].1.as_deref(), Some(name), "response compressed with {name}");
    }
    // A client that accepts nothing gets plain responses.
    let (mut plain, seen) = spy_client(&h, None, &[]).await;
    plain.read(authorised(&h, read)).await.unwrap();
    assert_eq!(seen.lock().unwrap()[0], (None, None));
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_vault_client_compresses_in_every_mode() {
    let h = Harness::mock().await;
    let request =
        ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L1-L60".into(); 50], expand: None, max_tokens: Some(20_000) };
    let expected = h.grpc().read(request.clone()).await.unwrap();
    for compression in [Compression::None, Compression::Gzip, Compression::Zstd] {
        let client = h.grpc_with(Some(h.token.expose()), ClientOptions { compression, ..Default::default() });
        assert_eq!(client.read(request.clone()).await.unwrap(), expected, "{compression:?}");
    }
    h.server.shutdown().await.unwrap();
}

// --------------------------------------------------------------------------------- connection and limits

/// A TCP forwarder that counts the connections it accepts.
async fn counting_proxy(target: std::net::SocketAddr) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    tokio::spawn(async move {
        while let Ok((mut inbound, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                if let Ok(mut outbound) = tokio::net::TcpStream::connect(target).await {
                    let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
                }
            });
        }
    });
    (addr, count)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_reuses_one_connection_for_many_calls() {
    let h = Harness::mock().await;
    let (proxy, accepted) = counting_proxy(h.server.grpc_addr).await;
    let client = vault_client::VaultClient::connect(
        &format!("http://{proxy}"),
        Some(vault_client::Secret::new(h.token.expose())),
        ClientOptions::default(),
    )
    .await
    .unwrap();
    let request = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L12".into()], expand: None, max_tokens: None };
    for _ in 0..20 {
        client.read(request.clone()).await.unwrap();
    }
    let parallel = (0..8).map(|_| {
        let (c, r) = (client.clone(), request.clone());
        tokio::spawn(async move { c.read(r).await })
    });
    for r in futures::future::join_all(parallel).await {
        r.unwrap().unwrap();
    }
    let _ = client.read_stream(request).await.unwrap().count().await;
    assert_eq!(accepted.load(Ordering::SeqCst), 1, "28 unary calls and a stream shared one TCP connection");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn connect_reports_an_unreachable_server_and_a_bad_endpoint() {
    let dead = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let options = ClientOptions { connect_timeout: Duration::from_millis(500), ..Default::default() };
    let err = vault_client::VaultClient::connect(&format!("http://{dead}"), None, options).await.unwrap_err();
    assert!(matches!(err, ClientError::Connect { .. }), "{err}");
    let err = vault_client::VaultClient::connect_lazy("not a url", None, ClientOptions::default()).unwrap_err();
    assert!(matches!(err, ClientError::Endpoint { .. }), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lazy_client_survives_a_server_restart() {
    let h = Harness::mock().await;
    let (addr, token) = (h.server.grpc_addr, h.token.clone());
    let client = vault_client::VaultClient::connect_lazy(
        &h.grpc_url(),
        Some(vault_client::Secret::new(token.expose())),
        ClientOptions {
            retry_backoff: Duration::from_millis(200),
            max_retries: 4,
            request_timeout: Duration::from_secs(10),
            ..Default::default()
        },
    )
    .unwrap();
    client.vault_info().await.unwrap();
    h.server.shutdown().await.unwrap();

    // The same port, a new server: the client reconnects by itself.
    let options = libraryd::server::ServerOptions {
        mcp_bind: "127.0.0.1:0".parse().unwrap(),
        grpc_bind: addr,
        allowed_hosts: vec!["127.0.0.1".into()],
        request_timeout: Duration::from_secs(5),
        ..libraryd::server::ServerOptions::new(token)
    };
    let second = libraryd::server::start(Arc::new(MockVault::new()), options).await.unwrap();
    client.vault_info().await.unwrap();
    second.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_requests_are_refused_and_the_server_carries_on() {
    let h = Harness::mock().await;
    let huge = ReadRequest { addresses: vec!["a".repeat(1_000); 5_000], expand: None, max_tokens: None };
    assert!(h.grpc().read(huge).await.is_err(), "a 5 MB request is over the limit");
    assert!(h.grpc().vault_info().await.is_ok());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn starting_on_a_taken_port_names_the_port() {
    let h = Harness::mock().await;
    let options = libraryd::server::ServerOptions {
        mcp_bind: h.server.mcp_addr,
        grpc_bind: "127.0.0.1:0".parse().unwrap(),
        allowed_hosts: vec!["127.0.0.1".into()],
        ..libraryd::server::ServerOptions::new(h.token.clone())
    };
    let err = libraryd::server::start(Arc::new(MockVault::new()), options).await.err().expect("bind fails");
    assert!(err.to_string().contains(&h.server.mcp_addr.to_string()), "{err}");
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_closes_both_ports() {
    let h = Harness::mock().await;
    let (mcp, grpc) = (h.server.mcp_addr, h.server.grpc_addr);
    assert!(tokio::net::TcpStream::connect(mcp).await.is_ok());
    h.server.shutdown().await.unwrap();
    assert!(tokio::net::TcpStream::connect(mcp).await.is_err());
    assert!(tokio::net::TcpStream::connect(grpc).await.is_err());
}

// ------------------------------------------------------------------------------------ retries, deadlines

#[tokio::test(flavor = "multi_thread")]
async fn transient_failures_are_retried_a_limited_number_of_times() {
    let fast =
        |retries| ClientOptions { max_retries: retries, retry_backoff: Duration::from_millis(5), ..Default::default() };
    let request = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L1".into()], expand: None, max_tokens: None };

    // Two failures, then success: two retries are enough.
    let api = Arc::new(Scripted::failing(ApiError::Unavailable("warming up".into()), 2));
    let h = Harness::start(api.clone()).await;
    h.grpc_with(Some(h.token.expose()), fast(2)).read(request.clone()).await.unwrap();
    assert_eq!(api.calls(), 3);
    h.server.shutdown().await.unwrap();

    // One retry is not enough for two failures, and no more calls are made after giving up.
    let api = Arc::new(Scripted::failing(ApiError::Unavailable("warming up".into()), 2));
    let h = Harness::start(api.clone()).await;
    let err = h.grpc_with(Some(h.token.expose()), fast(1)).read(request.clone()).await.unwrap_err();
    assert!(matches!(err, ClientError::Api(ApiError::Unavailable(_))), "{err:?}");
    assert_eq!(api.calls(), 2);
    h.server.shutdown().await.unwrap();

    // Errors that would repeat are not retried.
    for error in [
        ApiError::InvalidArgument("x".into()),
        ApiError::NotFound("x".into()),
        ApiError::Internal("x".into()),
        ApiError::NotYetAvailable("x".into()),
    ] {
        let api = Arc::new(Scripted::failing(error.clone(), usize::MAX));
        let h = Harness::start(api.clone()).await;
        assert!(h.grpc_with(Some(h.token.expose()), fast(3)).read(request.clone()).await.is_err());
        assert_eq!(api.calls(), 1, "{error:?} must not be retried");
        h.server.shutdown().await.unwrap();
    }

    // Unavailable retries forever only up to the limit.
    let api = Arc::new(Scripted::failing(ApiError::Unavailable("down".into()), usize::MAX));
    let h = Harness::start(api.clone()).await;
    assert!(h.grpc_with(Some(h.token.expose()), fast(3)).read(request).await.is_err());
    assert_eq!(api.calls(), 4);
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_deadline_cuts_off_a_slow_call() {
    let h = Harness::start(Arc::new(Scripted { flat_delay: Duration::from_secs(3), ..Scripted::default() })).await;
    let client = h.grpc_with(
        Some(h.token.expose()),
        ClientOptions { request_timeout: Duration::from_millis(200), max_retries: 0, ..Default::default() },
    );
    let started = Instant::now();
    let err = client.vault_info().await.unwrap_err();
    assert!(matches!(err, ClientError::Api(ApiError::Unavailable(_))), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_deadline_applies_on_both_interfaces() {
    let slow = Scripted { flat_delay: Duration::from_secs(3), ..Scripted::default() };
    let h = Harness::start_with(Arc::new(slow), |o| o.request_timeout = Duration::from_millis(150)).await;

    let started = Instant::now();
    let err = h
        .grpc_with(Some(h.token.expose()), ClientOptions { max_retries: 0, ..Default::default() })
        .vault_info()
        .await
        .unwrap_err();
    assert!(matches!(&err, ClientError::Api(ApiError::Unavailable(m)) if m.contains("150ms")), "{err:?}");

    let result = mcp_call(&h.mcp().await, "vault_info", &serde_json::json!({})).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.structured_content.unwrap()["kind"], "unavailable");
    assert!(started.elapsed() < Duration::from_secs(2));
    h.server.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_works_as_a_vault_api() {
    let h = Harness::mock().await;
    let remote: Arc<dyn VaultApi> = Arc::new(h.grpc());
    let local = MockVault::new();
    let request =
        FindDocsRequest { queries: strings(&["cowan"]), filters: Filters::default(), k: None, max_tokens: None };
    assert_eq!(remote.find_docs(request.clone()).await.unwrap(), local.find_docs(request).await.unwrap());
    let bad = ReadRequest { addresses: vec![], expand: None, max_tokens: None };
    assert!(matches!(remote.read(bad).await, Err(ApiError::InvalidArgument(_))));
    h.server.shutdown().await.unwrap();
}

// ------------------------------------------------------------------------------------------ shutdown

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_lets_a_running_call_finish() {
    let h = Harness::start(Arc::new(Scripted { flat_delay: Duration::from_millis(400), ..Scripted::default() })).await;
    let client = h.grpc();
    let call = tokio::spawn(async move { client.vault_info().await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    h.server.shutdown().await.unwrap();
    assert!(call.await.unwrap().is_ok(), "the call that was running when shutdown began completes");
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_aborts_calls_that_outlast_the_grace_period() {
    let slow = Scripted { flat_delay: Duration::from_secs(30), ..Scripted::default() };
    let h = Harness::start_with(Arc::new(slow), |o| o.shutdown_grace = Duration::from_millis(300)).await;
    let client = h.grpc_with(Some(h.token.expose()), ClientOptions { max_retries: 0, ..Default::default() });
    let call = tokio::spawn(async move { client.vault_info().await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let started = Instant::now();
    h.server.shutdown().await.unwrap();
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    assert!(call.await.unwrap().is_err());
}

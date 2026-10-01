//! Informational local round-trip latency of `read` over MCP and over gRPC against the mock.
//! Run: `cargo test -p libraryd --release --test latency -- --ignored --nocapture`

mod common;

use common::*;
use std::time::{Duration, Instant};
use vault_api::ReadRequest;

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank]
}

fn report(name: &str, mut samples: Vec<Duration>) {
    samples.sort();
    let mean = samples.iter().sum::<Duration>() / samples.len() as u32;
    println!(
        "{name}: n={} p50={:?} p95={:?} p99={:?} max={:?} mean={mean:?}",
        samples.len(),
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
        samples.last().unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "informational timing; run explicitly"]
async fn read_round_trip_latency() {
    const WARMUP: usize = 200;
    const SAMPLES: usize = 2_000;
    let h = Harness::mock().await;
    let request = ReadRequest { addresses: vec!["3f9a1c07b2e4d55e#L12".into()], expand: None, max_tokens: None };

    let grpc = h.grpc();
    let mut samples = Vec::with_capacity(SAMPLES);
    for i in 0..WARMUP + SAMPLES {
        let started = Instant::now();
        grpc.read(request.clone()).await.unwrap();
        if i >= WARMUP {
            samples.push(started.elapsed());
        }
    }
    report("read over gRPC (vault-client, zstd on)", samples);

    let mcp = h.mcp().await;
    let mut samples = Vec::with_capacity(SAMPLES);
    for i in 0..WARMUP + SAMPLES {
        let started = Instant::now();
        let result = mcp_call(&mcp, "read", &request).await;
        if i >= WARMUP {
            samples.push(started.elapsed());
        }
        assert_ne!(result.is_error, Some(true));
    }
    report("read over MCP (rmcp client, streamable HTTP)", samples);
    h.server.shutdown().await.unwrap();
}

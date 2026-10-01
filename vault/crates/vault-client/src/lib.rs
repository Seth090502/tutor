//! gRPC client for libraryd, plus the protobuf contract the server shares (docs/phase1/SPEC.md section 11).
//!
//! - [`VaultClient`] is the async client: one shared connection, deadlines, retries for repeatable calls and
//!   compression. It also implements [`vault_api::VaultApi`], so code written against the trait can run
//!   against a remote vault unchanged.
//! - [`proto`] holds the generated messages and, with the `server` feature, the generated tonic server stubs.
//! - [`convert`] maps between `vault-api` types and the messages; [`batch`] splits large batches for the
//!   streaming calls and merges the streamed answers back.

pub mod batch;
mod client;
pub mod convert;

pub use client::{ClientError, ClientOptions, Compression, Secret, VaultClient, merge_stream};

/// Generated from `proto/vault.proto`.
#[allow(clippy::all, clippy::pedantic, clippy::nursery, missing_docs, unreachable_pub)]
pub mod proto {
    tonic::include_proto!("vault.v1");
}

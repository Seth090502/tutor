//! Vault storage, canonical text, metadata, integrity and the VaultApi implementation. See docs/phase1/SPEC.md.
//!
//! - [`canon`]: canonical text for prose and code (SPEC 3).
//! - [`ids`], [`paths`]: identity, slugs, store names; path validation and confinement (SPEC 2, 4).
//! - [`config`]: machine config lookup and `vault.toml` (SPEC 4).
//! - [`store`]: write-once files, originals, integrity manifests, `verify` (SPEC 5).
//! - [`db`], [`jobs`]: the SQLite schema and the job queue (SPEC 6).
//! - [`sidecar`]: the typed sidecar (SPEC 3a).
//! - [`writer`]: intake's single writer, including `publish`.
//! - [`vault`]: the Phase 1 tools behind `vault_api::VaultApi` (SPEC 10).

pub mod canon;
pub mod config;
pub mod db;
pub mod error;
pub mod ids;
pub mod jobs;
pub mod paths;
pub mod sidecar;
pub mod store;
pub mod time;
pub mod vault;
pub mod writer;

#[cfg(test)]
mod testkit;

pub use config::VaultConfig;
pub use error::{CoreError, Result};
pub use vault::Vault;
pub use writer::Writer;

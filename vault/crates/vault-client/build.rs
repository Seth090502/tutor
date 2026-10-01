//! Compiles proto/vault.proto with the pure-Rust `protox` compiler, so no `protoc` binary is needed.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("../../proto");
    let proto = proto_dir.join("vault.proto");
    println!("cargo:rerun-if-changed={}", proto.display());
    println!("cargo:rerun-if-changed=build.rs");

    let descriptors = protox::compile([&proto], [&proto_dir])?;
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(std::env::var_os("CARGO_FEATURE_SERVER").is_some())
        .compile_fds(descriptors)?;
    Ok(())
}

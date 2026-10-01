// Records the compiler version and RUSTFLAGS so every result line says how the binary was built.
fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let ver = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default().replace('\u{1f}', " ");
    println!("cargo:rustc-env=VB_RUSTC_VERSION={ver}");
    println!("cargo:rustc-env=VB_RUSTFLAGS={flags}");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
}

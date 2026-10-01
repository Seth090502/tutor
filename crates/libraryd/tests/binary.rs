//! The `libraryd` binary itself: token creation, config handling, `--mock` start-up, serving a real vault, and
//! refusing to start when the vault cannot be opened.

mod common;

use common::raw_http;
use common::real;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use vault_api::ReadRequest;
use vault_client::{ClientOptions, Secret, VaultClient};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("libraryd-bin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_config(dir: &Path) -> PathBuf {
    let token = dir.join("token");
    let config = dir.join("libraryd.toml");
    let text = format!(
        "stage = \"local\"\nmachine = \"test-pc\"\n[server]\nmcp_bind = \"127.0.0.1:0\"\ngrpc_bind = \"127.0.0.1:0\"\ntoken_file = '{}'\nallowed_hosts = [\"127.0.0.1\"]\n",
        token.display()
    );
    std::fs::write(&config, text).unwrap();
    config
}

fn libraryd(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_libraryd"));
    command.args(args).env_remove("VAULT_CONFIG").stdin(Stdio::null());
    command
}

struct Running {
    child: Child,
    mcp: String,
    grpc: String,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts the binary (on the mock unless `mock` is false) and waits for its "ready" log line, which names the
/// ports it picked.
fn start(config: &Path, mock: bool) -> Running {
    let mut args = vec!["--config", config.to_str().unwrap()];
    if mock {
        args.push("--mock");
    }
    let mut child = libraryd(&args)
        .env("NO_COLOR", "1")
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn libraryd");
    let stderr = child.stderr.take().unwrap();
    let mut mcp = None;
    let mut grpc = None;
    for line in BufReader::new(stderr).lines() {
        let line = line.unwrap();
        if line.contains(" ready") || line.contains("ready ") {
            let field = |name: &str| {
                line.split(&format!("{name}=")).nth(1).and_then(|r| r.split_whitespace().next()).map(String::from)
            };
            mcp = field("mcp");
            grpc = field("grpc");
            break;
        }
    }
    // The pipe is dropped here; the server only writes more logs, and a closed pipe does not stop it.
    Running {
        child,
        mcp: mcp.expect("ready line with the MCP address"),
        grpc: grpc.expect("ready line with the gRPC address"),
    }
}

#[test]
fn init_token_writes_64_hex_and_never_overwrites() {
    let dir = scratch("token");
    let config = write_config(&dir);
    let out = libraryd(&["--config", config.to_str().unwrap(), "--init-token"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let token = std::fs::read_to_string(dir.join("token")).unwrap();
    assert_eq!(token.trim().len(), 64);
    assert!(token.trim().bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(!String::from_utf8_lossy(&out.stdout).contains(token.trim()), "the token is not printed");

    let again = libraryd(&["--config", config.to_str().unwrap(), "--init-token"]).output().unwrap();
    assert!(!again.status.success(), "a second --init-token must not replace the token");
    assert_eq!(std::fs::read_to_string(dir.join("token")).unwrap(), token);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn start_without_a_token_file_or_a_vault_fails_clearly() {
    let dir = scratch("fail");
    let config = write_config(&dir);

    let out = libraryd(&["--mock", "--config", config.to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("token"), "{}", String::from_utf8_lossy(&out.stderr));

    assert!(libraryd(&["--config", config.to_str().unwrap(), "--init-token"]).output().unwrap().status.success());
    let out = libraryd(&["--config", config.to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot open the vault described by") && stderr.contains("missing field `vault`"),
        "{stderr}"
    );

    let missing = libraryd(&["--config", dir.join("nope.toml").to_str().unwrap()]).output().unwrap();
    assert!(!missing.status.success());
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_mock_binary_serves_health_mcp_auth_and_grpc() {
    let dir = scratch("serve");
    let config = write_config(&dir);
    assert!(libraryd(&["--config", config.to_str().unwrap(), "--init-token"]).output().unwrap().status.success());
    let token = std::fs::read_to_string(dir.join("token")).unwrap().trim().to_string();
    let server = start(&config, true);

    let mcp_addr: std::net::SocketAddr = server.mcp.parse().unwrap();
    let (status, _, body) = raw_http(mcp_addr, "GET", "/healthz", &[], "").await;
    assert_eq!(status, 200);
    assert!(
        body.contains("\"ok\":true") && body.contains("\"stage\":\"local\"") && body.contains("mock-vault-0001"),
        "{body}"
    );

    let (status, _, _) = raw_http(mcp_addr, "POST", "/mcp", &common::MCP_HEADERS, common::INITIALIZE).await;
    assert_eq!(status, 401);
    let auth = format!("Bearer {token}");
    let mut headers = common::MCP_HEADERS.to_vec();
    headers.push(("Authorization", &auth));
    let (status, _, _) = raw_http(mcp_addr, "POST", "/mcp", &headers, common::INITIALIZE).await;
    assert_eq!(status, 200);

    let client =
        VaultClient::connect(&format!("http://{}", server.grpc), Some(Secret::new(token)), ClientOptions::default())
            .await
            .unwrap();
    assert_eq!(client.vault_info().await.unwrap().vault_id, "mock-vault-0001");
    drop(server);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn start_before_intake_has_created_the_database_says_so() {
    let vault = real::empty();
    let config = vault.config_path.to_str().unwrap();
    assert!(libraryd(&["--config", config, "--init-token"]).output().unwrap().status.success());
    let out = libraryd(&["--config", config]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("hot/db/vault.sqlite does not exist yet; run intake once"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_binary_serves_the_real_vault_named_in_its_config() {
    let vault = real::build();
    let config = vault.config_path.to_str().unwrap();
    assert!(libraryd(&["--config", config, "--init-token"]).output().unwrap().status.success());
    let token = std::fs::read_to_string(&vault.token_file).unwrap().trim().to_string();
    let server = start(&vault.config_path, false);

    let (status, _, body) = raw_http(server.mcp.parse().unwrap(), "GET", "/healthz", &[], "").await;
    assert!(status == 200 && body.contains(real::VAULT_ID), "{body}");

    let client =
        VaultClient::connect(&format!("http://{}", server.grpc), Some(Secret::new(token)), ClientOptions::default())
            .await
            .unwrap();
    let info = client.vault_info().await.unwrap();
    assert_eq!((info.vault_id.as_str(), info.documents), (real::VAULT_ID, vault.docs.len() as u64));
    let (doc, text) = &vault.docs[0];
    let read = ReadRequest { addresses: vec![format!("{doc}#L1")], expand: None, max_tokens: None };
    let passage = client.read(read).await.unwrap().passages.remove(0);
    assert_eq!(passage.text, text.lines().next().unwrap());
}

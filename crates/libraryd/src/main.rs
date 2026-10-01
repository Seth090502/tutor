//! libraryd: MCP and gRPC over the VaultApi. See docs/phase1/SPEC.md section 11.

use clap::Parser;
use libraryd::auth::Token;
use libraryd::config::{Config, find_config};
use libraryd::server::{ServerOptions, start};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vault_api::VaultApi;

#[derive(Parser, Debug)]
#[command(version, about = "The vault server: MCP (streamable HTTP) and gRPC over one VaultApi")]
struct Cli {
    /// Machine config (TOML). Default lookup: VAULT_CONFIG, %ProgramData%\vault\libraryd.toml, %APPDATA%\vault\libraryd.toml.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Serve fixture data instead of a vault. For trying the interfaces before the vault is wired in.
    #[arg(long)]
    mock: bool,
    /// Create the token file named in [server].token_file (32 random bytes as hex), then exit. Never overwrites.
    #[arg(long)]
    init_token: bool,
}

/// Stderr that never fails. If whatever reads our logs goes away (a closed pipe, a service wrapper that died),
/// tracing would otherwise `eprintln!` its complaint, which panics, and that would kill request handling.
struct QuietStderr;

impl std::io::Write for QuietStderr {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(|| QuietStderr).init();
}

fn token_path(config: &Config) -> Result<&Path, String> {
    config.server.token_file.as_deref().ok_or_else(|| "[server].token_file is not set in the config".to_string())
}

/// Writes a new token file. `create_new` makes it impossible to replace a token that is in use.
fn init_token(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(Token::generate().expose().as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

#[cfg(feature = "mock")]
fn mock_api() -> Result<Arc<dyn VaultApi>, Box<dyn std::error::Error>> {
    tracing::warn!("serving MOCK fixture data, not a vault");
    Ok(Arc::new(libraryd::mock::MockVault::new()))
}

#[cfg(not(feature = "mock"))]
fn mock_api() -> Result<Arc<dyn VaultApi>, Box<dyn std::error::Error>> {
    Err("this build has no mock backend (the `mock` feature is off)".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let cli = Cli::parse();

    // `--mock` may run without any config file: defaults give the SPEC ports, but a token file is still needed.
    let config = match find_config(cli.config.clone()) {
        Ok(path) => Config::load(&path)?,
        Err(_) if cli.mock => Config::default(),
        Err(e) => return Err(e.into()),
    };

    if cli.init_token {
        let path = token_path(&config)?;
        init_token(path)?;
        println!("created token file {}", path.display());
        return Ok(());
    }

    let token = Token::load(token_path(&config)?)?;

    let api: Arc<dyn VaultApi> = if cli.mock {
        mock_api()?
    } else {
        // ===== WIRE-UP POINT =====
        // The lead replaces this branch once vault-core has landed:
        //     let vault = vault_core::Vault::open(&config)?;
        //     Arc::new(vault) as Arc<dyn VaultApi>
        // Nothing else in libraryd needs to change: both interfaces take any `Arc<dyn VaultApi>`.
        return Err("no vault backend is wired in yet; run with --mock".into());
    };

    let vault_id =
        api.vault_info().await.map_err(|e| format!("the vault did not answer vault_info at startup: {e}"))?.vault_id;
    let options = ServerOptions::from_config(&config.server, token, config.stage(), &vault_id);
    let server = start(api, options).await?;
    tracing::info!(mcp = %server.mcp_addr, grpc = %server.grpc_addr, machine = config.machine.as_deref().unwrap_or("-"), "ready");

    server
        .run_until(async {
            if tokio::signal::ctrl_c().await.is_ok() {
                tracing::info!("shutting down");
            } else {
                // No signal handler: keep serving rather than shutting down at once.
                std::future::pending::<()>().await;
            }
        })
        .await?;
    Ok(())
}

//! libraryd: MCP and gRPC over the VaultApi. See docs/phase1/SPEC.md section 11.

use clap::Parser;
use libraryd::auth::Token;
use libraryd::blocking::Blocking;
use libraryd::config::{Config, find_config};
use libraryd::server::{ServerOptions, start};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use vault_api::VaultApi;

#[derive(Parser, Debug)]
#[command(version, about = "The vault server: MCP (streamable HTTP) and gRPC over one VaultApi")]
struct Cli {
    /// Machine config (TOML). Default lookup: VAULT_CONFIG, %ProgramData%\vault\libraryd.toml, %APPDATA%\vault\libraryd.toml.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Serve fixture data instead of the vault in the config. For trying the interfaces without a vault.
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

/// Opens the vault that the machine config at `config_path` describes. vault-core reads the whole file (libraryd's
/// own [`Config`] reads the same file, for `[server]`), so a typo anywhere in it is reported here.
fn open_vault(config_path: &Path) -> Result<Arc<dyn VaultApi>, Box<dyn std::error::Error>> {
    let vault = vault_core::VaultConfig::from_file(config_path)
        .and_then(vault_core::Vault::open)
        .map_err(|e| format!("cannot open the vault described by {}: {e}", config_path.display()))?;
    // The vault's calls block on SQLite and file I/O, so they run on the blocking pool, not on a worker thread.
    Ok(Arc::new(Blocking::new(Arc::new(vault))))
}

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Display, not the Debug form `fn main() -> Result` would print: the message is for the person
            // starting the server.
            let _ = writeln!(std::io::stderr(), "libraryd: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // `--mock` may run without any config file: defaults give the SPEC ports, but a token file is still needed.
    let (config, config_path) = match find_config(cli.config.clone()) {
        Ok(path) => (Config::load(&path)?, Some(path)),
        Err(_) if cli.mock => (Config::default(), None),
        Err(e) => return Err(e.into()),
    };

    if cli.init_token {
        let path = token_path(&config)?;
        init_token(path)?;
        println!("created token file {}", path.display());
        return Ok(());
    }

    let token = Token::load(token_path(&config)?)?;

    let api: Arc<dyn VaultApi> = match config_path {
        _ if cli.mock => mock_api()?,
        Some(path) => open_vault(&path)?,
        // Only `--mock` carries on without a config file; every other case returned above.
        None => return Err("no config file found".into()),
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

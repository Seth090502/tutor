//! The machine config (docs/phase1/SPEC.md section 4), reduced to what libraryd itself reads.
//!
//! vault-core owns the full config. Until it lands this module reads the TOML directly: `stage`, `machine`,
//! `[vault].root` and `[server]`. Every other section is ignored. When vault-core's config module is ready,
//! replace `Config::load` with a conversion from that type and keep [`ServerSection`].

use serde::Deserialize;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("invalid config {path}: {message}")]
    Parse { path: String, message: String },
    #[error("invalid [server] config: {0}")]
    Invalid(String),
    #[error("no config file found; pass --config, set VAULT_CONFIG, or create {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct VaultSection {
    pub root: Option<PathBuf>,
}

/// `[server]` from SPEC section 4.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSection {
    pub mcp_bind: SocketAddr,
    pub grpc_bind: SocketAddr,
    /// File holding the bearer token.
    pub token_file: Option<PathBuf>,
    /// `Host` header values the MCP endpoint accepts (a host name, or `host:port`).
    pub allowed_hosts: Vec<String>,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            mcp_bind: SocketAddr::from(([127, 0, 0, 1], 7341)),
            grpc_bind: SocketAddr::from(([127, 0, 0, 1], 7342)),
            token_file: None,
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// `local` or `networked`.
    pub stage: Option<String>,
    /// Label for logs and result lines.
    pub machine: Option<String>,
    pub vault: VaultSection,
    pub server: ServerSection,
}

impl Config {
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(text)
            .map_err(|e| ConfigError::Parse { path: origin.to_string(), message: e.to_string() })?;
        config.server.validate()?;
        Ok(config)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ConfigError::Read { path: path.to_path_buf(), source })?;
        Self::from_toml(&text, &path.display().to_string())
    }

    /// `stage` as written, `local` when absent.
    pub fn stage(&self) -> &str {
        self.stage.as_deref().unwrap_or("local")
    }
}

impl ServerSection {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // An empty list would switch Host checking off in the MCP transport, which is never what is meant.
        if self.allowed_hosts.is_empty() {
            return Err(ConfigError::Invalid("allowed_hosts must list at least one host".into()));
        }
        if self.allowed_hosts.iter().any(|h| h.trim().is_empty() || h.contains('/') || h.contains(char::is_whitespace))
        {
            return Err(ConfigError::Invalid(format!(
                "allowed_hosts has an entry that is not a host name: {:?}",
                self.allowed_hosts
            )));
        }
        if self.mcp_bind.port() != 0 && self.mcp_bind == self.grpc_bind {
            return Err(ConfigError::Invalid("mcp_bind and grpc_bind must differ".into()));
        }
        Ok(())
    }
}

/// Lookup order from SPEC section 4: `--config`, then `VAULT_CONFIG`, then `%ProgramData%\vault\libraryd.toml`,
/// then `%APPDATA%\vault\libraryd.toml`.
pub fn find_config(cli: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    find_config_in(cli, std::env::var_os("VAULT_CONFIG"), std::env::var_os("ProgramData"), std::env::var_os("APPDATA"))
}

fn find_config_in(
    cli: Option<PathBuf>,
    env: Option<OsString>,
    program_data: Option<OsString>,
    app_data: Option<OsString>,
) -> Result<PathBuf, ConfigError> {
    if let Some(path) = cli {
        return Ok(path);
    }
    if let Some(path) = env.filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let candidates: Vec<PathBuf> = [program_data, app_data]
        .into_iter()
        .flatten()
        .map(|base| PathBuf::from(base).join("vault").join("libraryd.toml"))
        .collect();
    candidates.iter().find(|p| p.is_file()).cloned().ok_or_else(|| {
        ConfigError::NotFound(candidates.first().map_or_else(|| "libraryd.toml".into(), |p| p.display().to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_example_parses() {
        let text = r#"
stage = "local"
machine = "main-pc"
[vault]
root = 'E:\Stand-In Vault'
hot = ""
[server]
mcp_bind = "127.0.0.1:7341"
grpc_bind = "127.0.0.1:7342"
token_file = 'C:\ProgramData\vault\token'
allowed_hosts = ["127.0.0.1", "localhost"]
[intake]
originals = "copy"
[sidecars]
grobid_url = ""
"#;
        let c = Config::from_toml(text, "test").unwrap();
        assert_eq!(c.stage(), "local");
        assert_eq!(c.machine.as_deref(), Some("main-pc"));
        assert_eq!(c.vault.root.as_deref(), Some(Path::new(r"E:\Stand-In Vault")));
        assert_eq!(c.server.mcp_bind.port(), 7341);
        assert_eq!(c.server.grpc_bind.port(), 7342);
        assert_eq!(c.server.token_file.as_deref(), Some(Path::new(r"C:\ProgramData\vault\token")));
        assert_eq!(c.server.allowed_hosts, vec!["127.0.0.1", "localhost"]);
    }

    #[test]
    fn missing_server_section_uses_the_spec_defaults() {
        let c = Config::from_toml("stage = \"networked\"", "test").unwrap();
        assert_eq!(c.stage(), "networked");
        assert_eq!(c.server, ServerSection::default());
        assert_eq!(c.server.mcp_bind.to_string(), "127.0.0.1:7341");
    }

    #[test]
    fn typos_in_server_are_errors() {
        let err = Config::from_toml("[server]\nmcp_bnd = \"127.0.0.1:1\"", "test").unwrap_err();
        assert!(err.to_string().contains("mcp_bnd"), "{err}");
    }

    #[test]
    fn unusable_values_are_rejected() {
        for text in [
            "[server]\nallowed_hosts = []",
            "[server]\nallowed_hosts = [\"\"]",
            "[server]\nallowed_hosts = [\"http://evil/\"]",
            "[server]\nmcp_bind = \"127.0.0.1:9\"\ngrpc_bind = \"127.0.0.1:9\"",
            "[server]\nmcp_bind = \"not an address\"",
        ] {
            assert!(Config::from_toml(text, "test").is_err(), "accepted {text}");
        }
    }

    #[test]
    fn lookup_order_prefers_cli_then_env() {
        let cli = Some(PathBuf::from("cli.toml"));
        let env = Some(OsString::from("env.toml"));
        assert_eq!(find_config_in(cli, env.clone(), None, None).unwrap(), PathBuf::from("cli.toml"));
        assert_eq!(find_config_in(None, env, None, None).unwrap(), PathBuf::from("env.toml"));
        assert!(matches!(find_config_in(None, None, None, None), Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn lookup_falls_back_to_program_data_then_app_data() {
        let dir = std::env::temp_dir().join(format!("libraryd-config-test-{}", std::process::id()));
        let (pd, ad) = (dir.join("pd"), dir.join("ad"));
        std::fs::create_dir_all(pd.join("vault")).unwrap();
        std::fs::create_dir_all(ad.join("vault")).unwrap();
        std::fs::write(ad.join("vault").join("libraryd.toml"), "").unwrap();
        let found = find_config_in(None, None, Some(pd.clone().into()), Some(ad.clone().into())).unwrap();
        assert_eq!(found, ad.join("vault").join("libraryd.toml"));
        std::fs::write(pd.join("vault").join("libraryd.toml"), "").unwrap();
        let found = find_config_in(None, None, Some(pd.clone().into()), Some(ad.into())).unwrap();
        assert_eq!(found, pd.join("vault").join("libraryd.toml"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

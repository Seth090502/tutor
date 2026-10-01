//! Machine config and `vault.toml` (SPEC section 4).
//!
//! The vault itself names no machine, drive or absolute path: `vault.toml` gives its identity and the default
//! tier names relative to the root. Each machine's own TOML, outside the vault, says where the root (and any
//! relocated tier) is. Moving the vault is therefore a config change only.

use crate::error::{CoreError, Result};
use crate::paths::{RelPath, TierRoot, is_network_path};
use serde::Deserialize;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub const SUPPORTED_FORMAT_VERSION: u32 = 1;
pub const CONFIG_ENV: &str = "VAULT_CONFIG";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Local,
    Networked,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Local => "local",
            Stage::Networked => "networked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalsMode {
    #[default]
    Copy,
    /// Needs the owner's approval before it is configured.
    Move,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineConfig {
    pub stage: Stage,
    /// Label for logs and result lines; never written into the vault.
    #[serde(default)]
    pub machine: String,
    pub vault: VaultSection,
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub intake: IntakeSection,
    #[serde(default)]
    pub sidecars: SidecarsSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultSection {
    /// The folder holding `vault.toml`.
    pub root: PathBuf,
    /// Optional absolute tier overrides; empty means `<root>/<tier name from vault.toml>`.
    #[serde(default)]
    pub hot: String,
    #[serde(default)]
    pub cold: String,
    #[serde(default)]
    pub inbox: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerSection {
    pub mcp_bind: String,
    pub grpc_bind: String,
    pub token_file: Option<PathBuf>,
    pub allowed_hosts: Vec<String>,
}

impl Default for ServerSection {
    fn default() -> Self {
        ServerSection {
            mcp_bind: "127.0.0.1:7341".into(),
            grpc_bind: "127.0.0.1:7342".into(),
            token_file: None,
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct IntakeSection {
    pub originals: OriginalsMode,
    pub stability_seconds: u64,
    pub loose_quiet_minutes: u64,
    pub workers: u32,
}

impl Default for IntakeSection {
    fn default() -> Self {
        IntakeSection { originals: OriginalsMode::Copy, stability_seconds: 30, loose_quiet_minutes: 15, workers: 6 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SidecarsSection {
    pub grobid_url: String,
    pub mineru: String,
    pub pandoc: String,
    /// Name of the environment variable holding the key; the value is never read here or logged.
    pub openalex_key_env: String,
}

impl Default for SidecarsSection {
    fn default() -> Self {
        SidecarsSection {
            grobid_url: String::new(),
            mineru: String::new(),
            pandoc: String::new(),
            openalex_key_env: "OPENALEX_API_KEY".into(),
        }
    }
}

/// `vault.toml` at the vault root.
#[derive(Debug, Clone, Deserialize)]
pub struct VaultToml {
    pub format_version: u32,
    pub vault_id: String,
    #[serde(default)]
    pub tiers: TierNames,
    #[serde(default)]
    pub taxonomy: TaxonomyRef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TierNames {
    pub hot: String,
    pub cold: String,
    pub inbox: String,
}

impl Default for TierNames {
    fn default() -> Self {
        TierNames { hot: "hot".into(), cold: "cold".into(), inbox: "inbox".into() }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TaxonomyRef {
    pub categories: Option<String>,
    pub version: Option<String>,
}

/// Canonical tier roots.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: TierRoot,
    pub hot: TierRoot,
    pub cold: TierRoot,
    pub inbox: TierRoot,
}

impl Layout {
    pub fn tiers(&self) -> [&TierRoot; 3] {
        [&self.hot, &self.cold, &self.inbox]
    }

    pub fn db_path(&self) -> PathBuf {
        self.hot.path().join("db").join("vault.sqlite")
    }
}

/// Where the machine config was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    Flag,
    Env,
    ProgramData,
    AppData,
}

/// A loaded, validated configuration: the machine config, the vault's own identity, and resolved tiers.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub machine: MachineConfig,
    pub vault: VaultToml,
    pub layout: Layout,
}

impl VaultConfig {
    /// Finds the machine config (`--config`, then `VAULT_CONFIG`, then `%ProgramData%`, then `%APPDATA%`) and
    /// loads it.
    pub fn load(flag: Option<&Path>) -> Result<(Self, PathBuf, ConfigSource)> {
        let (path, source) = locate(flag, |k| std::env::var_os(k))?;
        let config = Self::from_file(&path)?;
        tracing::info!(config = %path.display(), ?source, stage = config.machine.stage.as_str(), "config loaded");
        Ok((config, path, source))
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| CoreError::io("read config", path, e))?;
        Self::from_toml(&text).map_err(|e| match e {
            CoreError::Config(m) => CoreError::Config(format!("{}: {m}", path.display())),
            other => other,
        })
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        let machine: MachineConfig =
            toml::from_str(text).map_err(|e| CoreError::Config(format!("invalid machine config: {e}")))?;
        validate_machine(&machine)?;
        let root_dir = &machine.vault.root;
        if !root_dir.is_absolute() {
            return Err(CoreError::Config(format!(
                "[vault].root must be an absolute path, got {}",
                root_dir.display()
            )));
        }
        let root = TierRoot::open("root", root_dir)
            .map_err(|e| CoreError::Config(format!("[vault].root {} cannot be opened: {e}", root_dir.display())))?;
        let vault = read_vault_toml(root.path())?;
        let tier = |name: &'static str, override_: &str, default_name: &str| -> Result<TierRoot> {
            let path = if override_.is_empty() {
                let rel = RelPath::new(default_name).map_err(|e| {
                    CoreError::Config(format!(
                        "vault.toml [tiers].{name} = {default_name:?} is not a relative path: {e}"
                    ))
                })?;
                let mut p = root.path().to_path_buf();
                p.extend(rel.components());
                p
            } else {
                let p = PathBuf::from(override_);
                if !p.is_absolute() {
                    return Err(CoreError::Config(format!(
                        "[vault].{name} must be absolute or empty, got {override_:?}"
                    )));
                }
                p
            };
            TierRoot::open(name, &path).map_err(|e| {
                CoreError::Config(format!(
                    "{name} tier {} cannot be opened (create it, or fix [vault].{name}): {e}",
                    path.display()
                ))
            })
        };
        let layout = Layout {
            hot: tier("hot", &machine.vault.hot, &vault.tiers.hot)?,
            cold: tier("cold", &machine.vault.cold, &vault.tiers.cold)?,
            inbox: tier("inbox", &machine.vault.inbox, &vault.tiers.inbox)?,
            root,
        };
        validate_layout(&layout)?;
        Ok(VaultConfig { machine, vault, layout })
    }
}

fn read_vault_toml(root: &Path) -> Result<VaultToml> {
    let path = root.join("vault.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        CoreError::Config(format!("{} cannot be read; is [vault].root the vault folder? {e}", path.display()))
    })?;
    let v: VaultToml =
        toml::from_str(&text).map_err(|e| CoreError::Config(format!("{} is invalid: {e}", path.display())))?;
    if v.format_version != SUPPORTED_FORMAT_VERSION {
        return Err(CoreError::Config(format!(
            "{} has format_version {}; this build reads format {SUPPORTED_FORMAT_VERSION}",
            path.display(),
            v.format_version
        )));
    }
    if v.vault_id.trim().is_empty() {
        return Err(CoreError::Config(format!("{} has an empty vault_id", path.display())));
    }
    Ok(v)
}

fn validate_machine(m: &MachineConfig) -> Result<()> {
    for (key, value) in [("server.mcp_bind", &m.server.mcp_bind), ("server.grpc_bind", &m.server.grpc_bind)] {
        value
            .parse::<SocketAddr>()
            .map_err(|e| CoreError::Config(format!("[{key}] {value:?} is not an address like 127.0.0.1:7341: {e}")))?;
    }
    if m.server.mcp_bind == m.server.grpc_bind {
        return Err(CoreError::Config("[server] mcp_bind and grpc_bind must differ".into()));
    }
    if !(1..=256).contains(&m.intake.workers) {
        return Err(CoreError::Config(format!("[intake].workers must be 1-256, got {}", m.intake.workers)));
    }
    if m.intake.stability_seconds == 0 {
        return Err(CoreError::Config("[intake].stability_seconds must be at least 1".into()));
    }
    let url = &m.sidecars.grobid_url;
    if !url.is_empty() && !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(CoreError::Config(format!("[sidecars].grobid_url must be empty or an http(s) URL, got {url:?}")));
    }
    if m.sidecars.openalex_key_env.trim().is_empty() {
        return Err(CoreError::Config("[sidecars].openalex_key_env must name an environment variable".into()));
    }
    Ok(())
}

fn validate_layout(l: &Layout) -> Result<()> {
    let tiers = l.tiers();
    for (i, a) in tiers.iter().enumerate() {
        for b in &tiers[i + 1..] {
            if a.path().starts_with(b.path()) || b.path().starts_with(a.path()) {
                return Err(CoreError::Config(format!(
                    "the {} tier ({}) and the {} tier ({}) overlap; each tier needs its own folder",
                    a.name(),
                    a.path().display(),
                    b.name(),
                    b.path().display()
                )));
            }
        }
    }
    if is_network_path(l.hot.path()) {
        return Err(CoreError::Config(format!(
            "the hot tier {} is on a network share; SQLite must never run over a network share",
            l.hot.path().display()
        )));
    }
    Ok(())
}

/// The lookup order, with the environment injected so it can be tested.
pub fn locate(flag: Option<&Path>, env: impl Fn(&str) -> Option<OsString>) -> Result<(PathBuf, ConfigSource)> {
    if let Some(p) = flag {
        return if p.is_file() {
            Ok((p.to_path_buf(), ConfigSource::Flag))
        } else {
            Err(CoreError::Config(format!("--config {} does not exist", p.display())))
        };
    }
    if let Some(v) = env(CONFIG_ENV).filter(|v| !v.is_empty()) {
        let p = PathBuf::from(v);
        return if p.is_file() {
            Ok((p, ConfigSource::Env))
        } else {
            Err(CoreError::Config(format!("{CONFIG_ENV}={} does not exist", p.display())))
        };
    }
    for (var, source) in [("ProgramData", ConfigSource::ProgramData), ("APPDATA", ConfigSource::AppData)] {
        if let Some(base) = env(var).filter(|v| !v.is_empty()) {
            let p = PathBuf::from(base).join("vault").join("libraryd.toml");
            if p.is_file() {
                return Ok((p, source));
            }
        }
    }
    Err(CoreError::Config(format!(
        "no machine config found; tried --config, {CONFIG_ENV}, %ProgramData%\\vault\\libraryd.toml and %APPDATA%\\vault\\libraryd.toml"
    )))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal vault folder with the three tiers.
    pub(crate) fn make_vault(dir: &Path) -> PathBuf {
        let root = dir.join("vault");
        for t in ["hot", "cold", "inbox"] {
            std::fs::create_dir_all(root.join(t)).unwrap();
        }
        std::fs::write(
            root.join("vault.toml"),
            "format_version = 1\nvault_id = \"62171af3-test\"\ncreated = \"2026-10-01\"\n[tiers]\nhot = \"hot\"\ncold = \"cold\"\ninbox = \"inbox\"\n[taxonomy]\nversion = \"2026-10-01.1\"\n",
        )
        .unwrap();
        root
    }

    pub(crate) fn config_text(root: &Path) -> String {
        format!("stage = \"local\"\nmachine = \"test\"\n[vault]\nroot = '{}'\n", root.display())
    }

    #[test]
    fn loads_a_minimal_config_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let root = make_vault(dir.path());
        let c = VaultConfig::from_toml(&config_text(&root)).unwrap();
        assert_eq!(c.machine.stage, Stage::Local);
        assert_eq!(c.machine.intake.originals, OriginalsMode::Copy);
        assert_eq!(c.machine.intake.workers, 6);
        assert_eq!(c.vault.vault_id, "62171af3-test");
        assert_eq!(c.vault.taxonomy.version.as_deref(), Some("2026-10-01.1"));
        assert!(c.layout.hot.path().ends_with("hot"));
        assert!(c.layout.db_path().ends_with("vault.sqlite"));
    }

    #[test]
    fn parses_the_documented_example() {
        let dir = tempfile::tempdir().unwrap();
        let root = make_vault(dir.path());
        let cold = dir.path().join("elsewhere-cold");
        std::fs::create_dir_all(&cold).unwrap();
        let text = format!(
            "stage = \"networked\"\nmachine = \"main-pc\"\n[vault]\nroot = '{}'\nhot = \"\"\ncold = '{}'\ninbox = \"\"\n\
             [server]\nmcp_bind = \"127.0.0.1:7341\"\ngrpc_bind = \"127.0.0.1:7342\"\ntoken_file = 'C:\\ProgramData\\vault\\token'\n\
             allowed_hosts = [\"127.0.0.1\", \"localhost\"]\n[intake]\noriginals = \"copy\"\nstability_seconds = 30\n\
             loose_quiet_minutes = 15\nworkers = 6\n[sidecars]\ngrobid_url = \"\"\nmineru = \"\"\npandoc = \"\"\n\
             openalex_key_env = \"OPENALEX_API_KEY\"\n",
            root.display(),
            cold.display()
        );
        let c = VaultConfig::from_toml(&text).unwrap();
        assert_eq!(c.machine.stage, Stage::Networked);
        assert!(c.layout.cold.path().ends_with("elsewhere-cold"), "cold override applies");
    }

    #[test]
    fn rejects_bad_configs_with_clear_errors() {
        let dir = tempfile::tempdir().unwrap();
        let root = make_vault(dir.path());
        let base = config_text(&root);
        let cases: Vec<(String, &str)> = vec![
            (base.replace("\"local\"", "\"remote\""), "stage"),
            (format!("{base}typo = 1\n"), "typo"),
            (format!("{base}[intake]\nworkers = 0\n"), "workers"),
            (format!("{base}[intake]\noriginals = \"delete\"\n"), "originals"),
            (format!("{base}[server]\nmcp_bind = \"localhost\"\n"), "mcp_bind"),
            (format!("{base}[sidecars]\ngrobid_url = \"grobid:8070\"\n"), "grobid_url"),
            ("stage = \"local\"\n[vault]\nroot = 'relative\\path'\n".to_string(), "absolute"),
            (base.replace(&root.display().to_string(), &dir.path().join("missing").display().to_string()), "root"),
            (format!("{base}cold = 'relative'\n"), "absolute or empty"),
            (format!("{base}hot = '{}'\n", root.join("cold").display()), "overlap"),
        ];
        for (text, needle) in cases {
            let err = VaultConfig::from_toml(&text).expect_err(&text).to_string();
            assert!(err.contains(needle), "error for {needle:?} was {err}");
        }
    }

    #[test]
    fn rejects_bad_vault_toml() {
        let dir = tempfile::tempdir().unwrap();
        let root = make_vault(dir.path());
        std::fs::write(root.join("vault.toml"), "format_version = 2\nvault_id = \"x\"\n").unwrap();
        assert!(VaultConfig::from_toml(&config_text(&root)).unwrap_err().to_string().contains("format_version 2"));
        std::fs::write(root.join("vault.toml"), "format_version = 1\nvault_id = \" \"\n").unwrap();
        assert!(VaultConfig::from_toml(&config_text(&root)).unwrap_err().to_string().contains("vault_id"));
        std::fs::write(root.join("vault.toml"), "format_version = 1\nvault_id = \"x\"\n[tiers]\nhot = \"../hot\"\n")
            .unwrap();
        assert!(VaultConfig::from_toml(&config_text(&root)).unwrap_err().to_string().contains("not a relative path"));
        std::fs::remove_file(root.join("vault.toml")).unwrap();
        assert!(VaultConfig::from_toml(&config_text(&root)).unwrap_err().to_string().contains("vault.toml"));
    }

    #[test]
    fn lookup_order() {
        let dir = tempfile::tempdir().unwrap();
        let flag = dir.path().join("flag.toml");
        let env_file = dir.path().join("env.toml");
        let pd = dir.path().join("pd");
        let ad = dir.path().join("ad");
        for d in [&pd, &ad] {
            std::fs::create_dir_all(d.join("vault")).unwrap();
            std::fs::write(d.join("vault").join("libraryd.toml"), "").unwrap();
        }
        std::fs::write(&flag, "").unwrap();
        std::fs::write(&env_file, "").unwrap();
        let env = |vars: Vec<(&'static str, PathBuf)>| {
            move |k: &str| vars.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone().into_os_string())
        };

        let all = env(vec![("VAULT_CONFIG", env_file.clone()), ("ProgramData", pd.clone()), ("APPDATA", ad.clone())]);
        assert_eq!(locate(Some(&flag), &all).unwrap(), (flag.clone(), ConfigSource::Flag));
        assert_eq!(locate(None, &all).unwrap(), (env_file.clone(), ConfigSource::Env));
        let no_env = env(vec![("ProgramData", pd.clone()), ("APPDATA", ad.clone())]);
        assert_eq!(locate(None, &no_env).unwrap().1, ConfigSource::ProgramData);
        let app_only = env(vec![("ProgramData", dir.path().join("none")), ("APPDATA", ad.clone())]);
        assert_eq!(locate(None, &app_only).unwrap().1, ConfigSource::AppData);
        assert!(locate(None, env(vec![])).is_err());
        // An explicit choice that does not exist is an error, never a silent fall-through.
        assert!(locate(Some(&dir.path().join("nope.toml")), &all).is_err());
        let bad_env = env(vec![("VAULT_CONFIG", dir.path().join("nope.toml")), ("ProgramData", pd.clone())]);
        assert!(locate(None, &bad_env).is_err());
    }
}

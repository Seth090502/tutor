//! Non-secret settings, as JSON in the app config dir.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "config.json";

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub elevenlabs_agent_id: Option<String>,
}

/// A missing file is the default config; a malformed one is an error, not silently reset.
pub fn load(path: &Path) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Set one key. An empty value clears it.
pub fn set(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let mut config = load(path)?;
    let value = Some(value.trim().to_string()).filter(|v| !v.is_empty());
    match key {
        "elevenlabs_agent_id" => config.elevenlabs_agent_id = value,
        _ => return Err(format!("unknown setting {key:?}")),
    }
    save(path, &config)
}

fn save(path: &Path, config: &Config) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    // Write then rename, so a crash never leaves a half-written file.
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(config).expect("config serializes");
    std::fs::write(&tmp, text).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(FILE_NAME);
        assert_eq!(load(&path).unwrap(), Config::default());

        set(&path, "elevenlabs_agent_id", "  agent_123 ").unwrap();
        assert_eq!(
            load(&path).unwrap().elevenlabs_agent_id.as_deref(),
            Some("agent_123")
        );

        set(&path, "elevenlabs_agent_id", "").unwrap();
        assert_eq!(load(&path).unwrap(), Config::default());
    }

    #[test]
    fn serializes_the_contract_shape() {
        let config = Config {
            elevenlabs_agent_id: None,
        };
        assert_eq!(
            serde_json::to_value(&config).unwrap(),
            serde_json::json!({"elevenlabs_agent_id": null})
        );
    }

    #[test]
    fn rejects_unknown_keys_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        assert!(set(&path, "voyage_api_key", "x")
            .unwrap_err()
            .contains("unknown setting"));
        assert!(!path.exists());
    }

    #[test]
    fn malformed_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, "{not json").unwrap();
        assert!(load(&path).is_err());
        assert!(set(&path, "elevenlabs_agent_id", "a").is_err());
    }
}

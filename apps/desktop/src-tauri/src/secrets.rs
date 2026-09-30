//! API keys in Windows Credential Manager. Values leave this module only as child-process
//! environment variables or request headers; they are never returned to the webview or logged.

use std::collections::BTreeMap;

use keyring::{Entry, Error};

pub const SERVICE: &str = "com.sethy.tutor";

/// The only secret names the shell stores (contract C3).
pub const NAMES: [&str; 5] = [
    "elevenlabs",
    "opencode",
    "anthropic",
    "openrouter",
    "voyage",
];

pub fn validate_name(name: &str) -> Result<&'static str, String> {
    NAMES
        .iter()
        .copied()
        .find(|known| *known == name)
        .ok_or_else(|| {
            format!(
                "unknown secret {name:?}; expected one of {}",
                NAMES.join(", ")
            )
        })
}

fn entry(name: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, validate_name(name)?).map_err(describe)
}

/// The stored value, for passing to a service or an API. Not for the webview.
pub fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(describe(e)),
    }
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("the key is empty".into());
    }
    entry(name)?.set_password(value).map_err(describe)
}

/// Deleting a key that is not stored is not an error.
pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(describe(e)),
    }
}

/// Presence of every known secret, never the values.
pub fn status() -> Result<BTreeMap<&'static str, bool>, String> {
    NAMES
        .iter()
        .map(|&name| Ok((name, get(name)?.is_some())))
        .collect()
}

/// Every stored secret, for building a service's environment.
pub fn stored() -> Result<BTreeMap<&'static str, String>, String> {
    let mut out = BTreeMap::new();
    for name in NAMES {
        if let Some(value) = get(name)? {
            out.insert(name, value);
        }
    }
    Ok(out)
}

fn describe(e: Error) -> String {
    match e {
        // These variants carry the stored bytes; never format them.
        Error::BadEncoding(_) | Error::BadDataFormat(..) => {
            "Credential Manager: the stored value is unreadable; delete and save it again".into()
        }
        other => format!("Credential Manager: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exactly_the_contract_names() {
        for name in NAMES {
            assert_eq!(validate_name(name), Ok(name));
        }
    }

    #[test]
    fn rejects_other_names_before_touching_the_store() {
        for bad in [
            "",
            "ELEVENLABS",
            "elevenlabs ",
            "openai",
            "voyage/../x",
            "com.sethy.tutor",
        ] {
            let err = validate_name(bad).unwrap_err();
            assert!(err.contains("unknown secret"), "{bad:?}: {err}");
            assert!(set(bad, "value").is_err());
            assert!(delete(bad).is_err());
            assert!(get(bad).is_err());
        }
    }

    #[test]
    fn empty_values_are_rejected() {
        assert_eq!(set("voyage", "  \n").unwrap_err(), "the key is empty");
    }
}

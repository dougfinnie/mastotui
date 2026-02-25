//! Non-secret configuration (XDG paths, instance URL, client id).
//! Secrets (access token, client secret) are stored in the system credential store; see credential.rs.

use serde::{Deserialize, Serialize};

use crate::error::{MastotuiError, Result};

/// Non-sensitive app configuration persisted to disk.
/// r[config.persist-after-login]: instance URL and client id are stored here; secrets go to keyring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Mastodon instance base URL (e.g. https://mastodon.social)
    pub instance_url: String,
    /// OAuth app client id (public; safe in config file)
    pub client_id: String,
}

impl AppConfig {
    pub fn new(instance_url: String, client_id: String) -> Self {
        Self {
            instance_url: instance_url.trim_end_matches('/').to_string(),
            client_id,
        }
    }
}

/// Returns the XDG config directory for mastotui (e.g. ~/.config/mastotui).
/// r[config.first-run]: used to decide if we show login vs timeline.
pub fn config_dir() -> Result<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "mastotui", "mastotui")
        .map(|d| d.config_dir().to_path_buf())
        .ok_or_else(|| MastotuiError::Config("Could not determine config directory".into()))
}

/// Path to the config file (TOML, non-secret data only).
pub fn config_path() -> Result<std::path::PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Load config from disk if it exists.
pub fn load_config() -> Result<Option<AppConfig>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&path)
        .map_err(|e| MastotuiError::Config(format!("Failed to read config: {e}")))?;
    let config: AppConfig = toml::from_str(&s)
        .map_err(|e| MastotuiError::Config(format!("Invalid config TOML: {e}")))?;
    Ok(Some(config))
}

/// Save non-secret config to disk. Caller must persist secrets via credential module.
pub fn save_config(config: &AppConfig) -> Result<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| MastotuiError::Config(format!("Failed to create config dir: {e}")))?;
    let path = config_path()?;
    let s = toml::to_string_pretty(config)
        .map_err(|e| MastotuiError::Config(format!("Failed to serialize config: {e}")))?;
    std::fs::write(&path, s)
        .map_err(|e| MastotuiError::Config(format!("Failed to write config: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify config.first-run]
    #[test]
    fn load_config_returns_none_when_file_missing() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let result = load_config();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // r[verify config.persist-after-login]
    #[test]
    fn config_toml_has_no_secret_keys() {
        let config = AppConfig::new("https://example.com".into(), "client-id".into());
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(!toml.to_lowercase().contains("secret"));
        assert!(!toml.to_lowercase().contains("token"));
    }
}

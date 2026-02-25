//! Secure storage for secrets (access token, client secret) via system credential store.
//! Ensures sensitive information is encrypted at rest; no plain-text secrets on disk.

use keyring::Entry;

use crate::error::{MastotuiError, Result};

const SERVICE_NAME: &str = "mastotui";

/// Keyring account key for a given instance (hostname). Avoids storing secrets in config file.
fn account_key(instance_host: &str) -> String {
    format!("{}@{}", instance_host, "oauth")
}

/// Store access token in system keyring (encrypted at rest by OS).
/// r[config.persist-after-login]: token is persisted securely, not in plain text.
pub fn set_access_token(instance_host: &str, token: &str) -> Result<()> {
    let key = account_key(instance_host);
    let entry =
        Entry::new(SERVICE_NAME, &key).map_err(|e| MastotuiError::Credential(e.to_string()))?;
    entry
        .set_password(token)
        .map_err(|e| MastotuiError::Credential(e.to_string()))?;
    Ok(())
}

/// Retrieve access token from keyring. Returns None if not found or error.
pub fn get_access_token(instance_host: &str) -> Result<Option<String>> {
    let key = account_key(instance_host);
    let entry =
        Entry::new(SERVICE_NAME, &key).map_err(|e| MastotuiError::Credential(e.to_string()))?;
    match entry.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(MastotuiError::Credential(e.to_string())),
    }
}

/// Remove stored access token (e.g. on logout or 401).
pub fn delete_access_token(instance_host: &str) -> Result<()> {
    let key = account_key(instance_host);
    let entry =
        Entry::new(SERVICE_NAME, &key).map_err(|e| MastotuiError::Credential(e.to_string()))?;
    entry
        .delete_credential()
        .map_err(|e| MastotuiError::Credential(e.to_string()))?;
    Ok(())
}

/// Store client secret in keyring (Mastodon app secret; must not be in config file).
pub fn set_client_secret(instance_host: &str, secret: &str) -> Result<()> {
    let key = format!("{}@client_secret", instance_host);
    let entry =
        Entry::new(SERVICE_NAME, &key).map_err(|e| MastotuiError::Credential(e.to_string()))?;
    entry
        .set_password(secret)
        .map_err(|e| MastotuiError::Credential(e.to_string()))?;
    Ok(())
}

/// Retrieve client secret from keyring.
pub fn get_client_secret(instance_host: &str) -> Result<Option<String>> {
    let key = format!("{}@client_secret", instance_host);
    let entry =
        Entry::new(SERVICE_NAME, &key).map_err(|e| MastotuiError::Credential(e.to_string()))?;
    match entry.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(MastotuiError::Credential(e.to_string())),
    }
}

/// Extract host from instance URL for use as keyring account scope.
pub fn instance_host_from_url(instance_url: &str) -> Result<String> {
    let url = url::Url::parse(instance_url).map_err(|e| MastotuiError::Config(e.to_string()))?;
    url.host_str()
        .map(str::to_string)
        .ok_or_else(|| MastotuiError::Config("Instance URL has no host".into()))
}

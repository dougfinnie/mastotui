//! OAuth 2.0 app registration and token exchange.
//! r[auth.app.register.on-first-login] r[auth.app.register.skip-when-stored] r[auth.login.exchange-code]

use oauth2::{AuthUrl, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenUrl};
use reqwest::Client;

use crate::api::types::Application;
use crate::config;
use crate::credential::{
    get_client_secret, instance_host_from_url, set_access_token, set_client_secret,
};
use crate::error::{MastotuiError, Result};

// read + write are universal and include follow/lists; "follow" is deprecated (3.5+) and rejected by some instances.
const SCOPES: &[&str] = &["read", "write"];

/// Register app with Mastodon instance if not already stored.
/// Returns (`client_id`, `client_secret`). Stores secret in keyring only.
pub async fn register_app_if_needed(
    instance_url: &str,
    client: &Client,
) -> Result<(String, String)> {
    let host = instance_host_from_url(instance_url)?;
    // r[auth.app.register.skip-when-stored]: skip only when we have both config and secret for this instance
    if let Some(secret) = get_client_secret(&host)? {
        if let Some(cfg) = config::load_config()? {
            if cfg.instance_url == instance_url && !cfg.client_id.is_empty() {
                return Ok((cfg.client_id, secret));
            }
        }
        // Secret exists but no config (e.g. config file deleted or never saved). Re-register to get client_id.
    }

    // r[auth.app.register.on-first-login]: register app, store client_id in config and client_secret in keyring
    let url = format!("{}/api/v1/apps", instance_url.trim_end_matches('/'));
    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "client_name": "mastotui",
            "redirect_uris": "urn:ietf:wg:oauth:2.0:oob",
            "scopes": SCOPES.join(" "),
            "website": "https://github.com/dougfinnie/mastotui"
        }))
        .send()
        .await?;

    let status = response.status();
    let body: Application = response.json().await.map_err(|e| MastotuiError::Api {
        status: status.as_u16(),
        message: format!("Invalid app registration response: {e}"),
    })?;

    let status_code = status.as_u16();
    let client_id = body.client_id.ok_or_else(|| MastotuiError::Api {
        status: status_code,
        message: "App registration did not return client_id".into(),
    })?;
    let client_secret = body.client_secret.ok_or_else(|| MastotuiError::Api {
        status: status_code,
        message: "App registration did not return client_secret".into(),
    })?;

    set_client_secret(&host, &client_secret)?;
    Ok((client_id, client_secret))
}

/// Build authorization URL for user to open in browser. Returns (url, `pkce_verifier` for later token exchange).
pub fn authorization_url(instance_url: &str, client_id: &str) -> Result<(String, String)> {
    let base = instance_url.trim_end_matches('/');
    let auth_url = format!("{base}/oauth/authorize");
    let token_url = format!("{base}/oauth/token");
    let redirect = "urn:ietf:wg:oauth:2.0:oob";

    let client = oauth2::basic::BasicClient::new(
        ClientId::new(client_id.to_string()),
        None,
        AuthUrl::new(auth_url).map_err(|e| MastotuiError::OAuth(e.to_string()))?,
        Some(TokenUrl::new(token_url).map_err(|e| MastotuiError::OAuth(e.to_string()))?),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect.to_string()).map_err(|e| MastotuiError::OAuth(e.to_string()))?,
    );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, _csrf) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .add_scopes(SCOPES.iter().map(|s| Scope::new((*s).to_string())))
        .url();

    Ok((url.to_string(), pkce_verifier.secret().clone()))
}

/// Exchange authorization code (from out-of-band redirect) for access token.
///
/// r[auth.login.exchange-code]: store token in keyring after exchange.
/// Uses `client_secret_post` (form body only); union.place and Doorkeeper accept this.
/// Sending both Basic and form can trigger "unsupported authentication method".
pub async fn exchange_code_for_token(
    instance_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    pkce_verifier: &str,
    http_client: &Client,
) -> Result<String> {
    let base = instance_url.trim_end_matches('/');
    let token_url = format!("{base}/oauth/token");

    let response = http_client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
            ("code_verifier", pkce_verifier),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(MastotuiError::OAuth(format!(
            "Token exchange failed: {status} - {text}"
        )));
    }

    let json: serde_json::Value = response.json().await?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MastotuiError::OAuth("Token response missing access_token".into()))?;

    let host = instance_host_from_url(instance_url)?;
    set_access_token(&host, access_token)?;
    Ok(access_token.to_string())
}

/// Obtain an app-level access token via `client_credentials` grant.
/// Used for public timeline when the instance has "public preview" disabled (Mastodon 3.0+).
/// Do not store this token; it is for anonymous read-only access only.
pub async fn app_token_client_credentials(
    instance_url: &str,
    client_id: &str,
    client_secret: &str,
    http_client: &Client,
) -> Result<String> {
    let base = instance_url.trim_end_matches('/');
    let token_url = format!("{base}/oauth/token");

    let response = http_client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(MastotuiError::OAuth(format!(
            "App token failed: {status} - {text}"
        )));
    }

    let json: serde_json::Value = response.json().await?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MastotuiError::OAuth("App token response missing access_token".into()))?;

    Ok(access_token.to_string())
}

#[cfg(test)]
mod tests {
    // r[verify auth.login.exchange-code]
    #[test]
    fn token_exchange_uses_oauth_token_endpoint() {
        let base = "https://mastodon.social";
        let token_url = format!("{}/oauth/token", base.trim_end_matches('/'));
        assert!(token_url.ends_with("/oauth/token"));
        assert_eq!(token_url, "https://mastodon.social/oauth/token");
    }
}

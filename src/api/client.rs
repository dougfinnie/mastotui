//! Mastodon API HTTP client. On 401 clears token and returns NotAuthenticated.
//! r[timeline.home.fetch] r[timeline.pagination] r[toot.post.submit] r[toot.reply] r[toot.boost.toggle] r[toot.favourite.toggle]
//! r[auth.login.invalid-token]

use reqwest::Client;

use crate::api::types::Status;
use crate::credential::{delete_access_token, get_access_token, instance_host_from_url};
use crate::error::{MastotuiError, Result};

/// API client for a single Mastodon instance with a given access token.
pub struct MastodonClient {
    base_url: String,
    instance_host: String,
    access_token: String,
    client: Client,
}

impl MastodonClient {
    pub fn new(base_url: String, access_token: String) -> Result<Self> {
        let instance_host = instance_host_from_url(&base_url)?;
        let client = Client::builder().user_agent("mastotui/0.1").build()?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            instance_host,
            access_token,
            client,
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    /// On 401, clear stored token and return NotAuthenticated.
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response> {
        let url = self.api_url(path);
        let mut req = self
            .client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            req = req.json(&b);
        }

        let response = req.send().await?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            let _ = delete_access_token(&self.instance_host);
            return Err(MastotuiError::NotAuthenticated);
        }
        Ok(response)
    }

    /// r[timeline.home.fetch]: fetch home timeline
    pub async fn get_timeline_home(&self, max_id: Option<&str>) -> Result<Vec<Status>> {
        let path = match max_id {
            Some(id) => format!("/timelines/home?limit=20&max_id={}", id),
            None => "/timelines/home?limit=20".to_string(),
        };
        let response = self.request(reqwest::Method::GET, &path, None).await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MastotuiError::Api {
                status: status.as_u16(),
                message: text,
            });
        }
        let statuses: Vec<Status> = response.json().await?;
        Ok(statuses)
    }

    /// r[toot.post.submit]: post new status
    pub async fn post_status(&self, status: &str, in_reply_to_id: Option<&str>) -> Result<Status> {
        let body = serde_json::json!({
            "status": status,
            "in_reply_to_id": in_reply_to_id
        });
        let response = self
            .request(reqwest::Method::POST, "/statuses", Some(body))
            .await?;
        let s = response.status();
        if !s.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MastotuiError::Api {
                status: s.as_u16(),
                message: text,
            });
        }
        Ok(response.json().await?)
    }

    /// r[toot.boost.toggle]: reblog or unreblog
    pub async fn reblog(&self, id: &str, reblog: bool) -> Result<Status> {
        let path = if reblog {
            format!("/statuses/{}/reblog", id)
        } else {
            format!("/statuses/{}/unreblog", id)
        };
        let response = self.request(reqwest::Method::POST, &path, None).await?;
        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MastotuiError::Api {
                status: response.status().as_u16(),
                message: text,
            });
        }
        Ok(response.json().await?)
    }

    /// r[toot.favourite.toggle]: favourite or unfavourite
    pub async fn favourite(&self, id: &str, favourite: bool) -> Result<Status> {
        let path = if favourite {
            format!("/statuses/{}/favourite", id)
        } else {
            format!("/statuses/{}/unfavourite", id)
        };
        let response = self.request(reqwest::Method::POST, &path, None).await?;
        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MastotuiError::Api {
                status: response.status().as_u16(),
                message: text,
            });
        }
        Ok(response.json().await?)
    }

    /// Get a single status by id (for thread context). r[toot.view-detail]
    pub async fn get_status(&self, id: &str) -> Result<Status> {
        let path = format!("/statuses/{}", id);
        let response = self.request(reqwest::Method::GET, &path, None).await?;
        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MastotuiError::Api {
                status: response.status().as_u16(),
                message: text,
            });
        }
        Ok(response.json().await?)
    }
}

/// Build a client from stored config and keyring. r[auth.login.use-stored-token]
pub fn client_from_stored_credentials(instance_url: &str) -> Result<Option<MastodonClient>> {
    let host = instance_host_from_url(instance_url)?;
    let token = get_access_token(&host)?;
    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    Ok(Some(MastodonClient::new(instance_url.to_string(), token)?))
}

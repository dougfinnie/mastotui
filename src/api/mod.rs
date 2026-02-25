//! Mastodon API client and OAuth 2.0 flow.

mod client;
mod oauth;
mod types;

pub use client::{client_from_stored_credentials, MastodonClient};
pub use oauth::{authorization_url, exchange_code_for_token, register_app_if_needed};
pub use types::{Account, Application, Card, Status, Visibility};

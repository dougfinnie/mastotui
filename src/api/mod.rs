//! Mastodon API client and OAuth 2.0 flow.

mod client;
mod oauth;
mod types;

pub use client::{client_from_stored_credentials, get_public_timeline, MastodonClient};
pub use oauth::{
    app_token_client_credentials, authorization_url, exchange_code_for_token,
    register_app_if_needed,
};
pub use types::{Account, Application, Card, List, Status, Visibility};

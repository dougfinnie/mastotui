//! Mastodon API response types (subset needed for MVP).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Application {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub id: String,
    pub display_name: String,
    pub username: String,
    #[serde(default)]
    pub acct: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Public,
    Unlisted,
    Private,
    Direct,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    pub id: String,
    pub uri: String,
    pub content: String,
    pub account: Account,
    pub created_at: String,
    pub visibility: Option<Visibility>,
    pub reblog: Option<Box<Self>>,
    pub favourited: Option<bool>,
    pub reblogged: Option<bool>,
    pub in_reply_to_id: Option<String>,
    pub in_reply_to_account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Card {
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

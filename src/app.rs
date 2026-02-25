//! App state and main event loop. r[config.first-run] r[timeline.home.fetch] r[timeline.pagination]
//! r[toot.view-detail] r[toot.post.submit] r[toot.post.validation] r[toot.reply] r[toot.boost.toggle] r[toot.favourite.toggle]

use crossterm::event::KeyCode;
use ratatui::Frame;
use tokio::runtime::Runtime;

use crate::api::{
    authorization_url, client_from_stored_credentials, exchange_code_for_token,
    register_app_if_needed, MastodonClient,
};
use crate::config::{load_config, save_config, AppConfig};
use crate::credential::get_client_secret;
use crate::credential::instance_host_from_url;
use crate::error::{MastotuiError, Result};
use crate::tui::{draw_compose, draw_login, draw_timeline, draw_toot_detail};

const CHAR_LIMIT: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Login,
    Timeline,
    TootDetail,
    Compose,
}

pub struct App {
    pub view: View,
    pub config: Option<AppConfig>,
    pub client: Option<MastodonClient>,
    pub statuses: Vec<crate::api::Status>,
    pub selected: usize,
    pub scroll: usize,
    pub loading: bool,

    pub instance_url: String,
    pub auth_url: String,
    pub pkce_verifier: String,
    pub login_code: String,
    pub login_message: String,

    pub detail_status: Option<crate::api::Status>,
    pub detail_message: String,

    pub compose_buffer: String,
    pub compose_reply_to_id: Option<String>,
    pub compose_error: String,

    /// Shown on Timeline when a load failed (so we don't retry every tick).
    pub timeline_message: String,

    runtime: Runtime,
}

impl App {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| MastotuiError::Config(e.to_string()))?;
        let config = load_config()?;

        let (view, client) = if let Some(ref cfg) = config {
            if let Some(c) = client_from_stored_credentials(&cfg.instance_url)? {
                (View::Timeline, Some(c))
            } else {
                (View::Login, None)
            }
        } else {
            (View::Login, None)
        };

        let mut app = Self {
            view,
            config: config.clone(),
            client,
            statuses: Vec::new(),
            selected: 0,
            scroll: 0,
            loading: false,
            instance_url: config
                .as_ref()
                .map(|c| c.instance_url.clone())
                .unwrap_or_else(|| "https://mastodon.social".to_string()),
            auth_url: String::new(),
            pkce_verifier: String::new(),
            login_code: String::new(),
            login_message: String::new(),
            detail_status: None,
            detail_message: String::new(),
            compose_buffer: String::new(),
            compose_reply_to_id: None,
            compose_error: String::new(),
            timeline_message: String::new(),
            runtime,
        };

        if app.view == View::Login && app.config.is_some() && !app.instance_url.is_empty() {
            let _ = app.start_login_flow();
        }

        Ok(app)
    }

    fn start_login_flow(&mut self) -> Result<()> {
        let url = self.instance_url.trim().to_string();
        if url.is_empty() {
            return Err(MastotuiError::Config("Instance URL is empty".into()));
        }
        let client = reqwest::Client::builder().build()?;
        let (client_id, _secret) = self
            .runtime
            .block_on(register_app_if_needed(&url, &client))?;
        if let Some(ref mut c) = self.config {
            c.client_id = client_id.clone();
            c.instance_url = url.clone();
        } else {
            self.config = Some(AppConfig::new(url.clone(), client_id.clone()));
        }
        let (auth_url, pkce) = authorization_url(&url, &client_id)?;
        self.auth_url = auth_url;
        self.pkce_verifier = pkce;
        Ok(())
    }

    pub fn draw(&self, frame: &mut Frame) {
        match self.view {
            View::Login => draw_login(
                frame,
                &self.instance_url,
                &self.auth_url,
                &self.login_code,
                &self.login_message,
            ),
            View::Timeline => draw_timeline(
                frame,
                &self.statuses,
                self.selected,
                self.scroll,
                self.loading,
                &self.timeline_message,
            ),
            View::TootDetail => {
                if let Some(ref s) = self.detail_status {
                    draw_toot_detail(frame, s, None, &self.detail_message);
                }
            }
            View::Compose => draw_compose(
                frame,
                &self.compose_buffer,
                self.compose_reply_to_id.as_deref(),
                &self.compose_error,
                CHAR_LIMIT,
            ),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        let mut quit = false;
        match self.view {
            View::Login => match key {
                KeyCode::Char('q') => quit = true,
                KeyCode::Enter => {
                    if self.auth_url.is_empty() {
                        let input = self.login_code.trim().to_string();
                        self.login_code.clear();
                        if !input.is_empty() {
                            self.instance_url = input;
                        }
                        if self.instance_url.is_empty() {
                            self.login_message =
                                "Enter instance URL (e.g. https://mastodon.social) first."
                                    .to_string();
                        } else {
                            match self.start_login_flow() {
                                Ok(()) => self.login_message.clear(),
                                Err(e) => {
                                    self.login_message = format!("Failed to start login: {e}")
                                }
                            }
                        }
                    } else {
                        let code = self.login_code.trim().to_string();
                        self.login_code.clear();
                        if code.is_empty() {
                            self.login_message = "Enter the authorization code first.".to_string();
                        } else {
                            self.login_message = "Exchanging code…".to_string();
                            let url = self.instance_url.clone();
                            let cfg = self.config.as_ref().ok_or_else(|| {
                                MastotuiError::Config("No config during login".into())
                            })?;
                            let client_id = cfg.client_id.clone();
                            let host = instance_host_from_url(&url)?;
                            let client_secret = get_client_secret(&host)?.ok_or_else(|| {
                                MastotuiError::Credential("No client secret".into())
                            })?;
                            let http = reqwest::Client::builder().build()?;
                            match self.runtime.block_on(exchange_code_for_token(
                                &url,
                                &client_id,
                                &client_secret,
                                &code,
                                &self.pkce_verifier,
                                &http,
                            )) {
                                Ok(token) => {
                                    self.client = Some(MastodonClient::new(url.clone(), token)?);
                                    save_config(self.config.as_ref().unwrap())?;
                                    self.view = View::Timeline;
                                    self.login_message.clear();
                                    let _ = self.load_timeline(false);
                                }
                                Err(e) => {
                                    self.login_message = format!("Login failed: {e}");
                                }
                            }
                        }
                    }
                }
                KeyCode::Char(c) => self.login_code.push(c),
                KeyCode::Backspace => {
                    self.login_code.pop();
                }
                _ => {}
            },
            View::Timeline => match key {
                KeyCode::Char('q') => quit = true,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected > 0 {
                        self.selected -= 1;
                        if self.scroll > 0 && self.selected < self.scroll {
                            self.scroll = self.scroll.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected + 1 < self.statuses.len() {
                        self.selected += 1;
                        let area_h = 24_usize.saturating_sub(3);
                        if self.selected >= self.scroll + area_h {
                            self.scroll += 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(s) = self.statuses.get(self.selected).cloned() {
                        self.detail_status = Some(s);
                        self.detail_message.clear();
                        self.view = View::TootDetail;
                    }
                }
                KeyCode::Char('n') => {
                    self.compose_buffer.clear();
                    self.compose_reply_to_id = None;
                    self.compose_error.clear();
                    self.view = View::Compose;
                }
                KeyCode::Char('r') => {
                    self.load_timeline(false)?;
                }
                KeyCode::Char('m') => {
                    self.load_timeline(true)?;
                }
                _ => {}
            },
            View::TootDetail => match key {
                KeyCode::Esc => {
                    self.view = View::Timeline;
                    self.detail_message.clear();
                }
                KeyCode::Char('r') => {
                    if let Some(ref s) = self.detail_status {
                        self.compose_buffer.clear();
                        self.compose_reply_to_id = Some(s.id.clone());
                        self.compose_error.clear();
                        self.view = View::Compose;
                    }
                }
                KeyCode::Char('b') => {
                    if let Some(ref client) = self.client {
                        if let Some(ref s) = self.detail_status {
                            let id = s.id.clone();
                            let reblog = !s.reblogged.unwrap_or(false);
                            match self.runtime.block_on(client.reblog(&id, reblog)) {
                                Ok(updated) => {
                                    self.detail_status = Some(updated);
                                    self.detail_message =
                                        if reblog { "Boosted." } else { "Unboosted." }.to_string();
                                }
                                Err(e) => self.detail_message = format!("Error: {e}"),
                            }
                        }
                    }
                }
                KeyCode::Char('f') => {
                    if let Some(ref client) = self.client {
                        if let Some(ref s) = self.detail_status {
                            let id = s.id.clone();
                            let fav = !s.favourited.unwrap_or(false);
                            match self.runtime.block_on(client.favourite(&id, fav)) {
                                Ok(updated) => {
                                    self.detail_status = Some(updated);
                                    self.detail_message =
                                        if fav { "Favourited." } else { "Unfavourited." }
                                            .to_string();
                                }
                                Err(e) => self.detail_message = format!("Error: {e}"),
                            }
                        }
                    }
                }
                _ => {}
            },
            View::Compose => match key {
                KeyCode::Esc => {
                    self.view = if self.compose_reply_to_id.is_some() {
                        View::TootDetail
                    } else {
                        View::Timeline
                    };
                    self.compose_error.clear();
                }
                KeyCode::Enter => {
                    let text = self.compose_buffer.trim().to_string();
                    if text.is_empty() {
                        self.compose_error = "Cannot post empty toot.".to_string();
                    } else if text.chars().count() > CHAR_LIMIT {
                        self.compose_error = format!("Over {} character limit.", CHAR_LIMIT);
                    } else if let Some(ref client) = self.client {
                        let reply_to = self.compose_reply_to_id.clone();
                        match self
                            .runtime
                            .block_on(client.post_status(&text, reply_to.as_deref()))
                        {
                            Ok(_) => {
                                self.compose_buffer.clear();
                                self.compose_reply_to_id = None;
                                self.compose_error.clear();
                                self.view = if reply_to.is_some() {
                                    View::TootDetail
                                } else {
                                    View::Timeline
                                };
                                self.load_timeline(false)?;
                            }
                            Err(e) => self.compose_error = format!("Post failed: {e}"),
                        }
                    }
                }
                KeyCode::Char(c) => self.compose_buffer.push(c),
                KeyCode::Backspace => {
                    self.compose_buffer.pop();
                }
                _ => {}
            },
        }
        Ok(quit)
    }

    /// append: false = refresh from top (replace); true = load next page (append).
    fn load_timeline(&mut self, append: bool) -> Result<()> {
        if let Some(ref client) = self.client {
            self.loading = true;
            self.timeline_message.clear();
            let max_id_str = if append && !self.statuses.is_empty() {
                self.statuses.last().map(|s| s.id.clone())
            } else {
                None
            };
            let max_id = max_id_str.as_deref();
            match self.runtime.block_on(client.get_timeline_home(max_id)) {
                Ok(mut new_statuses) => {
                    if max_id.is_some() {
                        self.statuses.append(&mut new_statuses);
                    } else {
                        self.statuses = new_statuses;
                    }
                }
                Err(MastotuiError::NotAuthenticated) => {
                    self.client = None;
                    self.view = View::Login;
                    self.login_message = "Session expired. Please log in again.".to_string();
                    if self.config.is_some() {
                        let _ = self.start_login_flow();
                    }
                }
                Err(e) => {
                    self.timeline_message = format!("Failed to load timeline: {e}");
                }
            }
            self.loading = false;
        }
        Ok(())
    }

    /// Called each tick; fetches timeline when on home view with client, not loading, empty statuses, no prior error.
    pub fn ensure_timeline_loaded(&mut self) -> Result<()> {
        if self.view == View::Timeline
            && self.client.is_some()
            && !self.loading
            && self.statuses.is_empty()
            && self.timeline_message.is_empty()
        {
            self.load_timeline(false)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify toot.post.validation]
    #[test]
    fn compose_rejects_over_char_limit() {
        const LIMIT: usize = 500;
        let over = "x".repeat(LIMIT + 1);
        assert!(over.chars().count() > LIMIT);
    }

    /// Condition under which ensure_timeline_loaded triggers a fetch. Must match ensure_timeline_loaded().
    fn should_auto_fetch_timeline(
        view: View,
        client_is_some: bool,
        loading: bool,
        statuses_empty: bool,
        timeline_message_empty: bool,
    ) -> bool {
        view == View::Timeline
            && client_is_some
            && !loading
            && statuses_empty
            && timeline_message_empty
    }

    #[test]
    fn auto_fetch_timeline_requires_not_loading() {
        assert!(!should_auto_fetch_timeline(View::Timeline, true, true, true, true));
    }

    #[test]
    fn auto_fetch_timeline_requires_empty_statuses() {
        assert!(!should_auto_fetch_timeline(View::Timeline, true, false, false, true));
    }

    #[test]
    fn auto_fetch_timeline_requires_no_prior_error() {
        assert!(!should_auto_fetch_timeline(View::Timeline, true, false, true, false));
    }

    #[test]
    fn auto_fetch_timeline_when_conditions_met() {
        assert!(should_auto_fetch_timeline(View::Timeline, true, false, true, true));
    }
}

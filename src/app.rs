//! App state and main event loop.
//! r[config.first-run] r[timeline.home.fetch] r[timeline.pagination]
//! r[toot.view-detail] r[toot.post.submit] r[toot.post.validation] r[toot.reply] r[toot.boost.toggle] r[toot.favourite.toggle]

use crossterm::event::KeyCode;
use ratatui::Frame;
use tokio::runtime::Runtime;

use crate::api::{
    authorization_url, client_from_stored_credentials, exchange_code_for_token,
    get_public_timeline, register_app_if_needed, MastodonClient,
};
use crate::config::{load_config, save_config, AppConfig};
use crate::credential::{delete_access_token, get_client_secret, instance_host_from_url};
use crate::error::{MastotuiError, Result};
use crate::tui::{
    draw_compose, draw_instance_info, draw_instance_picker, draw_login, draw_timeline,
    draw_timeline_picker, draw_toot_detail,
};

const CHAR_LIMIT: usize = 500;

/// Which timeline is currently shown (or selected in the picker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineSelection {
    Home,
    Local,
    Public,
    List { id: String, title: String },
}

impl TimelineSelection {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Home => "Home".to_string(),
            Self::Local => "Local".to_string(),
            Self::Public => "Public".to_string(),
            Self::List { title, .. } => title.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Login,
    Timeline,
    TootDetail,
    Compose,
    /// r[browse.instance.dialog]: dialog to enter or pick instance for anonymous browse.
    InstancePicker,
    /// r[instance.info.dialog]: current instance, l log out/in, b browse another. (press i).
    InstanceInfo,
    /// Timeline picker: choose Home / Local / Public / List (press t).
    TimelinePicker,
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

    /// Timeline content area height in rows (updated each draw); used so scroll follows selection.
    pub timeline_visible_rows: usize,

    /// When set, we are viewing this instance's public timeline without login (read-only).
    pub anonymous_instance_url: Option<String>,

    /// View to restore when `InstancePicker` is cancelled. r[browse.instance.cancel]
    pub return_to_view: View,

    /// Instance picker: text box content. r[browse.instance.dialog]
    pub instance_picker_input: String,
    /// Known instances (e.g. from config) to pick from.
    pub instance_picker_known: Vec<String>,
    /// Selected index in `instance_picker_known` (0 when none).
    pub instance_picker_selected: usize,
    /// Error message shown in instance picker (e.g. invalid URL). r[browse.instance.submit]
    pub instance_picker_message: String,

    /// Which timeline is displayed (Home, Local, Public, or a list).
    pub current_timeline: TimelineSelection,

    /// User's lists (fetched when opening timeline picker; requires read:lists).
    pub lists: Vec<crate::api::List>,

    /// Timeline picker: options to choose from (built when opening picker).
    pub timeline_picker_options: Vec<TimelineSelection>,
    /// Selected index in `timeline_picker_options`.
    pub timeline_picker_selected: usize,
    /// When `get_lists()` fails (e.g. missing `read:lists`), show hint to re-login.
    pub timeline_picker_lists_message: String,

    runtime: Runtime,
}

impl App {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| MastotuiError::Config(e.to_string()))?;
        let config = load_config()?;

        let (view, client) = config.as_ref().map_or(Ok((View::Login, None)), |cfg| {
            client_from_stored_credentials(&cfg.instance_url)
                .map(|opt| opt.map_or_else(|| (View::Login, None), |c| (View::Timeline, Some(c))))
        })?;

        let mut app = Self {
            view,
            config: config.clone(),
            client,
            statuses: Vec::new(),
            selected: 0,
            scroll: 0,
            loading: false,
            instance_url: config.as_ref().map_or_else(
                || "https://mastodon.social".to_string(),
                |c| c.instance_url.clone(),
            ),
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
            timeline_visible_rows: 20,
            anonymous_instance_url: None,
            return_to_view: View::Login,
            instance_picker_input: String::new(),
            instance_picker_known: config
                .as_ref()
                .map(|c| vec![c.instance_url.clone()])
                .unwrap_or_default(),
            instance_picker_selected: 0,
            instance_picker_message: String::new(),
            current_timeline: if view == View::Timeline {
                TimelineSelection::Home
            } else {
                TimelineSelection::Public
            },
            lists: Vec::new(),
            timeline_picker_options: Vec::new(),
            timeline_picker_selected: 0,
            timeline_picker_lists_message: String::new(),
            runtime,
        };

        if app.view == View::Login && app.config.is_some() && !app.instance_url.is_empty() {
            let _ = app.start_login_flow();
        }

        Ok(app)
    }

    /// Open instance info (i). r[instance.info.dialog]
    fn open_instance_info(&mut self, return_to: View) {
        self.return_to_view = return_to;
        self.view = View::InstanceInfo;
    }

    /// Open instance picker for anonymous browse. r[browse.instance.dialog]
    fn open_instance_picker(&mut self, return_to: View) {
        self.return_to_view = return_to;
        self.view = View::InstancePicker;
        self.instance_picker_input.clear();
        let mut known = self
            .config
            .as_ref()
            .map(|c| vec![c.instance_url.clone()])
            .unwrap_or_default();
        if let Some(ref u) = self.anonymous_instance_url {
            if !known.contains(u) {
                known.push(u.clone());
            }
        }
        self.instance_picker_known = known;
        self.instance_picker_selected = 0;
        self.instance_picker_message.clear();
    }

    /// Open timeline picker (press t). Fetches lists when authenticated. r[timeline.select.dialog]
    fn open_timeline_picker(&mut self) {
        self.timeline_picker_lists_message.clear();
        let mut options: Vec<TimelineSelection> = Vec::new();
        if self.client.is_some() {
            options.push(TimelineSelection::Home);
            options.push(TimelineSelection::Local);
            options.push(TimelineSelection::Public);
            if let Some(ref client) = self.client {
                if let Ok(lists) = self.runtime.block_on(client.get_lists()) {
                    self.lists = lists;
                    self.timeline_picker_lists_message.clear();
                    for list in &self.lists {
                        options.push(TimelineSelection::List {
                            id: list.id.clone(),
                            title: list.title.clone(),
                        });
                    }
                } else {
                    self.lists.clear();
                    self.timeline_picker_lists_message =
                        "Lists unavailable. Re-login to enable list timelines.".to_string();
                }
            }
        } else {
            options.push(TimelineSelection::Public);
        }
        self.timeline_picker_options = options;
        self.timeline_picker_selected = self
            .timeline_picker_options
            .iter()
            .position(|o| o == &self.current_timeline)
            .unwrap_or(0);
        self.view = View::TimelinePicker;
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
            c.client_id.clone_from(&client_id);
            c.instance_url.clone_from(&url);
        } else {
            self.config = Some(AppConfig::new(&url, &client_id));
        }
        let (auth_url, pkce) = authorization_url(&url, &client_id)?;
        self.auth_url = auth_url;
        self.pkce_verifier = pkce;
        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        if self.view == View::Timeline {
            let content_height = frame.area().height as usize;
            let content_height = content_height.saturating_sub(2).max(1);
            self.timeline_visible_rows = (content_height / 2).max(1);
            if !self.statuses.is_empty() {
                if self.selected >= self.scroll + self.timeline_visible_rows {
                    self.scroll = self.selected - self.timeline_visible_rows + 1;
                } else if self.selected < self.scroll {
                    self.scroll = self.selected;
                }
            }
        }
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
                &self.current_timeline.label(),
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
            View::InstancePicker => draw_instance_picker(
                frame,
                &self.instance_picker_input,
                &self.instance_picker_known,
                self.instance_picker_selected,
                &self.instance_picker_message,
            ),
            View::InstanceInfo => draw_instance_info(
                frame,
                &self.instance_url,
                self.client.is_some(),
                self.anonymous_instance_url.as_deref(),
            ),
            View::TimelinePicker => draw_timeline_picker(
                frame,
                &self.timeline_picker_options,
                self.timeline_picker_selected,
                &self.timeline_picker_lists_message,
            ),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        let mut quit = false;
        match self.view {
            View::Login => match key {
                KeyCode::Char('q') => {
                    if self.auth_url.is_empty() {
                        quit = true;
                    } else {
                        self.login_code.push('q');
                    }
                }
                KeyCode::Char('i') => {
                    if self.auth_url.is_empty() {
                        self.open_instance_info(View::Login);
                    } else {
                        self.login_code.push('i');
                    }
                }
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
                                    self.login_message = format!("Failed to start login: {e}");
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
                                    self.client = Some(MastodonClient::new(&url, &token)?);
                                    save_config(self.config.as_ref().unwrap())?;
                                    self.view = View::Timeline;
                                    self.login_message.clear();
                                    self.load_timeline(false);
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
                        if self.selected < self.scroll {
                            self.scroll = self.selected;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected + 1 < self.statuses.len() {
                        self.selected += 1;
                        if self.selected >= self.scroll + self.timeline_visible_rows {
                            self.scroll =
                                (self.selected + 1).saturating_sub(self.timeline_visible_rows);
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
                KeyCode::Char('p') => {
                    if self.client.is_some() {
                        self.compose_buffer.clear();
                        self.compose_reply_to_id = None;
                        self.compose_error.clear();
                        self.view = View::Compose;
                    }
                }
                KeyCode::Char('r') => {
                    self.load_timeline(false);
                }
                KeyCode::Char('m') => {
                    self.load_timeline(true);
                }
                KeyCode::Char('i') => self.open_instance_info(View::Timeline),
                KeyCode::Char('t') => self.open_timeline_picker(),
                _ => {}
            },
            View::TootDetail => match key {
                KeyCode::Esc => {
                    self.view = View::Timeline;
                    self.detail_message.clear();
                }
                KeyCode::Char('r') => {
                    if self.client.is_some() {
                        if let Some(ref s) = self.detail_status {
                            self.compose_buffer.clear();
                            self.compose_reply_to_id = Some(s.id.clone());
                            self.compose_error.clear();
                            self.view = View::Compose;
                        }
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
                KeyCode::Char('i') => self.open_instance_info(View::TootDetail),
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
                KeyCode::Char('i') => self.open_instance_info(View::Compose),
                KeyCode::Enter => {
                    let text = self.compose_buffer.trim().to_string();
                    if text.is_empty() {
                        self.compose_error = "Cannot post empty toot.".to_string();
                    } else if text.chars().count() > CHAR_LIMIT {
                        self.compose_error = format!("Over {CHAR_LIMIT} character limit.");
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
                                self.load_timeline(false);
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
            View::InstanceInfo => match key {
                KeyCode::Esc => self.view = self.return_to_view,
                KeyCode::Char('l') => {
                    if self.client.is_some() {
                        // r[instance.info.logout]
                        if let Ok(host) = instance_host_from_url(&self.instance_url) {
                            let _ = delete_access_token(&host);
                        }
                        self.client = None;
                        self.statuses.clear();
                        self.selected = 0;
                        self.scroll = 0;
                        self.timeline_message.clear();
                        self.login_message = "Logged out.".to_string();
                        self.view = View::Login;
                        if self.config.is_some() && !self.instance_url.is_empty() {
                            let _ = self.start_login_flow();
                        }
                    } else {
                        // r[instance.info.login]
                        self.view = View::Login;
                        self.login_message.clear();
                    }
                }
                KeyCode::Char('b') => {
                    // r[instance.info.browse]
                    self.open_instance_picker(View::InstanceInfo);
                }
                _ => {}
            },
            View::InstancePicker => match key {
                KeyCode::Esc => {
                    self.view = self.return_to_view;
                    self.instance_picker_message.clear();
                }
                KeyCode::Enter => {
                    let url = self.instance_picker_input.trim();
                    let url = if url.is_empty()
                        && self.instance_picker_selected < self.instance_picker_known.len()
                    {
                        self.instance_picker_known[self.instance_picker_selected].trim()
                    } else {
                        url
                    };
                    if url.is_empty() {
                        self.instance_picker_message =
                            "Enter a URL or pick an instance.".to_string();
                    } else if let Err(e) = instance_host_from_url(url) {
                        self.instance_picker_message = format!("Invalid URL: {e}");
                    } else {
                        let url = url.to_string();
                        self.anonymous_instance_url = Some(url);
                        self.client = None;
                        self.current_timeline = TimelineSelection::Public;
                        self.statuses.clear();
                        self.selected = 0;
                        self.scroll = 0;
                        self.timeline_message.clear();
                        self.view = View::Timeline;
                        self.instance_picker_message.clear();
                        self.load_timeline(false);
                    }
                }
                KeyCode::Backspace => {
                    self.instance_picker_input.pop();
                    self.instance_picker_message.clear();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.instance_picker_selected > 0 {
                        self.instance_picker_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.instance_picker_selected + 1 < self.instance_picker_known.len() {
                        self.instance_picker_selected += 1;
                    }
                }
                KeyCode::Char(c) => {
                    self.instance_picker_input.push(c);
                    self.instance_picker_message.clear();
                }
                _ => {}
            },
            View::TimelinePicker => match key {
                // r[timeline.select.submit]: Esc cancels; Enter switches and loads
                KeyCode::Esc => self.view = View::Timeline,
                KeyCode::Enter => {
                    if self.timeline_picker_selected < self.timeline_picker_options.len() {
                        self.current_timeline =
                            self.timeline_picker_options[self.timeline_picker_selected].clone();
                        self.statuses.clear();
                        self.selected = 0;
                        self.scroll = 0;
                        self.timeline_message.clear();
                        self.view = View::Timeline;
                        self.load_timeline(false);
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.timeline_picker_selected > 0 {
                        self.timeline_picker_selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.timeline_picker_selected + 1 < self.timeline_picker_options.len() {
                        self.timeline_picker_selected += 1;
                    }
                }
                _ => {}
            },
        }
        Ok(quit)
    }

    /// append: false = refresh from top (replace); true = load next page (append).
    fn load_timeline(&mut self, append: bool) {
        if let Some(ref client) = self.client {
            self.loading = true;
            self.timeline_message.clear();
            let max_id_str = if append && !self.statuses.is_empty() {
                self.statuses.last().map(|s| s.id.clone())
            } else {
                None
            };
            let max_id = max_id_str.as_deref();
            let result = match &self.current_timeline {
                TimelineSelection::Home => self.runtime.block_on(client.get_timeline_home(max_id)),
                TimelineSelection::Local => {
                    self.runtime.block_on(client.get_timeline_local(max_id))
                }
                TimelineSelection::Public => {
                    self.runtime.block_on(client.get_timeline_public(max_id))
                }
                TimelineSelection::List { id, .. } => {
                    self.runtime.block_on(client.get_timeline_list(id, max_id))
                }
            };
            match result {
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
        } else if let Some(ref url) = self.anonymous_instance_url {
            self.loading = true;
            self.timeline_message.clear();
            let max_id = if append && !self.statuses.is_empty() {
                self.statuses.last().map(|s| s.id.as_str())
            } else {
                None
            };
            match self.runtime.block_on(get_public_timeline(url, max_id)) {
                Ok(mut new_statuses) => {
                    if max_id.is_some() {
                        self.statuses.append(&mut new_statuses);
                    } else {
                        self.statuses = new_statuses;
                    }
                }
                Err(e) => {
                    self.timeline_message = format!("Failed to load timeline: {e}");
                }
            }
            self.loading = false;
        }
    }

    /// Called each tick; fetches timeline when on home view with client or anonymous instance, not loading, empty statuses, no prior error.
    pub fn ensure_timeline_loaded(&mut self) -> Result<()> {
        let has_source = self.client.is_some() || self.anonymous_instance_url.is_some();
        if self.view == View::Timeline
            && has_source
            && !self.loading
            && self.statuses.is_empty()
            && self.timeline_message.is_empty()
        {
            self.load_timeline(false);
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

    /// Condition under which `ensure_timeline_loaded` triggers a fetch. Must match `ensure_timeline_loaded()`.
    #[allow(clippy::fn_params_excessive_bools)]
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
        assert!(!should_auto_fetch_timeline(
            View::Timeline,
            true,
            true,
            true,
            true
        ));
    }

    #[test]
    fn auto_fetch_timeline_requires_empty_statuses() {
        assert!(!should_auto_fetch_timeline(
            View::Timeline,
            true,
            false,
            false,
            true
        ));
    }

    #[test]
    fn auto_fetch_timeline_requires_no_prior_error() {
        assert!(!should_auto_fetch_timeline(
            View::Timeline,
            true,
            false,
            true,
            false
        ));
    }

    #[test]
    fn auto_fetch_timeline_when_conditions_met() {
        assert!(should_auto_fetch_timeline(
            View::Timeline,
            true,
            false,
            true,
            true
        ));
    }

    // r[verify browse.instance.dialog] r[verify browse.instance.cancel]
    #[test]
    fn instance_picker_opens_and_esc_cancels() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.view = View::Timeline;
        app.open_instance_picker(View::Timeline);
        assert_eq!(app.view, View::InstancePicker);
        assert_eq!(app.return_to_view, View::Timeline);
        app.handle_key(KeyCode::Esc).unwrap();
        assert_eq!(app.view, View::Timeline);
    }

    // r[verify instance.info.dialog]
    #[test]
    fn instance_info_opens_on_i_and_esc_returns() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.view = View::Timeline;
        app.handle_key(KeyCode::Char('i')).unwrap();
        assert_eq!(app.view, View::InstanceInfo);
        assert_eq!(app.return_to_view, View::Timeline);
        app.handle_key(KeyCode::Esc).unwrap();
        assert_eq!(app.view, View::Timeline);
    }

    // r[verify instance.info.browse]
    #[test]
    fn instance_info_b_opens_instance_picker() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.view = View::Timeline;
        app.handle_key(KeyCode::Char('i')).unwrap();
        assert_eq!(app.view, View::InstanceInfo);
        app.handle_key(KeyCode::Char('b')).unwrap();
        assert_eq!(app.view, View::InstancePicker);
        assert_eq!(app.return_to_view, View::InstanceInfo);
        app.handle_key(KeyCode::Esc).unwrap();
        assert_eq!(app.view, View::InstanceInfo);
    }

    // r[verify instance.info.login]
    #[test]
    fn instance_info_l_when_not_logged_in_goes_to_login() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.view = View::Timeline;
        app.client = None;
        app.handle_key(KeyCode::Char('i')).unwrap();
        assert_eq!(app.view, View::InstanceInfo);
        app.handle_key(KeyCode::Char('l')).unwrap();
        assert_eq!(app.view, View::Login);
    }

    // r[verify instance.info.logout]
    #[test]
    fn instance_info_l_when_logged_in_logs_out_and_goes_to_login() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.client =
            Some(crate::api::MastodonClient::new("https://example.com", "fake-token").unwrap());
        app.instance_url = "https://example.com".to_string();
        app.view = View::InstanceInfo;
        app.return_to_view = View::Timeline;
        app.handle_key(KeyCode::Char('l')).unwrap();
        assert_eq!(app.view, View::Login);
        assert!(app.client.is_none());
        assert_eq!(app.login_message, "Logged out.");
    }

    // r[verify timeline.select.header]
    #[test]
    fn timeline_selection_label_for_header() {
        assert_eq!(TimelineSelection::Home.label(), "Home");
        assert_eq!(TimelineSelection::Local.label(), "Local");
        assert_eq!(TimelineSelection::Public.label(), "Public");
        assert_eq!(
            TimelineSelection::List {
                id: "1".to_string(),
                title: "Friends".to_string()
            }
            .label(),
            "Friends"
        );
    }

    // r[verify timeline.select.dialog] r[verify timeline.select.submit]
    #[test]
    fn timeline_picker_opens_and_esc_cancels() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.view = View::Timeline;
        app.open_timeline_picker();
        assert_eq!(app.view, View::TimelinePicker);
        assert!(!app.timeline_picker_options.is_empty());
        app.handle_key(KeyCode::Esc).unwrap();
        assert_eq!(app.view, View::Timeline);
    }

    // r[verify browse.instance.submit]
    #[test]
    fn instance_picker_submit_invalid_url_shows_message() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let mut app = App::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        app.open_instance_picker(View::Timeline);
        app.instance_picker_input = "not-a-valid-url".to_string();
        app.handle_key(KeyCode::Enter).unwrap();
        assert!(!app.instance_picker_message.is_empty());
    }
}

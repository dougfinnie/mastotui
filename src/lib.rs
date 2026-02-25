//! mastotui — TUI client for Mastodon.

#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::too_long_first_doc_paragraph
)]

pub mod api;
pub mod app;
pub mod config;
pub mod credential;
pub mod error;
pub mod tui;

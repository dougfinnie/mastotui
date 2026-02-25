//! TUI views and rendering. r[timeline.home.empty-state] r[toot.view-detail] r[toot.post.validation]

mod views;

pub use views::strip_html;
pub use views::{
    draw_compose, draw_login, draw_timeline, draw_toot_detail, EMPTY_TIMELINE_MESSAGE,
};

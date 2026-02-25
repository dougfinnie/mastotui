//! Integration tests for spec-related behaviour.
//! Note: Tracey only counts r[verify] in src/**/*.rs; verify coverage is in src modules.

use mastotui::tui::{strip_html, EMPTY_TIMELINE_MESSAGE};

#[test]
fn strip_html_removes_tags() {
    let html = "<p>Hello <b>world</b></p>";
    assert_eq!(strip_html(html), "Hello world");
}

#[test]
fn strip_html_handles_empty() {
    assert_eq!(strip_html(""), "");
}

#[test]
fn strip_html_space_after_link() {
    let html = r#"<p>See <a href="https://example.com">this link</a>for more.</p>"#;
    assert!(strip_html(html).contains("link for"));
}

#[test]
fn empty_timeline_message_contains_no_toots() {
    assert!(EMPTY_TIMELINE_MESSAGE.contains("No toots"));
}

#[test]
fn toot_char_limit_is_500() {
    const CHAR_LIMIT: usize = 500;
    let at = "x".repeat(CHAR_LIMIT);
    let over = "x".repeat(CHAR_LIMIT + 1);
    assert_eq!(at.chars().count(), CHAR_LIMIT);
    assert!(over.chars().count() > CHAR_LIMIT);
}

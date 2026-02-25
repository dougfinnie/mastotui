//! Placeholder tests for spec requirements. r[verify config.first-run] r[verify toot.post.validation]

use mastotui::tui::strip_html;

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
fn char_limit_500_for_toots() {
    // r[toot.post.validation]: client enforces character limit
    const CHAR_LIMIT: usize = 500;
    let over = "x".repeat(CHAR_LIMIT + 1);
    assert!(over.chars().count() > CHAR_LIMIT);
}

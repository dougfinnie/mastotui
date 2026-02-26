//! TUI view rendering. r[timeline.home.empty-state] r[toot.view-detail] r[toot.post.validation]

use hyperrat::Link;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::api::{Account, Status};

/// Strip HTML tags from Mastodon content for plain-text display.
///
/// Ensures a space after hyperlinks so "link</a>next" becomes "link next".
/// Block tags like </p> and <br> become newlines so content keeps paragraph breaks.
#[must_use]
pub fn strip_html(html: &str) -> String {
    let s = html
        .replace("</a>", "</a> ")
        .replace("</p>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let fragment = scraper::Html::parse_fragment(&s);
    fragment
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

/// r[instance.info.dialog]: instance info (press i): current instance, l log out/in, b browse another.
pub fn draw_instance_info(
    frame: &mut Frame,
    instance_url: &str,
    is_logged_in: bool,
    anonymous_instance_url: Option<&str>,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    let title = Paragraph::new("Instance").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let display_url: &str = if instance_url.trim().is_empty() {
        anonymous_instance_url.unwrap_or("(none)")
    } else {
        instance_url.trim()
    };
    let mut lines = vec![
        Line::from(Span::styled(
            "Instance: ",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ".to_string() + display_url),
        Line::from(""),
    ];
    let status = if is_logged_in {
        "Logged in."
    } else if anonymous_instance_url.is_some() {
        "Browsing anonymously (public timeline only)."
    } else {
        "Not logged in."
    };
    lines.push(Line::from(Span::styled(
        status,
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "l: Log out / Log in",
        Style::default().fg(Color::Cyan),
    )));
    lines.push(Line::from(Span::styled(
        "b: Browse another instance (anonymous)",
        Style::default().fg(Color::Cyan),
    )));
    let block = Block::default().borders(Borders::ALL).title(" Instance ");
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, chunks[1]);

    let help = Line::from(Span::styled(
        " [l] log out/in  [b] browse another  [Esc] back ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

/// r[browse.instance.dialog]: instance picker for anonymous browse.
pub fn draw_instance_picker(
    frame: &mut Frame,
    input: &str,
    known: &[String],
    selected: usize,
    message: &str,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(area);

    let title = Paragraph::new("Browse instance (anonymous)").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let mut lines = vec![Line::from(Span::styled(
        "Instance URL: ".to_string() + input + "▌",
        Style::default().fg(Color::Green),
    ))];
    if !known.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Known instances (↑/↓ to select, Enter to use):",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for (i, url) in known.iter().enumerate() {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(format!("  {url}"), style)));
        }
    }
    if !message.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(Color::Yellow),
        )));
    }
    let block = Block::default().borders(Borders::ALL).title(" Instance ");
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, chunks[1]);

    let help = Line::from(Span::styled(
        " [Enter] open public timeline  [Esc] cancel ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

/// Timeline picker dialog (press t): list of timeline options.
pub fn draw_timeline_picker(
    frame: &mut Frame,
    options: &[crate::app::TimelineSelection],
    selected: usize,
    lists_message: &str,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);

    let title = Paragraph::new("Select timeline").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let mut lines = vec![Line::from(Span::styled(
        "[↑]/[↓] or [j]/[k]: move  [Enter] select",
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    for (i, opt) in options.iter().enumerate() {
        let style = if i == selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", opt.label()),
            style,
        )));
    }
    if !lists_message.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            lists_message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::DIM),
        )));
    }
    let block = Block::default().borders(Borders::ALL).title(" Timeline ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, chunks[1]);

    let help = Line::from(Span::styled(
        " [Enter] switch  [Esc] cancel ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

/// r[config.first-run]: login / add instance screen.
pub fn draw_login(
    frame: &mut Frame,
    instance_url: &str,
    auth_url: &str,
    code_buffer: &str,
    message: &str,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    let title = Paragraph::new("mastotui — Mastodon TUI").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let url_display = if auth_url.is_empty() && !code_buffer.is_empty() {
        code_buffer
    } else {
        instance_url
    };
    let mut lines = vec![
        Line::from("Instance URL: ".to_string() + url_display),
        Line::from(""),
    ];
    if auth_url.is_empty() {
        lines.push(Line::from(
            "Enter URL above and press Enter to open browser for auth.",
        ));
        lines.push(Line::from(""));
        if !code_buffer.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "Input: ".to_string() + code_buffer + "▌",
            Style::default().fg(Color::Green),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Open in browser:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(""));
    if !message.is_empty() {
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(Color::Yellow),
        )));
    }
    // Top/bottom only so selecting the URL doesn't include side borders (│) and paste gets contiguous text.
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .title(" Login ");
    if auth_url.is_empty() {
        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        frame.render_widget(para, chunks[1]);
    } else {
        frame.render_widget(block.clone(), chunks[1]);
        let inner = block.inner(chunks[1]);
        let inner_chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .split(inner);
        let above = inner_chunks[0];
        let link_area = inner_chunks[1];
        let below = inner_chunks[2];
        let para_above = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(para_above, above);
        let link = Link::new(auth_url, auth_url);
        frame.render_widget(link, link_area);
        let mut lines_below = vec![
            Line::from(""),
            Line::from("After authorizing, paste the code and press Enter."),
            Line::from(Span::styled(
                "Code: ".to_string() + code_buffer + "▌",
                Style::default().fg(Color::Green),
            )),
        ];
        lines_below.push(Line::from(""));
        let para_below = Paragraph::new(lines_below).wrap(Wrap { trim: true });
        frame.render_widget(para_below, below);
    }

    let help = Line::from(Span::styled(
        " [q] quit (when entering URL)  [Ctrl+Q] or [Ctrl+C]: quit from any screen ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

/// Message shown when home timeline is empty. r[timeline.home.empty-state]
pub const EMPTY_TIMELINE_MESSAGE: &str = "No toots — home timeline is empty.";

/// Lines for attachments that have alt text: one "[media: {alt}]" per description.
fn media_alt_lines(status: &Status) -> Vec<Line<'static>> {
    let s = display_status(status).0;
    s.media_attachments
        .iter()
        .filter_map(|m| {
            m.description
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty())
        })
        .map(|alt| Line::from(Span::styled(
            format!("[media: {alt}]"),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
        )))
        .collect()
}

/// For display: the status whose author/content we show, and the booster account if this is a reblog.
fn display_status(status: &Status) -> (&Status, Option<&Account>) {
    status.reblog.as_ref().map_or((status, None), |inner| {
        (inner.as_ref(), Some(&status.account))
    })
}

/// r[timeline.home.fetch] r[timeline.home.empty-state] r[timeline.select.header]: timeline list and current timeline label in header.
pub fn draw_timeline(
    frame: &mut Frame,
    timeline_label: &str,
    statuses: &[Status],
    selected: usize,
    scroll: usize,
    loading: bool,
    message: &str,
) {
    let area = frame.area();
    let block_title = format!(" {timeline_label}  [t] timeline  [i] instance ");
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    let content_area = chunks[1];
    if loading {
        let para = Paragraph::new("Loading…").block(
            Block::default()
                .borders(Borders::ALL)
                .title(block_title.as_str()),
        );
        frame.render_widget(para, content_area);
        return;
    }

    if !message.is_empty() {
        let para = Paragraph::new(message)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title.as_str()),
            )
            .style(Style::default().fg(Color::Red));
        frame.render_widget(para, content_area);
    } else if statuses.is_empty() {
        let para = Paragraph::new(EMPTY_TIMELINE_MESSAGE)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title.as_str()),
            )
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, content_area);
    } else {
        // Each timeline item is 2 lines (header, then content on new line); items that fit = height/2
        let visible = (content_area.height as usize / 2).max(1);
        let start = scroll.min(statuses.len().saturating_sub(visible));
        let end = (start + visible).min(statuses.len());
        let mut lines: Vec<Line> = Vec::with_capacity(2 * (end - start));
        for (i, s) in statuses[start..end].iter().enumerate() {
            let idx = start + i;
            let style = if idx == selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let (display_status, booster) = display_status(s);
            let account = &display_status.account;
            let display = account.display_name.as_str();
            let handle = if account.acct.is_empty() {
                account.username.as_str()
            } else {
                account.acct.as_str()
            };
            let header = format!("@{} · {}", handle, display_status.created_at);
            let booster_prefix = booster
                .map(|a| {
                    let h = if a.acct.is_empty() {
                        &a.username
                    } else {
                        &a.acct
                    };
                    format!("@{h} boosted · ")
                })
                .unwrap_or_default();
            let header_line = Line::from(vec![
                Span::styled(
                    format!(" {display} "),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Green),
                ),
                Span::styled(
                    booster_prefix,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::ITALIC),
                ),
                Span::styled(header, Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(header_line);
            let content = strip_html(&display_status.content);
            let content_preview = content.lines().next().unwrap_or(&content);
            let content_line = Line::from(Span::styled(
                content_preview.chars().take(80).collect::<String>(),
                style,
            ));
            lines.push(content_line);
            for media_line in media_alt_lines(s) {
                lines.push(media_line);
            }
        }
        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title.as_str()),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(para, content_area);
    }

    let status_line = Line::from(Span::styled(
        " [↑]/[↓]  [Enter]: open  [p]: post  [t]: timeline  [q]: quit  [r]: refresh ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(status_line), chunks[2]);
}

/// r[toot.view-detail]: single toot with full content and actions.
pub fn draw_toot_detail(
    frame: &mut Frame,
    status: &Status,
    _reply_id: Option<&str>,
    message: &str,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(area);

    let title = Paragraph::new(" Toot ").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let (display_status, booster) = display_status(status);
    let acc = &display_status.account;
    let header = format!("@{} · {}", acc.acct, display_status.created_at);
    let content = strip_html(&display_status.content);
    let mut lines = vec![];
    if let Some(b) = booster {
        let handle = if b.acct.is_empty() {
            &b.username
        } else {
            &b.acct
        };
        lines.push(Line::from(Span::styled(
            format!("Boosted by @{handle}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(""));
    }
    lines.extend_from_slice(&[
        Line::from(Span::styled(
            acc.display_name.as_str(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(header, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(content),
    ]);
    for media_line in media_alt_lines(status) {
        lines.push(media_line);
    }
    let block = Block::default().borders(Borders::ALL);
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, chunks[1]);

    if !message.is_empty() {
        let msg = Paragraph::new(message).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, chunks[2]);
    }

    let help = Line::from(Span::styled(
        " [b] boost  [f] favourite  [r] reply  [Esc] back ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[3]);
}

/// r[toot.post.submit] r[toot.post.validation]: compose new toot or reply.
pub fn draw_compose(
    frame: &mut Frame,
    buffer: &str,
    in_reply_to: Option<&str>,
    error_message: &str,
    char_limit: usize,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(area);

    let title = match in_reply_to {
        Some(_) => " Reply ",
        None => " New toot ",
    };
    let title_w = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title_w, chunks[0]);

    let len = buffer.chars().count();
    let over = len > char_limit;
    let count_str = format!("{len}/{char_limit}");
    let count_style = if over {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(count_str, count_style));
    let para = Paragraph::new(buffer)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, chunks[1]);

    if !error_message.is_empty() {
        let err = Paragraph::new(error_message).style(Style::default().fg(Color::Red));
        frame.render_widget(err, chunks[2]);
    }

    let help = Line::from(Span::styled(
        " [Enter] post  [Esc] cancel  [Ctrl+i] instance ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify timeline.home.empty-state]
    #[test]
    fn empty_timeline_message_shown_when_no_toots() {
        assert!(EMPTY_TIMELINE_MESSAGE.contains("No toots"));
    }
}

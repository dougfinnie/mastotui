//! TUI view rendering. r[timeline.home.empty-state] r[toot.view-detail] r[toot.post.validation]

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::api::Status;

/// Strip HTML tags from Mastodon content for plain-text display.
pub fn strip_html(html: &str) -> String {
    let fragment = scraper::Html::parse_fragment(html);
    fragment.root_element().text().collect::<Vec<_>>().join("")
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
            "Open in browser: ",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(auth_url.to_string()));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "After authorizing, paste the code and press Enter.",
        ));
        lines.push(Line::from(Span::styled(
            "Code: ".to_string() + code_buffer + "▌",
            Style::default().fg(Color::Green),
        )));
    }
    lines.push(Line::from(""));
    if !message.is_empty() {
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(Color::Yellow),
        )));
    }
    let block = Block::default().borders(Borders::ALL).title(" Login ");
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, chunks[1]);

    let help = Line::from(Span::styled("q: quit", Style::default().dim()));
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

/// r[timeline.home.fetch] r[timeline.home.empty-state]: timeline list.
pub fn draw_timeline(
    frame: &mut Frame,
    statuses: &[Status],
    selected: usize,
    scroll: usize,
    loading: bool,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    let title = Paragraph::new(" Home timeline ").block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(title, chunks[0]);

    let content_area = chunks[1];
    if loading {
        let para = Paragraph::new("Loading…").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, content_area);
        return;
    }

    if statuses.is_empty() {
        let para = Paragraph::new("No toots — home timeline is empty.")
            .block(Block::default().borders(Borders::ALL).title(" Timeline "))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, content_area);
    } else {
        let visible = content_area.height as usize;
        let start = scroll.min(statuses.len().saturating_sub(visible));
        let end = (start + visible).min(statuses.len());
        let mut lines: Vec<Line> = Vec::with_capacity(end - start);
        for (i, s) in statuses[start..end].iter().enumerate() {
            let idx = start + i;
            let style = if idx == selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let account = &s.account;
            let display = account.display_name.as_str();
            let handle = if account.acct.is_empty() {
                account.username.as_str()
            } else {
                account.acct.as_str()
            };
            let header = format!("@{} · {}", handle, s.created_at);
            let content = strip_html(&s.content);
            let content_preview = content.lines().next().unwrap_or(&content);
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", display),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Green),
                ),
                Span::styled(header, Style::default().fg(Color::DarkGray)),
                Span::raw("\n"),
                Span::styled(content_preview.chars().take(80).collect::<String>(), style),
            ]);
            lines.push(line);
        }
        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Timeline "))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, content_area);
    }

    let status_line = Line::from(Span::styled(
        " ↑/↓: select  Enter: open  n: new toot  q: quit  r: refresh ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(status_line), chunks[2]);
}

/// r[toot.view-detail]: single toot with full content and actions.
pub fn draw_toot_detail(frame: &mut Frame, status: &Status, reply_id: Option<&str>, message: &str) {
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

    let acc = &status.account;
    let header = format!("@{} · {}", acc.acct, status.created_at);
    let content = strip_html(&status.content);
    let lines = vec![
        Line::from(Span::styled(
            acc.display_name.as_str(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(header, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(content),
    ];
    let block = Block::default().borders(Borders::ALL);
    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(para, chunks[1]);

    if !message.is_empty() {
        let msg = Paragraph::new(message).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, chunks[2]);
    }

    let help = Line::from(Span::styled(
        " b: boost  f: favourite  r: reply  Esc: back ",
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
    let count_str = format!("{}/{}", len, char_limit);
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
        " Enter: post  Esc: cancel (no post) ",
        Style::default().dim(),
    ));
    frame.render_widget(Paragraph::new(help), chunks[3]);
}

//! mastotui — TUI client for Mastodon. r[config.first-run]

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use mastotui::app::App;
use mastotui::error::Result;

fn main() -> Result<()> {
    let mut app = App::new()?;
    ratatui::run(|terminal| run_app(terminal, &mut app))
        .map_err(mastotui::error::MastotuiError::Io)?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> std::io::Result<()> {
    loop {
        app.ensure_timeline_loaded()
            .map_err(std::io::Error::other)?;
        terminal.draw(|f| app.draw(f))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && app.handle_key(key.code).map_err(std::io::Error::other)?
                {
                    break;
                }
            }
        }
    }
    Ok(())
}

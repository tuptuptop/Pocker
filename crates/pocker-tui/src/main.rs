//! Pocker TUI — Terminal User Interface.
//!
//! A Ratatui-based terminal interface for Pocker.
//! Provides chat, plugin management, sandbox, and approval UI.

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

mod app;
mod ui;

/// Run the TUI.
pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = app::App::new();

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &app))?;

        // Handle input
        if !app::handle_input(&mut app)? {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Pocker TUI — starting...");
    run().await
}

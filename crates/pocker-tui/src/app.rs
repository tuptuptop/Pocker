//! App state for the TUI.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

/// TUI application state.
pub struct App {
    pub should_quit: bool,
    pub current_tab: Tab,
    pub input: String,
    pub messages: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Chat,
    Plugins,
    Skills,
    Settings,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            current_tab: Tab::Chat,
            input: String::new(),
            messages: vec!["Welcome to Pocker TUI!".to_string()],
        }
    }
}

/// Handle keyboard input. Returns false if the app should quit.
pub fn handle_input(app: &mut App) -> anyhow::Result<bool> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(true);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('q') => {
                app.should_quit = true;
                return Ok(false);
            }
            KeyCode::Tab => {
                app.current_tab = match app.current_tab {
                    Tab::Chat => Tab::Plugins,
                    Tab::Plugins => Tab::Skills,
                    Tab::Skills => Tab::Settings,
                    Tab::Settings => Tab::Chat,
                };
            }
            KeyCode::Enter => {
                if !app.input.is_empty() {
                    app.messages.push(format!("> {}", app.input));
                    app.messages
                        .push("(not yet connected to engine)".to_string());
                    app.input.clear();
                }
            }
            KeyCode::Char(c) => {
                app.input.push(c);
            }
            KeyCode::Backspace => {
                app.input.pop();
            }
            _ => {}
        }
    }
    Ok(true)
}

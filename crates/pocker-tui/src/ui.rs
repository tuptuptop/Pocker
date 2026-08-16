//! UI rendering for the TUI.

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Tabs
    let titles = ["Chat", "Plugins", "Skills", "Settings"];
    let tab_index = match app.current_tab {
        Tab::Chat => 0,
        Tab::Plugins => 1,
        Tab::Skills => 2,
        Tab::Settings => 3,
    };
    let tabs = Tabs::new(titles.iter().map(|t| Span::raw(*t)))
        .block(Block::default().borders(Borders::ALL).title("Pocker TUI"))
        .select(tab_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    // Main content
    match app.current_tab {
        Tab::Chat => {
            let messages: Vec<ListItem> = app
                .messages
                .iter()
                .map(|m| ListItem::new(Line::from(m.as_str())))
                .collect();
            let list =
                List::new(messages).block(Block::default().borders(Borders::ALL).title("Chat"));
            f.render_widget(list, chunks[1]);
        }
        Tab::Plugins => {
            let text =
                "No plugins loaded.\n\nUse 'pocker plugin install <name>' to install plugins.";
            let p =
                Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Plugins"));
            f.render_widget(p, chunks[1]);
        }
        Tab::Skills => {
            let text =
                "No skills loaded.\n\nSkills are plugins that provide higher-level capabilities.";
            let p =
                Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Skills"));
            f.render_widget(p, chunks[1]);
        }
        Tab::Settings => {
            let text = "Pocker Settings\n\nProfile: (not loaded)\nModel: (not configured)";
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Settings"));
            f.render_widget(p, chunks[1]);
        }
    }

    // Input
    let input = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Input (Enter to send, Tab to switch, q to quit)"),
    );
    f.render_widget(input, chunks[2]);
}

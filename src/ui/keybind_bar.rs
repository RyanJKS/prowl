use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, InputMode};

/// A single key hint entry.
struct Hint {
    key: &'static str,
    desc: &'static str,
}

/// Render the keybind hint bar into `area`.
pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let hints: &[Hint] = match app.input_mode {
        InputMode::Insert => &[
            Hint { key: "Enter", desc: "open" },
            Hint { key: "Ctrl+E", desc: "editor" },
            Hint { key: "Ctrl+O", desc: "cd" },
            Hint { key: "Ctrl+J/K", desc: "navigate" },
            Hint { key: "↑/↓", desc: "scroll preview" },
            Hint { key: "Esc", desc: "normal mode" },
            Hint { key: "Ctrl+C", desc: "quit" },
        ],
        InputMode::Normal => &[
            Hint { key: "Enter", desc: "open" },
            Hint { key: "e", desc: "editor" },
            Hint { key: "c", desc: "cd" },
            Hint { key: "y", desc: "yank" },
            Hint { key: "o", desc: "xdg-open" },
            Hint { key: "j/k", desc: "navigate" },
            Hint { key: "i/:", desc: "insert mode" },
            Hint { key: "q", desc: "quit" },
        ],
    };

    // Build a single-line span list from hints
    let mut spans: Vec<Span> = Vec::new();
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            hint.key,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", hint.desc),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let paragraph = Paragraph::new(ratatui::text::Line::from(spans))
        .style(Style::default().bg(Color::Reset));

    Widget::render(paragraph, area, buf);
}

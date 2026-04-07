use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use crate::app::{App, InputMode};

/// Render the search input widget into `area`.
pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let (mode_indicator, indicator_color) = match app.input_mode {
        InputMode::Insert => ("❯", Color::Green),
        InputMode::Normal => ("●", Color::Yellow),
    };

    // Result count / scanning indicator
    let status_str = if app.is_scanning {
        " scanning…".to_string()
    } else {
        format!(
            " {}/{}",
            app.filtered_results.len(),
            app.candidates.len()
        )
    };

    let indicator_span = Span::styled(
        format!(" {mode_indicator} "),
        Style::default()
            .fg(indicator_color)
            .add_modifier(Modifier::BOLD),
    );

    let query_span = Span::styled(
        app.query.clone(),
        Style::default().fg(Color::White),
    );

    let status_span = Span::styled(
        status_str,
        Style::default().fg(Color::DarkGray),
    );

    let line = Line::from(vec![indicator_span, query_span, status_span]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(match app.input_mode {
            InputMode::Insert => Color::Green,
            InputMode::Normal => Color::DarkGray,
        }))
        .title(Span::styled(
            " prowl ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    Paragraph::new(line).block(block).render(area, buf);
}

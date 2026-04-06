use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, StatefulWidget};

use crate::app::App;

/// Render the results list into `area`.
pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " results ",
            Style::default().fg(Color::DarkGray),
        ));

    let items: Vec<ListItem> = app
        .filtered_results
        .iter()
        .map(|r| {
            let name_span = Span::styled(
                r.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
            let parent_span = Span::styled(
                format!("  {}", r.parent_display),
                Style::default().fg(Color::DarkGray),
            );
            ListItem::new(Line::from(vec![name_span, parent_span]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !app.filtered_results.is_empty() {
        state.select(Some(app.selected_index));
    }

    StatefulWidget::render(list, area, buf, &mut state);
}

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::StatefulWidget;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget, Wrap};

use crate::app::App;

/// Render the preview pane into `area`.
pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " preview ",
            Style::default().fg(Color::DarkGray),
        ));

    let content = if app.preview_content.is_empty() {
        if app.is_scanning {
            "scanning…".to_string()
        } else if app.filtered_results.is_empty() {
            "no results".to_string()
        } else {
            "loading preview…".to_string()
        }
    } else {
        app.preview_content.clone()
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));

    Widget::render(paragraph, area, buf);

    // Draw scrollbar on the right edge
    let content_line_count = app.preview_content.lines().count() as u16;
    if content_line_count > area.height.saturating_sub(2) {
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(content_line_count as usize)
            .position(app.preview_scroll as usize);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        scrollbar.render(area, buf, &mut scrollbar_state);
    }
}

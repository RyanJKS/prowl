use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// The computed layout areas for all UI components.
pub struct AppLayout {
    /// Search input bar.
    pub search: Rect,
    /// Results list.
    pub results: Rect,
    /// Preview pane (may be zero-sized if terminal is too narrow).
    pub preview: Rect,
    /// Keybind hint bar at the bottom.
    pub keybind_bar: Rect,
}

/// Compute the layout from the full frame area.
///
/// Layout rules:
/// - Keybind bar: 1 row at the bottom.
/// - Search bar: 3 rows at the top.
/// - Remaining area is split horizontally 50/50 between results and preview.
/// - Preview is hidden (zero-width) when terminal width < `preview_collapse_width`.
pub fn compute_layout(area: Rect, preview_collapse_width: u32) -> AppLayout {
    // Vertical split: [search (3), body (?), keybind_bar (1)]
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let search_area = vertical[0];
    let body_area = vertical[1];
    let keybind_bar_area = vertical[2];

    // Horizontal split of body area
    let show_preview = area.width >= preview_collapse_width as u16;

    let (results_area, preview_area) = if show_preview {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body_area);
        (horizontal[0], horizontal[1])
    } else {
        // Full width for results; zero-width preview
        (body_area, Rect::default())
    };

    AppLayout {
        search: search_area,
        results: results_area,
        preview: preview_area,
        keybind_bar: keybind_bar_area,
    }
}

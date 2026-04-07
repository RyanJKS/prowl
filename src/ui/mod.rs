mod keybind_bar;
mod layout;
mod preview;
mod results;
mod search_input;

use ratatui::Frame;

use crate::app::App;

/// Top-level draw function called each frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let layout = layout::compute_layout(area, app.config.preview_collapse_width);

    frame.render_widget(
        SearchWidget(app),
        layout.search,
    );

    frame.render_widget(
        ResultsWidget(app),
        layout.results,
    );

    if layout.preview.width > 0 && layout.preview.height > 0 {
        frame.render_widget(
            PreviewWidget(app),
            layout.preview,
        );
    }

    if app.config.show_keybind_bar {
        frame.render_widget(
            KeybindWidget(app),
            layout.keybind_bar,
        );
    }
}

// ─── Widget wrappers ─────────────────────────────────────────────────────────
// ratatui's render_widget requires a type that implements Widget. We wrap
// the app reference in thin newtype structs to delegate to each sub-module.

struct SearchWidget<'a>(&'a App);
struct ResultsWidget<'a>(&'a App);
struct PreviewWidget<'a>(&'a App);
struct KeybindWidget<'a>(&'a App);

impl ratatui::widgets::Widget for SearchWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        search_input::render(self.0, area, buf);
    }
}

impl ratatui::widgets::Widget for ResultsWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        results::render(self.0, area, buf);
    }
}

impl ratatui::widgets::Widget for PreviewWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        preview::render(self.0, area, buf);
    }
}

impl ratatui::widgets::Widget for KeybindWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        keybind_bar::render(self.0, area, buf);
    }
}

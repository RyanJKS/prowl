use std::fs;
use std::io::Write as _;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config as NucleoConfig, Nucleo, Utf32String};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::actions::{resolve_action, substitute_path, ActionMode};
use crate::config::Config;
use crate::search::query::parse_query;
use crate::search::walker::{walk_directories, WalkOptions};
use crate::ui;

// ─── AppEvent ─────────────────────────────────────────────────────────────────

/// Events sent from background tasks to the main thread.
pub enum AppEvent {
    /// The background directory walk finished.
    WalkComplete(Vec<CandidateEntry>),
    /// Preview content is ready for the current selection.
    PreviewReady(String),
}

// ─── CandidateEntry ───────────────────────────────────────────────────────────

/// A single directory candidate stored in the app.
#[derive(Clone)]
pub struct CandidateEntry {
    /// Full UTF-8 path.
    pub path: String,
    /// Basename (last component).
    pub name: String,
    /// Parent path display string.
    pub parent_display: String,
}

// ─── FilteredResult ───────────────────────────────────────────────────────────

/// A result item shown in the results list.
#[derive(Clone)]
pub struct FilteredResult {
    pub path: String,
    pub name: String,
    pub parent_display: String,
}

// ─── InputMode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Insert,
    Normal,
}

// ─── App ──────────────────────────────────────────────────────────────────────

/// Central application state.
pub struct App {
    pub config: Config,
    pub root: String,
    pub input_mode: InputMode,
    pub query: String,
    pub cursor_pos: usize,
    pub selected_index: usize,
    pub preview_scroll: u16,
    pub candidates: Vec<CandidateEntry>,
    pub filtered_results: Vec<FilteredResult>,
    pub preview_content: String,
    pub cd_path: Option<String>,
    pub is_scanning: bool,
    /// Pending shell command to run in suspend mode (processed by the run loop).
    pub pending_suspend_cmd: Option<String>,

    // Nucleo fuzzy matcher
    pub(crate) nucleo: Nucleo<CandidateEntry>,

    // Background event channel
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    event_tx: mpsc::UnboundedSender<AppEvent>,

    // Tokio runtime for async tasks
    rt: Arc<tokio::runtime::Runtime>,
}

impl App {
    /// Create a new `App` and kick off the initial directory walk.
    pub fn new(config: Config, root: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<AppEvent>();

        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime"),
        );

        let nucleo = Self::make_nucleo();

        let mut app = Self {
            config,
            root: root.clone(),
            input_mode: InputMode::Insert,
            query: String::new(),
            cursor_pos: 0,
            selected_index: 0,
            preview_scroll: 0,
            candidates: Vec::new(),
            filtered_results: Vec::new(),
            preview_content: String::new(),
            cd_path: None,
            is_scanning: true,
            pending_suspend_cmd: None,
            nucleo,
            event_rx: rx,
            event_tx: tx,
            rt,
        };

        app.start_walk();
        app
    }

    // ─── Nucleo helpers ───────────────────────────────────────────────────────

    fn make_nucleo() -> Nucleo<CandidateEntry> {
        Nucleo::new(
            NucleoConfig::DEFAULT,
            Arc::new(|| {}), // no-op notify; we poll via tick()
            None,
            1,
        )
    }

    /// Destroy and recreate the nucleo instance, re-injecting all candidates.
    pub fn rebuild_nucleo(&mut self) {
        self.nucleo = Self::make_nucleo();
        let injector = self.nucleo.injector();
        for entry in &self.candidates {
            let entry_clone = entry.clone();
            let path_clone = entry.path.clone();
            injector.push(entry_clone, move |_e, cols| {
                cols[0] = Utf32String::from(path_clone.as_str());
            });
        }
    }

    // ─── Background walk ──────────────────────────────────────────────────────

    fn start_walk(&mut self) {
        let tx = self.event_tx.clone();
        let roots: Vec<String> = if self.config.roots.is_empty() {
            vec![self.root.clone()]
        } else {
            self.config
                .expanded_roots()
                .into_iter()
                .map(|p| p.to_string())
                .collect()
        };
        let opts = WalkOptions {
            max_depth: self.config.default_depth as usize,
            show_hidden: self.config.show_hidden,
            respect_gitignore: self.config.respect_gitignore,
            follow_symlinks: self.config.follow_symlinks,
        };

        let rt = self.rt.clone();
        thread::spawn(move || {
            let entries = walk_directories(&roots, &opts);
            let candidates: Vec<CandidateEntry> = entries
                .into_iter()
                .map(|e| CandidateEntry {
                    path: e.path.to_string(),
                    name: e.name,
                    parent_display: e.parent_display,
                })
                .collect();
            rt.spawn(async move {
                let _ = tx.send(AppEvent::WalkComplete(candidates));
            });
        });
    }

    // ─── Event processing ─────────────────────────────────────────────────────

    /// Drain all pending background events.
    pub fn process_events(&mut self) {
        while let Ok(evt) = self.event_rx.try_recv() {
            match evt {
                AppEvent::WalkComplete(entries) => {
                    self.candidates = entries;
                    self.is_scanning = false;
                    self.rebuild_nucleo();
                    self.update_filtered_results();
                    self.update_preview();
                }
                AppEvent::PreviewReady(content) => {
                    self.preview_content = content;
                }
            }
        }
    }

    // ─── Filtered results ─────────────────────────────────────────────────────

    /// Parse the current query, update nucleo pattern, read snapshot, apply
    /// post-filters, and clamp the selection index.
    pub fn update_filtered_results(&mut self) {
        let parsed = parse_query(&self.query);

        // Update nucleo fuzzy pattern
        self.nucleo.pattern.reparse(
            0,
            &parsed.fuzzy,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );
        self.nucleo.tick(10);

        let snap = self.nucleo.snapshot();
        let count = snap.matched_item_count();
        let max = self.config.max_results as u32;

        let mut results: Vec<FilteredResult> = Vec::with_capacity(count.min(max) as usize);
        for item in snap.matched_items(..count.min(max)) {
            let entry = item.data;
            let path_lower = entry.path.to_lowercase();

            // Apply negation filters
            if parsed
                .negations
                .iter()
                .any(|n| path_lower.contains(n.as_str()))
            {
                continue;
            }

            // Apply prefix filter
            if let Some(ref prefix) = parsed.prefix {
                if !entry.path.starts_with(prefix.as_str()) {
                    continue;
                }
            }

            // Apply exact match filters (all must match)
            if parsed
                .exact
                .iter()
                .any(|e| !path_lower.contains(e.as_str()))
            {
                continue;
            }

            results.push(FilteredResult {
                path: entry.path.clone(),
                name: entry.name.clone(),
                parent_display: entry.parent_display.clone(),
            });
        }

        self.filtered_results = results;

        // Clamp selection index
        if !self.filtered_results.is_empty()
            && self.selected_index >= self.filtered_results.len()
        {
            self.selected_index = self.filtered_results.len() - 1;
        }
    }

    // ─── Preview ──────────────────────────────────────────────────────────────

    /// Kick off an async preview generation for the current selection.
    pub fn update_preview(&mut self) {
        let Some(result) = self.filtered_results.get(self.selected_index) else {
            self.preview_content = String::new();
            return;
        };
        let path = result.path.clone();
        let tx = self.event_tx.clone();

        self.rt.spawn(async move {
            let content = generate_preview(&path).await;
            let _ = tx.send(AppEvent::PreviewReady(content));
        });
    }

    // ─── Key handling ─────────────────────────────────────────────────────────

    /// Dispatch a key event to the correct handler based on the current mode.
    /// Returns `true` if the application should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        // Preview scroll: Up/Down/PgUp/PgDn
        match key.code {
            KeyCode::Up => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
                return false;
            }
            KeyCode::Down => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
                return false;
            }
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
                return false;
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(10);
                return false;
            }
            _ => {}
        }

        // Ctrl+J/K navigate regardless of mode
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('j') => {
                    self.move_selection(1);
                    return false;
                }
                KeyCode::Char('k') => {
                    self.move_selection(-1);
                    return false;
                }
                _ => {}
            }
        }

        match self.input_mode {
            InputMode::Insert => self.handle_key_insert(key),
            InputMode::Normal => self.handle_key_normal(key),
        }
    }

    fn handle_key_insert(&mut self, key: KeyEvent) -> bool {
        // Handle ctrl combos first
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('e') => {
                    self.dispatch_action("ctrl_e", "insert");
                    return false;
                }
                KeyCode::Char('o') => {
                    self.dispatch_action("ctrl_o", "insert");
                    return self.cd_path.is_some();
                }
                _ => return false,
            }
        }

        match key.code {
            KeyCode::Esc => {
                if !self.query.is_empty() {
                    self.query.clear();
                    self.cursor_pos = 0;
                    self.update_filtered_results();
                    self.update_preview();
                } else {
                    self.input_mode = InputMode::Normal;
                }
                false
            }
            KeyCode::Enter => {
                self.dispatch_action("enter", "insert");
                self.cd_path.is_some()
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let byte_pos = self.char_to_byte_pos(self.cursor_pos - 1);
                    self.query.remove(byte_pos);
                    self.cursor_pos -= 1;
                    self.update_filtered_results();
                    self.update_preview();
                }
                false
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                false
            }
            KeyCode::Right => {
                let len = self.query.chars().count();
                if self.cursor_pos < len {
                    self.cursor_pos += 1;
                }
                false
            }
            KeyCode::Char(c) => {
                let byte_pos = self.char_to_byte_pos(self.cursor_pos);
                self.query.insert(byte_pos, c);
                self.cursor_pos += 1;
                self.update_filtered_results();
                self.update_preview();
                false
            }
            _ => false,
        }
    }

    fn handle_key_normal(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('i') | KeyCode::Char('/') => {
                self.input_mode = InputMode::Insert;
            }
            KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Enter => {
                self.dispatch_action("enter", "normal");
                return self.cd_path.is_some();
            }
            KeyCode::Char(c) => {
                let key_str = c.to_string();
                self.dispatch_action(&key_str, "normal");
                return self.cd_path.is_some();
            }
            _ => {}
        }
        false
    }

    // ─── Selection movement ───────────────────────────────────────────────────

    fn move_selection(&mut self, delta: i32) {
        if self.filtered_results.is_empty() {
            return;
        }
        let len = self.filtered_results.len() as i32;
        let new_idx = (self.selected_index as i32 + delta).clamp(0, len - 1);
        let prev = self.selected_index;
        self.selected_index = new_idx as usize;
        if self.selected_index != prev {
            self.preview_scroll = 0;
            self.update_preview();
        }
    }

    // ─── Action dispatch ──────────────────────────────────────────────────────

    /// Resolve and execute the action bound to `key` in the given `input_mode`.
    pub fn dispatch_action(&mut self, key: &str, input_mode: &str) {
        let tab_mode = "dirs";

        let Some(action_def) = resolve_action(&self.config.actions, input_mode, tab_mode, key)
        else {
            return;
        };

        let cmd_template = action_def.cmd.clone();
        let mode = ActionMode::parse(&action_def.mode);

        let paths: Vec<String> = self
            .filtered_results
            .get(self.selected_index)
            .map(|r| vec![r.path.clone()])
            .unwrap_or_default();

        if paths.is_empty() {
            return;
        }

        match mode {
            ActionMode::Builtin => match cmd_template.as_str() {
                "__prowl_cd" => {
                    self.cd_path = paths.first().cloned();
                }
                "__prowl_yank" => {
                    if let Some(p) = paths.first() {
                        let _ = copy_to_clipboard(p);
                    }
                }
                _ => {}
            },
            ActionMode::Detach => {
                let cmd = substitute_path(&cmd_template, &paths);
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
            ActionMode::Suspend => {
                let cmd = substitute_path(&cmd_template, &paths);
                self.pending_suspend_cmd = Some(cmd);
            }
        }
    }

    // ─── Cursor helpers ───────────────────────────────────────────────────────

    fn char_to_byte_pos(&self, char_idx: usize) -> usize {
        self.query
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.query.len())
    }
}

// ─── Clipboard helper ─────────────────────────────────────────────────────────

/// Copy text to the system clipboard using arboard.
fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut ctx = arboard::Clipboard::new()?;
    ctx.set_text(text)?;
    Ok(())
}

// ─── Preview generation ───────────────────────────────────────────────────────

/// Generate a plain-text overview of a directory: name, path, 2-level tree,
/// and first 6 lines of README.md if present.
async fn generate_preview(path: &str) -> String {
    use std::path::Path;

    let p = Path::new(path);
    let mut out = String::new();

    // Header: directory name + full path
    let dir_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    out.push_str(&format!("{dir_name}\n"));
    out.push_str(&format!("{path}\n"));
    out.push_str(&"─".repeat(48));
    out.push('\n');

    // 2-level tree
    match p.read_dir() {
        Ok(entries) => {
            let mut top_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            // Sort: dirs first, then alphabetical
            top_entries.sort_by_key(|e| {
                (
                    !e.file_type().map(|ft| ft.is_dir()).unwrap_or(false),
                    e.file_name(),
                )
            });
            top_entries.truncate(20);

            for entry in &top_entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let suffix = if is_dir { "/" } else { "" };
                out.push_str(&format!("  {name_str}{suffix}\n"));

                if is_dir {
                    if let Ok(children) = entry.path().read_dir() {
                        let mut child_entries: Vec<_> = children.filter_map(|e| e.ok()).collect();
                        child_entries.sort_by_key(|e| {
                            (
                                !e.file_type().map(|ft| ft.is_dir()).unwrap_or(false),
                                e.file_name(),
                            )
                        });
                        child_entries.truncate(20);
                        for child in &child_entries {
                            let cname = child.file_name();
                            let cname_str = cname.to_string_lossy();
                            let c_is_dir =
                                child.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                            let c_suffix = if c_is_dir { "/" } else { "" };
                            out.push_str(&format!("    {cname_str}{c_suffix}\n"));
                        }
                    }
                }
            }
        }
        Err(e) => {
            out.push_str(&format!("(cannot read directory: {e})\n"));
        }
    }

    // README snippet
    let readme_path = p.join("README.md");
    if readme_path.exists() {
        out.push('\n');
        out.push_str(&"─".repeat(48));
        out.push('\n');
        out.push_str("README.md\n");
        if let Ok(contents) = fs::read_to_string(&readme_path) {
            for (i, line) in contents.lines().enumerate() {
                if i >= 6 {
                    out.push_str("...\n");
                    break;
                }
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out
}

// ─── Terminal run loop ────────────────────────────────────────────────────────

/// Temporarily leave the TUI, run a shell command, and restore it.
fn suspend_for_command(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    cmd: &str,
) -> Result<()> {
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    let _status = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status();

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    Ok(())
}

/// Main entry point for the TUI application.
pub fn run(config: Config, root: String) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new(config, root);

    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal unconditionally
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result?;

    // Write cd target to lastdir file if a cd was requested
    if let Some(ref cd_target) = app.cd_path {
        let lastdir = &app.config.paths.lastdir_file;
        if let Some(parent) = lastdir.parent() {
            let _ = fs::create_dir_all(parent.as_std_path());
        }
        let mut f = fs::File::create(lastdir.as_std_path())?;
        writeln!(f, "{cd_target}")?;
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Process background events (walk results, preview content)
        app.process_events();

        // Tick nucleo matcher
        app.nucleo.tick(10);

        // Draw the current frame
        terminal.draw(|f| ui::draw(f, app))?;

        // Run any pending suspend command
        if let Some(cmd) = app.pending_suspend_cmd.take() {
            suspend_for_command(terminal, &cmd)?;
            continue;
        }

        // Quit if cd was triggered
        if app.cd_path.is_some() {
            return Ok(());
        }

        // Poll for terminal input (16 ms ≈ 60 fps)
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                let should_quit = app.handle_key(key);

                // Handle suspend command queued during key processing
                if let Some(cmd) = app.pending_suspend_cmd.take() {
                    suspend_for_command(terminal, &cmd)?;
                    continue;
                }

                if should_quit || app.cd_path.is_some() {
                    return Ok(());
                }
            }
        }
    }
}

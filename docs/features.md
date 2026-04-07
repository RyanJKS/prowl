# Prowl — Feature Reference

> AI agent reference document. Describes what is currently implemented in the codebase.
> For planned/future features see `docs/superpowers/specs/2026-04-06-prowl-design.md`.

---

## Libraries

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.29 | TUI framework — layout, widgets, rendering via crossterm backend |
| `crossterm` | 0.28 | Cross-platform raw mode, keyboard event polling, alternate screen |
| `nucleo` | 0.5 | Async fuzzy matching with scoring; manages its own thread pool |
| `ignore` | 0.4 | Directory traversal respecting `.gitignore` / `.ignore` (same engine as ripgrep) |
| `tokio` | 1 | Async runtime for background preview generation and walk coordination |
| `arboard` | 3 | Cross-platform clipboard read/write |
| `clap` | 4 | CLI argument parsing with derive macros |
| `serde` | 1 | Serialise/deserialise config structs |
| `toml` | 0.8 | Parse `config.toml` into typed structs |
| `camino` | 1 | UTF-8 path types (`Utf8PathBuf`) — cleaner ergonomics than `std::path::PathBuf` |
| `anyhow` | 1 | Ergonomic error propagation (`Result<T>` with context) |
| `shell-escape` | 0.1 | Shell-quoting paths for safe command interpolation |
| `tempfile` | 3 | (dev) Temporary directories/files in tests |

---

## Features

| Feature | Status | Entry point | Notes |
|---------|--------|-------------|-------|
| Fuzzy directory search | Implemented | `src/search/walker.rs`, `src/app.rs` | nucleo matcher, scored + ranked |
| File traversal | Implemented | `src/search/walker.rs` | ignore crate, directories only |
| `.gitignore` / `.ignore` respect | Implemented | `WalkOptions.respect_gitignore` | configurable |
| Hidden directory filtering | Implemented | `WalkOptions.show_hidden` | configurable |
| Symlink following | Implemented | `WalkOptions.follow_symlinks` | configurable |
| Configurable search roots | Implemented | `Config.roots` | multiple roots, `~` expansion |
| Configurable search depth | Implemented | `Config.default_depth` | default 5 |
| Insert mode (type-to-search) | Implemented | `InputMode::Insert` in `src/app.rs` | default on launch |
| Normal mode (vim-style navigation) | Implemented | `InputMode::Normal` in `src/app.rs` | `j`/`k`, single-key actions |
| Query: fuzzy token | Implemented | `src/search/query.rs` | bare tokens sent to nucleo |
| Query: negation (`!`) | Implemented | `ParsedQuery.negations` | post-filter after nucleo |
| Query: prefix (`^`) | Implemented | `ParsedQuery.prefix` | `starts_with` filter on full path |
| Query: exact (`'`) | Implemented | `ParsedQuery.exact` | substring filter, all must match |
| Query: tag (`#`) | Parsed only | `ParsedQuery.tags` | parsed but not applied in filtering |
| Query: escape (`\`) | Implemented | `src/search/query.rs` | strips `\` for `#`, `!`, `^`, `'` |
| Directory preview | Implemented | `generate_preview()` in `src/app.rs` | 2-level tree + README.md snippet (6 lines) |
| Async preview | Implemented | tokio task in `App::update_preview()` | non-blocking; `PreviewReady` event |
| Preview scrolling | Implemented | `App.preview_scroll` | `↑`/`↓`, `PgUp`/`PgDn` |
| Preview pane auto-collapse | Implemented | `Config.preview_collapse_width` | hides below configured terminal width |
| Actions system | Implemented | `src/actions.rs`, `src/app.rs` | three modes: detach, suspend, builtin |
| Detach action | Implemented | `ActionMode::Detach` | spawns subprocess, TUI stays open |
| Suspend action | Implemented | `ActionMode::Suspend` | leaves alternate screen, runs cmd, restores |
| Builtin: cd | Implemented | `__prowl_cd` | writes path to lastdir file; shell wrapper `cd`s |
| Builtin: yank | Implemented | `__prowl_yank` | copies path to clipboard via arboard |
| Path substitution `{}` | Implemented | `substitute_path()` in `src/actions.rs` | first selected path, shell-quoted |
| Path substitution `{*}` | Implemented | `substitute_path()` in `src/actions.rs` | all paths, shell-quoted, space-joined |
| Mode-specific action tables | Implemented | `ActionsConfig` in `src/config.rs` | `normal`, `insert`, `normal-files`, `insert-files`, `normal-bookmarks`, `insert-bookmarks` |
| Shell integration: zsh | Implemented | `src/shell/zsh.rs` | `p()` wrapper; reads lastdir and `cd`s |
| Shell integration: bash | Implemented | `src/shell/bash.rs` | same as zsh |
| Shell integration: fish | Implemented | `src/shell/fish.rs` | fish `function p` equivalent |
| `--init <shell>` flag | Implemented | `src/main.rs` | prints integration script to stdout |
| `--config` flag | Implemented | `src/main.rs` | opens config in `$EDITOR`; creates default if missing |
| `[path]` CLI arg | Implemented | `src/main.rs` | overrides configured roots |
| TOML config loading | Implemented | `src/config.rs` | XDG-compliant; falls back to defaults |
| Default config creation | Implemented | `src/main.rs` + `src/default_config.toml` | embedded template, written on `--config` |
| Keybind bar | Implemented | `src/ui/keybind_bar.rs` | configurable via `show_keybind_bar` |
| Scanning indicator | Implemented | `App.is_scanning` | shown while walk is in progress |
| Max results cap | Implemented | `Config.max_results` | default 200 |
| Git hash in version string | Implemented | `build.rs` | `PROWL_GIT_HASH` env var at compile time |

---

## Config reference

All fields live in `src/config.rs`. Config file path: `~/.config/prowl/config.toml` (respects `XDG_CONFIG_HOME`).

### Top-level (`Config`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `roots` | `Vec<String>` | `[]` | Directories to walk on startup. Empty = use CLI path or cwd. Supports `~` expansion. |
| `default_depth` | `u32` | `5` | Max recursion depth during walk. 1 = immediate children only. |
| `respect_gitignore` | `bool` | `true` | Honour `.gitignore`, `.ignore`, global gitignore. |
| `show_hidden` | `bool` | `false` | Include directories whose name starts with `.`. |
| `follow_symlinks` | `bool` | `false` | Follow symbolic links during traversal. |
| `frecency_weight` | `f64` | `0.3` | Config field exists; frecency not yet implemented. |
| `default_mode` | `String` | `"dirs"` | Config field exists; tab mode switching not yet implemented. |
| `default_preview_tab` | `String` | `"preview"` | Config field exists; multiple preview tabs not yet implemented. |
| `show_keybind_bar` | `bool` | `true` | Show the keybind hint bar at the bottom of the UI. |
| `preview_collapse_width` | `u32` | `100` | Terminal width (columns) below which the preview pane is hidden. |
| `max_results` | `usize` | `200` | Maximum items displayed in the results list. |
| `actions` | `ActionsConfig` | see below | Per-mode keybinding definitions. |
| `paths` | `PathsConfig` | see below | Override XDG-based file paths. |

### Actions (`ActionsConfig`)

Defined under `[actions.<table>]` in TOML. Each value is `{ cmd = "...", mode = "..." }`.

| Table key | Applied when |
|-----------|-------------|
| `normal` | Normal mode, any tab |
| `insert` | Insert mode, any tab |
| `normal-files` | Normal mode, files tab (not yet active) |
| `insert-files` | Insert mode, files tab (not yet active) |
| `normal-bookmarks` | Normal mode, bookmarks tab (not yet active) |
| `insert-bookmarks` | Insert mode, bookmarks tab (not yet active) |

Action modes:

| Mode | Behaviour |
|------|-----------|
| `detach` | Spawn subprocess with stdin/stdout/stderr null; TUI stays open |
| `suspend` | Leave alternate screen, run command blocking, restore TUI on exit |
| `builtin` | Execute a prowl internal command (`__prowl_cd`, `__prowl_yank`) |

Path substitution tokens:

| Token | Expands to |
|-------|-----------|
| `{}` | First selected path, POSIX shell-quoted |
| `{*}` | All selected paths, POSIX shell-quoted, space-separated |

Default action map:

| Mode | Key | Cmd | Action mode |
|------|-----|-----|-------------|
| normal | `enter` | `code {}` | detach |
| normal | `e` | `nvim {*}` | suspend |
| normal | `c` | `__prowl_cd` | builtin |
| normal | `y` | `__prowl_yank` | builtin |
| normal | `o` | `xdg-open {}` | detach |
| insert | `enter` | `code {}` | detach |
| insert | `ctrl_e` | `nvim {*}` | suspend |
| insert | `ctrl_o` | `__prowl_cd` | builtin |

### Paths (`PathsConfig`)

| Field | Default |
|-------|---------|
| `frecency_db` | `$XDG_DATA_HOME/prowl/frecency.db` → `~/.local/share/prowl/frecency.db` |
| `bookmarks` | `$XDG_DATA_HOME/prowl/bookmarks.toml` → `~/.local/share/prowl/bookmarks.toml` |
| `lastdir_file` | `$XDG_CACHE_HOME/prowl/lastdir` → `~/.cache/prowl/lastdir` |

---

## Query syntax

Parsed in `src/search/query.rs`. Tokens are whitespace-separated.

| Prefix | Field | Applied as |
|--------|-------|-----------|
| *(none)* | `fuzzy` | Passed to nucleo for scored fuzzy matching against full path |
| `!` | `negations` | Post-filter: exclude results where path (lowercased) contains the string |
| `^` | `prefix` | Post-filter: require path to `starts_with` the given string (first `^` only) |
| `'` | `exact` | Post-filter: require path (lowercased) to contain the string (all must match) |
| `#` | `tags` | Parsed but not applied in current filtering |
| `\#`, `\!`, `\^`, `\'` | fuzzy | Strips the backslash, treats the rest as a fuzzy token |

---

## XDG file paths

| Path | Purpose | Created by |
|------|---------|-----------|
| `$XDG_CONFIG_HOME/prowl/config.toml` | User configuration | `prowl --config` (first run) |
| `$XDG_CACHE_HOME/prowl/lastdir` | Directory selected for `cd` | Written by prowl on cd action, read + deleted by shell wrapper |
| `$XDG_DATA_HOME/prowl/frecency.db` | Frecency database (not yet used) | Configured but not written |
| `$XDG_DATA_HOME/prowl/bookmarks.toml` | Bookmarks (not yet used) | Configured but not written |

`XDG_CONFIG_HOME` defaults to `~/.config`, `XDG_CACHE_HOME` to `~/.cache`, `XDG_DATA_HOME` to `~/.local/share`.

---

## Architecture summary

Three concurrent execution contexts:

| Context | Responsibility |
|---------|---------------|
| Main thread | `crossterm::event::poll()` at 16 ms (60 fps), input dispatch, `ratatui::draw()` |
| Tokio runtime (background thread) | Directory walk (`ignore` crate), preview generation, sends `AppEvent` to main thread via `mpsc` channel |
| Nucleo thread pool | Fuzzy scoring; main thread reads `Snapshot` on each tick |

Key types:

| Type | File | Role |
|------|------|------|
| `App` | `src/app.rs` | Central state; owns nucleo instance, tokio runtime, event channel |
| `AppEvent` | `src/app.rs` | `WalkComplete(Vec<CandidateEntry>)` \| `PreviewReady(String)` |
| `CandidateEntry` | `src/app.rs` | `path`, `name`, `parent_display` — stored in nucleo |
| `FilteredResult` | `src/app.rs` | Post-filter result shown in UI |
| `InputMode` | `src/app.rs` | `Insert` \| `Normal` |
| `Config` | `src/config.rs` | All configuration fields |
| `ActionsConfig` | `src/config.rs` | Per-mode action maps |
| `ActionDef` | `src/config.rs` | `{ cmd: String, mode: String }` |
| `ActionMode` | `src/actions.rs` | `Detach` \| `Suspend` \| `Builtin` |
| `ParsedQuery` | `src/search/query.rs` | Structured query with fuzzy/negations/prefix/exact/tags |
| `WalkOptions` | `src/search/walker.rs` | Traversal configuration passed to `walk_directories()` |

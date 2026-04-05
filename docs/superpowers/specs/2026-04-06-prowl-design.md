# Prowl — Design Specification

> A fast, single-binary TUI for navigating, previewing, and opening anything in your filesystem.

---

## Table of Contents

1. [Overview](#overview)
2. [Core Philosophy](#core-philosophy)
3. [Tech Stack](#tech-stack)
4. [Architecture & Event Loop](#architecture--event-loop)
5. [Input Model & Keybindings](#input-model--keybindings)
6. [UI Layout & Navigation](#ui-layout--navigation)
7. [Modes](#modes)
8. [Query Parsing & Search Pipeline](#query-parsing--search-pipeline)
9. [Scoring & Ranking](#scoring--ranking)
10. [Preview System](#preview-system)
11. [Actions System](#actions-system)
12. [Filter System](#filter-system)
13. [Frecency System](#frecency-system)
14. [Bookmarks](#bookmarks)
15. [Project Type Detection](#project-type-detection)
16. [Shell Integration & CLI Commands](#shell-integration--cli-commands)
17. [Configuration Reference](#configuration-reference)
18. [Themes & Terminal Compatibility](#themes--terminal-compatibility)
19. [External Tool Integrations](#external-tool-integrations)
20. [Error Handling & Edge Cases](#error-handling--edge-cases)
21. [Distribution & Packaging](#distribution--packaging)
22. [Performance Characteristics](#performance-characteristics)
23. [Project File Structure](#project-file-structure)
24. [Development Milestones](#development-milestones)
25. [Future Roadmap](#future-roadmap)

---

## Overview

Prowl is a terminal UI application written in Rust that replaces the collection of shell functions most developers accumulate for navigating their filesystem. Instead of separate scripts for opening projects in VSCode, jumping to directories, searching files, or previewing content, Prowl provides a single unified interface for all of it.

You invoke `p` (a shell wrapper), fuzzy-search for a directory or file, see a rich preview on the right, and press one of several keybindings to act on it — open in an editor, `cd` into it, copy the path, spawn a terminal there, open in a file manager, or any custom command you've defined. Everything is keyboard-driven, responds in milliseconds, and disappears when you're done, leaving your shell exactly where you need to be.

---

## Core Philosophy

**Single binary, zero runtime dependencies.** Prowl compiles to a static binary. No Python, no Node modules, no virtual environment. Copy the binary to any machine and it works.

**Sub-5ms startup.** The tool is invoked constantly throughout the day. Any perceptible startup delay accumulates into genuine friction. Prowl initialises Ratatui, reads config, and renders the first frame in under 5 milliseconds.

**Don't replace your tools — launch them.** Prowl is a launcher that knows how to find things fast and hand them off to specialist tools (Neovim, VSCode, lazygit, csvlens, glow) with a single keypress.

**Async everywhere the user would notice.** Preview rendering, git status checks, and frecency lookups all happen on background threads. Navigating the result list is never blocked by I/O.

**Configuration over convention, but sensible defaults.** Everything is overridable via `~/.config/prowl/config.toml`, but the defaults work for the majority of developers without touching the config file.

**Respect the terminal.** Prowl detects capabilities at startup and degrades gracefully — rich image rendering in Kitty or WezTerm, Unicode block art in everything else, clean ASCII fallback in minimal environments.

---

## Tech Stack

| Concern | Crate | Rationale |
|---|---|---|
| TUI framework | `ratatui` | Production-standard Rust TUI, active development, flexible layout system |
| Terminal I/O | `crossterm` | Cross-platform raw mode, event handling, works on Linux/macOS/Windows |
| File traversal | `ignore` | Powers `ripgrep` and `fd`; respects `.gitignore`, extremely fast |
| Fuzzy matching | `nucleo` | Async, scored, path-aware; used by Helix editor in production |
| Config parsing | `toml` + `serde` | Deserialise directly into typed structs, zero-cost at runtime |
| Clipboard | `arboard` | Cross-platform, no external dependencies |
| Git integration | `git2` | libgit2 bindings — branch, status, remote URL, blame |
| Async runtime | `tokio` | Non-blocking preview rendering, background indexing |
| Syntax highlighting | `syntect` | Same engine as `bat`, in-process, no subprocess overhead |
| Frecency storage | `rusqlite` | Embedded SQLite, single file, portable across machines |
| Path utilities | `camino` | UTF-8 paths — cleaner ergonomics than `std::path::PathBuf` |
| Error handling | `anyhow` | Ergonomic error propagation throughout |

**Why not shell out to `fd` and `fzf`?** Subprocess overhead on every keystroke is measurable. Running traversal and fuzzy matching in-process via `ignore` and `nucleo` eliminates that overhead entirely and allows tighter integration — streaming results into the UI as they're discovered rather than waiting for `fd` to finish.

---

## Architecture & Event Loop

The application has three threads of execution:

**Main thread** — Runs a synchronous loop: poll crossterm events (with ~16ms timeout for 60fps), process input, render via ratatui. Never blocks on I/O. Communicates with the background runtime via a channel (`std::sync::mpsc::Receiver<AppEvent>`).

**Tokio runtime thread** — A multi-threaded tokio runtime spawned on a background thread at startup. Handles: file traversal (via `ignore` crate), preview rendering (syntect, git2, chafa subprocess), frecency reads/writes (rusqlite), and any other async work. Sends results back to the main thread via the `AppEvent` channel.

**Nucleo's internal threads** — Nucleo manages its own thread pool for scoring. The main thread reads scored/ranked results from nucleo's `Snapshot` on each render tick.

```
┌─────────────────────┐     AppEvent channel      ┌──────────────────────┐
│     Main Thread      │ <─────────────────────── │   Tokio Runtime      │
│                      │                           │                      │
│  crossterm::poll()   │                           │  walker::traverse()  │
│  process input       │                           │  preview::render()   │
│  nucleo.tick()       │                           │  frecency::read()    │
│  ratatui::draw()     │                           │  chafa subprocess    │
└─────────────────────┘                           └──────────────────────┘
         │
         │  inject candidates
         ▼
┌─────────────────────┐
│  Nucleo Matcher      │
│  (internal threads)  │
└─────────────────────┘
```

**State machine:** The `App` struct holds all state — current mode/tab, query, selected index, scroll positions, active filters, preview cache, input mode (insert/normal). Input events mutate `App` state directly on the main thread. Background results arrive as `AppEvent` variants and are merged into state on the next loop iteration.

**Candidate cache:** Each tab maintains a `Vec<PathEntry>` as the source-of-truth candidate list alongside its nucleo instance. Nucleo's `Injector` is append-only — candidates cannot be removed once injected. When a re-traversal is triggered (depth change, root toggle, hidden files toggle), the candidate cache is rebuilt from the new traversal, the nucleo instance is destroyed and recreated, and all candidates are re-injected from the cache. This decouples candidate storage from nucleo's internals, avoids stale entries in the scorer, and makes future features (content search tab, dynamic filtering) feasible without architectural changes.

---

## Input Model & Keybindings

### Vim-style modal input

Prowl uses a two-mode input model inspired by Neovim's Telescope:

**Insert mode** (default on launch — you usually want to search immediately):
- All printable keys type into the query
- `Esc` — if query is non-empty, clear it; if empty, switch to normal mode
- `Ctrl+j` / `Ctrl+k` or `Up` / `Down` — navigate results while staying in insert mode
- `Enter` — trigger primary action
- `Ctrl+c` — quit immediately

**Normal mode** (enter via `Esc` from empty query):
- Single-letter keys trigger actions: `c`, `e`, `y`, `g`, `m`, `v`, etc.
- `j` / `k` — navigate results
- `/` or `i` — enter insert mode (focus search input)
- `Esc` or `q` — quit
- `Space` — toggle multi-select mark on current item

**Mode indicator:** The search bar prompt changes — `❯` for insert mode, `●` for normal mode — with a distinct color shift.

### Reserved keybindings (work in both modes)

| Key | Action |
|---|---|
| `Up` / `Ctrl+k` | Move selection up |
| `Down` / `Ctrl+j` | Move selection down |
| `PgUp` / `Ctrl+u` | Move up 10 results |
| `PgDn` / `Ctrl+d` | Move down 10 results |
| `Shift+Up` | Scroll preview pane up |
| `Shift+Down` | Scroll preview pane down |
| `Tab` | Next mode tab (dirs → files → bookmarks) |
| `Shift+Tab` | Previous mode tab |
| `[` / `]` | Previous / next preview sub-tab |
| `Ctrl+f` | Toggle filter bar |
| `Ctrl+r` | Open root picker |
| `Ctrl+b` | Toggle bookmark on selected item |
| `Ctrl+p` | Open path back-stack popup |
| `Esc` | Context-dependent (see mode descriptions) |
| `Ctrl+c` | Quit immediately |
| `?` | Toggle help overlay (normal mode only; types in insert mode) |
| `F1` | Toggle help overlay (works in both modes) |
| `Backspace` | Delete last character from query (insert mode) |

### User-defined action keybindings

All action keybindings are configured in TOML with separate sections per input mode:

```toml
# Normal mode — single keys trigger actions
[actions.normal]
c     = { cmd = "__prowl_cd", mode = "builtin" }
y     = { cmd = "__prowl_yank_abs", mode = "builtin" }
enter = { cmd = "code {}", mode = "detach" }
e     = { cmd = "nvim {*}", mode = "suspend" }
t     = { cmd = "wezterm cli new-tab -- zsh -c 'cd {}; exec zsh'", mode = "detach" }
m     = { cmd = "glow {}", mode = "suspend" }
v     = { cmd = "csvlens {}", mode = "suspend" }
g     = { cmd = "open $(git -C {} remote get-url origin 2>/dev/null)", mode = "detach" }
o     = { cmd = "xdg-open {}", mode = "detach" }

# Insert mode — only modifier/special keys (single letters type)
[actions.insert]
enter  = { cmd = "code {}", mode = "detach" }
ctrl_e = { cmd = "nvim {*}", mode = "suspend" }
ctrl_o = { cmd = "__prowl_cd", mode = "builtin" }

# Tab-specific overrides use flat "mode-tab" keys to avoid deep TOML nesting.
# This scales cleanly as new tabs are added (e.g. normal-search, insert-search).
[actions.normal-files]
enter = { cmd = "nvim {*}", mode = "suspend" }

[actions.insert-files]
enter = { cmd = "nvim {*}", mode = "suspend" }
```

**Action resolution order:** When a key is pressed, Prowl resolves the action by checking (most specific first):
1. `[actions.<input_mode>-<tab_mode>]` (e.g. `actions.normal-files`)
2. `[actions.<input_mode>]` (e.g. `actions.normal`)
3. No match → key is ignored (normal mode) or typed into query (insert mode)

---

## UI Layout & Navigation

### Overall structure

```
┌─────────────────────────────────────────────────────┐
│ ● ● ●   prowl                                       │  ← titlebar
├──────────────────────────────────────────────────────┤
│ [dirs]  [files]  [bookmarks]    ~/dev  ~/work  ...  │  ← tabs + active roots
├───────────────────────┬──────────────────────────────┤
│ ❯ query...      d:5 ▣ │ overview  tree  git  readme  │  ← search + preview sub-tabs
├───────────────────────┤                              │
│  result 1  ←selected  │   meta / preview content     │
│  result 2             │                              │
│  result 3             │                              │
│  ...                  │                              │
├───────────────────────┤                              │
│  N results  filter:x  │                              │  ← status bar
├───────────────────────┴──────────────────────────────┤
│ [tab] mode  [ctrl+f] filter  [ctrl+d] depth  [?] help│  ← keybind bar
└─────────────────────────────────────────────────────┘
```

### Panel behaviour

The layout splits 50/50 horizontally into a results pane (left) and a preview pane (right). Both panels are independently scrollable — the results list scrolls with `j`/`k`/arrows, the preview pane scrolls with `Shift+Up`/`Shift+Down`.

Below 100 terminal columns, the preview pane collapses automatically. In this narrow mode, pressing `p` (normal mode) toggles the preview as a full-screen overlay, and `Esc` dismisses it.

The keybind bar at the bottom shows contextually relevant bindings for the current state — it changes based on input mode (insert vs normal), active filters, file vs directory selection, and preview sub-tab.

### Tabs

Three tabs: **dirs**, **files**, **bookmarks**. `Tab` cycles forward, `Shift+Tab` cycles backward.

Each tab holds completely independent state: its own search query, scroll position, active depth setting, active filters, selected item, and nucleo instance with its own candidate set. Switching tabs restores the full previous state.

Active roots are shown on the right side of the tab bar. `Ctrl+r` opens the root picker.

---

## Modes

### Dirs mode

Searches for directories across all active roots up to the configured depth. Each result shows:
- Directory name (bold)
- Parent path (muted)
- Project type tag chips (colored)
- Git ahead/behind indicator (`↑2 ↓3`) if applicable
- Inline stats (`rust 12k loc`) dimmed

Default mode. Covers the primary use case: "I want to open a project."

### Files mode

Searches for files across all active roots. Each result shows:
- Filename (bold)
- Containing directory (muted)
- File extension tag
- File size

Files mode has its own independent action bindings — e.g. `enter` opens the file in Neovim rather than the directory in VSCode.

### Bookmarks mode

Pinned entries (directories and files) in a flat list. Ordered by most recently accessed first. Fuzzy-searched when a query is typed.

Bookmarks persist in `~/.local/share/prowl/bookmarks.toml`:

```toml
[[bookmarks]]
path    = "~/dev/rust/axum-rest-api"
label   = "axum template"
added   = "2025-03-14T10:22:00Z"
```

Custom labels override the directory name in the bookmarks tab.

---

## Query Parsing & Search Pipeline

### Query parser

The raw query string is parsed into a structured `ParsedQuery`:

```
Input: "axum #rust !vendor ^~/dev"

ParsedQuery {
    fuzzy: "axum",
    tags: [Rust],
    negations: ["vendor"],
    prefix: Some("~/dev"),
    exact: [],
}
```

**Parsing rules:**
- `#word` tokens → extracted as tag filters
- `!word` tokens → extracted as negation filters (exclude results containing substring)
- `^path` token → extracted as path prefix filter (only one allowed)
- `'word` tokens → extracted as exact substring matches
- `\#`, `\!`, `\^`, `\'` → escaped literal characters (treated as fuzzy input, not special tokens)
- Everything remaining → joined as the fuzzy query string passed to nucleo

**Escaping:** Prefix a special character with `\` to search for it literally. `\#my-project` fuzzy-matches paths containing `#my-project` instead of filtering by a tag called `my-project`.

### Search pipeline

```
1. Startup
   └─ Walker traverses all active roots (tokio runtime, background)
      └─ Each discovered path → injected into Nucleo's candidate set
         └─ Project type detection runs per directory (single readdir, cached)

2. On keystroke
   └─ Parse query → ParsedQuery
      └─ Update Nucleo's pattern (fuzzy portion only)
         └─ Nucleo re-scores in-memory candidates (its own threads)

3. On render tick
   └─ Read Nucleo's Snapshot (sorted by score)
      └─ Apply post-filters: tags, negations, prefix, exact matches
         └─ Blend frecency into final score (70/30 default)
            └─ Truncate to max_results → render
```

**Why post-filter:** Nucleo manages its own candidate set and scoring threads. We can't efficiently remove candidates per-keystroke. Instead, nucleo scores everything, and we filter the sorted snapshot before rendering. With `max_results = 200`, filtering a few thousand scored results is sub-millisecond.

**Re-traversal triggers:**
- Depth changes (via `Ctrl+d` picker)
- Active roots change (via `Ctrl+r` picker)
- Tab switches to a tab that hasn't been populated yet
- Each tab maintains its own nucleo instance (dirs vs files)

---

## Scoring & Ranking

The final score for each result blends two signals:

**Fuzzy match quality** (from `nucleo`) accounts for 70% of the score by default. A highly relevant search term always beats a frequently-visited but less relevant path.

**Frecency** accounts for the remaining 30%. Frecency combines visit frequency and recency using the same algorithm as `zoxide`: each visit increments a score, and scores decay exponentially over time.

**Score normalization:** Nucleo scores are arbitrary positive integers and frecency scores are floats on a different scale. Before blending, both are normalized to `[0.0, 1.0]`:
- Nucleo: `normalized = score / max_score_in_snapshot` (the top-ranked result always gets 1.0)
- Frecency: `normalized = score / max_frecency_score_across_candidates` (the most-visited path always gets 1.0)
- Final: `(1.0 - frecency_weight) * normalized_nucleo + frecency_weight * normalized_frecency`

When the query is empty (no fuzzy matching active), nucleo scores are all equal, so the ranking is purely by frecency. When frecency data is empty (fresh install), the frecency component is zero and ranking is purely by fuzzy match quality.

The blend is configurable: `frecency_weight = 0.0` for pure fuzzy matching, `frecency_weight = 1.0` for pure frecency. The default of `0.3` keeps search results predictable while surfacing recently used items.

---

## Preview System

### Async rendering pipeline

When the selection changes, the main thread sends a `PreviewRequest` to the tokio runtime. If a previous request is still in-flight for a different path, it's cancelled via `tokio::CancellationToken`. The preview pane shows a loading indicator until the result arrives.

**PreviewContext:** Each `PreviewRequest` includes a `PreviewContext` containing the selected item, the active sub-tab, and metadata about how the item was found (source tab, query match positions). The dispatcher routes on `(item_type, source_context)` rather than just `item_type`. For v0.1, the source context is always `Browsing` and the behavior is identical to simple file-type dispatch. This interface is designed to support future features like content search (where the preview would highlight matching lines based on the search query) without requiring architectural changes to the preview system.

### Preview sub-tabs

Navigated with `[` and `]`:

| Tab | Directory content | File content |
|---|---|---|
| **Overview** | Git branch + dirty/clean, last modified, file count, 2-level tree, README first 6 lines | Syntax-highlighted content, file size, language, git blame first visible line |
| **Tree** | Recursive directory tree, expandable/collapsible with `space`, colour-coded by extension | Parent directory tree with the file highlighted |
| **Git** | `git status` (staged/unstaged/untracked) + last 5 commit summaries | Same, scoped to the file's repo |
| **Readme** | README rendered as stripped plain text | Nearest parent directory's README, or "no README" |

### File type dispatch

| Detection | Renderer | Async? |
|---|---|---|
| Is directory | Tree + git + README composite | Yes (git2 + fs reads) |
| Extension matches syntect grammar | Syntax-highlighted code | Yes (`spawn_blocking`) |
| `.md` | Stripped markdown, plain text | No (fast enough inline) |
| `.csv` / `.tsv` | Aligned columns, header highlighted, first 20 rows | No |
| Image (png, jpg, gif, webp, svg) | `chafa` subprocess | Yes |
| `.pdf` | Page count + metadata + first page text via `pdftotext` | Yes |
| JSON | Pretty-printed, syntax highlighted | Yes (`spawn_blocking`) |
| TOML / YAML / XML | Syntax highlighted | Yes (`spawn_blocking`) |
| Shell scripts | Syntax highlighted | Yes (`spawn_blocking`) |
| Binary / unknown | Hex dump (first 64 bytes) + MIME guess + file size | No |
| Empty file | Dim placeholder with metadata | No |
| Symlink | Resolve target, show what it points to, then preview the target | Yes |

### Preview cache

Last 20 preview results cached in an LRU cache keyed by `(path, sub_tab)`. Scrolling back to a recently viewed item shows the preview instantly. Cache is invalidated on re-traversal.

### Terminal image detection

Runs once at startup, stored in app state:

1. `$TERM` contains `kitty` → Kitty graphics protocol
2. `$TERM_PROGRAM` is `WezTerm` → Sixel
3. `$TERM_PROGRAM` is `iTerm2` or `iTerm.app` → iTerm2 inline images
4. `$COLORTERM` is `truecolor` → Unicode half-block, 24-bit color
5. Fallback → Unicode half-block, 256 color
6. `chafa` not in `$PATH` → placeholder with image dimensions + file size

---

## Actions System

### Dispatch modes

| Mode | Behaviour | Examples |
|---|---|---|
| `detach` | Spawn child process, detach, Prowl stays open | VSCode, xdg-open, browser, terminal tab |
| `suspend` | Leave alternate screen, disable raw mode, run child, wait for exit, re-enter TUI | Neovim, glow, csvlens, lazygit |
| `builtin` | Internal Prowl action, no subprocess | cd, yank, bookmark toggle |

### Path substitution

Two placeholders are available in command strings:

- `{}` — replaced with the shell-escaped absolute path of the single selected item (or the first selected item if multiple are marked)
- `{*}` — replaced with all selected paths as space-separated, shell-escaped arguments

For multi-select: if the command uses `{*}`, all paths are passed in a single invocation (`nvim file1 file2 file3`). If the command uses `{}`, the action is applied to each selected item individually — sequentially for suspend mode, in parallel for detach mode. Actions should prefer `{*}` for tools that accept multiple arguments (editors, file managers) and `{}` for tools that operate on a single path (cd, clipboard).

### Suspend/resume sequence

1. Save TUI state (already in `App` struct)
2. `crossterm::execute!(LeaveAlternateScreen)`
3. `crossterm::terminal::disable_raw_mode()`
4. Spawn child process, wait for exit
5. `crossterm::terminal::enable_raw_mode()`
6. `crossterm::execute!(EnterAlternateScreen)`
7. Force full redraw

### Built-in actions

- `__prowl_cd` — Write selected path to `lastdir_file`, exit with code 0. Shell wrapper handles the actual `cd`.
- `__prowl_yank_abs` — Copy absolute path to clipboard via `arboard`.
- `__prowl_yank_rel` — Copy path relative to `$HOME` to clipboard.
- `__prowl_bookmark_toggle` — Add/remove selected item from bookmarks.

### Action hints

The bottom of the preview pane shows action chips — small coloured tags showing the key and tool name for each defined action. These update contextually based on the selected item and current input mode.

### Frecency recording

Any action dispatch (all three modes) records a visit to the selected path in the frecency database. This happens asynchronously on the tokio runtime — it never blocks the action.

---

## Filter System

`Ctrl+f` opens the filter bar, an inline input below the search bar. Works in both insert and normal mode.

### Filter types

| Filter | Syntax | Example |
|---|---|---|
| Project tag | tag name | `rust`, `node`, `git` |
| Recency | `recent:<duration>` | `recent:7d`, `recent:30d` |
| Depth | `depth:<n>` | `depth:1`, `depth:3` |
| Frecency tier | tier name | `hot`, `warm`, `cold` |

### Behaviour

- Multiple filters are AND-ed
- Shown as dismissible chips in the status bar
- `Ctrl+f` again clears all active filters
- Individual chips removable via navigating in the filter bar and pressing `Delete`
- Filters are per-tab and persist across tab switches within the session

### Pipeline integration

Filters are applied as a post-filter step on nucleo's scored snapshot, same stage as query negations and prefix filters. They don't trigger re-traversal — they narrow the displayed results only.

---

## Frecency System

### Storage

SQLite database at `~/.local/share/prowl/frecency.db`:

```sql
CREATE TABLE visits (
    path        TEXT PRIMARY KEY,
    score       REAL NOT NULL DEFAULT 0,
    last_visit  INTEGER NOT NULL,  -- Unix timestamp
    visit_count INTEGER NOT NULL DEFAULT 0
);
```

### Scoring algorithm

Zoxide-compatible decay:

```
new_score = old_score * decay_factor + 1.0
decay_factor = 0.99^(hours_since_last_visit)
```

Frequently visited paths accumulate higher scores, inactive paths decay toward zero over weeks.

### Zoxide import

`prowl --import-zoxide` reads `~/.local/share/zoxide/db.zo` and migrates entries into Prowl's frecency DB. Scores are normalized to Prowl's scale. Safe to run multiple times (upserts).

### Portability

The frecency database can be symlinked to cloud storage to share history across machines. SQLite WAL mode handles concurrent read/write safely.

---

## Bookmarks

Bookmarks are explicitly pinned paths — unlike frecency, they don't decay and appear in their own tab.

`Ctrl+b` toggles a bookmark on the currently selected item. A pin indicator appears next to bookmarked items in dirs and files tabs.

Stored in `~/.local/share/prowl/bookmarks.toml` as a human-readable list with optional custom labels.

---

## Project Type Detection

### Built-in tags

| Tag | Marker files / directories |
|---|---|
| `git` | `.git/` |
| `rust` | `Cargo.toml` |
| `node` | `package.json` |
| `py` | `pyproject.toml`, `setup.py`, `setup.cfg`, `requirements.txt` |
| `go` | `go.mod` |
| `docker` | `Dockerfile`, `docker-compose.yml`, `docker-compose.yaml` |
| `nix` | `flake.nix`, `shell.nix`, `default.nix` |
| `zig` | `build.zig` |
| `java` | `pom.xml`, `build.gradle` |
| `ruby` | `Gemfile` |
| `elixir` | `mix.exs` |
| `haskell` | `stack.yaml`, `cabal.project` |
| `dotnet` | `*.csproj`, `*.fsproj`, `*.sln` |
| `terraform` | `*.tf` files present |
| `k8s` | `*.yaml` files containing `kind:` present |

Detection is fast (a single `readdir` call per directory) and cached for the session. Multiple tags can apply simultaneously.

### Custom tags

User-defined in config:

```toml
[[custom_tags]]
name = "monorepo"
color = "magenta"
markers = ["lerna.json", "pnpm-workspace.yaml", "turbo.json"]

[[custom_tags]]
name = "hugo"
color = "cyan"
markers = ["hugo.toml"]
```

Custom tags are processed after built-in tags, filterable via `#tagname`, shown as colored chips same as built-ins.

**Performance note on content-based markers:** Markers that include content matching (e.g. `"config.toml:baseURL"`) are significantly more expensive than filename-only markers because they require reading file contents. Content-based markers are evaluated lazily — only when a directory already matches at least one filename-only marker in the same custom tag definition, or when the directory is selected in the results list. They are never evaluated during traversal. This keeps traversal performance unaffected by custom tag complexity.

### Inline project stats

For each directory result, lazily calculate:
- Primary language (by file extension count)
- Approximate line count (heuristic based on file sizes)

Displayed dimly after tag chips: `rust 12k loc`. Cached per session. Calculated on the tokio runtime.

### Git ahead/behind indicator

For directories inside a git repo with a tracking branch, compare local ref against remote tracking ref via `git2` (no network, pure ref comparison). Displayed as `↑2 ↓3` next to the branch name. Calculated lazily, cached.

---

## Shell Integration & CLI Commands

### The cd problem

A subprocess cannot change the parent shell's working directory. Prowl writes the chosen path to a known file and the shell wrapper reads it after exit.

### Setup

```bash
prowl --init zsh  >> ~/.zshrc
prowl --init bash >> ~/.bashrc
prowl --init fish >> ~/.config/fish/config.fish
```

### Shell wrapper (zsh)

```zsh
p() {
  prowl "$@"
  local _prowl_lastdir="${XDG_CACHE_HOME:-$HOME/.cache}/prowl/lastdir"
  if [[ -f "$_prowl_lastdir" ]]; then
    builtin cd "$(cat "$_prowl_lastdir")"
    rm -f "$_prowl_lastdir"
  fi
}
```

### Fish wrapper

```fish
function p
    prowl $argv
    set _prowl_lastdir (path join $HOME .cache prowl lastdir)
    if test -f $_prowl_lastdir
        builtin cd (cat $_prowl_lastdir)
        rm -f $_prowl_lastdir
    end
end
```

### CLI commands

| Command | Description |
|---|---|
| `prowl` | Launch TUI in current directory |
| `prowl <path>` | Launch TUI rooted at path |
| `prowl --init <shell>` | Print shell integration snippet |
| `prowl --config` | Open config in `$EDITOR` |
| `prowl --check-config` | Validate config, report errors with line numbers |
| `prowl --health` | Diagnostic: tools, terminal caps, config, DB status |
| `prowl --setup` | Interactive prerequisite installer |
| `prowl --setup --auto` | Non-interactive install all missing tools |
| `prowl --import-zoxide` | Migrate zoxide frecency data |
| `prowl --frecency reset` | Clear all frecency data |
| `prowl --frecency export` | Dump frecency DB to JSON |
| `prowl --version` | Print version |
| `prowl --help` | Print help |

---

## Configuration Reference

Full annotated `~/.config/prowl/config.toml`:

```toml
# ── Roots ─────────────────────────────────────────────────────────────────────
roots = [
  "~/dev",
  "~/work",
  "~/dotfiles",
  "~/notes",
]

# ── Search ────────────────────────────────────────────────────────────────────
default_depth = 5
respect_gitignore = true
show_hidden = false
follow_symlinks = false

# ── Ranking ───────────────────────────────────────────────────────────────────
frecency_weight = 0.3

# ── UI ────────────────────────────────────────────────────────────────────────
default_mode = "dirs"               # "dirs" | "files" | "bookmarks"
default_preview_tab = "overview"    # "overview" | "tree" | "git" | "readme"
show_keybind_bar = true
preview_collapse_width = 100
max_results = 200

# ── External tools ────────────────────────────────────────────────────────────
chafa_bin   = "chafa"
glow_bin    = "glow"
csvlens_bin = "csvlens"
bat_bin     = "bat"

# ── Actions ───────────────────────────────────────────────────────────────────
# {} = single selected path, {*} = all selected paths (multi-select)
[actions.normal]
c     = { cmd = "__prowl_cd", mode = "builtin" }
y     = { cmd = "__prowl_yank_abs", mode = "builtin" }
enter = { cmd = "code {}", mode = "detach" }
e     = { cmd = "nvim {*}", mode = "suspend" }
t     = { cmd = "wezterm cli new-tab -- zsh -c 'cd {}; exec zsh'", mode = "detach" }
m     = { cmd = "glow {}", mode = "suspend" }
v     = { cmd = "csvlens {}", mode = "suspend" }
g     = { cmd = "open $(git -C {} remote get-url origin 2>/dev/null)", mode = "detach" }
o     = { cmd = "xdg-open {}", mode = "detach" }

[actions.insert]
enter  = { cmd = "code {}", mode = "detach" }
ctrl_e = { cmd = "nvim {*}", mode = "suspend" }
ctrl_o = { cmd = "__prowl_cd", mode = "builtin" }

# Tab-specific overrides (flat "mode-tab" keys)
[actions.normal-files]
enter = { cmd = "nvim {*}", mode = "suspend" }

[actions.insert-files]
enter = { cmd = "nvim {*}", mode = "suspend" }

# ── Custom tags ───────────────────────────────────────────────────────────────
# [[custom_tags]]
# name = "monorepo"
# color = "magenta"
# markers = ["lerna.json", "pnpm-workspace.yaml", "turbo.json"]

# ── Theme ─────────────────────────────────────────────────────────────────────
[theme]
preset = "default"    # "default" | "catppuccin-mocha" | "gruvbox" | "nord" | "tokyo-night"

# ── Data paths ────────────────────────────────────────────────────────────────
[paths]
frecency_db  = "~/.local/share/prowl/frecency.db"
bookmarks    = "~/.local/share/prowl/bookmarks.toml"
lastdir_file = "~/.cache/prowl/lastdir"
```

---

## Themes & Terminal Compatibility

### Built-in themes

Five presets: `default`, `catppuccin-mocha`, `gruvbox`, `nord`, `tokyo-night`. Each defined as a TOML file in the `themes/` directory. Custom themes can be provided as a file path in config.

### Terminal support

| Terminal | Image rendering | Notes |
|---|---|---|
| Kitty | Kitty graphics protocol (pixel-perfect) | Best image quality |
| WezTerm | Sixel protocol | Excellent quality |
| iTerm2 | iTerm2 inline image protocol | High quality |
| Ghostty | Kitty graphics protocol | Excellent quality |
| Alacritty | Unicode half-block, 24-bit color | Good quality |
| foot | Sixel | High quality |
| xterm | Unicode half-block, 256 color | Acceptable |
| tmux | Passthrough to host terminal | Depends on host |
| Windows Terminal | Unicode half-block | Works, no images |
| SSH sessions | Unicode half-block, adaptive | Degrades gracefully |

**Colour support:** 256-colour minimum. Falls back to 8-colour mode for terminals reporting fewer colours. Plain ASCII with no styling for `TERM=dumb`.

---

## External Tool Integrations

### chafa

Image preview via subprocess. Protocol auto-selected based on terminal detection. If not installed, images show a placeholder with dimensions and file size.

### glow

Full rendered Markdown preview via suspend/resume. In the preview pane, Prowl renders a plain-text stripped version inline.

### csvlens

Interactive CSV browsing via suspend/resume. Preview pane shows first 20 rows as aligned columns.

### bat

Fallback for syntax highlighting when `syntect` grammars don't cover a file type. Only invoked as a subprocess in this edge case.

### lazygit

Not in default config but a natural addition:
```toml
l = { cmd = "lazygit -p {}", mode = "suspend" }
```

### Setup script

`prowl --setup` detects the OS/package manager and offers to install optional tools:

```
prowl setup
─────────────
✓ chafa        installed (v1.14)
✗ glow         not found — brew install glow? [y/n]
✗ csvlens      not found — cargo install csvlens? [y/n]
✓ bat          installed (v0.24)
✗ lazygit      not found — brew install lazygit? [y/n]

Shell integration:
✗ zsh          not configured — add to ~/.zshrc? [y/n]
```

Supports `apt`, `brew`, `pacman`, `cargo`, `nix` detection. Non-interactive mode: `prowl --setup --auto`.

### Health check

`prowl --health` shows: installed tools + versions, terminal capabilities detected, config file location + validity, frecency DB size + entry count, active roots and whether they exist.

---

## Error Handling & Edge Cases

**Missing external tools:** Logged at startup (viewable with `prowl --verbose`), feature disabled or degraded. Never crashes on a missing optional dependency.

**Permission denied:** Directories that cannot be read are silently skipped during traversal.

**Broken symlinks:** Shown with a distinct dimmed style and "→ broken" indicator.

**Empty roots:** Warning in the status bar, continues with remaining roots.

**Config parse errors:** Clear error message with line number, falls back to built-in defaults entirely.

**Database corruption:** Frecency DB integrity check on startup. If corrupt, renamed to `.bak`, fresh one created, warning logged.

**Very large directories:** Preview tree limited to 200 entries per level with `… N more` indicator. Full tree available in tree sub-tab with lazy loading.

**Very long paths:** Truncated in the middle with ellipsis: `~/dev/rust/…/src/handlers.rs`.

---

## Distribution & Packaging

### cargo-dist

Automated CI pipeline, builds on every `git tag v*` push:

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.14.0"
ci                 = ["github"]
installers         = ["shell", "homebrew"]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
```

### Installation methods

- Shell script: `curl -fsSL .../install.sh | sh`
- Homebrew: `brew install you/tap/prowl`
- Cargo: `cargo install prowl`
- Nix flake
- `.deb` via `cargo-deb`
- AUR: `prowl-bin`
- Manual: download binary, place in `$PATH`

### Binary characteristics

Release build with LTO: ~8-12 MB on Linux (static musl), ~6-9 MB on macOS. Fully self-contained, no runtime dependencies.

---

## Performance Characteristics

| Operation | Target | Notes |
|---|---|---|
| Cold startup to first frame | < 5ms | Config read + first render |
| Keystroke to result update | < 16ms | One frame at 60fps |
| Full traversal, 3 roots, depth 5 | < 500ms | Background thread, results stream in |
| Preview render (code file) | < 20ms | `syntect` in-process |
| Preview render (image, chafa) | < 100ms | Subprocess, async |
| Frecency read | < 2ms | SQLite, indexed by path |
| Frecency write (on exit) | < 5ms | Single row upsert |

Traversal and matching are async — the UI is always responsive. A spinner in the search bar indicates when traversal is still running.

---

## Project File Structure

```
prowl/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── README.md
├── CHANGELOG.md
├── LICENSE
│
├── src/
│   ├── main.rs                   # CLI parsing, init, launch
│   ├── app.rs                    # App struct, event loop, state machine
│   │
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs             # panel sizing, responsive breakpoints
│   │   ├── tabs.rs               # tab bar rendering
│   │   ├── search.rs             # search input widget
│   │   ├── results.rs            # result list + fuzzy highlight rendering
│   │   ├── preview.rs            # preview pane dispatcher
│   │   ├── keybind_bar.rs        # contextual keybind bar
│   │   ├── filter_bar.rs         # inline filter input
│   │   ├── root_picker.rs        # ctrl+r overlay
│   │   ├── depth_picker.rs       # ctrl+d inline picker
│   │   └── help.rs               # ? overlay
│   │
│   ├── search/
│   │   ├── mod.rs
│   │   ├── walker.rs             # ignore-crate traversal, root management
│   │   ├── query.rs              # query parser → ParsedQuery
│   │   ├── scorer.rs             # nucleo integration + frecency blend
│   │   └── detector.rs           # project type tag detection
│   │
│   ├── preview/
│   │   ├── mod.rs
│   │   ├── dispatcher.rs         # file type detection → preview strategy
│   │   ├── directory.rs          # tree + git status + README
│   │   ├── code.rs               # syntect syntax highlighting
│   │   ├── image.rs              # chafa subprocess
│   │   ├── csv.rs                # aligned column renderer
│   │   ├── markdown.rs           # strip + plain text render
│   │   └── hex.rs                # binary hex dump
│   │
│   ├── actions.rs                # keybind → command dispatch, suspend/resume
│   ├── config.rs                 # toml deserialisation, defaults, validation
│   ├── frecency.rs               # SQLite visit tracking, scoring, decay
│   ├── bookmarks.rs              # toml-backed bookmark store
│   ├── tags.rs                   # project type tag types + display
│   ├── theme.rs                  # colour theme loading + built-in presets
│   ├── health.rs                 # --health diagnostics
│   ├── setup.rs                  # --setup prerequisite installer
│   │
│   └── shell/
│       ├── mod.rs
│       ├── zsh.rs
│       ├── bash.rs
│       └── fish.rs
│
├── shell/
│   ├── prowl.zsh
│   ├── prowl.bash
│   └── prowl.fish
│
├── themes/
│   ├── default.toml
│   ├── catppuccin-mocha.toml
│   ├── gruvbox.toml
│   ├── nord.toml
│   └── tokyo-night.toml
│
└── dist/
    └── cargo-dist.toml
```

---

## Development Milestones

### v0.1 — Functional core
- Single root dir search via `ignore` crate
- Fuzzy results via `nucleo` (traverse once at startup, re-score on keystroke)
- Vim-style modal input (insert + normal modes)
- Query parser (fuzzy, exact, prefix, negation, tag tokens)
- Overview preview (2-level tree + README snippet)
- Preview scroll keybindings (`Shift+Up`/`Shift+Down`)
- Action dispatch: `detach`, `suspend`, `builtin` modes
- Default actions: `enter` → VSCode, `e` → Neovim, `c` → cd, `y` → yank
- Shell integration via `--init`
- Config file loading and validation

### v0.2 — Rich preview
- All file type previews: `syntect` for code, `chafa` for images, stripped markdown, CSV columns
- Preview sub-tabs (overview, tree, git, readme)
- Async preview rendering with cancellation tokens
- Preview LRU cache (20 entries)
- Terminal capability detection for image rendering
- Git ahead/behind indicator in results list

### v0.3 — Ranking & filters
- Frecency tracking via SQLite
- Nucleo + frecency score blend
- Zoxide import (`--import-zoxide`)
- `Ctrl+f` filter bar with tag, recency, depth, frecency tier filters
- Status bar with dismissible filter chips
- Project type detection for all 15 built-in tags
- Custom tag detection via config
- Inline project stats (`rust 12k loc`)

### v0.4 — Files tab + bookmarks
- Files mode with per-filetype action dispatch
- `Ctrl+b` bookmark pin/unpin
- Bookmarks tab with custom labels
- Multi-select (`Space` to mark, action applies to all)
- Path back-stack (`Ctrl+p` popup, last 10 actioned paths)
- Session restore (last query + scroll position per tab)
- `glow` and `csvlens` suspend/resume integration

### v0.5 — Multi-root + UX polish
- Root picker overlay (`Ctrl+r`)
- Per-tab independent root sets
- Depth picker (`Ctrl+d`)
- Responsive layout (narrow mode, preview toggle)
- Help overlay (`?`)
- Theme presets (5 built-in)
- Config validation (`--check-config`)

### v1.0 — Distribution
- `cargo-dist` CI pipeline for all platforms
- Homebrew tap
- Nix flake
- `.deb` and AUR packages
- Setup script (`--setup` / `--setup --auto`)
- Health check (`--health`)
- Stable config schema with migration support

---

## Future Roadmap

**Content search mode.** A fourth tab (`Ctrl+/`) that runs `ripgrep` inside results and shows matching lines inline in the preview pane. Select a match to jump directly to that line in Neovim.

**Git log browser.** A preview sub-tab showing `git log --oneline`, navigable with arrow keys. Selecting a commit shows the diff.

**Session groups.** Named groups of bookmarks that can be activated together — e.g. a "work" group that activates the `~/work` root only.

**Remote filesystem support.** SSH roots (e.g. `ssh://user@host:~/dev`) traversed via `sftp`, opened with `ssh` + `nvim` remote editing.

**Plugin system.** User-defined Lua scripts (via `mlua`) for custom preview renderers and action resolvers.

**TUI rework for v2.** Dynamic multi-pane workspace with draggable splits.

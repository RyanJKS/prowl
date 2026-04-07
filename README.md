# prowl

A fast, single-binary TUI for navigating and opening anything in your filesystem.

Invoke `p`, fuzzy-search for a directory, see a preview on the right, and press a key to act on it — open in an editor, `cd` into it, copy the path, or run any custom command. Keyboard-driven. Zero runtime dependencies.

---

## Installation

### From source

Requires [Rust](https://rustup.rs) stable (1.75+).

```sh
git clone https://github.com/RyanJKS/prowl
cd prowl
cargo install --path .
```

The `prowl` binary will be placed in `~/.cargo/bin/`. Make sure that directory is on your `$PATH`.

---

## Shell integration

Shell integration gives you the `p` wrapper function that handles `cd`-ing into directories selected in prowl. Without it, prowl still works, but directory navigation won't change your shell's working directory.

Add the appropriate line to your shell config:

**zsh** (`~/.zshrc`):
```sh
eval "$(prowl --init zsh)"
```

**bash** (`~/.bashrc`):
```sh
eval "$(prowl --init bash)"
```

**fish** (`~/.config/fish/config.fish`):
```fish
prowl --init fish | source
```

Restart your shell (or `source` the config file) after adding the line.

---

## Usage

```sh
p                  # search from configured roots (or cwd)
p ~/projects       # search from a specific directory
prowl --config     # open config in $EDITOR
```

### Input modes

Prowl has two input modes, similar to Vim:

| Mode | How to enter | Description |
|------|-------------|-------------|
| **Insert** | Default on launch, or press `i` / `/` from Normal | Type to search |
| **Normal** | Press `Esc` from Insert | Navigate and act on results |

### Keybindings

**Insert mode**

| Key | Action |
|-----|--------|
| Type | Filter results |
| `Enter` | Open in VSCode (detached) |
| `Ctrl+E` | Open in Neovim |
| `Ctrl+O` | `cd` into directory |
| `Esc` | Clear query (if non-empty) / switch to Normal mode |
| `←` / `→` | Move cursor in search input |
| `Backspace` | Delete character |

**Normal mode**

| Key | Action |
|-----|--------|
| `j` / `k` | Move selection up/down |
| `Enter` | Open in VSCode (detached) |
| `e` | Open in Neovim |
| `c` | `cd` into directory |
| `y` | Copy path to clipboard |
| `o` | Open with `xdg-open` |
| `i` / `/` | Switch to Insert mode |
| `q` / `Esc` | Quit |

**Both modes**

| Key | Action |
|-----|--------|
| `Ctrl+J` / `Ctrl+K` | Move selection up/down |
| `↑` / `↓` | Scroll preview |
| `PgUp` / `PgDn` | Scroll preview (10 lines) |
| `Ctrl+C` | Quit |

### Query syntax

| Prefix | Example | Effect |
|--------|---------|--------|
| *(none)* | `myproject` | Fuzzy match against full path |
| `!` | `!node_modules` | Exclude paths containing this string |
| `^` | `^/home/ryan/work` | Restrict results to this directory prefix |
| `'` | `'api` | Require exact substring match |
| `#` | `#rust` | Tag filter (parsed, reserved for future use) |
| `\` | `\!literal` | Escape a special character |

Tokens are whitespace-separated. You can combine them: `myproject !node_modules 'src`

---

## Configuration

Config file location (XDG-compliant):

```
~/.config/prowl/config.toml
```

Run `prowl --config` to open it in `$EDITOR`. The file is created with defaults if it doesn't exist.

### Options

| Key | Default | Description |
|-----|---------|-------------|
| `roots` | `[]` | Directories to walk on startup. Falls back to CLI path or `cwd`. |
| `default_depth` | `5` | Maximum recursion depth (1 = immediate children only) |
| `respect_gitignore` | `true` | Honour `.gitignore` / `.ignore` files |
| `show_hidden` | `false` | Include hidden directories (names starting with `.`) |
| `follow_symlinks` | `false` | Follow symbolic links during traversal |
| `max_results` | `200` | Maximum number of results shown in the list |
| `show_keybind_bar` | `true` | Show the keybind hint bar at the bottom |
| `preview_collapse_width` | `100` | Terminal width below which the preview pane hides |

### Custom actions

Actions are defined per-mode under `[actions.normal]` and `[actions.insert]`. Each action has a `cmd` and a `mode`:

- **`detach`** — spawn the command in the background and return to the shell
- **`suspend`** — pause the TUI, run the command in the foreground, then restore the TUI
- **`builtin`** — a prowl built-in (`__prowl_cd` or `__prowl_yank`)

Path substitution in `cmd`:
- `{}` — the selected path (shell-quoted)
- `{*}` — all selected paths (shell-quoted, space-separated)

Example — add a key to open in a terminal at that directory:

```toml
[actions.normal]
t = { cmd = "wezterm start --cwd {}", mode = "detach" }
```

---

## License

MIT

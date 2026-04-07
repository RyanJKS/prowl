# Contributing to prowl

## Prerequisites

- [Rust](https://rustup.rs) stable toolchain (1.75+)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build

```sh
git clone https://github.com/RyanJKS/prowl
cd prowl
cargo build
```

Run in development:

```sh
cargo run                  # launch from cwd
cargo run -- ~/projects    # launch from a specific directory
cargo run -- --init zsh    # print shell integration script
```

## Project structure

```
src/
  main.rs          CLI parsing (clap), shell init, config-open, root resolution
  lib.rs           Crate root — re-exports public modules
  app.rs           App state, event loop, input handling, preview generation
  config.rs        Config struct, ActionDef, ActionsConfig, PathsConfig, TOML loading
  actions.rs       ActionMode, path substitution, action resolution
  ui/
    mod.rs         Top-level draw() — composes all panes
    layout.rs      Terminal area splitting (search / results / preview)
    search_input.rs  Search box widget
    results.rs     Results list widget
    preview.rs     Preview pane widget
    keybind_bar.rs Keybind hint bar at the bottom
  search/
    mod.rs         Re-exports
    query.rs       Query parser — fuzzy / negation / prefix / exact / tag tokens
    walker.rs      Directory traversal via the `ignore` crate
  shell/
    mod.rs         Dispatches to per-shell init scripts
    zsh.rs         zsh `p()` wrapper
    bash.rs        bash `p()` wrapper
    fish.rs        fish `p` function
  default_config.toml  Embedded default config (written on `prowl --config` if missing)
build.rs           Embeds git hash into the binary version string
```

## Tests

```sh
cargo test
```

Tests live alongside the code in `#[cfg(test)]` modules. The main coverage is in `src/search/query.rs` (query parser) and `src/actions.rs` (path substitution and action resolution).

The `tempfile` dev-dependency is available for tests that need temporary directories.

## Code style

```sh
cargo fmt          # format
cargo clippy       # lint (warnings are treated as errors in CI)
```

The project follows standard Rust idioms. Keep modules focused — each file in `src/ui/` owns one widget, each file in `src/shell/` owns one shell's integration script.

## Pull requests

1. Branch off `main`.
2. Keep changes focused — one concern per PR.
3. Run `cargo fmt`, `cargo clippy`, and `cargo test` before opening the PR.
4. Describe *what* and *why* in the PR description, not just *what changed*.

use std::process;

use anyhow::Result;
use clap::{Parser, ValueEnum};

use prowl::config::Config;
use prowl::shell;

/// A fast, single-binary TUI for navigating, previewing, and opening anything
/// in your filesystem.
#[derive(Debug, Parser)]
#[command(
    name = "prowl",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("PROWL_GIT_HASH"), ")"),
    about = "Fast fuzzy filesystem navigator",
    long_about = None,
)]
struct Cli {
    /// Root directory to search (defaults to first config root, or cwd)
    path: Option<String>,

    /// Print shell integration script for the given shell and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    init: Option<Shell>,

    /// Open the prowl config file in $EDITOR
    #[arg(long)]
    config: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum Shell {
    Zsh,
    Bash,
    Fish,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("prowl: error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // --init: print shell integration script and exit
    if let Some(shell) = cli.init {
        let shell_name = match shell {
            Shell::Zsh => "zsh",
            Shell::Bash => "bash",
            Shell::Fish => "fish",
        };
        print!("{}", shell::init_script(shell_name));
        return Ok(());
    }

    // --config: open config in $EDITOR
    if cli.config {
        return open_config_in_editor();
    }

    // Default: launch the TUI
    let config = Config::load()?;

    let root = determine_root(&cli.path, &config);

    prowl::app::run(config, root)
}

/// Determine the root directory to search.
///
/// Priority: CLI path > first config root > cwd
fn determine_root(cli_path: &Option<String>, config: &Config) -> String {
    if let Some(p) = cli_path {
        return p.clone();
    }

    let roots = config.expanded_roots();
    if let Some(first) = roots.first() {
        return first.as_str().to_string();
    }

    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

/// Open the config file in $EDITOR. Creates it from the default template if it
/// does not exist yet.
fn open_config_in_editor() -> Result<()> {
    use std::fs;

    let config_path = prowl::config::config_path();

    // Create parent directory and default config if needed
    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent.as_std_path())?;
        }
        let default_contents = include_str!("default_config.toml");
        fs::write(config_path.as_std_path(), default_contents)?;
        eprintln!("prowl: created default config at {config_path}");
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = process::Command::new(&editor)
        .arg(config_path.as_std_path())
        .status()?;

    if !status.success() {
        anyhow::bail!("editor '{editor}' exited with non-zero status");
    }

    Ok(())
}

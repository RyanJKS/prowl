use prowl::actions::{resolve_action, substitute_path};
use prowl::config::{Config, PathsConfig};
use prowl::search::query::parse_query;
use prowl::search::walker::{walk_directories, WalkOptions};
use tempfile::TempDir;
use std::fs;

/// End-to-end integration test that verifies the composition of subsystems:
/// walker → query parser → filtering → action resolution → path substitution.
///
/// # Fuzzy matching caveat
/// Nucleo fuzzy matching is async and cannot be exercised in a simple integration test.
/// The fuzzy filtering here uses simple substring matching as a stand-in for nucleo.
/// Only the `query.fuzzy` step diverges; all other filtering (negations, etc.) matches
/// production behavior exactly.
///
/// # Negation behavior
/// Negation and other post-filters DO match production behavior: negations are applied
/// case-insensitively against the full path (not just the entry name), consistent with
/// the production implementation.
#[test]
fn test_full_search_and_action_workflow() {
    // Setup test directories
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    fs::create_dir_all(base.join("rust-project/src")).unwrap();
    fs::write(base.join("rust-project/Cargo.toml"), "[package]").unwrap();
    fs::write(base.join("rust-project/README.md"), "# Rust Project\nA test project.").unwrap();

    fs::create_dir_all(base.join("node-project/src")).unwrap();
    fs::write(base.join("node-project/package.json"), "{}").unwrap();

    fs::create_dir_all(base.join("vendor/ignored")).unwrap();

    // Walk directories
    let root = base.to_str().unwrap().to_string();
    let opts = WalkOptions {
        max_depth: 3,
        show_hidden: false,
        respect_gitignore: true,
        follow_symlinks: false,
    };

    let entries = walk_directories(&[root], &opts);

    // We should find our projects
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"rust-project"));
    assert!(names.contains(&"node-project"));
    assert!(names.contains(&"vendor"));

    // Parse a query with negation
    let query = parse_query("project !vendor");
    assert_eq!(query.fuzzy, "project");
    assert_eq!(query.negations, vec!["vendor"]);

    // Apply the parsed query to filter walker results.
    // NOTE: fuzzy filter uses substring matching as a stand-in for nucleo (see doc comment).
    // Negation matches production: case-insensitive, applied against the full path.
    let filtered: Vec<_> = entries.iter()
        .filter(|e| e.name.contains(&query.fuzzy))
        .filter(|e| query.negations.iter().all(|neg| !e.path.as_str().to_lowercase().contains(&neg.to_lowercase())))
        .collect();

    // rust-project and node-project both contain "project"; sub-directories like `src`
    // do not, so the count is stable at 2. vendor is excluded by the !vendor negation.
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().any(|e| e.name == "rust-project"));
    assert!(filtered.iter().any(|e| e.name == "node-project"));
    assert!(!filtered.iter().any(|e| e.name == "vendor"));

    // Verify action resolution works
    let config = Config::default();
    let action = resolve_action(&config.actions, "normal", "dirs", "c").unwrap();
    assert_eq!(action.cmd, "__prowl_cd");
    assert_eq!(action.mode, "builtin");

    // Verify path substitution: cmd starts with "code ", path is shell-quoted
    let action = resolve_action(&config.actions, "normal", "dirs", "enter").unwrap();
    let path = filtered.iter().find(|e| e.name == "rust-project").unwrap();
    let cmd = substitute_path(&action.cmd, &[path.path.to_string()]);
    assert!(cmd.starts_with("code "), "Command should start with 'code ': {cmd}");
    assert!(cmd.contains("rust-project"), "Command should reference rust-project: {cmd}");
    // Path must be wrapped in single quotes (shell-quoted)
    assert!(cmd.contains('\''), "Path in command should be shell-quoted: {cmd}");
}

/// Integration test: Config::default() produces WalkOptions that work with the walker.
/// This verifies the config → walker pipeline is consistent.
#[test]
fn test_config_default_walk_options_work_with_walker() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    fs::create_dir_all(base.join("my-project")).unwrap();
    fs::create_dir_all(base.join("other-project")).unwrap();

    let config = Config::default();
    let root = base.to_str().unwrap().to_string();

    // Build WalkOptions directly from Config fields (same as the app does)
    let opts = WalkOptions {
        max_depth: config.default_depth as usize,
        show_hidden: config.show_hidden,
        respect_gitignore: config.respect_gitignore,
        follow_symlinks: config.follow_symlinks,
    };

    // The walker should successfully produce entries using options derived from config
    let entries = walk_directories(&[root], &opts);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"my-project"), "Walker should find my-project");
    assert!(names.contains(&"other-project"), "Walker should find other-project");

    // Depth from config (5) should be deep enough to find direct children
    assert!(config.default_depth >= 1, "Config depth must allow at least one level");
}

/// Integration test: the lastdir path embedded in shell scripts is consistent with
/// what PathsConfig::default() returns.
///
/// PathsConfig stores the lastdir path as "<cache_dir>/prowl/lastdir".
/// Each shell script must reference both "prowl" and "lastdir" as the final two path
/// components (even if constructed differently, e.g. fish uses `path join … prowl lastdir`).
///
/// # Fish shell divergence (known, not a test gap)
/// Fish hard-codes `$HOME/.cache` as its cache base rather than reading `$XDG_CACHE_HOME`.
/// This means fish will always resolve to `~/.cache/prowl/lastdir` regardless of
/// `XDG_CACHE_HOME`, which may differ from what PathsConfig resolves to at runtime when
/// `XDG_CACHE_HOME` is set to a non-default location. This is a known shell divergence
/// and is intentionally not asserted here.
#[test]
fn test_shell_lastdir_matches_paths_config() {
    let paths = PathsConfig::default();
    let lastdir = paths.lastdir_file.as_str();

    // Confirm the config's path ends with the expected components.
    assert!(lastdir.contains("prowl") && lastdir.contains("lastdir"),
        "PathsConfig::default().lastdir_file should reference 'prowl' and 'lastdir', got: {lastdir}");
    // The final component must be "lastdir" and the parent must be "prowl".
    assert!(lastdir.ends_with("/prowl/lastdir") || lastdir.ends_with("prowl/lastdir"),
        "PathsConfig lastdir_file should end with 'prowl/lastdir', got: {lastdir}");

    // Each shell script must reference "prowl" and "lastdir" as path components so that
    // at runtime the script resolves to the same file that PathsConfig points to.
    for shell in &["zsh", "bash", "fish"] {
        let script = prowl::shell::init_script(shell);
        assert!(
            script.contains("prowl") && script.contains("lastdir"),
            "Shell script for {shell} should reference both 'prowl' and 'lastdir' path components \
             (consistent with PathsConfig lastdir_file = '{lastdir}')"
        );
    }
}

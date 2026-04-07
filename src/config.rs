use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

/// Returns the path to the prowl config file, respecting XDG_CONFIG_HOME.
pub fn config_path() -> Utf8PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Utf8PathBuf::from(xdg).join("prowl").join("config.toml")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        Utf8PathBuf::from(home)
            .join(".config")
            .join("prowl")
            .join("config.toml")
    }
}

// ---------------------------------------------------------------------------
// ActionDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionDef {
    pub cmd: String,
    pub mode: String,
}

// ---------------------------------------------------------------------------
// ActionsConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionsConfig {
    #[serde(default)]
    pub normal: HashMap<String, ActionDef>,

    #[serde(default)]
    pub insert: HashMap<String, ActionDef>,

    #[serde(default, rename = "normal-files")]
    pub normal_files: HashMap<String, ActionDef>,

    #[serde(default, rename = "insert-files")]
    pub insert_files: HashMap<String, ActionDef>,

    #[serde(default, rename = "normal-bookmarks")]
    pub normal_bookmarks: HashMap<String, ActionDef>,

    #[serde(default, rename = "insert-bookmarks")]
    pub insert_bookmarks: HashMap<String, ActionDef>,
}

impl Default for ActionsConfig {
    fn default() -> Self {
        let mut normal = HashMap::new();
        normal.insert(
            "enter".to_string(),
            ActionDef {
                cmd: "code {}".to_string(),
                mode: "detach".to_string(),
            },
        );
        normal.insert(
            "e".to_string(),
            ActionDef {
                cmd: "nvim {*}".to_string(),
                mode: "suspend".to_string(),
            },
        );
        normal.insert(
            "c".to_string(),
            ActionDef {
                cmd: "__prowl_cd".to_string(),
                mode: "builtin".to_string(),
            },
        );
        normal.insert(
            "y".to_string(),
            ActionDef {
                cmd: "__prowl_yank".to_string(),
                mode: "builtin".to_string(),
            },
        );
        normal.insert(
            "o".to_string(),
            ActionDef {
                cmd: "xdg-open {}".to_string(),
                mode: "detach".to_string(),
            },
        );

        let mut insert = HashMap::new();
        insert.insert(
            "enter".to_string(),
            ActionDef {
                cmd: "code {}".to_string(),
                mode: "detach".to_string(),
            },
        );
        insert.insert(
            "ctrl_e".to_string(),
            ActionDef {
                cmd: "nvim {*}".to_string(),
                mode: "suspend".to_string(),
            },
        );
        insert.insert(
            "ctrl_o".to_string(),
            ActionDef {
                cmd: "__prowl_cd".to_string(),
                mode: "builtin".to_string(),
            },
        );

        Self {
            normal,
            insert,
            normal_files: HashMap::new(),
            insert_files: HashMap::new(),
            normal_bookmarks: HashMap::new(),
            insert_bookmarks: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// PathsConfig
// ---------------------------------------------------------------------------

fn default_frecency_db() -> Utf8PathBuf {
    data_dir().join("frecency.db")
}

fn default_bookmarks() -> Utf8PathBuf {
    data_dir().join("bookmarks.toml")
}

fn default_lastdir_file() -> Utf8PathBuf {
    cache_dir().join("lastdir")
}

fn cache_dir() -> Utf8PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        Utf8PathBuf::from(xdg).join("prowl")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        Utf8PathBuf::from(home).join(".cache").join("prowl")
    }
}

fn data_dir() -> Utf8PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        Utf8PathBuf::from(xdg).join("prowl")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        Utf8PathBuf::from(home).join(".local").join("share").join("prowl")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_frecency_db")]
    pub frecency_db: Utf8PathBuf,

    #[serde(default = "default_bookmarks")]
    pub bookmarks: Utf8PathBuf,

    #[serde(default = "default_lastdir_file")]
    pub lastdir_file: Utf8PathBuf,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            frecency_db: default_frecency_db(),
            bookmarks: default_bookmarks(),
            lastdir_file: default_lastdir_file(),
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn default_roots() -> Vec<String> {
    vec![]
}

fn default_default_depth() -> u32 {
    5
}

fn default_respect_gitignore() -> bool {
    true
}

fn default_show_hidden() -> bool {
    false
}

fn default_follow_symlinks() -> bool {
    false
}

fn default_frecency_weight() -> f64 {
    0.3
}

fn default_default_mode() -> String {
    "dirs".to_string()
}

fn default_default_preview_tab() -> String {
    "preview".to_string()
}

fn default_show_keybind_bar() -> bool {
    true
}

fn default_preview_collapse_width() -> u32 {
    100
}

fn default_max_results() -> usize {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_roots")]
    pub roots: Vec<String>,

    #[serde(default = "default_default_depth")]
    pub default_depth: u32,

    #[serde(default = "default_respect_gitignore")]
    pub respect_gitignore: bool,

    #[serde(default = "default_show_hidden")]
    pub show_hidden: bool,

    #[serde(default = "default_follow_symlinks")]
    pub follow_symlinks: bool,

    #[serde(default = "default_frecency_weight")]
    pub frecency_weight: f64,

    #[serde(default = "default_default_mode")]
    pub default_mode: String,

    #[serde(default = "default_default_preview_tab")]
    pub default_preview_tab: String,

    #[serde(default = "default_show_keybind_bar")]
    pub show_keybind_bar: bool,

    #[serde(default = "default_preview_collapse_width")]
    pub preview_collapse_width: u32,

    #[serde(default = "default_max_results")]
    pub max_results: usize,

    #[serde(default)]
    pub actions: ActionsConfig,

    #[serde(default)]
    pub paths: PathsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: default_roots(),
            default_depth: default_default_depth(),
            respect_gitignore: default_respect_gitignore(),
            show_hidden: default_show_hidden(),
            follow_symlinks: default_follow_symlinks(),
            frecency_weight: default_frecency_weight(),
            default_mode: default_default_mode(),
            default_preview_tab: default_default_preview_tab(),
            show_keybind_bar: default_show_keybind_bar(),
            preview_collapse_width: default_preview_collapse_width(),
            max_results: default_max_results(),
            actions: ActionsConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Config {
    /// Parse a `Config` from a TOML string. Missing fields fall back to defaults.
    pub fn parse(s: &str) -> Result<Self> {
        let config: Config = toml::from_str(s)?;
        Ok(config)
    }

    /// Load the config from the default config file path. Falls back to
    /// `Config::default()` if the file does not exist.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let contents = fs::read_to_string(path.as_std_path())?;
            Self::parse(&contents)
        } else {
            Ok(Self::default())
        }
    }

    /// Expand a path string, replacing a leading `~` with `$HOME`.
    pub fn expand_path(path: &str) -> Utf8PathBuf {
        if let Some(rest) = path.strip_prefix("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            Utf8PathBuf::from(home).join(rest)
        } else if path == "~" {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            Utf8PathBuf::from(home)
        } else {
            Utf8PathBuf::from(path)
        }
    }

    /// Returns the list of roots with `~` expanded to the real home directory.
    pub fn expanded_roots(&self) -> Vec<Utf8PathBuf> {
        self.roots.iter().map(|r| Self::expand_path(r)).collect()
    }
}

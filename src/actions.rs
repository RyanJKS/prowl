use crate::config::{ActionDef, ActionsConfig};

#[derive(Debug, Clone, PartialEq)]
pub enum ActionMode {
    Detach,
    Suspend,
    Builtin,
}

impl ActionMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "suspend" => Self::Suspend,
            "builtin" => Self::Builtin,
            _ => Self::Detach,
        }
    }
}

/// Single-quote a path for POSIX shell, handling embedded single quotes.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            // End quote, escaped single quote, re-open quote
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

pub fn substitute_path(cmd: &str, paths: &[String]) -> String {
    let mut result = cmd.to_string();
    if result.contains("{*}") {
        let escaped: Vec<String> = paths.iter().map(|p| shell_quote(p)).collect();
        result = result.replace("{*}", &escaped.join(" "));
    }
    if result.contains("{}") {
        let first = paths.first().map(|p| p.as_str()).unwrap_or("");
        let escaped = shell_quote(first);
        result = result.replace("{}", &escaped);
    }
    result
}

pub fn resolve_action<'a>(
    actions: &'a ActionsConfig,
    input_mode: &str,
    tab_mode: &str,
    key: &str,
) -> Option<&'a ActionDef> {
    let specific = match (input_mode, tab_mode) {
        ("normal", "files") => actions.normal_files.get(key),
        ("insert", "files") => actions.insert_files.get(key),
        ("normal", "bookmarks") => actions.normal_bookmarks.get(key),
        ("insert", "bookmarks") => actions.insert_bookmarks.get(key),
        _ => None,
    };
    if specific.is_some() {
        return specific;
    }
    match input_mode {
        "normal" => actions.normal.get(key),
        "insert" => actions.insert.get(key),
        _ => None,
    }
}

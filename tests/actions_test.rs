use prowl::actions::{substitute_path, resolve_action, ActionMode};
use prowl::config::{ActionsConfig, ActionDef};

#[test]
fn test_substitute_single_path() {
    let cmd = "code {}";
    let paths = vec!["/home/user/project".to_string()];
    let result = substitute_path(cmd, &paths);
    assert_eq!(result, "code '/home/user/project'");
}

#[test]
fn test_substitute_multi_path() {
    let cmd = "nvim {*}";
    let paths = vec![
        "/home/user/file1.rs".to_string(),
        "/home/user/file2.rs".to_string(),
    ];
    let result = substitute_path(cmd, &paths);
    assert_eq!(result, "nvim '/home/user/file1.rs' '/home/user/file2.rs'");
}

#[test]
fn test_substitute_path_with_spaces() {
    let cmd = "code {}";
    let paths = vec!["/home/user/my project".to_string()];
    let result = substitute_path(cmd, &paths);
    assert_eq!(result, "code '/home/user/my project'");
}

#[test]
fn test_substitute_path_with_single_quotes() {
    let cmd = "code {}";
    let paths = vec!["/home/user/it's a dir".to_string()];
    let result = substitute_path(cmd, &paths);
    assert!(result.starts_with("code "));
    assert!(result.contains("it"));
}

#[test]
fn test_substitute_builtin_no_placeholder() {
    let cmd = "__prowl_cd";
    let paths = vec!["/home/user/project".to_string()];
    let result = substitute_path(cmd, &paths);
    assert_eq!(result, "__prowl_cd");
}

#[test]
fn test_action_mode_from_string() {
    assert_eq!(ActionMode::from_str("detach"), ActionMode::Detach);
    assert_eq!(ActionMode::from_str("suspend"), ActionMode::Suspend);
    assert_eq!(ActionMode::from_str("builtin"), ActionMode::Builtin);
    assert_eq!(ActionMode::from_str("unknown"), ActionMode::Detach);
}

#[test]
fn test_resolve_action_normal_files_overrides_normal() {
    let mut actions = ActionsConfig::default();
    actions.normal.insert("enter".into(), ActionDef { cmd: "code {}".into(), mode: "detach".into() });
    actions.normal_files.insert("enter".into(), ActionDef { cmd: "nvim {*}".into(), mode: "suspend".into() });
    let action = resolve_action(&actions, "normal", "files", "enter");
    assert!(action.is_some());
    let action = action.unwrap();
    assert_eq!(action.cmd, "nvim {*}");
    assert_eq!(action.mode, "suspend");
}

#[test]
fn test_resolve_action_falls_back_to_base() {
    let mut actions = ActionsConfig::default();
    actions.normal.insert("e".into(), ActionDef { cmd: "nvim {*}".into(), mode: "suspend".into() });
    let action = resolve_action(&actions, "normal", "files", "e");
    assert!(action.is_some());
    assert_eq!(action.unwrap().cmd, "nvim {*}");
}

#[test]
fn test_resolve_action_no_match() {
    let actions = ActionsConfig::default();
    let action = resolve_action(&actions, "normal", "dirs", "z");
    assert!(action.is_none());
}

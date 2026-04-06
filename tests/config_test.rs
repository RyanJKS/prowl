use prowl::config::Config;

#[test]
fn test_default_config_has_sensible_values() {
    let config = Config::default();
    assert_eq!(config.default_depth, 5);
    assert!(config.respect_gitignore);
    assert!(!config.show_hidden);
    assert!(!config.follow_symlinks);
    assert_eq!(config.frecency_weight, 0.3);
    assert_eq!(config.default_mode, "dirs");
    assert_eq!(config.max_results, 200);
    assert!(config.show_keybind_bar);
    assert_eq!(config.preview_collapse_width, 100);
}

#[test]
fn test_config_from_toml_string() {
    let toml_str = r#"
        roots = ["~/projects"]
        default_depth = 3
        show_hidden = true
    "#;
    let config = Config::from_str(toml_str).unwrap();
    assert_eq!(config.roots, vec!["~/projects"]);
    assert_eq!(config.default_depth, 3);
    assert!(config.show_hidden);
    assert!(config.respect_gitignore);
}

#[test]
fn test_config_invalid_toml_returns_error() {
    let toml_str = "this is [[[not valid toml";
    let result = Config::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn test_config_actions_parsing() {
    let toml_str = r#"
        roots = ["~/dev"]

        [actions.normal]
        enter = { cmd = "code {}", mode = "detach" }
        e = { cmd = "nvim {*}", mode = "suspend" }
        c = { cmd = "__prowl_cd", mode = "builtin" }

        [actions.insert]
        enter = { cmd = "code {}", mode = "detach" }

        [actions.normal-files]
        enter = { cmd = "nvim {*}", mode = "suspend" }
    "#;
    let config = Config::from_str(toml_str).unwrap();
    let normal = &config.actions.normal;
    assert_eq!(normal.get("enter").unwrap().cmd, "code {}");
    assert_eq!(normal.get("enter").unwrap().mode, "detach");
    assert_eq!(normal.get("e").unwrap().mode, "suspend");
    assert_eq!(normal.get("c").unwrap().mode, "builtin");
    let insert = &config.actions.insert;
    assert_eq!(insert.get("enter").unwrap().cmd, "code {}");
    let normal_files = &config.actions.normal_files;
    assert_eq!(normal_files.get("enter").unwrap().cmd, "nvim {*}");
}

#[test]
fn test_config_paths_defaults() {
    let config = Config::default();
    assert!(config.paths.lastdir_file.as_str().contains("prowl/lastdir"));
}

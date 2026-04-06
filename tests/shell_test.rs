use prowl::shell;

#[test]
fn test_zsh_init_contains_function() {
    let output = shell::init_script("zsh");
    assert!(output.contains("p()"));
    assert!(output.contains("prowl"));
    assert!(output.contains("builtin cd"));
    assert!(output.contains("prowl/lastdir"));
}

#[test]
fn test_bash_init_contains_function() {
    let output = shell::init_script("bash");
    assert!(output.contains("p()"));
    assert!(output.contains("prowl"));
    assert!(output.contains("builtin cd"));
    assert!(output.contains("prowl/lastdir"));
}

#[test]
fn test_fish_init_contains_function() {
    let output = shell::init_script("fish");
    assert!(output.contains("function p"));
    assert!(output.contains("prowl"));
    assert!(output.contains("builtin cd"));
    assert!(output.contains("lastdir"));
}

#[test]
fn test_unknown_shell_returns_error_message() {
    let output = shell::init_script("powershell");
    assert!(output.contains("Unsupported shell"));
}

mod bash;
mod fish;
mod zsh;

pub fn init_script(shell: &str) -> String {
    match shell {
        "zsh" => zsh::init().to_string(),
        "bash" => bash::init().to_string(),
        "fish" => fish::init().to_string(),
        other => format!("# Unsupported shell: {other}\n# Supported shells: zsh, bash, fish\n"),
    }
}

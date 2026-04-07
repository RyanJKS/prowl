pub fn init() -> &'static str {
    r#"# Prowl shell integration for fish
function p
    prowl $argv
    set _prowl_lastdir (path join $HOME .cache prowl lastdir)
    if test -f $_prowl_lastdir
        builtin cd (cat $_prowl_lastdir)
        rm -f $_prowl_lastdir
    end
end
"#
}

pub fn init() -> &'static str {
    r#"# Prowl shell integration for zsh
p() {
  prowl "$@"
  local _prowl_lastdir="${XDG_CACHE_HOME:-$HOME/.cache}/prowl/lastdir"
  if [[ -f "$_prowl_lastdir" ]]; then
    builtin cd "$(cat "$_prowl_lastdir")"
    rm -f "$_prowl_lastdir"
  fi
}
"#
}

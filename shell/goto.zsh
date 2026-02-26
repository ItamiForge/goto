# >>> goto integration (managed by goto) >>>
if ! whence -w compdef >/dev/null 2>&1; then
  autoload -Uz compinit
  compinit
fi

goto() {
  if [ "$#" -eq 0 ]; then
    command goto --help
    return
  fi

  case "$1" in
    list|setup|uninstall|__complete|--help|-h|--version|-V)
      command goto "$@"
      return
      ;;
  esac

  local dest
  dest="$(command goto "$@")" || return
  builtin cd "$dest"
}

_goto() {
  local cur="${words[CURRENT]}"
  local -a targets
  targets=("${(@f)$(command goto __complete "$cur")}")
  compadd -Q -U -a targets
}

compdef _goto goto
# <<< goto integration (managed by goto) <<<

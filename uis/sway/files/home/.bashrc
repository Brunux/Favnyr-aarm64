# ~/.bashrc: the interactive shell of the session.
#
# This file replaces Debian's /etc/skel/.bashrc for the desktop user, because the
# terminal is part of what this port ships: `ls` is eza, `cat` is bat, `cd` is
# zoxide, Ctrl-R/Ctrl-T/Alt-C are fzf and the prompt is starship (Deboma's
# starship.toml is at ~/.config/starship.toml). The tools themselves come from
# packages/tools.list and their environment from
# /etc/profile.d/90-cli-tools.sh, which is sourced below so that a login shell
# and a terminal window agree.
#
# Everything is guarded with `command -v`: a tree built without tools.list still
# gets a working shell, just a plain one.

# Non-interactive shells stop here (scp, `ssh host cmd`, sway's exec, cron).
case $- in
	*i*) ;;
	*) return ;;
esac

# --- history ----------------------------------------------------------------
# ignoreboth: no duplicate lines, and a command starting with a space is not
# recorded (the documented way to run a secret once).
HISTCONTROL=ignoreboth
HISTSIZE=32768
HISTFILESIZE=32768
shopt -s histappend checkwinsize

# --- environment ------------------------------------------------------------
# The single copy of the CLI environment: bat's pager and theme, PATH.
[ -r /etc/profile.d/90-cli-tools.sh ] && . /etc/profile.d/90-cli-tools.sh

# --- completion -------------------------------------------------------------
if ! shopt -oq posix && [ -r /usr/share/bash-completion/bash_completion ]; then
	. /usr/share/bash-completion/bash_completion
fi

# --- ls is eza --------------------------------------------------------------
# Deboma's aliases. --icons=auto draws the Nerd Font glyphs only when the
# terminal is a terminal; ~/.config/foot/foot.ini lists Symbols Nerd Font Mono as
# a fallback, which is what makes them render instead of boxes.
if command -v eza >/dev/null 2>&1; then
	alias ls='eza -lh --group-directories-first --icons=auto'
	alias lsa='ls -a'
	alias lt='eza --tree --level=2 --long --icons --git'
	alias lta='lt -a'
fi

# --- cat is bat -------------------------------------------------------------
# --paging=never so that `cat` never takes over the screen (use `bat` itself, or
# MANPAGER for man pages, when you want the pager); --style=plain so it prints
# content and nothing else. bat drops its colours by itself when the output is
# not a terminal, so `cat file | grep` still gets plain text.
if command -v bat >/dev/null 2>&1; then
	alias cat='bat --style=plain --paging=never'
fi

alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

# --- cd is zoxide -----------------------------------------------------------
# The wrapper is defined here; the zoxide initialization itself is the last
# thing in this file, after starship, because starship replaces PROMPT_COMMAND
# and zoxide's hook has to be appended after it (see the end of the file).
if command -v zoxide >/dev/null 2>&1; then
	# `z` alone cannot take a real path, so `cd` keeps working for paths and only
	# jumps by frecency when the argument is not a directory. `-` is bash's own
	# previous directory and not a zoxide query: `builtin cd -` also prints the
	# path, which is what typing `cd -` expects. Deboma prints a
	# Nerd Font glyph once the jump succeeded; the plain path is printed here so
	# the wrapper is also correct in a tty without that font.
	zd() {
		if [ "$#" -eq 0 ]; then
			builtin cd ~ || return
		elif [ "$1" = "-" ]; then
			builtin cd - || return
		elif [ -d "$1" ]; then
			builtin cd "$1" || return
		elif z "$@"; then
			pwd
		else
			printf 'cd: no such directory: %s\n' "$1" >&2
			return 1
		fi
	}
	alias cd='zd'
fi

# --- fzf --------------------------------------------------------------------
# The colours: `lazyos theme set` writes ~/.config/lazyos/fzf.env from the
# theme, so fzf follows the palette like every other surface.
[ -r "$HOME/.config/lazyos/fzf.env" ] && . "$HOME/.config/lazyos/fzf.env"
if command -v fzf >/dev/null 2>&1; then
	# Only the key bindings. fzf's completion.bash rewraps the completion of ~76
	# commands on every shell start, which is ~25 ms before the prompt (Deboma
	# skips it for the same reason).
	[ -r /usr/share/fzf/key-bindings.bash ] && . /usr/share/fzf/key-bindings.bash
	# fzf's own defaults walk the tree with find, .git included; fd knows
	# .gitignore, skips binaries' noise and is what the alias below assumes.
	if command -v fd >/dev/null 2>&1; then
		export FZF_DEFAULT_COMMAND='fd --type f --hidden --follow --exclude .git'
		export FZF_CTRL_T_COMMAND="$FZF_DEFAULT_COMMAND"
		export FZF_ALT_C_COMMAND='fd --type d --hidden --follow --exclude .git'
	fi
fi

# `ff`: pick a file with a preview, `eff` edit what was picked.
ff() {
	fzf --preview 'bat --style=numbers --color=always --line-range :200 {}' "$@"
}
eff() {
	local file
	file=$(ff) || return
	[ -n "$file" ] || return
	"${EDITOR:-nano}" "$file"
}

# --- prompt -----------------------------------------------------------------
# starship replaces PS1/PROMPT_COMMAND wholesale, so nothing here sets a prompt
# when it is missing: /etc/bash.bashrc's plain one is the fallback. That is also
# why zoxide is initialized after this block and not with its functions above.
if command -v starship >/dev/null 2>&1 && [ "${TERM:-}" != dumb ]; then
	eval "$(starship init bash --print-full-init)"
fi

# --- git --------------------------------------------------------------------
# Deboma's short forms; `t` attaches to the multiplexer or starts it.
alias g='git'
alias gcm='git commit -m'
alias gcam='git commit -a -m'
alias gcad='git commit -a --amend'
if command -v tmux >/dev/null 2>&1; then
	alias t='tmux attach || tmux new -s main'
fi

# --- opening things ---------------------------------------------------------
# In a subshell so bash does not announce a background job for every file the
# user opens.
open() (
	xdg-open "$@" >/dev/null 2>&1 &
)

# --- personal additions -----------------------------------------------------
# Aliases and functions of your own belong in ~/.bash_aliases: this file is
# replaced whenever the image is rebuilt, that one is not.
[ -r "$HOME/.bash_aliases" ] && . "$HOME/.bash_aliases"

# --- zoxide, last on purpose ------------------------------------------------
# zoxide appends its directory-tracking hook to PROMPT_COMMAND, and starship
# (above) replaces PROMPT_COMMAND wholesale. Initialized before starship, the
# hook is dropped and the first `cd -` prints "zoxide: detected a possible
# configuration issue"; initialized after it - and after ~/.bash_aliases, which
# could set a PROMPT_COMMAND of its own - the hook is the last thing appended
# and survives. The z/zd functions and `alias cd=zd` stay above: `z` is looked
# up when zd runs, so defining it here is enough.
if command -v zoxide >/dev/null 2>&1; then
	eval "$(zoxide init bash)"
fi

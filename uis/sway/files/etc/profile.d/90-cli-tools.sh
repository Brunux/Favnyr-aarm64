# Environment for the CLI tools in packages/tools.list.
#
# This is the one copy of these settings: /etc/profile.d runs it for login
# shells (SSH, `bash -l`), and ~/.bashrc sources it for the terminal, so the two
# cannot drift apart.
#
# Debian installs bat and fd-find as batcat and fdfind; scripts/rootfs-setup.sh
# adds the /usr/local/bin/bat and /usr/local/bin/fd symlinks that these settings
# and the aliases call.

# bat as the manual pager. col -bx does what man's own pager does: strip the
# overstrike (bold is "b\bbo\bold") and expand tabs, so bat receives plain text
# (bat -l man says "this is a man page", -p drops the header, line numbers and
# git status). MANROFFOPT=-c is man's side of it: pass -c to nroff, which keeps
# the overstrike this pipeline expects instead of producing SGR sequences.
export MANROFFOPT="-c"
export MANPAGER="sh -c 'col -bx | bat -l man -p'"

# `ansi` makes bat paint with the terminal's own 16 colours instead of one of its
# own truecolour themes; those 16 are the solarized palette in
# ~/.config/foot/foot.ini, so a file previewed by bat matches the session.
export BAT_THEME=ansi

# User-installed binaries (cargo, pipx, a user-level npm prefix) live here.
# Appended, so a system package always wins over a user copy of the same name.
# Deboma exports EDITOR=omarchy-launch-editor at this point; there is no launcher
# wrapper here, so EDITOR stays unset and the aliases fall back to nano.
if [ -n "${HOME:-}" ]; then
	case ":$PATH:" in
		*":$HOME/.local/bin:"*) ;;
		*) PATH="${PATH:+$PATH:}$HOME/.local/bin" ;;
	esac
	export PATH
fi

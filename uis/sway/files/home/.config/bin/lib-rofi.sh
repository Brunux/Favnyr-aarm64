#!/bin/sh
# Shared plumbing for the rofi menus in this directory.  Sourced, never run.
#
# One visual language for every list the session opens: a glyph in cyan, the
# name in bold base2, a dim detail (state, size, path), and the key of the row
# kept in a second column of a temporary TSV.  The key is what the caller acts
# on, because rofi hands back the *rendered* row and markup cannot be parsed
# back (the same reason rofi-keys keeps a table).
#
# The colours are the ones in ~/.config/rofi/solarized.rasi, repeated here
# because Pango markup is not a rofi theme; keeping them in one file is what
# stops two menus from drifting apart.
#
# Icons are font glyphs, not .desktop images: rofi resolving images through
# gdk-pixbuf costs 13 MB of PSS for ten rows, while a glyph costs nothing,
# inherits the palette and is already how the bar draws itself.  The launcher and
# the window switcher are the two places where real application icons are worth
# that cost, and config.rasi enables them for exactly those two modes - plus
# rofi-keys, whose list has one picture per section and asks the icon theme for it
# by name (-show-icons).
#
# A row is one line and rofi cannot scroll it sideways: no listview or textbox
# property moves the text horizontally (-scroll-method only chooses how the list
# moves between pages), so a long value is drawn at a smaller size and, past the
# width of the box, cut with an ellipsis that keeps its end (ui_trim).  A list
# whose rows still need more room asks for a wider window (ui_width), and the
# full value is always in the notification the choice raises.
#
# Notifications (mako) carry a named icon instead of a glyph: ui_say's fourth
# argument, resolved through the icon theme that ~/.config/mako/config points at.

UI_BIN=$(cd -- "$(dirname -- "$0")" && pwd)

UI_CYAN="#2aa198"
UI_BLUE="#268bd2"
UI_DIM="#586e75"
UI_TEXT="#93a1a1"
UI_STRONG="#eee8d5"
UI_YELLOW="#b58900"
UI_RED="#dc322f"
UI_GREEN="#859900"
UI_VIOLET="#6c71c4"

# The state dots used by the switch lists: filled for what is on, hollow for
# what is off.  Two characters that exist in every font, unlike a check mark.
UI_ON='●'
UI_OFF='○'

# Icons: Font Awesome 4.7 by codepoint (package fonts-font-awesome-legacy),
# written as the UTF-8 bytes of the codepoint.  POSIX sh has no \u escape, and a
# literal private-use character does not survive every editor or transport, which
# is how the first version of these menus shipped a row of empty spans.
UI_I_TERM=$(printf '\357\204\240')   # f120
UI_I_FILES=$(printf '\357\201\273')   # f07b
UI_I_WINDOW=$(printf '\357\213\220')  # f2d0
UI_I_DISK=$(printf '\357\202\240')   # f0a0
UI_I_COG=$(printf '\357\200\223')   # f013
UI_I_BOOK=$(printf '\357\200\255')   # f02d
UI_I_KEYBOARD=$(printf '\357\204\234')   # f11c
UI_I_USERS=$(printf '\357\203\200')   # f0c0
UI_I_WIFI=$(printf '\357\207\253')   # f1eb
UI_I_GLOBE=$(printf '\357\202\254')   # f0ac
UI_I_BT=$(printf '\357\212\223')   # f293
UI_I_LINK=$(printf '\357\203\201')   # f0c1
UI_I_VOLUME=$(printf '\357\200\250')   # f028
UI_I_MUSIC=$(printf '\357\200\201')   # f001
UI_I_BELL=$(printf '\357\203\263')   # f0f3
UI_I_BELL_OFF=$(printf '\357\207\266')   # f1f6
UI_I_CLOCK=$(printf '\357\200\227')   # f017
UI_I_TRASH=$(printf '\357\207\270')   # f1f8
UI_I_BUG=$(printf '\357\206\210')   # f188
UI_I_CALC=$(printf '\357\207\254')   # f1ec
UI_I_MIC=$(printf '\357\204\260')   # f130
UI_I_LOCK=$(printf '\357\200\243')   # f023
UI_I_EYE=$(printf '\357\201\256')   # f06e
UI_I_PAUSE=$(printf '\357\201\214')   # f04c
UI_I_MOON=$(printf '\357\206\206')   # f186
UI_I_OUT=$(printf '\357\213\265')   # f2f5
UI_I_REBOOT=$(printf '\357\200\241')   # f021
UI_I_POWER=$(printf '\357\200\221')   # f011
UI_I_SEARCH=$(printf '\357\200\202')   # f002
UI_I_IMAGE=$(printf '\357\200\276')   # f03e
UI_I_VIDEO=$(printf '\357\200\275')   # f03d
UI_I_FILE=$(printf '\357\205\234')   # f15c
UI_I_HOME=$(printf '\357\200\225')   # f015
UI_I_WARN=$(printf '\357\201\261')   # f071
UI_I_UNDO=$(printf '\357\203\242')   # f0e2
UI_I_PLAY=$(printf '\357\201\213')   # f04b
UI_I_NEXT=$(printf '\357\201\221')   # f051
UI_I_PREV=$(printf '\357\201\210')   # f048
UI_I_STOP=$(printf '\357\201\215')   # f04d
UI_I_ROTATE=$(printf '\357\200\241')   # f021
UI_I_MOUSE=$(printf '\357\211\205')   # f245
UI_I_BATTERY=$(printf '\357\211\200')   # f240
UI_I_BOX=$(printf '\357\206\207')   # f187
UI_I_CODE=$(printf '\357\204\241')   # f121
UI_I_CAMERA=$(printf '\357\200\260')   # f030
UI_I_CROP=$(printf '\357\204\245')   # f125
UI_I_QR=$(printf '\357\200\251')   # f029
UI_I_CLIPBOARD=$(printf '\357\203\252')   # f0ea
UI_I_SMILE=$(printf '\357\204\230')   # f118
UI_I_INFO=$(printf '\357\201\232')   # f05a
UI_I_CLOUD=$(printf '\357\203\202')   # f0c2
UI_I_BRUSH=$(printf '\357\207\274')   # f1fc
UI_I_TAG=$(printf '\357\200\253')   # f02b
UI_I_DOWNLOAD=$(printf '\357\200\231')   # f019
UI_I_KEY=$(printf '\357\202\204')   # f084
UI_I_DESKTOP=$(printf '\357\204\210')   # f108
UI_I_CHIP=$(printf '\357\213\233')   # f2db

# The notification icons are *names*, not glyphs: mako resolves them through the
# icon theme, and ~/.config/mako/config puts /usr/share/icons/LazyOS on its
# icon-path.  The names below are the ones scripts/make-icons.sh renders into
# that theme (its index.theme inherits Adwaita and hicolor for everything it does
# not ship), so a notification answers "what is this about" the same way a row
# does - and a name that does not resolve draws nothing, never an error.
UI_ICON_INFO=dialog-information
UI_ICON_ERROR=dialog-error
UI_ICON_CLIP=clipboard-history
UI_ICON_EMOJI=emoji-picker
UI_ICON_KEYS=preferences-desktop-keyboard
UI_ICON_TRASH=user-trash
UI_ICON_IMAGE=image-x-generic
UI_ICON_FOLDER=folder
# A download and a disk: both names are already in the theme for the windowed
# file manager (scripts/make-icons.sh, fourth table), so a new feature asks for a
# picture that exists instead of adding a name nobody renders.
UI_ICON_DOWNLOAD=folder-download
UI_ICON_DISK=drive-harddisk
# The three the file actions and the two searches ask for: an archive, a match
# and a film.  All three are already in the theme's third table (the file
# browser draws them on its rows), which is why the constants name them instead
# of adding a picture.
UI_ICON_BOX=package-x-generic
UI_ICON_SEARCH=system-search
UI_ICON_VIDEO=video-x-generic

# The third column of a row is the one that grows: a path, a revision string, a
# package description.  Past this many characters it is cut (ui_trim).  The number
# is measured, not guessed: rendering rows in a headless sway at the panel's
# 1280 px and reading where the text stops in the screenshot gives 11 px per bold
# name column and 8 px per detail column (Pango "small" of the theme's 12), and the
# 62% window leaves 741 px of text between its padding - 60 detail characters next
# to the longest name in these menus (17) with room to spare.
UI_DETAIL_MAX=60


# Pango treats &, < and > as markup; a tab or a newline would break the TSV or
# split the row.  Everything a menu shows goes through here.
ui_esc() {
	printf '%s' "${1:-}" | tr -d '\n\t' |
		sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

# ui_trim TEXT [MAX]: shorten TEXT to MAX characters and mark where it was cut.
# A value that looks like a path keeps its *end* (…/clipboard/clip-20261021.png),
# because the last components are the ones that identify it; anything else keeps
# its beginning, because a state ("connected · 192.0.2.1 · wlp1s0") says what it
# is at the front.  Either way the row says that it was cut instead of letting the
# visible part pass for the whole value, and the notification on selection is
# where the whole value is printed.  Done with parameter expansion and not
# cut(1): a row already costs two forks in ui_esc, and a menu can have hundreds.
ui_trim() {
	_ui_text=${1:-}
	_ui_max=${2:-$UI_DETAIL_MAX}
	[ "${#_ui_text}" -gt "$_ui_max" ] || { printf '%s' "$_ui_text"; return 0; }
	if [ "$_ui_text" != "${_ui_text#*/}" ]; then
		# Something is left of the last slash that still fits; walk in from the
		# left until a tail fits, so as much of the path as possible is kept.
		_ui_rest=${_ui_text#*/}
		while [ -n "$_ui_rest" ]; do
			if [ $((${#_ui_rest} + 2)) -le "$_ui_max" ]; then
				printf '…/%s' "$_ui_rest"
				return 0
			fi
			case $_ui_rest in
			*/*) _ui_rest=${_ui_rest#*/} ;;
			*) break ;;
			esac
		done
	fi
	# No slash, or even the last component is too long: keep the head.
	_ui_head=
	_ui_left=$((_ui_max - 1))
	while [ "$_ui_left" -gt 0 ] && [ -n "$_ui_text" ]; do
		_ui_head=$_ui_head${_ui_text%"${_ui_text#?}"}
		_ui_text=${_ui_text#?}
		_ui_left=$((_ui_left - 1))
	done
	printf '%s…' "$_ui_head"
}

# ui_row GLYPH NAME [DETAIL] [KEY] [COLOUR]
# The colour is for the glyph: cyan means available/on, dim means off, yellow
# means "this needs you", red means destructive.  The name stays at the theme's
# size and in bold (it is what is read first and what rofi matches), the detail is
# drawn at Pango's "small" and is the only column ui_trim shortens, so a long path
# or a long description no longer pushes the whole row into rofi's own ellipsis,
# which loses its end and gives no way to see it.
ui_row() {
	printf '<span foreground="%s">%s</span>   <span weight="bold" foreground="%s">%s</span>   <span size="small" foreground="%s">%s</span>\t%s\n' \
		"${5:-$UI_CYAN}" "${1:-}" "$UI_STRONG" "$(ui_esc "${2:-}")" \
		"$UI_DIM" "$(ui_esc "$(ui_trim "${3:-}")")" "${4:--}"
}

# ui_note TEXT [DETAIL]: a row that explains something and does nothing.
ui_note() {
	printf '<span foreground="%s">%s</span>  <span foreground="%s">%s</span>\t-\n' \
		"$UI_DIM" "$(ui_esc "${1:-}")" "$UI_DIM" "$(ui_esc "${2:-}")"
}

# ui_heading NAME: a section break.  It is a row like any other (rofi cannot do
# anything else), and ui_pick answers "none" for it so a menu can ignore it and
# stay open.
ui_heading() {
	printf '<span foreground="%s">── %s ──</span>\tnone\n' \
		"$UI_DIM" "$(ui_esc "${1:-}")"
}

# The ChromeOS kernel partition type.  A disk that carries one is a disk this
# firmware can load a kernel from, which makes it a system disk and never a
# candidate: the eMMC and both cards of this machine are excluded by it, whatever
# their device names are (p1 of the layout in scripts/lib-cgpt.sh).
UI_KERN_TYPE=fe3a2a5d-4f32-41a7-b725-accc3285a309

# ui_disks PREFIX [COLOUR]: one row per disk this session may write to.
#
# The safety is the list, not the question, and this is the one copy of it: a disk
# with anything mounted on it is not offered (that excludes the running root,
# /boot and every stick udisks mounted - and a busy disk would fail halfway
# anyway), a disk carrying a **ChromeOS kernel partition** is not offered, and
# zram, loop and optical devices are not disks a person writes to.  What is left
# has nothing this system needs on it; the caller still confirms, but it can only
# confirm one of these.
#
# PREFIX is the key the caller gets back (`fmt:sda`, `swap:mmcblk1`), so the same
# rows can feed two menus without either one guessing what the other does.
ui_disks() {
	_ui_disk_prefix=${1:-disk}
	_ui_disk_colour=${2:-$UI_YELLOW}
	lsblk -dnro NAME,SIZE,MODEL,TRAN 2>/dev/null | while read -r name size model tran; do
		[ -n "${name:-}" ] || continue
		# zram, loop, ram and optical devices are not disks a person writes to.
		# Neither is the SPI flash (mtd*, the firmware this port must never
		# touch) nor the eMMC's boot0/boot1 hardware partitions (mmcblk*boot*,
		# 4 MB each, listed by lsblk as disks of their own).  The last group is
		# pseudo/foreign block devices a session has no business formatting.
		case $name in
		zram* | loop* | ram* | sr* | mtd* | mmcblk*boot* | mmcblk*rpmb | dm-* | md* | nbd* | rbd*)
			continue
			;;
		esac
		# Anything mounted, on the disk itself or on a partition of it.  An
		# active swap partition shows [SWAP] here, so it is excluded too.
		_ui_disk_mnt=$(lsblk -nro MOUNTPOINT "/dev/$name" 2>/dev/null | tr -d '[:space:]' || true)
		[ -z "$_ui_disk_mnt" ] || continue
		# A bootable disk: one PARTTYPE read, matched with case so that no
		# `producer | grep -q` pipeline can report a false negative.
		_ui_disk_types=$(lsblk -nro PARTTYPE "/dev/$name" 2>/dev/null || true)
		case $_ui_disk_types in *"$UI_KERN_TYPE"*) continue ;; esac
		# -r escapes a space in a model as \x20; the row shows a model, not an
		# escape sequence ("USB\x20DISK\x203.0" is not what a person reads).
		_ui_disk_model=$(printf '%s' "${model:-unknown model}" | sed 's/\\x20/ /g')
		ui_row "$UI_I_DISK" "/dev/$name" \
			"$_ui_disk_model · $size${tran:+ · $tran}" "$_ui_disk_prefix:$name" "$_ui_disk_colour"
	done
}

# ui_width FILE: the window width, as a rofi distance, that the rows in FILE need.
# Rofi cannot scroll a row sideways, so a row wider than the box is cut with an
# ellipsis and the end - which for a path is the part that identifies it - is what
# is lost.  The theme's own 45% (576 px of this panel's 1280, 525 px of text
# between the rim's padding) is the launcher's width, sized for an application name
# and nothing beside it; a list of rows is drawn wider, and how much wider is the
# measured pixel width of its longest row:
#
#	45%  525 px of text - 48 detail columns next to an eight-column name
#	62%  741 px - what a name and a 60-column detail need
#	76%  922 px - a 60-column name and a block name beside it
#	88%  1075 px - the longest emoji name and its block
#
# The columns are 11 px (bold name) and 8 px (small detail) and the row opens with
# 44 px of glyph and gaps; all four numbers come from rendering the menus in a
# headless sway and reading the screenshots, and the steps are the four widths that
# the theme is written around rather than a percentage per list, so two menus that
# hold the same kind of text are the same size.  Past 88% the row is drawn as wide
# as the panel allows and rofi cuts what is left; ui_trim keeps that from happening
# to details, and the notification each menu raises carries the whole value.
ui_width() {
	_ui_px=$(cut -f1 "$1" | awk '
		function span(re,   s) {
			if (!match($0, re)) return ""
			s = substr($0, RSTART, RLENGTH)
			sub(/^[^>]*>/, "", s)
			sub(/<.*/, "", s)
			return s
		}
		{
			name = span("<span weight=\"bold\"[^>]*>[^<]*</span>")
			detail = span("<span size=\"small\"[^>]*>[^<]*</span>")
			if (name == "" && detail == "") {
				# A row with neither span (a plain note): measure all of it, wide.
				plain = $0
				gsub(/<[^>]*>/, "", plain)
				px = 44 + 11 * length(plain)
			} else {
				px = 44 + 11 * length(name) + 8 * length(detail)
			}
			if (px > m) m = px
		}
		END { print m + 0 }')
	case ${_ui_px:-0} in
	'' | *[!0-9]*) _ui_px=0 ;;
	esac
	if [ "$_ui_px" -le 525 ]; then
		printf '45%%'
	elif [ "$_ui_px" -le 741 ]; then
		printf '62%%'
	elif [ "$_ui_px" -le 922 ]; then
		printf '76%%'
	else
		printf '88%%'
	fi
}

# ui_pick PROMPT [MESSAGE] < rows: prints the chosen key, exits 1 on Escape.
ui_pick() {
	_ui_file=$(mktemp "${TMPDIR:-/tmp}/lazyos-menu.XXXXXX") || return 1
	cat >"$_ui_file"
	_ui_width=$(ui_width "$_ui_file")
	if [ -n "${2:-}" ]; then
		_ui_choice=$(cut -f1 "$_ui_file" |
			"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows -mesg "$2" \
				-theme-str "window { width: $_ui_width; } mainbox { children: [ inputbar, message, listview ]; }") ||
			{ rm -f "$_ui_file"; return 1; }
	else
		_ui_choice=$(cut -f1 "$_ui_file" |
			"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows \
				-theme-str "window { width: $_ui_width; }") ||
			{ rm -f "$_ui_file"; return 1; }
	fi
	_ui_key=$(awk -F'\t' -v c="$_ui_choice" '$1 == c { print $2; exit }' "$_ui_file")
	rm -f "$_ui_file"
	[ -n "${_ui_choice:-}" ] || return 1
	[ "$_ui_key" = '-' ] && return 1
	printf '%s\n' "$_ui_key"
}

# ui_pick_alt PROMPT [MESSAGE] [ALT] < rows
# ui_pick with one extra action: Enter exits 0, the extra binding (rofi's
# -kb-custom-1) exits 10 with the row still on stdout, which is how the emoji
# picker offers "copy" next to "insert" and the clipboard offers "delete" next to
# "copy back".  Prints the key for Enter and alt:KEY for the extra binding; exits 1
# when the window is dismissed.
#
# ALT is either the binding alone ("Alt+1") or the binding and what it does
# ("Alt+1 delete").  The label is worth having on screen, but rofi parses the
# whole value of -kb-custom-1 as one keybinding, with no room in it for a
# description: "alt+1 delete" is a syntax error it reports as
#   Failed to set binding alt+1 ... for: Custom Keybinding 1 (kb-custom-1)
# and the menu never opens at all.  So only the first word of ALT reaches rofi, and
# the rest is added to the message line - unless that message already names the
# binding, which both of this port's callers do ("Alt+1 deletes that entry"), in
# which case repeating it would be noise.
ui_pick_alt() {
	_ui_file=$(mktemp "${TMPDIR:-/tmp}/lazyos-menu.XXXXXX") || return 1
	cat >"$_ui_file"
	_ui_bind=${3:-Alt+1}
	_ui_bind=${_ui_bind%% *}
	_ui_what=${3:-}
	_ui_what=${_ui_what#"$_ui_bind"}
	_ui_msg=${2:-}
	case $_ui_msg in
	*"$_ui_bind"*) ;;
	*) [ -z "$_ui_what" ] || _ui_msg="${_ui_msg:+$_ui_msg · }$_ui_bind$_ui_what" ;;
	esac
	_ui_rc=0
	_ui_choice=$(cut -f1 "$_ui_file" |
		"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows -mesg "$_ui_msg" \
			-kb-custom-1 "$_ui_bind" \
			-theme-str "window { width: $(ui_width "$_ui_file"); } mainbox { children: [ inputbar, message, listview ]; }") || _ui_rc=$?
	_ui_key=$(awk -F'\t' -v c="${_ui_choice:-}" '$1 == c { print $2; exit }' "$_ui_file")
	rm -f "$_ui_file"
	[ -n "${_ui_choice:-}" ] || return 1
	[ "$_ui_key" = '-' ] && return 1
	case $_ui_rc in
	0) printf '%s\n' "$_ui_key" ;;
	10) printf 'alt:%s\n' "$_ui_key" ;;
	*) return 1 ;;
	esac
}

# ui_say TITLE BODY [URGENCY] [ICON]
# The icon is a name the notification's theme resolves (icon-path in
# ~/.config/mako/config); dialog-information is the default because most of these
# are answers to something that was just done.  Errors say so with dialog-error
# and the caller that knows better (a copied clipboard entry, a deleted one) says
# that instead.
ui_say() {
	notify-send -a LazyOS -i "${4:-$UI_ICON_INFO}" -u "${3:-normal}" \
		"${1:-LazyOS}" "${2:-}" 2>/dev/null || true
}

# ui_copy TEXT [NOTE]
ui_copy() {
	if printf '%s' "${1:-}" | wl-copy 2>/dev/null; then
		[ -n "${2:-}" ] && ui_say Copied "$2" normal "$UI_ICON_CLIP"
		return 0
	fi
	ui_say Clipboard "wl-copy is not available" critical "$UI_ICON_ERROR"
	return 1
}

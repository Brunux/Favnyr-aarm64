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
# Icons are font glyphs, not .desktop images: rofi resolves images through
# glycin (43 MB of PSS, measured), while a glyph costs nothing, inherits the
# palette and is already how the bar draws itself.  The launcher and the window
# switcher are the two places where real application icons are worth that cost,
# and config.rasi enables them for exactly those two modes.

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
UI_I_DESKTOP=$(printf '\357\204\210')   # f108
UI_I_CHIP=$(printf '\357\213\233')   # f2db


# Pango treats &, < and > as markup; a tab or a newline would break the TSV or
# split the row.  Everything a menu shows goes through here.
ui_esc() {
	printf '%s' "${1:-}" | tr -d '\n\t' |
		sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

# ui_row GLYPH NAME [DETAIL] [KEY] [COLOUR]
# The colour is for the glyph: cyan means available/on, dim means off, yellow
# means "this needs you", red means destructive.
ui_row() {
	printf '<span foreground="%s">%s</span>   <span weight="bold" foreground="%s">%s</span>   <span foreground="%s">%s</span>\t%s\n' \
		"${5:-$UI_CYAN}" "${1:-}" "$UI_STRONG" "$(ui_esc "${2:-}")" \
		"$UI_DIM" "$(ui_esc "${3:-}")" "${4:--}"
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

# ui_pick PROMPT [MESSAGE] < rows: prints the chosen key, exits 1 on Escape.
ui_pick() {
	_ui_file=$(mktemp "${TMPDIR:-/tmp}/nureb-menu.XXXXXX") || return 1
	cat >"$_ui_file"
	if [ -n "${2:-}" ]; then
		_ui_choice=$(cut -f1 "$_ui_file" |
			"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows -mesg "$2" \
				-theme-str 'mainbox { children: [ inputbar, message, listview ]; }') ||
			{ rm -f "$_ui_file"; return 1; }
	else
		_ui_choice=$(cut -f1 "$_ui_file" |
			"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows) ||
			{ rm -f "$_ui_file"; return 1; }
	fi
	_ui_key=$(awk -F'\t' -v c="$_ui_choice" '$1 == c { print $2; exit }' "$_ui_file")
	rm -f "$_ui_file"
	[ -n "${_ui_choice:-}" ] || return 1
	[ "$_ui_key" = '-' ] && return 1
	printf '%s\n' "$_ui_key"
}

# ui_pick_alt PROMPT [MESSAGE] [ALT-LABEL] < rows
# ui_pick with one extra action: Alt+1 (rofi's -kb-custom-1) exits 10 with the
# row still on stdout, which is how the emoji picker offers "copy" next to
# "insert" and the clipboard offers "delete" next to "copy back".  Prints the
# key for Enter and alt:KEY for Alt+1; exits 1 when the window is dismissed.
ui_pick_alt() {
	_ui_file=$(mktemp "${TMPDIR:-/tmp}/nureb-menu.XXXXXX") || return 1
	cat >"$_ui_file"
	_ui_rc=0
	_ui_choice=$(cut -f1 "$_ui_file" |
		"$UI_BIN/bar-pick" "${1:-Menu}" -markup-rows -mesg "${2:-}" \
			-kb-custom-1 "${3:-Alt+1}" \
			-theme-str 'mainbox { children: [ inputbar, message, listview ]; }') || _ui_rc=$?
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

# ui_say TITLE BODY [URGENCY]
ui_say() {
	notify-send -a Nureb -u "${3:-normal}" "${1:-Nureb}" "${2:-}" 2>/dev/null || true
}

# ui_copy TEXT [NOTE]
ui_copy() {
	if printf '%s' "${1:-}" | wl-copy 2>/dev/null; then
		[ -n "${2:-}" ] && ui_say Copied "$2"
		return 0
	fi
	ui_say Clipboard "wl-copy is not available" critical
	return 1
}

#!/bin/bash
# Regenerate the launcher's icons from the icon fonts.
#
# The bar and every menu this session draws put a Font Awesome codepoint in the
# text, because Pango resolves it through fontconfig and it costs nothing.  The
# launcher cannot: rofi's drun mode shows .desktop icons as *images* (glycin),
# never as text.  Debian's Adwaita 51 ships only monochrome symbolic icons and
# not one of this session's names, so a drun row for "Files" had a hole where an
# icon should be, and the only full-colour theme in the archive (Papirus) is
# 190 MB installed.
#
# This is the one script here in bash rather than POSIX sh: it needs printf's
# \u escape to turn a codepoint into a character, and dash does not have one.
#
# So the same codepoints are rendered once into 48x48 PNGs and installed as the
# Nureb icon theme.  Its index.theme inherits Adwaita and hicolor, so an
# application that brings its own icon (foot, btop, htop, anything installed
# later) still shows that one in colour, and only the session's own entries fall
# back to these.
#
# The PNGs are committed, so an image build needs no ImageMagick; run this only
# when the table below changes.  ImageMagick is a *host* tool (scripts/00-deps.sh
# installs it); the font is the one the image ships.
#
#   scripts/make-icons.sh          rewrite the 48x48 PNGs
#
# Font Awesome 4.7 is what fonts-font-awesome-legacy provides, and its codepoints
# are the ones ~/.config/bin/lib-rofi.sh uses, so the launcher, the menus and the
# bar all draw the same alphabet.

set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
theme="$here/uis/sway/files/usr/share/icons/Nureb"
out="$theme/48x48/apps"
alt="$theme/scalable/apps"
# rofi asks for its own icon size and GTK only falls back to a *larger* fixed
# directory, never to a smaller one (a theme with only 26x26 resolved nothing),
# so the same 48x48 PNGs are also reachable through a Scalable entry.  The
# launcher draws them at 22-24 px, where the downscale is invisible.
font=${ICON_FONT:-FontAwesome}
# Solarized cyan, brightened: at 22 px on the launcher's base03 the plain
# #2aa198 reads as a smudge next to 12 pt white text, while #41cfc4 keeps the
# hue and is legible as a glyph.
colour=${ICON_COLOUR:-#41cfc4}
size=${ICON_SIZE:-48}
points=${ICON_POINTS:-30}

command -v convert >/dev/null 2>&1 || {
	echo "make-icons.sh: ImageMagick (convert) is not installed" >&2
	exit 1
}

mkdir -p "$out" "$alt"

# icon name (the .desktop's Icon=) and the Font Awesome 4.7 codepoint.
while read -r name code; do
	case $name in '' | '#'*) continue ;; esac
	glyph=$(printf "\\u$code")
	# A codepoint the font does not have renders an empty canvas, which is the
	# one failure that would look like "the icon theme does not work"; check it.
	case $glyph in '' | *\\u*) echo "make-icons.sh: bad codepoint $code for $name" >&2; exit 1 ;; esac
	convert -size "${size}x${size}" xc:none -gravity center \
		-fill "$colour" -font "$font" -pointsize "$points" \
		-annotate +0+0 "$glyph" "$out/$name.png"
	ink=$(convert "$out/$name.png" -format '%[fx:mean.a]' info:)
	if ! awk -v v="$ink" 'BEGIN { exit !(v + 0 > 0.001) }'; then
		echo "make-icons.sh: $name rendered nothing ($font/$code)" >&2
		exit 1
	fi
	cp -f "$out/$name.png" "$alt/$name.png"
	echo "  $name.png ($code)"
done <<'TABLE'
utilities-terminal                 f120
system-file-manager                f07b
preferences-system                 f013
help-browser                       f02d
preferences-desktop-keyboard       f11c
system-users                       f0c0
accessories-calculator             f1ec
multimedia-player                  f001
preferences-system-notifications   f0f3
network-wireless                   f1eb
bluetooth                          f293
audio-speakers                     f028
system-lock-screen                 f023
system-suspend                     f04c
system-suspend-hibernate           f186
system-shutdown                    f011
preferences-desktop-screensaver    f06e
applets-screenshooter              f030
audio-input-microphone             f130
preferences-desktop-display        f108
system-software-update             f019
user-trash                         f1f8
utilities-system-monitor           f201
preferences-system-time            f017
preferences-system-network         f0ac
applications-development           f121
TABLE

echo "make-icons.sh: $out"

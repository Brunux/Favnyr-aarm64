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
# rofi's *run* mode (the command palette, Alt+Shift+D) needs the same thing for a
# different lookup: it asks the theme for an icon named after the binary it would
# run, so a second table renders those names (foot, btop, nvim, ...).
#
# The PNGs are committed, so an image build needs no ImageMagick; run this only
# when a table below changes.  ImageMagick is a *host* tool (scripts/00-deps.sh
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

# icon name (the Icon= of a .desktop entry, or the binary name of a command) and
# the Font Awesome 4.7 codepoint.  Two tables, one renderer.
render() {
	while read -r name code; do
		case $name in '' | '#'*) continue ;; esac
		glyph=$(printf "\\u$code")
		# A codepoint the font does not have renders an empty canvas, which is the
		# one failure that would look like "the icon theme does not work"; check it.
		case $glyph in '' | *\\u*) echo "make-icons.sh: bad codepoint $code for $name" >&2; exit 1 ;; esac
		convert -size "${size}x${size}" xc:none -gravity center \
			-fill "$colour" -font "$font" -pointsize "$points" \
			-annotate +0+0 "$glyph" -strip "$out/$name.png"
		ink=$(convert "$out/$name.png" -format '%[fx:mean.a]' info:)
		if ! awk -v v="$ink" 'BEGIN { exit !(v + 0 > 0.001) }'; then
			echo "make-icons.sh: $name rendered nothing ($font/$code)" >&2
			exit 1
		fi
		cp -f "$out/$name.png" "$alt/$name.png"
		echo "  $name.png ($code)"
	done
}

# 1. The launcher: one icon per .desktop entry, named after what the entry's
#    Icon= says.
render <<'TABLE'
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
media-record                       f03d
audio-input-microphone             f130
preferences-desktop-display        f108
system-software-update             f019
user-trash                         f1f8
utilities-system-monitor           f201
preferences-system-time            f017
preferences-system-network         f0ac
applications-development           f121
TABLE

# 2. The command palette.  rofi's run mode resolves an icon by *asking the icon
#    theme for the name of the binary it would run* (source/modes/run.c:
#    rofi_icon_fetcher_query(str[0], height)), not from a .desktop entry, so a
#    palette row has an icon only if the theme has an icon with that exact name.
#    Adwaita has none of these.  The element is a fixed 26 px column
#    (solarized.rasi), so the rows a person starts from a palette get a glyph and
#    the system binaries that have none keep their text aligned.
render <<'COMMANDS'
foot                               f120
footclient                         f120
firefox-esr                        f0ac
bash                               f120
dash                               f120
btop                               f201
htop                               f201
top                                f201
fastfetch                          f108
inxi                               f05a
lazygit                            f1d3
git                                f1d3
tig                                f1d3
nvim                               f121
vim                                f121
nano                               f044
micro                              f044
tmux                               f120
zellij                             f120
fzf                                f0b0
fd                                 f0b0
fdfind                             f0b0
rg                                 f002
bat                                f15c
batcat                             f15c
eza                                f07b
ls                                 f07b
ff                                 f07b
yazi                               f07b
ranger                             f07b
ncdu                               f0a0
du                                 f0a0
df                                 f0a0
mpv                                f03d
imv                                f03e
papers                             f02d
zathura                            f02d
man                                f02d
tldr                               f02d
glow                               f02d
swappy                             f030
grimshot                           f030
grim                               f030
slurp                              f030
wf-recorder                        f03d
tesseract                          f031
zbarimg                            f029
qrencode                           f029
yt-dlp                             f019
wget                               f019
curl                               f0c2
ssh                                f084
scp                                f0c1
sftp                               f0c1
rsync                              f0c1
python3                            f121
node                               f121
npm                                f121
ffmpeg                             f03d
playerctl                          f001
pactl                              f028
wpctl                              f028
nmcli                              f1eb
nmtui                              f1eb
bluetoothctl                       f293
systemctl                          f085
journalctl                         f15c
dmesg                              f15c
check-system                       f05a
dictate                            f131
dictate-model                      f019
rofi                               f002
rofi-capture                       f030
rofi-qr                            f029
rofi-files                         f07b
rofi-places                        f07b
rofi-quick                         f013
rofi-learn                         f02d
rofi-tz                            f017
rofi-calc                          f1ec
rofi-keys                          f11c
rofi-notify                        f0f3
rofi-media                         f001
rofi-kill                          f0e7
rofi-clean                         f1f8
rofi-agents                        f0c0
rofi-power                         f011
bar-menu                           f0c9
bar-net                            f1eb
bar-bt                             f293
bar-audio                          f028
bar-power                          f0e7
bar-pick                           f0b0
bar-key                            f11c
bar-updates                        f019
bar-busy                           f0f3
busy                               f0f3
swaymsg                            f085
swaylock                           f023
swayidle                           f04c
sway-suspend                       f04c
sway-hibernate                     f186
sway-screensaver                   f070
wifi.sh                            f1eb
COMMANDS

echo "make-icons.sh: $out"

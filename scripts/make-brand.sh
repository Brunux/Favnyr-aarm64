#!/bin/bash
# The LazyOS mark, rendered from brand/lazyos.svg into the files the image ships.
#
# Usage: scripts/make-brand.sh
#
#   brand/lazyos.svg                                        the source
#   .../icons/LazyOS/48x48/apps/lazyos.png                  the launcher, the About row
#   .../icons/LazyOS/scalable/apps/lazyos.svg               GTK at larger sizes
#   .../home/Pictures/Wallpapers/lazyos.png                 1366x912, the default background
#   .../home/.config/lazyos/wordmark.txt                    fastfetch's logo
#
# The outputs are committed, because build-rootfs.sh stages `git archive HEAD`
# and a build host without rsvg-convert must still be able to build an image.
# Only an edit to the SVG needs this script to run.
#
# Host tool, like make-icons.sh: rsvg-convert is `librsvg2-bin` on Debian and
# `librsvg` on Alpine (scripts/00-deps.sh installs it); the text is drawn with
# JetBrains Mono, which is `fonts-jetbrains-mono` on Debian.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
svg="$here/brand/lazyos.svg"
theme="$here/uis/sway/files/usr/share/icons/LazyOS"
wall="$here/uis/sway/files/home/Pictures/Wallpapers/lazyos.png"
wordmark="$here/uis/sway/files/home/.config/lazyos/wordmark.txt"

# The panel's own mode (docs/optimization.md): generating at the size it is
# shown at avoids a resample, and `fill` in the sway config crops rather than
# stretches if the output ever differs.
width=1366
height=912
mark=420

die() {
	echo "make-brand.sh: $*" >&2
	exit 1
}

command -v rsvg-convert >/dev/null 2>&1 ||
	die "rsvg-convert is not installed (Debian: librsvg2-bin, Alpine: librsvg)"

if command -v magick >/dev/null 2>&1; then
	im=magick
elif command -v convert >/dev/null 2>&1; then
	im=convert
else
	die "ImageMagick is not installed"
fi

# fc-match always answers, with a fallback when the family is missing; ask it
# for the family it resolved and refuse a substitute, because a wallpaper
# typeset in DejaVu is not the brand.
family=$(fc-match -f '%{family}' 'JetBrains Mono' 2>/dev/null || true)
[ "$family" = "JetBrains Mono" ] ||
	die "JetBrains Mono is not installed (Debian: fonts-jetbrains-mono)"
font=$(fc-match -f '%{file}' 'JetBrains Mono')

[ -r "$svg" ] || die "missing $svg"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/lazyos-brand.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$theme/48x48/apps" "$theme/scalable/apps" "$(dirname "$wall")" "$(dirname "$wordmark")"

echo "  mark 48x48  $(basename "$theme")/48x48/apps/lazyos.png"
rsvg-convert -w 48 -h 48 -o "$theme/48x48/apps/lazyos.png" "$svg"

echo "  mark svg    $(basename "$theme")/scalable/apps/lazyos.svg"
install -m 0644 "$svg" "$theme/scalable/apps/lazyos.svg"

rsvg-convert -w "$mark" -h "$mark" -o "$tmp/mark.png" "$svg"

echo "  wallpaper   ${width}x${height}  Pictures/Wallpapers/lazyos.png"
"$im" -size "${width}x${height}" xc:'#002b36' \
	"$tmp/mark.png" -gravity center -geometry +0-55 -composite \
	-font "$font" -pointsize 30 -fill '#2aa198' -gravity south -annotate +0+168 'lazy by design' \
	-pointsize 17 -fill '#839496' -gravity south -annotate +0+116 '» press any key to continue «' \
	-depth 8 -strip "$wall"

# fastfetch's logo: plain text, because a terminal is where it is drawn and the
# block art is the idle screen's.  Two lines are enough next to the modules.
echo "  wordmark    .config/lazyos/wordmark.txt"
printf 'LAZYOS\nlazy by design\n' >"$wordmark"

echo "Done."

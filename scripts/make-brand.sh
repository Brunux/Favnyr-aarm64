#!/bin/bash
# The LazyOS mark, rendered from brand/lazyos.svg into the files the image ships.
#
# Usage: scripts/make-brand.sh
#
#   brand/lazyos.svg                                        the source
#   .../icons/LazyOS/48x48/apps/lazyos.png                  the launcher, the About row
#   .../icons/LazyOS/scalable/apps/lazyos.svg               GTK at larger sizes
#   .../home/.config/lazyos/wordmark.txt                    fastfetch's logo
#
# The desktop wallpaper is not here: it is the session's own mountain
# photograph (uis/sway/files/home/Pictures/Wallpapers/mountains.jpg), not the
# mark, and a wallpaper is a surface to live on rather than a logo.
#
# The outputs are committed, because build-rootfs.sh stages `git archive HEAD`
# and a build host without rsvg-convert must still be able to build an image.
# Only an edit to the SVG needs this script to run.
#
# Host tool, like make-icons.sh: rsvg-convert is `librsvg2-bin` on Debian and
# `librsvg` on Alpine (scripts/00-deps.sh installs it).
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
svg="$here/brand/lazyos.svg"
theme="$here/uis/sway/files/usr/share/icons/LazyOS"
wordmark="$here/uis/sway/files/home/.config/lazyos/wordmark.txt"

die() {
	echo "make-brand.sh: $*" >&2
	exit 1
}

command -v rsvg-convert >/dev/null 2>&1 ||
	die "rsvg-convert is not installed (Debian: librsvg2-bin, Alpine: librsvg)"

[ -r "$svg" ] || die "missing $svg"

mkdir -p "$theme/48x48/apps" "$theme/scalable/apps" "$(dirname "$wordmark")"

echo "  mark 48x48  $(basename "$theme")/48x48/apps/lazyos.png"
rsvg-convert -w 48 -h 48 -o "$theme/48x48/apps/lazyos.png" "$svg"

echo "  mark svg    $(basename "$theme")/scalable/apps/lazyos.svg"
install -m 0644 "$svg" "$theme/scalable/apps/lazyos.svg"

# fastfetch's logo: plain text, because a terminal is where it is drawn and the
# block art is the idle screen's.  Two lines are enough next to the modules.
echo "  wordmark    .config/lazyos/wordmark.txt"
printf 'LAZYOS\nlazy by design\n' >"$wordmark"

echo "Done."

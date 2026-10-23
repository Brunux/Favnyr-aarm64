#!/bin/sh
# make-theme.sh - render the session's surfaces from a theme, on the build host.
#
# This is a host tool like make-icons.sh and make-brand.sh: its output is
# committed, because the image is staged from `git archive HEAD` and a host
# without a built `lazyos` still has to build. The files it writes are the
# solarized ones the session boots with; on the device, `lazyos theme set NAME`
# is what rewrites them, from the same templates and the same engine.
#
#   scripts/make-theme.sh            # solarized, the default
#   scripts/make-theme.sh catppuccin # a preview of another built-in
#
# The rendered files carry a header naming the theme, so `lazyos theme diff`
# (and `check-system.sh`) can tell a hand edit from a stale image.
set -eu

repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
slug=${1:-solarized}
# The install rewrites this to the configured user; the committed files carry
# the default one, exactly like every other /home path in uis/sway/files.
home=/home/brunux

binary=${LAZYOS_BIN:-}
if [ -z "$binary" ]; then
	for candidate in "$repo/rust/target/release/lazyos" "$repo/rust/target/debug/lazyos"; do
		[ -x "$candidate" ] && binary=$candidate && break
	done
fi
if [ -z "$binary" ]; then
	binary=$(command -v lazyos 2>/dev/null || true)
fi
if [ -z "$binary" ] || [ ! -x "$binary" ]; then
	echo "make-theme: no lazyos binary found." >&2
	echo "  build one with: cargo build --release -p lazyos-cli --manifest-path rust/Cargo.toml" >&2
	echo "  or point LAZYOS_BIN at a built binary." >&2
	exit 1
fi

"$binary" theme render --to "$repo/uis/sway/files/home" --home "$home" --theme "$slug"

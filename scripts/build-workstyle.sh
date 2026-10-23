#!/bin/bash
# Build workstyle (workspace icons) from the pinned crates.io tarball and package
# it for Debian.
#
# The bar's workspace pills showed only which number each one was. workstyle
# renames every workspace to "<number>: <icons>", one glyph per window in it, so
# the pill says what is running where - the behaviour Nura had and the reason
# uis/sway kept its config. Debian packages no workspace renamer (upstream's own
# answer is `cargo install workstyle`), so the release tarball is pinned in
# tools/workstyle/ and built here into:
#
#   workstyle_<version>_<arch>.deb   /usr/bin/workstyle
#
# The published crate (0.8.2) is what is built, not the git master: master's
# Cargo.toml still says 0.3.0 against swayipc 2.7, while 0.8.2 is the release the
# config, the lockfile and the deduplication option were tested with. See
# tools/workstyle/README.md for the three patched lines and why.
#
# Building needs `cargo` (scripts/00-deps.sh installs it for CONFIG_RUST=y) and
# network access to crates.io on the first run: Cargo.lock ships inside the
# tarball and --locked makes cargo use exactly those versions, so the build is
# reproducible even though the crates are fetched.
#
# Usage: build-workstyle.sh
# Output: $WORKSTYLE_STAGE/*.deb   (installed by install-workstyle.sh)
set -euo pipefail
. "$(dirname "$0")/lib.sh"

[ -f "$WORKSTYLE_DIR/SHA256SUMS" ] || die "no SHA256SUMS in $WORKSTYLE_DIR"
command -v cargo >/dev/null 2>&1 || die "cargo is not installed (run scripts/00-deps.sh)"
command -v patch >/dev/null 2>&1 || die "patch is not installed (run scripts/00-deps.sh)"

# Debian packaging revision: a change to the packaging itself (not the upstream
# source) bumps this, so apt sees an upgrade.
PACKAGING_REVISION="1lazyos1"
WORKSTYLE_PKG_VERSION="$WORKSTYLE_VERSION-$PACKAGING_REVISION"

log "Fetching workstyle $WORKSTYLE_VERSION"
if [ ! -f "$SRC_DIR/$WORKSTYLE_TARBALL" ]; then
	curl -fL --retry 3 --progress-bar -o "$SRC_DIR/$WORKSTYLE_TARBALL.part" "$WORKSTYLE_URL"
	mv "$SRC_DIR/$WORKSTYLE_TARBALL.part" "$SRC_DIR/$WORKSTYLE_TARBALL"
fi
( cd "$SRC_DIR" && sha256sum -c --ignore-missing "$WORKSTYLE_DIR/SHA256SUMS" ) ||
	die "checksum mismatch for $WORKSTYLE_TARBALL (delete $SRC_DIR/$WORKSTYLE_TARBALL and retry)"

# The source is re-extracted on every run and the patches applied to that fresh
# copy, the way build-kernel.sh does it: a changed patch cannot be silently
# skipped, and no "already prepared" marker can go stale. The compiled objects
# stay in CARGO_TARGET_DIR, outside the tree, so a rebuild is incremental.
log "Extracting workstyle $WORKSTYLE_VERSION"
rm -rf "$WORKSTYLE_SRC"
tar -xf "$SRC_DIR/$WORKSTYLE_TARBALL" -C "$SRC_DIR"
[ -d "$WORKSTYLE_SRC" ] || die "unexpected tarball layout: $WORKSTYLE_SRC is missing"

if [ -f "$WORKSTYLE_DIR/patches/series" ]; then
	while read -r p; do
		case "$p" in '' | '#'*) continue ;; esac
		log "  patch $p"
		patch -d "$WORKSTYLE_SRC" -p1 --no-backup-if-mismatch <"$WORKSTYLE_DIR/patches/$p"
	done <"$WORKSTYLE_DIR/patches/series"
fi

log "Building workstyle $WORKSTYLE_VERSION (cargo --release --locked)"
CARGO_TARGET_DIR="$WORK_DIR/workstyle-target" \
	cargo build --release --locked --manifest-path "$WORKSTYLE_SRC/Cargo.toml"
WORKSTYLE_BIN="$WORK_DIR/workstyle-target/release/workstyle"
[ -x "$WORKSTYLE_BIN" ] || die "cargo produced no binary at $WORKSTYLE_BIN"

log "Packaging"
rm -f "$WORKSTYLE_STAGE"/*.deb
WORKSTYLE_PKG="$WORKSTYLE_STAGE/pkg-$WORKSTYLE"
rm -rf "$WORKSTYLE_PKG"
mkdir -p "$WORKSTYLE_PKG/usr/bin"
install -m 0755 "$WORKSTYLE_BIN" "$WORKSTYLE_PKG/usr/bin/workstyle"
# Cargo's release profile keeps the symbols; Debian's packages are stripped.
strip --strip-unneeded "$WORKSTYLE_PKG/usr/bin/workstyle"

build_deb "$WORKSTYLE_STAGE" "$WORKSTYLE_PKG" "$WORKSTYLE" "$WORKSTYLE_PKG_VERSION" \
	"$(deb_depends "$WORKSTYLE_PKG/usr/bin/workstyle")" \
	"Renames sway/i3 workspaces to the icons of the windows in them (workstyle $WORKSTYLE_VERSION)"

log "workstyle $WORKSTYLE_VERSION packaged in $WORKSTYLE_STAGE"
ls -l "$WORKSTYLE_STAGE"/*.deb
log "Next: scripts/install-workstyle.sh (build rootfs) or scripts/install-workstyle.sh / (running system)"

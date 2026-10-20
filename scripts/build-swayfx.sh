#!/bin/bash
# Build the session compositor from source and package it for Debian.
#
# The session runs a wlroots fork that can round corners, draw shadows and blur
# (Deboma draws the same effects with Hyprland + Quickshell). Debian only
# packages plain sway, so both pieces are built here from the pinned tarballs in
# compositors/swayfx/ and turned into two .debs:
#
#   libscenefx-<v>_<version>_<arch>.deb   the effects renderer (scenefx)
#   swayfx_<version>_<arch>.deb           sway, swaymsg, swaynag, swaybar
#
# They install into /usr like Debian's sway, and the second one
# Provides/Conflicts/Replaces `sway`, so `sway`, /etc/sway and the greeter's
# command line do not change. See compositors/swayfx/README.md.
#
# Usage: build-swayfx.sh
# Output: $COMPOSITOR_STAGE/*.deb   (installed by install-swayfx.sh)
set -euo pipefail
. "$(dirname "$0")/lib.sh"

[ -f "$COMPOSITOR_DIR/SHA256SUMS" ] || die "no SHA256SUMS in $COMPOSITOR_DIR"

ARCH="$(dpkg --print-architecture)"
MULTIARCH="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
# /sbin is not on a normal user's PATH, and this script runs unprivileged.
LDCONFIG="$(command -v ldconfig || printf /sbin/ldconfig)"
# Debian packaging revision: a change to the packaging itself (not the upstream
# source) bumps this, so apt sees an upgrade.
PACKAGING_REVISION="1nureb1"
SCENEFX_PKG_VERSION="$SCENEFX_VERSION-$PACKAGING_REVISION"
COMPOSITOR_PKG_VERSION="$COMPOSITOR_VERSION-$PACKAGING_REVISION"

fetch() {
	# fetch <tarball> <url>: download once, then verify against SHA256SUMS.
	local tarball="$1" url="$2"
	if [ ! -f "$SRC_DIR/$tarball" ]; then
		log "Downloading $tarball"
		curl -fL --retry 3 --progress-bar -o "$SRC_DIR/$tarball.part" "$url"
		mv "$SRC_DIR/$tarball.part" "$SRC_DIR/$tarball"
	fi
	( cd "$SRC_DIR" && sha256sum -c --ignore-missing "$COMPOSITOR_DIR/SHA256SUMS" ) ||
		die "checksum mismatch for $tarball (delete $SRC_DIR/$tarball and retry)"
}

meson_setup() {
	# meson_setup <srcdir> <builddir> [extra options...]
	# Reuses an existing build directory, so a rebuild after a failure only
	# recompiles what changed. werror stays off: the projects enable it by
	# default, and a new compiler warning in a fork is not a reason to fail a
	# Debian image build.
	local src="$1" build="$2"
	shift 2
	if [ ! -f "$build/build.ninja" ]; then
		meson setup "$build" "$src" \
			--prefix=/usr --libdir="lib/$MULTIARCH" \
			--buildtype=release -Dwerror=false "$@"
	fi
}

fetch "$COMPOSITOR_TARBALL" "$COMPOSITOR_URL"
fetch "$SCENEFX_TARBALL" "$SCENEFX_URL"

log "Extracting scenefx $SCENEFX_VERSION and swayfx $COMPOSITOR_VERSION"
[ -d "$SCENEFX_SRC" ] || tar -xf "$SRC_DIR/$SCENEFX_TARBALL" -C "$SRC_DIR"
[ -d "$COMPOSITOR_SRC" ] || tar -xf "$SRC_DIR/$COMPOSITOR_TARBALL" -C "$SRC_DIR"
[ -d "$SCENEFX_SRC" ] || die "unexpected tarball layout: $SCENEFX_SRC is missing"
[ -d "$COMPOSITOR_SRC" ] || die "unexpected tarball layout: $COMPOSITOR_SRC is missing"

SCENEFX_ROOT="$COMPOSITOR_STAGE/scenefx-root"
COMPOSITOR_ROOT="$COMPOSITOR_STAGE/compositor-root"

log "Building scenefx $SCENEFX_VERSION"
meson_setup "$SCENEFX_SRC" "$SCENEFX_SRC/build"
ninja -C "$SCENEFX_SRC/build"
rm -rf "$SCENEFX_ROOT"
DESTDIR="$SCENEFX_ROOT" meson install -C "$SCENEFX_SRC/build" --strip

# swayfx links against the library that was just built, not against one
# installed on the build host. Its pkg-config file is copied with the prefix
# rewritten to the staging tree, so meson resolves -I/-L there; the library
# still has a plain soname (libscenefx-<v>.so) and no rpath, so the installed
# binary picks up the packaged copy from /usr/lib/<triplet> at runtime.
PC_DIR="$COMPOSITOR_STAGE/pkgconfig"
rm -rf "$PC_DIR"
mkdir -p "$PC_DIR"
sed "s|^prefix=.*|prefix=$SCENEFX_ROOT/usr|" \
	"$SCENEFX_ROOT/usr/lib/$MULTIARCH/pkgconfig/scenefx-$SCENEFX_VERSION.pc" >"$PC_DIR/scenefx-$SCENEFX_VERSION.pc"

log "Building swayfx $COMPOSITOR_VERSION"
export PKG_CONFIG_PATH="$PC_DIR${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
# The options mirror Debian's sway package: swaybar, swaynag, the tray (with
# gdk-pixbuf for more icon formats) and man pages are on; the sample wallpaper
# is not shipped, because the session sets its own.
meson_setup "$COMPOSITOR_SRC" "$COMPOSITOR_SRC/build" \
	-Dman-pages=enabled -Dtray=enabled -Dgdk-pixbuf=enabled -Ddefault-wallpaper=false
ninja -C "$COMPOSITOR_SRC/build"
rm -rf "$COMPOSITOR_ROOT"
DESTDIR="$COMPOSITOR_ROOT" meson install -C "$COMPOSITOR_SRC/build" --strip
# Upstream installs its default /etc/sway/config. This port owns that path
# (uis/sway/files/etc/sway), and install-swayfx.sh can run after the rootfs
# overlay, so a packaged copy would replace the tested configuration.
rm -rf "$COMPOSITOR_ROOT/etc"

deb_depends() {
	# deb_depends <file>...: the packages that own the shared libraries the
	# given binaries need, as a comma separated list. dpkg-shlibdeps does this
	# from a debian/ directory; this build has none, so each DT_NEEDED name is
	# resolved to the real file by the host's ldconfig cache and that exact path
	# is asked of dpkg. Asking dpkg for the bare soname is not the same thing:
	# `libc.so.6` also matches the linker script of that name in libc6-dev, and
	# a -dev package would then end up on the target.
	local list="" soname path pkg f
	for f in "$@"; do
		for soname in $(readelf -d "$f" 2>/dev/null | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p'); do
			path="$("$LDCONFIG" -p | awk -v s="$soname" '$1 == s { print $NF; exit }')"
			if [ -z "$path" ]; then
				# libscenefx is built here and packaged by this script, so it is
				# not in the host's cache; anything else means the build host is
				# missing a library the binary will need at runtime.
				case "$soname" in libscenefx-*) ;; *)
					warn "no library named $soname on this host (needed by $(basename "$f"))"
					;;
				esac
				continue
			fi
			pkg="$(dpkg -S "$path" 2>/dev/null | head -n1 | cut -d: -f1)"
			if [ -z "$pkg" ]; then
				warn "$path (needed by $(basename "$f")) belongs to no package"
				continue
			fi
			# Keep the list comma separated without spaces while deduplicating
			# (a pattern cannot look for ", pkg," and ",pkg," at once).
			case ",$list," in
			*",$pkg,"*) ;;
			*) list="${list:+$list,}$pkg" ;;
			esac
		done
	done
	printf '%s\n' "$list" | sed 's/,/, /g'
}

build_deb() {
	# build_deb <tree> <package> <version> <depends> <description> [extra control lines]
	local tree="$1" name="$2" version="$3" depends="$4" desc="$5" extra="${6:-}"
	local size
	size="$(du -sk --exclude=DEBIAN "$tree" | cut -f1)"
	mkdir -p "$tree/DEBIAN"
	{
		printf 'Package: %s\n' "$name"
		printf 'Version: %s\n' "$version"
		printf 'Architecture: %s\n' "$ARCH"
		printf 'Maintainer: %s\n' "Nureb builder <builder@localhost>"
		printf 'Installed-Size: %s\n' "$size"
		if [ -n "$depends" ]; then printf 'Depends: %s\n' "$depends"; fi
		printf 'Section: x11\n'
		printf 'Priority: optional\n'
		if [ -n "$extra" ]; then printf '%s\n' "$extra"; fi
		printf 'Description: %s\n' "$desc"
	} >"$tree/DEBIAN/control"
	dpkg-deb --root-owner-group --build "$tree" \
		"$COMPOSITOR_STAGE/${name}_${version}_${ARCH}.deb" >/dev/null
	log "Packaged ${name}_${version}_${ARCH}.deb"
}

# Only the runtime library ships: the headers and the pkg-config file are build
# inputs, and nothing in the image compiles against scenefx.
log "Packaging"
rm -f "$COMPOSITOR_STAGE"/*.deb
SCENEFX_PKG="$COMPOSITOR_STAGE/pkg-scenefx"
COMPOSITOR_PKG="$COMPOSITOR_STAGE/pkg-$COMPOSITOR"
rm -rf "$SCENEFX_PKG" "$COMPOSITOR_PKG"
mkdir -p "$SCENEFX_PKG/usr/lib/$MULTIARCH"
cp -a "$SCENEFX_ROOT/usr/lib/$MULTIARCH/libscenefx-$SCENEFX_VERSION.so" "$SCENEFX_PKG/usr/lib/$MULTIARCH/"
cp -a "$COMPOSITOR_ROOT" "$COMPOSITOR_PKG"

build_deb "$SCENEFX_PKG" "libscenefx-$SCENEFX_VERSION" "$SCENEFX_PKG_VERSION" \
	"$(deb_depends "$SCENEFX_PKG/usr/lib/$MULTIARCH/libscenefx-$SCENEFX_VERSION.so")" \
	"Effects renderer for wlroots (scenefx $SCENEFX_VERSION)"

build_deb "$COMPOSITOR_PKG" "$COMPOSITOR" "$COMPOSITOR_PKG_VERSION" \
	"$(deb_depends "$COMPOSITOR_PKG"/usr/bin/*), libscenefx-$SCENEFX_VERSION" \
	"i3-compatible Wayland compositor (swayfx $COMPOSITOR_VERSION, based on sway)" \
	"Provides: sway
Conflicts: sway
Replaces: sway"

log "Compositor $COMPOSITOR $COMPOSITOR_VERSION (scenefx $SCENEFX_VERSION) packaged in $COMPOSITOR_STAGE"
ls -l "$COMPOSITOR_STAGE"/*.deb
log "Next: scripts/install-swayfx.sh (build rootfs) or scripts/install-swayfx.sh / (running system)"

#!/bin/bash
# Install the compositor packages built by build-swayfx.sh.
#
# Usage:
#   install-swayfx.sh [ROOTFS]
#
# Without an argument the rootfs built by build-rootfs.sh is used and the
# packages are installed inside a chroot of it; `install-swayfx.sh /` installs
# them on the running system (the counterpart of `install-kpart.sh` for a
# kernel update). apt resolves the dependencies (wlroots, cairo, pango, …) from
# the target's own sources, so nothing here is a hand-written Depends list.
#
# The packages Conflict/Replace/Provide `sway`, so this also upgrades a system
# that still has Debian's plain sway installed.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

[ -n "${1:-}" ] && ROOTFS="$1"
# Resolve before re-execing: sudo may not keep the caller's working directory,
# and a relative path would then point somewhere else.
ROOTFS="$(readlink -f "$ROOTFS" 2>/dev/null || printf '%s' "$ROOTFS")"

[ -d "$ROOTFS" ] || die "rootfs not found: $ROOTFS (run build-rootfs.sh first)"
ls "$COMPOSITOR_STAGE"/*.deb >/dev/null 2>&1 ||
	die "no compositor packages in $COMPOSITOR_STAGE (run build-swayfx.sh first)"

if [ "$(id -u)" -ne 0 ]; then
	# sudo does not preserve the environment, so a BUILD_ROOT/WORK_DIR/DEVICE
	# given to this script would be replaced by its default and the packages
	# would be looked for in the wrong place.
	exec sudo -n \
		BUILD_ROOT="$BUILD_ROOT" WORK_DIR="$WORK_DIR" SRC_DIR="$SRC_DIR" \
		DEVICE="$DEVICE" ROOTFS="$ROOTFS" \
		"$(readlink -f "$0")"
fi

if [ "$ROOTFS" = "/" ]; then
	log "Installing the compositor on the running system"
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "$COMPOSITOR_STAGE"/*.deb
else
	log "Installing the compositor into $ROOTFS"
	STAGE_IN_CHROOT="/var/tmp/compositor"
	rm -rf "$ROOTFS$STAGE_IN_CHROOT"
	mkdir -p "$ROOTFS$STAGE_IN_CHROOT"
	cp "$COMPOSITOR_STAGE"/*.deb "$ROOTFS$STAGE_IN_CHROOT/"

	mount --bind /dev "$ROOTFS/dev"
	mount -t proc proc "$ROOTFS/proc"
	mount -t sysfs sys "$ROOTFS/sys"
	cleanup() {
		rm -rf "$ROOTFS$STAGE_IN_CHROOT"
		for m in dev proc sys; do
			umount -lf "$ROOTFS/$m" 2>/dev/null || true
		done
	}
	trap cleanup EXIT

	# The glob is expanded inside the chroot, where the .debs were copied to.
	chroot "$ROOTFS" /bin/bash -c \
		"DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $STAGE_IN_CHROOT/*.deb"
fi

# Verify with the binary the session will actually run, from inside the target:
# plain sway prints "sway version 1.12.0", the fork prints
# "swayfx version 0.6 (based on sway 1.12.0)".
_COMPOSITOR_VERSION=""
if [ "$ROOTFS" = "/" ]; then
	_COMPOSITOR_VERSION="$(sway --version 2>/dev/null | head -n1)"
else
	_COMPOSITOR_VERSION="$(chroot "$ROOTFS" sway --version 2>/dev/null | head -n1)"
fi
case "$_COMPOSITOR_VERSION" in
*"$COMPOSITOR"*) log "Installed: $_COMPOSITOR_VERSION" ;;
*) die "the installed compositor is not $COMPOSITOR ('${_COMPOSITOR_VERSION:-sway --version failed}')" ;;
esac

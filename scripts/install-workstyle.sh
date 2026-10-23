#!/bin/bash
# Install the workstyle package built by build-workstyle.sh.
#
# Usage:
#   install-workstyle.sh [ROOTFS]
#
# Without an argument the rootfs built by build-rootfs.sh is used; `install-workstyle.sh /`
# installs it on the running system (the counterpart of install-kpart.sh for a
# kernel update).
#
# Unlike install-swayfx.sh there is nothing for apt to resolve: the package
# depends on the C library and the compiler runtime every rootfs already has, so
# `dpkg --root` is enough - no chroot is mounted and no apt cache is touched. A
# dependency that is really missing fails loudly instead of being installed.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

[ -n "${1:-}" ] && ROOTFS="$1"
# Resolve before re-execing: sudo may not keep the caller's working directory,
# and a relative path would then point somewhere else.
ROOTFS="$(readlink -f "$ROOTFS" 2>/dev/null || printf '%s' "$ROOTFS")"

[ -f "$ROOTFS/var/lib/dpkg/status" ] ||
	die "not a root filesystem: $ROOTFS/var/lib/dpkg/status is missing"
ls "$WORKSTYLE_STAGE"/*.deb >/dev/null 2>&1 ||
	die "no workstyle package in $WORKSTYLE_STAGE (run build-workstyle.sh first)"

if [ "$(id -u)" -ne 0 ]; then
	# sudo does not preserve the environment, so a BUILD_ROOT/WORK_DIR/DEVICE
	# given to this script would be replaced by its default and the package would
	# be looked for in the wrong place.
	exec sudo -n \
		BUILD_ROOT="$BUILD_ROOT" WORK_DIR="$WORK_DIR" SRC_DIR="$SRC_DIR" \
		DEVICE="$DEVICE" ROOTFS="$ROOTFS" \
		"$(readlink -f "$0")"
fi

if [ "$ROOTFS" = "/" ]; then
	log "Installing workstyle on the running system"
	dpkg -i "$WORKSTYLE_STAGE"/*.deb
else
	log "Installing workstyle into $ROOTFS"
	dpkg --root="$ROOTFS" -i "$WORKSTYLE_STAGE"/*.deb
fi
log "workstyle installed; the session starts it from uis/sway/files/etc/sway/config.d/02-workstyle.conf"

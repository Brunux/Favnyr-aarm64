#!/bin/bash
# Build a Debian (forky) arm64 rootfs for the Hayato with debootstrap.
#
# Requires root (re-execs itself through sudo).
set -euo pipefail
. "$(dirname "$0")/lib.sh"

ROOTFS="${ROOTFS:-$WORK_DIR/rootfs}"
UI="${UI:-sway}"
TARGET_HOSTNAME="${TARGET_HOSTNAME:-hayato}"
USERNAME="${USERNAME:-brunux}"
USERPASS="${USERPASS:-debian}"

if [ "$(id -u)" -ne 0 ]; then
	export BUILD_ROOT WORK_DIR SRC_DIR REPO_DIR
	exec sudo -n -E "$(readlink -f "$0")" "$@"
fi

cleanup_mounts() {
	for m in dev/pts dev proc sys; do
		mountpoint -q "$ROOTFS/$m" && umount -lf "$ROOTFS/$m" || true
	done
}
trap cleanup_mounts EXIT

log "debootstrap $DEBIAN_SUITE/$DEBIAN_ARCH -> $ROOTFS"
rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"
debootstrap --arch="$DEBIAN_ARCH" --variant=minbase \
	--components="main,contrib,non-free-firmware" \
	"$DEBIAN_SUITE" "$ROOTFS" "$DEBIAN_MIRROR"

log "Mounting virtual filesystems"
mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sys "$ROOTFS/sys"
cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"

install -Dm755 "$REPO_DIR/scripts/rootfs-setup.sh" "$ROOTFS/root/rootfs-setup.sh"
rm -rf "$ROOTFS/tmp/packages"
cp -r "$REPO_DIR/packages" "$ROOTFS/tmp/packages"

log "Running rootfs-setup.sh in chroot"
chroot "$ROOTFS" env \
	HOSTNAME="$TARGET_HOSTNAME" \
	USERNAME="$USERNAME" \
	USERPASS="$USERPASS" \
	DEBIAN_SUITE="$DEBIAN_SUITE" \
	DEBIAN_MIRROR="$DEBIAN_MIRROR" \
	UI="$UI" \
	/bin/bash /root/rootfs-setup.sh

rm -rf "$ROOTFS/tmp/packages" "$ROOTFS/root/rootfs-setup.sh"
log "Rootfs ready: $ROOTFS"

#!/bin/bash
# Build a Debian rootfs for the selected device using debootstrap.
#
# Device: env DEVICE (default: the only one under devices/).
# Requires root (re-execs itself through sudo).
set -euo pipefail
. "$(dirname "$0")/lib.sh"

UI="${UI:-sway}"
TARGET_HOSTNAME="${TARGET_HOSTNAME:-$(echo "$DEVICE" | sed 's/.*-//')}"
USERNAME="${USERNAME:-brunux}"
USERPASS="${USERPASS:-debian}"

if [ "$(id -u)" -ne 0 ]; then
	exec sudo -n DEVICE="$DEVICE" "$(readlink -f "$0")" "$@"
fi

cleanup_mounts() {
	for m in dev proc sys; do
		umount -lf "$ROOTFS/$m" 2>/dev/null || true
	done
}
trap cleanup_mounts EXIT

log "Device : $DEVICE ($DEVICE_ARCH)"
log "Kernel : $KERNEL_FLAVOR $KERNEL_RELEASE"
log "Rootfs : $ROOTFS"

log "debootstrap $DEBIAN_SUITE/$DEBIAN_ARCH"
rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"
debootstrap --arch="$DEBIAN_ARCH" --variant=minbase \
	--components="$(echo $DEBIAN_COMPONENTS | tr ' ' ',')" \
	"$DEBIAN_SUITE" "$ROOTFS" "$DEBIAN_MIRROR"

log "Mounting virtual filesystems"
mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sys "$ROOTFS/sys"
cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"

install -Dm755 "$REPO_DIR/scripts/rootfs-setup.sh" "$ROOTFS/root/rootfs-setup.sh"
rm -rf "$ROOTFS/tmp/packages"
mkdir -p "$ROOTFS/tmp/packages"
cp "$PACKAGES_DIR"/*.list "$ROOTFS/tmp/packages/"
[ -f "$DEVICE_DIR/packages.list" ] && cp "$DEVICE_DIR/packages.list" "$ROOTFS/tmp/packages/device.list"

log "Running rootfs-setup.sh in chroot"
chroot "$ROOTFS" env \
	HOSTNAME="$TARGET_HOSTNAME" \
	USERNAME="$USERNAME" \
	USERPASS="$USERPASS" \
	DEBIAN_SUITE="$DEBIAN_SUITE" \
	DEBIAN_MIRROR="$DEBIAN_MIRROR" \
	DEBIAN_COMPONENTS="$DEBIAN_COMPONENTS" \
	UI="$UI" \
	/bin/bash /root/rootfs-setup.sh

rm -rf "$ROOTFS/tmp/packages" "$ROOTFS/root/rootfs-setup.sh"
log "Rootfs ready: $ROOTFS"

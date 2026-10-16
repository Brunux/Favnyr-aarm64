#!/bin/bash
# Install the staged kernel (modules, vmlinuz, dtbs) into a rootfs and build its initramfs.
#
# Usage: install-kernel.sh [ROOTFS]
set -euo pipefail
. "$(dirname "$0")/lib.sh"

ROOTFS="${1:-${ROOTFS:-$WORK_DIR/rootfs}}"
STAGE="${KERNEL_STAGE:-$WORK_DIR/kernel-stage}"
KREL="$KERNEL_RELEASE"

if [ "$(id -u)" -ne 0 ]; then
	export BUILD_ROOT WORK_DIR SRC_DIR REPO_DIR
	exec sudo -n -E "$(readlink -f "$0")" "$@"
fi

[ -d "$ROOTFS" ] || die "rootfs not found: $ROOTFS (run build-rootfs.sh first)"
[ -d "$STAGE/lib/modules/$KREL" ] || die "staged kernel not found: $STAGE (run build-kernel.sh first)"

log "Installing modules for $KREL"
mkdir -p "$ROOTFS/lib/modules"
rm -rf "$ROOTFS/lib/modules/$KREL"
cp -a "$STAGE/lib/modules/$KREL" "$ROOTFS/lib/modules/"

log "Installing vmlinuz / config / dtbs"
mkdir -p "$ROOTFS/boot/dtbs"
cp "$STAGE/boot/vmlinuz" "$ROOTFS/boot/vmlinuz-$KREL"
[ -f "$STAGE/boot/config" ] && cp "$STAGE/boot/config" "$ROOTFS/boot/config-$KREL"
[ -f "$STAGE/boot/System.map" ] && cp "$STAGE/boot/System.map" "$ROOTFS/boot/System.map-$KREL"
cp -a "$STAGE/boot/dtbs/." "$ROOTFS/boot/dtbs/"

log "depmod + initramfs"
mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sys "$ROOTFS/sys"
cleanup() {
	for m in dev proc sys; do
		mountpoint -q "$ROOTFS/$m" && umount -lf "$ROOTFS/$m" || true
	done
}
trap cleanup EXIT
chroot "$ROOTFS" depmod -a "$KREL"
chroot "$ROOTFS" update-initramfs -c -k "$KREL"

log "Kernel installed into $ROOTFS"
ls -la "$ROOTFS/boot"

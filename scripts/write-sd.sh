#!/bin/bash
# Partition and populate a microSD/USB device so it boots via depthcharge (Ctrl+U).
#
# Layout (matches Nura's proven layout):
#   p1  pmOS_kernel  128 MiB  ChromeOS kernel partition (signed FIT kpart)
#   p2  pmOS_root    rest     ext4 root filesystem
#
# Usage: write-sd.sh [DEVICE]     (default: /dev/mmcblk1)
set -euo pipefail
. "$(dirname "$0")/lib.sh"

DEVICE="${1:-/dev/mmcblk1}"
ROOTFS="${ROOTFS:-$WORK_DIR/rootfs}"
KPART="${KPART:-$WORK_DIR/vmlinuz.kpart}"
SIZE_SECTORS="$KPART_SIZE_SECTORS"
START_SECTORS="$KPART_START_SECTORS"

if [ "$(id -u)" -ne 0 ]; then
	export BUILD_ROOT WORK_DIR SRC_DIR REPO_DIR
	exec sudo -n -E "$(readlink -f "$0")" "$@"
fi

[ -b "$DEVICE" ] || die "not a block device: $DEVICE"
[ -d "$ROOTFS" ] || die "rootfs not found: $ROOTFS (run build-rootfs.sh first)"

# Safety: never touch the internal eMMC or the running root device.
ROOT_SRC="$(findmnt -n -o SOURCE / 2>/dev/null || true)"
case "$DEVICE" in
	*/mmcblk0|*/mmcblk0p*) die "refusing to write to internal eMMC: $DEVICE" ;;
esac
case "$ROOT_SRC" in
	"$DEVICE"|"$DEVICE"p*) die "refusing to write to the running root device: $DEVICE" ;;
esac

log "Target device: $DEVICE ($(blockdev --getsize64 "$DEVICE" | numfmt --to=iec 2>/dev/null || blockdev --getsize64 "$DEVICE") bytes)"

# Release the device: swap + mounts.
for part in "$DEVICE" "$DEVICE"p*; do
	[ -b "$part" ] || continue
	swapoff "$part" 2>/dev/null || true
	mountpoint -q "$part" 2>/dev/null && umount "$part" || true
done
grep -q "^$DEVICE" /proc/swaps && swapoff -a || true

log "Creating ChromeOS GPT on $DEVICE"
wipefs -a "$DEVICE" >/dev/null 2>&1 || true
cgpt create -z "$DEVICE"
cgpt add -i 1 -b "$START_SECTORS" -s "$SIZE_SECTORS" -t kernel -l pmOS_kernel \
	-S 1 -T 5 -P 10 "$DEVICE"

TOTAL="$(blockdev --getsz "$DEVICE")"
P2_START=$((START_SECTORS + SIZE_SECTORS))
P2_SIZE=$((TOTAL - P2_START - 2048))
[ "$P2_SIZE" -gt 1048576 ] || die "device too small for a root filesystem: $DEVICE"
cgpt add -i 2 -b "$P2_START" -s "$P2_SIZE" -t data -l pmOS_root "$DEVICE"

partprobe "$DEVICE" 2>/dev/null || true
command -v udevadm >/dev/null && udevadm settle || sleep 2

ROOT_PART="$(lsblk -nro PATH "$DEVICE" | sed -n '2p')"
[ -n "$ROOT_PART" ] || ROOT_PART="${DEVICE}p2"
log "Root partition: $ROOT_PART"

log "Formatting root filesystem (ext4)"
mkfs.ext4 -F -L pmOS_root -U "$(uuidgen 2>/dev/null || cat /proc/sys/kernel/random/uuid)" "$ROOT_PART" >/dev/null
ROOT_UUID="$(blkid -s UUID -o value "$ROOT_PART")"
[ -n "$ROOT_UUID" ] || die "could not read root filesystem UUID"
log "Root UUID: $ROOT_UUID"

log "Copying root filesystem"
MNT="$WORK_DIR/sd-mnt"
mkdir -p "$MNT"
mount "$ROOT_PART" "$MNT"
rsync -aHAX --delete "$ROOTFS"/ "$MNT"/

cat >"$MNT/etc/fstab" <<EOF
UUID=$ROOT_UUID	/	ext4	defaults,noatime,errors=remount-ro	0 1
EOF

log "Building kpart with root=UUID=$ROOT_UUID"
"$REPO_DIR/scripts/make-kpart.sh" "$MNT" "$KPART" "root=UUID=$ROOT_UUID rootwait rw"

log "Writing kpart to kernel partition ($(lsblk -nro PATH "$DEVICE" | sed -n '1p'))"
KPART_PART="$(lsblk -nro PATH "$DEVICE" | sed -n '1p')"
dd if="$KPART" of="$KPART_PART" bs=1M conv=fsync
sync

umount "$MNT"
log "Done. Remove $DEVICE and boot with Ctrl+U."
cgpt show "$DEVICE" || true

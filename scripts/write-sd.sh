#!/bin/bash
# Partition and populate a microSD/USB device so it boots via depthcharge (Ctrl+U).
#
# Layout: p1 = <start>,<size> ChromeOS kernel partition (signed FIT kpart)
#         p2 = rest                ext4 root filesystem
# (sizes come from deviceinfo_cgpt_kpart_start / _size)
#
# Usage: write-sd.sh [DEVICE]     (default: /dev/mmcblk1)
set -euo pipefail
. "$(dirname "$0")/lib.sh"

BLOCK="${1:-/dev/mmcblk1}"

if [ "$(id -u)" -ne 0 ]; then
	exec sudo -n DEVICE="$DEVICE" "$(readlink -f "$0")" "${1:-}"
fi

[ -b "$BLOCK" ] || die "not a block device: $BLOCK"
[ -d "$ROOTFS" ] || die "rootfs not found: $ROOTFS (run build-rootfs.sh first)"

# Safety: never touch the internal eMMC or the running root device.
ROOT_SRC="$(findmnt -n -o SOURCE / 2>/dev/null || true)"
case "$BLOCK" in
	*/mmcblk0|*/mmcblk0p*) die "refusing to write to internal eMMC: $BLOCK" ;;
esac
case "$ROOT_SRC" in
	"$BLOCK"|"$BLOCK"p*) die "refusing to write to the running root device: $BLOCK" ;;
esac

log "Device: $DEVICE -> $BLOCK ($(blockdev --getsize64 "$BLOCK") bytes)"

# Release the device: swap + mounts.
for part in "$BLOCK" "$BLOCK"p*; do
	[ -b "$part" ] || continue
	swapoff "$part" 2>/dev/null || true
	mountpoint -q "$part" 2>/dev/null && umount "$part" || true
done

log "Creating ChromeOS GPT on $BLOCK"
wipefs -a "$BLOCK" >/dev/null 2>&1 || true
cgpt create -z "$BLOCK"
cgpt add -i 1 -b "$KPART_START_SECTORS" -s "$KPART_SIZE_SECTORS" -t kernel -l pmOS_kernel \
	-S 1 -T 5 -P 10 "$BLOCK"

TOTAL="$(blockdev --getsz "$BLOCK")"
P2_START=$((KPART_START_SECTORS + KPART_SIZE_SECTORS))
P2_SIZE=$((TOTAL - P2_START - 2048))
[ "$P2_SIZE" -gt 1048576 ] || die "device too small for a root filesystem: $BLOCK"
cgpt add -i 2 -b "$P2_START" -s "$P2_SIZE" -t data -l pmOS_root "$BLOCK"

partprobe "$BLOCK" 2>/dev/null || true
command -v udevadm >/dev/null && udevadm settle || sleep 2

KPART_PART="$(lsblk -nro PATH "$BLOCK" | sed -n '1p')"
ROOT_PART="$(lsblk -nro PATH "$BLOCK" | sed -n '2p')"
[ -n "$KPART_PART" ] && [ -n "$ROOT_PART" ] || die "could not resolve partitions on $BLOCK"

log "Formatting $ROOT_PART (ext4)"
mkfs.ext4 -F -L pmOS_root "$ROOT_PART" >/dev/null
ROOT_UUID="$(blkid -s UUID -o value "$ROOT_PART")"
[ -n "$ROOT_UUID" ] || die "could not read root filesystem UUID"
log "Root UUID: $ROOT_UUID"

log "Copying root filesystem"
MNT="$WORK_DIR/sd-mnt-$DEVICE"
mkdir -p "$MNT"
mount "$ROOT_PART" "$MNT"
rsync -aHAX --delete "$ROOTFS"/ "$MNT"/
cat >"$MNT/etc/fstab" <<EOF
UUID=$ROOT_UUID	/	ext4	defaults,noatime,errors=remount-ro	0 1
EOF

log "Building kpart (root=UUID=$ROOT_UUID)"
"$REPO_DIR/scripts/make-kpart.sh" "$MNT" "$KPART" "root=UUID=$ROOT_UUID rootwait rw"

log "Writing kpart to $KPART_PART"
dd if="$KPART" of="$KPART_PART" bs=1M conv=fsync
sync
umount "$MNT"

log "Done. Remove $BLOCK and boot with Ctrl+U."
cgpt show "$BLOCK" || true

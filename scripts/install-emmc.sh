#!/bin/bash
# Install the currently running system onto the internal eMMC (default /dev/mmcblk0).
#
# WARNING: DESTROYS the existing eMMC contents. Run it from the microSD-booted
# Debian system with CONFIRM=yes.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

BLOCK="${DEVICE_BLOCK:-/dev/mmcblk0}"

if [ "$(id -u)" -ne 0 ]; then
	exec sudo -n DEVICE="$DEVICE" DEVICE_BLOCK="$BLOCK" "$(readlink -f "$0")" "$@"
fi

[ -b "$BLOCK" ] || die "not a block device: $BLOCK"
ROOT_SRC="$(findmnt -n -o SOURCE /)"
case "$ROOT_SRC" in
	"$BLOCK"|"$BLOCK"p*) die "already running from $BLOCK; boot from microSD first" ;;
esac

log "About to ERASE and install $DEVICE on $BLOCK"
log "  running root: $ROOT_SRC"
lsblk "$BLOCK"
[ "${CONFIRM:-}" = "yes" ] || die "refusing to continue without CONFIRM=yes"

log "Creating ChromeOS GPT on $BLOCK"
for part in "$BLOCK" "$BLOCK"p*; do
	[ -b "$part" ] || continue
	swapoff "$part" 2>/dev/null || true
	mountpoint -q "$part" 2>/dev/null && umount "$part" || true
	wipefs -a "$part" >/dev/null 2>&1 || true
done
# See write-sd.sh for the cgpt pitfalls: no '-z', and parted must create the
# protective MBR or the kernel will not parse the GPT.
TOTAL="$(blockdev --getsz "$BLOCK")"
dd if=/dev/zero of="$BLOCK" bs=512 count=34 conv=fsync 2>/dev/null
dd if=/dev/zero of="$BLOCK" bs=512 seek=$((TOTAL - 33)) count=33 conv=fsync 2>/dev/null
parted -s "$BLOCK" mktable gpt
cgpt create "$BLOCK"
cgpt add -i 1 -b "$KPART_START_SECTORS" -s "$KPART_SIZE_SECTORS" -t kernel -l pmOS_kernel \
	-S 1 -T 5 -P 10 "$BLOCK"
P2_START=$((KPART_START_SECTORS + KPART_SIZE_SECTORS))
P2_SIZE=$((TOTAL - P2_START - 2048))
cgpt add -i 2 -b "$P2_START" -s "$P2_SIZE" -t data -l pmOS_root "$BLOCK"
partx -d "$BLOCK" 2>/dev/null || true
partx -a "$BLOCK" 2>/dev/null || blockdev --rereadpt "$BLOCK" 2>/dev/null || true
command -v udevadm >/dev/null && udevadm settle || sleep 2

# lsblk lists the whole device first, then its partitions: skip the disk line.
KPART_PART="$(lsblk -nro PATH "$BLOCK" | sed -n '2p')"
ROOT_PART="$(lsblk -nro PATH "$BLOCK" | sed -n '3p')"
[ -n "$KPART_PART" ] && [ -n "$ROOT_PART" ] || die "could not resolve partitions on $BLOCK"
[ "$KPART_PART" != "$BLOCK" ] && [ "$ROOT_PART" != "$BLOCK" ] ||
	die "refusing to operate on the whole disk; partition resolution failed for $BLOCK"

log "Formatting $ROOT_PART (ext4)"
mkfs.ext4 -F -L pmOS_root "$ROOT_PART" >/dev/null
ROOT_UUID="$(blkid -s UUID -o value "$ROOT_PART")"

log "Copying the running system to $ROOT_PART"
MNT="$WORK_DIR/emmc-mnt-$DEVICE"
mkdir -p "$MNT"
mount "$ROOT_PART" "$MNT"
rsync -aHAX --delete \
	--exclude=/proc --exclude=/sys --exclude=/dev --exclude=/run \
	--exclude=/mnt --exclude="$MNT" --exclude=/media \
	/ "$MNT"/
cat >"$MNT/etc/fstab" <<EOF
UUID=$ROOT_UUID	/	ext4	defaults,noatime,errors=remount-ro	0 1
EOF

log "Building kpart for the eMMC root"
"$REPO_DIR/scripts/make-kpart.sh" "$MNT" "$KPART.emmc" "root=UUID=$ROOT_UUID rootwait rw"
dd if="$KPART.emmc" of="$KPART_PART" bs=1M conv=fsync
sync
umount "$MNT"

log "Done. Reboot and remove the microSD to boot from eMMC."
cgpt show "$BLOCK"

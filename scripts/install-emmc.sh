#!/bin/bash
# Install the currently running system onto the internal eMMC (mmcblk0).
#
# WARNING: this DESTROYS the existing eMMC contents (including a Nura/pmOS
# installation). It is meant to be run from the microSD-booted Debian system,
# with the source tree and build scripts present.
#
# Run with CONFIRM=yes to proceed.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

DEVICE="${DEVICE:-/dev/mmcblk0}"
SIZE_SECTORS="$KPART_SIZE_SECTORS"
START_SECTORS="$KPART_START_SECTORS"

if [ "$(id -u)" -ne 0 ]; then
	export BUILD_ROOT WORK_DIR SRC_DIR REPO_DIR
	exec sudo -n -E "$(readlink -f "$0")" "$@"
fi

[ -b "$DEVICE" ] || die "not a block device: $DEVICE"

ROOT_SRC="$(findmnt -n -o SOURCE /)"
case "$ROOT_SRC" in
	"$DEVICE"|"$DEVICE"p*) die "already running from $DEVICE; boot from microSD first" ;;
esac

log "About to ERASE and install Debian on $DEVICE"
log "  running root: $ROOT_SRC"
lsblk "$DEVICE"
[ "${CONFIRM:-}" = "yes" ] || die "refusing to continue without CONFIRM=yes"

log "Creating ChromeOS GPT on $DEVICE"
for part in "$DEVICE" "$DEVICE"p*; do
	[ -b "$part" ] || continue
	swapoff "$part" 2>/dev/null || true
	mountpoint -q "$part" 2>/dev/null && umount "$part" || true
done
wipefs -a "$DEVICE" >/dev/null 2>&1 || true
cgpt create -z "$DEVICE"
cgpt add -i 1 -b "$START_SECTORS" -s "$SIZE_SECTORS" -t kernel -l pmOS_kernel \
	-S 1 -T 5 -P 10 "$DEVICE"
TOTAL="$(blockdev --getsz "$DEVICE")"
P2_START=$((START_SECTORS + SIZE_SECTORS))
P2_SIZE=$((TOTAL - P2_START - 2048))
cgpt add -i 2 -b "$P2_START" -s "$P2_SIZE" -t data -l pmOS_root "$DEVICE"
partprobe "$DEVICE" 2>/dev/null || true
command -v udevadm >/dev/null && udevadm settle || sleep 2

KPART_PART="$(lsblk -nro PATH "$DEVICE" | sed -n '1p')"
ROOT_PART="$(lsblk -nro PATH "$DEVICE" | sed -n '2p')"

log "Formatting $ROOT_PART (ext4)"
mkfs.ext4 -F -L pmOS_root "$ROOT_PART" >/dev/null
ROOT_UUID="$(blkid -s UUID -o value "$ROOT_PART")"

log "Copying the running system to $ROOT_PART"
MNT="$WORK_DIR/emmc-mnt"
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
"$REPO_DIR/scripts/make-kpart.sh" "$MNT" "$WORK_DIR/vmlinuz.kpart.emmc" "root=UUID=$ROOT_UUID rootwait rw"
dd if="$WORK_DIR/vmlinuz.kpart.emmc" of="$KPART_PART" bs=1M conv=fsync
sync
umount "$MNT"

log "Done. Reboot and remove the microSD to boot from eMMC."
cgpt show "$DEVICE"

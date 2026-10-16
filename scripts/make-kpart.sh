#!/bin/bash
# Build the signed depthcharge FIT kernel partition image (kpart) for the device.
#
# Mirrors pmOS boot-deploy for ChromeOS devices:
#   depthchargectl build --root none --board <board> --kernel ... \
#       --kernel-cmdline ... --initramfs ... --fdtdir ... --compress ... -o ...
#
# Usage: make-kpart.sh [ROOTFS] [OUTPUT] [CMDLINE_EXTRA]
set -euo pipefail
. "$(dirname "$0")/lib.sh"

[ -n "${1:-}" ] && ROOTFS="$1"
OUT="${2:-$KPART}"
CMDLINE_EXTRA="${3:-}"

KERNEL_IMG="$ROOTFS/boot/vmlinuz-$KERNEL_RELEASE"
INITRD="$ROOTFS/boot/initrd.img-$KERNEL_RELEASE"
[ -f "$KERNEL_IMG" ] || die "kernel not found: $KERNEL_IMG"
[ -f "$INITRD" ] || die "initramfs not found: $INITRD"
[ -n "$DEPTHCHARGE_BOARD" ] || die "deviceinfo_depthcharge_board not set for $DEVICE"

BUILDDIR="$WORK_DIR/kpart-dtbs-$DEVICE"
rm -rf "$BUILDDIR"
mkdir -p "$BUILDDIR"
# Expand the deviceinfo_dtb glob (e.g. "mediatek/mt8192-asurada*").
# shellcheck disable=SC2086
set -- "$ROOTFS"/boot/dtbs/$DTB_GLOB.dtb
[ -e "$1" ] || die "no DTBs matching '$DTB_GLOB' under $ROOTFS/boot/dtbs"
cp "$@" "$BUILDDIR"/

CMDLINE="${KERNEL_CMDLINE}kern_guid=%U ${CMDLINE_EXTRA}"
log "Building kpart (device=$DEVICE board=$DEPTHCHARGE_BOARD compression=$DEPTHCHARGE_COMPRESSION)"
log "  kernel : $KERNEL_IMG"
log "  initrd : $INITRD"
log "  dtbs   : $(ls "$BUILDDIR")"
log "  cmdline: $CMDLINE"

depthchargectl build --root none \
	--board "$DEPTHCHARGE_BOARD" \
	--kernel "$KERNEL_IMG" \
	--kernel-cmdline "$CMDLINE" \
	--initramfs "$INITRD" \
	--fdtdir "$BUILDDIR" \
	--compress "$DEPTHCHARGE_COMPRESSION" \
	--timestamp "${SOURCE_DATE_EPOCH:-$(date +%s)}" \
	--tmpdir "$WORK_DIR/dct-$DEVICE" \
	--output "$OUT"

log "Built $OUT ($(du -h "$OUT" | cut -f1))"

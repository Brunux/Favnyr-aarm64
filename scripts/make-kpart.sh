#!/bin/bash
# Build the signed depthcharge FIT kernel partition image (vmlinuz.kpart).
#
# This mirrors what pmOS `boot-deploy` does for ChromeOS devices:
#   depthchargectl build --root none --board <board> --kernel ... \
#       --kernel-cmdline ... --initramfs ... --fdtdir ... --compress ... -o ...
#
# Usage: make-kpart.sh [ROOTFS] [OUTPUT] [CMDLINE_EXTRA]
set -euo pipefail
. "$(dirname "$0")/lib.sh"

ROOTFS="${1:-${ROOTFS:-$WORK_DIR/rootfs}}"
OUT="${2:-$WORK_DIR/vmlinuz.kpart}"
CMDLINE_EXTRA="${3:-}"
KREL="$KERNEL_RELEASE"
DTB_GLOB="${DTB_GLOB:-mt8192-asurada*.dtb}"

KERNEL="$ROOTFS/boot/vmlinuz-$KREL"
INITRD="$ROOTFS/boot/initrd.img-$KREL"
[ -f "$KERNEL" ] || die "kernel not found: $KERNEL"
[ -f "$INITRD" ] || die "initramfs not found: $INITRD"

BUILDDIR="$WORK_DIR/kpart-dtbs"
rm -rf "$BUILDDIR"
mkdir -p "$BUILDDIR"
find "$ROOTFS/boot/dtbs" -name "$DTB_GLOB" -exec cp -t "$BUILDDIR" {} +
[ -n "$(ls -A "$BUILDDIR")" ] || die "no DTBs matching '$DTB_GLOB' under $ROOTFS/boot/dtbs"

CMDLINE="kern_guid=%U quiet loglevel=2 ${CMDLINE_EXTRA}"
log "Building kpart (board=$DEPTHCHARGE_BOARD)"
log "  kernel : $KERNEL"
log "  initrd : $INITRD"
log "  dtbs   : $(ls "$BUILDDIR")"
log "  cmdline: $CMDLINE"

depthchargectl build --root none \
	--board "$DEPTHCHARGE_BOARD" \
	--kernel "$KERNEL" \
	--kernel-cmdline "$CMDLINE" \
	--initramfs "$INITRD" \
	--fdtdir "$BUILDDIR" \
	--compress lzma \
	--timestamp "${SOURCE_DATE_EPOCH:-$(date +%s)}" \
	--tmpdir "$WORK_DIR/dct" \
	--output "$OUT"

log "Built $OUT ($(du -h "$OUT" | cut -f1))"

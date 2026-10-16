#!/bin/bash
# Full build pipeline (does NOT touch any block device).
#
# Produces, under $WORK_DIR:
#   kernel-stage/    built kernel, modules, dtbs
#   rootfs/          Debian forky rootfs with the kernel installed
#   vmlinuz.kpart    signed depthcharge FIT kernel partition image
#
# To write the result to a microSD/USB device, run scripts/write-sd.sh.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

"$REPO_DIR/scripts/build-kernel.sh"
"$REPO_DIR/scripts/build-rootfs.sh"
"$REPO_DIR/scripts/install-kernel.sh"
"$REPO_DIR/scripts/make-kpart.sh"

log "Build complete."
log "  rootfs : $WORK_DIR/rootfs"
log "  kpart  : $WORK_DIR/vmlinuz.kpart"
log "Next: scripts/write-sd.sh /dev/mmcblk1"

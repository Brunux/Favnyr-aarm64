#!/bin/bash
# Full build pipeline for the selected device (does NOT touch any block device).
#
# Produces, under $WORK_DIR:
#   kernel-stage-<flavor>/  built kernel, modules, dtbs
#   swayfx-stage/           the compositor's .debs (scenefx + swayfx)
#   rootfs-<device>/        Debian rootfs with the kernel and compositor installed
#   <device>.vmlinuz.kpart  signed depthcharge FIT kernel partition image
#
# To write the result to a microSD/USB device, run scripts/write-sd.sh.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

"$REPO_DIR/scripts/build-kernel.sh"
# The rootfs does not install Debian's sway (uis/sway/packages.list), so the
# compositor has to be built before the image is useful.
"$REPO_DIR/scripts/build-swayfx.sh"
"$REPO_DIR/scripts/build-rootfs.sh"
"$REPO_DIR/scripts/install-kernel.sh"
"$REPO_DIR/scripts/install-swayfx.sh"
"$REPO_DIR/scripts/make-kpart.sh"

log "Build complete."
log "  device : $DEVICE"
log "  compositor : $COMPOSITOR $COMPOSITOR_VERSION (scenefx $SCENEFX_VERSION)"
log "  rootfs : $ROOTFS"
log "  kpart  : $KPART"
log "Next: scripts/write-sd.sh /dev/mmcblk1"

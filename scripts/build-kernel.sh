#!/bin/bash
# Build the MediaTek MT8192 (mt81) kernel from Nura's pinned source + config,
# and install it into a staging directory.
#
# Source: mainline Linux (cdn.kernel.org) + patches + config from
#   pmaports 0e458a24 device/community/linux-postmarketos-mediatek-mt81
set -euo pipefail
. "$(dirname "$0")/lib.sh"

KERNEL_SRC="$SRC_DIR/linux-$KERNEL_VERSION"
PATCH_DIR="$REPO_DIR/kernel/mt81/patches"
CONFIG="$REPO_DIR/kernel/mt81/config-postmarketos-mediatek-mt81.aarch64"
STAGE="${KERNEL_STAGE:-$WORK_DIR/kernel-stage}"
JOBS="${JOBS:-$(nproc)}"

# Patches must be applied in this order.
PATCHES="
mt8183-fix-bluetooth
mt8186-enable-dpi
mt8186-ASoC-hdmi-codec-Add-event-handler-for-hdmi-TX
mt8186-SoC-mediatek-mt8186-correct-the-HDMI-widgets
mt8186-drm-bridge-it6505-Add-audio-support
mt8186-ASoC-mediatek-mt8186-make-FE-nonatomic-and-no_pcm
mtk-pmdomain
"

fetch_and_prepare() {
	mkdir -p "$SRC_DIR"
	if [ ! -f "$SRC_DIR/$KERNEL_TARBALL" ]; then
		log "Downloading $KERNEL_URL"
		curl -fSL -o "$SRC_DIR/$KERNEL_TARBALL" "$KERNEL_URL"
	fi
	if [ ! -f "$KERNEL_SRC/.prepared" ]; then
		log "Extracting kernel source"
		rm -rf "$KERNEL_SRC"
		tar xf "$SRC_DIR/$KERNEL_TARBALL" -C "$SRC_DIR"
		log "Applying patches"
		for p in $PATCHES; do
			patch -d "$KERNEL_SRC" -p1 --no-backup-if-mismatch < "$PATCH_DIR/$p.patch"
		done
		cp "$CONFIG" "$KERNEL_SRC/.config"
		touch "$KERNEL_SRC/.prepared"
	fi
}

build_kernel() {
	log "Building kernel ($JOBS jobs)"
	make -C "$KERNEL_SRC" ARCH=arm64 LLVM=1 -j"$JOBS"
}

install_kernel() {
	log "Installing kernel into $STAGE"
	rm -rf "$STAGE"
	mkdir -p "$STAGE/boot"
	make -C "$KERNEL_SRC" ARCH=arm64 LLVM=1 \
		INSTALL_PATH="$STAGE/boot" \
		INSTALL_MOD_PATH="$STAGE" \
		INSTALL_MOD_STRIP=1 \
		INSTALL_DTBS_PATH="$STAGE/boot/dtbs" \
		zinstall modules_install dtbs_install
	cp "$KERNEL_SRC/.config" "$STAGE/boot/config"
	cp "$KERNEL_SRC/System.map" "$STAGE/boot/System.map" 2>/dev/null || true
	log "Kernel release: $(cat "$KERNEL_SRC/include/config/kernel.release")"
	ls -la "$STAGE/boot"
}

fetch_and_prepare
build_kernel
install_kernel
log "Kernel build done: $STAGE"

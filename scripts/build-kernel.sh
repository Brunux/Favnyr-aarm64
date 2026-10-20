#!/bin/bash
# Build the kernel for the selected device's kernel flavor and stage it.
#
# Source: mainline tarball from cdn.kernel.org + kernels/<flavor>/patches
# (applied in the order listed in kernels/<flavor>/patches/series).
set -euo pipefail
. "$(dirname "$0")/lib.sh"

STAGE="$KERNEL_STAGE"
JOBS="${JOBS:-$(nproc)}"
SERIES="$KERNEL_PATCH_DIR/series"

# What the extracted tree was built from: the patch list and every patch file.
# The marker holds this hash, so editing a patch re-extracts instead of silently
# building the previous ones (the config is not part of it - that is synced on
# every run, below).
patches_hash() {
	{
		cat "$SERIES" 2>/dev/null
		cat "$KERNEL_PATCH_DIR"/*.patch 2>/dev/null
	} | sha256sum | cut -d' ' -f1
}

fetch_and_prepare() {
	mkdir -p "$SRC_DIR"
	if [ ! -f "$SRC_DIR/$KERNEL_TARBALL" ]; then
		log "Downloading $KERNEL_URL"
		curl -fSL -o "$SRC_DIR/$KERNEL_TARBALL" "$KERNEL_URL"
	fi
	_want="$(patches_hash)"
	if [ -f "$KERNEL_SRC/.prepared" ] && [ "$(cat "$KERNEL_SRC/.prepared")" = "$_want" ]; then
		# Already extracted and patched; the config is still synced (below),
		# because kbuild only picks it up from .config.
		log "Kernel source already prepared (same patches)"
	else
		log "Extracting $KERNEL_TARBALL"
		rm -rf "$KERNEL_SRC"
		tar xf "$SRC_DIR/$KERNEL_TARBALL" -C "$SRC_DIR"

		log "Applying patches"
		if [ -f "$SERIES" ]; then
			while read -r p; do
				case "$p" in '' | '#'*) continue ;; esac
				log "  $p"
				patch -d "$KERNEL_SRC" -p1 --no-backup-if-mismatch <"$KERNEL_PATCH_DIR/$p"
			done <"$SERIES"
		fi
		printf '%s\n' "$_want" >"$KERNEL_SRC/.prepared"
	fi
	# Always: kernels/<flavor>/config.aarch64 is the source of truth, and a
	# build that silently ignored an edited config is the same class of bug as a
	# working tree that never reached the image.
	if ! cmp -s "$KERNEL_CONFIG" "$KERNEL_SRC/.config"; then
		log "Syncing kernel config (kernels/$KERNEL_FLAVOR/config.aarch64)"
	fi
	cp "$KERNEL_CONFIG" "$KERNEL_SRC/.config"
}

build_kernel() {
	log "Building kernel $KERNEL_RELEASE ($JOBS jobs)"
	# olddefconfig first, non-interactively. kbuild runs it on its own when the
	# tree grew a symbol, but then it asks its question on the build's stdin and
	# the answer is whatever the stream happens to contain: a config with a
	# duplicated `# CONFIG_HZ_1000 is not set` once produced a full rebuild at
	# HZ=100 instead of 1000, silently.
	make -C "$KERNEL_SRC" ARCH="$KERNEL_ARCH" LLVM=1 olddefconfig </dev/null >/dev/null
	# The shipped file is the source of truth, so anything that ends up different
	# is drift worth seeing (compiler-probe symbols embed tool versions and change
	# on every build, hence the exclusions).
	_drift=""
	while IFS= read -r _line; do
		case "$_line" in
			CONFIG_CC_*|CONFIG_AS_*|CONFIG_LD_*|CONFIG_BINDGEN_*|CONFIG_RUSTC_*|CONFIG_CLANG_*|CONFIG_PAHOLE_*) continue ;;
		esac
		grep -qxF "$_line" "$KERNEL_CONFIG" || _drift="$_drift ${_line%%=*}"
	done <"$KERNEL_SRC/.config"
	[ -z "$_drift" ] || warn "config drift (built value differs from $KERNEL_CONFIG):$_drift"
	make -C "$KERNEL_SRC" ARCH="$KERNEL_ARCH" LLVM=1 -j"$JOBS"
}

install_kernel() {
	log "Installing kernel into $STAGE"
	rm -rf "$STAGE"
	mkdir -p "$STAGE/boot"
	make -C "$KERNEL_SRC" ARCH="$KERNEL_ARCH" LLVM=1 \
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

log "Device : $DEVICE"
log "Kernel : $KERNEL_FLAVOR $KERNEL_VERSION ($KERNEL_ARCH)"

fetch_and_prepare
build_kernel
install_kernel
log "Kernel build done: $STAGE"

#!/bin/bash
# Install host build dependencies (Alpine/apk).
#
# Run as a normal user (it re-execs itself through sudo) or as root.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

if [ "$(id -u)" -ne 0 ]; then
	exec sudo -n "$(readlink -f "$0")" "$@"
fi

log "Installing host build dependencies"
apk add --no-cache \
	bash bc bison flex \
	clang clang-libclang lld llvm \
	rust rust-bindgen \
	openssl-dev elfutils-dev pahole \
	perl python3 \
	rsync cpio \
	xz zstd lz4 gzip \
	git curl wget \
	dtc kmod \
	make gcc musl-dev binutils pkgconf \
	debootstrap \
	cgpt vboot-utils depthcharge-tools \
	e2fsprogs dosfstools parted gptfdisk \
	util-linux

log "Done. Tool versions:"
clang --version | head -1
ld.lld --version
rustc --version 2>/dev/null || true
bindgen --version 2>/dev/null || true
debootstrap --version 2>/dev/null || true

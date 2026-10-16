# shellcheck shell=bash
# Shared configuration for the debian-aarch64 build.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Resolve the *invoking* user's home even when running under sudo, so large artifacts
# stay on the real filesystem and are not written to /root.
if [ -n "${SUDO_USER:-}" ]; then
	_invoking_home="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
else
	_invoking_home="$HOME"
fi
# /tmp is a small tmpfs (~1.9 GiB) on the target device; keep large artifacts on the
# real filesystem instead.
BUILD_ROOT="${BUILD_ROOT:-${_invoking_home:-/root}/.cache/debian-aarch64}"
WORK_DIR="${WORK_DIR:-$BUILD_ROOT/work}"
SRC_DIR="${SRC_DIR:-$BUILD_ROOT/src}"
mkdir -p "$WORK_DIR" "$SRC_DIR"

# Device / kernel identity (pinned to pmaports 0e458a24)
DEVICE="google-asurada"
KERNEL_FLAVOR="mt81"
KERNEL_VERSION="7.2.7"
KERNEL_LOCALVERSION="-mt81"
KERNEL_RELEASE="${KERNEL_VERSION}${KERNEL_LOCALVERSION}"   # 7.2.7-mt81
KERNEL_TARBALL="linux-${KERNEL_VERSION}.tar.xz"
KERNEL_URL="https://cdn.kernel.org/pub/linux/kernel/v${KERNEL_VERSION%%.*}.x/${KERNEL_TARBALL}"

# Debian target
DEBIAN_SUITE="${DEBIAN_SUITE:-forky}"
DEBIAN_ARCH="arm64"
DEBIAN_MIRROR="${DEBIAN_MIRROR:-http://deb.debian.org/debian}"
DEBIAN_SECURITY="${DEBIAN_SECURITY:-http://deb.debian.org/debian-security}"
DEBIAN_COMPONENTS="main contrib non-free-firmware"

# depthcharge / ChromeOS
DEPTHCHARGE_BOARD="mt8192"
KPART_SIZE_SECTORS="262144"   # 128 MiB
KPART_START_SECTORS="8192"

log()  { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
warn() { printf '\033[1;33mWARN: %s\033[0m\n' "$*" >&2; }
die()  { printf '\033[1;31mERROR: %s\033[0m\n' "$*" >&2; exit 1; }

need_root() {
	[ "$(id -u)" -eq 0 ] || die "this command must run as root (use sudo)"
}

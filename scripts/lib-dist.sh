#!/bin/bash
# What a *published* artifact must not carry, shared by the scripts that produce
# one (make-image.sh, make-release.sh).
#
# Sourced after lib.sh, which provides die()/log()/warn() and $USERNAME.
#
# The build rootfs keeps state that belongs to the maintainer's machine on
# purpose: build-rootfs.sh imports the Wi-Fi profiles from $WIFI_DIR so the
# maintainer's own cards come up already knowing the networks. That state must
# never leave the machine it was built on, so a released artifact is copied,
# sanitized, and then *asserted* clean - the assertion is what makes it
# impossible to publish a profile by accident, which is the failure this file
# exists for:
#
#   * /etc/NetworkManager/system-connections/*.nmconnection holds the Wi-Fi PSK
#     in clear. Shipping one publishes the maintainer's Wi-Fi password.
#   * /var/lib/NetworkManager is the state NetworkManager built from those
#     profiles (including the BSSIDs it has seen).
#   * /etc/ssh/ssh_host_* makes every installation of the image share one host
#     identity, so it stops identifying a machine at all.
#   * /etc/machine-id and /var/lib/dbus/machine-id are the same problem for
#     systemd, DHCP leases and journal files.
#   * the build root itself ($HOME/.cache/debian-aarch64) contains the exported
#     Wi-Fi profiles, so a copy of it inside an artifact leaks them too.
#
# Everything else removed here is a build leftover. Nothing personal is rescued:
# sanitize is destructive by design, which is why it runs on a copy (an image
# mount or a staging tree) and never on $ROOTFS.
#
# /etc/machine-id is emptied rather than deleted: systemd-machine-id-setup
# generates a fresh one on the first boot when the file is empty. The SSH host
# keys are simply deleted because the image ships sshd-keygen.service (wanted by
# ssh.service), which generates the ones it is missing.

# Remove the state above from a rootfs tree. <dir> must be a copy.
dist_sanitize() {
	_d="${1%/}"
	[ -d "$_d/etc" ] || die "dist_sanitize: $_d does not look like a root filesystem"

	log "Sanitizing $_d for publication"
	rm -rf "$_d/etc/NetworkManager/system-connections"
	rm -rf "${_d:?}"/var/lib/NetworkManager/*
	mkdir -p "$_d/var/lib/NetworkManager"
	# A build root left inside the tree carries $WIFI_DIR with it.
	rm -rf "${_d:?}"/home/*/.cache/debian-aarch64

	rm -f "$_d"/etc/ssh/ssh_host_*
	: >"$_d/etc/machine-id"
	rm -f "$_d/var/lib/dbus/machine-id"

	rm -rf "$_d/root/.ssh"
	rm -rf "${_d:?}"/var/log/journal/*
	rm -f "$_d/var/log/bootstrap.log"
	rm -rf "$_d/tmp/repo"
}

# Refuse to publish <dir> if any of the state above is still in it. This runs on
# the artifact itself (the mounted image, the staging tree) right before it is
# compressed, so a future change to dist_sanitize() cannot silently ship a
# profile.
dist_assert_clean() {
	_d="${1%/}"
	_bad=""

	_bad="$_bad$(find "$_d" -xdev \( -name '*.nmconnection' -o -name '*.lease' \) -print 2>/dev/null | head -5)"
	_bad="$_bad$(find "$_d/etc/ssh" -name 'ssh_host_*' -print 2>/dev/null | head -3)"
	if [ -s "$_d/etc/machine-id" ]; then
		_bad="$_bad$_d/etc/machine-id (not empty)"
	fi
	if [ -e "$_d/var/lib/dbus/machine-id" ]; then
		_bad="$_bad$_d/var/lib/dbus/machine-id"
	fi
	if find "$_d/root/.ssh" "$_d"/home/*/.cache/debian-aarch64 -mindepth 1 -print -quit 2>/dev/null | grep -q .; then
		_bad="$_bad$_d/root/.ssh or $_d/home/*/.cache/debian-aarch64 (not empty)"
	fi

	if [ -n "$_bad" ]; then
		printf '%s\n' "$_bad" | sed 's/^/  /' >&2
		die "refusing to publish: $_d still carries state that is not meant to leave this machine"
	fi
	log "Clean: no Wi-Fi profile, host key or machine id in $_d"
}

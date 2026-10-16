#!/bin/bash
# Configure a freshly debootstrapped Debian rootfs. Runs INSIDE the chroot.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

: "${HOSTNAME:=debian}"
: "${USERNAME:=brunux}"
: "${USERPASS:=debian}"
: "${DEBIAN_SUITE:=forky}"
: "${DEBIAN_MIRROR:=http://deb.debian.org/debian}"
: "${DEBIAN_COMPONENTS:=main contrib non-free non-free-firmware}"
: "${UI:=sway}"
PKGDIR="/tmp/packages"

log() { printf '\n\033[1;34m[rootfs] ==> %s\033[0m\n' "$*"; }
pkgs() { grep -vE '^\s*(#|$)' "$1" 2>/dev/null | tr '\n' ' '; }

log "Configuring apt sources ($DEBIAN_SUITE)"
cat >/etc/apt/sources.list <<EOF
deb $DEBIAN_MIRROR $DEBIAN_SUITE $DEBIAN_COMPONENTS
EOF
apt-get update

log "Installing base packages"
# shellcheck disable=SC2046
apt-get install -y --no-install-recommends $(pkgs "$PKGDIR/base.list")

if [ -f "$PKGDIR/device.list" ]; then
	log "Installing device packages"
	# shellcheck disable=SC2046
	apt-get install -y --no-install-recommends $(pkgs "$PKGDIR/device.list")
fi

if [ -f "$PKGDIR/$UI.list" ]; then
	log "Installing UI packages ($UI)"
	# shellcheck disable=SC2046
	apt-get install -y --no-install-recommends $(pkgs "$PKGDIR/$UI.list")
fi

log "Locale / timezone / hostname"
sed -i 's/^# *en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" >/etc/default/locale
ln -sf /usr/share/zoneinfo/UTC /etc/localtime
echo "$HOSTNAME" >/etc/hostname
cat >/etc/hosts <<EOF
127.0.0.1	localhost
127.0.1.1	$HOSTNAME
::1		localhost ip6-localhost ip6-loopback
EOF

log "Creating user $USERNAME"
if ! id "$USERNAME" >/dev/null 2>&1; then
	useradd -m -s /bin/bash -G sudo,video,audio,input,render,netdev "$USERNAME"
	echo "$USERNAME:$USERPASS" | chpasswd
fi
echo "%sudo ALL=(ALL) NOPASSWD:ALL" >/etc/sudoers.d/99-$USERNAME
chmod 440 /etc/sudoers.d/99-$USERNAME

log "Enabling services"
systemctl enable NetworkManager ssh 2>/dev/null || true
systemctl enable zramswap 2>/dev/null || true
systemctl set-default graphical.target 2>/dev/null || true

if [ -f "$PKGDIR/$UI.list" ] && [ "$UI" = "sway" ]; then
	log "Configuring greetd + sway"
	mkdir -p /etc/greetd
	cat >/etc/greetd/config.toml <<EOF
[terminal]
vt = 1

[default_session]
command = "tuigreet --time --remember --remember-session --cmd sway"
user = "greetd"
EOF
	systemctl enable greetd 2>/dev/null || true

	install -d -o "$USERNAME" -g "$USERNAME" "/home/$USERNAME/.config/sway"
	cat >"/home/$USERNAME/.config/sway/config" <<EOF
# Minimal sway configuration for this device
set \$mod Mod4
set \$term foot

output * bg #000000 solid_color
input * {
	xkb_layout us
}
exec dbus-update-activation-environment --systemd DISPLAY WAYLAND_DISPLAY XDG_CURRENT_DESKTOP
exec swaybg -c '#1d2021'
exec nm-applet --indicator
bindsym \$mod+Return exec \$term
bindsym \$mod+d exec fuzzel
bindsym \$mod+Shift+q kill
include /etc/sway/config.d/*
EOF
	chown -R "$USERNAME:$USERNAME" "/home/$USERNAME/.config"
fi

log "Rootfs configuration done"

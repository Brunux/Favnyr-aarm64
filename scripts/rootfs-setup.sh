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
warn() { printf '\n\033[1;33m[rootfs] WARN: %s\033[0m\n' "$*" >&2; }
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
	log "Configuring greetd"
	# Debian's greetd package creates the '_greetd' system user while the
	# upstream documentation uses 'greetd'. Pick whichever exists, otherwise
	# greetd exits with "configured default session user 'greetd' not found".
	GREETD_USER=_greetd
	grep -q "^$GREETD_USER:" /etc/passwd || GREETD_USER=greetd
	grep -q "^$GREETD_USER:" /etc/passwd || GREETD_USER=root
	mkdir -p /etc/greetd
	# vt 7: Debian's greetd.service declares Conflicts=getty@tty7.service, so it
	# expects to own tty7; on vt 1 it collides with getty@tty1.
	#
	# The personalisation below is the maintainer's Nura greeter (solarized
	# colours, greeting, time format, F-key menus).  Nura runs the tuigreet fork,
	# which reads a TOML config; Debian ships the original 0.9.1, which has no
	# --config, so the same look is expressed with flags.  Unknown --theme
	# components/colours are ignored by 0.9.1 instead of being fatal.  Not
	# portable: the fork's `matrix` backdrop, custom title and widget positions.
	# --remember* needs this cache directory to belong to the greeter user.
	install -d -o "$GREETD_USER" -g "$GREETD_USER" -m 0755 /var/cache/tuigreet
	cat >/etc/greetd/config.toml <<EOF
[terminal]
vt = 7

[default_session]
command = "tuigreet --time --time-format '%a, %d %b  %H:%M' --greeting 'Welcome back to Debian' --greet-align center --width 60 --window-padding 2 --container-padding 1 --prompt-padding 1 --asterisks --asterisks-char '*' --kb-command 2 --kb-sessions 3 --kb-power 12 --power-no-setsid --remember --remember-session --theme 'border=cyan;text=white;time=cyan;container=black;title=yellow;greet=cyan;prompt=cyan;input=white;action=blue;button=yellow' --cmd sway"
user = "$GREETD_USER"
EOF
	systemctl enable greetd 2>/dev/null || true
fi

log "Installing UI files ($UI)"
# uis/<ui>/files is a tree: everything but home/ is copied over / as root,
# home/ is merged into the user's home directory.
if [ -d /tmp/ui-files ]; then
	# The shipped configuration is written against the default user's home;
	# rewrite it when a different USERNAME was requested.
	if [ "$USERNAME" != "brunux" ]; then
		grep -rlZ --binary-files=without-match '/home/brunux' /tmp/ui-files 2>/dev/null |
			xargs -0 -r sed -i "s|/home/brunux|/home/$USERNAME|g"
	fi
	for entry in /tmp/ui-files/*; do
		[ -e "$entry" ] || continue
		[ "$(basename "$entry")" = home ] && continue
		cp -a "$entry" /
	done
	if [ -d /tmp/ui-files/home ]; then
		install -d -o "$USERNAME" -g "$USERNAME" "/home/$USERNAME"
		cp -a /tmp/ui-files/home/. "/home/$USERNAME/"
		chown -R "$USERNAME:$USERNAME" "/home/$USERNAME"
	fi
else
	warn "no UI files staged for '$UI'"
fi

log "Rootfs configuration done"

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
: "${REPO_NAME:=debian-aarch64}"
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

if [ -f "$PKGDIR/tools.list" ]; then
	log "Installing CLI tool packages"
	# shellcheck disable=SC2046
	apt-get install -y --no-install-recommends $(pkgs "$PKGDIR/tools.list")
fi

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

log "Linking the short names Debian's bat and fd-find packages do not install"
# Both binaries were renamed in Debian to avoid a name clash (bat -> batcat,
# fd-find -> fdfind), and every shell alias, the MANPAGER and
# FZF_DEFAULT_COMMAND call them by their upstream names. Deboma's setup.sh does
# the same thing. ln is guarded: a tree without packages/tools.list keeps
# building, it just does not get the aliases' targets.
for _pair in bat:batcat fd:fdfind; do
	_name=${_pair%%:*}
	_target=${_pair#*:}
	if [ -x "/usr/bin/$_target" ]; then
		ln -sfn "/usr/bin/$_target" "/usr/local/bin/$_name"
	fi
done
unset _pair _name _target

if command -v depthchargectl >/dev/null 2>&1; then
	log "Disabling depthcharge-tools' system hooks"
	# depthcharge-tools ships /etc/kernel/postinst.d and
	# /etc/initramfs/post-update.d hooks that build a depthcharge image and write
	# it to a kernel partition on every kernel/initramfs change. They know
	# nothing about this port: the board and the command line come from our
	# deviceinfo and are applied by scripts/make-kpart.sh, and the write goes
	# through scripts/install-kpart.sh, which resolves the partition from
	# kern_guid. Left enabled they also break the *build*: update-initramfs runs
	# inside the chroot, where /proc/cmdline is the host's (on this machine Nura's,
	# which does contain cros_secure), so the hook believes it is on a depthcharge
	# system, finds no kernel partition and exits 1 - and run-parts makes
	# install-kernel.sh fail. This is the switch the package provides.
	install -d /etc/depthcharge-tools
	cat >/etc/depthcharge-tools/config <<'EOF'
[depthchargectl/write]
enable-system-hooks = False
EOF
fi

log "Locale / timezone / hostname"
# Both are build inputs (scripts/lib.sh); the defaults here keep the script
# usable on its own.
LOCALE="${LOCALE:-en_US.UTF-8}"
TIMEZONE="${TIMEZONE:-America/Mexico_City}"

# /etc/locale.gen ships every entry commented out.  Uncomment the requested
# locale and generate it: a name that is not in the file makes locale-gen
# produce nothing at all, silently leaving the image without a UTF-8 locale.
if grep -q "^# *$LOCALE " /etc/locale.gen; then
	sed -i "s|^# *$LOCALE |$LOCALE |" /etc/locale.gen
elif ! grep -q "^$LOCALE " /etc/locale.gen; then
	warn "locale '$LOCALE' is not in /etc/locale.gen, using en_US.UTF-8"
	LOCALE=en_US.UTF-8
	sed -i 's|^# *en_US.UTF-8 |en_US.UTF-8 |' /etc/locale.gen
fi
locale-gen
echo "LANG=$LOCALE" >/etc/default/locale

# A bad name would leave /etc/localtime a dangling symlink (and every timestamp
# wrong), so check it against the tzdata database first.
if [ ! -e "/usr/share/zoneinfo/$TIMEZONE" ]; then
	warn "unknown timezone '$TIMEZONE', using UTC"
	TIMEZONE=UTC
fi
ln -sf "/usr/share/zoneinfo/$TIMEZONE" /etc/localtime
# /etc/timezone is what dpkg-reconfigure tzdata reads; keep it in sync.
printf '%s\n' "$TIMEZONE" >/etc/timezone
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

log "System tuning for this device (see docs/optimization.md)"
# zram: swap in compressed RAM. systemd-zram-generator reads this file (the
# package's own /usr/lib/systemd/zram-generator.conf already creates a zram0 at
# min(ram/2, 4096); these are the values written out so they are visible and
# deliberate). zram-size and swap-priority repeat the defaults; the algorithm is
# the choice: lz4 compresses and decompresses fast, which is what matters when
# the machine is already under pressure, and zstd is the alternative to measure
# if the ratio turns out to matter more than the CPU (docs/optimization.md).
install -d /etc/systemd
cat >/etc/systemd/zram-generator.conf <<'EOF'
[zram0]
zram-size = min(ram / 2, 4096)
compression-algorithm = lz4
swap-priority = 100
EOF

# Memory and writeback policy. Debian's swappiness default is 60, which prefers
# dropping page cache over swapping: wrong when swap is compressed RAM, where a
# swapped page costs a few hundred microseconds and buys real memory back. The
# dirty limits are deliberately the opposite of Deboma's 256/64 MB - on eMMC a
# large dirty window is where multi-second stalls come from when copying files.
install -d /etc/sysctl.d
cat >/etc/sysctl.d/90-zram-emmc.conf <<'EOF'
vm.swappiness = 150
vm.vfs_cache_pressure = 50
vm.dirty_bytes = 67108864
vm.dirty_background_bytes = 16777216
EOF

# I/O scheduler: kyber, which is built for flash (low latency under mixed load)
# instead of the kernel's default mq-deadline. Both are compiled in, so this is a
# runtime choice; see docs/optimization.md.
install -d /etc/udev/rules.d
cat >/etc/udev/rules.d/60-io-scheduler.rules <<'EOF'
# eMMC and microSD: kyber (CONFIG_MQ_IOSCHED_KYBER is built in). The match is the
# whole device only - "mmcblk[0-9]*" would also match mmcblk0p1 and friends, and
# a partition has no scheduler to set.
ACTION=="add|change", KERNEL=="mmcblk[0-9]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="kyber"
EOF

log "Enabling services"
systemctl enable NetworkManager ssh 2>/dev/null || true
# systemd-oomd (PSI-based; Debian's defaults configure it) and the zram swap the
# generator creates. systemd-zram-generator needs no enabling - it is a systemd
# generator, and dev-zram0.swap exists from the moment the unit files are
# generated at boot. zramswap is zram-tools' unit and is gone with the package.
systemctl enable systemd-oomd 2>/dev/null || true
systemctl set-default graphical.target 2>/dev/null || true

log "Generating ssh host keys whenever they are missing"
# Debian's sshd-keygen.service runs only when ConditionFirstBoot=yes, i.e. when
# /etc/machine-id was empty at boot. Everything built here ships as a copy of an
# image (write-sd.sh, install-emmc.sh) and a copy is routinely written from a
# tree that already generated its id, so the condition is false, ssh-keygen
# never runs and ssh.service dies on every boot with
# "sshd: no hostkeys available -- exiting." (hit on both the card and the eMMC
# install). Reset the inherited condition list and key on the thing that
# actually matters, the host keys being absent. It is a no-op - one stat - from
# the boot that creates them on.
install -d /etc/systemd/system/sshd-keygen.service.d
cat >/etc/systemd/system/sshd-keygen.service.d/90-run-when-keys-are-missing.conf <<'EOF'
[Unit]
ConditionFirstBoot=
ConditionPathExists=!/etc/ssh/ssh_host_ed25519_key
EOF

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

	# greetd does not pass its own environment to the session: its worker execs
	# /bin/sh with the PAM environment alone (pam_getenvlist()), and Debian's
	# common-session only brings in pam_systemd. Without pam_env the session
	# therefore starts with no LANG at all - every terminal prints
	# "warning: foot: 'C' is not a UTF-8 locale, falling back to 'C.UTF-8'" - and
	# sorting, matching and non-ASCII text are C instead of UTF-8. These are the
	# two lines Debian's /etc/pam.d/login uses for the same job, and pam_systemd
	# passes what they set on to the user manager as well.
	if ! grep -q 'pam_env\.so.*envfile=/etc/default/locale' /etc/pam.d/greetd; then
		cat >>/etc/pam.d/greetd <<'EOF'
# Locale for the session: greetd builds the session environment from PAM alone.
session	required	pam_env.so readenv=1
session	required	pam_env.so readenv=1 envfile=/etc/default/locale
EOF
	fi
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

log "Installing the repository copy ($REPO_NAME)"
# The device-side scripts (install-emmc.sh, install-kpart.sh) source
# scripts/lib.sh and read devices/, kernels/ and uis/, so the checkout has to be
# on the booted system for them to run there at all.  build-rootfs.sh stages it
# from git (HEAD) and writes /etc/debian-aarch64-revision, which also travels
# with the copy.
if [ -d /tmp/repo ]; then
	DEST="/home/$USERNAME/Dev/$REPO_NAME"
	install -d -o "$USERNAME" -g "$USERNAME" "${DEST%/*}"
	rm -rf "$DEST"
	cp -a /tmp/repo "$DEST"
	[ -f /etc/debian-aarch64-revision ] && cp /etc/debian-aarch64-revision "$DEST/REVISION"
	chown -R "$USERNAME:$USERNAME" "$DEST"
	# The health report is standalone by design, so ship it as a system tool too
	# (it still takes the checkout as an argument for its package checks).
	install -Dm755 /tmp/repo/scripts/check-system.sh /usr/local/sbin/check-system.sh
else
	warn "no repository staged for the image: the device-side scripts will be missing"
fi

log "Rootfs configuration done"

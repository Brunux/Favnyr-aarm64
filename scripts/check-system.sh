#!/bin/bash
# Health report for a system built by this repository.
#
# Runs *on the target device*, not on the build host, so it is standalone: it
# does not source scripts/lib.sh, it needs no repository and it changes nothing.
# Every check maps to a failure this port has actually hit (see AGENTS.md), so a
# red line here is a regression, not a hypothetical.
#
# It must run on the *booted* system. Inside a chroot over a mounted image the
# /proc, /sys and dmesg readings describe the host and `systemctl is-active`
# answers "Running in chroot, ignoring command" with exit status 0 for every
# unit, so the report would be silently wrong; the script warns when PID 1 is
# not systemd.
#
# Usage:
#   scripts/check-system.sh [REPO_DIR]
#
# Run it as the desktop user: the audio checks talk to that user's PipeWire, and
# the privileged reads (dmesg, iw, bluetoothctl, journalctl) go through
# passwordless `sudo -n`. From a root shell it re-executes itself as the session
# user.
#
# With a repository checkout (REPO_DIR, default: the parent of this script) it
# also verifies that every package declared in packages/*.list,
# devices/*/packages.list and uis/*/packages.list is installed.
#
# Exit status: 0 when nothing FAILs, 1 otherwise.
#
# Pitfall, with `pipefail` set: never test the status of `producer | grep -q ...`.
# `grep -q` exits on the first match, the producer then dies of SIGPIPE (141) and
# the pipeline - and therefore the `if` - reports failure even though the pattern
# was found. Whether it bites depends on how far the producer got before the pipe
# closed, so it shows up as a random false FAIL/WARN (the firmware check below
# reported a loaded firmware as missing on the card, and not on the host, with
# the very same input). Feed `grep` a here-string of an already-captured variable
# (`grep -q PAT <<<"$var"`) or use `grep -rqs PAT <path>` instead.
set -uo pipefail

PASS=0 WARN=0 FAIL=0
if [ -t 1 ]; then
	C_OK=$'\033[1;32m' C_WARN=$'\033[1;33m' C_FAIL=$'\033[1;31m'
	C_HEAD=$'\033[1;34m' C_DIM=$'\033[1;30m' C_OFF=$'\033[0m'
else
	C_OK= C_WARN= C_FAIL= C_HEAD= C_DIM= C_OFF=
fi

ok()      { PASS=$((PASS + 1)); printf '  %sOK%s    %s\n' "$C_OK" "$C_OFF" "$*"; }
warn()    { WARN=$((WARN + 1)); printf '  %sWARN%s  %s\n' "$C_WARN" "$C_OFF" "$*"; }
fail()    { FAIL=$((FAIL + 1)); printf '  %sFAIL%s  %s\n' "$C_FAIL" "$C_OFF" "$*"; }
skip()    { printf '  %sSKIP%s  %s\n' "$C_DIM" "$C_OFF" "$*"; }
info()    { printf '  ----  %s\n' "$*"; }
section() { printf '\n%s==> %s%s\n' "$C_HEAD" "$*" "$C_OFF"; }
have()    { command -v "$1" >/dev/null 2>&1; }

# A session started by greetd does not necessarily export the sbin directories,
# and several tools this report needs live there (swapon, findfs, blkid, iw,
# tuigreet). Fix the PATH once here instead of discovering each of them as a
# false failure - the first real report from the booted card did exactly that
# with swapon (which made an active zram swap look absent).
case ":$PATH:" in
	*:/usr/sbin:*) ;;
	*) PATH="$PATH:/usr/sbin:/sbin"; export PATH ;;
esac

# ---------------------------------------------------------------------------
# Privileges: privileged reads use `sudo -n`, session commands must run as the
# desktop user (they need that user's PipeWire and Wayland sockets).
# ---------------------------------------------------------------------------
if [ "$(id -u)" -eq 0 ]; then
	_user="${SUDO_USER:-}"
	if [ -z "$_user" ]; then
		for _d in /run/user/[0-9]*; do
			[ -d "$_d" ] || continue
			_user="$(stat -c %U "$_d" 2>/dev/null)"
			[ -n "$_user" ] && break
		done
	fi
	if [ -n "$_user" ] && [ "$_user" != root ] && have runuser; then
		exec runuser -u "$_user" -- \
			env XDG_RUNTIME_DIR="/run/user/$(id -u "$_user")" \
			"$(readlink -f "$0")" "$@"
	fi
	warn "running as root: the session checks (audio, PipeWire, sway) will not work"
fi

if [ "$(id -u)" -eq 0 ]; then
	asroot() { "$@" 2>/dev/null; }
else
	asroot() { sudo -n "$@" 2>/dev/null; }
	asroot true 2>/dev/null || warn "sudo -n does not work here: privileged checks will be skipped"
fi

# `systemctl is-active` always succeeds inside a chroot ("Running in chroot,
# ignoring command 'is-active'"), so service checks need their own gate: exit
# status 2 means "cannot tell", printed as SKIP instead of OK or FAIL.
if [ -d /run/systemd/system ]; then
	svc() { systemctl is-active --quiet "$1"; }
else
	svc() { return 2; }
fi
# check_svc UNIT "message when the unit is active"
check_svc() {
	svc "$1"
	case "$?" in
		0) ok "$2" ;;
		2) skip "$2 (cannot tell: not running under systemd)" ;;
		*) fail "$2" ;;
	esac
}

# Optional repository checkout, for the declared-package check.
_self="$(cd "$(dirname "$0")" && pwd)"
REPO="${1:-$(cd "$_self/.." 2>/dev/null && pwd)}"
if [ -z "$REPO" ] || [ ! -d "$REPO/packages" ] || [ ! -d "$REPO/uis" ]; then
	REPO=""
fi

# sway -C needs a runtime dir; keep one for the whole run.
RUNTIME="$(mktemp -d)"
trap 'rm -rf "$RUNTIME"' EXIT

printf '%sdebian-aarch64 system report%s  %s\n' "$C_HEAD" "$C_OFF" "$(date '+%Y-%m-%d %H:%M:%S %Z')"

# /proc, /sys and dmesg describe the kernel this script runs under. If PID 1 is
# not systemd it is not the booted Debian image, and everything read from them
# belongs to whatever the host is (typically Nura, over a mounted image).
_init="$(cat /proc/1/comm 2>/dev/null)"
if [ "$_init" != systemd ]; then
	warn "PID 1 is '${_init:-unknown}', not systemd: this is not the booted image."
	warn "The /proc, /sys and dmesg checks below describe the host kernel."
fi

# ---------------------------------------------------------------------------
section "Boot"
# ---------------------------------------------------------------------------
info "kernel  $(uname -r)"
info "cmdline $(cat /proc/cmdline)"
CMDLINE="$(cat /proc/cmdline)"
case "$CMDLINE" in
	*kern_guid=%U*)
		fail "kern_guid=%U was not substituted: depthcharge did not boot this kpart" ;;
	*kern_guid=*)
		ok "depthcharge substituted kern_guid" ;;
	*)
		warn "no kern_guid in the kernel command line" ;;
esac
case "$CMDLINE" in
	*root=UUID=*) ok "root=UUID= present" ;;
	*)            warn "the cmdline does not select the root filesystem by UUID" ;;
esac

# The kernel does not come from an ESP or from /boot: depthcharge loads it from
# the ChromeOS kernel partition named by the kern_guid it substituted. That
# partition is also what scripts/install-kpart.sh rewrites after a kernel
# update, so verify it is really there and really holds a signed FIT.
_guid=""
for _x in $CMDLINE; do
	case "$_x" in
		kern_guid=*) _guid="${_x#kern_guid=}" ;;
	esac
done
if [ -n "$_guid" ]; then
	_kpart_part="$(findfs PARTUUID="$_guid" 2>/dev/null || true)"
	if [ -z "$_kpart_part" ]; then
		fail "kern_guid=$_guid names no partition: nothing to boot from next time"
	else
		ok "kernel partition $_kpart_part (kern_guid=$_guid)"
		if have futility; then
			# Do not pipe futility into `grep -q` (see the header).
			_fv="$(asroot futility show "$_kpart_part" 2>/dev/null || true)"
			case "$_fv" in
				*'Body verification succeeded'*)
					ok "the kernel partition holds a dev-signed FIT"
					;;
				*)
					fail "futility cannot verify a signed kpart in $_kpart_part"
					;;
			esac
		else
			skip "futility not installed (package vboot-kernel-utils): kernel partition not verified"
		fi
	fi
fi

if [ -r /sys/firmware/devicetree/base/model ]; then
	_model="$(tr -d '\0' < /sys/firmware/devicetree/base/model)"
	_compat="$(tr '\0' ' ' < /sys/firmware/devicetree/base/compatible)"
	case "$_compat" in
		*google,*) ok "device tree: $_model" ;;
		*)         fail "device tree is not a Google Chromebook DTB: $_model ($_compat)" ;;
	esac
else
	fail "/sys/firmware/devicetree/base is missing (booted without a device tree?)"
fi

if [ -d "/lib/modules/$(uname -r)" ]; then
	ok "modules for $(uname -r) installed"
else
	fail "no /lib/modules/$(uname -r): the running kernel does not match the installed modules"
fi

# ---------------------------------------------------------------------------
section "Firmware"
# ---------------------------------------------------------------------------
if have dmesg; then
	_dmesg="$(asroot dmesg 2>/dev/null)"
	_missing="$(printf '%s\n' "$_dmesg" | grep -oE 'Direct firmware load for [^ ]+ failed' |
		awk '{print $5}' | sort -u)"
	if [ -z "$_missing" ]; then
		ok "no failed firmware loads in dmesg"
	else
		# A failed direct load is not necessarily a missing firmware: a driver
		# built as a module can request its firmware from the initramfs, long
		# before the root filesystem - and therefore /lib/firmware - exists, and
		# get it seconds later when the module is loaded for good. That is what
		# mtk-scp does here (mediatek/mt8192/scp.img: two -2 failures, then
		# "Booting fw image ... SCP is ready"), so what matters is the outcome,
		# not the first attempt.
		for _fw in $_missing; do
			if grep -q "Booting fw image $_fw" <<<"$_dmesg"; then
				ok "firmware $_fw: requested before the rootfs was up, then loaded"
			elif [ -e "/lib/firmware/$_fw" ] || [ -e "/lib/firmware/$_fw.zst" ] ||
				[ -e "/lib/firmware/$_fw.xz" ]; then
				warn "firmware $_fw exists in /lib/firmware but the kernel never loaded it"
			else
				warn "firmware $_fw is not in /lib/firmware (the kernel asked for it)"
			fi
		done
	fi
	_dmesg_err="$(printf '%s\n' "$_dmesg" | grep -icE '\b(error|failed)\b.*(rtw88|bluetooth|hci|sound|asoc)' || true)"
	[ "${_dmesg_err:-0}" -eq 0 ] && ok "no errors from the wireless/Bluetooth/audio subsystems" ||
		warn "$_dmesg_err dmesg line(s) mention errors in the wireless/Bluetooth/audio subsystems"
else
	skip "dmesg not available"
fi

# ---------------------------------------------------------------------------
section "Storage"
# ---------------------------------------------------------------------------
_root_src="$(findmnt -n -o SOURCE / 2>/dev/null)"
_root_uuid="$(findmnt -n -o UUID / 2>/dev/null)"
info "root    $_root_src (UUID $_root_uuid)"
case "$_root_src" in
	*mmcblk0*) info "booted from the internal eMMC" ;;
	*mmcblk1* | *mmcblk2* | *sd[a-z]*) info "booted from removable storage" ;;
esac

# A root UUID that does not match /etc/fstab boots exactly once. blkid is the
# fallback because findmnt's UUID column needs the device node to be present.
_fstab_uuid="$(awk '$2 == "/" { print $1 }' /etc/fstab 2>/dev/null | sed -n 's/^UUID=//p')"
if [ -z "$_root_uuid" ] && have blkid; then
	_root_uuid="$(blkid -s UUID -o value "$_root_src" 2>/dev/null)"
fi
if [ -z "$_fstab_uuid" ]; then
	fail "/etc/fstab has no UUID= entry for /"
elif [ -z "$_root_uuid" ]; then
	skip "/etc/fstab not compared: the root UUID could not be read"
elif [ "$_fstab_uuid" = "$_root_uuid" ]; then
	ok "/etc/fstab matches the mounted root"
else
	fail "/etc/fstab root UUID ($_fstab_uuid) != mounted root ($_root_uuid): the next boot will fail"
fi

# Two sources, so a stripped-down PATH or an odd util-linux cannot fake a
# "no swap" report on a machine that is swapping.
_swap="$(swapon --show=NAME --noheadings 2>/dev/null || true)"
[ -n "$_swap" ] || _swap="$(awk 'NR > 1 { print $1 }' /proc/swaps 2>/dev/null || true)"
if [ -n "$_swap" ]; then
	ok "swap active: $(printf '%s' "$_swap" | tr '\n' ' ')"
else
	warn "no swap active (zram-tools should provide /dev/zram0)"
fi

# p2 is /boot (see scripts/lib-cgpt.sh). If it is missing the system still
# boots - the kpart is self-contained - but the kernel files would then live in
# the root filesystem, which is not the layout this repository installs.
_boot_fstab="$(awk '$2 == "/boot" { print $1 }' /etc/fstab 2>/dev/null | sed -n 's/^UUID=//p')"
if [ -n "$_boot_fstab" ]; then
	_boot_src="$(findmnt -n -o SOURCE /boot 2>/dev/null || true)"
	_boot_uuid="$(findmnt -n -o UUID /boot 2>/dev/null || true)"
	[ -n "$_boot_uuid" ] && [ -n "$_boot_src" ] ||
		_boot_uuid="$(blkid -s UUID -o value "$_boot_src" 2>/dev/null || true)"
	if ! mountpoint -q /boot; then
		fail "/etc/fstab mounts /boot but it is not mounted"
	elif [ -n "$_boot_uuid" ] && [ "$_boot_uuid" = "$_boot_fstab" ]; then
		ok "/boot is a separate filesystem ($_boot_src) matching /etc/fstab"
	else
		fail "/boot UUID mismatch: fstab $_boot_fstab, mounted ${_boot_uuid:-unknown}"
	fi
	if [ -f "/boot/vmlinuz-$(uname -r)" ]; then
		ok "the running kernel's vmlinuz is on /boot"
	else
		fail "/boot/vmlinuz-$(uname -r) is missing: /boot does not hold the running kernel"
	fi
else
	warn "/etc/fstab has no /boot entry (not the layout this repository installs)"
fi

_used="$(df -P / | awk 'NR == 2 { print $5 }' | tr -d '%')"
if [ -n "$_used" ] && [ "$_used" -ge 90 ]; then
	warn "root filesystem ${_used}% full"
else
	ok "root filesystem ${_used}% used"
fi

# ---------------------------------------------------------------------------
section "Display and GPU"
# ---------------------------------------------------------------------------
_connected=""
for _s in /sys/class/drm/card*-*/status; do
	[ -r "$_s" ] || continue
	[ "$(cat "$_s")" = connected ] && _connected="$(basename "$(dirname "$_s")")"
done
if [ -n "$_connected" ]; then
	ok "display connected on $_connected"
else
	fail "no connected DRM connector"
fi

if [ -d /sys/module/panfrost ]; then
	ok "panfrost loaded (GPU acceleration available)"
else
	warn "panfrost is not loaded: no GPU acceleration"
fi

# The XF86MonBrightness* bindings run brightnessctl as the session user, which
# needs write access to /sys/class/backlight/*/brightness; Debian grants none,
# so uis/sway/files/etc/udev/rules.d/90-brightnessctl.rules does.
_bl=""
for _b in /sys/class/backlight/*/brightness; do
	[ -e "$_b" ] || continue
	_bl="$_b"
	_mode="$(stat -c %a "$_b")"
	_grp="$(stat -c %G "$_b")"
	if [ "$_grp" = video ] && [ "${_mode: -2:1}" -ge 6 ]; then
		ok "backlight writable by group video ($_mode $_grp)"
	else
		fail "$_b is $_mode $_grp: brightnessctl will fail with Permission denied"
	fi
done
[ -n "$_bl" ] || warn "no backlight device"
if [ "$(id -u)" -ne 0 ]; then
	_groups="$(id -nG 2>/dev/null || true)"
	if grep -qw video <<<"$_groups"; then
		ok "session user is in group video"
	else
		fail "session user is not in group video: brightnessctl cannot write the backlight"
	fi
fi

# ---------------------------------------------------------------------------
section "Input"
# ---------------------------------------------------------------------------
if [ -r /proc/bus/input/devices ]; then
	_kb="$(grep -cE '^H: Handlers=.*kbd' /proc/bus/input/devices)"
	# Pointers and touchscreens are the devices that report absolute axes;
	# they carry no 'mouse' handler (that needs CONFIG_INPUT_MOUSEDEV).
	_pt="$(grep -cE '^B: ABS=[0-9a-f]*[1-9a-f]' /proc/bus/input/devices)"
	[ "${_kb:-0}" -ge 1 ] && ok "$_kb keyboard device(s)" || fail "no keyboard device"
	[ "${_pt:-0}" -ge 1 ] && ok "$_pt pointer/touch device(s)" ||
		fail "no pointer or touchscreen device (absolute-axis input)"
	info "devices: $(sed -n 's/^N: Name="\(.*\)"$/\1/p' /proc/bus/input/devices | tr '\n' '|')"
else
	skip "/proc/bus/input/devices not available"
fi

if have keyd || have keyd.rvaiya; then
	check_svc keyd "keyd.service is active"
	# keyd grabs the keyboard, so a `sleep = ...` mapping swallows KEY_SLEEP
	# before systemd-logind's button handling sees it and the Lock key stops
	# suspending the machine. That is exactly what happened once (AGENTS.md).
	if grep -rqsE '^[[:space:]]*sleep[[:space:]]*=' /etc/keyd/ 2>/dev/null; then
		fail "keyd maps the sleep key: logind never sees KEY_SLEEP and the Lock key cannot suspend"
	else
		ok "keyd leaves the sleep key alone"
	fi
	_match="$(asroot journalctl -u keyd -b --no-pager 2>/dev/null | grep 'DEVICE: match' | tail -1)"
	if [ -n "$_match" ]; then
		ok "keyd matched its keyboard: $(printf '%s' "$_match" | sed 's/.*DEVICE: match *//')"
	else
		warn "keyd logged no 'DEVICE: match' this boot"
	fi
else
	skip "keyd is not installed"
fi

# ---------------------------------------------------------------------------
section "Network"
# ---------------------------------------------------------------------------
if have iw; then
	_ifaces="$(asroot iw dev | awk '/Interface/ {print $2}')"
	if [ -n "$_ifaces" ]; then
		ok "wireless interface: $(printf '%s' "$_ifaces" | tr '\n' ' ')"
	else
		fail "no wireless interface (is rtw88_8822ce loading its firmware?)"
	fi
	# A world regulatory domain (country 00) is not a bug by itself: it is what
	# remains while neither NetworkManager, wpa_supplicant nor the AP's 802.11d
	# country IE has set one. What *is* a bug is wireless-regdb missing, which
	# makes cfg80211 log "failed to load regulatory.db" and pins the world domain
	# no matter what connects (AGENTS.md).
	_reg="$(asroot iw reg get | sed -n 's/^country \([^:]*\).*/\1/p' | head -1)"
	_regdb_bad="$(asroot dmesg 2>/dev/null | grep -c 'failed to load regulatory.db' || true)"
	if [ "${_regdb_bad:-0}" -gt 0 ]; then
		warn "cfg80211 could not load regulatory.db (package wireless-regdb): the world domain cannot change"
	elif [ -z "$_reg" ] || [ "$_reg" = 00 ]; then
		info "regulatory domain: world (country 00), set once NetworkManager or the AP provides one"
	else
		ok "regulatory domain: $_reg"
	fi
else
	skip "iw is not installed"
fi

check_svc NetworkManager "NetworkManager is active ($(nmcli -t -f STATE,CONNECTIVITY g 2>/dev/null))"
# Profiles: ask NetworkManager when it runs, otherwise count the files.
_profiles="$(nmcli -t -f NAME,TYPE con show 2>/dev/null | grep -c ':802-11-wireless')"
[ "${_profiles:-0}" -eq 0 ] &&
	_profiles="$(ls /etc/NetworkManager/system-connections/*.nmconnection 2>/dev/null | wc -l)"
[ "${_profiles:-0}" -ge 1 ] && ok "$_profiles Wi-Fi profile(s) installed" ||
	warn "no Wi-Fi profile imported (scripts/wifi.sh import)"
# A profile copied verbatim from Nura pins interface-name=wlan0 and is never
# activated on Debian, where the interface is wlp1s0.
if grep -rqs '^interface-name=' /etc/NetworkManager/system-connections/ 2>/dev/null; then
	warn "a Wi-Fi profile still pins interface-name= (scripts/wifi.sh import strips it)"
elif [ "${_profiles:-0}" -ge 1 ]; then
	ok "no Wi-Fi profile pins an interface name"
fi
have nmtui && ok "nmtui present" || warn "nmtui missing (package network-manager-tui)"

# ---------------------------------------------------------------------------
section "Bluetooth"
# ---------------------------------------------------------------------------
if have bluetoothctl; then
	_show="$(asroot bluetoothctl show)"
	info "controller $(printf '%s\n' "$_show" | grep -m1 'Controller' | cut -d' ' -f2-)"
	case "$_show" in
		*"Powered: yes"*) ok "controller powered" ;;
		*) fail "controller is not powered (bluetoothd? rtl_bt firmware?)" ;;
	esac
	have btmgmt && info "btmgmt: $(asroot btmgmt info | grep -m1 'current settings' | sed 's/^ *//')"
else
	skip "bluetoothctl is not installed (package bluez)"
fi

# ---------------------------------------------------------------------------
section "Audio"
# ---------------------------------------------------------------------------
if have pactl; then
	if pactl info >/dev/null 2>&1; then
		ok "audio server: $(pactl info | sed -n 's/^Server Name: //p')"
		# PipeWire names an ALSA card after the platform device
		# (alsa_card.platform-sound), not after the codec, so the codec name is
		# asked of ALSA instead. What matters is that a card exists at all: with
		# no card there is no UCM boot sequence and no sink but Dummy Output.
		_cards="$(pactl list cards short 2>/dev/null)"
		if [ -z "$_cards" ]; then
			fail "no sound card in PipeWire at all (UCM boot sequence did not run?)"
		else
			ok "sound card: $(printf '%s\n' "$_cards" | head -1 | cut -f2)"
			_codec="$(grep -oE 'mt[0-9]+[a-z0-9_]*' /proc/asound/cards 2>/dev/null | sort -u | tr '\n' ' ')"
			[ -n "$_codec" ] && ok "ALSA codec: $(printf '%s' "$_codec" | sed 's/ $//')" ||
				warn "no MediaTek codec name in /proc/asound/cards"
		fi
		# The card's routing lives in a UCM BootSequence, which alsa-lib only
		# runs on an explicit `set _boot`; alsactl does it at card add. Without
		# it ACP marks HiFi unsupported and the sink is a Dummy Output.
		_profile="$(pactl list cards 2>/dev/null | grep -m1 'Active Profile:' | sed 's/.*: //')"
		case "$_profile" in
			"" | off) fail "card active profile is '${_profile:-none}': the UCM boot sequence did not run (alsa-utils)" ;;
			*)        ok "card active profile: $_profile" ;;
		esac
		_sinks="$(pactl list short sinks 2>/dev/null | awk '{print $2}' | tr '\n' ' ')"
		case "$_sinks" in
			"" | *auto_null*) fail "no real sink (found: ${_sinks:-none}): WirePlumber fell back to Dummy Output" ;;
			*)                ok "sinks: $_sinks" ;;
		esac
	else
		fail "pactl cannot reach the session PipeWire/PulseAudio server"
	fi
else
	skip "pactl is not installed (package pulseaudio-utils)"
fi
# ALSA-only programs need a usable `default` PCM, which pipewire-alsa supplies.
if have aplay; then
	_pcm="$(aplay -L 2>/dev/null || true)"
	if grep -qx default <<<"$_pcm"; then
		ok "alsa 'default' PCM available (pipewire-alsa)"
	else
		fail "no alsa 'default' PCM: aplay/emulators fail without an explicit -D"
	fi
else
	skip "aplay is not installed (package alsa-utils)"
fi

# ---------------------------------------------------------------------------
section "Suspend and power"
# ---------------------------------------------------------------------------
grep -qw mem /sys/power/state 2>/dev/null && ok "/sys/power/state advertises suspend ($(cat /sys/power/state))" ||
	fail "no 'mem' in /sys/power/state: the kernel cannot suspend"
check_svc systemd-logind "systemd-logind is active (its button handling is the polkit-free suspend path)"
check_svc polkit "polkitd is active (session suspend via loginctl)"
[ -x "$HOME/.config/bin/sway-suspend" ] && ok "sway-suspend helper is executable" ||
	warn "~/.config/bin/sway-suspend is missing or not executable"
_susp="$(asroot journalctl -b --no-pager | grep -c 'PM: suspend entry')"
[ "${_susp:-0}" -ge 1 ] && ok "$_susp suspend/resume cycle(s) recorded in this boot" ||
	warn "no suspend recorded in this boot: press the Lock key to test it"

# ---------------------------------------------------------------------------
section "System tuning"
# ---------------------------------------------------------------------------
# Everything below is a deliberate choice documented in docs/optimization.md, and
# the failure mode is always the same: something silently takes it back (a
# package resets the governor, an overlay stops being installed) and the machine
# is slower or hungrier with no message anywhere.

_govs="$(cat /sys/devices/system/cpu/cpufreq/policy*/scaling_governor 2>/dev/null | sort -u | tr '\n' ' ')"
case "$_govs" in
	"schedutil ") ok "cpufreq governor is schedutil on every policy" ;;
	*) warn "cpufreq governor is '${_govs% }', not schedutil (ondemand pins both clusters at max)" ;;
esac

# MGLRU (CONFIG_LRU_GEN + CONFIG_LRU_GEN_ENABLED): reclaim by generation instead
# of a full scan, which is what a 4 GB machine under zram pressure needs.
if [ -r /sys/kernel/mm/lru_gen/enabled ]; then
	_lru="$(cat /sys/kernel/mm/lru_gen/enabled)"
	case "$_lru" in
		0x0000) fail "MGLRU is built in but disabled (lru_gen/enabled=$_lru)" ;;
		*) ok "MGLRU enabled ($_lru)" ;;
	esac
else
	fail "no /sys/kernel/mm/lru_gen: the kernel was built without CONFIG_LRU_GEN"
fi

# THP=madvise comes from the kernel command line: with 4 GB and zram, 'always'
# both wastes memory and adds latency spikes.
_thp="$(cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null)"
case "$_thp" in
	*'[madvise]'*) ok "transparent hugepages: madvise" ;;
	*) warn "transparent hugepages: ${_thp:-unknown} (expected [madvise] from the cmdline)" ;;
esac

# Swap has to be zram, and it has to be a unit: systemd-oomd's ManagedOOMSwap
# only sees units, and the generator (not a swapon in a script) is what creates
# dev-zram0.swap.
case "$(cat /proc/swaps 2>/dev/null)" in
	*/dev/zram0*) ok "swap is zram ($(awk '/zram0/ { print $4 " kB" }' /proc/swaps | head -1))" ;;
	*) fail "no zram swap: memory pressure has nowhere to go" ;;
esac
case "$(systemctl list-units --all --no-legend 'dev-zram0.swap' 2>/dev/null)" in
	*dev-zram0.swap*) ok "zram swap is a systemd unit (systemd-oomd can manage it)" ;;
	*) warn "zram swap is not a dev-zram0.swap unit: systemd-oomd cannot manage it" ;;
esac
check_svc systemd-oomd "systemd-oomd is active (kills a hog on memory pressure)"

[ "$(cat /proc/sys/vm/swappiness 2>/dev/null || echo 0)" -ge 100 ] &&
	ok "vm.swappiness=$(cat /proc/sys/vm/swappiness) (swapping to compressed RAM is cheap)" ||
	warn "vm.swappiness=$(cat /proc/sys/vm/swappiness): too low for a zram-only system"

case "$(cat /sys/block/mmcblk0/queue/scheduler 2>/dev/null)" in
	*'[kyber]'*) ok "eMMC I/O scheduler: kyber" ;;
	*) warn "eMMC I/O scheduler: $(cat /sys/block/mmcblk0/queue/scheduler 2>/dev/null) (expected [kyber])" ;;
esac

# The kernel options the tuning depends on, read from the config the image ships
# in /boot rather than assumed: a kernel built from a different config (or the
# stock one after a bad install) is invisible otherwise.
_cfg="$(ls -1 /boot/config-* 2>/dev/null | head -1)"
if [ -r "$_cfg" ]; then
	_missing_cfg=""
	[ -n "$(grep -x 'CONFIG_HZ=1000' "$_cfg" 2>/dev/null)" ] || _missing_cfg="$_missing_cfg HZ=1000"
	[ -n "$(grep -x 'CONFIG_LRU_GEN=y' "$_cfg" 2>/dev/null)" ] || _missing_cfg="$_missing_cfg LRU_GEN"
	[ -n "$(grep -x 'CONFIG_CPU_FREQ_DEFAULT_GOV_SCHEDUTIL=y' "$_cfg" 2>/dev/null)" ] || _missing_cfg="$_missing_cfg governor=schedutil"
	[ -z "$_missing_cfg" ] && ok "kernel config carries the tuning ($(basename "$_cfg"))" ||
		warn "kernel config lacks:$_missing_cfg"
else
	skip "no /boot/config-* to check the kernel options against"
fi

# ---------------------------------------------------------------------------
section "Clock"
# ---------------------------------------------------------------------------
if have timedatectl; then
	info "$(timedatectl show -p Timezone --value 2>/dev/null) / local RTC $(timedatectl show -p LocalRTC --value 2>/dev/null)"
	[ "$(timedatectl show -p NTPSynchronized --value 2>/dev/null)" = yes ] && ok "clock synchronised" ||
		warn "clock not synchronised (systemd-timesyncd needs network)"
else
	skip "timedatectl is not installed"
fi

# ---------------------------------------------------------------------------
section "Session"
# ---------------------------------------------------------------------------
check_svc greetd "greetd is active"
if [ -r /etc/greetd/config.toml ]; then
	# Debian's greetd creates the _greetd user and its unit conflicts with
	# getty@tty7, so vt must be 7; with any other name the service dies and the
	# machine falls back to a console login (AGENTS.md).
	_vt="$(sed -n 's/^vt *= *//p' /etc/greetd/config.toml | tr -d '"')"
	_user="$(sed -n 's/^user *= *//p' /etc/greetd/config.toml | tr -d '"')"
	[ "$_vt" = 7 ] && ok "greeter vt = 7" ||
		fail "greeter vt = ${_vt:-unset} (must be 7: greetd conflicts with getty@tty7)"
	getent passwd "$_user" >/dev/null 2>&1 && ok "greeter user $_user exists" ||
		fail "greeter user '${_user:-unset}' does not exist: greetd exits and the greeter never shows"
else
	fail "/etc/greetd/config.toml is missing"
fi
have tuigreet && ok "tuigreet is installed" || warn "tuigreet is not installed"
if have sway; then
	# The session runs the wlroots fork built by scripts/build-swayfx.sh, not
	# Debian's plain sway: 26-swayfx.conf needs it (corners, shadows, and blur
	# switched off explicitly).  The version string is the only cheap way to
	# tell the two /usr/bin/sway binaries apart, and plain sway would start but
	# silently ignore the effects.
	_sway_version="$(sway --version 2>/dev/null | head -n1)"
	case "$_sway_version" in
	*swayfx*) ok "compositor is $_sway_version" ;;
	*) fail "compositor is '${_sway_version:-unknown}', not swayfx: rounded corners and shadows are missing (run scripts/install-swayfx.sh)" ;;
	esac
	if XDG_RUNTIME_DIR="$RUNTIME" WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 \
		sway -C -c /etc/sway/config >/dev/null 2>&1; then
		ok "sway configuration is valid"
	else
		fail "sway -C rejected /etc/sway/config"
	fi
	# sway starts Xwayland itself (it is not in the configuration), so a missing
	# package shows up only as two red lines in the journal and as X11 programs
	# that cannot launch (see AGENTS.md).
	if grep -rqsE '^[[:space:]]*xwayland[[:space:]]+disable' /etc/sway/config /etc/sway/config.d 2>/dev/null; then
		skip "Xwayland disabled in the sway configuration"
	elif [ -x /usr/bin/Xwayland ]; then
		ok "Xwayland present (X11 programs can run)"
	else
		warn "Xwayland is missing: sway logs a startup error and X11 programs cannot run"
	fi
fi
_missing_cmds=""
for _c in waybar foot rofi swaylock swaynag swaybg brightnessctl grimshot wlsunset wpctl pactl mako; do
	have "$_c" || _missing_cmds="$_missing_cmds $_c"
done
if [ -z "$_missing_cmds" ]; then
	ok "every command the sway configuration invokes is installed"
else
	warn "commands referenced by the sway configuration are missing:$_missing_cmds"
fi

# Notifications: mako owns org.freedesktop.Notifications on the session bus. If
# the daemon is not running (crashed, or never started) nothing logs it and
# every application's notifications silently disappear, so ask the bus.
if have busctl; then
	_notify_name="$(busctl --user list --no-legend 2>/dev/null | awk '$1 == "org.freedesktop.Notifications" { print $1 }')"
	if [ -n "$_notify_name" ]; then
		ok "a notification daemon owns org.freedesktop.Notifications (mako)"
	else
		warn "no notification daemon on the session bus: mako is not running, notifications are lost"
	fi
else
	skip "busctl is not installed"
fi

# The bar's menus are the whole port of Deboma's panels: each indicator draws
# from waybar and runs ~/.config/bin/bar-* on click.  A missing script or a
# missing tool does not log anything - the indicator still appears and every
# click is a silent no-op - so this is a FAIL, not a warning.
_missing_bar=""
for _c in rofi jq nmcli bluetoothctl pactl brightnessctl foot; do
	have "$_c" || _missing_bar="$_missing_bar $_c"
done
for _s in bar-pick bar-menu bar-net bar-bt bar-key bar-audio bar-updates bar-power; do
	[ -x "$HOME/.config/bin/$_s" ] || _missing_bar="$_missing_bar bin/$_s"
done
if [ -z "$_missing_bar" ]; then
	ok "bar menus complete (scripts plus fuzzel, nmcli, bluetoothctl, jq)"
else
	fail "bar menus broken:$_missing_bar (clicking those indicators does nothing)"
fi

# The idle screensaver is a foot window running ~/.config/bin/sway-screensaver:
# without tte or the logo it still shows the static frame, but the animation
# (the point of the entry) is silently missing.
_saver=""
have tte || _saver="$_saver tte"
[ -r "$HOME/.config/nureb/screensaver.txt" ] || _saver="$_saver .config/nureb/screensaver.txt"
[ -x "$HOME/.config/bin/sway-screensaver" ] || _saver="$_saver bin/sway-screensaver"
[ -r "$HOME/.config/foot/screensaver.ini" ] || _saver="$_saver .config/foot/screensaver.ini"
if [ -z "$_saver" ]; then
	ok "idle screensaver complete (tte, logo, launcher, terminal profile)"
else
	fail "idle screensaver incomplete:$_saver (the idle screen is blank)"
fi

# The CLI tools of packages/tools.list, wired by /etc/profile.d/90-cli-tools.sh
# and ~/.bashrc.  A missing binary is the quietest kind of breakage: the alias is
# simply not defined and `ls` silently falls back to coreutils, so this is a FAIL
# and not a warning.
_missing_tools=""
for _c in eza bat fd rg fzf zoxide starship tldr btop fastfetch inxi tmux \
	lazygit gum curl unzip whois socat inotifywait qrencode zbarimg tesseract \
	ffmpeg node npm col man; do
	have "$_c" || _missing_tools="$_missing_tools $_c"
done
[ -r "$HOME/.bashrc" ] || _missing_tools="$_missing_tools .bashrc"
[ -r /etc/profile.d/90-cli-tools.sh ] || _missing_tools="$_missing_tools profile.d/90-cli-tools.sh"
[ -r "$HOME/.config/starship.toml" ] || _missing_tools="$_missing_tools .config/starship.toml"
if [ -z "$_missing_tools" ]; then
	ok "CLI tools complete (eza, bat, fd, rg, fzf, zoxide, starship, btop, ...)"
else
	fail "CLI tools incomplete:$_missing_tools"
fi

# `ls` is the one people notice.  Ask an interactive bash what the alias is -
# that also proves ~/.bashrc is read at all - and match with a case statement:
# `producer | grep -q` is the pipefail trap this script documents at the top.
_ls_alias="$(bash -ic 'alias ls' 2>/dev/null || true)"
case "$_ls_alias" in
	*eza*) ok "ls is eza in an interactive shell" ;;
	*) warn "ls is not eza (alias: ${_ls_alias:-none}); ~/.bashrc is not being read" ;;
esac

# The launcher is rofi, themed by two files that the launcher, the window
# switcher and the bar's dmenu all read. Without them rofi still opens, in its
# default colours, which looks like a themed launcher that lost its theme rather
# than a missing file.
if have rofi && [ -r "$HOME/.config/rofi/config.rasi" ] && [ -r "$HOME/.config/rofi/solarized.rasi" ]; then
	ok "launcher complete (rofi, config.rasi, solarized.rasi)"
else
	fail "launcher incomplete: rofi, ~/.config/rofi/config.rasi or solarized.rasi missing"
fi
# rofi is also the bar's dmenu: without it every bar click is a silent no-op.
[ -x "$HOME/.config/bin/bar-pick" ] && ok "bar-pick (rofi -dmenu) is executable" ||
	fail "bin/bar-pick is missing or not executable"

# The manual, the key view, dictation and hibernate. The first two are pure
# scripts (a missing one is invisible: the menu entry just does nothing); the
# last two depend on hardware and packages, so they warn instead of failing.
_learn_pages=$(ls /usr/share/doc/nureb/*.md 2>/dev/null | wc -l)
if [ "$_learn_pages" -ge 6 ] && [ -x "$HOME/.config/bin/rofi-learn" ]; then
	ok "manual installed ($_learn_pages pages) and rofi-learn present"
else
	fail "manual incomplete: $_learn_pages page(s) in /usr/share/doc/nureb, rofi-learn $( [ -x "$HOME/.config/bin/rofi-learn" ] && echo present || echo missing)"
fi
[ -x "$HOME/.config/bin/rofi-keys" ] && ok "keybindings view (rofi-keys) is executable" ||
	fail "bin/rofi-keys is missing: the key view has no script"

# Dictation: the shortcut records with parecord and transcribes with whisper-cli.
# The model is fetched on first use, so its absence is not a failure.
if have whisper-cli && have wtype && have parecord; then
	ok "dictation tools present (whisper-cli, wtype, parecord)"
else
	warn "dictation incomplete:$(have whisper-cli || echo ' whisper-cli')$(have wtype || echo ' wtype')$(have parecord || echo ' parecord')"
fi
_model="$(ls "$HOME"/.local/share/whisper/ggml-*.bin 2>/dev/null | head -1)"
[ -n "$_model" ] && ok "speech model present ($(basename "$_model"))" ||
	skip "no speech model yet: the dictation shortcut fetches tiny.en (78 MB) on first use"

# Hibernate needs both a disk swap and resume=; without them the helper explains
# what is missing, which is the correct behaviour, not a failure.
_hib_swap=$(awk 'NR > 1 && $1 !~ /zram/ { print $1 }' /proc/swaps 2>/dev/null | head -1)
if [ -n "$_hib_swap" ]; then
	case " $(cat /proc/cmdline) " in
	*" resume="*) ok "hibernation possible (disk swap $_hib_swap, resume= present)" ;;
	*) warn "disk swap $_hib_swap is active but the kernel command line has no resume=: hibernate would not resume" ;;
	esac
else
	skip "no disk swap: hibernation needs a swap labelled nureb_swap (see sway-hibernate)"
fi
[ -x "$HOME/.config/bin/sway-hibernate" ] || fail "bin/sway-hibernate is missing"

# Agents: the menu is a script; whether a CLI is installed is informational,
# because installing one is the menu's own job.
[ -x "$HOME/.config/bin/rofi-agents" ] && ok "agents menu (rofi-agents) is executable" ||
	fail "bin/rofi-agents is missing: Alt+Ctrl+Shift+A does nothing"
_agent_bins=""
for _a in claude codex opencode gemini copilot; do have "$_a" && _agent_bins="$_agent_bins $_a"; done
[ -n "$_agent_bins" ] && ok "coding agent CLI(s) installed:$_agent_bins" ||
	skip "no coding-agent CLI installed yet (the Agents menu installs one with npm)"

# The session's own launcher entries. They are the only way to reach the bar's
# panels and the session actions by typing, and a missing one is invisible: the
# list simply has fewer rows and nobody notices.
_missing_desktop=""
for _d in wifi wifi-nmtui bluetooth audio-output power lock screensaver suspend \
	screenshot-area lazygit tmux; do
	[ -r "/usr/share/applications/org.nureb.$_d.desktop" ] || _missing_desktop="$_missing_desktop $_d"
done
if [ -z "$_missing_desktop" ]; then
	ok "launcher entries present (session actions, nmtui, lazygit, tmux)"
else
	fail "launcher entries missing:$_missing_desktop"
fi

# waybar reads the backlight by device name and shows an empty module (with no
# error anywhere) when the name does not exist.  Read it with bash's own regex
# rather than a pipe: `set -o pipefail` plus an early-exiting filter is the
# pitfall this script already documents at the top.
_bl=""
_cfg="$HOME/.config/waybar/config.jsonc"
if [ -r "$_cfg" ]; then
	while IFS= read -r _line; do
		if [[ $_line =~ \"device\":\ *\"([^\"]*)\" ]]; then
			_bl=${BASH_REMATCH[1]}
			break
		fi
	done <"$_cfg"
fi
if [ -n "$_bl" ]; then
	if [ -d "/sys/class/backlight/$_bl" ]; then
		ok "waybar backlight device $_bl exists"
	else
		fail "waybar names backlight device '$_bl'; /sys/class/backlight has: $(ls /sys/class/backlight 2>/dev/null | tr '\n' ' ')"
	fi
fi

# ---------------------------------------------------------------------------
section "Fonts and icons"
# ---------------------------------------------------------------------------
if have fc-list; then
	_families="$(fc-list : family 2>/dev/null | tr ',' '\n' | sed 's/^ *//' | sort -u)"
	for _f in "JetBrains Mono" "FontAwesome" "Material Design Icons" "Symbols Nerd Font"; do
		if grep -qF "$_f" <<<"$_families"; then
			ok "font family present: $_f"
		else
			fail "font family missing: $_f (waybar icons render as tofu boxes)"
		fi
	done
else
	skip "fc-list is not installed"
fi

# ---------------------------------------------------------------------------
section "Declared packages"
# ---------------------------------------------------------------------------
if [ -n "$REPO" ] && have dpkg-query; then
	_missing=""
	_count=0
	for _list in "$REPO"/packages/*.list "$REPO"/devices/*/packages.list "$REPO"/uis/*/packages.list; do
		[ -f "$_list" ] || continue
		while read -r _pkg; do
			case "$_pkg" in '' | '#'*) continue ;; esac
			_count=$((_count + 1))
			_st="$(dpkg-query -W -f='${Status}' "$_pkg" 2>/dev/null || true)"
			grep -q 'install ok installed' <<<"$_st" || _missing="$_missing $_pkg"
		done <"$_list"
	done
	if [ -z "$_missing" ]; then
		ok "all $_count declared package(s) are installed"
	else
		fail "declared but not installed:$_missing"
	fi
else
	skip "no repository checkout: run scripts/check-system.sh <repo> to check declared packages"
fi

# ---------------------------------------------------------------------------
section "Journal"
# ---------------------------------------------------------------------------
_errs="$(asroot journalctl -b -p err --no-pager | grep -vc '^-- No entries --')"
if [ "${_errs:-0}" -eq 0 ]; then
	ok "no error-priority messages this boot"
else
	warn "$_errs error-priority line(s) this boot; the last ones:"
	asroot journalctl -b -p err --no-pager | tail -6 | sed 's/^/          /'
fi

# ---------------------------------------------------------------------------
printf '\n%s==> Summary%s  %d OK, %d WARN, %d FAIL\n' "$C_HEAD" "$C_OFF" "$PASS" "$WARN" "$FAIL"
[ "$FAIL" -eq 0 ] || exit 1

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
# greetd hands the session only the PAM environment, so pam_env is what puts the
# locale there; without it every terminal warns "'C' is not a UTF-8 locale" and
# sorting/matching run on the C locale.
case "${LANG:-}" in
*UTF-8* | *utf8*) ok "session locale is UTF-8 (${LANG})" ;;
*) fail "session LANG is '${LANG:-unset}': greetd needs pam_env in /etc/pam.d/greetd (terminals warn and sorting is C)" ;;
esac
if have sway; then
	# The session runs the wlroots fork built by scripts/build-swayfx.sh, not
	# Debian's plain sway: 26-swayfx.conf needs it (corners, shadows, the blur
	# tuning and the layer_effects).  The version string is the only cheap way to
	# tell the two /usr/bin/sway binaries apart, and plain sway would start but
	# silently ignore the effects.
	_sway_version="$(sway --version 2>/dev/null | head -n1)"
	case "$_sway_version" in
	*swayfx*) ok "compositor is $_sway_version" ;;
	*) fail "compositor is '${_sway_version:-unknown}', not swayfx: rounded corners and shadows are missing (run scripts/install-swayfx.sh)" ;;
	esac
	# The renderer the session actually got. PanVK has no Mali-G57 support
	# ("vulkaninfo: Mali-G57 not supported") and SceneFX only implements a GLES
	# renderer, so panfrost through EGL is the whole stack. A forced
	# LIBGL_ALWAYS_SOFTWARE, a broken firmware load or a Mesa regression would
	# silently drop the session onto llvmpipe/swrast, where every frame and the
	# blur are drawn on the CPU - eglinfo reports the renderer without needing a
	# display, and it ships in mesa-utils (already a package here).
	if have eglinfo; then
		_gpu_renderer="$(eglinfo -B 2>/dev/null | awk -F': ' '/OpenGL ES profile renderer/ { print $2; exit }')"
		case "$_gpu_renderer" in
		*Panfrost*) ok "GPU renderer is $_gpu_renderer" ;;
		"") warn "eglinfo reported no renderer: cannot confirm the GPU driver" ;;
		*) fail "GPU renderer is '$_gpu_renderer', not Panfrost: the session is drawing on software (check LIBGL_ALWAYS_SOFTWARE and the GPU firmware)" ;;
		esac
	else
		skip "eglinfo is missing (mesa-utils): cannot confirm the GPU driver"
	fi
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

# A notification icon is a name mako looks up on its icon-path, and one it cannot
# find draws nothing at all: the card is text-only, with no log line and no error.
# The names are lib-rofi.sh's UI_ICON_* constants.
if [ -r "$HOME/.config/bin/lib-rofi.sh" ] && [ -d /usr/share/icons/LazyOS ]; then
	_missing_icon=
	for _n in $(sed -n 's/^UI_ICON_[A-Z]*=\([a-z-]*\)$/\1/p' "$HOME/.config/bin/lib-rofi.sh"); do
		ls /usr/share/icons/LazyOS/*/apps/"$_n".png >/dev/null 2>&1 ||
			_missing_icon="$_missing_icon $_n"
	done
	[ -z "$_missing_icon" ] &&
		ok "notification icons resolve in the LazyOS theme" ||
		warn "notification icon(s) missing from /usr/share/icons/LazyOS:$_missing_icon (scripts/make-icons.sh)"
fi

# mako reads icon-path itself (no theme metadata), so both halves have to be there
# for any picture to appear. The config has no --check, and a name it cannot find
# is silent, which is why this is asked of the configuration directly.
if [ -r "$HOME/.config/mako/config" ]; then
	_mako_icons=$(sed -n 's/^icons=//p' "$HOME/.config/mako/config" | head -1)
	_mako_path=$(sed -n 's/^icon-path=//p' "$HOME/.config/mako/config" | head -1)
	if [ "$_mako_icons" = 1 ] && [ -n "$_mako_path" ] && [ -d "$_mako_path" ]; then
		ok "notifications can carry a picture (icons=1, icon-path=$_mako_path)"
	else
		warn "mako draws no picture: icons=1 and an existing icon-path are missing from ~/.config/mako/config"
	fi
fi

# The bar's menus are the whole port of Deboma's panels: each indicator draws
# from waybar and runs ~/.config/bin/bar-* on click.  A missing script or a
# missing tool does not log anything - the indicator still appears and every
# click is a silent no-op - so this is a FAIL, not a warning.
_missing_bar=""
for _c in rofi jq nmcli bluetoothctl pactl brightnessctl foot; do
	have "$_c" || _missing_bar="$_missing_bar $_c"
done
for _s in bar-pick bar-menu bar-net bar-bt bar-key bar-audio bar-updates bar-power \
	bar-busy busy lib-rofi.sh; do
	[ -r "$HOME/.config/bin/$_s" ] || _missing_bar="$_missing_bar bin/$_s"
done
if [ -z "$_missing_bar" ]; then
	ok "bar menus complete (scripts plus rofi, nmcli, bluetoothctl, jq)"
else
	fail "bar menus broken:$_missing_bar (clicking those indicators does nothing)"
fi

# The bar's busy card is also the stop button: config.jsonc's custom/busy
# on-click runs `bar-busy stop`, and a lost key leaves a spinner that draws and
# does nothing when clicked - which is indistinguishable from "this action
# cannot be stopped", the answer the card gives on purpose. Read the module
# object out of the file (it is JSON with comments, so jq cannot parse it): the
# block starts at its key and ends at the first line that is only `}`.
_busy_mod=""
[ -r "$HOME/.config/waybar/config.jsonc" ] &&
	_busy_mod=$(awk '/"custom\/busy"[[:space:]]*:[[:space:]]*\{/ { f = 1 }
		f && /^[[:space:]]*\}/ { print; exit }
		f' "$HOME/.config/waybar/config.jsonc" 2>/dev/null || true)
case $_busy_mod in
*'on-click'*bar-busy*stop*)
	ok "the bar's busy card is the stop button (custom/busy on-click)" ;;
'')
	warn "custom/busy is not readable in ~/.config/waybar/config.jsonc: the spinner may not be clickable" ;;
*)
	warn "custom/busy has no on-click stop: the spinner cannot be clicked" ;;
esac

# Both ends share one protocol, exercised here for real because it is invisible
# until it fails: `busy start` records BUSY_STOP - a shell line, not a pid - in
# <tag>.stop (0600, since something executes it), `bar-busy stop` removes the
# card and runs it, and `busy cancelled` then answers 0 so the producer reports
# nothing instead of half a result. The probe is one file in the runtime
# directory and it is removed before this block ends.
_busy_rt=${XDG_RUNTIME_DIR:-/tmp}/lazyos-busy
_busy_bin="$HOME/.config/bin/busy"
if [ ! -d "${XDG_RUNTIME_DIR:-/tmp}" ] || [ ! -w "${XDG_RUNTIME_DIR:-/tmp}" ]; then
	skip "no writable runtime directory: the busy/stop protocol cannot be exercised here"
else
	if BUSY_STOP=: "$_busy_bin" start check-system Probe 'probe' >/dev/null 2>&1 &&
		[ -f "$_busy_rt/check-system.stop" ] &&
		[ "$(stat -c %a "$_busy_rt/check-system.stop" 2>/dev/null)" = 600 ]; then
		ok "busy records the stop command (mode 0600) the spinner runs"
		if "$_busy_bin" cancelled check-system >/dev/null 2>&1; then
			warn "busy cancelled says a running card is already gone (a stopped lookup looks finished)"
		else
			ok "busy cancelled tells a running task from a stopped one"
		fi
	else
		warn "busy does not record a stop command in $_busy_rt: the spinner cannot stop its task"
	fi
	"$_busy_bin" stop check-system >/dev/null 2>&1 || true
	if "$_busy_bin" cancelled check-system >/dev/null 2>&1; then
		ok "after a stop, busy says the task was cancelled (the producer reports nothing)"
	else
		warn "busy cancelled still sees the card after busy stop (the producer would report a half result)"
	fi
fi

# The idle screen is a foot window running ~/.config/bin/lazyos-screensaver: a
# loop of tte effects over the LazyOS wordmark, started by swayidle at 150 s.
# Each piece fails silently on its own: without tte or the art the static frame
# stays (the loop, which is the point of the entry, is gone), without the
# launcher the screen never appears, without the lock helper the 300 s timer
# does nothing, and without swayidle nothing starts at all.
_saver=""
have tte || _saver="$_saver tte"
have swayidle || _saver="$_saver swayidle"
[ -s "$HOME/.config/lazyos/screensaver.txt" ] || _saver="$_saver .config/lazyos/screensaver.txt"
[ -x "$HOME/.config/bin/lazyos-screensaver" ] || _saver="$_saver bin/lazyos-screensaver"
[ -x "$HOME/.config/bin/sway-lock" ] || _saver="$_saver bin/sway-lock"
[ -r "$HOME/.config/foot/screensaver.ini" ] || _saver="$_saver .config/foot/screensaver.ini"
if [ -z "$_saver" ]; then
	ok "idle screen complete (tte, LazyOS art, launcher, sway-lock, terminal profile)"
else
	fail "idle screen incomplete:$_saver (the idle screen is blank or never starts)"
fi

# The chain itself, read from the drop-in that owns it: a missing line means the
# screen never appears (150 s), the session never locks (300 s) or the machine
# suspends before the lock is covered (before-sleep).  The numbers are compared,
# because a different timeout is the same as none.
_idle_cfg=/etc/sway/config.d/23-lazyos-screensaver.conf
_idle=""
[ -r "$_idle_cfg" ] || _idle=" $_idle_cfg"
if [ -r "$_idle_cfg" ]; then
	grep -q "timeout 150 .*lazyos-screensaver" "$_idle_cfg" || _idle="$_idle no-150s-screen"
	grep -q "timeout 300 .*sway-lock" "$_idle_cfg" || _idle="$_idle no-300s-lock"
	grep -q "before-sleep .*sway-lock --wait" "$_idle_cfg" || _idle="$_idle no-lock-before-sleep"
fi
pgrep -x swayidle >/dev/null 2>&1 || _idle="$_idle swayidle-not-running"
if [ -z "$_idle" ]; then
	ok "idle chain armed (150 s idle screen, 300 s lock, lock before sleep)"
else
	fail "idle chain broken:$_idle"
fi

# The bug the guard in lazyos-screensaver exists for: a saver left running under
# an active lock animates to nobody and spends CPU (it used to do exactly that
# for the 150 s between the two timers).
if pgrep -x swaylock >/dev/null 2>&1 && pgrep -f org.lazyos.screensaver >/dev/null 2>&1; then
	warn "the idle screen is running under swaylock (invisible CPU; it should have refused)"
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

# The editors. /usr/bin/vi is the part that goes missing on its own: it is an
# update-alternatives link that vim's postinst registers, so a `vim` installed
# without its postinst running leaves `vim` working and `vi` absent, which only
# the script that calls vi notices.
_editors=""
for _c in nvim vim vi nano; do
	have "$_c" || _editors="$_editors $_c"
done
if [ -z "$_editors" ]; then
	ok "editors installed (nvim, vim, vi, nano)"
else
	fail "editor(s) missing:$_editors"
fi
# $EDITOR is what eff, `git commit` and `crontab -e` open, and it comes from
# /etc/profile.d/90-cli-tools.sh - which a session started by greetd does not
# source, so the question is asked of a login shell instead of this environment.
_editor=$(bash -lc 'printf %s "${EDITOR:-}"' 2>/dev/null || true)
if [ "$_editor" = nvim ]; then
	ok "EDITOR is nvim (eff, git commit, crontab -e)"
else
	warn "a login shell has EDITOR='${_editor:-unset}': eff and git open nano instead of nvim"
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

# Every Alt+1 second action in these menus (delete a clipboard entry, unmount a
# disk, act on a file row) is rofi's -kb-custom-1, and sway would answer Alt+1
# itself with "workspace number 1" before rofi ever saw the key: a layer surface
# with exclusive keyboard interactivity does not stop compositor bindings, only
# an active zwp_keyboard_shortcuts_inhibitor does, and rofi asks for one only
# when `global-kb` is on. Off, the menu switches workspace and does nothing -
# there is no error anywhere, which is why it is checked and not assumed.
if [ -r "$HOME/.config/rofi/config.rasi" ]; then
	if grep -qs '^[[:space:]]*global-kb:[[:space:]]*true' "$HOME/.config/rofi/config.rasi"; then
		ok "rofi inhibits the compositor's shortcuts while a menu is open (global-kb)"
	else
		warn "rofi/config.rasi does not set global-kb: true - Alt+1 inside a menu is taken by sway's workspace binding"
	fi
fi

# The Search key is the file search and $mod+d is the launcher: they used to be the
# same menu, so a binding that goes back to the launcher is a key that no longer
# does what the manual says. --search is a mode of bin/rofi-files itself, so both
# halves are asked for: without the binding the key is dead, without the mode it
# exits with a usage error and looks dead.
if grep -qs 'Super_L exec .*rofi-files --search' "$HOME/.config/sway/config"; then
	ok "the Search key opens the file search (rofi-files --search)"
else
	fail "the Search key does not run 'rofi-files --search' in ~/.config/sway/config (it opens the launcher again, or nothing)"
fi
grep -qs -- '--search)' "$HOME/.config/bin/rofi-files" ||
	fail "bin/rofi-files has no --search mode: the Search key exits with a usage error"

# The file search needs fd; without it rofi-files says so and browses $HOME instead,
# which looks like a search that found nothing.
if have fd || have fdfind; then
	ok "file search available (fd), which rofi-files --search runs"
else
	fail "fd is missing (package fd-find): rofi-files --search falls back to browsing"
fi

# Alt+Search asks the same question in a terminal: lazyos-find streams fd into
# fzf and hands the choice to lazyos-view. There are four separate ways for that
# key to look dead - no binding, a script without its executable bit, no fzf, no
# opener - and the fzf one is a FAIL rather than a warning because a missing
# filter is an *empty list*: the window opens, prints nothing, and reads as "no
# such file" instead of as a missing package.
if grep -qs 'Super_L+Mod1 exec .*lazyos-find' "$HOME/.config/sway/config"; then
	ok "Alt+Search opens the terminal file search (lazyos-find)"
else
	fail "Alt+Search does not run 'lazyos-find' in ~/.config/sway/config"
fi
if [ -x "$HOME/.config/bin/lazyos-find" ]; then
	ok "terminal file search (lazyos-find) is executable"
else
	fail "bin/lazyos-find is missing or not executable: Alt+Search opens a foot window that closes at once"
fi
if have fzf; then
	ok "fzf present, which the terminal file search filters with"
else
	fail "fzf is missing (package fzf): Alt+Search lists nothing"
fi
# Its notifications carry the search glyph, and a name mako cannot resolve draws
# no picture at all: the card is text-only with no log line.
[ -r /usr/share/icons/LazyOS/48x48/apps/system-search.png ] ||
	warn "the LazyOS theme has no system-search icon: lazyos-find's notifications are text-only"

# Search inside files (Alt+Ctrl+G) is the other half of the Search key: rofi-files
# finds a file by its name, rofi-grep by what is written in it. Two things take it
# away and neither says so - a missing binding (the key does nothing) or a missing
# ripgrep (the menu opens, refuses, and reads as "no matches").
if grep -rqs 'Ctrl+g exec .*rofi-grep' /etc/sway/config.d/; then
	ok "Alt+Ctrl+G searches inside the files (rofi-grep)"
else
	fail "no 'Ctrl+g exec ...rofi-grep' binding in /etc/sway/config.d/: the key does nothing"
fi
have rg && ok "ripgrep present, which the search inside files runs" ||
	fail "ripgrep is missing (package ripgrep): Alt+Ctrl+G finds nothing"

# Download (the root menu and the launcher) is yt-dlp plus the two things that
# make it useful: ffmpeg to merge a video stream with its audio, and node as the
# JavaScript runtime /etc/yt-dlp.conf names for YouTube's challenges. Without node
# yt-dlp still runs and warns that "some formats may be missing", which reads as a
# site problem instead of a missing package.
_missing_dl=""
for _c in yt-dlp ffmpeg node; do have "$_c" || _missing_dl="$_missing_dl $_c"; done
[ -x "$HOME/.config/bin/rofi-download" ] || _missing_dl="$_missing_dl bin/rofi-download"
if [ -z "$_missing_dl" ]; then
	ok "download complete (yt-dlp, ffmpeg, node, rofi-download)"
else
	fail "download incomplete:$_missing_dl (the row opens a menu that cannot fetch)"
fi
grep -qs 'js-runtimes node' /etc/yt-dlp.conf && ok "yt-dlp is told to use node (JS runtimes)" ||
	warn "/etc/yt-dlp.conf does not name node: yt-dlp logs 'JS runtimes: none' and YouTube may withhold formats"

# The home folders the manual and the menus name (Documents, Downloads, Music,
# Videos) are created by rootfs-setup.sh rather than shipped in the tree, because
# git cannot carry an empty directory. A missing Downloads only moves the download
# (rofi-download creates it), so this warns instead of failing.
_missing_home=""
for _d in Documents Downloads Music Videos; do
	[ -d "$HOME/$_d" ] || _missing_home="$_missing_home $_d"
done
[ -z "$_missing_home" ] && ok "the home folders are there (Documents, Downloads, Music, Videos)" ||
	warn "home folders missing:$_missing_home (rootfs-setup.sh creates them; a download recreates Downloads)"

# Alt+1 in the file browser is rofi's -kb-custom-1 and needs two halves: the flag
# in rofi-files and bin/rofi-file-actions behind it. Without the flag rofi exits 0
# and Enter's action runs instead - the row opens when the person asked to act on
# it - which is silent.
if grep -qs -- '-kb-custom-1 Alt+1' "$HOME/.config/bin/rofi-files"; then
	ok "the file browser passes Alt+1 through to rofi (-kb-custom-1)"
else
	warn "bin/rofi-files no longer passes -kb-custom-1 Alt+1: Alt+1 opens the row instead of acting on it"
fi
_missing_actions=""
for _c in tar unzip ffmpeg udisksctl; do have "$_c" || _missing_actions="$_missing_actions $_c"; done
[ -x "$HOME/.config/bin/rofi-file-actions" ] || _missing_actions="$_missing_actions bin/rofi-file-actions"
if [ -z "$_missing_actions" ]; then
	ok "the file actions have their tools (tar, unzip, ffmpeg, udisksctl)"
else
	fail "file actions incomplete:$_missing_actions (Alt+1 offers rows that cannot run)"
fi

# Format a disk writes a partition table and a filesystem, so both halves have to
# be there before the menu is allowed to list anything - and the menu is reached
# from Places, where a missing script is a row that does nothing.
_missing_fmt=""
for _c in parted mkfs.vfat mkfs.ext4; do have "$_c" || _missing_fmt="$_missing_fmt $_c"; done
[ -x "$HOME/.config/bin/rofi-format" ] || _missing_fmt="$_missing_fmt bin/rofi-format"
if [ -z "$_missing_fmt" ]; then
	ok "format a disk complete (parted, mkfs.vfat, mkfs.ext4, rofi-format)"
else
	fail "format a disk incomplete:$_missing_fmt (the Places row does nothing)"
fi

# The Learn menu's second row is the machine's own documentation: apropos searches
# the whatis database man-db builds, and man renders the page. Without man-db both
# are absent and the row reports a search that found nothing.
if have apropos && have man; then
	ok "manual pages searchable (apropos, man), which the Learn menu's second row opens"
else
	fail "man-db is missing (apropos/man): the Learn menu cannot show a program's own manual"
fi

# Password (the root menu) is openssl's random bytes, and a missing openssl is a
# row that answers "openssl is not installed" instead of a password.
have openssl && ok "openssl present, which the Password row generates with" ||
	fail "openssl is missing (package openssl): the Password row cannot generate anything"

# dig is the first step of every DNS problem in the manual, and there is no
# resolvectl here (the resolver is NetworkManager's, not systemd-resolved's), so
# without bind9-dnsutils that section begins with "command not found".
if have dig; then
	ok "dig present (bind9-dnsutils), which DNS troubleshooting asks for"
else
	fail "dig is missing (package bind9-dnsutils): 06-troubleshooting.md's DNS section cannot run"
fi

# The manual, the key view, dictation and hibernate. The first two are pure
# scripts (a missing one is invisible: the menu entry just does nothing); the
# last two depend on hardware and packages, so they warn instead of failing.
_learn_pages=$(ls /usr/share/doc/lazyos/*.md 2>/dev/null | wc -l)
if [ "$_learn_pages" -ge 6 ] && [ -x "$HOME/.config/bin/rofi-learn" ]; then
	ok "manual installed ($_learn_pages pages) and rofi-learn present"
else
	fail "manual incomplete: $_learn_pages page(s) in /usr/share/doc/lazyos, rofi-learn $( [ -x "$HOME/.config/bin/rofi-learn" ] && echo present || echo missing)"
fi
[ -x "$HOME/.config/bin/rofi-keys" ] && ok "keybindings view (rofi-keys) is executable" ||
	fail "bin/rofi-keys is missing: the key view has no script"
[ -x "$HOME/.config/bin/rofi-files" ] && ok "file browser (rofi-files) is executable" ||
	fail "bin/rofi-files is missing: Alt+Ctrl+F and the Files menu entry do nothing"

# Every menu behind the root menu is a rofi-* script. A missing one is the
# quietest failure in the session: the row is still listed (bar-menu draws it
# from a static list) and choosing it does nothing at all.
_missing_menus=""
for _s in rofi-quick rofi-notify rofi-clean rofi-kill rofi-tz rofi-places \
	rofi-media rofi-calc rofi-layout rofi-power rofi-about rofi-dns \
	rofi-pkg rofi-color rofi-remind lib-rofi.sh; do
	[ -r "$HOME/.config/bin/$_s" ] || _missing_menus="$_missing_menus bin/$_s"
done
if [ -z "$_missing_menus" ]; then
	ok "session menus complete (settings, notifications, free space, processes, time zone, places, media, calculator, layout, power, about, DNS, packages, colour, reminders)"
else
	fail "session menus missing:$_missing_menus (those root-menu rows do nothing)"
fi

# Presence is not enough: a script in bin/ without the execute bit makes its key
# do nothing at all, and nothing logs it (sway's `exec` gets EACCES and the line
# goes to the journal). Three menus of the batch that answered the card's report
# shipped that way - including this menu list, which passed because it asked for
# -r - so every file in bin/ is asked for -x here, once, instead of in each list
# above. lib-rofi.sh is sourced by the others, not run.
_noexec=""
for _s in "$HOME"/.config/bin/*; do
	[ -f "$_s" ] || continue
	case ${_s##*/} in lib-rofi.sh) continue ;; esac
	[ -x "$_s" ] || _noexec="$_noexec ${_s##*/}"
done
if [ -z "$_noexec" ]; then
	ok "every script in ~/.config/bin is executable"
else
	fail "not executable:$_noexec (the key or menu row that runs them does nothing)"
fi

# Reminders (Alt+Space > Reminders) are systemd transient timers, so what can
# silently take half the menu away is a missing systemd-analyze (every custom
# date is then refused while the five delays keep working) and a missing
# notification icon (the card has no picture and nothing logs it).
_reminder=""
have systemd-analyze || _reminder="$_reminder systemd-analyze"
have notify-send || _reminder="$_reminder notify-send"
have jq || _reminder="$_reminder jq"
[ -r /usr/share/icons/LazyOS/48x48/apps/preferences-system-time.png ] ||
	_reminder="$_reminder preferences-system-time.png"
[ -x "$HOME/.config/bin/rofi-remind" ] || _reminder="$_reminder bin/rofi-remind"
if [ -z "$_reminder" ]; then
	ok "reminders complete (systemd-analyze, notify-send, jq, the clock icon)"
else
	fail "reminders incomplete:$_reminder (custom dates or the notification icon disappear)"
fi

# A repeat is a calendar expression handed to the timer itself, so the parser that
# has to read it is worth one call: this is the exact expression the menu writes
# for "every weekday", and a systemd without calendar support is a menu that
# refuses every date.
if have systemd-analyze; then
	if LC_ALL=C systemd-analyze calendar --iterations=1 'Mon..Fri *-*-* 08:30:00' >/dev/null 2>&1; then
		ok "systemd parses the calendar expressions the reminder menu writes"
	else
		fail "systemd-analyze cannot read 'Mon..Fri *-*-* 08:30:00' (repeating reminders would be refused)"
	fi
fi

# 02-rofi.md sends a reader to the root menu for DNS, Packages, Colour, About and
# Reminders. A row that goes missing is a documented keypress that answers
# nothing - Colour was the one that had neither the row nor the `case` arm.
if [ -r "$HOME/.config/bin/bar-menu" ]; then
	_root_missing=
	for _m in rofi-dns rofi-pkg rofi-color rofi-about rofi-remind; do
		grep -qs "$_m" "$HOME/.config/bin/bar-menu" || _root_missing="$_root_missing ${_m#rofi-}"
	done
	[ -z "$_root_missing" ] &&
		ok "root menu reaches DNS, Packages, Colour, About and Reminders" ||
		fail "root menu has no entry for:$_root_missing (bar-menu)"
fi

# A row and its `case` arm are two halves of one entry; a row whose key has no
# arm closes the list and does nothing. Keys are the last word of a ui_row line,
# and the arms are indented inside the `case`. A row whose key is followed by a
# colour passes `"$UI_RED"` as its last word, so that one is read from the field
# before it - the earlier version took $NF always, which reported the Power row
# as dead, and matched arms only at column 0, which reported every row as dead.
if [ -r "$HOME/.config/bin/bar-menu" ]; then
	_dead_rows=$(awk '
		/^[[:space:]]*ui_row / {
			k = $NF
			if (k ~ /^\$|^"\$/) k = $(NF - 1)
			gsub(/"/, "", k)
			rows[k] = 1
		}
		/^[[:space:]]*[a-z][a-z-]*\)/ { k = $1; sub(/\).*/, "", k); arms[k] = 1 }
		END { for (k in rows) if (!(k in arms)) printf "%s ", k }' \
		"$HOME/.config/bin/bar-menu")
	[ -z "$_dead_rows" ] && ok "every root-menu row has an action" ||
		warn "root-menu rows with no action: $_dead_rows"
fi

# The key view draws one picture per section by name; a dropped flag makes the
# list silently text-only, the same class of bug as the file browser's.
if [ -r "$HOME/.config/bin/rofi-keys" ] &&
	grep -qs -- '-show-icons' "$HOME/.config/bin/rofi-keys" &&
	grep -qs -- '-icon-theme LazyOS' "$HOME/.config/bin/rofi-keys"; then
	ok "rofi-keys asks the theme for a picture per section (-show-icons, LazyOS)"
else
	warn "rofi-keys no longer passes -show-icons/-icon-theme LazyOS: the key view shows no pictures"
fi

# fastfetch exits when it has drawn, so without --hold the About window flashes
# and closes - which reads as "the row does nothing".
if [ -r "$HOME/.config/bin/rofi-about" ] && grep -qs -- '--hold' "$HOME/.config/bin/rofi-about"; then
	ok "rofi-about holds its fastfetch window open (--hold)"
else
	warn "rofi-about no longer passes --hold: the fastfetch picture flashes and closes"
fi

# The capture menu (Alt+Ctrl+C) is a list over external tools: grimshot for the
# screenshot, swappy to annotate, tesseract and zbarimg to read a region, and
# wf-recorder for the one action that outlives the menu. A missing binary is a
# row that does nothing when chosen, and a missing script is a key that does
# nothing at all, so both are FAILs.
_missing_capture=""
for _c in grimshot grim slurp tesseract zbarimg qrencode swappy wf-recorder; do
	have "$_c" || _missing_capture="$_missing_capture $_c"
done
for _s in rofi-capture rofi-qr screenshot; do
	[ -x "$HOME/.config/bin/$_s" ] || _missing_capture="$_missing_capture bin/$_s"
done
if [ -z "$_missing_capture" ]; then
	ok "capture menu complete (grimshot, swappy, tesseract, zbarimg, wf-recorder, qrencode)"
else
	fail "capture menu incomplete:$_missing_capture (Alt+Ctrl+C loses those rows)"
fi

# Screenshots land in ~/Pictures/Screenshots/<date>/, and grimshot only knows that
# when it is called through bin/screenshot: grimshot itself writes to $HOME on this
# image (there is no user-dirs.dirs), so one key that calls it directly drops the
# picture among the dotfiles again - which was the report.
grep -qs 'bin/screenshot' /etc/sway/config.d/90-screenshot-shortcuts.conf &&
	ok "the screenshot keys go through bin/screenshot" ||
	fail "90-screenshot-shortcuts.conf does not call bin/screenshot: the files land in \$HOME"
[ -z "$(grep -rs '^[[:space:]]*bindsym.*exec grimshot' /etc/sway/config.d/)" ] &&
	ok "no key calls grimshot directly" ||
	fail "a key calls grimshot directly: that screenshot lands in \$HOME"
grep -qs -- '"$dir/screenshot"' "$HOME/.config/bin/rofi-capture" &&
	ok "the capture menu saves through bin/screenshot" ||
	fail "rofi-capture calls grimshot directly: its screenshots land in \$HOME"
if [ -r /usr/share/applications/org.lazyos.screenshot-area.desktop ]; then
	grep -qs 'Exec=.*bin/screenshot' /usr/share/applications/org.lazyos.screenshot-area.desktop &&
		ok "the Screenshot (area) launcher entry uses bin/screenshot" ||
		fail "org.lazyos.screenshot-area.desktop still runs grimshot: that entry saves into \$HOME"
fi

# Clipboard history (Alt+Ctrl+V) and the emoji picker (Alt+Ctrl+E): each is a
# script over one tool and one file, and every half of it fails quietly. Without
# cliphist the list says so; without the watcher the list is simply empty and
# reads as "nothing was copied"; without the emoji table the picker refuses to
# open; without wtype the glyph reaches the clipboard but never the cursor.
_missing_clip=""
for _c in cliphist wl-paste wl-copy wtype; do
	have "$_c" || _missing_clip="$_missing_clip $_c"
done
[ -r "$HOME/.config/emojis.tsv" ] || _missing_clip="$_missing_clip .config/emojis.tsv"
for _s in rofi-emoji rofi-clip; do
	[ -x "$HOME/.config/bin/$_s" ] || _missing_clip="$_missing_clip bin/$_s"
done
if [ -z "$_missing_clip" ]; then
	ok "clipboard history and emoji picker complete (cliphist, wl-clipboard, wtype, emojis.tsv)"
else
	fail "clipboard/emoji incomplete:$_missing_clip (Alt+Ctrl+V and Alt+Ctrl+E fail or insert nothing)"
fi
# The watcher is the one part a binary cannot prove: it either is running or
# nothing is being stored.  `-fx` and not `-f`: pgrep -f matches this script's
# own command line, which contains the pattern, and would always answer "yes".
if pgrep -fx 'wl-paste --watch cliphist store' >/dev/null 2>&1; then
	ok "clipboard watcher running (wl-paste --watch cliphist store)"
else
	warn "the cliphist watcher is not running: the clipboard list stays empty (config.d/27-clipboard.conf starts it with the session)"
fi
# "Save the newest image" names the file after the type `file` reports, and
# without it the picture is written with no extension at all - the viewer then
# refuses it, which reads as a broken clipboard rather than a missing package.
if have file; then
	ok "file(1) is installed (a clipboard image is saved with its extension)"
else
	warn "file(1) is missing (package file): an image saved from the clipboard has no extension"
fi
# An inserted emoji needs a colour font; fc-match says what would draw it.
if have fc-match; then
	_emoji_font=$(fc-match -f '%{family}' 'emoji' 2>/dev/null || true)
	case ${_emoji_font:-} in
	*Emoji*) ok "a colour emoji font is installed ($_emoji_font)" ;;
	*) fail "no colour emoji font (fc-match emoji -> '${_emoji_font:-none}'): every inserted emoji renders as tofu" ;;
	esac
else
	skip "fc-match is not installed"
fi

# Tablet mode: the keyboard folded back. Four pieces have to be there for it to
# do anything - the switch, the script, the on-screen keyboard and the
# accelerometer - and each missing half is silent: no bindswitch means detaching
# the keyboard changes nothing, no wvkbd means the screen has no way to type, no
# accelerometer means the screen stays sideways. The accelerometer is read the
# same way the poller reads it (cros-ec-accel, in_accel_*_raw), so a kernel that
# renames or reorganises the devices is noticed here.
_tablet=""
have wvkbd-mobintl || _tablet="$_tablet wvkbd-mobintl"
[ -x "$HOME/.config/bin/tablet-mode" ] || _tablet="$_tablet bin/tablet-mode"
grep -rqs 'bindswitch tablet:' /etc/sway/config.d/ ||
	_tablet="$_tablet config.d/28-tablet.conf(bindswitch)"
if [ -z "$_tablet" ]; then
	ok "tablet mode wired (wvkbd-mobintl, tablet-mode, bindswitch tablet:on/off)"
else
	fail "tablet mode incomplete:$_tablet (detaching the keyboard does nothing, or the screen cannot type)"
fi
_accel=""
for _d in /sys/bus/iio/devices/iio:device*; do
	[ -r "$_d/name" ] || continue
	[ "$(cat "$_d/name" 2>/dev/null)" = cros-ec-accel ] || continue
	[ -r "$_d/in_accel_x_raw" ] && _accel=$_d && break
done
if [ -n "$_accel" ]; then
	ok "accelerometer present ($_accel, cros-ec-accel) for tablet rotation"
else
	fail "no cros-ec-accel with in_accel_*_raw: tablet mode starts but the screen never turns"
fi
# The two physical inputs are switched off with the mode, and the on-screen
# keyboard comes up first: a card whose keyboard was disabled before wvkbd proved
# alive would be a touchscreen with no way to type. The mode is the only thing in
# the session that disables a keyboard, so the pattern is a fair test of the wiring.
# The value is `$1` of inputs() and not the word `disabled`, so what is tested is
# the call (`inputs disabled`) and the order of the last one against `start_kbd`:
# `on` answers an already-running mode before it starts the keyboard, and that
# early `inputs disabled` is the one a plain line-order test would find.
_tm="$HOME/.config/bin/tablet-mode"
if grep -qs 'input type:keyboard events' "$_tm"; then
	_after_kbd=$(grep -n 'inputs disabled' "$_tm" | tail -1 | cut -d: -f1)
	_start_kbd=$(grep -n 'start_kbd ||' "$_tm" | head -1 | cut -d: -f1)
	if [ -n "$_after_kbd" ] && [ -n "$_start_kbd" ] && [ "$_after_kbd" -gt "$_start_kbd" ]; then
		ok "tablet mode switches the physical keyboard off (touchpad and keyboard, after wvkbd is up)"
	else
		fail "bin/tablet-mode disables the keyboard before wvkbd is confirmed alive: the touchscreen can be left unable to type"
	fi
else
	fail "bin/tablet-mode does not touch type:keyboard: the folded keys keep typing on the table"
fi
# The rotation reference is optional - the built-in -Y table is the default - but a
# file that does not parse is ignored in silence by orient(), which reads as "the
# rotation keeps turning back to the old way" after a calibration that claimed to work.
_calib_pose="${XDG_CONFIG_HOME:-$HOME/.config}/lazyos/tablet.conf"
if [ ! -f "$_calib_pose" ]; then
	skip "no rotation calibration ($_calib_pose): the built-in -Y reference is in use"
else
	_calib_line=$(sed -n '1s/[[:space:]]*$//;1s/^[[:space:]]*//;1p' "$_calib_pose" 2>/dev/null || true)
	case $_calib_line in
	normal=[xXyY][+-]) ok "rotation calibration is valid ($_calib_line in $_calib_pose)" ;;
	*) fail "rotation calibration is malformed ('${_calib_line:-empty}' in $_calib_pose): orient() ignores it and the screen turns the built-in way (delete the file to say so out loud)" ;;
	esac
fi
# The keyboard belongs to tablet mode and nowhere else. One that is already
# running while the mode is off is either a leftover from a session that was
# killed mid-mode or something starting it at login; either way it covers the
# bottom of the screen for no reason.
if [ ! -d "${XDG_RUNTIME_DIR:-/tmp}/lazyos-tablet" ]; then
	if pgrep -x wvkbd-mobintl >/dev/null 2>&1; then
		warn "wvkbd-mobintl is running outside tablet mode (config.d/28-tablet.conf starts it on the switch only)"
	fi
fi

# The manual is rendered with glow, and both the binary and the glamour style
# are needed: without the style glow prints an error and exits, and rofi-learn
# opens a foot window that closes immediately, which reads as "nothing happened".
_glow=""
have glow || _glow="$_glow glow"
[ -r "$HOME/.config/glow/solarized.json" ] || _glow="$_glow .config/glow/solarized.json"
[ -r "$HOME/.config/glow/glow.yml" ] || _glow="$_glow .config/glow/glow.yml"
if [ -z "$_glow" ]; then
	ok "manual renderer complete (glow with the solarized glamour style)"
else
	fail "manual renderer incomplete:$_glow (Learn opens a window that closes at once)"
fi
# And the style has to be readable by glamour: a colour written without its "#"
# is taken as a 256-colour index, so glow emits 38;5;839496, the terminal
# ignores it and the page falls back to its own colours. Nothing errors, which is
# why the first version of this style shipped 23 bare values and only looked
# wrong (inline code as a blank block). Cosmetic, so it warns.
if [ -r "$HOME/.config/glow/solarized.json" ] &&
	grep -qE '":[[:space:]]*"[0-9a-fA-F]{6}"' "$HOME/.config/glow/solarized.json"; then
	warn "glow style: a colour is missing its # (glamour reads it as a 256-colour index; the page loses its palette)"
fi

# The browser is the image's reader for what chafa cannot draw: a PDF, the HTML
# file, a URL. rofi-files and /usr/local/bin/xdg-open are the two things that
# start it; without it they answer with a notification instead, which is easy to
# miss.
if have firefox-esr; then
	ok "browser installed (firefox-esr), used by rofi-files and xdg-open"
else
	fail "firefox-esr is missing: Files cannot open an image or a PDF and xdg-open fails"
fi
if [ -x /usr/local/bin/xdg-open ]; then
	ok "xdg-open shim is installed (URLs and paths go to the browser)"
else
	fail "/usr/local/bin/xdg-open is missing: ctrl+click in foot and URL-printing tools fail"
fi
# Thumbnails in the file browser: rofi draws a row's picture through
# lazyos-thumbnail, which is ffmpeg for images and video and pdftocairo for a
# PDF's first page. A missing binary is silent - the row simply has no picture -
# so the two tools are a FAIL; the cache directory (rofi's own, under
# ~/.cache/thumbnails) only warns, because it is created on first use.
if have ffmpeg && have pdftocairo; then
	ok "thumbnails can be generated (ffmpeg for images/video, pdftocairo for PDF page 1)"
else
	fail "thumbnails cannot be generated:$(have ffmpeg || echo ' ffmpeg')$(have pdftocairo || echo ' pdftocairo') (Files rows lose their pictures)"
fi
if have chafa; then
	ok "picture viewer present (chafa draws sixels in foot)"
else
	fail "chafa is missing: Files cannot show an image without starting Firefox"
fi
# The three programs the file browser hands a file to, each with a package of its
# own: imv for a picture (chafa above is its fallback when the GPU gives it no
# EGL context), papers for a document, mpv for a container. Without one of them
# the branch falls through to the browser, which is 280 MB and cannot play a
# video at all - and nothing logs it, which is why this is a FAIL.
_viewers=""
for _c in imv papers mpv; do
	have "$_c" || _viewers="$_viewers $_c"
done
if [ -z "$_viewers" ]; then
	ok "viewers installed (imv for a picture, papers for a document, mpv for a video)"
else
	fail "missing viewer(s):$_viewers (that kind of file falls back to the browser)"
fi
# imv is not a package binary: Debian ships imv-wayland/imv-x11 and a
# /usr/libexec/imv/imv dispatcher, and rootfs-setup.sh puts that behind the short
# name. Without the symlink every `command -v imv` fails and a photo goes to
# chafa's sixels, which looks like "the new viewer does nothing".
if [ -x /usr/local/bin/imv ]; then
	ok "the imv short name resolves (/usr/local/bin/imv -> the package dispatcher)"
else
	fail "/usr/local/bin/imv is missing: rofi-files and lazyos-view cannot start the viewer"
fi
# mpv-mpris is a plugin file, not a program, so nothing else notices it: without
# it the film plays and playerctl, the Media menu and the bar see no player.
if [ -r /etc/mpv/scripts/mpris.so ]; then
	ok "mpv is an MPRIS player (mpv-mpris at /etc/mpv/scripts/mpris.so)"
else
	fail "mpv-mpris is missing: playerctl, the Media menu and the bar cannot see mpv"
fi
if [ -d "$HOME/.cache/thumbnails" ] && [ ! -w "$HOME/.cache/thumbnails" ]; then
	warn "$HOME/.cache/thumbnails is not writable: every picture is decoded again on every listing"
fi
# And the browser has to keep asking: an edit that drops the flag or the
# thumbnail:// suffix makes the list silently text-only.
if [ -r "$HOME/.config/bin/rofi-files" ] &&
	grep -qs -- '-show-icons' "$HOME/.config/bin/rofi-files" &&
	grep -qs 'thumbnail://' "$HOME/.config/bin/rofi-files"; then
	ok "rofi-files asks rofi for thumbnails (-show-icons and thumbnail://)"
else
	warn "rofi-files no longer passes -show-icons/thumbnail://: the file list shows no pictures"
fi

# A video is mpv's job: chafa only draws stills and answers a container with
# "unknown file format", which is the error the report carried. Without mpv the
# row falls back to Firefox (an .mkv then asks to be downloaded), so this is a
# FAIL - the row claims to play something.
if have mpv; then
	ok "media player installed (mpv), what rofi-files opens a video with"
else
	fail "mpv is missing: a video in Files falls back to Firefox (chafa cannot decode a container)"
fi
# A document is papers' job. Without it a PDF goes to Firefox, which is a
# 280 MB browser for a page, and an EPUB still needs it (papers does not read
# EPUB, whatever the comparison table once said).
if have papers; then
	ok "document reader installed (papers), what a double click on a PDF uses"
else
	warn "papers is missing: a PDF opens in Firefox instead of the reader (the fallback works, it is just the browser)"
fi

# Every row of the file browser carries an icon *name* the LazyOS theme has to
# resolve; a missing PNG is a blank 32 px column with no log line anywhere.
_missing_row_icons=""
for _i in folder go-up view-hidden dialog-warning text-x-generic text-html \
	application-pdf image-x-generic video-x-generic audio-x-generic \
	application-x-executable package-x-generic x-office-document; do
	[ -r "/usr/share/icons/LazyOS/48x48/apps/$_i.png" ] || _missing_row_icons="$_missing_row_icons $_i"
done
if [ -z "$_missing_row_icons" ]; then
	ok "file browser icons present (the LazyOS theme has every row name)"
else
	warn "LazyOS theme is missing row icons:$_missing_row_icons (those rows lose the icon column)"
fi

# The GTK3 half of the palette. One window in this image is GTK3 (pcmanfm), and
# without these two files it renders in Adwaita's light grey - a bright rectangle
# in a session whose every other pixel is base03. Both failures are silent: GTK
# drops a stylesheet rule it cannot parse with a line on stderr nobody reads, and
# an icon name the theme does not carry draws nothing at all.
if [ -r "$HOME/.config/gtk-3.0/gtk.css" ]; then
	ok "GTK3 windows use the solarized palette (gtk-3.0/gtk.css)"
else
	warn "$HOME/.config/gtk-3.0/gtk.css is missing: pcmanfm renders in Adwaita's light grey"
fi
if grep -qs '^gtk-icon-theme-name=LazyOS' "$HOME/.config/gtk-3.0/settings.ini" &&
	grep -qs '^gtk-application-prefer-dark-theme=1' "$HOME/.config/gtk-3.0/settings.ini"; then
	ok "GTK3 windows use the LazyOS icons on a dark base (gtk-3.0/settings.ini)"
else
	warn "gtk-3.0/settings.ini does not select LazyOS + dark: pcmanfm looks like a foreign program"
fi

# The windowed file manager is pcmanfm (the Files menu stays rofi-files, and the
# Search key finds files). What fails here does so silently: without its config
# the terminal command is x-terminal-emulator, which this image does not have, so
# "Open Terminal Here" does nothing; without the association a double click on a
# photo starts the browser (280 MB) or nothing; and libfm's own loader is the only
# thumbnail decoder, so a lost setting is invisible until a directory looks empty.
if have pcmanfm; then
	ok "file manager installed (pcmanfm $(dpkg-query -W -f='${Version}' pcmanfm 2>/dev/null))"
else
	fail "pcmanfm is missing: the Files (window) entry and 'Open containing folder' do nothing"
fi
# Mounting a USB stick needs udisks2: this session has no privilege for mount(8),
# and both consumers - pcmanfm's Devices pane and rofi-places' "Mount" rows - go
# through /usr/bin/udisksctl to udisksd on the system bus. The check is the client
# talking to the daemon (udisksctl status activates the D-Bus service), because
# the package can be installed with the service missing from the bus.
if have udisksctl; then
	if _udisks=$(udisksctl status 2>&1); then
		ok "removable media can be mounted (udisks2 answers on the system bus)"
	else
		fail "udisksctl cannot reach udisksd: $_udisks (Places cannot mount a stick)"
	fi
else
	fail "udisks2 is missing: an unprivileged session cannot mount a USB stick at all"
fi
_pcm_dirs=${XDG_CONFIG_DIRS:-/etc/xdg}
_pcm_conf=${_pcm_dirs%%:*}/pcmanfm/default/pcmanfm.conf
[ -r "$_pcm_conf" ] || _pcm_conf=/etc/xdg/pcmanfm/default/pcmanfm.conf
if grep -qs '^terminal=foot' "$_pcm_conf"; then
	ok "pcmanfm opens a terminal with foot (terminal=foot)"
else
	warn "no terminal=foot in $_pcm_conf: 'Open Terminal Here' (F4) starts nothing"
fi
if [ "$(grep -s '^show_thumbnail=' "$_pcm_conf" | cut -d= -f2)" = 0 ]; then
	warn "thumbnails are off in $_pcm_conf: the icon view shows no pictures"
fi
# The names it draws with come from the theme, not from the package: Adwaita 51
# ships no full-colour "folder" at all, so a PNG that did not make it into the
# image is a hole in the places sidebar and in the icon view.
_missing=
for _icon in folder user-home user-trash text-x-generic application-pdf \
	image-x-generic drive-harddisk view-refresh edit-copy folder-new; do
	[ -r /usr/share/icons/LazyOS/48x48/apps/"$_icon".png ] || _missing="$_missing $_icon"
done
if [ -z "$_missing" ]; then
	ok "the LazyOS theme carries every icon the file manager asks for"
else
	warn "the LazyOS theme has no icon for:$_missing (holes in pcmanfm's sidebar and icon view)"
fi
if [ -x "$HOME/.config/bin/lazyos-view" ]; then
	ok "open-with handler installed (lazyos-view: bat, imv, papers, mpv, firefox-esr)"
else
	fail "bin/lazyos-view is missing or not executable: a double click in pcmanfm opens nothing"
fi
# The launcher entry is a shadow of the package's (same file name, so it replaces
# Debian's "PCMan File Manager"): without it the launcher shows "Files" (the
# menu) next to "PCMan File Manager", which reads as two copies of one thing.
if grep -qs '^Name=Files (window)' "$HOME/.local/share/applications/pcmanfm.desktop"; then
	ok "the launcher names the window 'Files (window)'"
else
	warn "$HOME/.local/share/applications/pcmanfm.desktop is missing: the launcher shows 'PCMan File Manager' next to 'Files'"
fi
# `gio mime` resolves the association exactly like the application does, and it
# is also a test of the .desktop entry itself: GIO drops an entry whose Exec
# program does not exist, so a lazyos-view without its executable bit reads as
# "no default application" instead of as an error. LC_ALL=C so the label is the
# one this greps for, not a translation.
#
# The image ships `gio` (libglib2.0-bin) so this is the real question, but the
# file is the answer that does not depend on a package: the fallback used to
# read `[Default Applications]` and then compare it with gio's own wording, so
# every association warned even when the file was right. Both shapes now feed
# the same variable: the desktop id, nothing else.
_mime_default() { # MIME-TYPE
	if have gio; then
		# gio's line is `Default application for “image/png”: org.lazyos.view.desktop`
		# (typographic quotes, and the type before the colon), or
		# `No default applications for “...”` when there is none - the field
		# after the last `: ` is the answer in the first shape and absent in
		# the second, which is why the old `^Default application: ` prefix
		# matched nothing at all.
		LC_ALL=C gio mime "$1" 2>/dev/null |
			awk 'NR == 1 && /: / { sub(/^.*: /, ""); print; exit }'
	else
		sed -n "s|^$1=||p" "${_pcm_dirs%%:*}/mimeapps.list" 2>/dev/null | head -1
	fi
}
_mime=$(_mime_default image/png)
if [ "$_mime" = org.lazyos.view.desktop ]; then
	ok "a double click on an image reaches lazyos-view (mimeapps.list)"
else
	warn "image/png defaults to '${_mime:-(nothing)}', not org.lazyos.view.desktop: a photo opens in the browser or nowhere"
fi
# A document is papers' job, and the association is the half a package cannot
# carry: papers ships org.gnome.Papers.desktop, and this file is what points
# application/pdf at it. GIO drops an entry whose Exec does not exist, so the
# same question also proves the .desktop is installed.
_mime_pdf=$(_mime_default application/pdf)
if [ "$_mime_pdf" = org.gnome.Papers.desktop ]; then
	ok "a double click on a PDF reaches papers (mimeapps.list)"
else
	warn "application/pdf defaults to '${_mime_pdf:-(nothing)}', not org.gnome.Papers.desktop: a document opens in the browser"
fi
_dir=$(_mime_default inode/directory)
if [ "$_dir" = pcmanfm.desktop ]; then
	ok "'Open containing folder' opens pcmanfm (inode/directory)"
else
	warn "inode/directory defaults to '${_dir:-(nothing)}', not pcmanfm.desktop: 'Open containing folder' does nothing"
fi

# Dictation: the shortcut records with parecord and transcribes with whisper-cli.
# The model is fetched on first use, so its absence is not a failure.
if have whisper-cli && have wtype && have parecord; then
	ok "dictation tools present (whisper-cli, wtype, parecord)"
	have ffmpeg && ok "dictation normalises the level (ffmpeg dynaudnorm)" ||
		warn "ffmpeg is missing: the internal microphone records quietly and the wav is transcribed raw"
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
	skip "no disk swap: hibernation needs a swap labelled lazyos_swap (see sway-hibernate)"
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
# list simply has fewer rows and nobody notices. The idle screen's entry is
# checked apart because it is the one the brand owns: its name is the mark's.
_missing_desktop=""
for _d in wifi wifi-nmtui bluetooth audio-output power lock suspend \
	screenshot-area capture lazygit tmux files emoji clipboard about; do
	[ -r "/usr/share/applications/org.lazyos.$_d.desktop" ] || _missing_desktop="$_missing_desktop $_d"
done
[ -r /usr/share/applications/org.lazyos.screensaver.desktop ] ||
	_missing_desktop="$_missing_desktop lazyos-screensaver"
if [ -z "$_missing_desktop" ]; then
	ok "launcher entries present (session actions, nmtui, lazygit, tmux, about)"
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
section "The brand"
# ---------------------------------------------------------------------------
# /etc/os-release is Debian's symlink to /usr/lib/os-release, and base-files
# owns that file: a base-files upgrade can put the symlink back and the image
# then silently calls itself Debian.  The build writes a regular file; this is
# what notices when it is gone.
if [ -r /etc/os-release ]; then
	_os_id=$( (. /etc/os-release 2>/dev/null; printf '%s' "${ID:-}") )
	_os_name=$( (. /etc/os-release 2>/dev/null; printf '%s' "${PRETTY_NAME:-}") )
	_os_ver=$( (. /etc/os-release 2>/dev/null; printf '%s' "${VERSION_ID:-}") )
	if [ "$_os_id" = lazyos ] && [[ $_os_name == LazyOS* ]]; then
		ok "os-release says $_os_name (ID=$_os_id)"
	else
		fail "/etc/os-release is not LazyOS (ID='${_os_id}', PRETTY_NAME='${_os_name}'): base-files may have restored its symlink"
	fi
	# The image's own record of what it was built from.  A hand edit of
	# os-release is allowed; saying a different version than the build did is
	# what this catches.
	_rev_ver=$(sed -n 's/^version //p' /etc/debian-aarch64-revision 2>/dev/null)
	if [ -n "$_rev_ver" ] && [ -n "$_os_ver" ] && [ "$_rev_ver" != "$_os_ver" ]; then
		warn "os-release says VERSION_ID=$_os_ver, but the image was built from version $_rev_ver"
	fi
else
	fail "/etc/os-release is missing"
fi

# The wallpaper is the brand's largest surface, and a missing file leaves sway
# painting a flat colour with no message.  The lock screen deliberately keeps
# the mountain photograph: it is the surface that is up when nobody is looking.
if [ -r "$HOME/Pictures/Wallpapers/lazyos.png" ]; then
	ok "brand wallpaper present"
else
	fail "missing Pictures/Wallpapers/lazyos.png: the desktop falls back to a flat colour"
fi
if grep -qs 'Pictures/Wallpapers/lazyos.png' /etc/sway/config "$HOME/.config/sway/config" 2>/dev/null; then
	ok "sway paints the brand wallpaper"
else
	fail "sway does not name Pictures/Wallpapers/lazyos.png (output * bg)"
fi
if grep -qs 'mountains.jpg' "$HOME/.config/bin/sway-lock" 2>/dev/null; then
	ok "lock screen keeps the mountain wallpaper"
else
	warn "sway-lock no longer names mountains.jpg: the lock is a different surface from the desktop"
fi

# The mark itself: the About entry asks for Icon=lazyos, and a theme without it
# draws a hole where a picture should be, quietly.
_missing_mark=""
for _f in 48x48/apps/lazyos.png scalable/apps/lazyos.svg; do
	[ -r "/usr/share/icons/LazyOS/$_f" ] || _missing_mark="$_missing_mark $_f"
done
if [ -z "$_missing_mark" ]; then
	ok "the theme carries the mark (lazyos.png, lazyos.svg)"
else
	fail "the theme is missing the mark:$_missing_mark (About LazyOS shows a hole)"
fi

# The card the launcher's About row opens, and the logo fastfetch draws there.
if [ -r "$HOME/.config/fastfetch/config.jsonc" ]; then
	ok "fastfetch config present"
else
	warn "no ~/.config/fastfetch/config.jsonc: About shows Debian's swirl"
fi
if [ -r "$HOME/.config/lazyos/wordmark.txt" ]; then
	ok "fastfetch wordmark present"
else
	warn "no ~/.config/lazyos/wordmark.txt: fastfetch's logo has nothing to draw"
fi

# The greeter and the virtual terminals are the first and the cheapest surfaces
# a person sees, and both are files a later edit can quietly lose.
grep -qs 'lazy by design' /etc/greetd/config.toml 2>/dev/null &&
	ok "the greeter greets with lazy by design" ||
	warn "the greeter no longer greets with lazy by design"
grep -qs 'LazyOS' /etc/issue 2>/dev/null &&
	ok "virtual terminals greet LazyOS" ||
	warn "/etc/issue does not name LazyOS"

# The rename was a clean break, so a surviving old name is a reference that was
# missed.  Scoped to the shipped surfaces on purpose: ~/Dev/LazyOS carries the
# frozen release notes of the past, and reports/ is hardware evidence.
_stale=$(ls /usr/share/applications/org.nureb.*.desktop 2>/dev/null || true)
if [ -n "$_stale" ]; then
	fail "old desktop entries survive: $(printf '%s ' $_stale)"
else
	ok "no org.nureb.* entries survive"
fi
_stale=$(grep -rlis 'nureb' /usr/share/applications /usr/share/icons/LazyOS \
	/usr/share/doc/lazyos "$HOME/.config/bin" 2>/dev/null || true)
if [ -n "$_stale" ]; then
	warn "the old name still appears in: $(printf '%s\n' "$_stale" | sed -n '1,3p' | tr '\n' ' ')"
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

# The menus draw a Font Awesome codepoint from the alphabet in lib-rofi.sh, and a
# name is only a picture while some installed font still carries that codepoint:
# a name nobody renders draws nothing at all (a hole on the row, a text-only
# card) and logs nothing.  One fontconfig call answers for every glyph at once
# (the union of the charsets installed fonts cover), because a call per glyph
# cost two seconds of the report on a 4 GB device.
_alphabet="$HOME/.config/bin/lib-rofi.sh"
if have fc-list && [ -r "$_alphabet" ]; then
	_glyphs="$(sed -n 's/^\(UI_I_[A-Z_]*\)=.*#[[:space:]]*\([0-9a-f][0-9a-f]*\)$/\1 \2/p' "$_alphabet")"
	_glyph_total="$(printf '%s\n' "$_glyphs" | awk 'NF { n++ } END { print n + 0 }')"
	if [ "$_glyph_total" -eq 0 ]; then
		fail "no UI_I_* glyphs parsed from $_alphabet (the menus would draw empty spans)"
	else
		_missing_glyph="$(fc-list -f '%{charset}\n' 2>/dev/null | tr ' ' '\n' | sort -u |
			awk -v glyphs="$_glyphs" '
			function dec(h,   i, v) {
				v = 0
				for (i = 1; i <= length(h); i++)
					v = v * 16 + index("0123456789abcdef", substr(h, i, 1)) - 1
				return v
			}
			BEGIN {
				n = split(glyphs, line, "\n")
				for (i = 1; i <= n; i++) {
					if (line[i] == "") continue
					split(line[i], f, " ")
					want[i] = dec(f[2])
					name[i] = f[1]
					hex[i] = f[2]
				}
			}
			{
				lo = $1
				if (index(lo, "-")) {
					split(lo, r, "-")
					a = dec(r[1])
					b = dec(r[2])
				} else {
					a = b = dec(lo)
				}
				for (i = 1; i <= n; i++)
					if (!seen[i] && want[i] >= a && want[i] <= b) seen[i] = 1
			}
			END {
				for (i = 1; i <= n; i++)
					if (want[i] && !seen[i]) out = out " " name[i] "(U+" hex[i] ")"
				print out
			}')"
		if [ -z "$_missing_glyph" ]; then
			ok "menu alphabet renders ($_glyph_total glyphs, every codepoint covered by a font)"
		else
			fail "no installed font renders:$_missing_glyph (a hole on every row that uses it, and nothing logs it)"
		fi
	fi
fi

# The launcher shows application *images*, and Debian's Adwaita has none of this
# session's names, so the image ships the small LazyOS theme (48x48 PNGs rendered
# from the Font Awesome codepoints). Without it every org.lazyos.* row is a hole.
_icons=""
[ -d /usr/share/icons/LazyOS ] || _icons="$_icons /usr/share/icons/LazyOS"
[ -r /usr/share/icons/LazyOS/index.theme ] || _icons="$_icons LazyOS/index.theme"
_iconless=""
# The session's own entries, plus the viewer packages': their Icon= is not their
# binary name (imv asks for multimedia-photo-viewer, papers for org.gnome.Papers)
# and Adwaita 51 has neither, so the same hole appears on those rows.
for _d in /usr/share/applications/org.lazyos.*.desktop \
	/usr/share/applications/org.gnome.Papers.desktop \
	/usr/share/applications/imv.desktop \
	/usr/share/applications/mpv.desktop; do
	[ -r "$_d" ] || continue
	_icon=""
	while IFS= read -r _line; do
		case $_line in Icon=*) _icon=${_line#Icon=}; break ;; esac
	done <"$_d"
	[ -n "$_icon" ] || continue
	[ -r "/usr/share/icons/LazyOS/48x48/apps/$_icon.png" ] && continue
	# An application's own icon (hicolor, pixmaps) is fine too.
	[ -n "$(find /usr/share/icons /usr/share/pixmaps -name "$_icon.*" 2>/dev/null | head -1)" ] && continue
	_iconless="$_iconless $(basename "$_d" .desktop)=$_icon"
done
if [ -z "$_icons" ] && [ -z "$_iconless" ]; then
	ok "launcher icons complete (LazyOS theme, every session entry resolves)"
elif [ -z "$_icons" ]; then
	fail "launcher entries without an icon:$_iconless (a hole in the launcher row)"
else
	fail "icon theme incomplete:$_icons (the launcher shows empty spaces)"
fi
# rofi's run mode (Alt+Shift+D) asks the icon theme for an icon *named after the
# binary* it would run, not for a .desktop icon (source/modes/run.c), so the
# command palette only has icons because make-icons.sh renders them under those
# names too.  Missing ones are a blank 26 px column, not an error.
_runless=""
for _c in foot btop htop lazygit tmux nvim fzf rg bat eza mpv imv firefox-esr; do
	[ -r "/usr/share/icons/LazyOS/48x48/apps/$_c.png" ] || _runless="$_runless $_c"
done
if [ -z "$_runless" ]; then
	ok "command palette icons present (the run mode resolves binaries by name)"
else
	warn "command palette icons missing for:$_runless (blank column, not a failure)"
fi

if have grep; then
	_cfg_rasi="$HOME/.config/rofi/config.rasi"
	if [ -r "$_cfg_rasi" ]; then
		_show=$(grep -c 'show-icons: *true' "$_cfg_rasi" 2>/dev/null || true)
		_theme=$(grep -c 'icon-theme: *"LazyOS"' "$_cfg_rasi" 2>/dev/null || true)
		if [ "${_show:-0}" -ge 1 ] && [ "${_theme:-0}" -ge 1 ]; then
			ok "rofi shows icons from the LazyOS theme (show-icons, icon-theme)"
		else
			warn "rofi/config.rasi does not enable show-icons and icon-theme \"LazyOS\""
		fi
	fi
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

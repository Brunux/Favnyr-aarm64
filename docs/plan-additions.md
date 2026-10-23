# Plan: what to add next (October 2026)

An audit of what the session ships today, and a ranked list of additions that would improve what a
person *sees*, what the machine *costs* and what a person can *do* - written against the rules this
repository already keeps (icons first, nothing resident, three witnesses per feature).

Unlike `omarchy-gap.md`, this is not a parity list. It starts from the booted image (LazyOS 0.4.0 on
the Hayato card) and from what its own configuration says, so everything here is either a bug the
device shows today or a cheap addition at a place the tree already has.

The checkout this was written in has **uncommitted work in progress** (workspace icons through
`workstyle`, OpenCode shipped in the image, the `tools/` layout and its `build_deb` helper). Where
that work changes a conclusion here, the item says so in place; everything else describes the
committed 0.4.0.

## How this was checked

* The shipped session was inspected **on the device**, from inside a running sway: `ps`, `systemctl`,
  `swaymsg`, `busctl`, `apt-cache`, the configuration files, and a **headless sway** (`WLR_BACKENDS=
  headless`, its own `XDG_RUNTIME_DIR`) for the GUI experiments, so `papers` never opened in the
  maintainer's session. One `foot` window used for the idle-inhibit test did surface there - foot's
  client/server hands a new client the running server's Wayland connection - and it was closed
  afterwards; everything else that opened was on the headless display.
* Claims marked **verified** were reproduced; claims marked *to confirm* are the one step that needs
  the panel (DPMS wakes on input) or a package that is not installed.
* Package sizes are `Installed-Size` from forky, as `apt-cache show` reports them.

## The short list

| # | Change | Surface | Cost | Value |
|---|---|---|---|---|
| 1 | The screen stays on after the lock | `23-lazyos-screensaver.conf` | 3 lines, no package | **Battery**: the panel is the biggest idle load |
| 2 | Volume/brightness OSD as a mako card with its progress bar | new `bin/osd` + 3 icons | no package, no process | **Looks/UX**: the keys stop being invisible |
| 3 | GTK4/libadwaita apps paint themselves white | `~/.config/gtk-4.0/` | 2 files, no package | **Looks**: `papers` is the one window that is not the session |
| 4 | `ip`, `ping`, `ss`, `zip` are not installed | package lists | 8.8 MB | **Tools**: the first commands a user types |
| 5 | Fullscreen video in Firefox/papers can still lock the screen | 2 `for_window` lines | 2 lines, no package | **UX**: a film is not interrupted |
| 6 | Build-time font and icon caches | `rootfs-setup.sh` | 2 commands | **First-launch latency** |
| 7 | `pavucontrol` is installed and has no door | `bar-audio` | 1 row | **UX**: per-app volume |
| 8 | `check-system.sh` does not check the resident set | `check-system.sh` | one section | **Tooling**: enforces the repo's own rule |
| 9 | Non-Latin fonts | `fonts-noto-core` | 42.5 MB | **Coverage** |
| 10 | `sshd` is enabled on every network, with the documented default password | policy | a decision | **Security** |

Items 1-6 are the ones I would do in this order. 7-9 are an afternoon. 10 is a question, not a
patch (see the end).

---

## 1. The screen stays on after the lock

**Today** (verified: `pgrep -af swayidle` on the card):

```
swayidle -w timeout 150 lazyos-screensaver timeout 300 sway-lock
            before-sleep sway-lock --wait after-resume pkill …; night-light resume
```

The chain stops at the lock. After 300 s the machine shows a *lit* lock screen for as long as it is
left alone: the panel backlight is never turned off, which on a 1920x1200 IPS panel is the largest
idle consumer on the device, and a static image held for hours is also the wrong thing to leave on
an LCD.

**Fix** - one more timeout, and the wake half that makes it safe:

```sh
exec swayidle -w \
	timeout 150 '…/lazyos-screensaver' \
	timeout 300 '…/sway-lock' \
	timeout 360 'swaymsg "output * power off"' \
	resume 'swaymsg "output * power on"' \
	before-sleep '…/sway-lock --wait' \
	after-resume 'pkill -f "[o]rg.lazyos.screensaver"; swaymsg "output * power on"; …/night-light resume'
```

Three facts make this correct, and they are the reason the command lives here and not in a binding:

* `output * power` is the current spelling; `dpms` is a deprecated alias (`sway-output(5)`).
* swayidle's `resume` command (bound to the `timeout` it follows) runs on the idle notification's
  *resumed* event, i.e. the first activity after the timeout, so a key press turns the panel back
  on. A panel turned off by hand does not wake on input (upstream sway#2910: "outputs are turned
  back on only if swayidle turned them off"), which is why the command lives inside the swayidle
  chain and not in a keybinding. `after-resume` is the logind half (a real suspend/resume), and it
  gets its own `power on` for the case where the machine sleeps from the lock screen.
  *To confirm on the panel: press a key after 6 minutes idle.*
* Keys pressed while the panel is off can repeat once on wake (sway#7112, open upstream); that is
  cosmetic here because the lock is already up, but it is why the timeout is after the lock and not
  before it.

**Witnesses**: `check-system.sh` FAILs when the drop-in has no `power off` timeout or no `resume`;
`05-system.md` gets § *After the lock* (what the panel does, and why a key brings it back);
`AGENTS.md` gets the battery bullet with the sway#2910 quote, because "why is it not a binding" is
the question the next reader will have.

## 2. An OSD for volume and brightness

**Today** (verified: `20-brightness.conf`, `21-volume.conf`): the top-row keys run `pactl` and
`brightnessctl` and print nothing. The only feedback is the 12 px number on the bar - and when the
bar is covered by a fullscreen window there is no feedback at all. Every other desktop shows an
overlay for this; `omarchy-gap.md` declined one because `swayosd` is a daemon (it was also described
as unpackaged, which is no longer true - see the corrections), but the overlay does not need one.

**The mechanism already exists in the theme.** `~/.config/mako/config` carries
`progress-color=over #2aa198ff`, and `mako(5)` says: *"Progress can be indicated in a notification
by setting a hint, 'value' to an integer between 0 and 100"* - so a card with a `value` hint draws
exactly the solarized progress bar the busy card already uses for tasks.

**Fix** - one helper (`~/.config/bin/osd`), one implementation per action, the same shape as
`night-light`:

```sh
osd volume up|down|mute      # pactl + the card
osd brightness up|down       # brightnessctl + the card
```

```sh
notify-send -a LazyOS -i audio-volume-high -u low -t 1200 \
	-h string:x-canonical-private-synchronous:lazyos-osd \
	-h int:value:45 'Volume' '45%'
```

* The synchronous tag makes successive key presses **replace** the card instead of stacking, which
  is the whole point of an OSD.
* The icon follows the level (`audio-volume-low/medium/high/muted`), and brightness reuses
  `display-brightness`, which the theme already draws; the three volume names are new rows in
  **table 3** of `scripts/make-icons.sh` (the names the session asks for by freedesktop name), and
  their PNGs are regenerated on the host and committed - the icon rule, not an exception to it.
* The bindings stay `--locked` (they work on the lock screen) and call the wrapper instead of
  `pactl`/`brightnessctl`; the bar's own `on-scroll-*` keeps calling the tools directly, because the
  bar already shows the number and a toast for a scroll would be noise.

**Witnesses**: `check-system.sh` FAILs when the wrapper is missing or not executable, when a key
binding does not call it, and WARNs when the three volume icon names (plus `display-brightness`) are not in the theme;
`03-shortcuts.md` § *Volume and brightness*; `AGENTS.md` under the bar/notifications bullet.

## 3. GTK4 apps paint themselves white

**Today** (verified): the session ships `~/.config/gtk-3.0/{settings.ini,gtk.css}` and nothing for
GTK4. `papers` is GTK4 + libadwaita, and on the card it opens as a **white window**: no Settings
portal exists (`busctl` on the session bus lists only `org.freedesktop.impl.portal.desktop.wlr`,
which implements Screenshot and ScreenCast, not Settings), so libadwaita has no colour-scheme
source, and without `~/.config/gtk-4.0/settings.ini` it falls back to the light default. `papers`
says it itself:

```
Gdk-WARNING: Cannot get portal org.freedesktop.portal.Settings version: … No such interface
Gdk-WARNING: Cannot get portal org.freedesktop.portal.Inhibit version: … No such interface
```

**Fix** (verified: screenshot before and after in a headless sway; the second is solarized dark):

* `~/.config/gtk-4.0/settings.ini` - `[Settings] gtk-application-prefer-dark-theme=1`
* `~/.config/gtk-4.0/gtk.css` - the same palette as `gtk-3.0/gtk.css`, but through libadwaita's
  named colours (`window_bg_color`, `view_bg_color`, `headerbar_*`, `card_*`, `dialog_*`,
  `popover_*`, `sidebar_*`, `accent_*`), which is the supported way to restyle an Adwaita app.

GTK4 reads `settings.ini` (the string is in `libgtk-4.so.1`) and loads the user CSS after
libadwaita's own sheet, and the pair was verified to work: with `XDG_CONFIG_HOME` pointing at those
two files, `papers` came up solarized dark in the headless sway; without them, white. No package, no
portal and no process is involved.

**Alternative, not the default**: `xdg-desktop-portal-gtk` (776 kB) implements Settings *and*
Inhibit, which would also give GTK apps a way to ask the session not to idle (presentations, video).
It is D-Bus activated but stays resident once used, which is why the static files come first and the
portal is the item to add only if more GTK4 programs arrive.

**Witnesses**: `check-system.sh` FAILs when the two files are missing and WARNs when `papers` is
installed and the Settings portal is absent with no `gtk-4.0` config (the exact white-window state);
`04-software.md` § *Why a GTK4 window looks like the session*; `AGENTS.md` under the GTK3 bullet.

## 4. `ip`, `ping`, `ss` and `zip` are not installed

**Today** (verified on the card; none of these is in any list, so a fresh image has the same hole):

| command | package | installed size | why it matters |
|---|---|---|---|
| `ip`, `ss` | `iproute2` | 4.8 MB | addresses, routes, `ss -ltn` ("what is listening") |
| `ping` | `iputils-ping` | 245 kB | the first question about any network |
| `zip` | `zip` | 803 kB | `unzip` is installed, `zip` is not: archives can be read and not made |
| `strace` | `strace` | 2.9 MB | "why did this program fail" without a GUI |

The maintainer's own report asked "do we have all basic gnu tools?" and `dig` was added the same
day. These are the rest of the answer: `NetworkManager` links libnl rather than using `iproute2`,
so nothing pulls the tools in, and the session can connect a network but cannot inspect it from a
terminal.

**Proposal**: `iproute2` + `iputils-ping` in `packages/base.list` (the system needs them the way it
needs `iw` and `bind9-dnsutils`), `zip` + `strace` in `packages/tools.list` (what a person reaches
for), `ncdu` later with the disk work already planned in `plan-rofi.md`.

**Witnesses**: `check-system.sh` FAILs when `ip`, `ping` and `ss` are missing (the wireless checks
already assume `iw`); `06-troubleshooting.md` § *The network* gets the `ip a` / `ss -ltn` /
`ping -c3` sequence; `AGENTS.md` in the "quality of life" paragraph.

## 5. A fullscreen video can still be interrupted by the lock

**Today** (verified): mpv already asks the compositor not to idle - `--stop-screensaver` defaults to
`yes` and the binary carries `zwp_idle_inhibitor_v1` - so a film in mpv is safe. Firefox and
`papers` do not have that path here (Firefox uses the portal's Inhibit, which this image does not
provide; `papers` uses it for presentations), so a long video in a tab is interrupted at 150 s and
locked at 300 s.

**Fix** - the compositor can hold the inhibitor itself, per window, with no client support:

```sh
for_window [app_id="(?i)^firefox"] inhibit_idle fullscreen
for_window [app_id="org.gnome.Papers"] inhibit_idle fullscreen
```

**Verified**: with a window focused, `swaymsg inhibit_idle visible` sets
`idle_inhibitors.user=visible` and a 2 s `swayidle` timeout **does not fire**; with the inhibitor
unset the same timeout fires. `fullscreen` is the right scope for a browser (a Firefox window merely
open would otherwise keep the machine awake forever) and exactly matches a video or a presentation.

**Second half, optional**: a `"idle_inhibitor"` module on the bar (coffee-cup glyph, `f0f4`) and a
*Keep awake* row in the root menu, for reading or a long download. `waybar 0.15` ships the module
and it toggles the same `swaymsg inhibit_idle` (verified: `swaymsg inhibit_idle none` is accepted by
swayfx 0.6). It is one more card on the bar only while it is on.

**Witnesses**: `check-system.sh` FAILs when the two `for_window` rules are gone (the failure is
silent: a video is interrupted and nothing says why) and WARNs when the inhibit module is missing
from `config.jsonc`; `05-system.md` § *When the screen must not sleep*; `AGENTS.md` next to the idle
chain bullet.

## 6. Build-time font and icon caches

**Today** (verified on the card): `/var/cache/fontconfig` does not exist and
`/usr/share/icons/LazyOS/icon-theme.cache` is absent. Every first font lookup and the first icon
theme scan therefore walks the trees - the first terminal, the first `rofi -show drun`, the first
`papers` window. It is a one-time cost per boot for the user cache and a one-time cost per session
for the icon theme, small but free to remove.

**Fix** at the end of `rootfs-setup.sh` (the chroot is already there, and the theme is committed):

```sh
fc-cache -f -s                                   # /var/cache/fontconfig, system-wide
gtk-update-icon-cache -q -t -f /usr/share/icons/LazyOS
```

`gtk-update-icon-cache` is already on the build host (verified) and `fontconfig` is already a
dependency of the session; `00-deps.sh` needs the Debian name only if the host lacks it
(`gtk-update-icon-cache` ships in `gtk-update-icon-cache`). The second line is the one
`omarchy-gap.md` § 2 already predicted ("A future make-icons.sh could build the cache"); doing it
in `rootfs-setup.sh` keeps `make-icons.sh` a host tool with no chroot.

**Witnesses**: `check-system.sh` WARNs when the theme cache is missing (a rebuilt theme without the
cache is the silent regression); `05-system.md` § *The first launch*; `AGENTS.md` in the icon-theme
paragraph.

## 7. `pavucontrol` has no door

**Verified**: `pavucontrol` is in `uis/sway/packages.list` (line 221) and nothing in
`~/.config/bin` or the `.desktop` entries starts it. `bar-audio` switches sinks and mutes the
speakers and the microphone; it cannot answer "which application is too loud", which is the one
thing `pavucontrol` is for.

**Fix**: one row in `bar-audio` (*Volume per application*, `exec pavucontrol`) and the same row in
`rofi-quick` beside *Audio output*. No package, no process beyond the window; GTK3, so it already
matches the theme (and with item 3, so does everything else).

## 8. `check-system.sh` cannot see a stray daemon

The repository's strongest rule is "nothing resident", and the check does not enforce it: the only
process check is `wvkbd` outside tablet mode. The checkout is in the middle of admitting its
**second** approved resident process (`workstyle`, for workspace icons; `cliphist`'s watcher is the
first), which is exactly when the rule needs a test rather than a paragraph: the allowlist is the
agreement, and a process that is not in it is the thing a report should show.

**Fix** - a *Session* section with three cheap questions:

* **The declined daemons, by process name** (`pgrep -x` for `udiskie`, `swaync`, `dunst`, `picom`,
  `blueman-applet`, `packagekitd`, `localsearch-3`, `tracker-miner-fs-3`): a FAIL, because each one
  is a package the comments and `omarchy-gap.md` § 3 have already declined and a running one means
  the rule was broken silently.
* **The two approved residents**: `pgrep -x workstyle` and `pgrep -f "[w]l-paste --watch cliphist"`
  are expected, so their *absence* is a WARN - the workspace icons or the clipboard history are gone
  and nothing else in the report says so. (`workstyle` already has its own check in the work in
  progress; the clipboard watcher does not.)
* **User units**: `systemctl --user list-units --type=service --state=running --no-legend` against a
  short allowlist (`pipewire`, `wireplumber`, `wireplumber@*` are the session's own). An unknown unit
  is a WARN, because a distribution update can add one and the report should show it.

Note that neither approved resident is a *unit*: both are `exec`'d by the sway configuration, which
is why the process list is the place to look and the unit list is only the second question.

That is a check, not a feature: no package, no process, and it turns a documented intention into a
test - the same move the rest of the script makes.

## 9. Non-Latin text

**Verified** on the card: `fc-list :lang=ja`, `:lang=hi` and `:lang=th` return nothing, `:lang=ar`
finds four files and `:lang=he` two, and Cyrillic and Greek work through DejaVu. A file name, a web
page or an EPUB in Arabic, Hebrew, an Indic script or CJK is rendered as boxes today, and the
session has no fallback to offer.

**Proposal**: `fonts-noto-core` (42.5 MB) in `uis/sway/packages.list`, which covers Arabic, Hebrew,
Devanagari, Thai, Armenian, Georgian and the rest of the non-CJK Noto set. `fonts-noto-cjk`
(91 MB) stays out by default and is a documented `apt install` in `04-software.md` - it is the
largest item left on this list after `firefox-esr` (280 MB installed), and a card that only needs
Latin never pays for it.

**Witnesses**: `check-system.sh` WARNs when `fc-list :lang=ar` is empty (a one-line coverage test,
not a font-by-font one); `04-software.md` § *Other languages*; `AGENTS.md` beside the font bullet.

## 10. `sshd` is enabled and the password is documented

**Verified**: `systemctl is-enabled ssh` → `enabled`, `is-active` → `active`; the build sets
`brunux`/`debian`, `%sudo` is `NOPASSWD:ALL`, and the image is meant to be published. On a café
Wi-Fi that is a login prompt for anyone on the network, and the same image is what runs
`install-emmc.sh` with root.

This is the one item I would not decide alone. Three shapes, in increasing order of change:

1. **Leave it, document it.** `04-software.md` already says the default password; add that `ssh` is
   listening. Cheapest, no protection.
2. **Off by default, on from Settings.** Ship `ssh.service` disabled (the package enables it) and a
   *Remote login* row in `rofi-quick` that enables and starts it, and shows `ssh <user>@<ip>`. The
   first-boot welcome can mention it. One row, one `systemctl` call, no daemon until asked - the
   same principle as `udisks2`.
3. **Default-deny firewall.** `ufw` (or a static `nftables.conf`) with incoming denied and `ssh`
   allowed only when the row above enables it. A real system service; 1 MB and no per-session
   process.

For a distributed image I would take **2 now, 3 before a 1.0** - and the manual must say which is
in force, because "it did not connect" is otherwise a two-hour question.

---

## Corrections this audit found in the docs

* `omarchy-gap.md` § 4 says "*`swayosd` is not packaged*". It **is** packaged in forky
  (`swayosd 0.3.0-4`, 7.7 MB). The decision not to use it survives (it is a resident GTK4 server for
  an overlay the mako card draws better here), but the reason has to be the daemon, not the
  packaging.
* `omarchy-gap.md` § 1 calls `xdg-desktop-portal-gtk` "the backend for GTK file choosers and the
  screenshot dialog". It is also the **Settings** backend (which is what makes the dark preference
  exist at all) and the **Inhibit** backend (what a video player asks to stay awake). Both are
  verified missing on the card, and both are named in the `papers` warnings above.
* `uis/sway/README.md` (line 340) sells `papers` for "search, zoom, printing and annotations".
  Nothing in this image provides CUPS, so the print dialog has no destination - the capability is
  the program's, not the port's. Either say so there and in `04-software.md`, or add `cups` (a
  system service, out of scope for this list).

## Things I would not add (and the reason, so it is not re-litigated)

| Candidate | Why not |
|---|---|
| `swayosd` as the OSD | A resident GTK4 layer-shell server for a card mako already draws; item 2 is zero process and uses the theme's own progress bar. |
| `udiskie`, `swaync`, `blueman`, `power-profiles-daemon`, `plocate` | All resident, all already declined in `omarchy-gap.md` § 3 and the package comments. |
| `nautilus`, `localsearch` | 504 packages and an indexer; `pcmanfm` is the measured answer. |
| A lock screen with a clock | `swaylock-effects` is not packaged in forky (verified: `apt-cache policy swaylock-effects` is empty) and plain `swaylock` has no text surface; the wallpaper is what the lock can own. |
| Noto CJK by default | 91 MB; a documented install is the right size for this device. |
| `mpris` on the bar | Cheap and it works (`waybar 0.15` has the module, zero process), but the bar is already nine cards wide on a 1366 px panel and `rofi-media` is two clicks away (root menu → Media, or Settings → Media). Worth trying if the bar ever gets a second row. |
| Suspend after idle | Downloads, builds and agents hold no logind inhibitor by default; `logind.conf`'s `IdleAction` would sleep through them. The DPMS timeout gets most of the battery for none of the risk. |

## One thing worth a day's test, one left alone

* **`focus_follows_mouse no`** (sway's default is `yes`). On a 1267:287 touchpad the pointer moves
  while typing; this is taste, so it is a line to try for a day, not a patch.
* **`waybar`'s `custom/busy` every second** is 2-4 ms of `sh` per second, which is nothing on the
  CPU but is a wakeup per second on a battery. It is the spinner's frame rate, so 2 s would halve the
  wakeups and make the spinner jerky; it stays as it is.

## Order and effort

1. The battery line (item 1) - 3 lines, and the only change here that a charger notices.
2. The OSD (item 2) - one script, three icons, four bindings; the largest visible change per line.
3. The GTK4 files (item 3) - two files, and `papers` stops being the odd window.
4. The tools (item 4) - four package lines and the checks that make them stay.
5. The two `for_window` rules (item 5) - the cheapest fix on the list.
6. Caches and the resident-set section (items 6 and 8) - build and tooling, no user surface.
7. The rest (7, 9) as the next batch; 10 as a decision.

Each item ships the three witnesses the repository asks for: a `check-system.sh` line, a paragraph
in `usr/share/doc/lazyos/`, and an `AGENTS.md` bullet that says why it is built this way.

## Status (2026-10-22)

All ten items were implemented. Three changed shape while being built, and each change is the
reason the item is as it is:

| # | Done | Where |
|---|---|---|
| 1 | **yes** | `config.d/23-lazyos-screensaver.conf`: `timeout 360 'swaymsg "output * power off"'`, `resume` and `after-resume` turn it back on (a suspend is not an idle timeout) |
| 2 | **yes**, three icons | `bin/osd` + `audio-volume-{muted,low,high}`. Font Awesome 4.7 has no "medium" volume glyph (only `f026`/`f027`/`f028`), so the fourth name was dropped rather than duplicating `high`; brightness reuses `display-brightness` |
| 3 | **yes** | `~/.config/gtk-4.0/{settings.ini,gtk.css}` (libadwaita named colours); verified `papers` is dark, and the Settings-portal warning it logs confirms it never asks the portal successfully. `xdg-desktop-portal-gtk` was **not** added |
| 4 | **yes** | `iproute2` + `iputils-ping` in `packages/base.list`, `zip`/`strace`/`ncdu` in `packages/tools.list` |
| 5 | **yes**, automatic half only | `config.d/32-idle-inhibit.conf`: Firefox and papers `inhibit_idle fullscreen`, mpv `visible`; the bar's `idle_inhibitor` (coffee cup) is the manual switch. **The root-menu row was dropped**: waybar holds `zwp_idle_inhibitor_v1` on its own surface, sway's `inhibit_idle` has no `toggle` and its user inhibitor belongs to a view, so a menu could not share the bar's state without a second mechanism |
| 6 | **yes** | `rootfs-setup.sh`: `fc-cache -f -s` and `gtk-update-icon-cache -q -t -f /usr/share/icons/LazyOS` after the tree overlay |
| 7 | **yes** | `bar-audio` and `rofi-quick` *Volume per application* rows (`pavucontrol`, `setsid`) |
| 8 | **yes** | `check-system.sh`: declined daemons FAIL, unknown user units WARN, in the Session section |
| 9 | **yes** | `fonts-noto-core` in `uis/sway/packages.list`; CJK stays a documented install |
| 10 | **decision: leave it on** | `05-system.md` § *Remote login (ssh)* documents the default password and how to turn it off; `check-system.sh` prints an info line when it is enabled (WARN when enabled but not running) |

Two corrections to the list above came out of the implementation: `swayosd` **is** packaged in
forky (0.3.0-4) - the reason not to use it is that it is a resident GTK4 server, not availability -
and the Settings/Inhibit portal gap is now covered by `gtk-4.0/` and the `for_window` rules, so
`xdg-desktop-portal-gtk` remains a choice for file choosers only.

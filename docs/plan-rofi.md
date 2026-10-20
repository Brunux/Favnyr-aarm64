# Porting Omarchy's menus onto rofi

Omarchy's desktop is a Quickshell shell: every menu is a QML tree whose leaves call a script, and
every script prompts through `omarchy-menu-select` (a dmenu) before it acts. That is the same shape
this port already has, one process at a time:

| Omarchy | Nureb |
|---|---|
| `omarchy-menu` (a QML tree of routes) | `~/.config/bin/bar-menu` (a `rofi -dmenu` list) |
| `omarchy-menu-select "Title" -- <flags>` | `bar-pick` (a `rofi -dmenu` wrapper: prompt, theme, `-no-custom`) |
| `omarchy-shell shell summon omarchy.*` panels | `bar-*` scripts (Wi-Fi, Bluetooth, audio, power, updates) |
| walker / QML emoji grid, clipboard panel, image picker | the same lists, drawn by rofi |
| one resident QML process (361 MB PSS) | nothing resident; rofi is 14-17 MB while open |

So "porting features to rofi" is not a new shell: it is **adding leaves** to `bar-menu` and small
scripts that use rofi as their selector. Everything below assumes the rules already in AGENTS.md -
POSIX `sh`, one short-lived process, no daemons, icons off (glycin costs 43 MB), every adopted
feature gets a `check-system.sh` line.

## The rofi toolbox we can use (and what it is good for)

| Mechanism | What it gives | Where it lands |
|---|---|---|
| `rofi -dmenu` + `-p`, `-mesg`, `-lines`, `-no-custom` | a fixed or pre-generated list, selection on stdout, exit 1 on Escape | `bar-pick`, every feature below |
| script mode `-modi "name:script"` | the script is re-run as the query changes, so the list can be **computed per keystroke** | live calculator, `fd` file finder, apt package search |
| `-kb-accept-alt` (Shift+Enter) | a second action for the selected row | "run in terminal" in `-show run` (already used) |
| `-kb-custom-1..9` (Alt+1..9) | per-row extra actions; rofi exits **10+n** with the row on stdout | copy vs. paste, open vs. reveal, QR vs. clipboard |
| `-multi-select` (+ `-sep`) | pick several rows, all printed | batch delete/keep of emoji, files, entries |
| `-filter`, `-select`, `-theme-str` | prefill the query, preselect a row, one-off geometry | "current timezone" marked, password box (already used) |
| built-in modes | `-show drun`, `run`, `window`, `ssh`, `keys`, `combi` | already wired: drun/run/window |
| `-theme-str` only | no second theme to maintain | transient tweaks (width for a QR, height for a list) |

Not available in forky: `rofi-calc`, `rofi-emoji`, `rofi-file-browser` (none is packaged). All three
are reproducible with script modes or `-dmenu` over a generated list, which is what the plan does.

## Inventory: what Omarchy has, what we have, what is worth porting

Grouped by verdict. "Effort" is implementation + test on the card, not reading.

### Already covered (no work)

| Omarchy | Nureb today |
|---|---|
| Apps (`omarchy.dock apps`) | `rofi -show drun` |
| Window list | `rofi -show window` (`$mod+Tab`; Omarchy has no equivalent in the menu) |
| Power / Lock / Screensaver / Suspend | `bar-power`, `swaylock`, `sway-screensaver`, `sway-suspend` (+ `org.nureb.*` entries) |
| Wi-Fi / Bluetooth / audio panels | `bar-net`, `bar-bt`, `bar-audio` |
| Notifications | mako |
| Clipboard *panel* (QML grid) | not yet - see P0 #2 |
| Screenshot (`omarchy-capture-screenshot`) | `grimshot` + `org.nureb.screenshot-area` |

### P0 - cheap, high value, no new daemon

| # | Feature | Today | Route through rofi | Packages | Effort |
|---|---|---|---|---|---|
| 1 | **Emoji picker** (`omarchy.emojis`) | none | emoji TSV → `rofi -dmenu` → `wl-copy` + `wtype -M shift -k Insert` (Omarchy's exact insertion trick) | `wtype` | 1 h |
| 2 | **Clipboard history** (`omarchy.clipboard`) | none | `cliphist list \| rofi -dmenu \| cliphist decode \| wl-copy`; images too (`cliphist decode > file`) | `cliphist` | 1-2 h |
| 3 | **Calculator** (rofi-calc role) | none | script mode `-modi "calc:rofi-calc"`: `awk "BEGIN{print <input>}"`, recomputed per keystroke | none | 1 h |
| 4 | **Keybindings viewer** (`omarchy-menu-keybindings`) | none | parse `bindsym`/`bindcode` from `/etc/sway/config`, `config.d/*`, `~/.config/sway/config` → `rofi -dmenu`, `-kb-custom-1` copies the command | none | 2 h |
| 5 | **Timezone** (`omarchy-menu-timezone`) | none | `timedatectl list-timezones \| rofi -dmenu` → `timedatectl set-timezone` (+ mako notification), `-filter` on the current zone | none | 30 min |
| 6 | **QR code** (Wi-Fi QR, `omarchy.wifiqr`) | none | `qrencode -t ANSIUTF8` in foot for the current SSID/PSK; a generic "text → QR" dmenu entry | `qrencode` | 1 h |
| 7 | **OCR from a region** (`omarchy-capture-text`) | none | `grim -g "$(slurp)" - \| tesseract - - \| wl-copy` + notification | `tesseract-ocr`, `tesseract-ocr-eng` | 30 min |
| 8 | **QR/barcode from a region** (`omarchy-capture-qr`) | none | `grim -g "$(slurp)" - \| zbarimg -q --raw - \| wl-copy` | `zbar-tools` | 30 min |
| 9 | **About / system** (`About`, `omarchy-launch-about`) | none | `fastfetch` in foot, plus a read-only `rofi -dmenu` of the same fields | `fastfetch` | 30 min |
| 10 | **Media control** | keys only (`XF86Audio*`) | `playerctl` list: play/pause, next, previous, and the current track as `-mesg` | `playerctl` | 1 h |

### P1 - medium, still one-shot processes

| # | Feature | Route | Notes / cost |
|---|---|---|---|
| 11 | **Root menu tree** (`bar-menu` → Omarchy's System / Appearance / Capture / Share / Setup) | nested `bar-pick` menus, one rofi per level | design work, not code: the tree must only list what exists here |
| 12 | **File picker** (`omarchy-menu-file`) | script mode over `fd` in a nav state (a temp file holds the cwd), Enter opens with `xdg-open`/`foot` | needs a file opener: no browser, no GUI editor here yet |
| 13 | **DNS switcher** (`omarchy-dns`) | `nmcli connection modify <active> ipv4.dns ...` + `nmcli con up` | needs the active connection name; test on the card |
| 14 | **Screen recording** (`omarchy-capture-screenrecording`) | `wf-recorder` + `slurp` region, start/stop toggle via a pid file | `wf-recorder`; no audio capture (no PipeWire monitor source wired yet) |
| 15 | **Clipboard images/files** | `cliphist list` + decode to `~/Pictures/clipboard/` and open | depends on #2 |
| 16 | **Reminders** (`omarchy-reminder`) | a text file + `systemd-run --user --on-active=` for the notification | a timer, i.e. a service; decide whether that is acceptable |
| 17 | **Package search** (`omarchy-pkg-install` role) | `apt-cache search` in a script mode → `foot -e sudo apt install` | Arch's tree is not portable; ours would be apt-only |
| 18 | **Color picker** (`hyprpicker`) | `grim` one pixel + `magick` to hex, then `wl-copy` | Hyprland-only tool; needs a substitute |
| 19 | **Time / speed tests** (network, disk) | `curl` to a known endpoint; `fio` or `dd` for the eMMC | low value, real numbers need care |

### Not portable, and why

| Omarchy feature | Why not |
|---|---|
| **Install / Remove** trees (browsers, gaming, AI, dev envs, services, AUR, preinstalls) | pacman/AUR/mise installers for Arch packages; on Debian they are `apt` one-liners with different names, and several (Steam, Lutris, Ollama, 1Password) are not in forky at all |
| **Appearance**: animations, gaps, corners, opacity, workspace layouts, monitors, hyprsunset | Hyprland settings; sway has its own config language and `swayfx` handles corners/shadows. Nothing to switch at runtime |
| **Bar** customisation (position, background, indicators, clock formats, dock) | waybar's config is a different format; the port's bar is deliberately one fixed solarized layout |
| **Theme switcher / theme install / font install** | a real cross-cutting project in this port: waybar `style.css`, `foot.ini`, `rofi/*.rasi`, mako, fuzzel-style popups and `swaybg` all carry the palette. That is a feature of its own, not a rofi mode |
| **OSD** (volume/brightness overlay) | not a menu: it is a transient overlay that must not take keyboard focus. mako/waybar already show the values |
| **Learn** links (Omarchy/Hyprland/Arch/Neovim docs) | they open a browser; this image has none |
| **Plugins, dock, polkit agent, agents panel, image picker, dev-gallery** | Quickshell/QML plugins; rofi has no plugin API beyond script modes |
| **Hibernate** | needs a disk swap (see `docs/optimization.md`); not available today |
| **Fingerprint, FIDO2, NVIDIA, hybrid GPU, touchpad haptics** | no such hardware on this Chromebook |
| **Network/DNS QR, share** (localsend, clipboard/file/folder share) | `localsend` is not packaged; QR is P0 #6, sharing would need its own decision |

## Per-feature specs (the ones worth writing down)

All scripts follow the same skeleton: `set -eu`, ask `bar-pick` (or a script mode) for the choice,
act, end with a mako notification so the user sees what happened. Nothing stays running.

**1. Emoji** (`rofi-emoji` script mode + insert)
  - Data: `~/.config/emojis.tsv` (`name<TAB>glyph`), curated from the Unicode CLDR names; rofi shows
    `name glyph`, `-kb-custom-1` copies the glyph instead of inserting it.
  - Insert: `wl-copy` the glyph, then `wtype -M shift -k Insert -m shift` (Omarchy's trick: the
    focused application pastes the selection). `wtype` is the one new package.

**2. Clipboard history**
  - Storage: `cliphist` (`wl-paste --watch cliphist store`), which is one **small resident process**
    (~2 MB) - the only daemon in this plan. It replaces Omarchy's QML watcher + JSON store.
  - UI: `cliphist list | rofi -dmenu -p Clipboard -mesg "Enter copies, Alt+1 deletes"`, Enter
    copies (`cliphist decode | wl-copy`), `-kb-custom-1` deletes that entry (`cliphist delete`).
  - Files/images: `cliphist decode > ~/Pictures/clipboard/<date>.<ext>` and open with imv.

**3. Calculator**
  - Script mode: `rofi -show calc -modi "calc:rofi-calc"`, where `rofi-calc` reads the query, and if
    it matches `^[0-9 +-*/().^%]+$` prints `awk "BEGIN{printf \"%.10g\n\", <query>}"`.
  - `-kb-accept-alt` copies the result instead of pasting it as the selection.

**4. Keybindings**
  - Source: every `bindsym`/`bindcode` line in `/etc/sway/config`, `/etc/sway/config.d/*.conf` and
    `~/.config/sway/config`, with the `set $var` definitions expanded (the file itself is the
    documentation, so the viewer cannot drift from it).
  - Display: `mod+key    command`, `-filter` starts empty, `-kb-custom-1` copies the command.

**5. Timezone**: `timedatectl list-timezones`, `-filter "$(timedatectl show -p Timezone --value)"`,
Enter runs `sudo -n timedatectl set-timezone` (the port's user has passwordless sudo).

**6. QR**: two entries, "Wi-Fi QR" (reads the active SSID and its PSK with `nmcli -s`, prints
`WIFI:T:WPA;S:<ssid>;P:<psk>;;` as ANSI QR in foot, `-t ansiutf8` so no image viewer is needed) and
"Text → QR" (`rofi -dmenu` for the payload, `-theme-str 'window { width: 60%; }'`).

**7/8. OCR and QR from a region**: one `slurp` each, `grim` to a pipe, `tesseract`/`zbarimg` to
stdout, `wl-copy`, mako notification with the first line of the result.

**9. About**: `fastfetch` in foot (the full output), and a `rofi -dmenu`-based read-only summary
(`uptime`, kernel, RAM, swap/zram, disk, battery, Wi-Fi SSID, IP) for when a terminal is overkill.

**10. Media**: `playerctl status/metadata` as `-mesg`, rows = play/pause, next, previous, and the
current track (selecting the track copies it).

## Conventions for the new scripts

- `~/.config/bin/rofi-<feature>` (POSIX `sh` where possible), entries added to `bar-menu`, and a
  `$mod` binding only for the root menu and the emoji picker (the rest live in the menu, not on the
  keyboard).
- Every script must work when its data source is empty (no notifications running, no clipboard
  history, no player) - rofi with an empty list looks like a hang.
- No new package unless the table above lists it, and each one gets a `check-system.sh` line
  (`have cliphist`, `have wtype`, ...).
- One paragraph in `uis/sway/README.md` per adopted feature, with the measured cost if it is not
  "14-17 MB while open, nothing resident".

## Status

Done in the first batch: **Learn** (seven pages in `usr/share/doc/nureb` + `rofi-learn`, opened with
bat in foot), the **key view** (`rofi-keys`, generated from the configuration, `Alt+Shift+K` as
Omarchy's `Super+K`), **dictation** (`dictate` + `dictate-model`, whisper.cpp, `Alt+Ctrl+X` as
Omarchy's `Super+Ctrl+X`), **hibernate** (`sway-hibernate`, gated on a disk swap and `resume=`) and
**agents** (`rofi-agents`: launch/install/default/accounts, `Alt+Ctrl+Shift+A` as Omarchy's
`Super+Shift+Ctrl+A`; its usage meters are deliberately left out).
The plan's P0 #4 (keybindings) is therefore done, and #1/#2/#3/#5-#10 are still open.

## Ideas from other distros (the review asked for)

What other desktops ship that this port does not have, filtered by "can it be a script plus a rofi
list, with no daemon". Costs are the transient kind (14-17 MB while open) unless stated.

| Idea | Seen in | Cost | Verdict |
|---|---|---|---|
| **System tasks menu** (`ujust`-style: update, backup, restore, reinstall the kpart, check-system, journal errors, disk usage) | Universal Blue/Bazzite `ujust`, MX Tools, Garuda Assistant, CachyOS Hello | none (the scripts exist) | **best next win**: it exposes `check-system.sh`, `backup-system.sh`, `install-kpart.sh` and `ncdu` from the launcher, which is where a first-time user looks |
| **Snippets** (canned texts: addresses, signatures, commands) | Raycast/Spotlight workflows, Omarchy's clipboard | none (`wtype`, `wl-copy`) | cheap and genuinely useful; a `~/.config/snippets.tsv` + `rofi -dmenu` |
| **Password manager** (rbw or passmenu) | Omarchy (`Super+Shift+/`, rbw), many tiling setups | `rbw` (1.5 MB) or `pass` (129 kB) | worth it once a vault exists; `rbw` has rofi integration built in |
| **Screenshot annotation** (`grimshot` → `swappy`) | many sway setups, macOS | `swappy` (179 kB) | small, improves an existing feature |
| **Screen recording toggle** (`wf-recorder` + `slurp`) | Omarchy (`Alt+Print`), GNOME | `wf-recorder` (220 kB) | already in the P1 plan; no audio capture yet |
| **Emoji, clipboard history, calculator, file picker, OCR/QR, QR codes, media, timezone, DNS** | Omarchy | see P0/P1 above | already planned |
| **Touchscreen on/off and screen rotation** | Omarchy `toggle-touchscreen`; tablet distros | none (`swaymsg input`/`output`) | this machine is a detachable: portrait mode and "touchscreen off while typing" are real wins |
| **On-screen keyboard** for tablet mode | GNOME/Phosh (`squeekboard`), mobile distros | `wvkbd` (826 kB, lighter) + 2-25 MB resident while shown | needs tablet-mode detection (the cros-ec switch) and input-method setup; a project, not a script |
| **Phone integration** (KDE Connect: files, clipboard, notifications) | KDE, GSConnect | `kdeconnect` (8.6 MB) + a resident `kdeconnectd` | the one daemon here would be justified, but it is a daemon; decide deliberately |
| **First-run welcome** | CachyOS Hello, EndeavourOS Welcome, Garuda | none | already covered by the Learn page |
| **Pre-update snapshot** (Btrfs rollback) | Garuda, openSUSE | `backup-system.sh` before `apt full-upgrade` (GBs per run) | the port's backups are tarballs on removable media; automatic pre-update copies are a policy decision, not a feature |
| **Recent files** | GNOME/Pop!_OS | parse `recently-used.xbel` | low value here: no GUI applications write it |
| **Weather / world clock / calendar** | Omarchy's bar, GNOME | `curl wttr.in` in foot; waybar's calendar tooltip already exists | nice-to-have, one script |
| **Unit/currency conversion** | Raycast, GNOME's calculator | needs a rates source for currency | calculator yes (planned), currency no |
| **Notifications centre / history** | GNOME, KDE, Omarchy | mako's `makoctl` has a history (`makoctl history`), rofi could list it | small, once mako history is enabled |
| **Container/dev environments** (toolbox/distrobox) | Silverblue, Bazzite | `podman` + `distrobox` | out of scope for 4 GB and one eMMC |
| **Power profiles** | Pop!_OS, GNOME | `power-profiles-daemon` (a daemon) | the bar's power panel shows the readings; the daemon is not worth its memory |

## Order of work

1. **Batch A (one rebuild, ~4 h)**: emoji, calculator, keybindings, timezone, QR, OCR, QR-from-region,
   About, media, plus the new `bar-menu` entries. Packages: `wtype`, `cliphist`, `imv` (optional).
2. **Batch B (~4 h)**: clipboard history (the watcher decision), file picker, DNS, screen recording.
3. **Batch C (design)**: the full root menu tree, then whatever P1 items the tree makes visible.
4. **Never**: the Arch/Hyprland/Quickshell-specific lists and the theme system, unless the theme
   work becomes a project of its own.

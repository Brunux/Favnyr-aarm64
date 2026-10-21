# The Omarchy gap (October 2026)

What the Omarchy v4 desktop has that this port does not, what is covered by something else, and
what is deliberately absent. It exists because "are we missing anything?" is asked often enough to
deserve a checked answer instead of a feeling.

## How this was computed

* **Packages**: the Omarchy-v4 Debian port (Deboma, `uis/omarchy/packages.list`) is the app/tool
  half of upstream v4 mapped onto Debian names. Diffed against the four lists this port installs
  (`packages/base.list`, `packages/tools.list`, `uis/sway/packages.list`,
  `devices/google-asurada/packages.list`): **86 names in Omarchy are not in this tree**. Most (45)
  belong to the Qt6/Quickshell/Hyprland shell and its look-and-feel helpers - the desktop itself is
  the difference (swayfx + waybar + rofi + mako + foot), not a missing package - and three of those
  names (`hyprpicker`, `yaru-theme-icon`, `libqalculate23`) are standalone tools that are
  reclassified in the tables below.
* **Features**: the inventory in [`plan-rofi.md`](plan-rofi.md) and the manual's own "not built yet"
  list (`uis/sway/files/usr/share/doc/nureb/03-shortcuts.md`).
* Sizes are `Installed-Size` from forky, uncompressed.

## 1. Missing, and worth adding

Grouped by what the machine cannot do today. Sizes are per package, for a 4 GB device where the
whole image is ~4 GiB.

| Theme | Package | Size | What it buys |
|---|---|---|---|
| Media | `mpv` + `mpv-mpris` | 3.5 MB + 83 kB | A real player for local video/audio, and MPRIS so `playerctl`, the Media menu and the bar see it. Firefox can play a file, but it is not a player. |
| Media | `imv` | 375 kB | The image viewer (the file browser sends non-text files to Firefox today; imv is instant and is what clipboard images would open in). |
| Media | `papers` | 7.2 MB | The document viewer (PDF/EPUB). Firefox renders PDFs; papers does it without a browser. |
| Media | `ffmpegthumbnailer` | 89 kB | Video thumbnails, for any future file browser. |
| Capture | `swappy` | 179 kB | **Annotate a screenshot** (arrows, text, crop). `grimshot` only saves/copies; Debian has no `satty`. |
| Capture | `wf-recorder` | 220 kB | **Screen recording**. Nothing does this today. |
| Clipboard | `cliphist` | 2.2 MB | Clipboard history. The one piece of this list that is resident (~2 MB while running); the manual already reserves `Alt+Ctrl+V` for it. |
| Emoji | `fonts-noto-color-emoji` | 10.9 MB | **Colour emoji glyphs.** The Nerd Fonts carry monochrome symbol planes, not emoji; today an emoji renders as tofu even if something inserts it. |
| Fonts | `fonts-noto-core` | 42.5 MB | Cyrillic, Greek, Arabic, Hebrew, Devanagari… coverage. The image ships DejaVu + JetBrains + the icon fonts, which is Latin only. |
| Fonts | `fonts-noto-cjk` | 91 MB | Chinese/Japanese/Korean. Optional: it is the single biggest item here. |
| Storage | `udisks2` + `udiskie` | 3.2 MB + 523 kB | **Mounting removable media.** `rofi-places` lists disks that are *already* mounted; without udisks2 an unprivileged session cannot mount a USB stick at all. |
| Storage | `gnome-disk-utility` | 7.2 MB | Formatting/partitioning with a GUI. Today it is `parted`/`cgpt` in a terminal. |
| Files | `nautilus` + `python3-nautilus` + `gnome-sushi` + `gvfs-backends` | 4.6 MB | The GUI file manager and its previews. `rofi-files` is the port's answer and covers navigation, but has no thumbnails and no drag and drop. |
| Secrets | `gnome-keyring` + `libsecret-1-0` | 4.6 MB + 414 kB | Somewhere for applications to store a password/token (Git, Nextcloud, Firefox Sync). |
| Editor | `neovim` | 9.7 MB | Omarchy's editor. What there is: `nano`, `bat` and the `eff` helper, which edits with `${EDITOR:-nano}` (`EDITOR` is unset on purpose - `90-cli-tools.sh` says why). |
| Power | `power-profiles-daemon` | 207 kB | The battery profile switch. The kernel is pinned to `schedutil` (`optimization.md` item 2); PPD would expose performance/balanced/power-saver. |
| Portals | `xdg-desktop-portal-gtk` | 776 kB | The backend for GTK file choosers and the screenshot dialog. With only `xdg-desktop-portal-wlr`, an application that asks the portal for a file has no implementation. |
| Network | `nm-connection-editor` | 5.5 MB | VPN and 802.1x editing. `bar-net` lists and connects Wi-Fi; it cannot author a VPN profile. |
| Colour | `hyprpicker` | 408 kB | A colour picker. Debian describes it as wlroots-compatible (it uses wlr-screencopy); the fallback is `grim` + a pixel read. |

## 2. Covered by something else

| Omarchy | Here |
|---|---|
| `chromium` (280 MB) | `firefox-esr` (in `uis/sway/packages.list`), the image's one viewer and browser |
| `alacritty`, `kitty` | `foot` - half the memory of an alacritty window on a 4 GB machine |
| `yaru-theme-icon` | the **Nureb** icon theme, rendered by `scripts/make-icons.sh` from the Font Awesome codepoints used in the menus |
| `libxkbcommon-tools` (`xkbcli list`) | `keyd` for the keymap and `swaymsg -t get_inputs` + `jq` for the layout indicator |
| `libqalculate23` | `rofi-calc` (an `awk` expression, no library) |
| `hyprsunset` | `wlsunset` + the night-light row in `rofi-quick` |
| `pamixer` | `pactl` from `pulseaudio-utils` |
| `xdg-utils`, `xdg-terminal-exec` | the shipped `/usr/local/bin/xdg-open` shim (what foot's ctrl+click calls) and rofi's `terminal: foot` |
| `xdg-desktop-portal-hyprland` | `xdg-desktop-portal-wlr` |
| `desktop-file-utils`, `gtk-update-icon-cache` | build-time only: nothing at runtime needs `update-desktop-database`, and GTK scans a theme directory when its cache is absent. A future `make-icons.sh` could build the cache. |
| `fish`, `zsh` (upstream) | bash with the `90-cli-tools.sh` environment and the aliases in `~/.bashrc` |

## 3. Deliberately not here

| Package | Why not |
|---|---|
| `gawk` | Omarchy's theme renderer needs awk's `RT`; this port's theme is a static file, and mawk is faster. |
| `lua5.4`, `tree-sitter-cli` | Editor and keybindings-view tooling for Omarchy's own components; `rofi-keys` parses sway's config with `awk` instead. |
| `plocate` | `locate` **plus** `plocate-updatedb.timer` - a systemd timer that indexes the disk. Not on a card that is rewritten on every install. |
| `libnotify-bin` | `notify-send` is already pulled in by `grimshot`/mako's client tools. |
| Qt6, `qml6-module-*`, `quickshell`, `uwsm`, `hyprland*`, `gir1.2-*`, `liblayershellqt*`, `lxqt-qtplugin`, `qt6ct`, `nwg-look` | The Quickshell shell and its Qt runtime. This port's shell is waybar + rofi + mako, and its theme is static, so there is nothing to configure with a Qt/GTK appearance tool. |
| `xdg-desktop-portal-hyprland` | Hyprland-specific portal; `-wlr` is the wlroots one. |

## 4. Features, not packages

The menus are one-shot `rofi` scripts (`uis/sway/files/home/.config/bin/rofi-*`). The ones Omarchy
has that are still missing:

| Omarchy feature | Today | What it would take |
|---|---|---|
| **Emoji picker** (`Super+Ctrl+E`) | none | A data file (or a generated TSV) → `rofi -dmenu` → `wl-copy` + `wtype -M shift -k Insert`; `wtype` is installed. Needs `fonts-noto-color-emoji` to render. |
| **Clipboard history** (`Super+Ctrl+V`) | none | `cliphist` + `wl-paste --watch`, a `rofi` list, `Alt+1` to delete. The only resident process in this list. |
| **Capture menu** (`OCR`, `QR`, `record`) | `grimshot` save/copy only | `grim -g "$(slurp)" \| tesseract - - \| wl-copy` (OCR), `grim … \| zbarimg -q --raw -` (QR), `wf-recorder` for video. **`tesseract-ocr`, `zbar-tools` and `qrencode` are already installed** - this is wiring, not packages. |
| **Wi-Fi QR** | none | `nmcli -s -g 802-11-wireless-security.psk` + `qrencode -t ANSIUTF8` in foot. `qrencode` is installed. |
| **Package search/install** | none | `apt-cache search` → `rofi` → `foot -e sudo apt install`. Debian's archive, not AUR. |
| **DNS switcher** | none | `nmcli connection modify <active> ipv4.dns …` + a `rofi` list of common resolvers. |
| **Reminders** | none | A text file + `systemd-run --user --on-active=` (a transient timer, not a service). |
| **Theme switcher, appearance, animations, monitor layout** | fixed solarized theme | A real project: the palette lives in waybar CSS, foot.ini, rofi's rasi, mako, sway `client.*` and swaybg. Not a menu. |
| **OSD (volume/brightness overlay)** | mako + the bar show values | An overlay that must not take focus; `swayosd` is not packaged. |
| **Weather in the bar** | none | A waybar `custom` module and one HTTP call; the upstream one needs a location and an API key. |

Not portable from Omarchy at all: its **Install/Remove trees** (pacman/AUR/mise for Arch packages -
on Debian they are `apt` one-liners with different names), its **Quickshell plugins/dock/agents
panel**, and the **fingerprint/FIDO2/NVIDIA/hybrid-GPU** items, since this Chromebook has none of
that hardware.

## 5. What this port has and Omarchy does not

Worth saying, because the gap list is one-directional by nature: `sway-hibernate` gated on a real
disk swap and `resume=`; `dictate` (whisper.cpp, a pinned model, `wtype`); the agents menu with
per-account `CLAUDE_CONFIG_DIR`/`CODEX_HOME` homes; `rofi-files` as a script instead of a file
manager; and a session whose entire resident set is the compositor, waybar, mako, pipewire and the
bar scripts.

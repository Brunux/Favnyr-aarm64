# rofi: the launcher, the window list and every menu

One binary, a handful of jobs. It exists while its window is on screen and
nothing of it stays afterwards (13-17 MB of memory while open; there is no
daemon).

| You press | What opens | What it lists |
|---|---|---|
| **Search** or **Alt+D** | `rofi -show drun` | the applications: the `.desktop` entries, including this image's own (Settings, Wi-Fi, Bluetooth, audio, media, notifications, power, Files, Learn, Keys, Dictate, Agents…) |
| **Alt+Tab** | `rofi -show window` | the open windows (sway has no switcher of its own; this uses wlroots' foreign-toplevel protocol) |
| **Alt+Shift+D** | `rofi -show run` | every executable in `$PATH`. **Enter** runs it, **Shift+Enter** runs it in foot |
| **Alt+Space** | `bar-menu` | the root menu: what opens something, the system panels, and the session actions |
| **Alt+Ctrl+K** | `rofi-keys` | every key binding this session defines, parsed from `/etc/sway` and `~/.config/sway`. Enter copies the command |
| **Alt+Ctrl+F** | `rofi-files` | the filesystem: directories first, sizes and dates dimmed. Enter opens a directory or reads a text file in foot; the dotfiles are one row away |
| **Alt+Ctrl+C** | `rofi-capture` | **capture**: a screenshot (screen, area or window), annotate it, read the text or a QR code out of a region, and start or stop a recording |
| **Alt+Ctrl+O** | `rofi-quick` | **Settings**: the switches (Wi-Fi and Bluetooth radios, night light, do-not-disturb, touchpad, rotation) and the panels, in one list |
| **Alt+Ctrl+P** or **Alt+Escape** | `rofi-power` | **power**: lock, suspend, hibernate, log out, reboot, shut down - the last three ask once more |
| **Alt+Ctrl+N** | `rofi-notify` | the notification history, with do-not-disturb and "copy this notification" |
| **Alt+Ctrl+Q** | `rofi-calc` | a calculator: type `(3*4)/2`, Enter copies `6` |
| any bar module click | `bar-pick` → `rofi -dmenu` | the fixed list that module hands it (Wi-Fi networks, sinks, devices, power actions, …) |
| **Learn** (root menu) | `rofi-learn` | these pages, rendered with glow, plus a search across all of them |

The rest of the menus live in the root menu and the launcher: **Places**
(home, documents, downloads, pictures, the mounted disks), **Media** (what is
playing, play/pause/next/previous), **Time zone** (with network time),
**Free space** (what is using the disk, one action per row), **End a process**
(the heaviest twenty), **Calculator**, **SSH** (hosts from `~/.ssh/config`).

## Something is happening

A scan or a connection takes seconds, and a menu that answers only when it is
finished looks like a click that was never registered. Every slow action says so
in two places at once:

* a **mako notification** that does not expire while the action runs, and then
  becomes the result - the same card reads "Looking for networks…", "9 networks
  in range" or "Connected to Home". Errors are the same card in red (critical);
* a **spinner on the bar**, left of the updates indicator: a braille frame with
  the name of the task in yellow, so it is visible even when the notification has
  already scrolled away.

Both come from `~/.config/bin/busy` (`start`, `update`, `stop`, `end`, `fail`)
and `~/.config/bin/bar-busy`; nothing of either stays after the action. Wiring a
new action in is two lines:

```sh
busy start wifi Wi-Fi 'Looking for networks…' network-wireless
out=$(nmcli device wifi list --rescan yes)
busy end wifi '9 networks in range'      # or: busy fail wifi "$out"
```

The state is a file per task in `$XDG_RUNTIME_DIR/nureb-busy`, and the bar card
disappears on its own if a script dies without ending it.

## Browsing files

`rofi-files` (Alt+Ctrl+F, "Files" in the root menu and the launcher) is a shell
script over `rofi -dmenu`, because rofi 2.0 has no filebrowser mode:

* directories come first, in cyan and with a trailing `/`; then files, with the
  size and the date in the dim colour; a symlink to a directory navigates like a
  directory, and a broken one says so;
* **Enter** opens a directory (the listing reopens there), reads a text file with
  `bat` (paged, in the terminal's own solarized colours, `BAT_THEME=ansi`, inside
  the floated `popup` window), or opens anything else with **Firefox** - an
  image, a PDF, an SVG, the HTML file;
* the first row is `↑ parent directory`, and the last one toggles the dotfiles
  (hidden ones are out of the way until asked for);
* typing filters the list; a path that matches no row is taken literally, so
  `/etc/sway/config.d` or `~/notes.md` can be typed and opened with
  **Ctrl+Return** (or plain Enter once nothing matches);
* the line under the prompt says where you are, how many directories and files
  are there and how much space is left;
* a file that is not text goes to Firefox, which renders its own PDFs and shows
  an image at its real size; if the browser is ever removed, a notification says
  so instead of opening an empty page.

`xdg-open` exists too (`/usr/local/bin/xdg-open`, two dozen lines): a URL goes to
Firefox, a path becomes a `file://` URL. That is what foot's ctrl+click and the
CLI tools that print a URL call.

It is the same trade as everything else here: 16 MB while open, nothing
afterwards.

## Capture: the screen as a picture, as text, or as a video

**Alt+Ctrl+C** (`rofi-capture`, and "Capture" in the root menu and the launcher)
is the list behind the screenshot keys. Everything on it is one command that
ends before the list closes again:

| Row | What it does |
|---|---|
| **Screenshot** · the whole screen / an area / a window | `grimshot` saves it under `~/Pictures` and names the file in a notification, which also carries the image |
| **Screenshot · an area, to the clipboard** | the same, straight to the clipboard (what **Alt+Ctrl+S** also does) |
| **Annotate an area** | `grimshot` saves it and **swappy** opens it: arrows, rectangles, text, blur, crop. Enter saves back over the same file |
| **Text from an area** | `slurp` picks the region, `tesseract` reads it, and the text lands in the clipboard, with the first line in the notification. English is what the image ships (`tesseract-ocr-eng`); other languages are one `apt install` away |
| **QR code from an area** | `zbarimg` reads a QR code (or an EAN/Code-128 barcode) from the region and copies what it says |
| **Make a QR code** | the Wi-Fi network's password (read from the saved profile, so it is not typed again) or any text, drawn in a floating terminal by `qrencode`; nothing is left on disk |
| **Record** · the whole screen / an area | `wf-recorder` starts writing an `.mp4` into `~/Videos`, and the same menu row turns into **Stop recording** |

The recording is the one thing that outlives the list, so it says so twice: the
mako card stays until it is stopped, and the bar shows the spinner with
`Recording` in it. It encodes with x264 on the CPU because this SoC has no
hardware encoder (`veryfast`, crf 28, 30 fps - the point where a full-screen
recording still leaves the compositor its frame budget). There is **no audio
track**: that needs a PipeWire monitor source and nothing here wires one, so
`wf-recorder -a` would record the microphone instead. The file is finalised on
SIGINT, which is what Stop sends; a recorder that had to be killed leaves a file
the notification calls empty rather than claiming success.

The QR the Wi-Fi row shows is the phone-readable `WIFI:T:WPA;S:…;P:…;;`, with
`\`, `;`, `,` and `:` escaped. It contains the password, so the temporary file it
is drawn from is created `0600` and removed when the window closes.

Everything this menu needs is already in the image: `grimshot`, `swappy`,
`tesseract`, `zbar-tools`, `qrencode` and `wf-recorder`; `check-system.sh`
fails when one of them is missing, because a row that does nothing is worse than
a row that is not there.

## The manual, rendered

`rofi-learn` lists `usr/share/doc/nureb/*.md` and opens the chosen page with
**glow**, so it is a document (headings, tables, inline code, lists) instead of
markdown source. The style is `~/.config/glow/solarized.json`, the same palette
as everything else: headings carry a cyan/blue bar, inline code and fenced
blocks sit on base02, and code is highlighted with the solarized token colours.
The first row searches all pages with ripgrep and opens the one that matches.

`glow -p` pages through `less`, so the page scrolls with the usual keys and
leaves nothing behind when it closes.

## The theme and the icons

Two files, and everything - launcher, window list, bar panels, key view, the
manual - reads the same theme:

* `~/.config/rofi/config.rasi` - the settings: the modes, `terminal: "foot"`,
  fuzzy matching, history, `show-icons: true`, `icon-theme: "Nureb"`.
* `~/.config/rofi/solarized.rasi` - the look: base03 window, 2 px cyan rim,
  JetBrains Mono 12, base02 card on the selected row, the box shrinking to its
  entries.

Two kinds of icon, because rofi supports two:

* the lists this session draws itself show a **font glyph** in the row text
  (Font Awesome, the same alphabet as the bar). It costs nothing, follows the
  palette, is filtered away by typing, and is what `lib-rofi.sh` puts in every
  `rofi-*` menu;
* the launcher and the window switcher show the application's **image icon**,
  which rofi resolves through glycin. Ten rows of icons cost about 13 MB of
  memory while the window is open and nothing afterwards.

Debian's Adwaita ships only monochrome symbolic icons and none of this session's
names, so the image carries a small **Nureb** icon theme: the same Font Awesome
codepoints, rendered once to 48×48 PNGs by `scripts/make-icons.sh`. Its
`index.theme` inherits Adwaita and hicolor, so an application that brings its own
icon (foot, btop, htop, anything installed later) still shows it in colour.

The bar's popups pass `-no-show-icons`: they draw a glyph in the text and do not
need an icon column.

## Adding things to the launcher

* **An application**: drop a `.desktop` file in `~/.local/share/applications/`.
  For a terminal program, `Terminal=true` is enough - rofi starts it in foot.
  This is how `org.nureb.*` entries are made (they live in
  `/usr/share/applications/`, shipped by the image).
* **An icon for it**: put the name in its `Icon=`, and either let the
  application ship the image or add a line to `scripts/make-icons.sh` (a Font
  Awesome codepoint) and run it on the build host.
* **A menu**: `bar-pick "Prompt" < list-of-lines` gives you a rofi dmenu with the
  session's theme and the same "Escape means cancel" contract; the `rofi-*`
  scripts add the shared row style from `lib-rofi.sh` (`ui_row`, `ui_pick`,
  `ui_say`).
* **A dynamic list**: a script mode (`-modi "name:script"`) is called when the
  mode starts and when a row is chosen - *not* on every keystroke, which is why
  the calculator is a one-shot prompt and not a live preview. The file browser
  does not need it either: it lists one directory per run and reopens in the
  chosen one.

## When rofi misbehaves

* **A dialog in the middle of the screen naming a file, a line and a column**:
  that is a theme parse error. `rofi` only reports it when a window opens, so
  `rofi -dump-config` is not a syntax check.
* **The window opens empty**: the list handed to it was empty. An empty menu is
  indistinguishable from a hang; the scripts are written to exit instead.
* **A row of empty space where an icon should be**: the `.desktop` names an icon
  that no installed theme has. `check-system.sh` verifies the Nureb theme and
  every `org.nureb.*` icon.
* **Nothing happens on a bar click**: `bar-pick` is not executable, or rofi is
  missing. `check-system.sh` fails for both.

# rofi: the launcher, the window list and every menu

One binary, four jobs. It exists while its window is on screen and nothing of it
stays afterwards (14-17 MB of memory while open; there is no daemon).

| You press | What opens | What it lists |
|---|---|---|
| **Search** or **Alt+D** | `rofi -show drun` | the applications: the `.desktop` entries, including this image's own (Wi-Fi, Bluetooth, audio, power, lock, screensaver, suspend, screenshot, lazygit, tmux, Learn, Keys, Dictate, Hibernate) |
| **Alt+Tab** | `rofi -show window` | the open windows (sway has no switcher of its own; this uses wlroots' foreign-toplevel protocol) |
| **Alt+Shift+D** | `rofi -show run` | every executable in `$PATH`. **Enter** runs it, **Shift+Enter** runs it in foot |
| **Alt+Shift+K** | `rofi-keys` | every key binding this session defines, parsed from `/etc/sway` and `~/.config/sway`. Enter copies the command |
| any bar module click | `bar-pick` → `rofi -dmenu` | the fixed list that module hands it (Wi-Fi networks, sinks, devices, power actions, …) |
| **Learn** (bar menu) | `rofi-learn` | these pages, opened in foot |

## Keys inside rofi

| Key | Does |
|---|---|
| type | filters the list (fuzzy: `ff` finds `firefox` and `ffmpeg`) |
| Up / Down, Ctrl+P / Ctrl+N, Tab | move the selection |
| Page Up / Page Down | a page at a time |
| Ctrl+U, Ctrl+W, Ctrl+Backspace | clear the line, delete a word |
| Enter | accept the selected row |
| **Shift+Enter** | accept it *in a terminal* (run mode and terminal-only applications) |
| Escape | close without choosing (exit status 1, which is what the bar scripts test) |
| Ctrl+Tab | cycle modes, when the theme keeps the mode switcher (it does not here - each shortcut opens the mode it wants) |

## The theme

Two files, and everything - launcher, window list, bar panels, key view, the
manual - reads the same theme:

* `~/.config/rofi/config.rasi` - the settings: the three modes, `terminal: "foot"`,
  fuzzy matching, history, and `show-icons: false`.
* `~/.config/rofi/solarized.rasi` - the look: base03 window, 2 px cyan rim,
  JetBrains Mono 12, base02 card on the selected row, the box shrinking to its
  entries.

**Icons are off on purpose.** With `show-icons: true` the same application list
costs 57 MB instead of 16, because rofi 2.0 renders `.desktop` icons through
glycin, a sandboxed image loader. If you want them anyway, change that one line
and accept the cost.

Changing a colour is one edit in `solarized.rasi`; the properties are rofi's
own (`window`, `inputbar`, `element`, `element selected`, `prompt`, `element-text`).
`rofi-theme(5)` documents them, and `rofi -dump-theme` prints the merged result.

## Adding things to the launcher

* **An application**: drop a `.desktop` file in `~/.local/share/applications/`.
  For a terminal program, `Terminal=true` is enough - rofi starts it in foot.
  This is how `org.nureb.*` entries are made (they live in
  `/usr/share/applications/`, shipped by the image).
* **A menu**: `bar-pick "Prompt" < list-of-lines` gives you a rofi dmenu with the
  session's theme and the same "Escape means cancel" contract; see the scripts in
  `~/.config/bin/` for a dozen examples.
* **A dynamic list**: rofi can re-run a script on every keystroke (`-modi
  "name:script"`), which is how a calculator or a file finder is built. Nothing
  in this image needs it yet.

## When rofi misbehaves

* **A dialog in the middle of the screen naming a file, a line and a column**:
  that is a theme parse error. `rofi` only reports it when a window opens, so
  `rofi -dump-config` is not a syntax check.
* **The window opens empty**: the list handed to it was empty. An empty menu is
  indistinguishable from a hang; the scripts are written to exit instead.
* **Nothing happens on a bar click**: `bar-pick` is not executable, or rofi is
  missing. `check-system.sh` fails for both.

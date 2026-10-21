# Shortcuts

`$mod` is **Alt** on this port. Omarchy (the desktop this session is ported from)
uses Super, so the same actions usually keep the same letter and change the
modifier; the table at the end says so where it matters.

The live list is **Alt+Ctrl+K** (`rofi-keys`): it is parsed from the
configuration files, so it is never stale, it can be searched, and Enter copies
the command. This page is the tour.

## Getting around

| Keys | Does |
|---|---|
| Search key (or Alt+D) | launcher: applications |
| Alt+Space | the root menu (the bar's menu button, on the keyboard) |
| Alt+Tab | window switcher |
| Alt+Shift+D | command palette (`$PATH`; Shift+Enter runs it in foot) |
| Alt+Ctrl+K | every key binding, searchable, Enter copies the command |
| Alt+Ctrl+F | files: browse a directory, Enter reads a text file in foot |
| Alt+Ctrl+O | settings: the radio switches, night light, touchpad, rotation |
| Alt+Ctrl+N | notifications: the history, do-not-disturb, copy one |
| Alt+Ctrl+Q | calculator: type it, Enter copies the result |
| Alt+Return | terminal (foot) |
| Alt+Q (or Alt+Shift+Q) | close the focused window |
| Alt+Shift+C | reload the configuration |
| Alt+Shift+E | exit the session (asks first) |
| Alt+grave / Alt+Shift+grave | scratchpad: show the hidden window / send the focused one there |
| Alt+R | resize mode (arrows, Return or Escape to leave) |

## Panels, lock and power

These are the bar's panels without the mouse, on the same letters Omarchy uses
with `Super+Ctrl`:

| Keys | Does |
|---|---|
| Alt+Ctrl+A | audio output (sinks, and the two mutes) |
| Alt+Ctrl+B | Bluetooth (scan, connect, disconnect) |
| Alt+Ctrl+W | network (Wi-Fi list, strongest first) |
| Alt+Ctrl+P | power panel (charge, draw, time left) |
| Alt+Escape | system panel (lock, suspend, log out, reboot, power off) |
| Alt+Ctrl+L | lock the screen (the Lock key does the same and then suspends) |
| Alt+Ctrl+X | dictation on/off |
| Alt+Ctrl+Shift+A | agents (launch, accounts, install) |
| Alt+Ctrl+O | settings: Wi-Fi and Bluetooth radios, night light, do-not-disturb, touchpad, rotation |
| Alt+Ctrl+N | notifications: history, do-not-disturb, copy |
| Alt+Ctrl+Q | calculator |
| Alt+Shift+Escape / Alt+Ctrl+Escape | lock then suspend / suspend |

## Windows

| Keys | Does |
|---|---|
| Alt+arrows (or Alt+H/J/K/L) | focus |
| Alt+Shift+arrows | move the window |
| Alt+1..0 | go to a workspace |
| Alt+Shift+1..0 | move the window there |
| Alt+B / Alt+V | split horizontally / vertically |
| Alt+S / Alt+W / Alt+E | stacking / tabbed / toggle split |
| Alt+F | fullscreen |
| Alt+T | float the window |
| Alt+Shift+Space | toggle floating (the sway default) |
| Alt+A | focus the parent container |
| three-finger swipe left/right | previous / next workspace (touchpad) |

## Sound, light and screenshots

| Keys | Does |
|---|---|
| the volume keys (or F1/F2 with the Search key) | volume -5% / +5% |
| Mute | mute · **Shift+Mute**: switch the audio output |
| the brightness keys (or F3/F4 with Search) | brightness -5% / +5% |
| Alt+Shift+S / Alt+Shift+F / Alt+Ctrl+S | screenshot area / full screen / copy area |

The top row of this keyboard sends action keys, not F1-F12: keyd turns
**Ctrl+Alt+top row** into virtual terminals 1-10 and **Search+top row** into
F1-F10. Without keyd there would be no way to reach a VT on this machine.

## What was copied from Omarchy, and what differs

`Super` there is `Alt` here; the letters and Ctrl/Shift combinations are the same
wherever sway does not already own the key.

| Action | Omarchy | Here |
|---|---|---|
| terminal | Super+Return | Alt+Return |
| apps | Super+Alt+Space | Search key / Alt+D |
| root menu | Super+Space | **Alt+Space** |
| window switcher | Alt+Tab | **Alt+Tab** (rofi's window mode) |
| keybindings | Super+K | **Alt+Ctrl+K** (Alt+K is vim focus up, Alt+Shift+K moves the window) |
| settings / control centre | Super+Ctrl+O | **Alt+Ctrl+O** (one list of switches and panels) |
| calculator | Super+Ctrl+Q | **Alt+Ctrl+Q** |
| notifications | bar panel | **Alt+Ctrl+N** |
| dictation | Super+Ctrl+X | Alt+Ctrl+X |
| agents | Super+Shift+Ctrl+A | Alt+Ctrl+Shift+A |
| floating | Super+T | Alt+T |
| close window | Super+Q (or Super+W) | Alt+Q (Alt+W is sway's tabbed layout) |
| scratchpad | Super+grave | Alt+grave |
| lock | Super+Ctrl+L | Alt+Ctrl+L (and the Lock key) |
| system menu | Super+Escape | Alt+Escape → the power panel |
| audio / Bluetooth / network / power panels | Super+Ctrl+A/B/W/P | Alt+Ctrl+A/B/W/P |
| screenshot | Super+Shift+S, Print | Alt+Shift+S, Alt+Shift+F, Alt+Ctrl+S |
| files | Super+Shift+F opens the file manager | Alt+Ctrl+F: `rofi-files` |
| fullscreen | Super+F | Alt+F |
| volume, brightness, media keys | XF86 keys | the same keys |
| workspaces | Super+Tab cycles | Alt+1..0 and three-finger swipe |
| layouts | Super+L cycles | Alt+S/W/E (sway's three layouts) |

**Three deliberate deviations**, all because sway's defaults own the key Omarchy
uses: the key view is `Alt+Ctrl+K` (not `Alt+K`, which is vim focus up, nor `Alt+Shift+K`, which moves the window), closing
is `Alt+Q` (not `Alt+W`, which is the tabbed layout), and there is no single
"cycle layout" key because sway's layouts are stacking/tabbed/split rather than
dwindle/master/scrolling.

Not built yet: clipboard manager `Alt+Ctrl+V`, emoji `Alt+Ctrl+E` and the
capture menu `Alt+Ctrl+C` - the same letters as Omarchy. The capture menu is
only wiring (`tesseract`, `zbar-tools` and `qrencode` are already installed);
clipboard history needs `cliphist`, and the emoji picker needs
`fonts-noto-color-emoji` to draw a glyph at all. `docs/omarchy-gap.md` has the
full list.

Everything else lives in the root menu instead of on a key: Places, Media, Time
zone, Free space, End a process, SSH and the manual. `Alt+Space` then the first
letter is usually enough.

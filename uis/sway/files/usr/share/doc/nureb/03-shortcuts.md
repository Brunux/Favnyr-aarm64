# Shortcuts

`$mod` is **Alt** on this port. Omarchy (the desktop this session is ported from)
uses Super, so the same actions usually keep the same letter and change the
modifier; the table at the end says so where it matters.

The live list is **Alt+Shift+K** (`rofi-keys`): it is parsed from the
configuration files, so it is never stale, it can be searched, and Enter copies
the command. This page is the tour.

## Getting around

| Keys | Does |
|---|---|
| Search key (or Alt+D) | launcher: applications |
| Alt+Tab | window switcher |
| Alt+Shift+D | command palette (`$PATH`; Shift+Enter runs it in foot) |
| Alt+grave | the root menu (the bar's menu button, on the keyboard) |
| Alt+Shift+K | this list, live |
| Alt+Return | terminal (foot) |
| Alt+Shift+Q | close the focused window |
| Alt+Shift+C | reload the configuration |
| Alt+Shift+E | exit the session (asks first) |
| Alt+Escape | lock the screen |
| **Lock key** | lock, and logind suspends a moment later |
| Alt+Shift+Escape / Alt+Ctrl+Escape | lock then suspend / suspend |
| Alt+R | resize mode (arrows, Return or Escape to leave) |

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
| Alt+Minus / Alt+Shift+Minus | scratchpad show / move to it |
| three-finger swipe left/right | previous / next workspace (touchpad) |

## Sound, light and power

| Keys | Does |
|---|---|
| F1/F2 (top row with the Search key) or the volume keys | volume -5% / +5% |
| Mute | mute |
| Shift+Mute | switch the audio output |
| F3/F4 or the brightness keys | brightness -5% / +5% |
| Alt+Shift+S / Alt+Shift+F / Alt+Ctrl+S | screenshot area / full screen / copy area |
| Alt+Ctrl+X | dictation on/off |

The top row of this keyboard sends action keys, not F1-F12: keyd turns
**Ctrl+Alt+top row** into virtual terminals 1-10 and **Search+top row** into
F1-F10. Without keyd there would be no way to reach a VT on this machine.

## What was copied from Omarchy, and what differs

| Action | Omarchy | Here |
|---|---|---|
| launcher | Super+Space (menu), Super+Alt+Space (apps) | Search key / Alt+D |
| windows | Super+Tab in a group; no switcher | **Alt+Tab** (rofi's window mode) |
| keybindings | Super+K | Alt+Shift+K |
| dictation | Super+Ctrl+X | Alt+Ctrl+X |
| terminal | Super+Return | Alt+Return |
| screenshot | Super+Shift+S, Print | Alt+Shift+S, Alt+Shift+F, Alt+Ctrl+S |
| OCR from a screenshot | Super+Ctrl+Print | planned (the tools - tesseract - are installed) |
| clipboard manager | Super+Ctrl+V | planned (rofi + cliphist) |
| file manager | Super+Shift+F | planned |
| passwords | Super+Shift+/ (rbw) | not ported (rbw is packaged, no vault configured) |
| calculator | Super+Ctrl+Q | planned (rofi script mode) |
| emoji | Super+Ctrl+E | planned (rofi + wtype) |
| audio / Bluetooth / network / power | Super+Ctrl+A/B/W/P | the bar's modules, and the root menu |
| floating a window | Super+T | Alt+T |
| switch audio output | Shift+AudioMute | Shift+Mute |
| lock | Super+Ctrl+L | Alt+Escape, the Lock key |
| close window | Super+Q or Super+W | Alt+Shift+Q |

The modifier is the one deliberate difference: Alt is this port's `$mod` (it is
what Nura used, and the Search-key launcher is bound to `Super_L` alone, so
Super stays free for the firmware and for a single tap).

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
| Alt+Ctrl+V | clipboard history: Enter puts an entry back, Alt+1 deletes it |
| Alt+Ctrl+E | emoji: type a name, Enter inserts the glyph at the cursor |
| Alt+Return | terminal (foot) |
| Alt+Shift+Return | the browser (firefox-esr; Omarchy has no key for it) |
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
| Alt+Ctrl+P | power menu: lock, suspend, hibernate, log out, reboot, shut down |
| Alt+Escape | the same power menu (Omarchy's Super+Escape) |
| Alt+Ctrl+L | lock the screen (the Lock key does the same and then suspends) |
| Alt+Ctrl+X | dictation on/off (the notification stays while it records; press again to stop) |
| Alt+Ctrl+Shift+A | agents (launch, accounts, install) |
| Alt+Ctrl+O | settings: Wi-Fi and Bluetooth radios, night light, do-not-disturb, touchpad, tablet mode, rotation |
| Alt+Ctrl+N | notifications: history, do-not-disturb, copy |
| Alt+Ctrl+Q | calculator |
| Alt+Ctrl+V | clipboard history (Enter copies back, Alt+1 deletes the entry) |
| Alt+Ctrl+E | emoji picker (Enter inserts, Alt+1 copies only) |
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
| Alt+Shift+S / Alt+Shift+F / Alt+Ctrl+S | screenshot area / full screen / copy area (each one says so in a notification: the file, or the clipboard) |
| Alt+Ctrl+C | capture menu: screenshot, annotate, OCR, QR code, record the screen |

Dictation (`Alt+Ctrl+X`) records up to 60 seconds and types what it heard into
the focused window. It says what it is doing: the notification stays up while it
records ("Alt+Ctrl+X stops it"), becomes "Transcribing…" while whisper works, and
turns into the text it typed - or the reason it could not. Pressing the key again
**stops and transcribes**, and the root menu's **Dictation** row does the same
from the mouse. The text is also left on the clipboard, because a few surfaces
(a launcher, a fullscreen game) refuse typed input.

The top row of this keyboard sends action keys, not F1-F12: keyd turns
**Ctrl+Alt+top row** into virtual terminals 1-10 and **Search+top row** into
F1-F10. Without keyd there would be no way to reach a VT on this machine.

## Tablet mode

Fold the keyboard all the way back and the machine becomes a tablet: the
on-screen keyboard comes up over the bottom of the screen, the touchpad is
turned off (it is on the back now), and the screen turns with the device. The
rotation is read from the accelerometer once a second and only while the
keyboard is detached, so nothing about it runs in laptop mode. Fold it back and
all three undo themselves, with the screen upright again.

| Action | Does |
|---|---|
| the keyboard folded back | on-screen keyboard up, touchpad off, the screen follows the device |
| the keyboard back in place | keyboard gone, touchpad back, screen upright |
| Settings → Tablet mode | the same switch from the mouse, when the keyboard is already detached and the mode did not follow |
| Settings → Rotation | turn the screen by hand, four positions (this is not tablet mode, and it stays) |

The keyboard is `wvkbd` drawn in the session's palette and font. Its letters
layer is the mobile-international one, so accented characters are one **Cmp**
away: `Cmp` then `a` opens that key's variants (`á`, `à`, `â`, `ä`…) and the one
you tap is what gets typed; `Cmp` then `?` is how `¿` is reached. It types into
the focused window through the virtual-keyboard protocol, which is why it works
in foot, Firefox and the rofi prompts alike.

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
| clipboard history | Super+Ctrl+V | **Alt+Ctrl+V** |
| emoji picker | Super+Ctrl+E | **Alt+Ctrl+E** |
| dictation | Super+Ctrl+X | Alt+Ctrl+X |
| agents | Super+Shift+Ctrl+A | Alt+Ctrl+Shift+A |
| floating | Super+T | Alt+T |
| close window | Super+Q (or Super+W) | Alt+Q (Alt+W is sway's tabbed layout) |
| scratchpad | Super+grave | Alt+grave |
| lock | Super+Ctrl+L | Alt+Ctrl+L (and the Lock key) |
| system menu | Super+Escape | Alt+Escape → the power menu |
| audio / Bluetooth / network panels | Super+Ctrl+A/B/W | Alt+Ctrl+A/B/W |
| power menu | Super+Ctrl+P | **Alt+Ctrl+P** |
| browser | no key (its launcher) | **Alt+Shift+Return** (firefox-esr) |
| screenshot | Super+Shift+S, Print | Alt+Shift+S, Alt+Shift+F, Alt+Ctrl+S |
| capture menu (screenshot, OCR, QR, recording) | Super+Ctrl+C | **Alt+Ctrl+C** |
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

Clipboard history (`Alt+Ctrl+V`) and emoji (`Alt+Ctrl+E`) use the same letters as
Omarchy, and both are built: the history is `cliphist`, fed by `wl-paste --watch`
— the one resident process this session has, about 2 MB — and the picker is a
rofi list over the table `scripts/make-emojis.sh` generates, inserting with the
clipboard plus a virtual Shift+Insert. The capture menu `Alt+Ctrl+C` is built too
(screenshots, annotation, OCR, QR and recording). `docs/omarchy-gap.md` has the
full list.

Everything else lives in the root menu instead of on a key: Places, Media, Time
zone, Free space, End a process, SSH, DNS, Packages, Colour, Reminders, About and
the manual. `Alt+Space` then the first letter is usually enough.

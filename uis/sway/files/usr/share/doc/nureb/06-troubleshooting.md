# When something is wrong

Start with the report; it is faster than reading logs:

```sh
check-system.sh ~/Dev/Nureb
journalctl -b -p err --no-pager | tail -50
```

`FAIL` is a regression of something this port already fixed. `WARN` is usually
information (a regulatory domain, a battery gauge that reports "unknown"). Keep
the whole output when you ask for help: it contains the kernel, the firmware
state, the partitions, the session and the package check in one file.

## No sound, or a "Dummy Output" sink

The MT8192 sound card (`mt8192_mt6359_rt1015p_rt5682`) only works after its
**UCM boot sequence** has run: it connects the front-end and back-end DAIs, and
alsa-lib never runs it by itself. `alsactl restore` does, from udev on card
insert. Check:

```sh
pactl list cards short        # the card must show profile HiFi
wpctl status                  # a "Built-in Audio Speaker" sink, not "Dummy Output"
sudo alsactl restore          # run the boot sequence by hand
```

If the card is there but the profile is off, something removed `alsa-utils` or
`alsa-ucm-conf`; `apt install` them back.

**ALSA-only programs** (aplay without `-D`, emulators, some Java games) need
`pipewire-alsa`; without it they fail with *audio open error: No such file or
directory* while everything else works.

## Wi-Fi

```sh
nmcli device wifi list
nmtui                     # the full editor
```

`country 00` (world regulatory domain) is normal until NetworkManager or the
access point sets one; it is not a failure. A saved profile that never connects
is usually bound to an interface name (`interface-name=wlan0`) - the profiles
this image imports have that line stripped on purpose.

## Bluetooth

```sh
bluetoothctl show         # Powered: yes
btmgmt info
```

The controller (`hci0`) is attached to `serial0-0` and needs its RTL8822CS
firmware; if `bluetoothctl show` says *Powered: no*, check
`journalctl -u bluetooth` and that the `bluez` package is installed.

## The screen stays black after a kernel update

The firmware boots `p1`, not `/boot`. If the machine comes back to the firmware
menu, or boots the old kernel, the kernel partition was not rewritten:

```sh
sudo ~/Dev/Nureb/scripts/install-kpart.sh
```

To see the boot messages (they are quiet by default), build a verbose kpart once:

```sh
KERNEL_CMDLINE_OVERRIDE="console=tty0 ignore_loglevel loglevel=7" \
  sudo -E ~/Dev/Nureb/scripts/make-kpart.sh
```

Never append a bare `debug`: initramfs-tools then sends its own output to a file
inside the initramfs, and the messages that explain a failure never reach the
screen.

## The launcher opens empty, or shows an error dialog

* An error dialog naming a file, line and column is a **rofi theme parse error**;
  run `rofi -show drun` in a terminal after editing `~/.config/rofi/*.rasi`.
* An empty window means the list was empty - check that `/usr/share/applications`
  has entries and that `~/.config/rofi/config.rasi` exists.
* A bar click that does nothing: `~/.config/bin/bar-pick` must be executable.
  The same goes for every row of the root menu: each one is a `rofi-*` script in
  `~/.config/bin/`, and a missing script is a row that silently does nothing.
  `check-system.sh` fails for each one.
* **A hole where an icon should be.** The launcher draws application *images*,
  and the session's own entries take theirs from the small **Nureb** theme
  (`/usr/share/icons/Nureb`, 48x48 PNGs rendered from the icon fonts). If the
  theme is missing, rebuild it with `sudo ~/Dev/Nureb/scripts/make-icons.sh` - it
  needs ImageMagick - or reinstall the image. An application you installed that
  shows no icon is a packaging problem of that application, not this one.

## The manual opens a window that closes at once

`Learn` runs `glow -p <page>` in foot, and glow exits immediately when its style
is missing or unparsable. The style is named by **absolute path** in
`~/.config/glow/glow.yml` (a bare name is looked up among glow's built-ins), and
the file it points at is `~/.config/glow/solarized.json`. To see the error:

```sh
glow /usr/share/doc/nureb/01-welcome.md    # the message names the problem
```

## A file does not open, or opens in the wrong place

The file browser reads text with `bat` and hands everything else to
**firefox-esr**; `xdg-open` is a two-dozen-line shim around the same binary. If a
file only produces a notification, the browser is gone:

```sh
command -v firefox-esr || sudo apt install firefox-esr
```

## The bar shows a spinner that never ends

That card belongs to `~/.config/bin/bar-busy`: one file per running task lives in
`$XDG_RUNTIME_DIR/nureb-busy`, and a task that is killed without `busy end`
leaves one behind. They expire after five minutes on their own; to clear them at
once:

```sh
rm -rf "$XDG_RUNTIME_DIR/nureb-busy"/* && pkill -RTMIN+8 waybar
```

## No notifications

mako owns `org.freedesktop.Notifications`. If it is not running, notifications
disappear with no log line:

```sh
busctl --user list | grep Notifications
pgrep -a mako || exec mako
```

## Icons render as boxes

Terminals do not use fontconfig fallback: foot lists its fonts explicitly. The
shipped `~/.config/foot/foot.ini` names `Symbols Nerd Font Mono` after JetBrains
Mono, which is what makes eza's icons, faststart and the starship glyphs render.
If you replace the font line, keep a symbol font in the list.

## A terminal starts with "not a UTF-8 locale"

```
warning: foot: 'C' is not a UTF-8 locale, falling back to 'C.UTF-8'
```

The locale is installed but the **session** did not get it. greetd starts the
session with the PAM environment only, so `/etc/pam.d/greetd` needs the two
`pam_env.so` lines that `/etc/pam.d/login` uses (the image ships them; a
hand-built system may not):

```
session	required	pam_env.so readenv=1
session	required	pam_env.so readenv=1 envfile=/etc/default/locale
```

Log out and back in afterwards. `check-system.sh` reports the session locale, so
this is one line in the report rather than a warning you have to notice.

## The brightness keys do nothing

`brightnessctl` needs write access to `/sys/class/backlight/*/brightness`. The
image ships a udev rule (group `video`, `g+w`); after changing it,
`sudo udevadm control --reload && sudo udevadm trigger`.

## A key does nothing at all

keyd grabs the keyboard, and it reads its configuration only at start:

```sh
sudo keyd.rvaiya reload       # after editing /etc/keyd/default.conf
```

`backspace+escape+enter` makes keyd quit and release the keyboard if a mapping
goes wrong, which is the way back to a working top row.

## Nothing responds, the machine is thrashing

4 GB and a compressed swap: a runaway process can make everything crawl before
the kernel kills it. `systemd-oomd` is the guard (it kills a hog at 50 % memory
pressure). To see who is responsible:

```sh
ps -eo pid,rss,comm --sort=-rss | head
cat /proc/pressure/memory
```

`Alt+Shift+E` (exit the session) or a VT (Ctrl+Alt+top row with keyd) is the way
out when the session itself is stuck.

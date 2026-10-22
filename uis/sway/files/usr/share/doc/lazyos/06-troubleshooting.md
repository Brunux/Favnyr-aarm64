# When something is wrong

Start with the report; it is faster than reading logs:

```sh
check-system.sh ~/Dev/LazyOS
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

## DNS: what a name resolves to

The resolver is NetworkManager's (`/etc/resolv.conf` is written from the active
connection), so **there is no `resolvectl`** here: systemd-resolved is not
installed and asking it answers *command not found*, which is not a fault. The
two questions, and the tools that answer them:

```sh
dig example.com +short          # what the current server answers (bind9-dnsutils)
dig @1.1.1.1 example.com        # what a specific server answers
nmcli -f ipv4.dns,ipv4.ignore-auto-dns connection show "$(nmcli -g GENERAL.CONNECTION device show wlp1s0)"
```

The DNS menu (`Alt+Space` → **DNS**) writes the first two properties and brings
the connection back up; the profile holds the same values afterwards, so reading
them back is the check, not `resolv.conf` (which is regenerated).

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
sudo ~/Dev/LazyOS/scripts/install-kpart.sh
```

To see the boot messages (they are quiet by default), build a verbose kpart once:

```sh
KERNEL_CMDLINE_OVERRIDE="console=tty0 ignore_loglevel loglevel=7" \
  sudo -E ~/Dev/LazyOS/scripts/make-kpart.sh
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
  and the session's own entries take theirs from the small **LazyOS** theme
  (`/usr/share/icons/LazyOS`, 48x48 PNGs rendered from the icon fonts). If the
  theme is missing, rebuild it with `sudo ~/Dev/LazyOS/scripts/make-icons.sh` - it
  needs ImageMagick - or reinstall the image. An application you installed that
  shows no icon is a packaging problem of that application, not this one.

## The manual opens a window that closes at once

`Learn` runs `glow -p <page>` in foot, and glow exits immediately when its style
is missing or unparsable. The style is named by **absolute path** in
`~/.config/glow/glow.yml` (a bare name is looked up among glow's built-ins), and
the file it points at is `~/.config/glow/solarized.json`. To see the error:

```sh
glow /usr/share/doc/lazyos/01-welcome.md    # the message names the problem
```

## A file does not open, or opens in the wrong place

The file browser picks the program by what the file is: text goes to `bat`, a
document (PDF, DjVu, a comic) to **papers**, a picture to **imv** (or `chafa`,
drawn in a foot window, when imv cannot get a graphics context), a video to
**mpv**, and an EPUB, a `.deb` or anything else to **firefox-esr**; `xdg-open`
is a two-dozen-line shim that sends a URL to the browser and a path to the same
rules. If a file only produces a notification, the program is gone:

```sh
command -v firefox-esr papers imv mpv chafa bat
```

A video that opens in Firefox instead of mpv means mpv is missing; a document
that opens in the browser means `papers` is; a picture that opens in the browser
means both `imv` and `chafa` are.

## The bar shows a spinner that never ends

That card belongs to `~/.config/bin/bar-busy`: one file per running task lives in
`$XDG_RUNTIME_DIR/lazyos-busy`, and a task that is killed without `busy end`
leaves one behind. They expire after five minutes on their own; to clear them at
once:

```sh
rm -rf "$XDG_RUNTIME_DIR/lazyos-busy"/* && pkill -RTMIN+8 waybar
```

## No notifications

mako owns `org.freedesktop.Notifications`. If it is not running, notifications
disappear with no log line:

```sh
busctl --user list | grep Notifications
pgrep -a mako || exec mako
```

A card **without its picture** is a different fault with no log line either: the
notification arrived, but mako could not resolve the icon name the sender gave
it. mako reads no icon-theme metadata - it walks the roots named by
`icon-path` in `~/.config/mako/config`, which is where the session's own
`/usr/share/icons/LazyOS` has to be. `check-system.sh` reports both halves
(`icons=1` and an existing `icon-path`), and the names themselves are rendered by
`scripts/make-icons.sh` on the build host.

## Icons render as boxes

Terminals do not use fontconfig fallback: foot lists its fonts explicitly. The
shipped `~/.config/foot/foot.ini` names `Symbols Nerd Font Mono` after JetBrains
Mono, which is what makes eza's icons, faststart and the starship glyphs render.
If you replace the font line, keep a symbol font in the list.

Emoji are a second, separate font: `fonts-noto-color-emoji` is what draws them,
and without it an inserted emoji is a tofu box in the same way an icon without a
symbol font is. `fc-match emoji` names the font that would be used, and
`check-system.sh` fails when there is none.

## The clipboard history is empty

The list is only as full as the watcher has made it:

```sh
pgrep -fx 'wl-paste --watch cliphist store' || echo 'the watcher is not running'
cliphist list | head
```

The watcher is one line in `etc/sway/config.d/27-clipboard.conf`, so it starts
with the session; a session older than that line has no history. The list also
says so itself when `cliphist` is missing. `Alt+Ctrl+V` → **Clear the history**
empties it, and the database is `~/.cache/cliphist/db`.

## Tablet mode: no keyboard, or the screen does not turn

Detaching the keyboard is what turns tablet mode on: `cros_ec_buttons` reports
`SW_TABLET_MODE` and sway answers with `bindswitch tablet:on/off` from
`etc/sway/config.d/28-tablet.conf`, which runs `~/.config/bin/tablet-mode`. So
the first question is whether the switch arrived:

```sh
grep -A5 cros_ec_buttons /proc/bus/input/devices   # SW=3: lid + tablet
test -x ~/.config/bin/tablet-mode && echo 'the script is there'
~/.config/bin/tablet-mode status                     # on · screen normal | off
```

The mode can be switched by hand - `Settings` → **Tablet mode**, or
`~/.config/bin/tablet-mode on` - which is what to use when the switch did not
arrive but the screen is folded. If the keyboard comes up and the screen stays
sideways, the accelerometer is the missing half: the poller reads
`/sys/bus/iio/devices/iio:device*/in_accel_*_raw` of the `cros-ec-accel` device,
and `check-system.sh` reports its absence as a FAIL.

**The screen is upside down, or turns the wrong way.** The built-in mapping is
one guess about how the panel's accelerometer is mounted, and units differ.
Hold the card upright in your hands, screen facing you, in tablet mode, and run
`~/.config/bin/tablet-mode calibrate` (or **Settings → Calibrate rotation**, which
is only listed while the mode is on): it reads the sensor there and stores
`normal=<axis><sign>` in `~/.config/lazyos/tablet.conf`, one line, and the four
positions are derived from it at once. `~/.config/bin/tablet-mode calibration`
says `calibrated y-` or `default y-`; deleting the file goes back to the built-in
mapping, and `check-system.sh` reports a calibration file that does not parse,
because `orient()` ignores it silently. Calibrating while the device lies flat is
refused on purpose: flat says nothing about which way is up. The four
`transform` values in `orient()` are the mapping itself - see the table at the
top of `~/.config/bin/tablet-mode`.

**The physical keyboard still types in tablet mode.** It should not: the mode
switches the typing keyboards off one identifier at a time, after the on-screen
keyboard has started. `swaymsg -t get_inputs -r` lists them (`cros_ec`, and
keyd's virtual keyboard), and `~/.config/bin/tablet-mode status` says whether the
mode is on. If the keyboard is off and the on-screen one never appeared,
`tablet-mode off` (or folding the keyboard back) restores it, and the reason is
in what wvkbd printed - usually no Wayland session for that process.

**Folding the keyboard back does nothing** (the mode stays on, the on-screen
keyboard stays up). The tablet switch and the volume/power keys are one libinput
device, `cros_ec_buttons`, so a mode that disables every `type:keyboard` device
suspends the switch with the keys and never sees `tablet:off` - that was the
2026-10-22 card. `swaymsg -t get_inputs -r` must list `cros_ec_buttons` as a
`switch` with `"send_events": "enabled"`, tablet mode on or off; the script
leaves every identifier that also appears as a switch alone for exactly this
reason, and `check-system.sh` FAILs when a switch is suspended. To leave the mode
by hand, **Settings → Tablet mode** or `~/.config/bin/tablet-mode off` (the
on-screen keyboard can type it).

Nothing about this runs in laptop mode: one process for the keyboard and one
sleeping for a second at a time for the rotation, both gone when the keyboard is
back in place.



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

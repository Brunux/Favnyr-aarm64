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

## Is the network up?

The three commands every answer starts with are installed (`iproute2` and
`iputils-ping` are in the base list; Debian's base system does not pull them in):

```sh
ip -br addr                     # the interface and its address
ip route                        # the default gateway
ping -c3 1.1.1.1                # the link and the gateway, without DNS
ping -c3 deb.debian.org         # DNS and the internet
ss -tunlp                       # what is listening, and on which port
```

`ping` answering "Destination Host Unreachable" is the local link; the first
`ping` working and the second failing is DNS (next section). `ss -tunlp` is also
how to confirm that `ssh` is listening on port 22 (see `05-system.md` § *Remote
login*).

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

## The camera shows nothing, or an app cannot find it

**Check the device first, the program second.**

```sh
lsusb | grep -i cam                        # 0408:30d5 ... UVC WebCam
sudo dmesg | grep -i uvc                   # uvcvideo: Found UVC 1.10 device ...
ffmpeg -f v4l2 -i /dev/video0 -frames:v 1 /tmp/cam.png
```

If that PNG is a picture of what is in front of the tablet, the camera and its
driver are fine and the problem is whatever is displaying it. `ffmpeg -f v4l2
-list_formats all -i /dev/video0` shows what the device offers (MJPEG and YUYV,
1280x720 down to 160x120 here).

**The Camera row opens nothing.** guvcview is missing (`sudo apt install
guvcview`, 4.7 MB with its libraries) or `~/.config/bin/lazyos-camera` lost its
executable bit (`chmod 755`). `check-system.sh` FAILs on both, on a `/dev/video*`
node that no `uvcvideo` is bound to, and on the launcher entry or its icon.

**The window is open and everything is black.** Light, then exposure: the
**Image Controls** window has brightness, gain and exposure, and auto exposure
needs a second or two after the window appears, so the first frame is darker than
the second. If the `ffmpeg` frame above is bright and the window is not, move the
exposure slider; if the frame is dark too, the room is dark.

**There is no `/dev/video0` at all.** The camera is on the internal USB hub at
`1-1.4`. `lsusb` listing it while `/dev/video*` stays empty means the module did
not bind: `sudo modprobe uvcvideo`, then `sudo dmesg | tail`. `check-system.sh`
prints `FAIL uvcvideo is loaded but no /dev/video* node is bound to it` in
exactly that state. A node that exists but answers `Permission denied` means
your user is not in the `video` group: `id`, then `sudo usermod -aG video $USER`
and log in again.

**Nothing can use the camera while something else does.** The device opens
exclusively: a second program gets a black preview or an error, which looks like
"the camera stopped working" until the first one is closed - the guvcview window
(**Alt+Q**), or the browser tab that holds it.

**A Flatpak application sees no camera.** It asks
`org.freedesktop.portal.Camera`, and this image has no portal backend for it -
the journal says *The camera portal requires an access impl* once per session.
Native programs (guvcview, Firefox, `ffmpeg`) read the device directly and are
not affected. `04-software.md` states the limit; the portal is not going to
appear by itself.

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

## A GTK4 window is white

`papers` is GTK4/libadwaita, which does not read `~/.config/gtk-3.0/` at all: it
asks the **settings portal** for the desktop's colour scheme, and this image runs
no Settings portal (the only backend is the wlr one, for screenshots and screen
casting). With no answer libadwaita uses its light default, so the document
reader was a white window in a base03 session. The fix is the pair of files
`~/.config/gtk-4.0/settings.ini` (the dark preference) and
`~/.config/gtk-4.0/gtk.css` (the solarized named colours); `check-system.sh`
reports both. The other answer would be `xdg-desktop-portal-gtk`, a resident
daemon this port does not carry for one boolean.

**"Print" says it cannot find a print service**: the image ships no `cups` - it
is a system service, not a viewer. `sudo apt install cups` and add a queue once if
you print.

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

**Text in another script** is the same class of box and a different font.
`fonts-noto-core` covers Arabic, Hebrew, Devanagari and Thai; Chinese, Japanese
and Korean are a documented `apt install fonts-noto-cjk` (91 MB, deliberately not
in the image). Ask fontconfig, not the application:

```sh
fc-list :lang=ar               # anything at all = the script has a face
fc-list :lang=ja               # empty = install fonts-noto-cjk
```

`check-system.sh` reports `fc-list :lang=ar` in one line.

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

## The translation says the engine refused

`Alt+Ctrl+T` asks two free endpoints in order: Google's JSON one, then MyMemory
without a key. The card *No answer from the translator: is the network up?*
means neither answered, which is most often exactly that - `ping -c1 1.1.1.1`
settles it. Nothing is cached, so a translation that failed is not remembered as
failed: press the key again once the network is back.

A title ending in **(MyMemory)** means Google was unreachable and the fallback
answered. MyMemory cannot detect the source language, so it was told it is the
one `fallback_source=` names in `~/.config/lazyos/translate.conf`, or the
opposite of the target when that key is absent - a Spanish selection translated
into English is the default guess. A wrong guess gives a wrong translation, and
naming the language fixes it:

```sh
printf 'fallback_source=es\n' >> ~/.config/lazyos/translate.conf
```

**Already English** (or whatever the target is) means the detected language was
already the target, so there was nothing to translate; **Settings → Translate**
sets a different one. The whole translation is always on the clipboard
(`wl-paste -n` in a terminal), and when the card was only a preview the read
window has the rest - the card says so by opening it.

The key runs `~/.config/bin/translate` once and exits, so "it stopped working" is
never a daemon to restart. `check-system.sh` reports `curl`, `jq` and `wl-paste`,
SKIPs the network probe when the machine is offline, and FAILs when the binding
or the Settings row has gone missing.

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

## The volume or brightness keys change the value but show nothing

The overlay is a mako card drawn by `~/.config/bin/osd`: the key runs the helper,
the helper changes the value and sends the notification (icon plus the level as
a bar). Three things remove it, each silently:

* the helper lost its executable bit - `ls -l ~/.config/bin/osd` must show `x`,
  and a rebuild puts the mode back (the image stages files with the mode they
  were committed with);
* a notification daemon is not running - `Alt+Ctrl+N` (notifications) is empty
  and `check-system.sh` says no one owns `org.freedesktop.Notifications`; mako
  starts with the session from `config.d/20-notifications.conf`;
* `~/.config/mako/config` lost `icons=1` or its `progress-color`, in which case
  the card is text only, with no picture and no bar.

The value is still changed either way: `wpctl status` and
`cat /sys/class/backlight/backlight-lcd0/brightness` are the sources.

## The screen turns off while you are watching something

A fullscreen video in Firefox or the reader, and any video in mpv, ask the
compositor to keep the screen awake; the rules are
`/etc/sway/config.d/32-idle-inhibit.conf`. If a film is interrupted:

* it is **not** fullscreen (the Firefox and papers rules apply to a fullscreen
  window; fullscreen it, or turn on the bar's coffee cup);
* the application is running under Xwayland (`MOZ_ENABLE_WAYLAND=0`), where it
  has no Wayland app id and no rule can match it;
* a Firefox page playing in a background tab does not inhibit, by design - the
  page that is not on screen should not hold the machine awake.

The **coffee cup** on the bar is the manual override (yellow = kept awake) and
works regardless of what the window is.

## The screen stays lit after the lock

The chain is 150 s idle screen, 300 s lock and **360 s panel off** (DPMS), in
`/etc/sway/config.d/23-lazyos-screensaver.conf`. If the lock appears but the
panel stays lit, `swayidle` is the process to ask:

```sh
pgrep -af swayidle
swaymsg -t get_outputs | grep -i power
```

A `timeout 360 'swaymsg "output * power off"'` line that is missing is the whole
fault - the panel is not the compositor's to turn off otherwise. The first input
turns it back on (`resume`), and `after-resume` does the same after a suspend.

## The brightness keys do nothing

`brightnessctl` needs write access to `/sys/class/backlight/*/brightness`. The
image ships a udev rule (group `video`, `g+w`); after changing it,
`sudo udevadm control --reload && sudo udevadm trigger`.

## The screen is being shared but the viewer shows nothing

The paths in order, because each one fails in its own way:

1. **Is it on?** `rofi-vnc status` answers `off` or
   `localhost:5900 · read-only`. If it says off, Settings → **Share this
   screen**. If the row says *wayvnc is not installed*, that is the package:
   `sudo apt install wayvnc` (a rebuilt image has it).
2. **Did it start?** A failure is a notification quoting wayvnc's last two
   lines, and the log is `$XDG_RUNTIME_DIR/lazyos-vnc/wayvnc.log`. The usual
   cause is the port: something else already listens on 5900 —
   `ss -ltnp | grep 5900`. Stop that server, then start this one again.
3. **It only listens on the loopback address**, on purpose. `ss -ltn | grep
   5900` must show `127.0.0.1:5900` and not `0.0.0.0`. If it shows the wildcard,
   the script was edited: that is an unauthenticated desktop on the network, and
   `check-system.sh` reports it as a FAIL.
4. **The tunnel has to be open.** On the other machine:

   ```sh
   ssh -L 5900:127.0.0.1:5900 brunux@lazyos
   ```

   and leave that session open while you watch. The viewer connects to
   `localhost:5900` **on your machine** — not to `lazyos:5900`, which is closed
   from the network by design. Over a slow link, raise the compression in the
   viewer rather than the frame rate.
5. **You cannot type, click or scroll.** That is `--disable-input`, and it is
   the feature: the viewer looks. To act on this machine, use the ssh session
   you already have open.
6. **The bar's eye is missing while it is shared.** The module is `custom/vnc`
   in `~/.config/waybar/config.jsonc`; force the poll with
   `pkill -RTMIN+8 waybar`. The indicator is a state file being read, so it
   follows `rofi-vnc status`, and a wayvnc started by hand (without the menu)
   has no state file — the failure `check-system.sh` warns about.

There is no audio over VNC, and nothing to configure: the sound stays on this
machine.

## A key does nothing at all

keyd grabs the keyboard, and it reads its configuration only at start:

```sh
sudo keyd.rvaiya reload       # after editing /etc/keyd/default.conf
```

`backspace+escape+enter` makes keyd quit and release the keyboard if a mapping
goes wrong, which is the way back to a working top row.

## The red bar says "There are errors in your config file"

That bar is sway's, and **Toggle details** in it prints the file, the line and
what it did not understand. Two things cause it here:

* **a real syntax error** in `~/.config/sway/config`, in
  `~/.config/sway/config.d/*.conf` or in `/etc/sway/config.d/*`. The same check
  without a running session is
  `sway -C -c ~/.config/sway/config`, which prints `Error on line N ...`.
* **the same gesture defined twice**, once in a system drop-in and once in your
  own. A repeated `bindsym` is a silent override; a repeated `bindgesture` is
  not, and it is counted as an error on `swaymsg reload` while `sway -C` and a
  fresh start both accept the very same files - so the swipe still works and the
  bar is the whole symptom. Measured with the three-finger bindings in
  `/etc/sway/config.d/22-workspace-gestures.conf`: a diagnostic copy of
  `swipe:3:up` in `~/.config/sway/config.d/` raised the bar on every reload.

Delete the duplicate from your drop-in and edit the system file instead, then:

```sh
swaymsg reload
```

`check-system.sh` warns when a user drop-in repeats a system `bindgesture`.

## The three-finger swipes do nothing up or down

That is the design in every window that is not Firefox: the binding carries
sway's own filter, `[app_id="(?i)^firefox" con_id="__focused__"]`, so the
compositor checks the focused window and starts nothing otherwise. In foot, in
an editor, on the desktop - nothing happens, and nothing is supposed to.

Inside Firefox it should still move a tab. If it does not:

```sh
command -v wtype
swaymsg -t get_tree | jq -r '.. | objects | select(.focused==true) | .app_id'
```

`wtype` is the package of that name (`check-system.sh` FAILs when it is gone).
The second line names the focused window's Wayland app id; it must start with
`firefox` (Debian's package reports `firefox-esr`). A Firefox started with
`MOZ_ENABLE_WAYLAND=0` runs under Xwayland, has no app id at all, and gets no
gesture - start it normally.

To let the gesture work in more applications, widen that one regex in
`/etc/sway/config.d/22-workspace-gestures.conf` instead of adding a second
binding (`(?i)^(firefox|code)` and so on), then `swaymsg reload`: two
`bindgesture` lines for the same swipe are the duplicate that raises the red
bar above.

## Auxiliary swap or Format a disk says the tools are missing

The menu answered `Missing: parted mkswap swapon swapoff blkid` while every one of
them was installed. The reason is the session's `PATH`: greetd starts sway with
the PAM environment, and Debian keeps `/usr/sbin` and `/sbin` for root, so a
`command -v` inside a menu found none of them. Both disk menus source
`~/.config/bin/lib-rofi.sh`, and that file appends `/usr/sbin:/sbin` once for
every menu - one line instead of a check in each script:

```sh
. ~/.config/bin/lib-rofi.sh
command -v mkswap          # /usr/sbin/mkswap after that line, nothing before
```

`check-system.sh` asks the same question in a session-shaped `PATH` and fails
while the line is gone. An image built before it needs a rebuild; until then the
tools still work from a terminal with their full path (`/usr/sbin/parted`), and
the menu's own rows are the part that cannot see them.

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

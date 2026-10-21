# Hardware test checklist

What to try on a booted card, one item per feature that has not run on the
device yet, with the exact behaviour that means it works. `scripts/check-system.sh`
covers everything a script can see; this file is the half only a person can
judge - whether the glyph appears in the cursor, whether the screen turns the
right way, whether the notification showed up in time.

Run it after `check-system.sh`, and write both results to `reports/` (see
`reports/README.md`). A **FAIL** from `check-system.sh` is a regression, not a
question: every one of its checks maps to a bug this port has already hit.

## Before anything else

```sh
cat /etc/debian-aarch64-revision      # commit, device, UI, kernel, build date
/usr/local/sbin/check-system.sh ~/Dev/Nureb
```

* the revision must be the build you meant to test. The tree at
  `~/Dev/Nureb` is a snapshot (`git archive HEAD`), not a checkout, so compare
  that hash with `git log --oneline -1` on the machine that wrote the card;
* `check-system.sh` must end with **0 FAIL**. WARN and SKIP lines are expected
  and explained in `reports/README.md`; anything else stops the run here.

## Clipboard history - `Alt+Ctrl+V`

| Step | Expected |
|---|---|
| copy a line in a terminal (`echo hello-paste \| wl-copy`), then `Alt+Ctrl+V` | a row with the text and its kind (`text`) in the dim column |
| Enter on that row, then paste in foot (`Ctrl+Shift+V`) | `hello-paste` |
| copy a region to the clipboard (`Alt+Ctrl+S`), reopen the list | an image row, plus **Save the newest image** |
| Enter on **Save the newest image** | `~/Pictures/clipboard/<date>.<ext>`, opened in Firefox; notification names the file |
| `Alt+1` on a row | "Entry deleted from the history" and the row is gone when the list reopens |
| last row, **Clear the history** | the list says nothing has been copied yet |

An **empty** list is the watcher, not the menu: `pgrep -fx 'wl-paste --watch cliphist store'`
(`etc/sway/config.d/27-clipboard.conf` starts it once per session). An image
that does not open is `xdg-open`/`firefox-esr`.

## Emoji - `Alt+Ctrl+E`

| Step | Expected |
|---|---|
| type `sun` | the list narrows as you type (the row shows the glyph, its name and its group) |
| Enter with foot focused | the glyph is typed at the cursor, **in colour** |
| `Alt+1` on a row | "… copied to the clipboard", nothing typed |
| Enter with an XWayland or Java window focused | the notification says it was copied because the window takes no virtual keyboard |

Tofu boxes instead of glyphs mean `fonts-noto-color-emoji` is missing (a FAIL in
`check-system.sh`); an empty list means `~/.config/emojis.tsv` is missing
(regenerate it with `scripts/make-emojis.sh` on the build host).

## DNS - root menu (`Alt+Space`) -> **DNS**

| Step | Expected |
|---|---|
| open it | the servers in use, marked `in use`, plus **Automatic (the network)** and **Type the servers** |
| choose **Type the servers**, enter `1.1.1.1 9.9.9.9` | the busy card, then a notification; reopening shows them `in use` |
| `resolvectl status wlp1s0` (or `nmcli device show`) | the same servers |
| choose **Automatic (the network)** | back to what DHCP hands out |

Changing a connection needs root: without a passwordless `sudo` the menu says so
and changes nothing, which is the intended behaviour.

## Packages - root menu -> **Packages**

| Step | Expected |
|---|---|
| type `htop` | package names with their one-line description |
| Enter on `htop` | busy card, then the result; `htop` exists afterwards |
| search a package that is installed | the row is dim and says `installed` |
| Escape | nothing is installed |
| **Update the package lists** (last row) | `apt-get update` runs behind the same card, then the search repeats |

## Colour - root menu -> **Colour**

| Step | Expected |
|---|---|
| Enter on **Pick a pixel**, click anywhere | a notification with the hex, and `wl-paste` holds it |
| Enter on one of the Solarized swatch rows | the hex is copied (the launcher lists them by name) |

## Reminders - root menu -> **Reminders**

| Step | Expected |
|---|---|
| **New reminder**, type `tea`, choose **In 5 minutes** | a notification stating when systemd will fire it, in systemd's own words |
| reopen the list | the reminder with "in 4 min · HH:MM · Enter cancels" |
| Enter on it | "Cancelled …" and the row is gone |
| **At 09:00** | the notification names a **concrete date**, and it does not repeat the next morning |
| `systemctl --user list-timers` | the pending reminder is there (nothing on disk: `systemd-run --user --collect`) |

Reminders die with the session's user manager, which is the design.

## About - root menu -> **About**

| Step | Expected |
|---|---|
| open it | model, revision, kernel, uptime, memory, disk, battery, network, session, processes |
| **Copy this summary** | the same lines are on the clipboard |
| **The full picture** | fastfetch in a foot window |

## Files and thumbnails - `Alt+Ctrl+F`

| Step | Expected |
|---|---|
| open `~/Pictures` | rows with a **32 px** thumbnail for images, videos and PDFs; the icon is the picture, not a mimetype glyph |
| reopen the same directory | instant: the thumbnails come from `~/.cache/thumbnails/normal/` and are not decoded again |
| Enter on an image | a foot window drawing it with sixels (chafa), not Firefox |
| Enter on a PDF | Firefox |
| Enter on a text file | `bat` in the popup window (`BAT_THEME=ansi`, solarized) |
| spam `Alt+Ctrl+F` on a directory of large photos | the application keeps responding while the rows fill in (four decoders at a time) |

A list with no pictures at all means `-show-icons`/`thumbnail://` was dropped from
`rofi-files`, or `nureb-thumbnail` is not executable; `check-system.sh` warns
about both.

## Tablet mode

Fold the keyboard all the way back.

| Step | Expected |
|---|---|
| fold it back | the on-screen keyboard appears at the bottom (solarized, `JetBrains Mono`), the touchpad does nothing, one notification: "Tablet mode: the keyboard is up and the screen follows the device" |
| type in foot | letters land in the focused window; `Cmp` then `a` opens `á à â ä …` and tapping one inserts it; `Cmp` then `?` reaches `¿` |
| turn the device 90° clockwise, 90° back, upside down, and lay it flat | the screen follows within about a second, and the flat reading puts it back to `normal` |
| tilt it, do not turn it | the screen does **not** flip |
| fold it back into a laptop | keyboard gone, touchpad back (test the pointer), screen upright, "Tablet mode off" |
| `pgrep -x wvkbd-mobintl` and `pgrep -af 'tablet-mode poller'` afterwards | nothing |
| Settings (`Alt+Ctrl+O`) -> **Tablet mode** | toggles by hand; the row reads `on · screen 90` or `off` |
| Settings -> **Rotation** | still turns the screen by hand, independently |

What a failure means:

* **no keyboard on the fold, but it works by hand** (`~/.config/bin/tablet-mode on`):
  the switch did not arrive. `grep -A5 cros_ec_buttons /proc/bus/input/devices`
  must show `SW=3`, and `bindswitch tablet:` must be in
  `/etc/sway/config.d/28-tablet.conf`.
* **keyboard, no rotation**: the accelerometer (`/sys/bus/iio/devices/iio:device1`,
  `cros-ec-accel`); `check-system.sh` FAILs when it is gone.
* **rotation inverted** (screen turns the wrong way): swap the four `transform`
  values in `orient()` of `~/.config/bin/tablet-mode` and reinstall that one file.
* **keyboard appears but types nowhere**: the focused surface refuses virtual
  keyboards; the emoji picker's notification says the same thing, and pending
  `zwp_virtual_keyboard_manager_v1` support is in wlroots, not wvkbd.

## Regressions this round could have touched

| Step | Expected |
|---|---|
| `Alt+Ctrl+K` (key view) | the Clipboard and Emoji bindings with their descriptions |
| `Alt+Escape` | the power menu (not the battery banner) |
| a slow action (Wi-Fi scan, DNS change) | a mako card that becomes the result, and the bar's spinner while it runs |
| lock (`Alt+Ctrl+L`) and unlock | swaylock, then back to the session, no leftover screensaver |
| shut down from the power menu | the machine powers off (the question is asked once) |

## Writing it down

`reports/check-system-<date>-card.txt` gets the script's output verbatim (MAC
masked to the OUI), and `reports/feedback-<date>-session.txt` gets what only you
saw: which row was wrong, which notification never came, which key did nothing.
Both are what the next round's checks are written from - a FAIL here becomes a
line in `check-system.sh`, not a note in a commit message.

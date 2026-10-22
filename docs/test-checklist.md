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
cat /etc/debian-aarch64-revision ~/Dev/LazyOS/REVISION ~/Dev/LazyOS/VERSION
/usr/local/sbin/check-system.sh ~/Dev/LazyOS
```

* the revision must be the build you meant to test. The tree at
  `~/Dev/LazyOS` is a snapshot (`git archive HEAD`), not a checkout, so compare
  that hash with `git log --oneline -1` on the machine that wrote the card;
* the snapshot and the system must be one build: `~/Dev/LazyOS/REVISION` carries
  the same lines as `/etc/debian-aarch64-revision` (commit, device, UI, kernel,
  version, date), `~/Dev/LazyOS/VERSION` is the `VERSION_ID` in
  `/etc/os-release` and the version in `/etc/issue`, and on the build host
  `git rev-parse v<version>^{commit}` is the commit `REVISION` names. A
  disagreement means the card was assembled from two builds - stop here;
* `check-system.sh` must end with **0 FAIL**. WARN and SKIP lines are expected
  and explained in `reports/README.md`; anything else stops the run here.

## The brand - wallpaper, About, greeter, console

| Step | Expected |
|---|---|
| look at the desktop | the arcade button on base03, `lazy by design` under it and the arcade line at the bottom; the bar, foot, rofi and the menus are still solarized (the brand did not repaint the session) |
| `Alt+Ctrl+L`, then unlock | the **mountain photograph**, not the brand wallpaper - the lock is deliberately the other surface |
| `Alt+d`, type `About LazyOS` | the **About LazyOS** row has the mark as its icon (not a hole); Enter opens the card with the two-line wordmark on the left, `OS · LazyOS <version>` among the facts, and the 16-colour palette at the bottom |
| `cat /etc/os-release` | `ID=lazyos`, `PRETTY_NAME="LazyOS <version>"`, and `VERSION_ID` equal to the `version` line of `/etc/debian-aarch64-revision` |
| `hostname` | `lazyos` |
| log out to the greeter | the greeting is `LazyOS · lazy by design`, not "Welcome back to Debian" |
| `Ctrl+Alt+F2` (keyd's VT keys), log in on the text console | the banner is `LazyOS <version> · lazy by design` and `lazyos · tty2` |

The launcher's **About LazyOS** and the root menu's **About** open the same card;
the launcher entry is the one that carries `Icon=lazyos`. A missing wallpaper is
`Pictures/Wallpapers/lazyos.png`; regenerate it with
`scripts/make-brand.sh` on the build host. A hole where the About row's icon
should be is `lazyos.png` missing from `/usr/share/icons/LazyOS/48x48/apps/`.
Debian's swirl in the About card is `~/.config/fastfetch/config.jsonc` missing.
`check-system.sh` FAILs on the first two, on an `os-release` that is Debian's
again, and on an `org.nureb.*` entry that survived the rename; it WARNs when the
greeter, `/etc/issue` or the fastfetch files lose the brand, because those are a
hand-edit away.

## Clipboard history - `Alt+Ctrl+V`

| Step | Expected |
|---|---|
| copy a line in a terminal (`echo hello-paste \| wl-copy`), then `Alt+Ctrl+V` | a row with the text and its kind (`text`) in the dim column |
| Enter on that row, then paste in foot (`Ctrl+Shift+V`) | `hello-paste` |
| copy a region to the clipboard (`Alt+Ctrl+S`), reopen the list | an image row, plus **Save the newest image** |
| Enter on **Save the newest image** | `~/Pictures/clipboard/clip-<date>.<ext>`, opened in **imv** (a picture is the viewer's job now); the notification names the file. A foot window drawing the picture in sixels is `chafa`'s fallback for a GPU with no EGL context, not a failure |
| `Alt+1` on a row | "Entry deleted from the history" and the row is gone when the list reopens |
| last row, **Clear the history** | the list says nothing has been copied yet |

An **empty** list is the watcher, not the menu: `pgrep -fx 'wl-paste --watch cliphist store'`
(`etc/sway/config.d/27-clipboard.conf` starts it once per session). An image
that does not open is `~/.config/bin/lazyos-view` (not executable) or `imv`; a
saved file ending in `.bin` means `file` is missing and the extension could not
be read from the contents.

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
| `nmcli -f ipv4.dns,ipv4.ignore-auto-dns connection show <name>`, and `iw dev wlp1s0 link` for the interface's own name | the same servers; the interface is `wlp1s0` here (Nura calls it `wlan0`) |
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
| **New reminder**, type `tea`, Enter, leave the note empty, choose **In 5 minutes** | a notification stating when systemd will fire it, in systemd's own words |
| the same flow, but type a note at the second prompt | the note is part of the card the reminder fires later |
| reopen the list | the reminder with its title, and the note (if any) on a second, dimmer line |
| Enter on it | "Cancelled …" and the row is gone |
| **At 09:00** | the notification names a **concrete date**, and it does not repeat the next morning |
| a **repeat** choice (every day, every weekday, every week, every month) | the card names the next occurrence; the reminder stays in the list after it fires |
| type `*-*-* 25:00` where a date is asked for | systemd's own rejection (`Failed to parse calendar specification …`, `Invalid argument`) and the When menu again, title and note still known |
| `systemctl --user list-timers` | the pending reminder is there, and its `Description` carries the title (nothing on disk: `systemd-run --user --collect`) |

Reminders die with the session's user manager, which is the design. A reminder
that is **not** in `list-timers --all` was never created: `systemd-run` printed
the reason, and the menu shows it instead of claiming success.

## About - root menu -> **About**

| Step | Expected |
|---|---|
| open it | model, revision, kernel, uptime, memory, disk, battery, network, session, processes |
| **Copy this summary** | the same lines are on the clipboard |
| **The full picture** | fastfetch in a foot window that **stays open** and ends with `fastfetch has finished · close this window with Ctrl+Shift+W`; a window that flashed for a tenth of a second is `--hold` missing from `rofi-about` |
| **Errors this boot** | `journalctl -p err -b` in a foot window (the same lines the report calls a problem) |
| **Boot time** | `systemd-analyze blame` in a foot window, longest first |
| **Devices** | `lspci` then `lsusb` in a foot window |

## Agents - `Alt+Ctrl+Shift+A`, and the root menu's `ai` group

| Step | Expected |
|---|---|
| open the root menu (`Alt+Space`) | an `ai` heading with **Agents** and **Dictation** together, above `system`; **Agents** opens its menu |
| `Alt+Ctrl+Shift+A` with no agent installed | the five agents, each row reading `Name — install <npm>`, then **Set the default agent…** and **Open a terminal in ~/Work**; Escape leaves nothing running |
| from a terminal, `rofi-agents --new claude`, then the name `work` | `~/.local/share/lazyos/agents/claude/work` exists and is empty, and `~/.local/share/lazyos/agents/claude/default` holds `work` |
| with Claude Code or Codex installed, its row grows **accounts…** | `● work  (default)` is listed; `rofi-agents --env claude work` prints `CLAUDE_CONFIG_DIR` at the directory |
| launch the installed Claude Code row | foot opens in `~/Work` (or `$HOME` when `~/Work` does not exist) with `CLAUDE_CONFIG_DIR` pointing at `agents/claude/work` |

An account is a directory, not a login: the first launch in a new one runs the
CLI's own sign-in, so two subscriptions never share a token. The menu installs
with an explicit `--prefix ~/.local`, so npm never needs root and never touches
`/usr/local`.

## Files and thumbnails - `Alt+Ctrl+F`

| Step | Expected |
|---|---|
| open `~/Pictures` | rows with a **32 px** thumbnail for images, videos and PDFs; the icon is the picture, not a mimetype glyph |
| open a directory of source files or dotfiles | **every** row still has an icon (a page, an executable, a folder), so the names line up in one column - no bare text list |
| reopen the same directory | instant: the thumbnails come from `~/.cache/thumbnails/normal/` and are not decoded again |
| Enter on an image | **imv** in its own window (zoom, pan, rotate; the arrow keys walk the folder). A foot window drawing sixels means imv could not get an EGL context, which is the fallback |
| Enter on an SVG | **imv** as well, not `bat`: an SVG is a text file, but the image family is decided before the text test |
| Enter on a **video** (`.mp4`, `.mkv`) | **mpv** in its own window with sound; `q` closes it. Firefox means mpv is missing |
| Enter on a PDF, a DjVu or a comic | **papers**, with search, zoom and print. Firefox means papers is missing |
| Enter on an EPUB | Firefox: `papers` reads no EPUB |
| Enter on an HTML file | `bat` (it is text), not Firefox |
| Enter on a `.deb` | Firefox, as a download prompt: the browser is what a package is for here |
| Enter on a text file | `bat` in the popup window (`BAT_THEME=ansi`, solarized) |
| open a deep path (`~/Dev/LazyOS/uis/sway/files/home/.config/bin`) | the window is drawn wide enough that the **end** of the name is readable: rofi cannot scroll a row sideways, so `lib-rofi.sh`'s `ui_width` sizes the list instead of letting rofi cut it with `…` |
| spam `Alt+Ctrl+F` on a directory of large photos | the application keeps responding while the rows fill in (four decoders at a time) |
| **Alt+1** on a directory | the actions for it: disk usage, compress, copy the path, a terminal here. Enter still *opens* the row; only Alt+1 asks |
| **Alt+1** → **Compress** on a folder | a `busy` card, then `folder.tar.xz` beside it, and the bar spinner while it packs (click the spinner to stop it) |
| **Alt+1** → **Extract** on that archive | a folder named after the archive. Run it again: it refuses ("already exists") instead of merging into it |
| **Alt+1** on a video → **Convert** → MP3 | `name.mp3` beside it, with a card. On an `.mp4` → MP4 the output is `name-converted.mp4`, never the source |
| **Alt+1** on an `.iso` | it mounts through `udisksctl` and the browser opens where it landed |
| **Alt+1** on the dotfiles row | a notification saying it has nothing to act on, and nothing else happens |

An Alt+1 that switches to workspace 1 means rofi is not inhibiting the
compositor's keys (`global-kb: true` in `~/.config/rofi/config.rasi`); an Alt+1
that opens the row means `-kb-custom-1 Alt+1` was dropped from `rofi-files`.
`check-system.sh` warns about both.

A list with no pictures at all means `-show-icons`/`thumbnail://` was dropped from
`rofi-files`, or `lazyos-thumbnail` is not executable; `check-system.sh` warns
about both. A row with **no** icon in a text-only directory means the theme is
missing that name (`check-system.sh` checks the table against the PNGs).

## Search inside the files - `Alt+Ctrl+G`

| Step | Expected |
|---|---|
| press **Alt+Ctrl+G**, type a word you know is in a file | a `busy` card ("Looking for …") and then a menu of matches, the matching **text** on each row and `path:line` dimmed |
| Enter on a match | the file in `bat`, scrolled to that line and with the line highlighted |
| a word that appears 40 times in one file | at most three rows from that file |
| a word inside `~/.cache` or `node_modules` | not listed (the same four excludes as the other searches) |
| type `a(b` (a broken regex) | a critical card quoting ripgrep's own parse error, not an empty list |
| a word that is nowhere | "Nothing matches" |
| press it again while it is searching | the first card's spinner in the bar can be clicked to stop the run |

## Download - root menu -> **Download**

| Step | Expected |
|---|---|
| Alt+Space → Download → **Paste a link**, a short video URL, **Video** | a `busy` card, then "Saved `<name>.mp4`", and the file in `~/Downloads` |
| the same with **Audio only** | a `.m4a` in `~/Downloads`, no video stream |
| copy a URL first, then open Download | a second row, **the link in the clipboard**, which uses it without a prompt |
| a link to a video inside a playlist | one file, not the playlist |
| a link to something unavailable | a critical card with yt-dlp's own words ("Video unavailable"), not a traceback |
| click the bar's spinner mid-download | the card says the `.part` file is still there, and no half file is presented as finished |

## Format a disk - root menu -> **Places** -> **Format a disk**

| Step | Expected |
|---|---|
| open it with only the eMMC and the boot card attached | "Nothing to format": the eMMC is fixed storage and the card is the disk this system runs from, so neither is offered |
| boot from the card and wipe the eMMC's partition table (`sudo parted -s /dev/mmcblk0 mktable gpt`), then open it | still "Nothing to format": the eMMC is fixed storage and the disk this system depends on, which is the case the kernel-partition rule alone missed; `scripts/install-emmc.sh` is how it is written again |
| insert a USB stick, leave it unmounted | the stick, with its model and size |
| pick it → **Cancel** | nothing at all is written |
| pick it → **exFAT** → the confirmation | a card, then "is empty (exfat, label LAZYOS)"; the stick mounts again from Places |
| the same with **ext4** | `mkfs.ext4`, and the disk mounts on a Linux machine |

Neither menu may ever list `mtdblock0` (the SPI flash) or `mmcblk0boot0`/`mmcblk0boot1`
(the eMMC's hardware boot partitions): `lsblk -d` shows all three as small disks,
none mounted and none carrying a kernel partition, and writing any of them is how
this machine gets bricked or loses its boot blocks. Neither may list the eMMC or the
card the system is running from either, whatever their partition tables say.

## Auxiliary swap - root menu -> **Auxiliary swap**, or **Places** -> **Auxiliary swap**

| Step | Expected |
|---|---|
| open it with only the eMMC and the boot card attached | "Nothing to offer: every disk here is mounted, built into the machine, or the one this system runs from" |
| open it with the boot card and a spare, unmounted card/SD | the spare card only, with model and size |
| pick it → **Cancel** | no partition table is written |
| pick it → **The whole card** → the confirmation | a busy card, then a notification with the partition and "priority 10"; `swapon --show` lists it, `blkid -L lazyos_swap` resolves it |
| `grep lazyos_swap /etc/fstab` | one marked block: `LABEL=lazyos_swap none swap sw,nofail,x-systemd.device-timeout=15s,pri=10 0 0` |
| **Hibernate** from the power menu (`Alt+Escape`) on the default card (swap on, no `resume=`) | a notification naming the missing `resume=` and the kpart rebuild it needs; the machine stays up, it does not suspend |
| reboot with the card in | `swapon --show` lists it again without touching the menu (priority 10, below zram's 100) |
| remove the card and reboot | the boot still reaches the session (a swap unit may show as failed only if `nofail` was lost); it waits at most 15 s for the card |
| run the menu again | the status row (device, size, priority) and **Turn it off**, not the disk picker |
| **Turn it off** → confirm | the partition is swapped off, the fstab block is gone, the partition is still in `lsblk` with `FSTYPE=swap`, and the next boot does not try to use it |
| after turning it off, open **Format a disk** | the card is offered again (it is no longer active swap), and formatting erases the old swap partition |

Never pull the card while the swap is active: the pages living on it are lost. Turn
it off from the same menu first. Hibernate additionally needs
`resume=LABEL=lazyos_swap` on the kernel command line - the swap row does not edit
the kernel partition, so the default card refuses with that message. A partition
labelled `nureb_swap` (an old card) is still usable as swap, but that `resume=`
will not find it: re-label it (`sudo swaplabel -L lazyos_swap <device>`) or let
the swap menu recreate it.

## Search key - the file search

| Step | Expected |
|---|---|
| press **Search** (nothing else) | a list of files from all over `$HOME`, newest first, each with the same icon the browser draws |
| type `conf` | the list narrows as you type; a path is shown as `~/relative/path` |
| Enter on a text file | it opens exactly as it does from `Alt+Ctrl+F` |
| Escape | nothing opens |
| press Search twice in a row | two lists, no leftover window and no second process after both close |

`rofi-files --search` is the same mode from a terminal. An empty list on a home
with files means `fd`/`fdfind` is missing (`check-system.sh` FAILs).

## Alt+Search - the terminal file search

| Step | Expected |
|---|---|
| hold **Alt**, tap **Search**, release Search then Alt | a floated, centred foot window with an `fzf` prompt showing `~ > ` and rows filling in as the walk runs |
| type part of a path, two segments | the list narrows while you type, with no pause between keystrokes (the whole tree is the pool: nothing is capped at 500 rows) |
| put the cursor on a text file | the right 55 % shows its first lines through `bat` (`BAT_THEME=ansi`, solarized) |
| put it on a photo or a PDF | the preview draws a picture in the pane (symbols, not sixels - it survives the next redraw) |
| put it on a `.zip` or a binary | the pane names the type and the size instead of trying to draw it |
| **Ctrl+Y** | `copied the path` in the header; `wl-paste` is `~/relative/path` |
| Enter on a text file / a photo / a video | opens exactly as a double click in pcmanfm does (`bat`, `imv`, `mpv`) |
| type `config` | dotfiles are found here (`~/.config/...`), unlike the Search key's list |
| Escape | the window closes and nothing opens |
| press Alt+Search twice in a row | two windows, and no `fzf` process left after both close |
| `lazyos-find` from a foot prompt | the same search runs *in that terminal* instead of a new window |

A window that flashes and closes means `bin/lazyos-find` lost its executable
bit; an empty list on a home with files means `fzf` is missing
(`check-system.sh` FAILs both). `fd`'s `.cache`, `.git`, `node_modules` and
`.local/share/containers` are excluded, so a file inside one of them is
correctly absent.

## Files (window) - Search+Shift

| Step | Expected |
|---|---|
| hold **Shift**, tap **Search**, release Search then Shift | pcmanfm opens on `$HOME`, in the solarized palette (base03 window, cyan selection), JetBrains Mono, LazyOS icons |
| tap **Search** on its own | the file search, not the window (`rofi-files --search`) |
| hold **Search**, tap a top-row key | an F-key reaches the focused window; no menu opens on the way |
| right-click a file -> **Open With** | the first entry is what a double click does: text in `bat`, a document in `papers`, a picture in `imv` (or `chafa`), a video in `mpv`. Two deliberate exceptions: an HTML page is text to the Files list (`bat`) and a page to a double click (**Firefox**), and an EPUB goes to Firefox because `papers` reads none |
| press **F4** | `foot` opens in the current directory (not an error about `x-terminal-emulator`) |
| delete a file | it goes to the Trash and **Restore** puts it back (`gvfsd-trash`) |
| look at a folder of photos | thumbnails; a PDF or a video keeps its mimetype icon (libfm decodes images only - that is not a bug) |
| insert a USB stick | it appears in the sidebar already mounted (`udisks2`); nothing on it runs by itself |
| `dpkg -l pcmanfm gvfs` | both installed; `gvfs-backends` absent on purpose |
| `ls ~/.config/mimeapps.list` | no such file: only `/etc/xdg/mimeapps.list` decides, so `org.lazyos.view.desktop` is the system default. A file left from a pre-v0.3.0 card keeps the old viewer for every association it names |

A window that looks like Adwaita's light grey means `~/.config/gtk-3.0/gtk.css` is
missing; icons that are empty squares mean a name the LazyOS theme does not carry
(`check-system.sh` warns with the list). A double click that does nothing means
`~/.config/bin/lazyos-view` is not executable: `gio mime image/png` then answers
"no default application" instead of naming `org.lazyos.view.desktop`.

## USB sticks and SD cards - Places

| Step | Expected |
|---|---|
| insert a stick or a card and open **Places** (root menu, or `rofi-places`) | a **Mount** row for each partition with a filesystem, labelled with its name (a space in a label reads as a space, not `\x20`) |
| Enter on a **Mount** row | a mako card while it mounts, then `rofi-files` opens that directory. A trailing `.` in the path is the `udisksctl` message not being stripped |
| Enter on a mounted disk row | `rofi-files` browses it |
| **Alt+1** on a mounted disk row | "Unmounted `<label>`" and the row is gone from the next menu; pulling a stick that is still mounted is how a filesystem is lost |
| **Alt+1** on a folder row (Home, Documents) | "Only a disk can be unmounted", and nothing else happens |
| a swap partition or a LUKS one on the stick | no **Mount** row for it (a swap has nothing to show and LUKS needs a passphrase prompt this menu does not have) |
| unplug and replug the stick | it is listed again; a stick that stays mounted after the unplug is a sign the filesystem is dirty |
| `systemctl status udisks2` | *inactive (dead)* until something mounts - it is D-Bus activated, nothing runs resident, and `udiskie` is deliberately absent |

A **Mount** row that fails says why in a critical notification, in udisks' own
words. `vfat`/`exFAT`/`NTFS` need no helper package: the kernel has the drivers.

## Editors

| Step | Expected |
|---|---|
| `echo $EDITOR $VISUAL` in a new terminal | `nvim nvim` |
| `nvim file`, `:q` | Neovim opens; no configuration is shipped, so it starts plain |
| `vim file`, `vi file` | the same editor both ways (`vi` is the alternative `vim` registers) |
| `eff` | the fzf picker, and the choice opens in Neovim |
| `EDITOR=nano eff` | nano wins: the profile only sets the variable when nothing has |
| `nano file` | still there, and still what `git commit` falls back to without a session profile |

A `vi` that is missing while `vim` works means the `vim` postinst never ran
(`update-alternatives --display vi` says so); `check-system.sh` asks for `vi` by
name for that reason.

## Screenshots

| Step | Expected |
|---|---|
| `Alt+Shift+F` | the notification names a file under `~/Pictures/Screenshots/<today>/`, and carries the image |
| `Alt+Shift+S`, drag a region | the same, one area |
| `Alt+Ctrl+C` -> annotate | `swappy` opens on the newest file and writes the edits back over it |
| `Alt+Ctrl+C` -> the last row | opens the screenshots folder (or `~/Pictures` before the first one) |
| `ls -d ~/Pictures/Screenshots/*/` | one directory per day, no `*.png` left in `$HOME` |

## Tablet mode

Fold the keyboard all the way back.

| Step | Expected |
|---|---|
| fold it back | the on-screen keyboard appears at the bottom (solarized, `JetBrains Mono`), the touchpad **and the physical keyboard** do nothing, one notification: "Tablet mode: the keyboard is up and the screen follows the device" |
| type on the folded keys | nothing arrives in the focused window (the keys are under the screen now) |
| type in foot on the on-screen keyboard | letters land in the focused window; `Cmp` then `a` opens `á à â ä …` and tapping one inserts it; `Cmp` then `?` reaches `¿` |
| turn the device 90° clockwise, 90° back, upside down, and lay it flat | the screen follows within about a second, and the flat reading puts it back to `normal` |
| the pose you would type in (screen upright, facing you) | the screen is upright, **not** upside down - if it is not, see Calibrate below |
| tilt it, do not turn it | the screen does **not** flip |
| press the Lock key | the screen locks and the machine suspends (logind reads the power-button node, not the disabled keyboard) |
| fold it back into a laptop | keyboard gone, touchpad **and keyboard** back (type), screen upright, "Tablet mode off" |
| `pgrep -x wvkbd-mobintl` and `pgrep -af 'tablet-mode poller'` afterwards | nothing |
| Settings (`Alt+Ctrl+O`) -> **Tablet mode** | toggles by hand; the row reads `on · screen 90` or `off` |
| Settings -> **Calibrate rotation** (listed only in tablet mode) | hold it upright and it says so; `~/.config/lazyos/tablet.conf` then holds `normal=<axis><sign>` |
| Settings -> **Rotation** | still turns the screen by hand, independently |

What a failure means:

* **no keyboard on the fold, but it works by hand** (`~/.config/bin/tablet-mode on`):
  the switch did not arrive. `grep -A5 cros_ec_buttons /proc/bus/input/devices`
  must show `SW=3`, and `bindswitch tablet:` must be in
  `/etc/sway/config.d/28-tablet.conf`.
* **keyboard, no rotation**: the accelerometer (`/sys/bus/iio/devices/iio:device1`,
  `cros-ec-accel`); `check-system.sh` FAILs when it is gone.
* **rotation inverted or turned the wrong way** (the 2026-10-21 card came up
  upside down): run **Settings → Calibrate rotation** while holding the screen
  upright, or `~/.config/bin/tablet-mode calibrate`. It writes the reference to
  `~/.config/lazyos/tablet.conf` and applies it at once, and it needs no rebuild.
  `tablet-mode calibration` says `calibrated …` or `default …`; deleting the file
  goes back to the built-in mapping. The four `transform` values in `orient()`
  are now derived from that one reference.
* **the physical keyboard still types**: `swaymsg -t get_inputs -r | grep -c
  '"type": "keyboard"'` and `tablet-mode status`; `tablet-mode off` restores it.
  It is switched off only after the on-screen keyboard is confirmed alive, so a
  wvkbd that failed leaves the physical keyboard on and the notification says so.
* **keyboard appears but types nowhere**: the focused surface refuses virtual
  keyboards; the emoji picker's notification says the same thing, and pending
  `zwp_virtual_keyboard_manager_v1` support is in wlroots, not wvkbd.

## Regressions this round could have touched

| Step | Expected |
|---|---|
| `Alt+Ctrl+K` (key view) | the Clipboard and Emoji bindings with their descriptions, and **one picture per section** (terminal, gear, lock, sun, speaker…) on the section line |
| `Alt+Escape` | the power menu (not the battery banner) |
| a slow action (Wi-Fi scan, DNS change) | a mako card that becomes the result, and the bar's spinner while it runs |
| **click the spinner** during a Wi-Fi scan | the card turns into "Stopped Wi-Fi" (or whatever it was), no second toast, and the Wi-Fi menu opens nothing instead of half a list |
| click the spinner during an apt install | it says the task cannot be stopped, and the install finishes |
| any notification that has one (`sudo`, a file copy) | its picture is drawn; a text-only card means mako's `icon-path` |
| `Alt+Ctrl+V` and press `Alt+1` | the entry is deleted (the second action still works after the `-kb-custom-1` fix) |
| root menu (`Alt+Space`) | **Colour**, **DNS**, **Packages**, **Reminders**, **About**, **Download**, **Password** and **Search inside files** are all there, and the `ai` heading has **Agents** and **Dictation** together, above `system` |
| `Alt+Ctrl+G` and then `Alt+1` inside that menu | nothing is taken by sway: the menu answers, the workspace does not change |
| `Alt+Ctrl+F`, then `Alt+1` | the actions menu, not workspace 1 (the `global-kb` fix reached both menus) |
| `gio mime image/png`, `gio mime application/pdf`, `gio mime inode/directory` | `org.lazyos.view.desktop`, `org.gnome.Papers.desktop`, `pcmanfm.desktop`. This is GIO's own answer, not the file read back: a `lazyos-view` without its executable bit makes GIO say "no default application" |
| `dig +short example.com` and `host example.com` | an address. `dig` comes from `bind9-dnsutils`; `resolvectl` is deliberately absent because NetworkManager owns `resolv.conf`, and its absence is not a gap |
| a menu whose rows are long (Files on a deep path, a grep match with a long line) | the window is wide enough for the end of the path; a row cut with `…` mid-detail means `ui_width` stopped sizing that list |
| lock (`Alt+Ctrl+L`, the Lock key, and the 300 s idle timer) | the **mountain photograph** on base03 (the brand wallpaper is the desktop's, not the lock's), then back to the session; no idle screen left behind (it is stopped before the lock) |
| idle for 150 s without touching anything | the **LazyOS** wordmark, the caption and the arcade line, with an effect rebuilding them and another one after it (a still frame means `tte` did not start) |
| press any key while the idle screen animates | it closes at once, not at the end of the effect |
| `Alt+Shift+Escape` | the screen locks, *then* the machine suspends; on resume the lock is still up |
| shut down from the power menu | the machine powers off (the question is asked once) |

## Writing it down

`reports/check-system-<date>-card.txt` gets the script's output verbatim (MAC
masked to the OUI), and `reports/feedback-<date>-session.txt` gets what only you
saw: which row was wrong, which notification never came, which key did nothing.
Both are what the next round's checks are written from - a FAIL here becomes a
line in `check-system.sh`, not a note in a commit message.

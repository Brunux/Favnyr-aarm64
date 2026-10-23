# Software

Debian **forky** with a normal `apt`. There is one repository
(`deb.debian.org/debian forky main contrib non-free non-free-firmware`) and
`sudo` does not ask for a password.

```sh
sudo apt update                # refresh the package lists
sudo apt install <package>     # add something
sudo apt remove <package>      # remove it (add --purge for its configuration)
apt search <word>              # find packages
apt show <package>             # details, size, dependencies
sudo apt full-upgrade          # the whole system (see "The system" page: a new
                               # kernel also needs the boot partition rewritten)
```

Nothing about this machine depends on a package manager other than apt: no
Flatpak, no Snap, no AUR. If a program is not packaged for Debian, compiling it
in your home is the way (that is how the compositor got here), not a third-party
repository.

## What is installed, and where the lists are

The image is built from four package lists in the repository, copied on the
machine at `~/Dev/LazyOS/`:

| List | Holds |
|---|---|
| `packages/base.list` | what the system needs: systemd, NetworkManager, ssh, filesystems, audio plumbing, zram, the clock |
| `packages/tools.list` | the terminal tools (below) |
| `uis/sway/packages.list` | the session: sway, waybar, foot, rofi, mako, swayfx's dependencies, fonts |
| `devices/google-asurada/packages.list` | this hardware: the firmware, cgpt, vboot, depthcharge-tools, rsync |

Editing a list does not change the running machine; it changes the next image
that is built. For the machine in front of you, `apt install` is the answer.

## The terminal

**foot**, solarized, JetBrains Mono with a Nerd Font fallback for glyphs.

| Keys | Does |
|---|---|
| Ctrl+Shift+C / Ctrl+Shift+V | copy / paste |
| Ctrl+Shift+N | new window |
| Ctrl+Plus / Ctrl+Minus | text size |
| Ctrl+Shift+PageUp / PageDown | scrollback |
| Shift+PageUp | scrollback too |

### kitty, if you install it

`sudo apt install kitty` gives a second terminal that arrives already in this
session's palette: `~/.config/kitty/kitty.conf` ships with the image and carries
the same solarized colours as foot, JetBrains Mono 11 with the Nerd Font
fallback, 6 px of padding, 1000 lines of scrollback and 0.87 alpha, so the
compositor's blur shows through it exactly as it does through foot. Nothing here
starts it - `Alt+Return` and every menu open foot - so the file exists for one
reason: installing a terminal should not put a second, unthemed one beside the
first.

| Keys | Does |
|---|---|
| Ctrl+Shift+C / Ctrl+Shift+V | copy / paste |
| Ctrl+Shift+T | new tab (the tab bar appears with the second one) |
| Ctrl+Shift+N | new window |
| Ctrl+Shift+Z / Ctrl+Shift+X | previous / next shell prompt |
| Ctrl+Shift+Plus / Minus | text size (`Ctrl+Shift+Backspace` back to 11) |
| Ctrl+Shift+Up / Down | scrollback, one line |
| Alt+Q | close the window, which is sway's key and not kitty's |

Alt is the session's modifier and sway sees every Alt chord first, which is why
kitty keeps its own shortcuts on Ctrl+Shift. The one respect in which it differs
from foot: kitty draws JetBrains Mono's ligatures (`->`, `!=`), the same bytes
in a narrower column.

### The prompt

**starship**, in the shape of a powerline: one coloured block per question, left
to right - who you are, where you are, which branch and what the tree looks like,
which languages this directory has, which container, what time - then a second
line that is nothing but the character: cyan `❯` when the last command worked,
red `✗` when it did not. The blocks are solarized (base3 on orange for the user,
base02/base01 for time and containers, cyan for git, blue for the languages) and
the separators are the triangles `U+E0B0`/`U+E0B2`, which is the form that looks
the same in foot and in kitty.

Two things are worth knowing before editing `~/.config/starship.toml`:

- Most of what you see in the prompt is a *private-use* character, and a
  character no installed font has is a box - silently, in some terminals and not
  in others. This image ships the DFSG-stripped Nerd Fonts set, which has no
  Nerd Fonts v3 `nf-md` glyphs at all, so the OS logo the upstream prompt carried
  is not here (and neither are the distro logos). Every glyph the shipped prompt
  uses comes from JetBrains Mono, Symbols Nerd Font Mono, Font Awesome or
  Material Design Icons, all named in `~/.config/foot/foot.ini`.
- `check-system.sh` reads the prompt, asks fontconfig which font draws each
  codepoint and FAILs when the answer is "none", or when the only family that has
  it is not one foot is given (that one renders small, from fontconfig's
  fallback). Add a glyph, run the report: it names the codepoint.

`starship prompt` prints what the prompt would look like, and `starship preset
nerd-font-symbols` is a starting point if you would rather build another one.

The shell is bash with a set of aliases and tools that are worth knowing:

| Instead of | Use | Note |
|---|---|---|
| `ls` | **eza** | `ls`, `lsa`, `lt` (tree), `lta` |
| `cat` | **bat** | `cat file`, with colours; `bat file` pages it |
| `find` | **fd** | `fd name` |
| `grep` | **rg** | not aliased: `rg pattern` |
| `cd` | **zoxide** | `cd` still works for paths; `z name` jumps by frecency |
| `Ctrl+R` | **fzf** history | `Ctrl+T` files, `Alt+C` directories |
| `ff` / `eff` | fzf + bat | pick a file with a preview / edit the pick |
| `t` | tmux | attach to the `main` session or start it |
| the prompt | **starship** | user, directory, git, language, container, time |

Also there: `btop`, `lazygit`, `tmux`, `gum`, `tldr` (cached cheat sheets),
`fastfetch`, `inxi`, `tmux`, `yt-dlp` (with `ffmpeg` to merge streams), `node`
and `npm`, `tesseract` (OCR), `zbarimg` (QR/barcode), `qrencode`, `socat`,
`inotifywait`, `whois`, `unzip`, `jq`, `less`, `man` (pages go through bat).
`swappy` annotates a screenshot and `wf-recorder` records the screen, both
through the capture menu (Alt+Ctrl+C).

**`cd -` is bash's, not zoxide's.** `cd` is the wrapper `zd`, which keeps real
paths working and jumps by frecency otherwise; a first argument of `-` goes
straight to bash's own `builtin cd -`, so it returns to the previous directory
and prints it instead of asking zoxide for a match. The shell initialises zoxide
at the very end of `~/.bashrc`, after starship and after `~/.bash_aliases`:
starship replaces `PROMPT_COMMAND`, and zoxide's directory-tracking hook has to
be appended to it last - initialised earlier, the hook is dropped and the first
`cd -` prints "zoxide: detected a possible configuration issue".

`man <command>` and `tldr <command>` are the two fastest ways to learn a tool;
`tldr` needs one fetch the first time (`tldr --update`).

## Editors

**nano** is the editor that is always there (`nano file`).

**nvim** is what `$EDITOR` names, so `eff`, `git commit` and `crontab -e` open
Neovim: `nvim file` for one file, `nvim` for the empty buffer. It brings its own
Lua runtime and tree-sitter, and it starts with no configuration - plugins, a
language server and a theme live in `~/.config/nvim`, which the image does not
write. **vim** is the other one (`vim file`), and it is what makes `/usr/bin/vi`
exist: `vi file` is the same editor, reading `~/.vimrc`.

The variable comes from `/etc/profile.d/90-cli-tools.sh` (a login shell and this
terminal read it) and only when nothing has set it already, so an `EDITOR` you
export yourself wins.

## Emoji

**Alt+Ctrl+E** opens the emoji picker (the root menu and the launcher carry the
same row): type a name, Enter inserts the glyph at the cursor, Alt+1 only copies
it. The list is `~/.config/emojis.tsv`, and **every row is a glyph the shipped
font draws**: on the build host the table is filtered against the character map
of `fonts-noto-color-emoji`, so a symbol the font does not carry is never offered
instead of being inserted as an empty box. `~/.config/emojis.font` next to it
pins the SHA256 and the family of the font the filter used, which is how
`check-system.sh` notices a font upgrade; after one, re-running
`scripts/make-emojis.sh` on a host with `fontconfig` and the font rebuilds the
table. Nothing of this runs on the machine: the picker is one rofi window and the
table is a file.

## Working in the terminal all day

* `tmux` (alias `t`) keeps a session across terminal windows; `Ctrl+B` is its
  prefix.
* `ssh` keys: generate your own with `ssh-keygen -t ed25519`; the machine also
  runs `sshd`, so you can log in from another computer.
* `journalctl -b -p err` shows this boot's errors; `dmesg` the kernel's.
* `/usr/share/doc/<package>/` and `man` are local; `tldr` is the shortcut.
* The network's three questions are `ip -br addr`, `ping` and `ss -tunlp` (the
  first two packages are `iproute2` and `iputils-ping`, in the base list because
  no "is it up" answer starts without them).
* `zip` is the writing half of `unzip`, `strace` is the debugger when a program
  fails without a message, and `ncdu` is the disk walk to reach for when this
  58 GB eMMC fills (`du` prints the same numbers as a wall of text).

## The browser

**firefox-esr** is the one application outside the session that this image
installs on purpose (280 MB on disk, none of it resident while it is closed). It
is the browser, and it is also the last resort for a file: a downloaded page, an
EPUB, a `.deb`, or anything whose own viewer is missing.

* **Files** (Alt+Ctrl+F) hands it an **EPUB** or a `.deb`, the formats nothing
  else here opens. A PDF, a DjVu page or a comic goes to **papers** (the reader),
  a picture to **imv**, a video to **mpv** and an HTML page to `bat`, because an
  HTML page *is* text - each of those is cheaper than a browser window (see
  below). A **URL** is the browser's job, which is what `xdg-open` below is
  about.
* `xdg-open` exists (`/usr/local/bin/xdg-open`, two dozen lines): a URL goes to
  Firefox, and a path goes to the same rule table a double click in pcmanfm uses
  (`lazyos-view`), so a file opened from a script lands where it would land from
  the desktop. That is what foot's ctrl+click and the command-line tools that
  print a URL call - there is no desktop portal in this image.
* It is a normal Wayland client here (no Xwayland needed), and its icon is its
  own: the launcher shows Firefox's icon, not a substitute.
* **Alt+Shift+Return** opens it from anywhere - the one program that earns a key
  of its own (Omarchy has none; its launcher is the way there).

## Playing a video

**mpv** is the video player, and the only program the file browser needs for
`video`: **Files** (Alt+Ctrl+F) starts it on the file you chose and leaves it
running after the list closes. It seeks, shows subtitles, follows the screen's
refresh rate and takes the keyboard while its window has the focus - `q` quits,
`space` pauses, the arrow keys seek, `f` goes fullscreen.

It is deliberately plain: `rofi-files` starts it with `--no-config`, so nothing
in `~/.config/mpv` (a shader, a GUI driver, a profile copied from another
machine) changes what a click does - write that file if you want to, but the
session's behaviour does not depend on it. A still picture does not start mpv:
**imv** draws it on the GPU (zoom, pan, rotate, and a slideshow of the folder),
and `chafa`'s sixels in a foot window are the answer when imv cannot get an EGL
context - both cost a fraction of a player, and a picture looks like a picture
rather than like a video that has not started.

**The first picture comes with its keys.** imv has no on-screen help, so the
first picture of a session raises one notification with the bindings its
`/etc/imv_config` defines: `i` and `o` zoom, `h` `j` `k` `l` pan, **Ctrl+R**
rotates, **Left** and **Right** walk the folder, `f` is fullscreen and `q` quits.
It is one card per session (the state file is
`$XDG_RUNTIME_DIR/lazyos-imv-hint`), it appears only when a picture actually
opens in imv - not for text, a document or a video - and it is queued in the
background, so it never delays the picture.

**A film keeps the screen awake.** mpv asks the compositor for an idle inhibitor
while its window is visible, so the idle screen and the lock do not arrive in the
middle of one. Firefox and the document reader ask the same thing through their
own interface and get it from the compositor (see the next section); the bar's
coffee cup is the manual switch for everything else - a long download, a recipe,
a page you are reading from across the room.

## The camera

**The camera works because the kernel drives it, not because something was
installed for it.** It is a USB device on the internal hub (`lsusb`:
`0408:30d5 Quanta USB2.0 HD UVC WebCam`, at `1-1.4`), and `uvcvideo` - a module
of this kernel - turns it into `/dev/video0`. `/dev/video2` and `/dev/video3`
are the SoC's hardware video encoder and decoder, not cameras. This device tree
carries no MIPI sensor, so there is no second (rear) camera to open: a machine
with one would need a kernel and an image pipeline, not a package.

**To open it: Camera, in the root menu (Alt+Space) and the launcher.** It runs
`~/.config/bin/lazyos-camera`, which finds the node bound to `uvcvideo` - by
driver, because the numbering is what a replug reorders - sends a notification
when there is no camera at all, and starts **guvcview**: the preview, the
camera's own controls, and two capture buttons. **Alt+Q** closes the window
(sway's kill; guvcview's own quit is the **Quit** button). The device opens
exclusively, so pressing **Camera** again while it is open brings that window
back instead of starting a second one.

**The controls are half of what it is for.** The preview and **Image Controls**
are two windows: exposure, gain, brightness, contrast, white balance and
sharpness come from the camera itself through UVC, and a camera that looks black
in a dim room is usually exposure, not a broken driver. Auto exposure takes a
moment to settle, so the first frame after the window appears is darker than the
second one - that is the sensor ramping, not a fault. **Cap. Image (I)** writes
`~/Pictures/Camera/photo-N.jpg`, **Cap. Video (V)** writes
`~/Videos/camera-N.mkv`; guvcview continues that numbering across runs, so
nothing is overwritten, and a clip carries the internal microphone.

**Without the window**, from a terminal:
`ffmpeg -f v4l2 -i /dev/video0 -frames:v 1 /tmp/cam.png` captures one frame, and
`ffmpeg -f v4l2 -list_formats all -i /dev/video0` lists what this camera offers
(MJPEG and YUYV, from 1280x720 down to 160x120). That is the test that separates
"the camera is broken" from "the window is broken", and it is the same device
`check-system.sh` reports on.

**A browser reads the camera directly.** A video call in Firefox needs none of
the above: Firefox asks the kernel for the device and prompts for permission in
its own window. **The camera *portal* is what this image does not have** -
`org.freedesktop.portal.Camera` needs a portal backend that implements it, the
only backend here is the wlr one (Screenshot and ScreenCast), and
`xdg-desktop-portal` therefore logs *The camera portal requires an access impl*
once per session. Native programs are unaffected; a Flatpak application, which
may only reach a camera through that portal, sees none at all. That is a stated
limit rather than a reason to run one more daemon.

## Files in a window

**Search+Shift** opens **pcmanfm**: the files **Alt+Search** and **Alt+Ctrl+F**
list, with a mouse. It is a second entry and not a replacement, because the two
answer different questions - a list is one keystroke and no window, a window is
what drag and drop, several files at once, renaming in place and the trash need.
The launcher (**Alt+D**) has both rows, **Files** and **Files (window)**, the
root menu (**Alt+Space**) lists them the same way, and the window is a normal
one: tile it, move it to another workspace, close it with **Alt+Q**.

* **A double click opens what the Files menu opens.** `/etc/xdg/mimeapps.list`
  decides, and it names the same programs the list does: a document (PDF, DjVu, a
  comic) goes to `papers`, a directory to pcmanfm, a page or a `.deb` to Firefox,
  and text, pictures and video to `org.lazyos.view.desktop` ->
  `~/.config/bin/lazyos-view`, which is the rule table `rofi-files` follows (text
  in `bat`, a picture in `imv` - sixels in a foot window when imv cannot start - a
  video in `mpv`, an EPUB in Firefox; the list reads an HTML file as text where a
  double click opens the page). There is no desktop portal in this image, so those
  entries *are* the "open with" mechanism - for pcmanfm, for `gio open` and for
  any application that asks to open something.
* **A terminal from inside it is foot** (`F4`, "Open Terminal Here"): the shipped
  configuration names `foot`, because pcmanfm's default `x-terminal-emulator`
  is not installed here and the menu entry would do nothing.
* **Thumbnails are pictures only.** libfm decodes them itself with gdk-pixbuf and
  ignores the freedesktop `.thumbnailer` files `poppler-utils` ships, so a PDF or
  a video keeps its mimetype icon in the icon view - the Files list still draws a
  real thumbnail for both. A file over 4 MB shows its icon instead of a decode:
  a 24 MP photo costs ~83 MB of memory for a 128 px thumbnail, and this is a 4 GB
  machine.
* **The trash works** (`gvfs` provides `gvfsd-trash`), so a deleted file lands in
  `~/.local/share/Trash` and can be restored. SMB and phone mounts do not,
  because `gvfs-backends` (31 MB of `samba-libs`) is deliberately not installed
  and the Network places are off; a USB stick appears in the sidebar once
  `udisks2` mounts it, and `autorun` is off so a card never runs its own
  contents.
* **Its colours are the session's.** `~/.config/gtk-3.0/settings.ini` loads
  Adwaita dark with the **LazyOS** icons on JetBrains Mono 10, and `gtk.css` next
  to it repaints the surfaces in the solarized palette: base03 windows, base1
  text, the cyan selection the menus use, and no animations of its own (swayfx
  already animates the window). It is the only window whose look comes from the
  GTK theme (waybar is GTK3 too but paints the bar from its own stylesheet), which
  is why the two files exist rather than a whole GTK theme package.
* **The document reader gets the same palette, from a different pair of files.**
  `papers` is GTK4/libadwaita, which does not read `gtk-3.0/` at all: it asks the
  *settings portal* for the desktop's colour scheme, and this image runs no
  Settings portal (the only backend is the wlr one, for screenshots and screen
  casting). It therefore opened in libadwaita's light default, a white rectangle
  in a base03 session. `~/.config/gtk-4.0/settings.ini` states the dark
  preference and `~/.config/gtk-4.0/gtk.css` paints the libadwaita named colours
  in the same solarized values as the GTK3 file next door. Any other GTK4 or
  libadwaita application you install inherits it, and no portal daemon is
  installed for one boolean.
* **A document can be read, searched, zoomed and annotated; printing needs a
  print service.** "Print" in `papers` answers that it cannot find one, because
  the image ships no `cups` daemon (it is a system service, not a viewer). If you
  print, install it (`sudo apt install cups`) and configure a queue once.

## USB sticks and SD cards

A USB stick or a card shows up as a **Mount** row in **Places** (the root menu,
Alt+Space, and the Places entry in the launcher): picking it mounts it and opens
it in Files, and **Alt+1** on a mounted disk unmounts it. Eject before pulling
the stick out - a filesystem that is still mounted when it disappears is how one
gets corrupted.

* Mounting is **udisks2**'s job, because an ordinary session cannot mount
  anything itself (that is a root operation). It is D-Bus activated: nothing of
  it runs until a mount is asked for, and nothing stays afterwards.
* The same disks are in pcmanfm's sidebar (**Search+Shift**), and a stick shows
  up there as soon as it is mounted.
* **Unmounting from pcmanfm needs `eject`.** The window's Unmount action runs
  `eject`, not `udisksctl`, and the package is not a dependency of udisks2 or
  gvfs: without it the action fails with `Failed to execute child process
  "eject" (No such file or directory)` and the stick stays mounted. `eject` is
  in `uis/sway/packages.list`; the Places menu unmounts through `udisksctl` and
  does not need it, which is why the failure only showed up in the window.
* `vfat`, `exFAT` and `NTFS` all work - the kernel has those drivers - so a
  stick from a camera or from Windows needs no extra package.
* **Auxiliary swap** is the row above it: it takes a card that is not a system card
  and turns the whole thing - or 2, 4 or 8 GiB of it - into a swap partition
  labelled `lazyos_swap`, adds the `/etc/fstab` line that activates it at every
  boot, and turns it on now. "Not a system card" is the list: nothing mounted on
  it, no bootable kernel partition, and not the disk this system runs from. The
  card that installed the eMMC has one of those kernel partitions, so with it
  attached the row answers *Nothing to offer* - that is the guard, not a fault,
  and the same sentence names the reason. Repurposing such a card is a deliberate
  root action (`sudo wipefs -a /dev/mmcblk1`, which erases the rescue system too),
  never a menu row. The priority is 10 against zram's 100, so the
  compressed RAM is used first and the card is the spillover; `nofail` plus a 15 s
  device timeout keep a boot without the card from waiting or failing. It is a
  dedicated swap card, not a filesystem, so it never appears as a Mount row.
  Pulling it while it is active loses whatever pages live there: open the same row
  and choose **Turn it off** first (`swapoff`, then the fstab entry is removed; the
  partition stays, and **Format a disk** erases it). `resume=LABEL=lazyos_swap` is
  the extra piece hibernate needs - see **The system** page.
* **Format a disk** is the last row of Places: it offers only disks with nothing
  mounted on them, no ChromeOS kernel partition, and that are neither built into
  the machine nor the disk this system is running from - so the eMMC and the card
  this system boots from are never candidates, even after a partition table is
  wiped - and it asks twice with Cancel first. The filesystem is exFAT (`LAZYOS`,
  for a camera, a TV or another computer: Windows, macOS and Linux all read it,
  and unlike FAT32 it has no 4 GB file limit) or ext4 (for a Linux machine), and
  the whole disk becomes one partition. The internal eMMC is only written by
  `scripts/install-emmc.sh`; this menu is for sticks and cards. It is not
  stoppable: a killed `mkfs` leaves a disk that looks formatted and is not.
* A failed mount says why in the notification, in udisks' own words: "not
  authorized" is a policy problem, "already mounted" means it is already in the
  list above.

## The night light

The session starts `wlsunset` at login: from sunset it warms the screen to
4000 K, and by day it leaves it alone. **Settings → Night light** (`Alt+Ctrl+O`)
turns it off and on by hand; turning it on forces the warm end immediately
instead of waiting for sunset, because a switch that does nothing at 17:00 reads
as a broken switch. The command line and the remembered mode live in
`~/.config/bin/night-light` (`login`, `start`, `stop`, `resume`, `status`), and
the mode is one line in `$XDG_RUNTIME_DIR/lazyos-night-light`, so it dies with
the session.

Suspend and resume are the part that used to lose it: the panel comes back with
its own gamma table, so a running `wlsunset` no longer has any effect and the
screen wakes up at daylight colour while the switch still says on. The idle
drop-in's `after-resume` hook asks the helper to restart the mode that was
running (`night-light resume`), and it does nothing when the night light was off
before the sleep. `night-light status` prints `on schedule`, `on warm` or `off`
from a terminal.

## Translate

**Alt+Ctrl+T** translates **the text you selected with the mouse** - in the
terminal, in the browser, in any window - or, when nothing is selected, what is
on the clipboard. Nothing is opened first: the selection *is* the question, and
the key answers it in place.

The source language is always detected; everything goes into the language
**Settings → Translate** sets. Until that is set it is **English**, so an image
that was never configured answers the common case with one key. The Settings row
also says which language is in use, and the menu it opens lists the twenty-six
languages both engines take with the how-to on the same screen
(`~/.config/bin/translate config` from a terminal). The setting is one line in
`~/.config/lazyos/translate.conf`:

    target=es

The translation is **copied to the clipboard** and shown in a card that names
the direction it went (`Spanish → English`). When it is longer than a few lines
the card is only the summary: a read window opens with the whole translation and
the selection underneath it, paged with `bat`, and Alt+Q closes it. A selection
cut at 3000 characters says so on the card. English into English is the one
direction that answers something already known, so that case says **Already
English** instead of echoing the text and points at Settings.

**The network** decides the rest. Google's JSON endpoint is asked first
(`clients5` with `client=dict-chrome-ex` - the same answer that detects the
language), and MyMemory without a key is the fallback, in 450-character pieces
because 500 is its ceiling. When neither answers, the card says so - *No answer
from the translator: is the network up?* - instead of showing half a
translation. The request is one `curl`; a `busy` card and the bar's spinner say
it is running, and **a click on the spinner stops it**.

Nothing is stored by this feature: no history file, no cache. The result is on
the clipboard, which means it is in the clipboard history (`Alt+Ctrl+V`) like
anything else copied, and the card is in mako's history - so a secret translated
is a secret in both, and the clipboard menu's last row clears that history.

`translate-shell` is deliberately not installed. It would have been one package
(277 kB), but its engine is Google's *other* endpoint, which answers this
machine with a captcha page; the package would be a command that never works
here. `check-system.sh` reports the tools this script does need (`curl`, `jq`,
`wl-paste`) and SKIPs the network probe when the machine is offline.

## Text in other languages

`fonts-noto-core` is installed, so Arabic, Hebrew, Devanagari, Thai, Armenian,
Georgian and the rest of Noto's non-CJK families have a face: a file name, a web
page or an EPUB in one of those scripts is text instead of a row of boxes, in the
terminal and in the browser alike. Nothing has to be configured - fontconfig's
fallback finds them.

Chinese, Japanese and Korean are **not** installed. `fonts-noto-cjk` is 91 MB
installed, the largest single item left after the browser, and a card that only
needs Latin should not pay for it. If you read those scripts:

```sh
sudo apt install fonts-noto-cjk
```

No logout is needed; fontconfig picks the new faces up on the next lookup, and
`fc-cache -f` forces it if an application cached the miss. `fc-list :lang=ja`
answers whether a Japanese face is there, and `check-system.sh` reports the
non-Latin case with `fc-list :lang=ar`.

## What is deliberately not installed

A GUI editor, an office suite and any Electron-style application. The image is
2.9 GB and every one of those would add hundreds of megabytes and a resident
process to a machine with 4 GB of RAM. If you want one, `apt search` it - the
session is a normal Wayland desktop and any GTK/Qt application works. The file
manager above is the one GUI exception, because it does two things a list
cannot; it costs 4.9 MB and starts no daemon.

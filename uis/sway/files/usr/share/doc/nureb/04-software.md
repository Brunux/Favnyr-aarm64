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
machine at `~/Dev/Nureb/`:

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
| the prompt | **starship** | directory and git only |

Also there: `btop`, `lazygit`, `tmux`, `gum`, `tldr` (cached cheat sheets),
`fastfetch`, `inxi`, `tmux`, `yt-dlp` (with `ffmpeg` to merge streams), `node`
and `npm`, `tesseract` (OCR), `zbarimg` (QR/barcode), `qrencode`, `socat`,
`inotifywait`, `whois`, `unzip`, `jq`, `less`, `man` (pages go through bat).
`swappy` annotates a screenshot and `wf-recorder` records the screen, both
through the capture menu (Alt+Ctrl+C).

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

## Working in the terminal all day

* `tmux` (alias `t`) keeps a session across terminal windows; `Ctrl+B` is its
  prefix.
* `ssh` keys: generate your own with `ssh-keygen -t ed25519`; the machine also
  runs `sshd`, so you can log in from another computer.
* `journalctl -b -p err` shows this boot's errors; `dmesg` the kernel's.
* `/usr/share/doc/<package>/` and `man` are local; `tldr` is the shortcut.

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
  (`nureb-view`), so a file opened from a script lands where it would land from
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

## Files in a window

**Search+Shift** opens **pcmanfm**: the files the Search key and **Alt+Ctrl+F**
list, with a mouse. It is a second entry and not a replacement, because the two
answer different questions - a list is one keystroke and no window, a window is
what drag and drop, several files at once, renaming in place and the trash need.
The launcher (**Alt+D**) has both rows, **Files** and **Files (window)**, the
root menu (**Alt+Space**) lists them the same way, and the window is a normal
one: tile it, move it to another workspace, close it with **Alt+Q**.

* **A double click opens what the Files menu opens.** `/etc/xdg/mimeapps.list`
  decides, and it names the same programs the list does: a document (PDF, DjVu, a
  comic) goes to `papers`, a directory to pcmanfm, a page or a `.deb` to Firefox,
  and text, pictures and video to `org.nureb.view.desktop` ->
  `~/.config/bin/nureb-view`, which is the rule table `rofi-files` follows (text
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
  Adwaita dark with the **Nureb** icons on JetBrains Mono 10, and `gtk.css` next
  to it repaints the surfaces in the solarized palette: base03 windows, base1
  text, the cyan selection the menus use, and no animations of its own (swayfx
  already animates the window). It is the only window whose look comes from the
  GTK theme (waybar is GTK3 too but paints the bar from its own stylesheet), which
  is why the two files exist rather than a whole GTK theme package.

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
* `vfat`, `exFAT` and `NTFS` all work - the kernel has those drivers - so a
  stick from a camera or from Windows needs no extra package.
* **Format a disk** is the last row of Places: it offers only disks with nothing
  mounted on them and no ChromeOS kernel partition, so the eMMC and the card this
  system boots from are never candidates, and it asks twice with Cancel first.
  The filesystem is FAT32 (`NUREB`, for a camera, a TV or a car) or ext4 (for a
  Linux machine), and the whole disk becomes one partition. It is not stoppable:
  a killed `mkfs` leaves a disk that looks formatted and is not.
* A failed mount says why in the notification, in udisks' own words: "not
  authorized" is a policy problem, "already mounted" means it is already in the
  list above.

## What is deliberately not installed

A GUI editor, an office suite and any Electron-style application. The image is
2.9 GB and every one of those would add hundreds of megabytes and a resident
process to a machine with 4 GB of RAM. If you want one, `apt search` it - the
session is a normal Wayland desktop and any GTK/Qt application works. The file
manager above is the one GUI exception, because it does two things a list
cannot; it costs 4.9 MB and starts no daemon.

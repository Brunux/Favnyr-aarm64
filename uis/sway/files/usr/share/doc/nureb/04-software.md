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

`man <command>` and `tldr <command>` are the two fastest ways to learn a tool;
`tldr` needs one fetch the first time (`tldr --update`).

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
is not a convenience: without a viewer, the file browser can only read text, and
a PDF, a photo or a downloaded page had no way to open at all.

* **Files** (Alt+Ctrl+F) hands it anything that is not text: images, PDFs, SVG,
  HTML. It renders PDFs itself, so no separate reader is needed.
* `xdg-open` exists (`/usr/local/bin/xdg-open`, two dozen lines): a URL goes to
  Firefox and a path becomes a `file://` URL. That is what foot's ctrl+click and
  the command-line tools that print a URL call - there is no desktop portal in
  this image.
* It is a normal Wayland client here (no Xwayland needed), and its icon is its
  own: the launcher shows Firefox's icon, not a substitute.

## What is deliberately not installed

A GUI editor, a file manager, an office suite and any Electron-style application.
The image is 2.9 GB and every one of those would add hundreds of megabytes and a
resident process to a machine with 4 GB of RAM. If you want one, `apt search` it
- the session is a normal Wayland desktop and any GTK/Qt application works.

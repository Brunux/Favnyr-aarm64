# From Nureb to LazyOS

The distribution has called itself Nureb since the first commit. The idle screen has called itself
**LAZYOS** since the day it was written, and `01-welcome.md` has told the reader that the rest of
the system still uses the old name. This is the plan that finishes it.

## The name

`LazyOS` in prose and in the wordmark, `lazyos` in identifiers (files, desktop IDs, package names,
runtime directories, the icon theme's directory), `LazyOS` as the icon theme's name.

**`lazy by design`** is the tagline and it is already on the screen. It is the contract the lazy
tools share - lazygit, lazydocker, LazyVim: the tool does the boring part, the defaults are the good
answer, and the thing to press is on the screen. That is also how this session is built: `rofi-*`
menus, `busy` cards that can be stopped, `Alt+Ctrl+K` generated from the configuration, every
failure naming itself. The rename is not a new idea; it is the name catching up with the design.

Four words carry the personality, and each one is already a surface:

| Pillar | Surface |
|---|---|
| **Lazy** | the menus, the busy cards, the manual, `check-system.sh` |
| **AI** | `rofi-agents` (Claude Code, Codex and accounts) and `dictate` (whisper), grouped under an `ai` heading |
| **Fun** | the idle screen's effects, emoji, clipboard, the password row |
| **Gaming** | the arcade line, the greeter, the mark itself: a button |
| **Energetic** | motion, not saturation: effects, spinner, animations |

## The mark

The identity duo is already in the palette: cyan `#2aa198` is the session accent, orange `#cb4b16`
is the idle screen's foreground, base03 `#002b36` is every surface, base1 `#eee8d5` is text. **The
session is not repainted**: energy lives in motion and in the mark, never in a louder palette.

The mark is an **arcade button** - a cyan ring, an orange dome, a base03 crescent for depth and a
base1 highlight - because it reads at 48 px, it is made of the palette's own four colours, and it is
the brand's instruction: *press any key to continue*. It ships as

- `brand/lazyos.svg`, the hand-drawn source, rasterised by `scripts/make-brand.sh` (the host tool,
  committed output - the same rule as `make-icons.sh` and `make-emojis.sh`);
- `48x48/apps/lazyos.png` and `scalable/apps/lazyos.svg` in the icon theme, for the launcher's
  **About LazyOS** row and for `os-release`'s `LOGO=`;
- `Pictures/Wallpapers/lazyos.png`, the default background: the mark at 1366x912, the panel's own
  mode (`docs/optimization.md` §7), with `lazy by design` and the arcade line beneath it.

The ANSI-Shadow **LAZYOS** wordmark stays what it is, terminal art: the idle screen draws it with
`tte`, and `~/.config/lazyos/screensaver.txt` is its only copy. It is *not* rasterised into the
wallpaper. A monospace font does not lay the block art out the way the terminal does (verified: the
shadow columns detach from the letters), so each surface gets the form it can draw: text art in the
terminal, the mark everywhere else.

## The naming

| Kind | Today | After |
|---|---|---|
| Repository | `Brunux/Nureb` | `Brunux/LazyOS` |
| Clone / staged copy | `~/Dev/Nureb` | `~/Dev/LazyOS` |
| Icon theme | `Nureb` | `LazyOS` |
| Desktop entries | `org.nureb.*` (25) | `org.lazyos.*` |
| Open-with handler | `nureb-view` | `lazyos-view` |
| Terminal file search | `nureb-find`, app id `nureb-find` | `lazyos-find` |
| Thumbnailer | `nureb-thumbnail` | `lazyos-thumbnail` |
| Manual | `/usr/share/doc/nureb/` | `/usr/share/doc/lazyos/` |
| Runtime state | `$XDG_RUNTIME_DIR/nureb-{busy,tablet,capture}` | `lazyos-*` |
| User data | `~/.config/nureb/`, `~/.local/share/nureb/` | `~/.config/lazyos/` (already exists), `~/.local/share/lazyos/` |
| Notification app name | `Nureb` | `LazyOS` |
| Temp files, units, recordings | `nureb-menu.*`, `nureb-reminder-*`, `nureb-*.mp4` | `lazyos-*` |
| Swap label | `nureb_swap` | `lazyos_swap` |
| Format label | `NUREB` | `LAZYOS` |
| Package revision | `1nureb1` | `1lazyos1` |
| Distro identity | Debian's `/etc/os-release` | `NAME="LazyOS"`, `ID=lazyos`, `ID_LIKE=debian`, `PRETTY_NAME` from `VERSION` |
| Hostname default | `asurada` (from the device) | `lazyos` |

**Frozen:** `reports/` (hardware evidence, verbatim by rule), `docs/releases/v0.1.*` and
`docs/releases/v0.2.0.md` (history), **`Nura`** (a different, upstream project - no `nur` wildcard
sweeps), and `debian-aarch64` as a *build* path (`$HOME/.cache/debian-aarch64`,
`/etc/debian-aarch64-revision`, `lib-dist.sh`): renaming the build root would invalidate the kernel
and compositor trees for no user-visible gain. The *report header* still says LazyOS.

## The phases

Each phase is one commit. The mechanical ones are gated by a fresh rootfs, `sway -C` in the chroot
and the sweep below; the card is spent twice, not seven times.

1. **The plan** (this file), so the sweep includes it.
2. **The open-with chain**: the three scripts, the 25 `.desktop` entries, `mimeapps.list`, and
   `check-system.sh` in the same commit - the check is the witness, not a follow-up.
3. **The icon theme**: the directory, `Name=`, and the four consumers that fail silently when
   missed - `rofi/config.rasi`, `rofi-keys -icon-theme`, `gtk-3.0/settings.ini`, and `mako`'s
   `icon-path=`, which is a literal path because mako reads no theme metadata.
4. **Runtime, data, labels**: the runtime directories, the notification app name, the temp and unit
   names, both labels, the package revision.
5. **Prose**: `/usr/share/doc/lazyos/`, the `~/Dev/LazyOS` paths, the manual's opening page, and the
   six sentences that talk *about* the rename - a sweep turns those into "still LazyOS until the
   rename lands".
6. **The identity**: the mark, `make-brand.sh`, the wallpaper, `os-release`, `/etc/issue`, the
   greeter's greeting, the `fastfetch` card, **About LazyOS** in the launcher, and the `ai` heading.
7. **The release**: `docs/releases/v0.3.0.md`, the README pointer, the tag after the card report.

The sweep is `git grep` plus explicit substitutions, never a `nur` wildcard. Its exit criterion is
`git grep -ic nureb` naming only the frozen files.

## What the review found, and what this plan does about it

- **ImageMagick cannot render SVG here** (no `rsvg-convert`, no SVG coder). `00-deps.sh` gains
  `librsvg2-bin` and `fonts-jetbrains-mono`, both host-only, because `make-brand.sh` needs both.
- **The block art does not survive rasterisation.** The wallpaper uses the mark, as above.
- **The panel is 1366x912**, not the 1920x1200 an old comment claimed; the wallpaper is generated at
  that size.
- **`base-files` owns `/etc/os-release`** (a symlink to `/usr/lib/os-release`). The brand writes the
  file and `check-system.sh` FAILs when it is gone, because an upgrade can silently put the symlink
  back.
- **The version must not drift**: a `VERSION` file is the one source, read by `build-rootfs.sh` and
  passed to `rootfs-setup.sh`; the tag and `os-release` cannot disagree.
- **`make-image.sh`'s comment already named an artifact that does not exist** (`nureb-<device>.img.xz`;
  the real name is `<device>.img`), so the example is corrected rather than translated.
- **The app id is a second name**: `foot --app-id=nureb-find` is matched by three `for_window` rules.
  A rename that misses it leaves the window unfloated and uncentred, silently.
- **`check-system.sh` grows the anti-drift check**: FAIL on a surviving `org.nureb.*`, WARN on any
  other stale name - scoped to the shipped surfaces, never to `~/Dev/LazyOS`, whose frozen release
  notes are history.

## The release

`v0.3.0`, after v0.2.0 (the session release) is tagged. Its notes carry a short *coming from Nureb*
section: the rename is a clean break, so a card from before it keeps its old directories, its old
swap label and its old desktop IDs.

## Why one release and not a trickle

`install-emmc.sh` copies the **running** card. A half-renamed card installs a half-renamed eMMC, and
the two names then live in the tree at once - which is exactly the state this plan ends.

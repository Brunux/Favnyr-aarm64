# workstyle

Upstream: <https://github.com/pierrechevalier83/workstyle> · crate `workstyle` 0.8.2
(MIT). This directory pins the source, `scripts/build-workstyle.sh` builds it and
`scripts/install-workstyle.sh` puts the package in the image.

## What it does, and why it is here

workstyle renames every sway/i3 workspace to `"<number>: <icons>"`, one glyph per
window in it, and renames it back to the bare number when the last window leaves.
The bar draws the workspace name, so a pill becomes **"3: 🖥 🌐"**: what is running
where, on a keyboard-driven session where `Alt+3` and the third pill have to keep
meaning the same thing.

Nura ran it, and `uis/sway/files/home/.config/workstyle/config.toml` came over
with the rest of that session - but this port shipped it as a config without the
program: Debian packages no workspace renamer, upstream's own answer is
`cargo install workstyle` on the device, and the binary Nura ran was linked
against musl, which cannot run against this image's glibc. Building it here
glibc-native is what makes the feature possible at all.

The pin is the **published crate** and not the git master: master's `Cargo.toml`
still says 0.3.0 against `swayipc` 2.7, while 0.8.2 is the release that added the
TOML config and `deduplicate_icons` - the options the config in `uis/sway/` uses.
The tarball carries its `Cargo.lock`, and the build passes `--locked`, so the
dependency versions are the ones that release was tested with even though cargo
fetches them from crates.io on the first build.

## Building

```sh
scripts/build-workstyle.sh                    # -> $WORKSTYLE_STAGE/workstyle_<v>_arm64.deb
scripts/install-workstyle.sh [ROOTFS]         # into the built rootfs (chroot-free dpkg --root)
scripts/install-workstyle.sh /                # onto the running system
```

`scripts/build.sh` runs both, in that order, around `build-rootfs.sh`. The build
needs `cargo` (already in `scripts/00-deps.sh`, for `CONFIG_RUST=y`), `patch`, and
network access to `static.crates.io`/`crates.io` on the first run. The `.deb` has
one file, `/usr/bin/workstyle`, and its `Depends` is generated from the ELF
(`deb_depends` in `scripts/lib.sh`), so it is `libc6` and `libgcc-s1` - nothing
hand-written to drift.

## The patch

`patches/0001-log-and-reconnect.patch` changes three lines of `main.rs`, and each
one is a failure this image would otherwise live with:

| line | upstream | here | why |
|---|---|---|---|
| unmapped window | `error!` | `debug!` | the fallback glyph is the visible signal; an app nobody mapped (any GTK program) would otherwise add an error line to the journal on every workspace event. `RUST_LOG=debug workstyle` still shows it. |
| lock already held | `error!`, exit 1 | `debug!`, exit 0 | a second instance is the *normal* case: `exec_always` runs on every `swaymsg reload`, and the lockfile is what keeps the session at one process. Exiting 1 made every reload a red line. |
| reconnect loop | retries forever | gives up after 60 tries | it exists for `swaymsg restart` (a couple of seconds); after a logout the compositor is not coming back, and a daemon orphaned in the user's scope would retry once a second for the rest of the boot. |

Everything else is upstream's.

## How the session starts it

`uis/sway/files/etc/sway/config.d/02-workstyle.conf` is one line:
`exec_always --no-startup-id workstyle`. It holds `$XDG_RUNTIME_DIR/workstyle.lock`
so a reload cannot leave two of them, re-reads
`~/.config/workstyle/config.toml` on every workspace/window event (an edit lands on
the next switch, no restart), and defaults to `error`-level logging, which is why
the patch above is what keeps the journal clean.

It is the session's second resident process, after `cliphist`'s clipboard watcher,
and the reason is the same shape: nobody can answer "which windows are in this
workspace" without watching. Measured here: **1 MB of PSS** (2.4 MB of RSS, 4.7 MB at
peak - it maps a 1.9 MB binary and touches little of it), **0 CPU ticks in 60 s** while idle
(it blocks on sway's event stream), and nothing after logout (see the patch).

## The bar's side

`~/.config/waybar/config.jsonc` draws `"{value}"` - the whole name, number
included. `{name}` is only what waybar finds *after* the colon, so it would drop
the number and leave the icons; `{icon}` behaves the same. There is no
`persistent-workspaces` in that module any more, on purpose: the sway module
matches that list by workspace **name**, so once a workspace is renamed every
entry becomes a duplicate pill beside the real one. (The `"*": [1, 2, 3, 4, 5]`
shape the file carried before is the *Hyprland* module's format and never drew
anything here - a pill appears when its workspace exists.)

The icons are 12 px like the numbers, inheriting the pill's colour: an unfocused
workspace's glyphs are as dim as its number, and the bar's own Sun icon (Font
Awesome, same family) sits at 12-13 px in the other modules.

## The icon table

`uis/sway/files/home/.config/workstyle/config.toml` is the table. Three rules:

* **Every glyph is a `UI_I_*` codepoint** from `~/.config/bin/lib-rofi.sh`, so the
  bar, the menus and the pills draw from one alphabet, and `check-system.sh`'s
  font check (which asks fontconfig whether some installed font draws each
  codepoint) covers them too. A codepoint from anywhere else is a tofu box in a
  pill, and nothing logs it. `check-system.sh` fails on a glyph that is not in the
  alphabet for that reason.
* **Glyphs are `\uXXXX` escapes.** TOML parses them; the file stays ASCII. A
  literal private-use character does not survive every editor, which is exactly
  how an earlier glyph table in this repository shipped as a row of empty spans.
* **The patterns are app_ids, not titles**, as far as possible, and they are
  ordered specific-first. Matching is a case-insensitive substring of the title,
  *then* the app_id, then the WM_CLASS, and the first pattern in the file wins -
  so `firefox` sits above `foot`: a browser tab whose title mentions a terminal is
  still a browser. `popup`, `lazyos-find` and `lazyos-welcome` are foot windows
  with their own app_id and are named before `foot` for the same reason.

`deduplicate_icons = true` is the one option set away from Nura's config: three
terminals in a workspace are one terminal glyph, which keeps the pill the width of
a number. Set it to `false` to see one glyph per window.

## What changes for the rest of the system

The workspace *names* change, so anything that addresses a workspace by name
breaks. The sway configuration already addresses them by number everywhere
(`workspace number 3`, `move container to workspace number 3`), and
`check-system.sh` **fails** when a binding or command names a workspace instead.
Reading `swaymsg -t get_workspaces` shows the icons in the names; the `num` field
is the stable one.

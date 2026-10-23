# rust/

The Rust side of LazyOS: the workspace that `docs/plan-ui.md` grows into the session UI
framework (`lazyos-ui`, `lazyos-theme`, `lazyos` multi-call binary, `lazyos mcp`).

It holds the framework's crates (`lazyos-cli`, `lazyos-ui`, `lazyos-theme`) and
the spike that validated the one technical risk before anything depended on it.
The framework is built by `scripts/build-lazyos.sh` and installed by
`scripts/install-lazyos.sh`; `lazyos` reports the repository's `VERSION`
(compiled in), which is what `check-system.sh` compares with the image record.

## `crates/lazyos-cli` - the binary

`lazyos`, the session's multi-call binary (`specs/001-core.md`): the command
table, the `argv[0]`/words dispatch, the host (`foot` window or this terminal),
the terminal restore and the two states spec 001 draws. `cargo test -p
lazyos-cli` runs the unit tests and `tests/cli.rs`, which drives the real binary
in a pty.

## `crates/lazyos-ui` - the widgets

The frame's building blocks; spec 001 ships the empty state and the error card,
spec 002 (`specs/002-design-system.md`) grows the design system here.

## `crates/lazyos-theme` - the palette

The colour roles a surface asks for (`Palette`, `Rgb`), solarized dark today.
Spec 004 turns it into the engine that owns `theme.toml` and the generated
palette files.

## `crates/spike-sixel`

The only technical risk of the UI plan, validated first: foot renders sixels natively, and
`ratatui-image` can draw the theme's PNGs and document pictures through them. The spike draws
the 48 px LazyOS mark and a screenshot in one full-screen frame, prints the protocol the
terminal answered, and leaves its measurements behind.

```sh
cargo build --release
# in a real foot on the device:
foot --app-id=spike-sixel --window-size-pixels=1100x650 \
  -e rust/target/release/spike-sixel \
     uis/sway/files/usr/share/icons/LazyOS/48x48/apps/lazyos.png \
     docs/screenshots/hayato-r1-sway.png
```

It writes `/tmp/opencode/spike-drawn` when a frame is on screen (protocol, cell size,
terminal size, draw time), appends every event to `/tmp/opencode/spike-events`, and exits on
a key or after 30 s.

What it established (device: Hayato, foot 1.28, release build, 150x45 cells):

- `Picker::from_query_stdio()` gets `ProtocolType::Sixel` and a 9x20 cell.
- A 1366x912 picture fills the pane at full size; the first frame costs **169 ms**
  (resize + quantize + sixel encode), every later repaint of the same frame **1-2 ms**
  (the encoded protocol is an unchanged cell, so ratatui's diff sends nothing).
- The fixed `Image` widget **draws nothing** when the protocol does not fit the area; use
  `StatefulImage` with `new_resize_protocol` instead. `Resize::Fit` only shrinks — a hero
  that must fill its card needs `Resize::Scale`.
- `ratatui-image`'s default features pull `libchafa-dev` through `chafa-dyn`; sixel does not
  need it, so the dependency is declared with `default-features = false` and
  `features = ["crossterm"]`. The build host stays free of chafa.
- The `image` crate is declared with `png` + `jpeg` only: its default features are every codec
  it has (AVIF, EXR, TIFF, WebP, GIF, QOI, …), measured at +98 s of build and +1.4 MB of binary
  for formats nothing here opens (`docs/optimization.md` §13). A codec is opt-in.

## Building

`cargo build` in this directory builds the whole workspace. `target/` is ignored; build
scripts that ship binaries set `CARGO_TARGET_DIR` under `$WORK_DIR`
(`$HOME/.cache/debian-aarch64`) like every other build artifact in this repository.

Versions are pinned in `Cargo.lock` (committed: this workspace will ship binaries).

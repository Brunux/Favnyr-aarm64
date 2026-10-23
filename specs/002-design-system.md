# Spec 002 — Design system: tokens, widgets, motion

- **Status**: draft
- **Phase**: 2
- **Depends on**: 001
- **Model**: `opencode-go/glm-5.3` — visual judgement is the hard half (spacing, hierarchy,
  focus); the widgets themselves are conventional Ratatui. Fallback
  `opencode-go/kimi-k2.7-code`.
- **Branch**: `spec/002-design-system`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §4 (design language), §5 (surfaces), §5.11
  (mouse), §14 (motion budget)

## Problem

Today every menu picks its own padding, its own dim colour and its own way of saying "this row
is selected" through rofi's theme. Once the menus are Rust, "make it look like the session"
must be a library, not 30 decisions: a selected row, a card, a key hint and a gauge have to be
the same in the root menu, the installer and the AI surface.

## Scope

**In (`lazyos-ui`):**

- **Tokens** derived from the theme (spec 004): `ColorRole` (bg, surface, surface_alt, text,
  dim, accent, accent_alt, ok, warn, danger, selection), `Space` (0/1/2/3), `Border`
  (rounded/none), `TextStyle` (title, body, dim, code, key, danger), `Density` (normal,
  tablet).
- **Widgets**: `Row` (glyph + label + value + dim detail + selected), `List` (scrolling,
  filterable, key bar), `Card` (titled block), `Header` (title + subtitle + right status),
  `Footer`/`KeyBar` (letters, not Alt chords), `Badge`, `Gauge`, `Progress`, `EmptyState`,
  `Confirm` (Cancel first), `LogPane` (tail, follow).
- **Selection and focus**: one focus ring style; arrows/Enter/letters; mouse click on a row
  selects AND activates (spec §5.11), hover moves the selection. Hit regions are produced by
  the layout, not recomputed by each screen.
- **Motion**: `tachyonfx` for at most 300 ms on open/close and on a result card; the animation
  is skipped when `LAZYOS_REDUCED_MOTION=1` and under a 10 fps-throttled tick. Nothing loops.
- **One frame**: every command renders through the same `App` shell (header, body, key bar),
  so the window shape is identical across surfaces.

**Out:** the palette files (004), the glyph table (003), the task surface (005), any command.

## Design

- The public API is a small vocabulary: `App::new(title)`, `.header()`, `.body(widget)`,
  `.keybar()`, `.run()`. A command does not touch `Frame` directly.
- Density is chosen once from the tablet state directory (the existing
  `$XDG_RUNTIME_DIR/lazyos-tablet`), so the touch surface is bigger without a second theme.
- Colours come only from `ColorRole`; a raw `Color::Rgb` outside `lazyos-theme` fails CI
  (a grep test in the crate's `tests/`).
- The key bar draws letters in a bracketed style (`[f] files`) because Alt chords never reach
  the TUI: sway sees them first (`plan-installer.md` §8).

## Acceptance criteria

- [ ] every widget has a `TestBackend` snapshot at 80x24 and at the panel's 152x45; a change
      that moves a border fails the suite.
- [ ] one command (the empty-state menu from 001) is re-rendered through `App`; its header,
      body and key bar are the shell every later spec uses.
- [ ] a mouse click on a row activates it; the hit region matches the drawn row at both
      densities (unit test against the layout).
- [ ] a raw colour outside `lazyos-theme` fails `cargo test` (the grep test).
- [ ] motion: a test asserts every effect is ≤300 ms, and `LAZYOS_REDUCED_MOTION=1` renders
      the final frame without an animation tick.
- [ ] witness: `scripts/check-system.sh` FAILs when the session's font lacks the box-drawing
      and block glyphs the widgets use, and when the app's header height/one-frame rule is
      bypassed by a command that draws its own `Frame` (grep in `rust/`).
- [ ] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/01-welcome.md` describes
      the window shape (title, list, key bar) so a person knows where to look.
- [ ] witness: an `AGENTS.md` bullet says why tokens and snapshots exist.

## Tests

- **Unit** (`cargo test`): token mapping (role → style), space arithmetic, key-bar generation from a keymap.
- **Snapshot**: `Row` (normal/selected/dim), `List` (empty, filtered, scrolling), `Card`,
  `Confirm` (cancel selected), `Gauge` at 0/37/100, `EmptyState`, `LogPane` tail.
- **Integration**: an `App` with a scripted key sequence (`Down`, `Down`, `Enter`) selects the
  third row and returns its key.
- **Hardware**: the menu's selected row reads at arm's length in sunlight (the panel is
  glossy); the tablet density is tried folded.

## Risks

- **Taste codified too early** — the tokens are a first draft; the snapshots make a change
  visible, not correct. The maintainer reviews the first three screens (spec 007) as the
  acceptance of the taste.
- **Motion on a 4 GB device** — every effect is opt-in per surface and bounded; the tick is
  throttled, and a test enforces the ceiling.

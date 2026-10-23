# Spec 002 — Design system: tokens, widgets, motion

- **Status**: in progress
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

- The public API is a small vocabulary: `App::new(Header)`, `.design()`, `.keybar()`,
  `.screen(&mut impl Screen)`, `.run(terminal)`. A command does not touch `Frame` directly: it
  draws into `BodyCtx` (area, buffer, design) and answers `key`/`click`/`hit`.
  `App::new(title).body(widget)` from the plan is not the shipped shape: a body closure cannot
  own a list's state between frames, and a screen trait is what lets the integration test drive
  keys and clicks without a terminal. `Screen::key`/`Screen::hit` receive the `Design` so a hit
  region cannot be computed with a density the window was not drawn with.
- Density is chosen once from the tablet state directory (the existing
  `$XDG_RUNTIME_DIR/lazyos-tablet`), so the touch surface is bigger without a second theme.
- Colours come only from `ColorRole`; a raw `Color::Rgb` outside `lazyos-theme` fails CI
  (a grep test in the crate's `tests/`).
- The key bar draws letters in a bracketed style (`[f] files`) because Alt chords never reach
  the TUI: sway sees them first (`plan-installer.md` §8).

## Acceptance criteria

- [x] every widget has a `TestBackend` snapshot at 80x24 and at the panel's 152x45; a change
      that moves a border fails the suite. *(observed: `lazyos-ui/tests/snapshots.rs`, 17
      committed files under `tests/snapshots/`, 18 tests green; `UPDATE_SNAPSHOTS=1` regenerates
      deliberately and a mismatch prints the first differing line.)*
- [x] one command (the empty-state menu from 001) is re-rendered through `App`; its header,
      body and key bar are the shell every later spec uses. *(observed: `lazyos menu` builds an
      `App` around a `Screen`; the pty test still sees "the session menu" and exit 1, and
      `tests/snapshots/app-empty.txt` pins the three bands.)*
- [x] a mouse click on a row activates it; the hit region matches the drawn row at both
      densities (unit test against the layout). *(observed: `lazyos-ui/tests/app.rs` renders the
      shell, clicks the third row at normal and tablet density, and a click on the blank line
      between two touch rows does nothing.)*
- [x] a raw colour outside `lazyos-theme` fails `cargo test` (the grep test). *(observed:
      `lazyos-ui/tests/colours.rs`: no `Color::Rgb` in `lazyos-ui/src` or `lazyos-cli/src` outside
      `design.rs`, and no hex literal in `design.rs`.)*
- [x] motion: a test asserts every effect is ≤300 ms, and `LAZYOS_REDUCED_MOTION=1` renders
      the final frame without an animation tick. *(observed: `app.rs` tests build a shell with
      reduced motion, assert no effect was started, and assert `MOTION_MS` is within 300 ms; the
      running animation is `fx::sweep_in` with an `EffectTimer::from_ms(MOTION_MS, …)` and the
      loop wakes at 10 fps only while it runs.)*
- [x] frame budget (measured, `optimization.md` §13): a test renders the shell at 152x45 for
      1 000 ticks and asserts the per-tick cost stays under the measured ceiling (ratatui's own
      empty-draw floor is 154 µs on this device; the whole tick must stay under 1 ms, which the
      measurement shows is 231 µs with one changing cell and 445 µs only if rows are rebuilt).
      The test fails when a tick formats text that did not change. *(observed: `tests/budget.rs`
      on the device — 424 µs per tick in release with 40 rows, 6.2 ms unoptimised (the debug
      ceiling is 10 ms); the "pinned" snapshot never changes, and `tests/no_alloc.rs` measures
      **0 allocations per frame** with a counting allocator, which is the allocation half of the
      same rule — a `format!` per row would be 40.)*
- [x] witness: `scripts/check-system.sh` FAILs when the session's font lacks the box-drawing
      and block glyphs the widgets use, and when the app's header height/one-frame rule is
      bypassed by a command that draws its own `Frame` (grep in `rust/`). *(observed: the
      framework section reads `lazyos --glyphs` and asks fontconfig for every codepoint in one
      call — 12/12 covered here — and greps `lazyos-cli/src` for `ratatui::Frame`/`&mut Frame`;
      the same grep is `lazyos-cli/tests/frame.rs`.)*
- [x] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/01-welcome.md` describes
      the window shape (title, list, key bar) so a person knows where to look. *(observed:
      § *The windows the framework draws* — the three bands, the keys, the click, the tablet
      rows, the Nerd Font pictures and reduced motion.)*
- [x] witness: an `AGENTS.md` bullet says why tokens and snapshots exist. *(observed:
      *The design system is a vocabulary, and the tests are what keep it one*, with the measured
      numbers.)*

## Tests

- **Unit** (`cargo test`): token mapping (role → style), space arithmetic, key-bar generation from a keymap.
- **Snapshot**: `Row` (normal/selected/dim), `List` (empty, filtered, scrolling), `Card`,
  `Confirm` (cancel selected), `Gauge` at 0/37/100, `EmptyState`, `LogPane` tail.
- **Integration**: an `App` with a scripted key sequence (`Down`, `Down`, `Enter`) selects the
  third row and returns its key.
- **Hardware**: the menu's selected row reads at arm's length in sunlight (the panel is
  glossy); the tablet density is tried folded. *(pending: needs the rebuilt image on the card;
  the click and the density change are covered by `tests/app.rs`, what remains is the judge.)*

## Risks

- **Taste codified too early** — the tokens are a first draft; the snapshots make a change
  visible, not correct. The maintainer reviews the first three screens (spec 007) as the
  acceptance of the taste.
- **Motion on a 4 GB device** — every effect is opt-in per surface and bounded; the tick is
  throttled, and a test enforces the ceiling.

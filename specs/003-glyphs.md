# Spec 003 — The Nerd Font glyph alphabet

- **Status**: merged
- **Phase**: 2
- **Depends on**: 001
- **Model**: `opencode-go/gpt-6-luna` — a data table checked against a font, plus a sweep of
  every call site; long context and cheap iterations matter more than taste. Fallback
  `opencode-go/claude-haiku-5-5`.
- **Branch**: `spec/003-glyphs`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §6, `AGENTS.md` *Icons first, never forced*

## Problem

The row alphabet today is `UI_I_*` in `lib-rofi.sh`: Font Awesome codepoints, which foot
cannot draw (its fallback is Symbols Nerd Font Mono — a tofu box, documented for fzf and the
welcome page). A Rust row that inherits those codepoints would draw empty boxes in the very
surface the port is for, and nothing in the build would notice.

**Corrected while building this** (the criterion is kept, the premise is not): foot *does*
search fontconfig's dynamic fallback (`foot.ini(5)`), so a Font Awesome codepoint renders here
once `fonts-font-awesome*` is installed - measured in the session with an `f1eb` (wifi) on a
probe window, next to Codicons that rendered from `fonts-nerd-symbols` at the same time. The
alphabet is Nerd Font anyway, for the reason that survives the measurement: a row's picture
has to come from the one symbol font the image guarantees and occupy one cell, not from
whichever family fontconfig happens to answer with (and at that family's size).

## Scope

**In:**

- `enum Glyph` in `lazyos-ui`, one variant per existing `UI_I_*` name (terminal, globe,
  folder, image, video, document, window, camera, capture, shield, check, search, gear,
  download, code, user, clipboard, power, network, bluetooth, audio, brightness, keyboard,
  theme, display, …) plus the names the new surfaces need.
- A **table** mapping variant → codepoint, generated and filtered by
  `scripts/make-glyphs.sh` (sibling of `make-emojis.sh`): it asks fontconfig
  (`fc-match`/`fc-query --format=%{charset}`) which codepoints the installed symbol font
  actually draws, drops what it cannot, and writes a committed Rust source file plus a
  sidecar pin (font family + SHA256), exactly like `emojis.tsv`/`emojis.font`.
- `Glyph::draw(area, buf)` and `Row` integration (spec 002): the glyph column exists only when
  the glyph is present in the table, so a row without one keeps its text aligned.
- The **fallback rule**: an unknown or unfiltered name renders the plain text label and logs
  once at debug, never a box and never a panic.

**Out:** the PNG icon theme (`scripts/make-icons.sh` keeps feeding rofi, toasts, pcmanfm and
the sixel surfaces); the sixel pictures (spec 009/012 use the PNGs directly).

## Design

- The font is `fonts-nerd-symbols` (DFSG-stripped: Octicons, Codicons, Devicons, Seti,
  Powerline). The table therefore draws from those planes, **not** Font Awesome or `nf-md`
  plane 15 — the same fact the prompt bullet already records.
- The generated file carries the codepoint, its name and the font's pin; `check-system.sh`
  compares the pin with the *installed* font, so a font upgrade that drops a glyph is a
  report, not a silent hole.
- The alphabet is used by `Row`, the TaskView checklist, the notification card and the key
  bar; the installer TUI links the same crate.

**Four decisions the build made, each because the alternative broke a rule of the crate:**

1. **`Glyph` has no `draw`.** The spec asked for `Glyph::draw(area, buf)`; the widgets draw
   from state that is built once per change and copy cells per frame, with **0 allocations
   per frame** (`tests/no_alloc.rs`, spec 002). A `Glyph` that drew would either allocate a
   string per frame or duplicate `put`. The alphabet is therefore resolved where the row is
   built: `Row::glyph(Glyph)` stores `Glyph::cell()`, and `App`/`ListState` stay the only
   things that write into the buffer.
2. **The column belongs to the list, not to the row.** The spec's wording ("the column exists
   only when the glyph is present") is the failure it also forbids: a per-row column leaves
   the rows without a picture a cell to the left. `ListState` reserves the widest cell among
   its rows when the rows change - all of them, not the filtered ones, so typing a filter
   does not move the labels - and a row with no picture spends it in spaces.
3. **The debug line is opt-in.** "Logs once at debug" became `LAZYOS_GLYPH_DEBUG=1` on stderr:
   a TUI that writes to the terminal it is drawing on corrupts itself. The machine-readable
   answer (`lazyos --glyph-font`) is what the witness reads, and the fallback is a documented
   state, not an error.
4. **`--glyphs` carries the name.** It now prints `U+XXXX<TAB>name` and includes the frame's
   characters *and* the alphabet, so one check covers both and a failure names the glyph
   (`Symbols Nerd Font Mono does not carry: bluetooth U+F293`) instead of a bare codepoint.

## Acceptance criteria

- [x] every `Glyph` renders a visible glyph with the image's fonts, verified against the
      font's charset by `make-glyphs.sh` and by `check-system.sh` on the device.
      **Observed**: 101 names; 93 draw a codepoint and 8 have none in the package, which is
      the next criterion. `make-glyphs.sh` filtered them against
      `SymbolsNerdFontMono-Regular.ttf` (Nerd Fonts 3.5.1); on the device,
      `check-system.sh` answered `the rows' glyphs render (105 codepoints, every one in
      Symbols Nerd Font Mono)` - 12 frame characters plus 93 alphabet codepoints.
- [x] a name the font does not draw is filtered out at generation time; the variant falls back
      to its text label and a debug line, never a tofu box.
      **Observed**: `scripts/make-glyphs.sh` writes `draw: None` when the character map does
      not carry the codepoint (Keyboard, Wifi, Bluetooth, Battery, Mouse, Calculator, Qr,
      Crop - all Font Awesome-only names), and `Glyph::cell()` answers the short token
      (`kbd`, `wifi`, `bt`, `bat`, `pad`, `=`, `qr`, `crop`). `check-system.sh` fails when a
      fallback is not short text; the integration test runs the generator against a fixture
      charset missing exactly `U+EA85` and asserts that `Terminal` - and only it - loses its
      codepoint.
- [x] rows with and without a glyph line up (the column is reserved by the layout, not by the
      glyph).
      **Observed**: `widgets::tests::a_row_without_a_glyph_lines_up_with_one_that_has_it`
      renders a pictured row, a bare row and a text-token row and asserts all three labels
      start at column 5; the `app-menu` snapshot shows the same on screen (`bt Bluetooth`
      beside the codepoint rows).
- [x] witness: `scripts/check-system.sh` FAILs when `fonts-nerd-symbols` is missing, when the
      committed sidecar pin differs from `fc-query` of the installed font, and when a codepoint
      in the table is absent from the font.
      **Observed**: four lines in the framework section. Exercised with stubs: a fontconfig
      answer that is not the family FAILs, a stale `sha256` FAILs (`the alphabet's pin is for
      another font`), a codepoint the font lacks FAILs naming it (`does not carry: face
      U+1F600`), a fallback that is not short text FAILs, and a pre-003 binary FAILs
      (`--glyphs/--glyph-font printed nothing`). The real device answers four `ok` lines in
      0.16 s.
- [x] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/03-shortcuts.md` (or
      `01-welcome.md`) explains that the list glyphs are text and what a missing font looks
      like.
      **Observed**: `01-welcome.md` § *The windows the framework draws* (why Nerd Font, the
      eight text names, `check-system.sh`); `06-troubleshooting.md` § *Icons render as boxes*
      gained the symptom (`fc-match`, the SHA256, `apt install fonts-nerd-symbols`, log out
      and in) and corrects the page's old claim that foot has no fallback.
- [x] witness: an `AGENTS.md` bullet says the TUI alphabet is Nerd Font, distinct from the
      rofi/Pango `UI_I_*` alphabet.
      **Observed**: the bullet after the design-system one ("A row's picture is a Nerd Font
      codepoint, and the table behind it is generated"), including the measured correction
      about foot's fallback and the eight names that are text.
- [x] unit tests: `every_variant_has_a_codepoint`, `no_duplicate_codepoints`,
      `fallback_is_text_not_box`, `sidecar_matches_table`.
      **Observed**: `glyphs::tests::{every_variant_has_an_entry_and_the_table_adds_nothing,
      every_variant_draws_a_codepoint_or_a_text_token, no_two_names_draw_the_same_codepoint,
      no_codepoint_comes_from_plane_fifteen, the_fallback_is_ascii_text_not_a_box,
      the_generated_table_is_ascii, the_pin_matches_the_table,
      names_are_unique_and_are_the_lower_case_form}` - eight, the four named ones plus the
      drift and ASCII guards they imply.

## Tests

- **Unit** (`cargo test`): table integrity (unique codepoints, no plane-15 entries), text
  fallback, the pin against the table, and the enum against the table in both directions.
- **Snapshot**: a list mixing glyph rows and text-only rows (`app-menu`, `list`,
  `list_filtered`, `list_without_focus`, `card` regenerated); both align.
- **Integration**: `make-glyphs.sh` run against a fixture charset produces the expected table;
  a charset missing one codepoint drops exactly that one
  (`tests/glyph_script.rs`), and `make-glyphs.sh --check` proves the committed table is what
  the script writes (skipped where `fonts-nerd-symbols` is not installed).
- **Hardware**: the check-system lines above were run on the device against the branch's
  binary; the glyph grid that chose the codepoints was rendered with `pango-view` from the
  installed font and looked at row by row (`grid.png`, and a probe window in the real session
  confirmed foot draws all of them, including `U+23FB`).

## Risks

- **A glyph that exists in fontconfig's answer but renders as tofu anyway** (a fallback family
  without the glyph) — the check asks the *installed symbol font's own* character map for the
  alphabet (not the union: that is what the frame's characters get, because they are text) and
  the manual says how to install the missing set.
- **A font upgrade silently changes a glyph's shape** — the SHA256 pin notices the upgrade and
  the report says to regenerate; it cannot notice a codepoint whose drawing changed but stayed
  in the map, which is why the generator's candidate table carries the upstream Nerd Fonts name
  in a comment beside every codepoint.

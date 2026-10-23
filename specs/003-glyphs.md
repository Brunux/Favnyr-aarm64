# Spec 003 — The Nerd Font glyph alphabet

- **Status**: draft
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

## Acceptance criteria

- [ ] every `Glyph` renders a visible glyph with the image's fonts, verified against the
      font's charset by `make-glyphs.sh` and by `check-system.sh` on the device.
- [ ] a name the font does not draw is filtered out at generation time; the variant falls back
      to its text label and a debug line, never a tofu box.
- [ ] rows with and without a glyph line up (the column is reserved by the layout, not by the
      glyph).
- [ ] witness: `scripts/check-system.sh` FAILs when `fonts-nerd-symbols` is missing, when the
      committed sidecar pin differs from `fc-query` of the installed font, and when a codepoint
      in the table is absent from the font.
- [ ] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/03-shortcuts.md` (or
      `01-welcome.md`) explains that the list glyphs are text and what a missing font looks
      like.
- [ ] witness: an `AGENTS.md` bullet says the TUI alphabet is Nerd Font, distinct from the
      rofi/Pango `UI_I_*` alphabet.
- [ ] unit tests: `every_variant_has_a_codepoint`, `no_duplicate_codepoints`,
      `fallback_is_text_not_box`, `sidecar_matches_table`.

## Tests

- **Unit** (`cargo test`): table integrity (unique codepoints, no plane-15 entries), text fallback.
- **Snapshot**: a list mixing glyph rows and text-only rows; both align.
- **Integration**: `make-glyphs.sh` run against a fixture charset produces the expected table;
  a charset missing one codepoint drops exactly that one.
- **Hardware**: the session's list and the TaskView checklist read correctly in foot and in a
  screenshot (`grim` + the PNG is the evidence).

## Risks

- **A glyph that exists in fontconfig's answer but renders as tofu anyway** (a fallback family
  without the glyph) — the check asks `fc-list :charset=<cp>` for *families*, and the manual
  says how to install the missing set.

# Spec 004 — The theme engine

- **Status**: draft
- **Phase**: 2
- **Depends on**: 001
- **Model**: `opencode-go/deepseek-v4-pro` — a schema, a set of generators, atomic and
  idempotent writes, and a CLI surface: the correctness is in the state machine. Fallback
  `opencode-go/glm-5.3`.
- **Branch**: `spec/004-theme-engine`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §8 (themes), §9 (cycle keys), §16 phase 5

## Problem

The palette is copied by hand into `foot.ini`, `style.css`, `solarized.rasi`, `mako/config`,
`starship.toml`, btop/lazygit/nvim/fzf configs and `brand` assets. Changing one value means
seven edits, and a theme switch is impossible without a script that rewrites files under the
user's home — the exact class of change that already bit this repo (a hand edit that looked
applied, an icon cache that never refreshed).

## Scope

**In (`lazyos-theme` + `lazyos theme`):**

- **`theme.toml`**: the palette (16 ANSI + semantic roles), the accent pair, wallpaper list,
  icon-set hint, and the surface table. Validated on load (contrast floor, required roles,
  known keys); an unknown key is an error naming the line.
- **Built-ins**: solarized (the current session, byte-identical output at first install) plus
  the seven from the plan (Catppuccin Mocha, Tokyo Night, Rose Pine, Nord, Gruvbox, Everforest,
  Kanagawa) as data files.
- **Generators**: `foot.ini`, waybar `style.css`, the legacy rofi `solarized.rasi` (until rofi
  leaves), mako `config` (until spec 013), btop, lazygit, nvim, fzf, starship, swaylock, and
  the wallpaper selection. Each generator is a pure function `Palette -> String`.
- **Writes**: every generated file goes through `tmp + rename`, and regeneration is
  **idempotent**: a second run with the same theme produces zero-byte diffs. A file the user
  edited by hand is never silently overwritten: `lazyos theme set` prints the diff and asks
  (spec 002 `Confirm`), `--force` answers.
- **Commands**: `lazyos theme list`, `set NAME`, `validate`, `preview` (palette card, sixel
  optional later), `current`.
- **Cycle keys**: `Alt+Shift+T` (next theme) and `Alt+Shift+W` (next wallpaper of the active
  theme), reserved in `etc/sway/config.d/` with the description comment `rofi-keys` reads.

**Out:** the extra themes' *wallpaper art* (spec 013), the greeter (013), the notification
daemon (013), the icon theme (the PNGs stay as they are).

## Design

- The active theme is one line in `~/.config/lazyos/theme` (name + version); generated files
  carry a header with the theme name, so `lazyos theme current` can verify what a surface
  actually contains.
- `check-system.sh` regenerates into a temp directory and compares with the installed files
  (**the idempotence witness**), and fails when a surface lost its header or a role.
- The current session's colours are the solarized built-in's expected output; the first `set`
  is therefore a no-op diff on a healthy machine — the fastest way to spot a generator bug.
- The cycle keys run `lazyos theme cycle` and raise no window (a one-shot write + a
  notification card), so they cost nothing when unused.

## Acceptance criteria

- [ ] `lazyos theme set solarized` on a stock card changes nothing (empty diff on every
      surface); `set catppuccin` changes the footer, the bar, mako, btop and starship, and
      `set solarized` restores byte-identical files.
- [ ] `lazyos theme set` refuses an unknown name (exit 2, list shown); `validate` rejects a
      palette below the contrast floor with the failing pair named.
- [ ] an edited hand-written generated file is reported and not overwritten without
      confirmation; `--force` overwrites and says so.
- [ ] `Alt+Shift+T`/`Alt+Shift+W` cycle and persist; a session restart keeps the choice.
- [ ] witness: `scripts/check-system.sh` FAILs when the active theme file is missing, when the
      regeneration diff is non-empty, when a surface lacks the palette header, or when the two
      cycle bindings are absent.
- [ ] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/04-software.md` § Themes
      says how to switch, what changes, and how to undo a hand edit.
- [ ] witness: an `AGENTS.md` bullet records that palette files are generated and the
      idempotence check is the witness.
- [ ] unit tests: schema parsing (valid/invalid), every generator's golden output per built-in,
      idempotence, atomic failure injection, theme resolution and cycle order.

## Tests

- **Unit** (`cargo test`): `theme.toml` round-trip and rejection cases; `generator_<surface>_is_stable`
  (golden files under `tests/fixtures/themes/`); contrast validation; name resolution.
- **Snapshot**: the palette card and the confirm dialog for a hand-edited file.
- **Integration**: generate into a temp `HOME`, run twice, assert an empty diff; then append a
  comment to one file and assert `set` refuses without confirmation.
- **Hardware**: switch to each theme and look at foot, waybar, mako, btop, lazygit, nvim; the
  panel's gamma (not covered by night light) is judged by eye; a switch during a session does
  not require a logout (only foot windows opened *after* it carry the new palette — the manual
  says so).

## Risks

- **A generator drifts from the hand-written file** and the first switch breaks a surface —
  the byte-identical first `set` and the golden tests are the catch.
- **A write interrupts a session** (rename across filesystems) — the temp file is created in
  the target directory, and a test injects a failure mid-write.

# Spec 001 — The `lazyos` binary and its host

- **Status**: merged
- **Phase**: 2 (framework core, plan §16)
- **Depends on**: none
- **Model**: `opencode-go/deepseek-v4-pro` — cross-crate architecture and a dispatch table that
  every later spec multiplies; the code is small but the shape is load-bearing. Fallback
  `opencode-go/kimi-k2.7-code`.
- **Branch**: `spec/001-core`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §3 (the frame), §7 (keys), §12 (files)
- **Old paths**: none replaced yet; the binary is added beside the scripts

## Problem

The session is ~64 scripts that each own their terminal handling, their colours and their error
story: `rofi-*` prints a menu, `bar-*` opens a popup, `lib-rofi.sh` carries the plumbing, and
every one of them restores the terminal on its own terms. Nothing can be tested from outside a
Wayland session, and a second surface (the bar, a notification, the installer) re-implements
the same row, the same palette and the same "it failed" card.

There is no process that can be *the* UI: one binary that the bindings, the `.desktop` entries
and the agent can call.

## Scope

**In:**

- The Rust workspace becomes `lazyos-cli` (the binary), `lazyos-ui` (widgets, spec 002) and
  `lazyos-theme` (spec 004). This spec builds only the bones: the binary, the dispatch, the
  host, the error card, the version.
- **Dispatch by `argv[0]`**: launching `lazyos menu`, `lazyos menu files` or a symlink named
  `rofi-files` resolves to the same handler. The table is one `enum Command` + one
  `fn dispatch(argv0, args)`. Unknown names exit 2 with the list of known ones.
- **The host**: when the process has no usable terminal (`WAYLAND_DISPLAY` set and no
  `LAZYOS_UI`), it opens `foot --app-id=lazyos-<shape> --window-size-pixels=<shape size>
  -e <self> …` and returns; inside (`LAZYOS_UI=1`) it runs inline. The shape (pick/task/full)
  is a field of the command table, not a decision each command makes.
  *Corrected while building it, with the measurement:* the floating/centring step is **not** a
  `swaymsg` from the app. The app runs before foot's first commit, so a criteria match at that
  moment finds no node (the window stayed tiled, `floating=auto_off`, 0x0), and a `resize set`
  at map time is overwritten by foot's own commit (a 900x600 rule produced a 700x496 window
  while the centre still computed from 900). The size is therefore foot's
  `--window-size-pixels`, and sway floats and centres the app-id from
  `config.d/18-framework-windows.conf` - which is what `plan-ui.md` §3 already specified.
- **Terminal hygiene**: raw mode, alternate screen, cursor, mouse and the sixel protocol are
  restored on every exit path, including panic (`std::panic::set_hook`) and SIGINT. Nothing
  writes to the pty after the restore.
- **Paths**: `~/.config/lazyos/`, `~/.local/state/lazyos/`, `~/.cache/lazyos/` (XDG respected,
  created on demand), and one `--version` that reads the committed `VERSION` at build time.
- `lazyos menu` exists but only prints an empty-state card ("the menu arrives in spec 007"), so
  the host, the frame and the error card are exercised by a real command.

**Out:** every actual menu (specs 007-012), the theme engine (004), the design system widgets
(002), the glyphs (003), MCP (008).

## Design

- Crates: `lazyos-cli` depends on `lazyos-ui`/`lazyos-theme`; neither depends on the CLI, so
  the installer TUI (`plan-installer.md`) can link the same widgets.
- The command table is static data (`&'static [CommandSpec]`), so `lazyos --help`,
  `lazyos mcp --tools` (spec 008) and the check-system witness read the **same** source.
- `LAZYOS_UI` is inherited into the foot child; a nested invocation never spawns a second
  window (the guard is in the host, not in the commands).
- Exit codes: `0` success/cancel, `1` the person pressed Escape (nothing happened), `2` usage,
  `3` the action failed (with the card already shown). The symlinked shell callers can keep
  using `|| exit 0` for Escape.

## Acceptance criteria

- [x] `lazyos --version` prints the `VERSION` file's value; `lazyos nope` exits 2 and lists
      `menu`; `lazyos menu` from sway opens one floating foot window, and from inside a
      terminal (`LAZYOS_UI=1`) runs in that terminal.
      *Observed:* `tests/cli.rs` (version, unknown, no-command, fake-foot argv) and on the
      Hayato: `swaymsg -t get_tree` showed `lazyos-pick floating=user_on 234,175 898x596`
      (centred on the 1366x912 panel), one window, and the parent returned before it opened.
- [x] A symlink named `rofi-files` → `lazyos` reaches the same handler as `lazyos menu files`.
      *Observed:* `command::tests::dispatch_resolves_symlink_names` (both spellings resolve to
      `menu files`); the process-level `a_symlink_resolves_to_its_command` runs the binary
      through a link named `lazyos-menu` and reaches `menu`, not "unknown command".
- [x] A forced panic inside the frame restores the cursor and leaves the alternate screen; the
      window that hosted it stays usable.
      *Observed:* `a_forced_panic_restores_the_terminal` runs `LAZYOS_FORCE_PANIC=1` in a real
      pty and asserts the restore sequence arrives after the alternate screen was entered
      (exit 101); `reports/2026-10-23-framework-001.md`.
- [x] Escape returns 1 with nothing written to disk; a failed command returns 3 and shows the
      error card (the fallback when no foot exists is stderr, still exit 3).
      *Observed:* `escape_returns_one_and_draws_the_empty_state`, `a_menu_writes_nothing_to_disk`
      (the temporary `$HOME` is empty afterwards), `a_failed_command_shows_the_card_and_exits_three`
      (`lazyos menu bogus` draws the card and exits 3) and `without_a_terminal_the_reason_goes_to_stderr`.
- [x] witness: `scripts/check-system.sh` FAILs when the `lazyos` binary is missing, when a
      symlink in the migrated set does not resolve, and when `lazyos --help` does not list a
      subcommand whose symlink exists.
      *Observed:* the new `==> The framework` section; removing `/usr/bin/lazyos-menu` on the
      device turned the symlink line FAIL (reported in `reports/2026-10-23-framework-001.md`).
- [x] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/02-rofi.md` says the menus
      are one program now and how to call it from a terminal.
      *Observed:* § *The menus are becoming one program*.
- [x] witness: an `AGENTS.md` bullet records the argv[0] dispatch and the `LAZYOS_UI` guard.
      *Observed:* the bullet after the framework's direction bullet.
- [x] unit tests: `dispatch_resolves_symlink_names`, `dispatch_rejects_unknown`,
      `paths_honour_xdg`, `version_matches_file`, `host_guard_does_not_nest`.
      *Observed:* `cargo test --workspace` (19 unit tests in `lazyos-cli`, 5 in `lazyos-ui`,
      2 in `lazyos-theme`, 13 process tests).

## Tests

- **Unit** (`cargo test`): the command table (unique names, no duplicate symlink), argv[0] resolution
  (basename, `rofi-` prefix, path suffix), XDG path construction with an injected environment.
- **Snapshot**: the error card and the empty state render on a `TestBackend`; one test per
  state.
- **Integration**: run the binary with `LAZYOS_UI=1` and a fake `foot` on `PATH`; assert the
  child's argv (the host passes the resolved command, not the symlink) and that the parent
  returns immediately.
- **Hardware**: open and close the menu ten times from the binding; nothing stays in
  `ps`/`wpctl`, the terminal is clean after a panic, and `swaymsg -t get_tree` shows no leftover
  `lazyos-*` view.

## Risks

- **A second window on every key press** if the guard misses an inherited case — the
  integration test with a fake `foot` is the catch.
- **A symlink that resolves to the wrong command** (the old names are not uniform) — the
  check-system line compares the table with the installed links, and the unit test refuses
  duplicate names.

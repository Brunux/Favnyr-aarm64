# Spec 005 — The task surface (TaskView)

- **Status**: draft
- **Phase**: 2
- **Depends on**: 002
- **Model**: `opencode-go/kimi-k2.7-code` — a state machine with process control, stop/reattach
  and a snapshot per state; dense implementation with tests is exactly its shape. Fallback
  `opencode-go/deepseek-v4-pro`.
- **Branch**: `spec/005-taskview`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §10 (TaskView), §11 (the loop), §14 (perf)

## Problem

The kit's slow actions (`busy` cards in `lib-rofi.sh`: the Wi-Fi scan, the Bluetooth scan, an
apt install, a dictation) hand their result to a toast that expires. A person who looked away
misses the result, a failure leaves a card with no reason, and nothing can be stopped from the
window except by killing a pkill pattern. The installer has the same problem at a larger scale.

The plan's answer is a task surface that **keeps the person in the loop**: the action ends by
showing its own result, in the same window, with the log it produced.

## Scope

**In (`lazyos-ui::task` + `lazyos task`):**

- **Model**: `Task { title, steps: Vec<Step>, state, progress, log, stoppable, started_at }`;
  `Step` has a label, a state (waiting/running/ok/failed/skipped) and optional detail. State
  transitions are the only way to change the view.
- **View**: header (title + elapsed), checklist (glyph per step), spinner + gauge while
  running, a **live log tail** (last N lines, follow), footer with the one action that applies
  (`[s] stop`, `[c] copy`, `[r] rerun`, `[q] close`) and the result block when it ends.
- **Process execution**: a task runs one child at a time, captures stdout/stderr line by line,
  and owns the stop contract: **SIGINT first, SIGTERM after 2 s**, then `Stopped` (not
  `Failed`); the exit status of a stopped child is never reported as a result.
- **Reattach**: state is a file in `~/.local/state/lazyos/tasks/<id>.json` (phase, percent,
  pid, log path, stoppable), written atomically; `lazyos task attach` reopens a running task
  after the window closed. A pid whose `/proc/<pid>/comm` no longer matches is not touched.
- **The bridge**: while a task runs, it also writes the existing `busy` protocol
  (`$XDG_RUNTIME_DIR/lazyos-busy/<tag>`) so the bar's spinner keeps working until the bar is
  ported (spec 014), including the `BUSY_STOP` line the click runs.
- **Results inline**: success renders the result in the window (path, count, URL, table);
  failure renders the last `ERROR:`-style line in `danger`, with the full log one key away.

**Out:** the actual tasks (Wi-Fi, apt, dictation — their pickers' specs), the notification
daemon, the installer's screens (they *use* this).

## Design

- A task is **not stoppable by default**; the spec of each task says whether it is. Format,
  install and DNS in the shell menus are deliberately not stoppable — the same rule carries
  over, and the view says so.
- The log is a ring buffer in memory plus a file; the view reads each frame, so a killed window
  does not lose the reason.
- The tick is 100 ms only while a task runs (spinner/progress); idle is event-only, so a task
  left open costs nothing.
- `Task::spawn(cmd)` takes an argv array, never a shell string, so a pattern from a menu cannot
  become a shell injection; the stop pattern is data, not a command line.

## Acceptance criteria

- [ ] a scripted task (3 steps, 2 s each) renders waiting → running → done with the checklist,
      a moving spinner and a gauge; the snapshots are asserted.
- [ ] a task that fails on step 2 shows step 1 ok, step 2 failed with its stderr line, and the
      window stays open on the result.
- [ ] pressing `s` on a stoppable task ends it as `Stopped` with the last line kept; the same
      key on a non-stoppable task says why and does nothing.
- [ ] closing the window during a task leaves the task running and reattachable; `attach`
      after it finished shows the saved result.
- [ ] the bar spinner appears and disappears with the task, and a click on it stops a stoppable
      task exactly like `s`.
- [ ] witness: `scripts/check-system.sh` FAILs when a task declared stoppable has no STOP
      handler, when a non-stoppable task command lists `pkill`, and when a task writes a
      `busy` tag without the matching `BUSY_STOP` rule.
- [ ] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/02-rofi.md` describes the
      task screen, the stop key and where the log lives.
- [ ] witness: an `AGENTS.md` bullet says a slow action ends by showing its result.
- [ ] unit tests: state machine transitions (no result after stop, no stop after done),
      log ring, atomic state file, PID identity check, `BUSY_STOP` mirroring.

## Tests

- **Unit** (`cargo test`): `stop_is_not_failure`, `late_output_after_stop_is_dropped`,
  `state_file_is_atomic`, `pid_identity_rejects_reuse`, `busy_tag_matches_stop`.
- **Snapshot**: queued/running/done/failed/stopped, with and without a stoppable action.
- **Integration**: run `/bin/sh -c` fixtures (success, failure, long sleep, ignore-SIGINT) in
  a temp state dir and assert the final state, exit code and log tail.
- **Hardware**: run a real `nmcli device wifi list` and an `apt install` through it; stop the
  scan and confirm the bar spinner disappears and no `nmcli` survives.

## Risks

- **A stopped child leaves a half-written artifact** (the note db, a download) — the task's
  spec owns cleanup; this spec only guarantees the state says `Stopped` and the log says what
  was interrupted.
- **Reattach across a reboot** — the state file carries the boot id; a mismatch shows "did not
  finish" instead of pretending.

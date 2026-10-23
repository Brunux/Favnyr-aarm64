# Spec 006 — Cards: notifications and in-window messages

- **Status**: draft
- **Phase**: 2
- **Depends on**: 002, 003
- **Model**: `opencode-go/claude-haiku-5-5` — a small, well-shaped surface over an existing
  daemon (mako stays), mostly fixtures and icon resolution. Fallback
  `opencode-go/minimax-m3.1-flash`.
- **Branch**: `spec/006-notifications`
- **Plan**: [`docs/plan-ui.md`](../docs/plan-ui.md) §5.6 (notifications), §13 (the AI surface)
- **Old paths**: `notify-send` call sites in `~/.config/bin/*` keep working until spec 013

## Problem

Notifications are already `mako` and work, but every script has to remember the house rules:
an icon name the LazyOS theme draws, the `x-canonical-private-synchronous` tag for OSD/busy,
a progress hint, the right urgency, and never a secret in the body. Today a text-only card is
invisible in review ("the row worked"), and the AI/desktop surfaces need the same card as an
in-window panel, not a toast.

This spec builds one card vocabulary used by both: `Card::send()` (through mako today) and
`Card::render()` (inside a `lazyos` window), with the same fields and the same icon lookup.

## Scope

**In (`lazyos-ui::card` + `lazyos notify`):**

- **Fields**: title, body, urgency (low/normal/critical), icon (a `Glyph` or a theme PNG name),
  progress (0-100), group tag, action (a command to run on click), timeout.
- **Icon resolution**: glyph → spec 003 table; name → `/usr/share/icons/LazyOS/...` (the same
  roots mako's `icon-path` uses). A missing icon logs one line and renders text — it never
  silently drops the picture (the rule that made `notify-send -i folder` look correct while
  the card stayed blank).
- **Secrets rule**: a helper `Card::secret()` that copies to the clipboard and sends a card
  whose body says only that it is copied; the value never enters the body (and therefore never
  mako's history).
- **In-window**: `Card` as a widget for results and errors inside a task or a menu; same
  urgency colours, same icon.
- **`lazyos notify`**: `send`, `say` (a one-line result), `progress` (the OSD path: one card,
  updated in place by tag).

**Out:** the notification daemon (`mako` stays until spec 013), notification history beyond
mako's own, sound.

## Design

- The mako backend writes a `notify-send`-compatible call (a small D-Bus message via
  `zbus`/`notify-rust`), not a shell-out; the tag and hints are the same strings the shell
  menus use, so the bar and OSD behaviour does not change.
- The card's style comes from the theme (spec 004): role colours, not hex.
- `--icon=auto` derives the icon from the command/result type, so a new call site cannot
  forget it.

## Acceptance criteria

- [ ] `lazyos notify send "Saved" --icon image --body <path>` shows a card with the theme's
      image icon; the same call without an icon shows a text card and logs the fallback.
- [ ] the OSD group (`x-canonical-private-synchronous:lazyos-osd`) still updates one card in
      place, with the progress bar, when called in a loop.
- [ ] `Card::secret()` puts the value on the clipboard and the card names no part of it.
- [ ] an in-window card renders the same icon and colours; a long body truncates with an
      ellipsis instead of resizing the window.
- [ ] witness: `scripts/check-system.sh` FAILs when a `notify-send` call in `~/.config/bin`
      names an application but no icon, when an icon name used by the binary is absent from
      the LazyOS theme, and when mako's `icon-path` loses the theme root.
- [ ] witness: a paragraph in `uis/sway/files/usr/share/doc/lazyos/05-system.md`
      § *Notifications* says where they appear, how to dismiss them, and that history is kept.
- [ ] witness: an `AGENTS.md` bullet says the card is one vocabulary for toast and window.
- [ ] unit tests: icon resolution order (glyph, theme, fallback), secret body, truncation,
      group tag propagation, urgency mapping.

## Tests

- **Unit** (`cargo test`): `icon_resolution_prefers_glyph`, `missing_icon_is_text_and_logs`,
  `secret_never_in_body`, `body_truncates_to_width`, `tag_is_stable`.
- **Snapshot**: card at three urgencies, with/without icon, with progress.
- **Integration**: a fake notification service on a private bus (a stub over the D-Bus name)
  receives exactly the expected hints; the mako config parses the same file.
- **Hardware**: the volume/brightness OSD still replaces one card, a critical card shows over
  a fullscreen video, and the icon is visible (not blank).

## Risks

- **A `notify-send` call kept in a script but bypassed by the new path** — the check greps
  both the scripts and the Rust call sites for an application name without an icon.
- **mako's history keeps a secret** — the secret helper is the only allowed password path and
  it is unit-tested.

# Coding agents

Omarchy ships an "agents" feature: a bar panel with subscription usage meters,
plus a set of scripts that keep several accounts per provider and can switch
accounts automatically near a rate limit. This image carries the local half of
that - launching and account profiles - and not the usage meters.

**The menu**: `rofi-agents`, on **Alt+Ctrl+Shift+A** (Omarchy's `Super+Shift+Ctrl+A`),
or "Agents" in the root menu / the launcher.

| Row | Does |
|---|---|
| `<Agent>` | runs it in foot, in `~/Work` when that directory exists (agents remember trust per directory, and they refuse `$HOME`) |
| `<Agent> — install …` | `npm install -g --prefix ~/.local <package>` in a terminal, so nothing is written to `/usr/local` and no root prompt appears |
| `<Agent> accounts…` | create, switch and remove accounts (see below) |
| `Set the default agent…` | marks the agent shown with `●` |
| `Open a terminal in ~/Work` | a shell where the agent binaries are on `PATH` |

Every row carries the glyph the rest of the session's menus use (`lib-rofi.sh`):
a code sign for an agent, two people for its accounts, a download arrow for an
install, a gear for the default, a terminal for the shell, a bin for removing an
account and an undo arrow for going back. The default agent is still the one
marked `●`, next to its glyph.

The agents it knows about, and their packages:

| Agent | Binary | npm package |
|---|---|---|
| Claude Code | `claude` | `@anthropic-ai/claude-code` |
| Codex | `codex` | `@openai/codex` |
| OpenCode | `opencode` | `opencode-ai` |
| Gemini CLI | `gemini` | `@google/gemini-cli` |
| GitHub Copilot | `copilot` | `@github/copilot` |

`node` and `npm` are already installed (they arrived for exactly this), and
`~/.local/bin` is on `PATH` for every shell, so a CLI installed here is visible
in the next terminal without logging out.

## Accounts

Claude Code and Codex both support an alternative configuration directory -
`CLAUDE_CONFIG_DIR` and `CODEX_HOME` - and that is all an "account" is here: a
directory per account under `~/.local/share/lazyos/agents/<agent>/<account>/`,
with the last one used recorded next to it. Creating one is empty on purpose:
the first launch in that account runs the CLI's normal sign-in and the profile is
created there, so two subscriptions never share a session or a token.

From the shell, the same thing without the menu:

```sh
eval "$(rofi-agents --env 'Claude Code' work)"   # export CLAUDE_CONFIG_DIR=…
claude
```

OpenCode, Gemini and Copilot have no such variable; their menu entry says so
instead of pretending.

## What is not here, and why

* **Usage meters.** Omarchy's bar panel reads subscription usage through
  provider APIs and local session logs. Those change shape when a provider
  changes, they need credentials, and a broken meter is worse than none. If they
  come back, the shape that fits this port is one `python3` script (standard
  library only, already in the image) printing a single line for a waybar
  `custom` module on an interval - no daemon and no new language.
* **Automatic account switching** near a limit: it depends on the meters above.

## Cost

The menu is a rofi window (about 16 MB while open) and a foot window for the
agent itself. The agents are Node programs: 150-400 MB resident while they run,
which is a lot on 4 GB. Run one at a time, and close it when you are done - the
bar's power panel shows memory and swap if you want to check.

# Plan: agents (Omarchy's "agents" feature, ported)

**P1 (launch/install/default) and P2 (accounts) are committed** as the first cut, in
`~/.config/bin/rofi-agents`. P3-P5 are the plan: nothing of those exists yet.

## What Omarchy actually ships

Read from `uis/omarchy` (Deboma). It is three subsystems, not one:

| Piece | What it does | Cost over there |
|---|---|---|
| **Bar panel** `shell/plugins/agents` (QML) | per-provider usage meters: rate limits with pace, today, last week, all-time model breakdown; providers Claude, Codex, Grok, Fireworks, Kimi; refresh every 900 s; optional cross-machine aggregation through a synced folder (`syncDir`, `<hostname>.json`) | resident QML + network calls with credentials |
| **Launcher** `omarchy-agent`, `omarchy-default-agent` | launches the default agent in a terminal, `--pick` opens the Agents list, `--prompt` passes one prompt; a 15-entry map of CLIs and installers; the default is a file (`~/.config/omarchy/defaults/agent`); bash aliases `a`, `c`, `cx`, `cy`, `ic` | none (process per launch) |
| **Accounts** `omarchy-agent-account-*` | many accounts per provider (Claude, Codex, Grok) with separate homes, `--reauth`, `mode auto <threshold>` to switch accounts near a rate limit, `use/list/rename/copy/transfer` | none (files + env) |
| **Usage scripts** `omarchy-agent-usage-{claude,codex,grok,fireworks,kimi}` | the panel's data source | network + credentials |

## What fits this port

The rules that decide everything below: no resident process, no QML, no credentials stored by us,
and every provider-specific piece must fail soft (a provider that changes shape shows "n/a", it does
not break the menu).

**The "thin layer" question, answered:** no new language. The image already has `python3` (stdlib
only) and `jq`; shell does the plumbing and rofi/foot do the UI. A Go or Rust helper would only pay
off for a *long-lived* process, which is exactly what this port does not want. If usage ever needs a
background refresh, the shape is a `python3` script run by waybar's `custom` module on an interval -
still no daemon, and no new toolchain to build and carry.

## Phases

### P1 - launch, install, default (half-written, untracked)

* `rofi-agents`: a rofi menu listing the agents it knows (Claude Code, Codex, OpenCode, Gemini CLI,
  GitHub Copilot); launch in foot from `~/Work`; install the missing ones with
  `npm install -g --prefix ~/.local <pkg>` (no root, no writes to `/usr/local`); set the default.
* Binding `Alt+Ctrl+Shift+A` (Omarchy's `Super+Shift+Ctrl+A`), an `org.lazyos.agents.desktop` entry,
  a root-menu entry, a page in the manual (`07-agents.md`).
* `PATH` must be forced to include `~/.local/bin` inside the script: sway's environment does not have
  it (only login/interactive shells get it from `profile.d`), which was already a real bug in the
  sketch - the menu offered to install an agent that was installed.
* Cost: rofi ~16 MB while open, then the agent itself. **Effort: 1-2 h** (mostly done).

### P2 - accounts (half-written, untracked)

* An account is a directory under `~/.local/share/lazyos/agents/<binary>/<name>`, selected with the
  variable each CLI documents: **`CLAUDE_CONFIG_DIR`** (Claude Code) and **`CODEX_HOME`** (Codex).
* Create (empty), use (writes the agent's default), remove, and `--env` for the shell:
  `eval "$(rofi-agents --env 'Claude Code' work)"`.
* The first launch in a new account runs the CLI's own sign-in, so two subscriptions never share a
  token. That sign-in is the only flow that touches the network, and it is the CLI's, not ours.
* OpenCode / Gemini / Copilot have no equivalent variable: the menu says so instead of pretending.
  Grok is left out until its CLI documents one.
* **Effort: 2-3 h** including the sign-in test on the card (it is the risky part: it needs a real
  account and a terminal).

### P3 - usage, local only (the honest version of the meters)

* Parse the agents' **own session logs** and total tokens per day and model: Claude Code writes
  `~/.claude/projects/**/*.jsonl`, Codex writes under `~/.codex/`. One `python3` script (stdlib),
  output one line for a waybar `custom` module and a `-mesg` line in the agents menu.
* No credentials, no network, no provider API to break. It re-states what the CLIs already recorded.
* Cost: a few hundred ms per run, every 15 minutes at most. **Effort: 3-4 h** (the log formats are
  undocumented; the script must tolerate missing fields and unknown shapes).

### P4 - provider usage (Omarchy's meters, for real)

* Only if P3 is not enough. Needs provider credentials and per-provider endpoints, and it must be
  fail-soft. Keep it out of the bar: run it from the menu on demand, print what answered, cache the
  last answer in `~/.cache/lazyos/agents-usage.json`.
* **Effort: unknown, and it is the part that ages badly.** I would not ship it without a reason.

### P5 - automatic account switching near a limit

* Depends on P4 being trustworthy. Explicitly **not planned** before that.

### Not planned

* The QML panel (a resident shell is the thing this port exists without).
* Cross-machine usage sync (a synced folder of JSON; cheap to add later if P3 exists).
* The 15-agent map: five verified npm CLIs is the honest list; the rest need installers that are
  Arch-specific or curl-pipe-to-shell.
* Aliases like `c`/`cx`/`cy` in `~/.bashrc`: they assume one specific CLI per letter; the menu is
  the same thing without the guesswork.

## Costs, side by side

| | Omarchy | This plan |
|---|---|---|
| resident | QML panel + refresh timer + network | nothing |
| memory while in use | panel (~30-60 MB) + agent | rofi 16 MB, then the agent (150-400 MB: they are Node programs) |
| disk | agents + installers | agents only, in `~/.local` |
| credentials | panel API calls | the CLIs' own files, untouched |
| failure modes | provider API changes, panel crash | npm package names, undocumented log formats (P3) |

## Checks each phase must pass

* P1: the menu lists an installed CLI as launchable (the `PATH` bug above is the test), installs one
  without root, and the default is remembered across sessions.
* P2: two accounts hold different tokens (launch each, `claude /status` or equivalent), `--env` in a
  shell does the same as the menu, removing an account asks first and leaves no default pointing at
  it.
* P3: token totals from the logs match what the CLI reports for the same day (spot check), and the
  script exits 0 with an empty line when no logs exist.
* Every phase gets a `check-system.sh` line and a paragraph in `uis/sway/README.md`; P1/P2 also get
  the manual page.

## Decisions needed

1. **Scope now**: P1 only, P1+P2, or P1-P3?
2. **Where usage shows** (if P3): a `-mesg` line in the agents menu, a waybar module, or both?
3. **Agent list**: keep the five npm CLIs, or add Grok / Crush / Cursor CLI with their
   non-npm installers?
4. **Language**: stay with `sh` + `jq` + `python3` (recommended), or allow one small compiled helper?

## Status

Committed: **Learn** (seven manual pages + `rofi-learn`), the **key view** (`rofi-keys`),
**hibernate** (`sway-hibernate`), **dictation** (`dictate`, `dictate-model`) and the **agents
P1/P2** (`rofi-agents`, its manual page, launcher entry and binding). Each was verified in the
headless compositor (screenshots of the key list, the manual list, the agents menu) and the session
configuration validates with `sway -C`.

The kernel rebuild finished (`HZ=1000`, MGLRU enabled). The rootfs/card pipeline has **not** run:
the current image is older than all of this, so a `scripts/build-rootfs.sh` + `install-kernel.sh` +
`install-swayfx.sh` + `make-kpart.sh` + `write-sd.sh` cycle is still needed before any of it boots.

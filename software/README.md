# Software

Terminal software considered for this port, by category. It is a *shopping
list*, not the image: nothing here is wired into `packages/*.list` or
`uis/sway/packages.list`, and the target is a later release. The seed is the
TUI list on the maintainer's own desktop notes
(`docs/screenshots/hayato-r1-sway.png`), extended with the tools already
mentioned in this repository.

Every row carries four facts:

| Column | Meaning |
|---|---|
| **Tool** | the upstream name, as it is typed |
| **What it is** | one line, no marketing |
| **Package** | the Debian binary package in forky arm64, or `—` when Debian does not ship it |
| **Size** | that package's `Installed-Size` from this host's forky arm64 archive |

`Size` is the package alone, not its dependency closure (a Python or Go tool
costs more than the number shown), and it drifts between releases — re-read it
when the list is used. `image` in the Package column means the binary already
ships in `packages/tools.list` or `uis/sway/packages.list` (`lazygit`, `tmux`,
`fzf`, `glow`, `bluetoothctl`).

After a `—`, the parenthesised tools are the closest **packaged alternatives**:
what a release can actually install today. That column is the important one,
because most of the interesting software in these lists has no Debian package.

## What a `—` entry would take

Two ways in, and one that is closed:

1. **Source build** — the shape `compositors/swayfx/` already has: pinned
   upstream version, `SHA256SUMS`, patches, a build script and a `.deb`. Rust
   and Go are the usual candidates, and the crates are sometimes already
   packaged (`librust-yazi-dev` exists even though `yazi` does not).
2. **Use the alternative** in the same row. This is the default.
3. A **static binary** downloaded from a release page is not an option: no
   provenance, no update path, no architecture guarantee, which is what
   `scripts/lib-dist.sh` and the release rules exist to prevent.

## Categories

| File | Covers |
|---|---|
| `development.md` | editors, git, databases, documents, hex, HTTP/API |
| `comms.md` | mail |
| `office.md` | documents, spreadsheets, slides, data |
| `media.md` | video, music, podcasts, Spotify |
| `ops.md` | disk, kernel, network, firewall |
| `utilities.md` | files, multiplexer, finder, dashboard, Bluetooth, transfer |
| `games.md` | terminal games |

# Comms

Mail. Target: a later release; see [`README.md`](README.md) for the columns.

| Tool | What it is | Package | Size |
|---|---|---|---|
| **himalaya** | mail CLI/TUI | — (`aerc` 19 MB, `mutt` 7.5 MB, `neomutt` 9.4 MB, `meli` 17.8 MB) | — |
| **aerc** *(added)* | mail TUI; the closest packaged path | `aerc` | 19 MB |
| **neomutt** *(added)* | mail TUI | `neomutt` | 9.4 MB |
| **mutt** *(added)* | mail TUI | `mutt` | 7.5 MB |
| **meli** *(added)* | mail TUI | `meli` | 17.8 MB |

Any of these needs an account and, for a local Maildir, an IMAP synchroniser:
`isync` (379 KB) is what provides `mbsync`. Nothing here is resident between
runs.

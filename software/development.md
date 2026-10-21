# Development

Editors, git, databases, documents, hex viewers and HTTP/API tooling. Target: a
later release; see [`README.md`](README.md) for the columns. `image` means the
binary already ships.

## Editors

| Tool | What it is | Package | Size |
|---|---|---|---|
| **Neovim** | Vim fork; "LazyVim" (the notes' IDE) is a configuration in `~/.config/nvim`, not a package | `neovim` | 9.7 MB |
| **micro** | nano-like editor with mouse support | `micro` | 13.8 MB |
| **Helix** | modal editor with multiple selections | — (`neovim`; `kakoune` 4.0 MB) | — |
| **ttt-editor** | TUI IDE, unmaintained upstream (the notes already say "needs manual update") | — (`neovim`) | — |

## Git

| Tool | What it is | Package | Size |
|---|---|---|---|
| **lazygit** | terminal UI for git | `lazygit` **image** | 22.6 MB |
| **serie** | git graph renderer | — (`tig` 1.3 MB) | — |

## Databases

| Tool | What it is | Package | Size |
|---|---|---|---|
| **lazysql** | DB browser for MySQL, Postgres and SQLite | — (`usql` 17 MB; `harlequin` 527 KB + Python) | — |
| **qrypad** | SQL workbench for MySQL, Postgres and SQLite | — (`harlequin`; `usql`) | — |
| **rainfrog** | Postgres TUI | — (`pgcli` 358 KB; `usql`) | — |
| **sqlit** | SQL TUI | — (`litecli` 194 KB; `usql`) | — |

## Documents

| Tool | What it is | Package | Size |
|---|---|---|---|
| **frogmouth** | markdown browser | — (`glow` 17 MB **image**; `w3m` 3 MB) | — |

## Binary

| Tool | What it is | Package | Size |
|---|---|---|---|
| **hexyl** | coloured hex viewer | — (`hexedit` 91 KB; `xxd` 170 KB; `fq` 10.7 MB for structured formats) | — |

## HTTP and APIs

| Tool | What it is | Package | Size |
|---|---|---|---|
| **HTTPie** | HTTP client; the binary is `http` | `httpie` | 404 KB + Python |
| **mitmproxy** | HTTP debugging proxy | `mitmproxy` 8.1.1 (older than upstream) | 3.65 MB + Python |
| **atac** | HTTP API client TUI | — (Debian's `atac` is a DNA aligner, not this; use `httpie`, `xh` 8.6 MB or `curlie` 2.6 MB) | — |
| **openapi-tui** | browse and run OpenAPI specs | — (nothing packaged; `httpie` plus a pager) | — |
| **APIer** | HTTP client, a Firefox add-on rather than a TUI | Firefox (`addons.mozilla.org`) | — |

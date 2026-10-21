# Ops

Disk, kernel, network and firewall. Target: a later release; see
[`README.md`](README.md) for the columns.

| Tool | What it is | Package | Size |
|---|---|---|---|
| **ncdu** | disk usage browser | `ncdu` | 156 KB |
| **kmon** | kernel module and dmesg browser | `kmon` | 1.2 MB |
| **gping** | ping with a graph | `gping` | 3.4 MB |
| **rusnet** | network analysis TUI | — (`bandwhich` 3.4 MB, `nethogs` 149 KB, `iftop` 158 KB) | — |
| **netscanner** | network scanner and diagnostics | — (`nmap` 4.8 MB, `wavemon` 159 KB) | — |
| **caligula** | disk image writer TUI, `dd` with a progress display | — (`pv` 412 KB + `progress` 123 KB) | — |
| **tufw** | ufw TUI | — (`ufw` 862 KB; the firewall itself, no TUI) | — |

The network rows need `sudo -n` for raw sockets, which the image's passwordless
sudo covers; `wavemon` is the one that matters on this Wi-Fi card.

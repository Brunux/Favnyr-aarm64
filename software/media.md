# Media

Video, music, podcasts and Spotify. Target: a later release; see
[`README.md`](README.md) for the columns.

| Tool | What it is | Package | Size |
|---|---|---|---|
| **mpv** | video and audio player; the image currently has no video player | `mpv` | 3.5 MB + libraries, most already present through `ffmpeg` |
| **cmus** *(added)* | local music player, one process, PipeWire output | `cmus` | 1.6 MB |
| **myx** | Spotify client | — (no Spotify client in forky: `ncspot`, `spotify-player` and `spotifyd` are absent; Flatpak, static binary or the web player in Firefox) | — |
| **ncspot** | Spotify client | — (same) | — |
| **castero** | podcast client | — (`yt-dlp` + `mpv` play a feed; no podcast TUI is packaged) | — |

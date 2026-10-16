# uis/sway — Sway desktop

A UI is a package list plus a file tree:

```
uis/sway/
  packages.list          packages to install (Debian names)
  files/                 tree overlaid onto the rootfs
    etc/...              copied to /            (owned by root)
    home/...             merged into /home/$USERNAME (owned by the user)
```

`scripts/build-rootfs.sh` stages both into the chroot and
`scripts/rootfs-setup.sh` installs them, rewriting `/home/brunux` to the
configured `USERNAME` inside the text files.

This desktop was ported from the maintainer's Nura (postmarketOS) session, so it
keeps the solarized theme, the `Alt` modifier + Search-key launcher, the bemenu
launcher, waybar and the Alacritty terminal.

## Layout

| Path | What it is |
|---|---|
| `etc/sway/config` | System half of the session (replaces Debian's default file). Includes `config.d/*` at the end. |
| `etc/sway/config.d/*` | Drop-ins: logo key, bar, launcher, brightness, waybar, lock, volume, sleep, gestures, screenshots, theme, touchpad. |
| `home/.config/sway/config` | Personal overlay: re-includes `/etc/sway/config`, then adds the keyboard repeat rate and the Search-key launcher binding. |
| `home/.config/waybar/{config.jsonc,style.css}` | Solarized waybar theme. |
| `home/.config/workstyle/config.toml` | Icon map for workstyle (see below). |
| `home/Pictures/Wallpapers/mountains.jpg` | Wallpaper referenced by `etc/sway/config`. |

## Deviations from the Nura original

* **Fonts.** The theme asks for `AdwaitaMono Nerd Font` / `JetBrainsMono Nerd Font`, which Debian
  does not ship (its `fonts-adwaita` has no Mono variant), so `style.css` and the Pango spans in
  `config.jsonc` use `JetBrains Mono`. The icons come from fontconfig fallback and need two
  packages people usually miss:

  | Glyphs | Package |
  |---|---|
  | Font Awesome, `U+F000`-`U+F2FF` | `fonts-font-awesome-legacy` |
  | Material Design Icons, plane 15 | `fonts-materialdesignicons-legacy` |
  | Octicons, Codicons, Devicons, Powerline | `fonts-nerd-symbols` |

  `fonts-nerd-symbols` alone is not enough: Debian stripped Font Awesome and Material Design
  Icons from it for DFSG reasons, so every waybar icon would render as a tofu box with the hex
  codepoint inside. Nothing else in the configuration changes.
* **workstyle is not installed.** It is not packaged in Debian and the Nura
  build was a cargo binary linked against musl (it cannot run against glibc).
  It renames workspaces to icons; the waybar theme shows workspace numbers, so
  nothing is missing visually. `home/.config/workstyle/config.toml` is kept so
  that enabling it later needs no extra configuration:

  ```sh
  cargo install workstyle
  echo 'exec_always --no-startup-id workstyle' >/etc/sway/config.d/03-workstyle.conf
  ```

* `etc/sway/config.d/01-status-bar.conf` disables swaybar (waybar is used
  instead); the block is kept commented out for reference.

# Welcome to Nureb

Nureb is Debian (**forky**, arm64) running on a Google **Hayato**: the ASUS
Chromebook Detachable CM3 with a MediaTek **MT8192** ("Asurada") SoC, 4 GB of RAM
and a 58 GB eMMC. It is not a Debian remix: the hardware support comes from
Nura (the postmarketOS-based system this port started from), and Debian supplies
everything above the kernel.

## The first five minutes

* Log in with the user account the image was built with (`brunux` by default,
  password `debian`). Change the password now: `passwd`.
* `sudo` does not ask for a password on this machine. Anything that needs root
  goes through it, including the scripts that rewrite the boot partition.
* Open a terminal: **Alt+Return** (or the bar's menu → Terminal).
* Open the launcher: the **Search key** (the magnifier, left of Tab) or
  **Alt+D**. Type part of a name, Enter.
* See every key binding: **Alt+Ctrl+K**.
* This manual: the **Learn** entry in the bar menu (**Alt+Space**), or
  `rofi-learn` in a terminal.

## What is on the screen

| Piece | What it is |
|---|---|
| the bar (top) | waybar: menu, workspaces, clock, updates, tray, keyboard layout, Bluetooth, network, volume, brightness, battery. Left/right click and scroll do different things on each |
| compositor | **swayfx**, a fork of sway that draws rounded corners, shadows and a blurred backdrop under the translucent terminal, launcher and bar |
| the terminal | **foot**: solarized palette, no GPU renderer, about half the memory of an Alacritty window |
| the launcher | **rofi**: applications, windows, commands, and the bar's own menus |
| notifications | **mako**, bottom of the top-right corner, under the bar |
| the session | greetd starts sway at boot; swayidle runs the screensaver at 150 s and the lock at 300 s |

## Where things live

| Path | Contents |
|---|---|
| `/etc/sway/` | the system half of the session: `config` plus `config.d/*.conf` drop-ins |
| `~/.config/sway/` | your half: it includes the system one and adds your own settings |
| `~/.config/{bar-*,rofi-keys,rofi-learn,dictate,sway-*}` | the scripts the bar and the keys call |
| `~/.config/{waybar,foot,rofi,mako}/` | the look of each piece |
| `~/.config/bin/` | every script this desktop ships; they are all plain shell |
| `/usr/share/doc/nureb/` | these pages |
| `~/Dev/Nureb/` | a **snapshot** of the repository this image was built from, with a `REVISION` file naming the commit. The directory is named after the repository: if it is ever renamed, the copy takes the new name |

That last one matters: the image carries the tested tree, and the scripts that
rewrite the kernel partition (`scripts/install-kpart.sh`) or back the system up
(`backup-system.sh`) are meant to be run *from it*, on the machine. If you see a
`REVISION` that does not match what you expect, the card or the install is older
than you think.

## Is everything healthy?

```sh
check-system.sh ~/Dev/Nureb
```

It prints one line per check (`OK`, `WARN`, `FAIL`, `SKIP`), and every check maps
to a bug this port has already hit - a missing firmware, a key that a remapper
swallowed, the mixer's boot sequence, the backlight permissions, the clock. Run it
after a boot that behaved strangely, and keep the output when you ask for help.

## Power

* **Lock key** (top row, right-most): locks the screen, and systemd-logind
  suspends the machine a moment later. That path needs no polkit and no session
  state, which is why it always works. **Alt+Ctrl+L** locks without suspending
  (Omarchy's `Super+Ctrl+L`).
* **Alt+Escape** opens the same system panel as the battery module (Omarchy's
  `Super+Escape` system menu): lock, suspend, log out, reboot and power off.
* **Alt+Shift+Escape**: lock, then suspend. **Alt+Ctrl+Escape**: suspend without
  locking.
* The bar's battery module opens the power panel (charge, draw, time left,
  charger, uptime, memory and swap), and its **root menu** (**Alt+Space**) has log
  out, reboot and power off.
* Hibernate is offered only when a disk swap and `resume=` are both present; see
  **The system** page.

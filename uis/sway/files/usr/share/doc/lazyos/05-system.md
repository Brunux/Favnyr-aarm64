# The system

## The disk, and why the third partition is not the whole story

The disk has three partitions, the ChromeOS layout this machine's firmware
expects:

| Partition | Size | What |
|---|---|---|
| `p1` `pmOS_kernel` | 128 MiB | the **signed FIT** ("kpart") the firmware boots: a kernel, the initramfs and the device trees in one file |
| `p2` `pmOS_boot` | 512 MiB | ext2, mounted on `/boot`: the same kernel and initramfs, unpacked |
| `p3` `pmOS_root` | the rest | the Debian root filesystem |

The firmware does not read `/boot`. It loads `p1`, which is why a kernel update
is only half done after `apt`: the new kernel lands in `/boot` and `/lib/modules`,
but `p1` still carries the old one. Finish it from the repository copy:

```sh
sudo ~/Dev/LazyOS/scripts/install-kpart.sh
```

That script finds the partition this machine booted from (it compares
`kern_guid` in `/proc/cmdline`), builds a fresh kpart, verifies it is dev-signed,
backs up what it replaces to `~/.cache/debian-aarch64/work/`, and writes it.
`CONFIRM=yes` is required.

## The kernel

Linux **7.2.7-mt81**, built here with clang and lld from
`kernels/mediatek-mt81/`. It is not a Debian kernel: the MediaTek display, MMC,
panfrost (the GPU) and ext4 are built in, and the configuration carries this
machine's tuning:

* **MGLRU** enabled by default - page reclaim by generation instead of a full
  scan, which is what 4 GB under pressure wants;
* **schedutil** as the CPU governor (`cat
  /sys/devices/system/cpu/cpufreq/policy*/scaling_governor`), instead of
  `ondemand`, which pinned both clusters at maximum;
* **1000 Hz** timers, full preemption;
* transparent huge pages set to `madvise` on the command line.

Memory: `zram` provides a 1.9 GB compressed swap device (zram-generator,
`/etc/systemd/zram-generator.conf`, lz4), and `systemd-oomd` kills a memory hog
at 50 % pressure instead of letting the machine freeze. `swapon --show` and
`zramctl` show what is in use.

## Health and backups

```sh
check-system.sh ~/Dev/LazyOS    # OK / WARN / FAIL, one line per check
```

Every check corresponds to a bug this port has already hit, so a `FAIL` is a
regression, not a theory. The image ships the script as
`/usr/local/sbin/check-system.sh` too.

Backups are made of the *parts* a disk cannot be regenerated from:

```sh
sudo ~/Dev/LazyOS/scripts/backup-system.sh /media/you/BACKUP
sudo ~/Dev/LazyOS/scripts/restore-system.sh /media/you/BACKUP /dev/mmcblk0
```

The kernel partition, the partition table, `/boot` and the root filesystem go
into tars (ACLs and xattrs preserved) plus a `README.md` with the restore steps
and a `SHA256SUMS`. `restore-system.sh` wants a disk of the same size and
`CONFIRM=yes`, and refuses to run from the disk it would overwrite.

## Suspend and hibernate

Suspend-to-RAM works and resumes cleanly. Two paths lead to it, and they are
different on purpose:

* **The Lock key** is `KEY_SLEEP`, and that is exactly the key systemd-logind
  turns into a suspend on its own. It needs no polkit, no session and no helper,
  which is why it always works. The same key runs `~/.config/bin/sway-lock`, so
  the screen is locked a fraction of a second before the machine sleeps.
* `sway-suspend` (Alt+Shift+Escape locks first, Alt+Ctrl+Escape suspends
  directly) goes through `loginctl suspend`, a D-Bus method guarded by polkit. It
  has a fallback chain and logs to `~/.cache/sway-suspend.log`, so if the session
  is in a state logind does not consider active, it still works.

`sway-lock` is the lock every path goes through, including the 300 s idle timer:
it stops the idle screen first, waits (up to 4 s before a suspend) for
`swaylock`'s own readiness signal, and puts the session's wallpaper behind the
lock instead of a flat colour.

**Hibernate** is offered when it can actually resume:

* a swap on disk (not zram) big enough for the image - the kernel targets 2/5 of
  RAM, so 2 GB is the practical minimum here;
* `resume=` on the kernel command line, so the next boot restores the image
  instead of starting fresh. Because `resume=` is resolved through udev, a
  **label** is enough: a swap partition labelled `lazyos_swap` plus
  `resume=LABEL=lazyos_swap` in `devices/google-asurada/kernel-cmdline.conf`.

The **Auxiliary swap** row in Places (and in the root menu) writes that partition
and the `/etc/fstab` line that activates it on every boot, with priority 10 so
zram keeps being used first. It does not add `resume=`: that is a kernel command
line change and needs the kernel partition rebuilt.

`sway-hibernate` (in the root menu, and as a launcher entry) checks both and says
which one is missing. It never hibernates into a swap the machine cannot resume
from.

## The microSD and the firmware

The internal eMMC and the microSD are written the same way
(`scripts/write-sd.sh`, `scripts/install-emmc.sh`), so a card is a full rescue
system: the same three partitions, the same signed kpart, the same session. The
stock firmware's developer-mode menu picks **internal disk** (the eMMC) or
**external disk** (the card), so a broken install on the eMMC is recoverable by
booting the card and running the installer again.

## Install LazyOS

There is no separate installer program: `scripts/install-emmc.sh` copies **the
system that is running** onto the internal disk, which is why the card is what
you install from. The menu (*Install LazyOS* in the root menu and in the
launcher) is its front end, so what lands on the eMMC is the system that was
tested, not a second implementation of it.

Open it from the microSD. When the machine is running from the disk it would
erase, it says so and offers nothing to press: that is the one mistake nothing
can undo.

The menu is a report before it is a button. It checks the target and the room
the copy needs, the charger and the battery (below half it says so, below 20% on
battery it refuses), the running kernel's own files - the kernel partition is
rebuilt from them *after* the copy, and without them the install would fail with
the disk already formatted - the installer's tools, and that `sudo` does not ask
for a password, because the copy runs detached and nothing can answer a prompt.

**Cancel is the first row**, and it is selected, so a stray Enter writes
nothing. Choosing *Install LazyOS* then asks for the disk's name typed in full
(`mmcblk0` for the eMMC): a list cannot ask that, because Enter on a row is
exactly the gesture that must not erase a disk.

After that it runs the documented install unchanged - `install-emmc.sh` with
`CONFIRM=yes` - and does not partition or format anything itself. Around that
call it does the two things the installer refuses to start with: it stops
**udisks2** and masks it for the length of the run, so nothing can auto-mount
the new partitions mid-write, and it unmounts any partition of the target. Both
are put back afterwards on every exit path, and a failure there stops before a
byte is written. `systemd-inhibit` holds a sleep lock for the length of the copy.

The copy takes minutes, so it is **not stoppable**: a killed `rsync` or `mkfs`
leaves a disk that is neither the old system nor the new one. A notification
card that does not expire and the bar's spinner follow the installer's own
steps, and the card says why it cannot be stopped when it is clicked.

The end is read from the disk and not from the exit code: the three partitions
must carry `pmOS_kernel`, `pmOS_boot` and `pmOS_root`, and the kernel partition
must verify with `futility show`. Only then does the card say to reboot and pick
**internal disk** in the firmware menu.

Every run writes `~/.local/state/lazyos/install-<date>.log`. **Show the install
log** opens it in a window with `less`, following it live while the copy runs
and at the end afterwards; the menu also keeps the last run's outcome under
*last time*, and a `running` line with no process behind it means the run was
killed. When something fails, the card carries the installer's own last `ERROR:`
line and the path of the log.

## Updating

```sh
sudo apt update && sudo apt full-upgrade
sudo ~/Dev/LazyOS/scripts/install-kpart.sh   # if the kernel changed
check-system.sh ~/Dev/LazyOS
```

The image itself is a snapshot: it is rebuilt from the repository, not upgraded
in place forever. Your home and your `apt` changes survive; the shipped
`/etc` files and the session scripts are replaced by the next image, which is why
personal settings belong in `~/.config/`.

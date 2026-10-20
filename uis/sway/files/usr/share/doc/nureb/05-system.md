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
sudo ~/Dev/debian-aarch64/scripts/install-kpart.sh
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
check-system.sh ~/Dev/debian-aarch64    # OK / WARN / FAIL, one line per check
```

Every check corresponds to a bug this port has already hit, so a `FAIL` is a
regression, not a theory. The image ships the script as
`/usr/local/sbin/check-system.sh` too.

Backups are made of the *parts* a disk cannot be regenerated from:

```sh
sudo ~/Dev/debian-aarch64/scripts/backup-system.sh /media/you/BACKUP
sudo ~/Dev/debian-aarch64/scripts/restore-system.sh /media/you/BACKUP /dev/mmcblk0
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
  which is why it always works. The same key is bound to `swaylock`, so the
  screen is locked a fraction of a second before the machine sleeps.
* `sway-suspend` (Alt+Shift+Escape locks first, Alt+Ctrl+Escape suspends
  directly) goes through `loginctl suspend`, a D-Bus method guarded by polkit. It
  has a fallback chain and logs to `~/.cache/sway-suspend.log`, so if the session
  is in a state logind does not consider active, it still works.

**Hibernate** is offered when it can actually resume:

* a swap on disk (not zram) big enough for the image - the kernel targets 2/5 of
  RAM, so 2 GB is the practical minimum here;
* `resume=` on the kernel command line, so the next boot restores the image
  instead of starting fresh. Because `resume=` is resolved through udev, a
  **label** is enough: a swap partition labelled `nureb_swap` plus
  `resume=LABEL=nureb_swap` in `devices/google-asurada/kernel-cmdline.conf`.

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

## Updating

```sh
sudo apt update && sudo apt full-upgrade
sudo ~/Dev/debian-aarch64/scripts/install-kpart.sh   # if the kernel changed
check-system.sh ~/Dev/debian-aarch64
```

The image itself is a snapshot: it is rebuilt from the repository, not upgraded
in place forever. Your home and your `apt` changes survive; the shipped
`/etc` files and the session scripts are replaced by the next image, which is why
personal settings belong in `~/.config/`.

# The installer: design

> The deep dive for the installer that `docs/plan.md` announces. It covers the three layers, how a
> device and a target are found, the plan and its phases, the safety invariants, the state protocol,
> the TUI, autoinstall, packaging, tests and the witnesses. The roadmap, the language policy and the
> migration tiers stay in [`plan.md`](plan.md); this file is the contract for v0.4.0.

**Scope of v0.4.0: clone mode.** The live card is the installer. It clones the running system onto
internal storage, cleans the machine identity, writes the fstab, kernel command line and kernel
partition, verifies them, and records the installation. **Fresh mode** (new user, home, hostname and
password) is v0.4.1 and is designed here only where the clone path has to leave room for it.

The word **install** below means "replace the target disk with a copy of this running system". The
word **engine** means `lazyos-install`; **TUI** means `lazyos-installer`; **host** means the `lazyos`
binary's installer subcommands. The **marker** is `/etc/lazyos/installation`; the **kpart** is the
signed depthcharge FIT that the firmware loads from `p1`.

---

## 1. The shape

```
live card (the tested system)
  │
  ├── lazyos-core       profile, `compatible`, running-state detection, target discovery
  ├── lazyos-disk       ChromeOS GPT, mkfs, fstab, mount, kpart, verify
  ├── lazyos-identity   clone identity cleanup; fresh user/home in v0.4.1
  ├── lazyos-install    probe, plan, apply, state, doctor, auto, cancel   (root, no UI)
  └── lazyos-installer  Ratatui TUI                                       (user, no disk I/O)
        │
        └── lazyos     host: `installer`, `installer-autostart`, `disk ...`
```

- **The card is the source and the operator console.** There is no installer ISO, no squashfs, no
  second root filesystem and no Calamares. The card that was tested is the card that installs; the
  installed eMMC is the card's system with a different identity.
- **One binary per responsibility.** The TUI never runs as root and never opens the target. The
  engine is never interactive and never draws. The host only decides where the TUI runs and starts
  the engine; it does not touch the disk.
- **The engine is detached.** The TUI starts it under `setsid`, so closing the window, losing the
  foot process or logging out of the session does not stop a write that has already begun. The TUI
  is a view over `/run/lazyos-install`, not the owner of the install.
- **The dangerous work is Rust.** The GPT, mkfs, kpart and identity rules are in crates the engine
  calls; no shell is sourced to decide anything. The C `mkfs`, `cgpt`, `parted`, `rsync` and
  `depthchargectl` remain the tools, invoked with arguments the crates own.
- **The device layer is data.** `devices/<codename>/` plus `kernels/<flavor>/` describe a port; the
  engine has no Hayato branch, no `mmcblk0` constant and no board name. The Hayato profile is the
  first instance, not the definition.

## 2. The three layers

### 2.1 The device profile

The build stages the committed profile at `/usr/share/lazyos/`:

| File | Used at install time | Contents |
|---|---|---|
| `deviceinfo` | yes | the pmaports-style fields: kernel-partition start/size, board, compression, DTB glob, architecture, name, codename; plus `deviceinfo_install_target` when the internal disk must be named |
| `compatible` | yes | the exact strings from `/proc/device-tree/compatible`, one per line, most specific first |
| `kernel-cmdline.conf` | yes | the base command line, one fragment per line |
| `modules-initfs` | no (build) | the modules forced into the initramfs |
| `home-template/` | v0.4.1 | the home tree Fresh mode seeds |

The build-time `packages.list` and `APKBUILD.reference` stay in the repository; they are not runtime
inputs. The profile is installed root-owned and read-only.

`deviceinfo` is **parsed, never sourced**. The Rust parser accepts `key="value"` lines, comments and
blank lines, rejects anything else, and produces a typed struct. The shell scripts currently source
the file; that is one of the reasons the device-side `lib.sh` is on the v0.4.0 migration list. A
profile cannot execute code just by being installed.

The Hayato profile gains one line in v0.4.0:

```
deviceinfo_install_target="/dev/mmcblk0"
```

It is a hint, not a hardcoded engine path: it names the disk that is "internal" on a device whose
kernel reports both the eMMC and the microSD as non-removable (`RM=0`, verified on this machine for
`mmcblk0` and `mmcblk1`). A device whose internal storage is genuinely removable omits the hint and
the target screen chooses; a card install is an explicit profile choice, never the default.

### 2.2 The engine

`lazyos-install` is a library (`lazyos-install`) with a small binary. It is root-only for `plan`,
`apply`, `cancel`, `kpart` and `auto`; `probe`, `status`, `doctor` and `version` run as the desktop
user and only read. It writes only to `/run/lazyos-install` and to the target, after the checks.

It does not depend on the session, the compositor or the terminal. `--json` output is stable and
machine-readable; human output goes to the log. It never reads a password from argv; v0.4.0 has no
password at all.

### 2.3 The TUI

`lazyos-installer` is a Ratatui application on crossterm, fullscreen in foot, run as the desktop
user. It:

- probes through `lazyos-install probe --json`;
- asks the engine (through `sudo -n`) for the plan and the apply;
- polls `/run/lazyos-install/state.json` and renders it;
- never mounts, formats or writes the target;
- holds no secret, because clone mode has none.

The TUI's own trade-offs (foot, compositor bindings, idle chain) are in §8, as `plan.md` promises.

### 2.4 The host

`lazyos` v0.4.0 is small:

```
lazyos installer              # open or attach the TUI; on foot, fullscreen
lazyos installer-autostart    # the exec-once entry: open once if this is a live card
lazyos disk layout|mount|fstab|kpart   # hidden host/engine helpers, used by the build scripts
```

The host decides only:

- **live or installed** (`lazyos-core` detection);
- **already open** (a `flock` on the user's runtime directory, so two launcher clicks do not open
  two TUIs);
- **already running** (there is a state file with a running engine, so open the TUI attached);
- **where the TUI runs** (a tty: run `lazyos-installer` in it; no tty: `foot --app-id=org.lazyos.installer
  --fullscreen --title "LazyOS Installer" -e lazyos-installer`).

The launcher `.desktop` is `Terminal=false`, `Exec=lazyos installer`, `Icon=lazyos`: the host, not
the launcher, owns the fullscreen foot. The root-menu row is one line in the `system` group of
`bar-menu`, with the existing `UI_I_DISK` glyph; no new icon is added for it.

## 3. Detection

### 3.1 The device

`lazyos-core` reads `/proc/device-tree/compatible` (NUL-separated) and the installed `compatible`
file. A line matches only if the running device tree carries that exact string. The first match
wins; the most specific string is first in the file. No match is a **hard stop**: the engine does not
plan, does not offer a target and prints both lists. A profile that does not match the device it is
running on is a build error, not a warning.

The exact Hayato strings are taken from the device and committed in `devices/google-asurada/compatible`;
they are not guessed here. `check-system.sh` compares the file with the running DT, so an image
carried to another board fails visibly.

### 3.2 Live or installed

The marker is a regular JSON file on the target root:

```json
{
  "schema": 1,
  "version": "0.4.0",
  "mode": "clone",
  "target": "/dev/mmcblk0",
  "target_model": "...",
  "target_serial": "...",
  "root_uuid": "....",
  "boot_uuid": "....",
  "installed_at": "2026-10-21T...Z"
}
```

It is written **last**, after the copy, identity, fstab, kpart and file checks. Its `root_uuid` is the
UUID the engine formatted into `p3`, not the card's. Detection reads the UUID of the mounted root
from the superblock and compares:

| Marker | Root UUID | State | Autostart |
|---|---|---|---|
| absent | — | **live** (also the v0.3.0 legacy image) | opens |
| present, parses | equals `/` | **installed** | does not open |
| present, parses | differs from `/` | **live** (a card written by an install on another root, for example a restored copy whose UUIDs were changed) | opens; `doctor` says which root the marker describes |
| present, malformed | — | **live**, with a warning | opens; `check-system.sh` FAILs the parse |

The comparison is why there is no kernel command-line flag to forge, no file on the card that a
restore can make ambiguous, and no second state file to keep in sync. A **restored** installed
system carries the marker through the backup; it says "installed" while the root UUID matches, which
is true, and `doctor` notes when the marker's target is not the disk the system is running from.

The v0.3.0 images have no marker. They are live, as intended: the card is the installer, and the
engine writes the marker on the target it installs. There are no in-place upgrades, so a v0.4.0
engine never has to detect an older installed system by anything else.

### 3.3 The target

Target discovery is its own function in `lazyos-core`; it **must not reuse `ui_disks`**.

`ui_disks` in `lib-rofi.sh` is the candidate list for **Format a disk** and **Auxiliary swap**. It
deliberately excludes any disk carrying a ChromeOS kernel partition, because those menus must never
erase this port's own eMMC or the card it is running from. The installer's target is the opposite
case: the eMMC that currently holds Nura or a previous LazyOS is exactly what it is supposed to
replace. Calling `ui_disks` here would hide the only valid target.

The installer's rules are:

1. **Whole disks only.** `lsblk -dn` entries of type `disk`; a partition path in the profile is a
   profile error. The disk may carry mounted partitions; the probe reports them and `prepare`
   unmounts them (rule 6).
2. **Never the running disk.** Resolve `/` and `/boot` to their parent whole disks and exclude both.
   The card the engine is running from is the rescue system; it is never a target. `write-sd.sh`
   remains the way to write a card, from the host.
3. **Never the SPI or the hardware boot areas.** Exclude `mtd*` (the 8 MB `mtdblock0` on this
   machine is the firmware this port must never write), `mmcblk*boot*` (the eMMC's 4 MB
   `mmcblk0boot0`/`mmcblk0boot1`, listed as disks by `lsblk -d`), `*rpmb`, and by name `zram*`,
   `loop*`, `dm-*`, `md*`, `ram*`, `sr*`, `nbd*`, `rbd*` and `fd*`.
4. **The profile hint first, then the only candidate.** With `deviceinfo_install_target`, that disk
   is the default after the exclusions. Without it, one remaining whole disk is the target; two or
   more require the TUI to choose, and `auto` fails rather than guess.
5. **The kernel partition is allowed.** A previous ChromeOS install carries `pmOS_kernel` in `p1`;
   that is not an exclusion, it is the expected content of the disk whose table is about to be
   replaced. An existing Nura or LazyOS layout is listed by its labels so the confirmation can name
   it.
6. **A mounted target is reported, not silently unmounted.** The probe lists every mounted
   partition and active swap on the target. The confirmation says they will be unmounted; the
   engine stops `udisks2`, unmounts the target's partitions and turns off its swap in `prepare`. A
   partition busy for another reason (an open file, a dm stacking) fails with the holder named.

The Hayato facts that make the hint necessary are in the report: the running root here is
`/dev/mmcblk0p3` (the eMMC), the microSD is `/dev/mmcblk1`, both report `RM=0`, and `lsblk -d` also
lists `mtdblock0`, `mmcblk0boot0`, `mmcblk0boot1` and `zram0`. "Removable" cannot choose the target;
the profile can.

The target record stored in the plan is the resolved whole-disk path plus size, model, serial when
the device reports one, and the `RM` flag. `apply` re-probes and refuses if any of them changed, if
the path is now the running disk, or if a new mount appeared. The eMMC serial and the card serial
differ on this machine, which is the second discriminator behind the path.

### 3.4 Tools

The engine invokes the proven tools rather than reimplementing filesystems:

| Tool | Package | Used for |
|---|---|---|
| `parted` | `parted` | protective MBR and the base GPT |
| `cgpt` | `cgpt` | ChromeOS entries, types, labels, kernel flags |
| `partx`, `partprobe`, `blockdev`, `findmnt`, `lsblk`, `blkid`, `swapoff`, `mount` | `util-linux` | re-read the table, resolve by number, inspect, mount |
| `mkfs.ext2`, `mkfs.ext4`, `e2fsck` | `e2fsprogs` | the two filesystems, planned UUIDs, the read-only check |
| `depthchargectl` | `depthcharge-tools` | the FIT kpart |
| `futility` | `vboot-kernel-utils` | verify the dev-signed body |
| `rsync` | `rsync` | the copy |
| `unshare`, `systemd-inhibit` | `util-linux`, `systemd` | private mounts, no sleep during the write |

`depthchargectl`, `futility` and `cgpt` are already device packages because the running card updates
its own kernel partition. `rsync`, `e2fsprogs`, `util-linux`, `systemd` and `sudo` are in
`packages/base.list`. No package is added for the TUI: the Rust binaries are staged, not packaged.

## 4. The plan

The plan is the complete decision, made before anything is written, and the only thing `apply`
executes:

| Item | Hayato value / source |
|---|---|
| Kernel partition | `p1`, start `deviceinfo_cgpt_kpart_start` (8192 sectors), size `deviceinfo_cgpt_kpart_size` (262144 sectors = 128 MiB), type `kernel`, label `pmOS_kernel`, flags `-S 1 -T 5 -P 10` |
| Boot partition | `p2`, start = p1 start + p1 size, 512 MiB (`BOOT_SIZE_MIB=512`), type `efi`, label `pmOS_boot`, `ext2` |
| Root partition | `p3`, start = p2 start + p2 size, size = disk sectors − root start − 33, type `data`, label `pmOS_root`, `ext4` |
| Root UUID | random v4, planned in the plan, passed to `mkfs.ext4 -U` |
| Boot UUID | random v4, planned in the plan, passed to `mkfs.ext2 -U` |
| Base command line | `deviceinfo` `kernel-cmdline.conf`, joined with single spaces |
| Root arguments | `root=UUID=<root uuid> rootwait rw` |
| Extra arguments | `/etc/lazyos/kernel-cmdline-extra` from the live card, if present (for example `resume=LABEL=lazyos_swap`) |
| fstab | the two managed lines below; every other live line (auxiliary swap, user mounts) is preserved |
| Source kernel | the **running** kernel release (`uname -r`), `/boot/vmlinuz-<release>`, `/boot/initrd.img-<release>`, `/boot/dtbs/<deviceinfo_dtb>*.dtb` |
| DTB order | the profile glob, sorted so hayato is first; the FIT's default config is the first DTB |

The fstab lines are pmbootstrap's options with systemd's pass column, exactly as the shell writes
them:

```
UUID=<root>	/	ext4	defaults	0 1
UUID=<boot>	/boot	ext2	nodev,nosuid,noexec	0 2
```

The root and `/boot` lines are replaced even if the card's own `/etc/fstab` differs; the other lines
survive. That is how `LABEL=lazyos_swap none swap sw,nofail,...` made on the card reaches the
installed system, and why the swap menu and the installer do not have two opinions about swap.

The **33 sectors** are the secondary GPT and its header; the root partition does not consume them.
The p1 and p2 constants come from the profile; the boot size and filesystem come from the same
defaults `lib-cgpt.sh` uses (`BOOT_SIZE_MIB=512`, `BOOT_FS=ext2`), overridable for a port.

The plan also stores:

- the hash of the profile, the command line and the kpart inputs;
- the live root UUID, the live root device and its parent disk;
- the target record from §3.3;
- the source's used bytes and the number of files, for the fit check and the progress;
- the identity steps (clone: machine-id, D-Bus machine-id, SSH host keys);
- a `plan_hash`, the SHA-256 of the canonical plan; the TUI displays its first 12 hex digits, and
  `apply` refuses to run if the file changed after the plan.

`/run/lazyos-install/` holds:

| Path | Mode | Contents |
|---|---|---|
| `plan.json` | 0600 root | the full plan; v0.4.1 adds the password **hash**, never the password |
| `plan-summary.json` | 0644 | everything the TUI draws: target, geometry, UUIDs, command line, warnings, hash |
| `state.json` | 0644 | phase, progress, first-write flag, result, error, pid/pgid |
| `engine.log` | 0644 | the tools' output, with secrets redacted; the TUI's log pane and `journalctl` |
| `private/` | 0700 root | the kpart image until it is written; nothing else secret in v0.4.0 |
| `target/` | 0755 | the mount point; empty in the card's namespace because the mounts are private |
| `lock` | 0600 | `flock`: one engine at a time |

The plan fixes the target's UUIDs **before** mkfs, and the kpart is built from the live `/boot`
with the planned root UUID **before** the first target write. A copy that finishes with a UUID the
kpart does not name is therefore impossible; a kpart that does not fit is refused while the target
still has its old contents.

## 5. The phases

The state machine, in order. The engine enters the private mount namespace and starts the inhibitor
before the phases; the table's "cancelable" column describes the **target** writes. **The first write
is the GPT in `partition`; everything before it is cancellable, nothing after it is.**

| # | Phase | Writes | Cancellable | What it does |
|---|---|---|---|---|
| 1 | `probe` | nothing | yes | profile match, live/installed, target candidates, tool check, battery/AC |
| 2 | `plan` | `/run` only | yes | the plan above; plan hash; target revalidation record |
| 3 | `kpart` | `/run` only | yes | build the FIT from the live `/boot` and the planned UUID; `futility show` must say `Body verification succeeded`; fit check against p1 |
| 4 | `prepare` | nothing on the target | yes | stop `udisks2`, unmount the target's partitions, `swapoff` them |
| 5 | `partition` | **target GPT** | **no** | zero both GPT areas, `parted mktable gpt`, `cgpt create`/`add` with the types, labels and flags above, `partx -d`/`-a`, `udevadm settle`, resolve by `PARTN`, assert the three labels |
| 6 | `format` | target filesystems | no | `mkfs.ext2 -F -q -L pmOS_boot -U <boot>`, `mkfs.ext4 -F -q -L pmOS_root -U <root>`, read the UUIDs back, mount root then `/boot` |
| 7 | `copy` | target filesystems | no | the clone rules in §6; `rsync` with progress; recreate the mount-point directories; verify `/dev` and `/sbin/init` |
| 8 | `identity` | target | no | clone cleanup in §6; v0.4.1 creates the fresh user/home here instead |
| 9 | `boot` | target `/etc/fstab`, p1 | no | rewrite fstab preserving extras; verify the target's `/boot` matches the kpart inputs; write the prebuilt kpart to p1; `futility show` on p1; `cmp` with the built image; `cgpt show` flags; `sync` |
| 10 | `verify` | nothing | no | `/dev`, `/sbin/init`, the fstab UUIDs, the plan's root UUID against the formatted root, the target's `/boot` against the kpart inputs, the marker preconditions |
| 11 | `marker` | target `/etc/lazyos/installation` | no | the last write to the target: the JSON from §3.2; `sync` |
| 12 | `finish` | `/run`, udisks state | no | unmount `/boot` then the root, `e2fsck -f -n` read-only, leave `udisks2` stopped on success (so it cannot auto-mount the fresh target), restart it on failure/cancel, exit so the namespace and the inhibitor's parent go away, write the final state |

Details that are not obvious:

- **`prepare` stops udisks2 on purpose.** `lib-cgpt.sh` refuses to run while the daemon is active
  because it auto-mounts the new partitions within a second of `partx -a`, after the table was
  already replaced. The engine handles that itself: stop it before the table, leave it stopped on
  success, restart it on a failed or cancelled run. D-Bus activation starts it again on the next
  client request (a login, a desktop volume monitor, `udisksctl`), and a reboot starts it normally;
  the installed target is not auto-mounted in the meantime.
- **The kpart is built from the live `/boot`, not from the target's copy.** The copy is made later
  and could in principle differ; the engine hashes the kernel, initramfs and DTBs before the copy
  and compares the target's files after it. A mismatch stops before p1 is written, leaving the
  target's filesystems written but not bootable; the card is intact and the engine can be re-run.
- **The check is `futility show`, not a file size.** The FIT must be dev-signed and its body must
  verify; a kpart that only fits is not enough. The partition is checked again after the write.
- **The marker is last.** A target with the marker but no kernel partition would claim to be
  installed and not boot; the reverse (a bootable target with no marker) is harmless: the next boot
  is live and the installer offers to finish.
- **The cancel path.** Before `partition`, `cancel` sends SIGTERM to the engine's
  process group; the engine stops at the next phase boundary, unmounts, restores udisks2/inhibitor
  and writes `cancelled`. After `partition`, `cancel` exits with `not cancellable` (code 5) and
  sends nothing; the TUI disables the action and says why. A SIGKILL can still kill the engine; the
  card is intact, the target is half-written, and a failed state is recorded by the next run's
  probe (or by `doctor`). No design can make SIGKILL safe; the answer is that the card is the
  rescue system.

## 6. The copy, the identity and the boot handoff

### 6.1 The clone

The engine copies the **running** filesystem, following the rules `install-emmc.sh` has proved,
with the identity excludes moved into the copy:

```
rsync -aHAX --delete --info=progress2 \
  --exclude=/proc --exclude=/sys --exclude=/dev --exclude=/run \
  --exclude=/mnt --exclude=/media --exclude=<target mountpoint> \
  --exclude=/tmp/* --exclude=/var/tmp/* \
  --exclude=/var/cache/* --exclude=/var/log/journal/* \
  --exclude=/etc/machine-id --exclude=/var/lib/dbus/machine-id \
  --exclude=/etc/ssh/ssh_host_* \
  / <target mountpoint>/
```

- **No `-x`/`--one-file-system`.** `/boot` is a separate filesystem mounted at `<target>/boot`;
  `-x` would silently skip the kernel and initramfs. The target mount is inside `/run`, which is
  excluded, so there is no recursion.
- **ACLs, xattrs, hardlinks and ownership** are part of the copy (`-aHAX`), because the installed
  system has to be the tested one, not a mode approximation.
- **Some excludes are whole directories, some are contents; the directories are recreated.** rsync
  drops the excluded directories entirely; the initramfs's `run-init` needs `/dev`, `/proc`, `/sys`,
  `/run`, `/mnt` and `/media` to move its own mounts into. The engine recreates `dev:755 proc:555
  sys:555 run:755 mnt:755 media:755` root-owned after the copy, the same list and modes as the
  shell.
- **The identity files never land.** The shell copies `/etc/machine-id` and the SSH host keys and
  then empties/removes them, leaving the card's identities in the target's free blocks. The engine
  excludes them from the copy in the first place, writes an empty `/etc/machine-id` (mode 0444) in
  `identity`, leaves `/var/lib/dbus/machine-id` absent (it is a separate regular file here, not a
  symlink, and D-Bus generates its own), and leaves `/etc/ssh/ssh_host_*` absent. The `sshd-keygen`
  drop-in from `rootfs-setup.sh` generates the host keys on the target's first boot.
- **The Wi-Fi stays.** `/etc/NetworkManager/system-connections` is part of the clone; the installed
  machine comes up knowing the user's networks. This is a personal install; the **published** image
  is still sanitized by `lib-dist.sh`, and the release path is unchanged.
- **The fit check runs before the copy.** The source's used bytes (from `statvfs`) must fit in p3
  with a margin; p2 has `BOOT_SIZE_MIB` and `/boot`'s used bytes must fit in it. The 58 GB eMMC
  against a card's used space is not the close case; the check is for the smaller targets.
- **The progress is the file count and the bytes.** `--info=progress2` is parsed into the state
  (`percent`, `detail`), not dumped once at the end. The TUI's gauge is the copy's own report; the
  other phases show their name and elapsed time.
- **The source is read-only.** The engine mounts the target, not the card, and runs no command that
  writes to `/`. The clone cannot modify the rescue system.

### 6.2 The identity

Clone mode, v0.4.0:

| Item | Action | Why |
|---|---|---|
| `/etc/machine-id` | created empty, mode 0444, excluded from the copy | systemd generates a new id on the target's first boot; two machines sharing one id share a journal and a DHCP identity |
| `/var/lib/dbus/machine-id` | excluded, left absent | D-Bus generates its own; on this system it is a separate regular file, so removing only `/etc/machine-id` is not enough |
| `/etc/ssh/ssh_host_*` | excluded from the copy, left absent | the target generates its own keys on first boot; the card's keys never travel |
| `/etc/lazyos/installation` | written last | the installed state, §3.2 |
| everything else | copied | the user, home, applications and Wi-Fi are the clone |

The target's `/etc/hostname`, `/etc/hosts`, user, password and home are the card's; clone mode does
not pretend to be a new user. That is the deliberate scope: the user already configured the card,
and the first release installs **that** system. Fresh mode is where the identity is new.

Fresh mode, v0.4.1, replaces the identity block with:

- create the user, home (from `/usr/share/lazyos/home-template`, plus the repo snapshot the card
  carries), groups and password hash (read from stdin or the autoinstall `USERPASS_HASH`, never
  argv);
- copy `/etc/skel` and the template, set ownership, mode and the shell;
- write `/etc/hostname` and `/etc/hosts`;
- write `/etc/locale.gen`, run `locale-gen`, write `/etc/default/locale` and `/etc/locale.conf`;
- write `/etc/localtime` and `/etc/timezone` from the chosen zone;
- install the passwordless-sudo drop-in for the new user, and **not** the card's user;
- leave the card's Wi-Fi and the applications, because the source is still the tested card.

The engine refuses `--mode fresh` until that path has been tested on hardware; the mode is not
half-wired in v0.4.0.

### 6.3 The boot handoff

The kpart is built with the same tool and arguments as `make-kpart.sh`:

```
depthchargectl build --root none \
  --board <deviceinfo_depthcharge_board> \
  --kernel <live>/boot/vmlinuz-<uname -r> \
  --kernel-cmdline "<base cmdline> root=UUID=<root> rootwait rw <extra>" \
  --initramfs <live>/boot/initrd.img-<uname -r> \
  --fdtdir <staged DTBs> \
  --compress <deviceinfo_depthcharge_compression> \
  --timestamp <SOURCE_DATE_EPOCH or now> \
  --tmpdir <private>/dct \
  --output <private>/kpart.img
```

`kern_guid=%U` is **not** added: `mkdepthcharge`/`depthchargectl` prepends it, as `make-kpart.sh`
documents. The command line has exactly one space between fragments; the base file's final newline
must not glue `root=UUID` to the last fragment. The DTBs are staged in a temporary directory
(`--root none` means the FIT carries the kernel, initramfs and DTBs itself), sorted with the
Hayato DTB first, because with `--board mt8192` no config matches by `compatible` and depthcharge
falls back to the first config.

`futility show` must report a dev-signed key block and `Body verification succeeded`; the image
must fit in 128 MiB; both checks happen before `partition`. After the copy the target's
`/boot/vmlinuz-<release>` and `/boot/initrd.img-<release>` are compared with the inputs; after
`boot` the partition is compared with the image and `cgpt show` reports the kernel entry with its
flags.

The `depthcharge-tools.service` on the installed system stays enabled: the firmware spends one try
per boot until the partition is marked successful, and `depthchargectl bless` is what marks it.
Remove that service and the target becomes unbootable after five boots. The installer sets the
partition successful initially (`-S 1`), but the service is still the mechanism that keeps it so
after kernel updates.

## 7. Safety invariants and the protocol

### 7.1 Invariants

1. **The running disk is never a target.** Root and `/boot` are resolved to their parent disks;
   both are excluded, as is every disk with a mounted partition.
2. **The SPI is never written.** `mtd*`, `mmcblk*boot*`, `*rpmb`, `zram*`, `loop*`, `dm-*`, `md*`,
   `ram*`, `sr*`, `nbd*`, `rbd*` and `fd*` are not candidates by name before the profile hint is
   considered. Only GPT partitions of the chosen target are touched; the firmware is not.
3. **The profile is the only device input.** No board, DTB, partition offset, boot size or command
   line is hardcoded in the engine; a missing or mismatched profile stops before any target read
   that could lead to a write.
4. **The plan is fixed before the first write.** `apply` re-probes the profile, the running root,
   the target's path/size/model/serial and the plan hash. A change means a new plan, not a patched
   one.
5. **The kpart is built and verified before the first write.** Its root UUID is the one the plan
   will format; its inputs are hashed; it must fit and pass `futility show`.
6. **The first write is the GPT.** Before it, cancel is a clean unmount and a restored desktop;
   after it, no cancel, no prompt, no second confirmation. The TUI disables the action and says the
   install cannot be stopped.
7. **The mounts are private and the machine does not sleep.** The engine enters a private mount
   namespace (`unshare(CLONE_NEWNS)` plus `MS_REC|MS_PRIVATE` for `/`, or the `unshare -m` utility)
   so the target's mounts never appear in the card's session, and runs under
   `systemd-inhibit --what=sleep:idle:handle-lid-switch --mode=block`. The TUI also pauses the
   session's `swayidle` so the screensaver does not paint over the install; its guard resumes it,
   and a stale guard is resumed by the next installer start.
8. **The boot chain is checked at both ends.** The kpart is verified before the write and the
   written partition is verified after it; the fstab UUIDs are read back from the formatted
   filesystems; the command line carries the planned root UUID; `/dev` and `/sbin/init` are checked
   before p1 is touched.
9. **The marker is last.** The target is installed only when every earlier phase has succeeded and
   the marker has been written and synced. A crash before it leaves a live-detected target for the
   next boot and a clear failed state.
10. **The target's loss is named, and Cancel is first.** The Target and Confirm screens list the
    target's model, size, serial and existing partitions; the confirmation's first row is Cancel;
    the Install row requires moving to it. The engine does not back up the old target; the card is
    the rescue system and `backup-system.sh` is how a backup is made.
11. **Secrets never touch argv, the log or the state.** v0.4.0 has no secret. v0.4.1's Fresh
    password is read from stdin in the TUI, hashed in memory, stored only in the 0600 plan and in
    the 0600 autoinstall file as `USERPASS_HASH`; the plaintext is never written and never echoed.
12. **No shell is sourced to make a decision.** `deviceinfo`, the plan, the state and the marker are
    data parsed by Rust; the tools are executed with argument vectors.

### 7.2 The engine CLI

```
lazyos-install probe [--json]
lazyos-install plan [--mode clone|fresh] [--target auto|PATH]
                    [--password-stdin]  # v0.4.1; refused in a clone plan
                    [--json]
lazyos-install apply --plan PATH [--json]
lazyos-install cancel [--json]
lazyos-install status [--json]
lazyos-install doctor [--json]
lazyos-install auto CONFIG [--json]
lazyos-install kpart [--image PATH] [--confirm] [--json]
lazyos-install version
```

- `--json` writes one object to stdout and nothing else; progress belongs to the state.
- `plan` writes `plan.json`/`plan-summary.json`; `--target auto` uses the profile hint, then the only
  candidate. `--mode fresh` returns a typed "fresh mode is not implemented in this release" until
  v0.4.1.
- `apply` requires root, the plan file, the lock and a revalidated target. It is the only command
  that writes the target.
- `cancel` sends SIGTERM to the engine's process group only before the first write; after it, it
  refuses with code 5.
- `status` never needs root and never lies: it reads the state and, if the engine's pid is gone
  without a final result, reports `interrupted` with the last phase.
- `doctor` is the read-only diagnostic the manual and `check-system.sh` call: the probe plus the
  marker's target/root comparison, the last state and log, the autoinstall file, the tool list and
  the kpart build check. It never writes the target.
- `auto` reads the KEY=value file, builds a plan and applies it with no UI; `CONFIRM=yes` is
  required, and a `MODE=fresh` config without a password hash is refused.
- `kpart` is the device-facing replacement for `install-kpart.sh`: it resolves `kern_guid` from
  `/proc/cmdline`, backs up the partition it is about to replace under
  `/var/cache/lazyos/kpart-backup-<timestamp>.img` (or `$WORK_DIR` on the build host), builds from
  the running root's `/boot`, verifies, writes and verifies again. It keeps the chroot guard: the
  stat of `/proc/1/root` and `/` must be the **same** device on a real system; differing means a
  chroot, and the engine refuses to rewrite the host's partition from the chroot's `kern_guid`.
- `version` prints `lazyos-install <VERSION>`; the same string is in the marker.

Exit codes are stable: `0` success; `1` usage/validation; `2` profile or device; `3` plan
missing/stale; `4` target changed/unsafe; `5` not cancellable; `6` kpart; `7` copy/verify; `8`
internal; `9` no running install. The `--json` error object carries a stable `code` and the tools'
own last lines, not a Rust backtrace.

### 7.3 The state and the log

`state.json` is written atomically (temporary file, fsync, rename); the TUI never reads a torn file.
A representative state:

```json
{
  "schema": 1,
  "version": "0.4.0",
  "plan_hash": "…",
  "phase": "copy",
  "phase_index": 7,
  "first_write": true,
  "cancellable": false,
  "result": "running",
  "progress": {"percent": 41, "detail": "12.3 GiB / 29.8 GiB"},
  "pid": 1234,
  "pgid": 1234,
  "error": null,
  "started_at": "…",
  "updated_at": "…"
}
```

The state's phases are the table in §5, in that order; the TUI draws the done phases, the current
one and the rest, not a fake percentage. `result` becomes `installed`, `cancelled`, `failed` or
`interrupted` once; the TUI's Done/Failed screen reads it.

The log is line-oriented and root-owned `0644`; the TUI tails it in the Install and Failed screens.
Every external command is logged with its argument vector (the v0.4.0 arguments contain no secret),
its exit status and its last lines on failure. The TUI's "show log" is a scroll pane; the manual
tells the user where the file is.

### 7.4 The engine's session

The TUI starts the engine as root through the user's passwordless sudo, under `setsid` and
`systemd-inhibit`, with `--plan` pointing at the plan it generated. The engine:

1. takes the `/run/lazyos-install/lock` (`flock`, non-blocking);
2. revalidates the plan and the target;
3. enters the private mount namespace and starts the inhibitor (if the wrapper did not);
4. runs the phases, updating the state at each boundary and on meaningful progress;
5. releases the lock, restores udisks2 on failure/cancel, writes the final state.

The engine's stdout/stderr are the log; it never writes to the terminal because there is none. A
second `apply` fails on the lock with the first engine's pid. The TUI closing does not send
anything to the engine; `cancel` is an explicit command. The `setsid` session is what keeps the
engine alive when the terminal goes away: it is not in the terminal's session or foreground
process group, `systemd-inhibit` is its parent and holds the lock until the engine exits, and the
engine is reparented if the TUI exits.

## 8. The TUI

### 8.1 Host and compositor

The TUI is a **fullscreen foot**, not a layer-shell surface:

```
foot --app-id=org.lazyos.installer --fullscreen --title "LazyOS Installer" \
  -e lazyos-installer
```

The session's drop-in `etc/sway/config.d/30-installer.conf` carries:

```
# The installer: open once on a live card, and keep it fullscreen.
exec-once lazyos installer-autostart
for_window [app_id="org.lazyos.installer"] fullscreen enable
```

**Why not a layer-shell GUI.** Ratatui is a terminal UI: it needs a terminal, and foot is the
terminal the session already ships, themes and fonts. A layer-shell client would have to draw its
own text and input handling, duplicate the palette work, and add a GUI toolkit to an image that has
none of the heavy ones. The TUI in foot is one process for the duration of the installation, and
the compositor, the keyboard and the touchscreen work with it unchanged.

**Sway bindings vs the TUI.** Sway evaluates its bindings before it forwards a key to the focused
client. Every `Alt+…` binding of the session therefore stays live behind the installer: `Alt+1`
would switch workspace instead of reaching the TUI, and `Alt+Q` would close the window behind it.
The TUI uses **no Alt chords**: arrows, `j`/`k`, `Tab`, `Enter`, `Esc`, `q`, `?`, `r`, `s`, `l`,
`PageUp`/`PageDown`. The same lesson `config.rasi`'s `global-kb` teaches rofi, solved here by not
needing the modifier. `q`/`Esc` on the Install screen closes the TUI and leaves the engine running;
the TUI says so before it closes. A second TUI is prevented by a `flock` in the user's runtime
directory, and the host opens the existing install attached instead.

**The idle chain.** The session runs `swayidle` with a 150 s screensaver and a 300 s lock. If it
kept running, the screensaver would paint over the fullscreen installer, and the lock would appear
in the middle of the confirmation. The TUI:

- finds the session's `swayidle`, sends `SIGSTOP`, and records its pid and that it was running in
  `$XDG_RUNTIME_DIR/lazyos-installer/idle.pid`;
- on exit, on failure, or on the next start with a stale guard, sends `SIGCONT` and removes the
  file;
- never stops a `swaylock` that is already up; the user unlocks first.

The engine's `systemd-inhibit` is separate and stronger: it blocks logind's sleep, idle and
lid-switch handling for the whole write, even if the TUI is closed. The TUI's pause is for the
picture; the inhibitor is for the machine. The manual says the screen will not lock while the
installer is open, and the check-system warns when `swayidle` is stopped and no installer is
running (a crashed TUI would otherwise leave the session without an idle screen).

### 8.2 Screens

v0.4.0 has seven screens plus the Installed notice and the help overlay:

| Screen | Shows | Keys |
|---|---|---|
| **Welcome** | the brand, version, "live card", the device name/codename, battery/AC, one paragraph: the card is the source, the internal disk will be erased, there is no undo | `Enter` begin, `?` help, `q` close |
| **Device** | profile match, board, running kernel, running root/disk, the card, the checks that passed/failed, warnings (no AC, low battery, source kernel mismatch, no target) | `Enter` continue, `r` rescan, `Esc` back |
| **Target** | the candidate whole disks: path, model, size, serial, current partitions; the excluded disks and the reason; the profile hint marked; if none, the explanation | `↑`/`↓`, `Enter` select, `r` rescan, `Esc` back |
| **Confirm** | the chosen target's model/size/serial and every existing partition that will be gone; the planned p1/p2/p3 with sizes; the source kernel/root/used; clone mode and the identity cleanup; "no undo; after the first write it cannot be stopped"; **Cancel first**, Install second | `↑`/`↓`, `Enter`, `Esc` = Cancel |
| **Install** | the phases with done/current/pending, the copy's gauge, elapsed time, the log tail, the "cannot be stopped" banner after the first write | `s` stop **before** the first write, `Enter`/`Esc`/`q` close the window (engine continues), `l` log |
| **Done** | the target, "remove the card", the firmware menu step (internal disk), the rescue card sentence, reboot/close | `r` reboot (asks), `q` close |
| **Failed** | the phase that failed, the tool's last lines, "the target may be half-written; the card is intact", the log, retry | `l` log, `r` re-plan, `q` close |
| **Installed** | when the TUI is launched on an installed system: "this system is installed; run the installer from the live card" | `q` close, `?` help |

The **Welcome** screen is the only place that says what the installer is. The **Confirm** screen is
the only place that lists what is lost; it does not rely on the Target screen having been read. The
**Install** screen's stop action is present but disabled from the moment `first_write` is true, with
the reason next to it; the engine refuses a `cancel` in that state as well.

v0.4.1 adds:

| Screen | Shows | Keys |
|---|---|---|
| **Mode** | Clone (this card's user and apps) / Fresh (new user, home and hostname), with the trade-off | `↑`/`↓`, `Enter` |
| **Identity** | username, full name, hostname, password + confirmation (masked), locale, timezone; validation inline | `Tab`, `Enter`, `Esc` back |

### 8.3 Look and testing

The TUI uses the session's solarized palette: `base03` background, `base0` text, cyan for the
selection and the brand, yellow for warnings, red only for the irreversible action. It draws with
Ratatui styles and box-drawing characters; it uses **no Font Awesome codepoints**, because foot's
fallback list does not carry them (the reason the file browser's rows use the theme's PNGs and the
terminal's menus use `UI_I_*` only where a Nerd Font glyph exists). The Welcome screen's picture is
the wordmark text (`~/.config/lazyos/wordmark.txt`) if it is present, otherwise the name; the
launcher's picture is the `lazyos` theme icon.

The layout is responsive down to 80x24: the phase list collapses to the current phase, the target
table hides the serial column, and the log pane shrinks. `TestBackend` snapshots at 80x24 and at the
panel's 1366x912 are committed, along with key-sequence tests for the destructive paths (Cancel
first, Stop only before the first write, `Esc` closing without cancelling).

## 9. Autoinstall

`/etc/lazyos/install.conf` is a root-only (`0600`) KEY=value file:

```
MODE=clone
TARGET=auto
CONFIRM=yes
# v0.4.1:
#USERNAME=lazyos
#HOSTNAME=lazyos
#USERPASS_HASH=$6$...
#LOCALE=en_US.UTF-8
#TIMEZONE=America/Mexico_City
```

`lazyos-autoinstall.service` is shipped **disabled**:

```ini
[Unit]
Description=LazyOS unattended install
ConditionPathExists=/etc/lazyos/install.conf
After=local-fs.target
Before=greetd.service

[Service]
Type=oneshot
TimeoutStartSec=infinity
ExecStart=/usr/bin/lazyos-install auto /etc/lazyos/install.conf
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

- The operator enables it deliberately (`systemctl enable lazyos-autoinstall.service` on the card)
  and is responsible for the card that it will erase. The unit exists so a fleet card can install
  without a person at the screen; it is not a default.
- `Before=greetd.service` keeps a graphical session from competing for the target while the engine
  runs. The engine holds its own inhibitor; the machine does not suspend mid-install because a lid
  closed.
- `Restart=no` is intentional: a failed destructive action must not loop. The state/log and the
  journal are the record; the card still boots.
- The auto path uses the same `plan` and `apply` as the TUI. It has no UI, no prompt and no
  password in the environment. A `MODE=fresh` without `USERPASS_HASH`, an unknown key, a
  duplicate key, a `CONFIRM` other than `yes`, or a target that fails the profile/running-disk
  checks is a plan error, not a partial install.
- The **published** image must not carry the file or the unit's enablement symlink. `lib-dist.sh`
  gains both to its sanitizer list and `dist_assert_clean` fails while either remains; the release
  notes say the autoinstall file is the operator's. `check-system.sh` reports a live card with the
  file as a warning ("this card will install on boot"), because that is a legitimate provisioning
  card but never a released one.

v0.4.0's autoinstall supports `MODE=clone` only. `USERPASS_HASH` arrives with Fresh mode; the parser
rejects it in a clone file instead of silently keeping a secret it does not use.

## 10. Packaging and build

### 10.1 The workspace

```
rust/Cargo.toml                 workspace, resolver 2, Cargo.lock committed
rust/crates/lazyos-core         profile parser, compatible, run paths, live/installed, targets
rust/crates/lazyos-disk         GPT, mkfs, fstab, mount, kpart, verify
rust/crates/lazyos-identity     clone cleanup; fresh identity in v0.4.1
rust/crates/lazyos-install      probe/plan/apply/state/doctor/auto + binary
rust/crates/lazyos-installer    Ratatui TUI + binary
rust/crates/lazyos              host: installer, installer-autostart, disk helpers
```

- Dependencies are pure Rust for the runtime path (`serde`, `serde_json`, `ratatui`, `crossterm`,
  `sha2`, `uuid` with `v4`, `rustix`/`libc` for the syscalls). No C library is built for these
  crates; `mkfs`, `cgpt`, `parted`, `rsync` and `depthchargectl` are the system tools.
- `cargo build --release --locked`; the target directory is under `$WORK_DIR`; the release profile
  strips symbols. The `VERSION` file is the only source of the version: `build-rust.sh` exports it,
  `build.rs`/`env!` bakes it, and the TUI, `--version`, the marker and the release notes all read
  it. A build cannot say 0.2 while the tag says v0.4.0.
- The binary is built natively for aarch64. The build script runs `cargo test --locked` (unit and
  `TestBackend`; the loop test is `--ignored` and root-only) before installing it into the rootfs,
  and runs `lazyos-install version` inside the chroot so a glibc mismatch fails the build instead
  of the card.

### 10.2 The build order

`scripts/build-rust.sh` builds into `$WORK_DIR/rust-stage/usr/bin/`; `scripts/install-rust.sh`
copies the three binaries into the rootfs. `scripts/build.sh` becomes:

```
build-kernel → build-swayfx → build-rust → build-rootfs → install-kernel
             → install-rust → install-swayfx → make-kpart
```

`build-rootfs.sh` already stages the committed tree, the profile, the UI overlay and the manual.
The Rust binaries are installed after the chroot exists and before `make-kpart`; `make-kpart` does
call the host `lazyos disk kpart` in v0.4.0, replacing the shell's `depthchargectl` invocation.
`00-deps.sh` already installs `rustc`/`cargo` (the kernel's Rust needs them); no new host package
is required for the TUI.

### 10.3 The image

- Binaries: `/usr/bin/lazyos`, `/usr/bin/lazyos-install`, `/usr/bin/lazyos-installer` (0755, root).
- Profile: `/usr/share/lazyos/{deviceinfo,compatible,kernel-cmdline.conf,modules-initfs,home-template/}`.
- Launcher: `/usr/share/applications/org.lazyos.installer.desktop` with `Icon=lazyos`,
  `Terminal=false`, `Exec=lazyos installer`.
- Session: `etc/sway/config.d/30-installer.conf` with the `exec-once` and `for_window` lines.
- Root menu: the installer row in `bar-menu`'s `system` group.
- Manual: `/usr/share/doc/lazyos/08-installer.md`; `rofi-learn` lists it automatically (its loop
  reads every `*.md`), so no menu edit is needed for the page itself.
- Marker: not shipped; written by the installer on the target.
- Autoinstall: the unit is shipped but not enabled; the config is not shipped.

No Debian package wraps the binaries: the image is assembled from this repository, and the Rust
workspace is the source of truth. A `.deb` would duplicate the version and the staging logic for a
program that only exists inside LazyOS.

## 11. Tests and witnesses

### 11.1 Tests

| Test | Where | What it proves |
|---|---|---|
| Profile parser, compatible match, target filters | `cargo test`, `lazyos-core` | comments/quotes parse; a wrong profile fails closed; `mtd*`/`mmcblk*boot*`/the running disk are never candidates; the kernel partition does not exclude the eMMC |
| Geometry, fstab, command line, marker, plan hash | `cargo test`, `lazyos-disk`/`lazyos-install` | the formulas and the 33-sector reserve; the two fstab lines; the one-space command line; the marker's fields; a changed plan is refused |
| TUI snapshots and key sequences | `TestBackend` | 80x24 and 1366x912 layouts; Cancel first; Stop disabled after the first write; `Esc` closes without cancelling |
| **Loop diff against the shell** | root integration test / `scripts/test-installer-loop.sh` | the Rust GPT on a 2048 MiB loop matches `lib-cgpt.sh`: `cgpt show`, `parted -m`, the PMBR signature, types/labels/flags, the geometry; the partition that `partx` reorders is still resolved by `PARTN`; the labels abort a wrong resolution |
| kpart golden | root/host test | `depthchargectl` arguments, `kern_guid` not duplicated, `futility` says the body verifies, the image fits 128 MiB, the Hayato DTB first |
| Autoinstall parser | `cargo test` | unknown/duplicate keys, `CONFIRM`, the clone-only schema, the fresh hash rule |
| Dry run on the card | hardware | the TUI opens once on a live card, the Device/Target/Confirm screens match `lsblk`, the plan is refused on the installed system, no target write |
| Real install | hardware checklist | the card installs the eMMC; the target boots; `check-system.sh` is green; the marker and the new machine-id/SSH keys; the second install from the card |
| Recovery | hardware | a failed copy is detected as live, the card still boots, the target can be re-planned; a kpart that does not fit stops before the GPT |
| Auto path | hardware, test card | the unit runs before greetd, no prompt, the state/log record, `Restart=no` leaves a failed target alone |

The loop diff is the only reason the shell can be deleted with confidence. It runs the shell
library and the Rust crate on two loop devices of the same size, compares the tables byte for byte
where they must match and structurally where they may differ (UUIDs), and repeats the attach 16
times to catch the `partx` reordering. The expected output is kept as a golden after the shell is
gone, so a later change to the geometry fails a test instead of a card.

### 11.2 The three witnesses

**`check-system.sh`** gains an Installer section (the table is the implementation contract):

| Check | Severity | Fails when |
|---|---|---|
| `/usr/bin/lazyos`, `lazyos-install`, `lazyos-installer` exist and are executable | FAIL | a binary is missing or not executable |
| `/usr/share/lazyos/deviceinfo`, `compatible`, `kernel-cmdline.conf`, `modules-initfs` | FAIL | the profile is incomplete or the parser rejects it |
| `/usr/share/lazyos/home-template/` | WARN in v0.4.0, FAIL in v0.4.1 | Fresh mode's template is missing |
| The running DT matches a line of `compatible` | FAIL | the image is on the wrong device; both lists are printed |
| `org.lazyos.installer.desktop` exists, names `lazyos installer`, and `Icon=lazyos` is a PNG in the LazyOS theme | FAIL | the launcher row draws nothing |
| `config.d/30-installer.conf` has the `exec-once` and `for_window` lines | FAIL | the TUI does not open or is not fullscreen |
| `bar-menu` has the installer row | FAIL | the menu cannot reach it |
| `depthchargectl`, `futility`, `cgpt`, `partx`, `rsync`, `mkfs.ext2`, `mkfs.ext4`, `unshare`, `systemd-inhibit` | FAIL | the engine cannot finish |
| `sudo -n true` | FAIL | the TUI cannot start the root engine |
| The marker parses | FAIL | the installed state is ambiguous |
| The marker's root UUID matches `/` | WARN if not | a restored/copied card is live; `doctor` names it |
| The running system is the profile's install target with no marker | WARN | a v0.3.0 install (or a lost marker) is live to the installer; the card can reinstall it |
| A live card's no-marker state | OK | expected: the card is the installer; the check reports "live" |
| A running or failed install in `/run/lazyos-install` | WARN | the log explains; the machine is mid-install or needs a re-run |
| `/etc/lazyos/install.conf` on a live card | WARN | this card installs on boot; a published image must not have it |
| `08-installer.md` present and readable | FAIL | the manual is the operator's recovery |
| The binary version matches `/etc/debian-aarch64-revision` | WARN | the image and the binaries are from different builds |
| `swayidle` stopped with no installer running | WARN | a crashed TUI left the idle screen off |
| `lazyos-install probe --json` on a live card | WARN | no target; the card still works |

The check runs as the desktop user, as every other section; the root-only reads go through
`sudo -n`. It must not run `plan` or `apply`.

**The manual** is `08-installer.md`: the live card, the target, Clone mode and its identity, Fresh
mode (v0.4.1), the firmware menu, the autoinstall, the recovery (the card is the rescue system; a
failed target is re-installed, not repaired) and the "no in-place upgrades" sentence. It is the page
a person reads on the device, not a copy of this design.

**`AGENTS.md`** gains one bullet when the feature ships: the card is the installer; the marker and
the root-UUID detection; the target hint and why `RM=0` makes it necessary; the installer's target
discovery not being `ui_disks` and why; the first-write/cancel boundary; the kpart and the chroot
guard; the identity excludes; the TUI in foot with no Alt chords and the paused idle chain; the
autoinstall and the sanitizer; the loop/golden test. The bullet is the contract for the next person,
not a release note.

## 12. Migration map

The full tier table is `plan.md` §5. The installer's rows are:

| Today | v0.4.0 | The shell's remaining job |
|---|---|---|
| `scripts/lib-cgpt.sh` | `lazyos-disk`: geometry, GPT, mkfs, fstab, mount, kpart | `write-sd.sh`/`make-image.sh` copy glue calls `lazyos disk …` |
| `scripts/install-emmc.sh` | `lazyos-install apply` (clone, excludes, identity, boot, verify) | none; the script is the reference until the hardware checklist, then deleted |
| `scripts/install-kpart.sh` + `scripts/make-kpart.sh` | `lazyos-install kpart` + `lazyos-disk::kpart` | `build.sh` calls the Rust kpart for the image |
| `scripts/rootfs-setup.sh` (machine identity) | `lazyos-identity`: machine-id, D-Bus id, SSH keys, marker | the build's user/home creation stays shell until Fresh mode shares the rules; it runs before the Rust binaries are staged |
| `scripts/lib.sh` (device side) | `lazyos-core`: deviceinfo/compatible/profile | the host build's `lib.sh` stays (host orchestration) |
| layout calls of `write-sd.sh` / `make-image.sh` | `lazyos-disk` | the surrounding download/copy/publish glue stays shell |

The rule from `plan.md` applies: the shell path is the reference until its port's witnesses are
green; then the old path becomes a symlink or is deleted, the manual and `check-system.sh` name the
new command, and the loop/golden diff catches drift. `install-emmc.sh` is not replaced by a
half-shell/half-Rust script: the v0.4.0 path is the TUI/engine, and the script is removed only
after a real install and a reboot have been recorded in `reports/`.

## 13. Risks and open questions

| Risk | Mitigation |
|---|---|
| The target hint names the card, or the eMMC changes name | the hint is checked against the running disk and the exclusions; the target record revalidated by path/size/model/serial; the TUI lists every candidate; the card is refused as the running disk |
| The installer erases a disk `ui_disks` would have hidden | that is the point: the installer's own filter allows a ChromeOS kernel partition; the existing partitions are named in the confirmation; only the running disk, the SPI, the boot areas and the pseudo disks are excluded |
| A mounted target or udisks2 auto-mounts the new table | `prepare` stops udisks2, unmounts and swapoffs; `partition` re-checks after `partx`; the engine leaves udisks2 stopped on success and restarts it on failure |
| `partx` reorders partitions | resolve by `PARTN`, assert the three labels; the loop test repeats the attach 16 times |
| The kpart is built for a different root | the UUIDs are planned before mkfs and passed to the kpart; the kpart is built before the first write; the written partition is compared and `futility`-checked |
| The live `/boot` is not the running kernel | the engine uses `uname -r`, requires `/boot/vmlinuz-<release>`, `/boot/initrd.img-<release>` and the DTBs, and hashes them; a mismatch stops before p1 |
| The card's identity travels | `/etc/machine-id`, `/var/lib/dbus/machine-id` and `/etc/ssh/ssh_host_*` are excluded from the copy, not copied then removed; the first boot generates new ones |
| The TUI's `Alt` keys hit sway | no Alt chords; the compositor's bindings stay behind the window; the fullscreen foot has the focus |
| The screensaver/lock paints over the install | the TUI pauses `swayidle` with a pid guard; the engine's `systemd-inhibit` blocks sleep/idle/lid; a stale guard is resumed, and the check-system warns |
| Closing the window stops the install | the engine is in its own session under `setsid`, so the terminal's hangup does not reach it; it is reparented if the TUI exits; reattaching reads the state |
| `SIGKILL` leaves a half-written target | no design can stop it; the card is intact, the marker is absent, the target is live-detected and the failed state/log say which phase; re-run with a new plan |
| The autoinstall erases a disk nobody chose | the unit is disabled and the file is operator-created and 0600; the engine still refuses the running disk/SPI and requires `CONFIRM=yes`; the release sanitizer removes the file and the enablement |
| Fresh mode's password leaks | v0.4.1 only; stdin, in-memory hash, 0600 plan, 0600 auto file; never argv/log/state; the mode is refused until tested |
| The shell and Rust disagree | the loop diff and the golden; the shell is deleted only after the hardware checklist; the profile parser has the same fields as `lib.sh` |

Open questions, all deliberately after v0.4.0:

- **The kiosk session.** A minimal greetd session (sway with no bindings and no idle chain) would
  remove the Alt/idle handling from the TUI's world. tuigreet 0.9.1 cannot preselect a session, so
  it is a manual pick; v0.4.1 evaluates it after the fullscreen-foot path is used on hardware.
- **Fresh-mode defaults.** Whether locale/timezone come from the live card or a fixed default, the
  exact `USERPASS_HASH` format and the home template's contents (the manual or only the session
  files).
- **Autoinstall after success.** Reboot, power off or wait for the operator; v0.4.0 leaves the
  machine at the installed system's login and leaves the choice to the unit's operator.
- **Touch and the TUI.** The keyboard-first screens work with `wvkbd` in tablet mode; a touch-only
  target picker and larger rows are a v0.4.1 evaluation, not a v0.4.0 requirement.
- **The target's backup.** The installer does not copy the old target away; `backup-system.sh` does,
  and the card is the rescue system. A "back up first" row would need a destination and a size
  check that v0.4.0 does not have.
- **LUKS2 and A/B.** The plan/apply boundary is designed so a second slot or an encrypted root is a
  plan change, not an engine rewrite; neither ships before the single-slot clone is proven on more
  than one device.
- **`lazyos disk`'s public surface.** v0.4.0 uses it from the build scripts and the engine's
  crates; whether it becomes a documented host command or stays hidden is decided with the host
  tools in v0.5.0.

## 14. References

- [`plan.md`](plan.md) — the roadmap, the language policy, the migration tiers and the risks.
- [`AGENTS.md`](../AGENTS.md) — the boot chain, the ChromeOS GPT pitfalls, the identity rules and
  the feature witnesses.
- `scripts/lib-cgpt.sh`, `scripts/install-emmc.sh`, `scripts/make-kpart.sh`,
  `scripts/install-kpart.sh`, `scripts/lib-dist.sh` — the behavior this design reproduces or
  replaces.
- `scripts/rootfs-setup.sh` — the build-time identity and session setup that Fresh mode shares.
- [`recon.md`](recon.md), [`optimization.md`](optimization.md), [`test-checklist.md`](test-checklist.md)
  — the machine, the measurements and the hardware checklist.
- Upstream: Nura / postmarketOS, pinned `pmaports` commit
  `0e458a2417d6f5ed61c7b70621d5632e30136091` (2026-10-15); pmbootstrap's `pmb/install/partition.py`,
  `format.py`, `_install.py`, `blockdevice.py` and `boot-deploy-functions.sh`.

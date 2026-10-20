# System optimization for google-asurada (MT8192, 4 GB)

This port compiles its own kernel (`kernels/mediatek-mt81`, `scripts/build-kernel.sh`),
which means the kernel *configuration* is a knob we own — not something a distro
hands us. This file records what the device actually does today, and the changes
that are worth making because of it.

Every number marked *(measured)* was read from the device (the eMMC install,
kernel 7.2.7-mt81, `cat /proc/...`, `systemd-analyze`, `/sys/...`). The rest are
config facts read from `kernels/mediatek-mt81/config.aarch64`. Where the running
host differs from what `uis/sway` ships (the host is an older Omarchy-era
install), that is said explicitly — it is the difference between "this device"
and "this image".

Items are ordered by (effect × safety) / effort.

## Status

**In this batch**: 1 (MGLRU), 2 (`schedutil`), 3 (`HZ=1000`), 4 (`PREEMPT_DYNAMIC`),
11a (sysctl drop-in, written by `rootfs-setup.sh`), 11b (`systemd-zram-generator`
replaces `zram-tools`; the swap is now a `dev-zram0.swap` unit), 11c
(`systemd-oomd`), 11d (the algorithm is written down as `lz4`, with `zstd` the
alternative to measure), 12 (`transparent_hugepage=madvise` on the command line),
13 (kyber udev rule). `check-system.sh` grew a "System tuning" section that fails
or warns on each of them, because the failure mode here is silent:

**Next**: 5 (kernel zstd), 6 (thin LTO), 9 (dead options) — one card test.
**Optional**: 7 (zswap needs `ZPOOL`/`FRONTSWAP` first), 8 (sched_ext).
**Left out on purpose**: 14 (IRQ affinity). Pinning every IRQ to CPU0 may be the
deliberate low-power choice on this SoC — it is what ChromeOS does — so it needs a
before/after measurement of both latency and idle drain, not an assumption. It
stays in the list as something to measure, not something to ship.

## 1. What the device does today

### CPU

| | |
|---|---|
| Clusters | `policy0` 4×A55, 500 MHz - 2.0 GHz, capacity 388. `policy4` 4×A76, 774 MHz - 2.2 GHz, capacity 1024 *(measured)* |
| Governor | **`ondemand`** (Debian's `CONFIG_CPU_FREQ_DEFAULT_GOV_ONDEMAND=y`); `schedutil` is compiled in and available |
| Energy-aware scheduling | **off**: the DT has `capacity-dmips-mhz` (capacities are parsed) but **no `dynamic-power-coefficient`**, so no energy model registers and EAS cannot activate |
| AMU / PSI / uclamp | `CONFIG_ARM64_AMU_EXTN=y`, `CONFIG_PSI=y`, `CONFIG_UCLAMP_TASK=y` *(measured: `/proc/pressure/{cpu,io,irq,memory}` exist)* |
| `time_in_state` *(measured)* | little: 81 134 ticks at 2.0 GHz and a thin tail below; big: 142 675 at 2.2 GHz, **0** at 2.1/1.98/1.93 GHz |

`ondemand` ramps to the maximum and stays there. That is the classic signature in
the last row: with `schedutil` (and the AMU the driver has) the clusters spend
their time at the frequency the work actually needs. This is the difference
between battery and no battery on a 2-cluster Chromebook, and it costs one
config line.

`CONFIG_HZ=250` (`CONFIG_HZ_250=y`): input and scheduling latency for a desktop
are better at 1000. `CONFIG_PREEMPT=y` is already the full-preemption model.

### Memory

| | |
|---|---|
| RAM | 3 896 816 kB total, ~1.0 GB available under the load this was read at *(measured)* |
| Swap | `zram0` 1 948 156 kB (`PERCENT=50`), priority 100, **1.4 GB in use** *(measured)* — on the host the active algorithm is `lzo-rle`, in the image `ALGO=lz4` |
| THP | `[always]` |
| `swappiness` | 60 (Debian default) in the image; the host has 150, from Deboma's `/etc/sysctl.d/99-omarchy-sysctl.conf` |
| dirty limits | host: `dirty_bytes=256 MiB`, `dirty_background_bytes=64 MiB`; **the image ships no sysctl drop-in at all** |
| MGLRU | **off** — `CONFIG_LRU_GEN` is not set |
| zswap | `CONFIG_ZSWAP=y` but `CONFIG_ZPOOL`/`CONFIG_FRONTSWAP` are **not**, so zswap cannot be enabled: the option is inert |

### Storage and interrupts

| | |
|---|---|
| I/O scheduler | host: **kyber** (Deboma's `/etc/udev/rules.d/60-omarchy-io-scheduler.rules`); **the image has no scheduler rule**, so the eMMC gets `mq-deadline` (kernel default; both schedulers are compiled in) |
| `read_ahead_kb` / `nr_requests` | 128 / 32 (eMMC), 128 / 128 (card) |
| IRQ placement *(measured)* | everything on CPU0: eMMC `11f60000.mmc` 1 024 210, i2c `11d21000.i2c` 351 792, WiFi `rtw88_pci` 376 090, and `irqbalance` is not installed |
| Modules | 30 MB (zstd-compressed) |

### Boot

`2.445 s kernel + 5.183 s userspace = 7.628 s` *(measured, `systemd-analyze`)*.
Userspace blame: `dev-mmcblk0p3.device` 1.6 s, `systemd-udev-trigger` 1.2 s,
`systemd-journald` 0.9 s, `upower` 0.76 s, **`depthcharge-tools.service` 0.66 s**,
`systemd-binfmt` 0.6 s, `rtkit-daemon` 0.55 s.

**Do not mask `depthcharge-tools.service`.** It runs `depthchargectl bless`, which
marks the ChromeOS kernel partition successful and resets its `tries` counter
(`-S 1 -T 5 -P 10` in `lib-cgpt.sh`). Without it the firmware burns one try per
boot and after five the kernel partition is no longer bootable. The image ships
and enables it (`/etc/systemd/system/basic.target.wants/`), and that 0.66 s is the
price of the boot chain. `systemd-binfmt` (0.6 s) is only needed for foreign-arch
binaries (`qemu-user`); everything else on the list is a few hundred ms each and
worth more than it costs.

## 2. Kernel configuration (we compile it, so these are ours)

| # | Option | Today | Propose | Why / cost | Risk |
|---|---|---|---|---|---|
| 1 | `CONFIG_LRU_GEN` + `CONFIG_LRU_GEN_ENABLED` | off | **on** | Multi-generational LRU: reclaim decides by generation instead of scanning everything, which is exactly the 4 GB-under-zram case (ChromeOS ships it for this). Runtime knob stays at `/sys/kernel/mm/lru_gen/enabled`. | low; new MM default, but it is the kernel's own upstream feature |
| 2 | `CONFIG_CPU_FREQ_DEFAULT_GOV_SCHEDUTIL` | `ondemand` | **`schedutil`** | Frequency follows scheduler utilization + AMU instead of `ondemand`'s sampling ramp-to-max (see the `time_in_state` row). Also the prerequisite for EAS (item 12). Runtime alternative: a systemd rule writing the governor, no rebuild. | low |
| 3 | `CONFIG_HZ_1000` | `HZ_250` | **1000** | Desktop latency: timer granularity 4 ms → 1 ms. Costs a little idle power; with `NO_HZ_IDLE` the tick is off when idle anyway. | low |
| 4 | `CONFIG_PREEMPT_DYNAMIC` | not set | **on** | Keeps `PREEMPT=y` as the default but lets `preempt=full|voluntary|none` be chosen on the command line, so the model can be A/B tested without a rebuild. | low |
| 5 | `CONFIG_KERNEL_ZSTD` | unset (kernel **Image is uncompressed, 15.6 MB**) | **on** | The kpart is 29 MB, most of it kernel + initramfs; a compressed `Image` (~6 MB) shrinks it and gives depthcharge less to load. It also frees the compressed copy at boot. | **needs a card test**: `scripts/make-kpart.sh` reads `/boot/vmlinuz-$release` and depthcharge has to boot the self-decompressing `Image`. Confirm what `make zinstall` installs first. |
| 6 | `CONFIG_LTO_CLANG_THIN` | `LTO_NONE` | **on** | The build is already `LLVM=1`, so thin LTO is a config line: better cross-file inlining (a few % in hot paths) and a smaller image, at ~10-20 % build time. | medium: a miscompile would show as a boot failure; one card test covers it |
| 7 | `CONFIG_ZPOOL` + `CONFIG_FRONTSWAP` | off | on **only if** zswap is wanted | Today `CONFIG_ZSWAP=y` does nothing. With these, zswap can sit in front of a disk swap (better ratio, writeback to eMMC). With zram-only there is no need. | low |
| 8 | `CONFIG_SCHED_CLASS_EXT` | off | optional | Enables sched_ext, i.e. trying a `scx_*` scheduler (e.g. `scx_lavd`, built for handhelds) without patching the kernel. `CONFIG_BPF_JIT=y` is already on. Needs the `scx` userland, which is not packaged. | medium: an external scheduler can misbehave; drop it if it does |
| 9 | Dead options | `ARM64_SVE`, `ARM64_MTE`, `XEN_DOM0`, `SCHED_SMT`, `SLUB_DEBUG` on | drop | None of this hardware has SVE or MTE, there is no Xen domain, and ARM has no SMT. Small size/build-time win only. **Keep `KALLSYMS_ALL`** — crash symbolization is worth its few MB. | low |
| 10 | Security/perf | `HARDENED_USERCOPY`, `FORTIFY_SOURCE`, `ARM64_SSBD` **already off** | leave | There is nothing left for `mitigations=off` to disable, so the usual "free 5 %" does not exist here. | — |

## 3. Image configuration (no kernel rebuild needed)

| # | What | Today | Propose | Why |
|---|---|---|---|---|
| 11a | swap policy | no sysctl drop-in; Debian `swappiness=60` | `vm.swappiness=150` (+ `vfs_cache_pressure=50`, `dirty_bytes=64 MiB`, `dirty_background_bytes=16 MiB`) | Swapping into compressed RAM is cheap, and 60 makes the kernel prefer dropping page cache instead. The dirty limits are the opposite of Deboma's 256/64 MB: on eMMC a large dirty window is where multi-second stalls come from when copying files. |
| 11b | one zram implementation | image: `zram-tools` (works, `OK swap active: /dev/zram0` in `reports/`) | keep `zram-tools`, or move to `systemd-zram-generator` | The host here has **both**, and `zramswap.service` fails on every boot (`Device or resource busy` → `mkswap: /dev/zram0 is mounted`) while systemd's generator does the work. Exactly one mechanism. `systemd-zram-generator` has one advantage: the swap becomes a real `dev-zram0.swap` unit, which is what `systemd-oomd` needs (11c). Its algorithm/size live in `/etc/systemd/zram-generator.conf`. |
| 11c | `systemd-oomd` | not installed | install + `ManagedOOMSwap=kill` | PSI is enabled, and on 4 GB the failure mode is "frozen machine", not "process killed". oomd kills a hog by pressure instead of letting the kernel thrash. Note the requirement in 11b: `ManagedOOMSwap=` matches swap *units*. |
| 11d | zram algorithm/size | `lz4`, 50 % (1.9 GB) | measure `lz4` vs `zstd` | `zstd` compresses better (more effective memory) at CPU cost, charged only on the swap path. Worth one `stress-ng --vm` run before changing. |
| 12 | THP | `[always]` | `transparent_hugepage=madvise` on the command line | `always` + zram on 4 GB wastes memory (a 2 MB fault vs. compressed 4 KB pages) and adds latency spikes; only a handful of workloads want it. Runtime-testable via `/sys/kernel/mm/transparent_hugepage/enabled`. |
| 13 | I/O scheduler | `mq-deadline` in the image, `kyber` on this host | ship a udev rule (or `elevator=kyber`) | kyber is built for flash: low latency under mixed load instead of deadline's global reordering. Both schedulers are compiled in, so this is a runtime choice. |
| 14 | IRQ affinity | every IRQ on CPU0 | `irqbalance`, or explicit `smp_affinity` rules for `mmc`, `rtw88_pci`, the i2c EC | eMMC + WiFi + input interrupts all land on the same core the scheduler uses; moving them to another little core (or the big cluster) is free latency. `irqbalance` is packaged (253 kB). |

## 4. Research items

* **EAS with a DT patch (item 2's other half).** The kernel has `ENERGY_MODEL`
  and the DT has capacities, but no `dynamic-power-coefficient`, so no energy
  model registers and `sched_energy_aware` never appears. Since
  `kernels/mediatek-mt81/patches/` is ours, adding the property to the CPU nodes
  would enable EAS — if the coefficients are right. Wrong values make it worse
  than off. This is the battery-life item, and it needs measurement, not
  guesswork.
* **Hibernation.** `CONFIG_HIBERNATION=y`, `/sys/power/state` = `freeze mem disk`,
  `/sys/power/disk` = `[shutdown]`. It needs a disk swap the image does not have
  (see the swap discussion) plus `resume=` on the kpart's command line and
  `RESUME=` in `initramfs-tools`, and a hardware test — a failed resume burns the
  kernel partition's `tries`. Suspend-to-RAM already works and resumes cleanly.
* **Boot without an initramfs.** `MMC_MTK`, `EXT4_FS`, `DRM_MEDIATEK` and
  `PANFROST` are all built in, so the kernel can mount `root=UUID=…` itself. The
  initramfs is 19 MB of the 29 MB kpart and its decompression is part of boot
  time. Worth one experiment on the card; initramfs-tools is still needed for
  hibernation and for `update-initramfs` after a kernel install.
* **sched_ext** (item 8) and **initramfs-free boot** are both "try it on the card
  and measure", not commitments.

## 5. How to verify each change

```sh
cat /sys/devices/system/cpu/cpufreq/policy*/scaling_governor
cat /sys/devices/system/cpu/cpufreq/policy*/stats/time_in_state   # frequency, per state
cat /sys/kernel/mm/lru_gen/enabled                                # MGLRU
cat /sys/kernel/mm/transparent_hugepage/enabled                   # THP
cat /sys/block/zram0/comp_algorithm /proc/swaps                   # zram
cat /proc/pressure/memory                                         # PSI, before/after load
systemd-analyze ; systemd-analyze blame                           # boot
awk '{for (i=2;i<=NF-2;i++) s[i]+=$i} END {for (i in s) printf "cpu%d %d\n", i-2, s[i]}' /proc/interrupts
sudo cgpt show /dev/mmcblk0 | grep -A3 pmOS_kernel                 # tries/successful
```

Load to measure against: `stress-ng --vm 2 --vm-bytes 1G --timeout 60s`
(624 kB package) and `fio` (2.4 MB) for the eMMC. Boot-chain changes are only
proven by booting the card and running `scripts/check-system.sh` on it.

Two rules for this work, both learned from this port: **one risky change per
card boot** (a kernel that will not boot tells you nothing if three things
changed), and **never leave the kernel partition's `tries` counter short** —
`scripts/install-kpart.sh` writes a fresh `-T 5`.

## 6. Suggested order

1. **The no-rebuild batch**: 1 (MGLRU), 2 (schedutil), 3 (HZ), 11a-11d (zram,
   swappiness, oomd), 12 (THP), 13 (kyber), 14 (IRQs), 9 (dead options). One
   kernel rebuild + one card boot; several of them are runtime-testable first.
2. **The size/compression batch**: 5 (kernel zstd), 6 (thin LTO), 4
   (PREEMPT_DYNAMIC), tested on the card together but changed one at a time in
   the config.
3. **EAS** (DT patch) — battery, needs coefficients and measurement.
4. **Hibernation / swap layout**, **sched_ext**, **initramfs-free boot** — only
   if wanted.

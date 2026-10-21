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

## 7. Compositor effects: what blur costs, and which renderer

Added when blur was turned on in `uis/sway/files/etc/sway/config.d/26-swayfx.conf`. It is not a
kernel knob, but it is the same kind of decision: the effect is either affordable on this GPU or it
is not, and that is a measurement, not an opinion.

### The renderer this device has

* **GLES on panfrost, and nothing else.** swayfx never calls `wlr_renderer_autocreate()` - it
  creates SceneFX's renderer itself (`fx_renderer_create()` in `sway/server.c`), and SceneFX only
  implements a GLES renderer. `WLR_RENDERER` therefore does not reach it at all; there is no
  Vulkan, pixman or "try the other one" path for the session.
* **Vulkan is not available on this GPU anyway.** `vulkaninfo` with Mesa 26.1.6 answers
  `Mali-G57 not supported (VK_ERROR_INCOMPATIBLE_DRIVER)` from `panvk_physical_device.c`: PanVK has
  no support for the Mali-G57. The two facts agree, which is why there is nothing to tune here
  beyond making sure the GLES driver is the hardware one (it is: EGL driver name `panfrost`,
  `GL_RENDERER` "Mali-G57 MC5 (Panfrost)", GLES 3.1).
* **The active mode is 1366x912 at 60.004 Hz** (`wayland-info`; every number below is quoted at that
  resolution), not the 1920x1200 the old comment in `26-swayfx.conf` claimed. A frame is 16.7 ms.
* **What the image actually runs, checked rather than assumed.** greetd starts `sway` with no
  `WLR_*`, `MESA_*` or `LIBGL_*` override anywhere in the tree, so every knob is the default; the
  rootfs has `libegl-mesa0`, `libgl1-mesa-dri` and `mesa-libgallium` 26.1.6 with `panfrost_dri.so`
  installed and **no `mesa-vulkan-drivers` at all** (nothing would load it on this GPU anyway).
  `check-system.sh` now fails when `eglinfo -B` does not name Panfrost, because a forced
  `LIBGL_ALWAYS_SOFTWARE` or a Mesa regression would otherwise drop the session onto llvmpipe
  silently, with every frame and the blur drawn on the CPU.
* **The Mesa tuning variables that exist do not help.** `PAN_MESA_DEBUG` accepts `dump`, `trace`,
  `sync`, `inorder`, `nocache` and `precompile` - instrumentation, validation, and `inorder`/
  `nocache` which *cost* performance. There is no "make it faster" switch; the way to make this
  GPU faster is to draw less, which is what the two blur paths below are about.
* **Colour management is compiled in and switched off.** SceneFX's meson found `lcms2` and built
  the colour path, but `fx_renderer.c` sets `features.output_color_transform = false`, so no
  per-frame conversion happens and there is nothing to disable at runtime.

### The "Mali-G57 PanVK driver" is not a shortcut

Mesa's own matrix (docs.mesa3d.org/drivers/panfrost.html) leaves the Vulkan column empty for
*Valhall (v9) G57/G68*; Mesa main implements `panvk_v6`, `v7` and `v10`-`v14`, and the G57 is v9.
`PAN_I_WANT_A_BROKEN_VULKAN_DRIVER=1` does not change that - tested on the device, `vulkaninfo`
then enumerates only llvmpipe. The binary driver that circulates for this GPU
(FristOneRR/PanVK, 2026-10) cannot fill the gap:

* it is not a source project - the repository is a README and a license, the releases are a
  prebuilt 17.6 MB `libvulkan_panfrost.so`;
* that `.so` is built with the Android NDK ("for Android 24", minApi 33) and needs `liblog.so`,
  `libhardware.so`, `libnativewindow.so` and `libsync.so` - bionic libraries Debian does not have
  and cannot substitute at the Vulkan-loader level;
* it drives **kbase** (`/dev/mali0`, `KBASE_IOCTL_*`, gralloc); this kernel binds the GPU to
  **panfrost** (`/dev/dri/renderD128`, DRM uAPI - `/dev/mali*` does not exist here), so the
  interface it was written against is absent;
* **the source forks add v9 for kbase, not for panfrost.** `apexspan-svg/mesa-panvk-mali-g57` is a
  full Mesa tree with G57/v9 support and `zenithblue-oss/panvk-kbase-android` a standalone build;
  both say outright that they target the Arm `kbase` JM interface instead of mainline
  `panfrost.ko`. `Vtgamer998/PanVK-v9-Driver` even ships a **glibc** build
  (`zig cc -target aarch64-linux-gnu.2.35`), but it is a Winlator ICD - `pan_kmod_kbase.c` +
  `kbase_winsys.c` on `/dev/mali0`, XCB WSI - and its README calls it "not a complete driver"
  (286 stubbed extensions) that "may cause GPU JOB_READ_FAULT". Using any of them means replacing
  their kernel interface with Mesa's `pan_kmod` DRM backend: a Mesa driver project, not a package
  drop;
* even a working Vulkan ICD would not touch this section: swayfx creates SceneFX's **GLES**
  renderer, so the compositor and its blur would keep using panfrost either way. Vulkan would only
  matter to Vulkan applications.

If upstream PanVK grows Valhall v9 support on `pan_kmod`'s DRM backend, the answer becomes
`apt install mesa-vulkan-drivers` on forky - no port and no private driver.

### The two paths SceneFX has

SceneFX (0.5) classifies each blurred node:

* **Cached ("optimized") blur.** A tiled window that cannot overlap another one (and this session
  has animations off: `animation_duration_ms` stays at its 0 default) samples a buffer that was
  blurred once, when the wallpaper or the output size changed. Per frame that is one bilinear tap
  per pixel.
* **Live blur.** A floating window, or one that can overlap another, is re-blurred from what is
  actually behind it: `blur_passes` downsample passes (5 taps) and the same number of upsample
  passes (9 taps), then the full-screen draw of the result.

`blur_xray` chooses between them: enabled, *every* surface uses the cached buffer - including a
floating window, which then shows the wallpaper through itself instead of the window behind it.
That is visible (verified with two screenshots of the same scene, one per setting), so the session
keeps it disabled and only floating windows pay for a live blur.

### Measured

[`bench-blur.c`](bench-blur.c) is the harness: an EGL/GLES2 context on `renderD128`, SceneFX's own
`blur1.frag`/`blur2.frag` verbatim, `glFinish()` on both sides, best of five runs of 300 iterations,
at 1366x912:

```sh
gcc -O2 -o bench-blur docs/bench-blur.c -lEGL -lGLESv2 -lm && ./bench-blur 1366 912
```

| Path | Work per frame | Time |
|---|---|---|
| Cached, full screen | 1 bilinear tap of the cached texture | **0.07-0.11 ms** |
| Live, radius 4 / 2 passes, full screen | 2 down + 2 up passes + the final draw | **2.1-2.2 ms** |
| Live plus the post-effects pass (noise/tint) | one more full-screen pass | **+0.6 ms** |
| Shipped defaults (radius 5, 3 passes, effects) | two more passes than the session's setting | **3.0-3.1 ms** |
| Building the cached buffer | the same chain, once per wallpaper/resize | ~2.7 ms, once |

The live path scales with the window's area: a 1200x800 floating window is 69 % of the screen, so
about 1.5 ms. An end-to-end check agrees that this is small: a client repainting a 2600x1700
window in a headless swayfx held 60 fps with blur off, with the cached path, with the live path and
with SceneFX's defaults (radius 5, 3 passes, noise). The blur is not what limits this GPU at this
panel's resolution; it was never the frame-rate cliff the port's earlier notes assumed.

The parameters in `26-swayfx.conf` follow from the table:

* `animation_duration_ms 0` is written out rather than left to its default. A non-zero value makes
  every moving window "could overlap" for the length of its animation, which sends it down the live
  path; with animations off the cached path is always the one a tiled window takes.
* `blur_passes 2` + `blur_radius 4` = a 32 px sample (`2^(passes+1) * radius`; the shipped default
  is 5 x 3 = 80 px). Passes are the price of a live blur, and the sample size is also how far a
  *live* blur's damage region is grown, so this is what a floating window re-renders around itself.
* `blur_noise 0` with brightness/contrast/saturation at 1.0 makes SceneFX skip the post-effects
  pass entirely; it costs one full-screen pass on a live blur and adds visible grain. For the
  cached path the pass is baked in when the buffer is built, so `blur_brightness 0.9` is free
  there - it is the line to add if a dimmer frost is wanted.
* Blur is drawn *behind* a surface, so it is only visible under one with alpha < 1. The three
  carriers are `~/.config/foot/foot.ini` (`alpha=0.92`), `~/.config/rofi/solarized.rasi` (0.88) and
  `~/.config/waybar/style.css` (0.78 for the bar, 0.86 for the cards). The bar is a layer surface
  that only ever has the wallpaper behind it, so `layer_effects "waybar"` uses the cached path; the
  launcher floats over windows, so `layer_effects "rofi"` uses the live one (and only repaints
  while open).
* Blur does not disable direct scanout by itself: `has_blur` is consulted only to render the cached
  buffer and to compensate damage, never in `scene_entry_try_direct_scanout`, so an opaque
  fullscreen buffer can still be scanned out with a blur node underneath it. What does cost scanout
  is the translucency that makes the blur visible - an `alpha < 1` surface is composited. That is
  the deliberate trade for the frosted look, and it is cheap in practice because the compositor is
  damage-driven: an idle translucent terminal produces no frames at all.

`vulkaninfo`, `eglinfo` and `wayland-info` give the facts that bound the options above. One detail the
harness had to get right, and a naive version gets wrong: every pass must read and write *different*
buffers. Sampling the target you are writing is a feedback loop, which the driver may legally throw
away - a first version of it measured a chain that cost less than one of its own passes.

# compositors/swayfx — the session compositor

Debian ships plain sway, and plain sway cannot round a window's corners, draw a shadow under it or
blur what is behind it. Deboma draws those effects around its Quickshell panels with Hyprland; the
equivalent here is **swayfx**, a fork of sway on top of **scenefx**, a wlroots extension that adds
the effects to the renderer. It is built from source by `scripts/build-swayfx.sh` and installed as
two Debian packages, because nothing in forky provides them.

| File | What it pins |
|---|---|
| `version` | swayfx release to build (the tag on `github.com/WillPower3309/swayfx`) |
| `scenefx-version` | scenefx release to build (the tag on `github.com/wlrfx/scenefx`) |
| `SHA256SUMS` | checksums of the two tarballs, verified on every build |

scenefx is tagged `0.5` by upstream while its meson project calls itself `0.5.0`; the pkg-config
dependency swayfx looks for is `scenefx-0.5` in either case. The versioned file holds the tag,
because that is what the build has to download.

## Why build it instead of taking it from the distribution

forky packages `sway` 1.12.0 against `wlroots` 0.20.2. swayfx 0.6 is a fork of exactly that sway
release and requires `wlroots >=0.20.0 <0.21.0`, so the wlroots already in the archive is the one it
was written against — nothing else has to be rebuilt or pinned. Only scenefx (the effects library)
and swayfx itself are missing, and both are small meson projects.

## What is built

Two packages, into `$COMPOSITOR_STAGE` (see `scripts/lib.sh`):

| Package | Contents |
|---|---|
| `libscenefx-0.5_0.5-1lazyos1_arm64.deb` | `libscenefx-0.5.so`: the effects renderer scenefx adds to wlroots |
| `swayfx_0.6-1lazyos1_arm64.deb` | `sway`, `swaymsg`, `swaynag`, `swaybar`, the `sway.desktop` session file, man pages and shell completions |

Both install into `/usr`, exactly where Debian's sway lives, so the greeter (`greetd` runs `sway`),
`/etc/sway/config`, the drop-ins and every keybind keep working. The swayfx package is
`Provides`/`Conflicts`/`Replaces: sway`, which is also the upgrade path for a system that already has
Debian's sway: `apt` swaps one for the other instead of refusing.

Two things about the contents are deliberate:

* **`/etc/sway/config` is dropped from the package.** Upstream installs its default configuration
  there. This port owns that path (`uis/sway/files/etc/sway/config`), and because
  `install-swayfx.sh` can run after the rootfs overlay, a packaged copy would silently replace the
  solarized, tested configuration with upstream's sample.
* **The `.pkgconfig` file and the headers are not shipped.** Only runtime code goes into the image;
  they exist in the staging tree for the duration of the build, which is what lets swayfx link
  against the freshly built library.

## Effects policy: cheap effects, and a blur that is only live where it must be

`uis/sway/files/etc/sway/config.d/26-swayfx.conf` enables rounded corners, shadows and blur.

SceneFX produces blur in two ways and the difference is the whole reason it can be on. A tiled
window that cannot overlap another one samples a buffer that was blurred once, when the wallpaper
changed: **one bilinear tap per pixel, 0.07-0.11 ms full screen** on this GPU. A floating window (or
one that can overlap) is re-blurred from what is behind it - two downsample and two upsample passes
plus the draw - **2.1-2.2 ms full screen at radius 4 / 2 passes**, scaling with its area, and only
while it is on screen. `blur_xray` decides: enabled, every surface samples the cached wallpaper
blur, which is cheapest but shows the wallpaper through a floating window instead of the window
behind (verified with screenshots); the session keeps it disabled, so the common case stays free.

The other knobs follow the measurement: `blur_passes 2` + `blur_radius 4` is a 32 px sample instead
of the shipped 80 px, `blur_noise 0` with neutral brightness/contrast/saturation skips SceneFX's
second full-screen pass, and the three surfaces that reveal the effect are translucent (foot 0.92,
rofi 0.88, waybar 0.78/0.86) because blur is drawn *behind* a surface. `layer_effects` gives the bar
the cached path (it only has the wallpaper behind it) and the launcher the live one (it floats over
windows). `docs/optimization.md` has the numbers, the method and why Vulkan is not an option here
(PanVK refuses the Mali-G57, and SceneFX creates its own GLES renderer regardless).

## Build and install

```sh
scripts/build-swayfx.sh          # download, verify, build, package -> $WORK_DIR/swayfx-stage
scripts/install-swayfx.sh        # into the built rootfs (chroots and runs apt)
scripts/install-swayfx.sh /      # into the running system
```

`scripts/build.sh` runs both in order, and so does an image rebuild: the rootfs does **not** install
Debian's `sway` (it is commented out of `uis/sway/packages.list`), so a rootfs built and then booted
without `install-swayfx.sh` has no compositor at all.

Dependencies are resolved inside the target chroot by `apt`, so the packages do not need a
hand-maintained `Depends:` — the shared libraries are mapped back to their Debian packages with
`dpkg -S` at build time.

## Verify

```sh
sway --version        # "swayfx version 0.6 (based on sway 1.12.0)"
dpkg -l swayfx libscenefx-0.5
scripts/check-system.sh     # on the booted image: compositor + `sway -C`
```

`sway -C` is the meaningful check: the drop-in uses swayfx-only directives, so plain sway rejects the
configuration outright instead of starting without the effects.

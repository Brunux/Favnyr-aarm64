# opencode

The coding agent this port is built with, shipped with the image.

`version` and `SHA256SUMS` pin the **arm64 tarball of the platform package**
from the npm registry - `@opencode/cli-linux-arm64`, which is nothing but
`package/bin/opencode`, the compiled program (236 MB) - because that is the one
upstream artifact that needs no npm at build time: no postinstall, no
`node_modules`, no registry metadata in the image.

`scripts/build-rootfs.sh` fetches it (cached in `$SRC_DIR`, verified against
`SHA256SUMS`, like the kernel and the compositor) and stages it inside the
chroot; `scripts/rootfs-setup.sh` extracts it to `~/.opencode/bin/opencode`,
owned by the user.

## Why that directory, and not /usr/local

`opencode upgrade` decides how to update itself from where its binary lives.
From `~/.opencode/bin` it prints *Using method: curl* and rewrites the binary in
place; from anywhere else it prints *update skipped: installation method not
found* / *Could not detect the installation method. Update with your package
manager*. A root-owned `/usr/local/bin/opencode` would therefore be a version
only a rebuilt image could move, in an agent that changes every week; the user's
own directory keeps `opencode upgrade` working with no root and no extra
script. It is also what makes the image ship the agent **signed out**: the
sign-ins live in the data directory (`~/.local/share/opencode`), which the build
never populates.

The npm route (`npm install -g @opencode/cli`) stays available and it is the
fallback the Agents menu offers when this directory is gone. Two things about it
are worth writing down: the V1 package (`opencode-ai`) is *not* what this image
ships, and npm 12 blocks install scripts by default, so OpenCode's postinstall
(the step that picks the platform binary) has to be allowed explicitly -
`--allow-scripts=@opencode/cli`.

## Bumping it

```sh
ver=2.1.4
curl -fL -o "opencode-cli-linux-arm64-$ver.tgz" \
	"https://registry.npmjs.org/@opencode/cli-linux-arm64/-/cli-linux-arm64-$ver.tgz"
sha256sum "opencode-cli-linux-arm64-$ver.tgz"    # -> SHA256SUMS
printf '%s\n' "$ver" > tools/opencode/version
```

The image is built from the pin, so nothing else changes; `check-system.sh`
prints the version the card actually has, which is the string that tells a card
apart from this file.

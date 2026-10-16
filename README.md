# Debian aarch64 for Google Hayato (MT8192)

A Debian port for the **Google Hayato** Chromebook (MediaTek MT8192, `google-asurada`), reusing
Nura's hardware support (kernel, device tree, patches, modules, boot flow) in place of Alpine.

- **Status:** scaffolding. No installable image yet.
- **Plan:** [`docs/plan.md`](docs/plan.md)
- **Recon of the target machine:** [`docs/recon.md`](docs/recon.md)
- **Agent instructions:** [`AGENTS.md`](AGENTS.md)

## Goal

1. Build a Debian (suite `forky`) arm64 image that boots from **microSD** via depthcharge.
2. Install it to the internal **eMMC** with an installer equivalent to Nura's
   `pmbootstrap install --disk=/dev/mmcblk0`.

## Reference

- Nura / postmarketOS: <https://nura.eco> · <https://gitlab.postmarketos.org/postmarketOS>
- Vendored from `pmaports` commit `0e458a2417d6f5ed61c7b70621d5632e30136091`.

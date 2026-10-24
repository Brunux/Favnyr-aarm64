# Favnyr

Upstream: <https://github.com/GlitchAwakened/Favnyr> · single binary 0.3.6 (MIT). This directory pins the source from the existing compiled binary, and `scripts/build-favnyr.sh` copies it into the image as a single binary.

## What it does, and why it is here

Favnyr is a portable file browser for Windows and Linux, designed to be a modern, lightweight replacement for desktop file managers. It features:

- **Tabbed interface** with split views (up to 32 views)
- **Workspace management** for saving and restoring window layouts
- **Favorites system** for quick access to frequently used folders
- **Custom commands** for running programs on selected files
- **Built-in archive support** (ZIP, 7Z, TAR)
- **Thumbnail support** for images, videos, and documents
- **Keyboard shortcuts** with customizable bindings
- **Multi-language support** (English, French, Spanish, German, Italian, Russian, Simplified Chinese, and Japanese)

This port replaces pcmanfm in the LazyOS session, providing a modern, Rust-based file browser that integrates with the existing session framework.

## Building

```sh
scripts/build-favnyr.sh                    # -> $WORK_DIR/favnyr-stage/favnyr_<v>_<arch>
scripts/install-favnyr.sh [ROOTFS]         # into the built rootfs (chroot-free copy)
scripts/install-favnyr.sh /                # onto the running system
```

`scripts/build.sh` runs both, in that order, around `build-rootfs.sh`. Favnyr is a single statically-linked Rust binary, so no additional dependencies are required at runtime.

## Configuration

Favnyr stores its configuration in `~/.config/favnyr/` with the following files:

- `config.toml` - Main application settings
- `workspaces.json` - Saved workspace layouts
- `favorites.json` - Favorite folder shortcuts
- `commands.json` - Custom command definitions

## Integration with LazyOS

- **Launcher**: Accessible via `Alt+Ctrl+F` (same as pcmanfm)
- **Window management**: Managed by sway (compositor)
- **Theme**: Compatible with LazyOS's solarized dark theme
- **Icons**: Uses the same `UI_I_*` codepoints as other menu items
- **Window opener**: Like pcmanfm, double-click in file lists opens files

## Benefits over pcmanfm

1. **Modern UI**: Rust-based with native look and feel
2. **Performance**: Statically linked with minimal dependencies
3. **Features**: Advanced workspace management and custom commands
4. **Integration**: Better compatibility with modern desktop environments
5. **Maintenance**: Actively maintained upstream project

## Configuration Files

The Favnyr configuration is generated during the rootfs setup and placed in:

- `~/.config/favnyr/config.toml` - Application settings
- `~/.local/share/lazyos/favnyr/` - Additional configuration files

## Test

On first run, Favnyr will generate default configuration files if they don't exist. The application will run without any initial setup required.

## Windows

See: https://github.com/GlitchAwakened/Favnyr
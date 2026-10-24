# Favnyr for LazyOS

## Overview

This directory contains the integration of Favnyr file browser for the LazyOS project. Favnyr replaces pcmanfm as the default file browser in the LazyOS session.

## What is Favnyr?

Favnyr is a modern file browser written in Rust that provides:

- **Tabbed Interface**: Up to 32 split views with individual tabs, history, and sorting
- **Workspace Management**: Save and restore window layouts for project-based workflows  
- **Favorites System**: Quick access to frequently used folders
- **Custom Commands**: Run programs on selected files with placeholders
- **Archive Support**: Built-in ZIP, 7Z, and TAR archive handling
- **Thumbnail Support**: Visual previews for images, videos, documents, and 3D files
- **Keyboard Shortcuts**: Customizable keyboard shortcuts
- **Multi-language Support**: English, French, Spanish, German, Italian, Russian, Simplified Chinese, and Japanese

## Integration

### Files Added

1. **`tools/favnyr/`** - Package structure for Favnyr:
   - `README.md` - Package documentation
   - `build.md` - Build instructions

2. **`scripts/build-favnyr.sh`** - Build script to copy the pre-compiled Favnyr binary
3. **`scripts/install-favnyr.sh`** - Install script to set up Favnyr in the image
4. **`uis/sway/packages.list`** - Updated to remove pcmanfm and add Favnyr
5. **`uis/sway/files/usr/local/bin/favnyr`** - Launcher script for Favnyr
6. **`uis/sway/files/usr/share/applications/favnyr.desktop`** - Desktop entry for application menu

### Build Pipeline

Favnyr is integrated into the build pipeline (`scripts/build.sh`):

```bash
./scripts/build.sh
```

The Favnyr binary is built from the source in `~/Dev/Favnyr-0.3.6/target/release/favnyr`
and installed into the rootfs during the build process.

## Usage

### Launch Favnyr

Favnyr is accessible via:
- **Keyboard shortcut**: `Alt+Ctrl+F` (same as pcmanfm)
- **Application menu**: From the launcher (Search key)
- **Terminal**: `favnyr` command

### Configuration

Favnyr configuration is auto-generated during rootfs setup:
- `~/.config/favnyr/config.toml` - Application settings
- Workspaces, favorites, and custom commands

### Features

#### Tabbed Browsing
- Open multiple files and folders in tabs
- Switch between tabs with keyboard shortcuts
- Save tab layouts as workspaces

#### Workspace Management
- Save complete window layouts
- Restore workspaces with one click
- Assign workspaces to keyboard shortcuts

#### Custom Commands
- Run programs on selected files
- Use placeholders like `{file}`, `{files}`, `{dir}`
- Commands can be pinned to right-click menus

#### Archive Support
- Compress files to ZIP, 7Z, or TAR
- Extract archives directly in Favnyr
- Built-in 7-Zip integration

#### Thumbnails
- Visual previews for images, videos, and documents
- 3D file thumbnails (requires 7-Zip)
- PDF thumbnails (requires poppler-utils)

## Migration from pcmanfm

### Key Differences

| Feature | pcmanfm | Favnyr |
|---------|---------|--------|
| **Interface** | GTK3 | Rust/Slint |
| **Tabs** | No | Up to 32 tabs |
| **Workspaces** | No | Yes |
| **Favorites** | Basic | Advanced |
| **Custom Commands** | No | Yes |
| **Archive Support** | No | ZIP, 7Z, TAR |
| **Thumbnails** | Basic | Advanced |

### Migration Steps

1. **Install**: Run `./scripts/build.sh` to rebuild the image with Favnyr
2. **First Run**: Favnyr will generate default configuration
3. **Learn Shortcuts**: Most keyboard shortcuts are preserved
4. **Customize**: Configure workspaces and favorites as needed

## Dependencies

Favnyr requires no additional system dependencies beyond what's already in LazyOS.
The binary is statically linked and can run on any Linux system.

## Testing

Check-system.sh validates Favnyr integration by:
- Verifying the binary is executable
- Checking configuration files exist
- Ensuring the launcher is properly configured

## Building

### Prerequisites

- Favnyr source code at `~/Dev/Favnyr-0.3.6/`
- Rust compiler (if rebuilding from source)

### Build Favnyr

```bash
# Ensure Favnyr is built
cd ~/Dev/Favnyr-0.3.6
cargo build --release
```

### Build LazyOS with Favnyr

```bash
cd /home/brunux/Dev/LazyOS
./scripts/build.sh
```

## Troubleshooting

### Favnyr Not Found

If Favnyr binary is missing:
```bash
# Check if Favnyr is built
ls -la ~/Dev/Favnyr-0.3.6/target/release/favnyr

# Build it if missing
cd ~/Dev/Favnyr-0.3.6
cargo build --release
```

### Favnyr Launcher Not Working

Check permissions:
```bash
# Ensure executable bit is set
chmod +x /usr/local/bin/favnyr
chmod +x ~/.config/bin/favnyr
```

### Configuration Issues

Delete and regenerate configuration:
```bash
# Remove broken configuration
rm -rf ~/.config/favnyr/config.toml

# Rebuild the image
./scripts/build.sh
```

## Credits

- **Favnyr**: https://github.com/GlitchAwakened/Favnyr
- **LazyOS**: https://github.com/Brunux/LazyOS
- **Integration**: OpenHands AI

## License

This integration is part of the LazyOS project, which is licensed under the MIT License.
Favnyr itself is licensed under the MIT License.
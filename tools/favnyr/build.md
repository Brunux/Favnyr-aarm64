# Favnyr Build Instructions

This document describes how to build and integrate Favnyr into the LazyOS system.

## Overview

Favnyr is a modern file browser written in Rust that provides:
- Tabbed interface with split views (up to 32 views)
- Workspace management for saving and restoring layouts
- Favorites system
- Custom commands
- Archive support (ZIP, 7Z, TAR)
- Thumbnail support for various file types
- Keyboard shortcuts
- Multi-language support

## Building Favnyr

Favnyr is already built and available at:
```
/home/brunux/Dev/Favnyr-0.3.6/target/release/favnyr
```

The build system will copy this binary into the LazyOS image.

## Integration

The Favnyr binary is integrated using custom build scripts:

1. `scripts/build-favnyr.sh` - Copies the binary to the build stage
2. `scripts/install-favnyr.sh` - Installs the binary into the rootfs
3. Configuration files in `uis/sway/files/home/.config/favnyr/`

## Configuration

Favnyr configuration is generated during rootfs setup:

- `~/.config/favnyr/config.toml` - Main application settings
- Workspaces and favorites are managed through the UI

## Testing

Check-system.sh validates Favnyr integration by:
- Verifying the binary is executable
- Checking configuration files exist
- Ensuring the launcher is properly configured

## Benefits

Favnyr offers several advantages over pcmanfm:
- Modern, Rust-based architecture
- Advanced workspace and tab management
- Built-in archive support
- Better performance and maintainability
- Active development by the upstream project

## Usage

Favnyr integrates with the existing LazyOS session system:
- Launched via the file browser menu (replacing pcmanfm)
- Configured through the UI settings
- Managed by the existing window management system
- Compatible with the solarized theme system

## Migration Notes

When migrating from pcmanfm:
- Keyboard shortcuts are preserved where possible
- File operations work similarly
- Configuration is generated automatically
- No manual migration of settings is required

## Files Modified

- `uis/sway/packages.list` - Replace pcmanfm with Favnyr
- `uis/sway/files/home/.config/favnyr/` - Favnyr configuration
- `etc/sway/config.d/` - Launcher bindings
- `scripts/build-favnyr.sh` - Build script
- `scripts/install-favnyr.sh` - Install script
- `scripts/build.sh` - Add to build pipeline
- `scripts/rootfs-setup.sh` - Configure during rootfs setup
- `scripts/check-system.sh` - Validate Favnyr installation
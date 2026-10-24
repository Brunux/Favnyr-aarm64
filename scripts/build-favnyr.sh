#!/bin/bash
# Build Favnyr from the existing source/binary

set -euo pipefail

. "$(dirname "$0")/lib.sh"

log() {
    printf '\n\033[1;34m[build-favnyr] ==> %s\033[0m\n' "$*"
}

log "Building Favnyr file browser"

# Create the stage directory
FAVNYR_STAGE="${WORK_DIR}/favnyr-stage"
mkdir -p "$FAVNYR_STAGE"

# Copy the existing Favnyr binary
FAVNYR_SOURCE="/home/brunux/Dev/Favnyr-0.3.6/target/release/favnyr"
FAVNYR_BINARY="$FAVNYR_STAGE/favnyr"

if [ ! -f "$FAVNYR_SOURCE" ]; then
    error "Favnyr source binary not found at $FAVNYR_SOURCE"
    exit 1
fi

log "Copying Favnyr binary to stage directory"
cp "$FAVNYR_SOURCE" "$FAVNYR_BINARY"
chmod 0755 "$FAVNYR_BINARY"

log "Favnyr binary copied and staged at $FAVNYR_BINARY"
log "Binary size: $(du -h "$FAVNYR_BINARY" | cut -f1)"

# Verify the binary
if ! "$FAVNYR_BINARY" --help >/dev/null 2>&1; then
    warn "Favnyr binary does not respond to --help, but was copied successfully"
fi

log "Favnyr build complete - binary ready for installation"
#!/bin/bash
set -e

# =============================================================================
# Docker entrypoint script for PLATEAU Tile Server with MapLibre support
# Starts Xvfb (X Virtual Frame Buffer) for headless OpenGL rendering
# =============================================================================

# Cleanup function for graceful shutdown
cleanup() {
    echo "Shutting down..."
    if [ -n "$XVFB_PID" ]; then
        kill $XVFB_PID 2>/dev/null || true
    fi
    exit 0
}

# Trap signals for graceful shutdown
trap cleanup SIGTERM SIGINT

# Start Xvfb (X Virtual Frame Buffer) for headless rendering
echo "Starting Xvfb on display ${DISPLAY}..."
Xvfb "${DISPLAY}" -screen 0 1280x1024x24 -ac +extension GLX +render -noreset -nolisten tcp &
XVFB_PID=$!

# Wait for Xvfb to be ready
sleep 1

# Verify Xvfb is running
if ! kill -0 $XVFB_PID 2>/dev/null; then
    echo "ERROR: Failed to start Xvfb"
    exit 1
fi

echo "Xvfb started successfully (PID: $XVFB_PID)"
echo "Using software renderer: LIBGL_ALWAYS_SOFTWARE=${LIBGL_ALWAYS_SOFTWARE}"
echo "Gallium driver: GALLIUM_DRIVER=${GALLIUM_DRIVER}"

# Execute the main application
echo "Starting tile server..."
exec "$@"

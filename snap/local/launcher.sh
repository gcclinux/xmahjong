#!/bin/bash
# LMahjong Snap launcher wrapper
# Checks required snap interfaces are connected before launching the game.

set -e

check_interface() {
    local interface="$1"
    local slot_path="$2"

    if [ -d "$slot_path" ] || [ -e "$slot_path" ]; then
        return 0
    fi
    return 1
}

# Check for display server connectivity (x11 or wayland)
display_available=false

# Check X11 - the x11 plug provides access to the X display socket
if [ -n "$DISPLAY" ]; then
    display_available=true
fi

# Check Wayland - the wayland plug provides access to the Wayland display socket
if [ -n "$WAYLAND_DISPLAY" ]; then
    display_available=true
fi

if [ "$display_available" = false ]; then
    echo "ERROR: LMahjong requires a display server connection." >&2
    echo "" >&2
    echo "No display server (X11 or Wayland) is available." >&2
    echo "Please ensure at least one of the following interfaces is connected:" >&2
    echo "" >&2
    echo "  sudo snap connect lmahjong:x11" >&2
    echo "  sudo snap connect lmahjong:wayland" >&2
    echo "" >&2
    echo "If you are running in a graphical session and still see this error," >&2
    echo "try reconnecting the interfaces with the commands above." >&2
    exit 1
fi

# Check OpenGL - needed for hardware-accelerated rendering
# The opengl plug provides access to /dev/dri and GPU libraries
if [ ! -d "/dev/dri" ] && [ ! -e "/dev/nvidia0" ]; then
    echo "WARNING: OpenGL interface may not be connected." >&2
    echo "The game may run with software rendering, which could be slow." >&2
    echo "" >&2
    echo "To connect the OpenGL interface:" >&2
    echo "  sudo snap connect lmahjong:opengl" >&2
    echo "" >&2
    # Don't exit - the game may still work with software rendering
fi

# Check audio-playback - not critical, game works without audio
# Just note if it's unavailable
if ! pulseaudio --check 2>/dev/null && ! pipewire --version 2>/dev/null; then
    echo "NOTE: Audio playback interface may not be connected." >&2
    echo "The game will run without sound." >&2
    echo "" >&2
    echo "To enable audio:" >&2
    echo "  sudo snap connect lmahjong:audio-playback" >&2
    echo "" >&2
    # Don't exit - audio is optional
fi

# Launch the game
exec "$SNAP/bin/lmahjong" "$@"

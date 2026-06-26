#!/usr/bin/env bash
#
# hover-select.sh -- Cross-platform launcher for tmux-hover-select
#
# Detects OS, display server, mouse device, and screen resolution.
# Builds the Rust binary if needed, then runs it.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/hover-select"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[hover-select]${NC} $*"; }
warn()  { echo -e "${YELLOW}[hover-select]${NC} $*" >&2; }
error() { echo -e "${RED}[hover-select]${NC} $*" >&2; exit 1; }

# --- Check if already running ---
if pgrep -f "hover-select" >/dev/null 2>&1; then
    warn "Already running (PID $(pgrep -f 'hover-select' | head -1)). Stopping old instance."
    pkill -f "hover-select" 2>/dev/null || true
    sleep 0.2
fi

# --- Detect OS ---
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        FreeBSD*) echo "freebsd" ;;
        *)       echo "unknown" ;;
    esac
}

OS=$(detect_os)
info "Detected OS: $OS"

# --- Detect display server ---
detect_display() {
    if [ -n "${WAYLAND_DISPLAY:-}" ]; then
        echo "wayland"
    elif [ -n "${DISPLAY:-}" ]; then
        echo "x11"
    elif [ "$OS" = "macos" ]; then
        echo "macos"
    else
        echo "none"
    fi
}

DISPLAY_SERVER=$(detect_display)
info "Detected display: $DISPLAY_SERVER"

# --- Find mouse device (Linux/FreeBSD only) ---
find_mouse_device() {
    local device=""
    for dev in /dev/input/event*; do
        [ -e "$dev" ] || continue
        local name
        name=$(cat "/sys/class/input/$(basename "$dev")/device/name" 2>/dev/null || echo "")
        case "${name,,}" in
            *mouse*)
                echo "$dev"
                return 0
                ;;
        esac
    done
    # Fallback: touchpad
    for dev in /dev/input/event*; do
        [ -e "$dev" ] || continue
        local name
        name=$(cat "/sys/class/input/$(basename "$dev")/device/name" 2>/dev/null || echo "")
        case "${name,,}" in
            *touchpad*|*trackpoint*)
                echo "$dev"
                return 0
                ;;
        esac
    done
    return 1
}

# --- Get screen resolution ---
get_resolution() {
    case "$DISPLAY_SERVER" in
        wayland)
            # Try wlr-randr (wlroots compositors)
            if command -v wlr-randr &>/dev/null; then
                wlr-randr 2>/dev/null | awk '/px,/ && /Hz/ { split($1, a, "x"); print a[1], a[2]; exit }'
                return
            fi
            # Try swaymsg
            if command -v swaymsg &>/dev/null; then
                swaymsg -t get_outputs 2>/dev/null | python3 -c "
import json, sys
d = json.load(sys.stdin)
if d: print(d[0]['rect']['width'], d[0]['rect']['height'])
" 2>/dev/null && return
            fi
            ;;
        x11)
            if command -v xrandr &>/dev/null; then
                xrandr 2>/dev/null | awk '/\*/ { split($1, a, "x"); print a[1], a[2]; exit }'
                return
            fi
            ;;
        macos)
            if command -v system_profiler &>/dev/null; then
                system_profiler SPDisplaysDataType 2>/dev/null | awk '/Resolution:/ { print $2, $4; exit }'
                return
            fi
            ;;
    esac
    echo "1920 1080"
}

# --- Check if Rust binary exists, build if not ---
build_binary() {
    if [ -f "$BINARY" ]; then
        info "Binary exists: $BINARY"
        return
    fi

    if ! command -v cargo &>/dev/null; then
        error "cargo not found. Install Rust: https://rustup.rs"
    fi

    info "Building hover-select..."
    cd "$SCRIPT_DIR"
    cargo build --release 2>&1 | tail -1
    info "Build complete: $BINARY"
}

# --- Main ---
MOUSE_DEVICE=""
if [ "$OS" = "linux" ] || [ "$OS" = "freebsd" ]; then
    MOUSE_DEVICE=$(find_mouse_device) || error "No mouse/touchpad device found in /dev/input/"
    info "Mouse device: $MOUSE_DEVICE"
fi

RESOLUTION=$(get_resolution)
SCREEN_W=$(echo "$RESOLUTION" | awk '{print $1}')
SCREEN_H=$(echo "$RESOLUTION" | awk '{print $2}')
info "Screen: ${SCREEN_W}x${SCREEN_H}"

build_binary

# --- Launch ---
info "Starting hover-select (Ctrl+C to stop)..."
exec "$BINARY" \
    --screen "${SCREEN_W}x${SCREEN_H}" \
    ${MOUSE_DEVICE:+--device "$MOUSE_DEVICE"} \
    --display "$DISPLAY_SERVER"

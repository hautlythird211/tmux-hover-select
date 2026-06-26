#!/usr/bin/env python3
"""
tmux-hover-select: Automatically select tmux pane on mouse hover.

Reads evdev mouse/touchpad events to track cursor position,
then selects the tmux pane under the cursor when it changes.
"""

import fcntl
import glob
import os
import signal
import struct
import subprocess
import sys
import time

# evdev event types and codes
EV_REL = 0x02
EV_SYN = 0x00
REL_X = 0x00
REL_Y = 0x01
SYN_REPORT = 0x00

# struct input_event size: timeval(16) + type(2) + code(2) + value(4) = 24 on 64-bit
INPUT_EVENT_SIZE = 24
INPUT_EVENT_FORMAT = "llHHI"  # sec, usec, type, code, value


def find_mouse_device():
    """Find the first mouse (not touchpad) evdev device."""
    candidates = glob.glob("/dev/input/event*")
    for dev_path in sorted(candidates):
        name_path = f"/sys/class/input/{os.path.basename(dev_path)}/device/name"
        try:
            with open(name_path) as f:
                name = f.read().strip().lower()
            # Prefer explicit mouse devices over touchpads
            if "mouse" in name:
                return dev_path
        except (OSError, IOError):
            continue
    # Fallback: use touchpad
    for dev_path in sorted(candidates):
        name_path = f"/sys/class/input/{os.path.basename(dev_path)}/device/name"
        try:
            with open(name_path) as f:
                name = f.read().strip().lower()
            if "touchpad" in name or "trackpoint" in name:
                return dev_path
        except (OSError, IOError):
            continue
    return None


def get_screen_size():
    """Get screen resolution from wlr-randr, swaymsg, or /sys/class/drm."""
    # Try wlr-randr (wlroots-based compositors)
    try:
        out = subprocess.check_output(["wlr-randr"], stderr=subprocess.DEVNULL, timeout=2).decode()
        for line in out.splitlines():
            if "px," in line and "Hz" in line:
                parts = line.strip().split()[0].split("x")
                return int(parts[0]), int(parts[1])
    except (subprocess.SubprocessError, FileNotFoundError, ValueError):
        pass

    # Try swaymsg
    try:
        out = subprocess.check_output(
            ["swaymsg", "-t", "get_outputs"], stderr=subprocess.DEVNULL, timeout=2
        ).decode()
        import json
        outputs = json.loads(out)
        if outputs:
            rect = outputs[0].get("rect", {})
            return rect.get("width", 1920), rect.get("height", 1080)
    except (subprocess.SubprocessError, FileNotFoundError, ValueError):
        pass

    # Try xrandr (XWayland / X11)
    try:
        out = subprocess.check_output(["xrandr"], stderr=subprocess.DEVNULL, timeout=2).decode()
        for line in out.splitlines():
            if "*" in line:
                parts = line.split()[0].split("x")
                return int(parts[0]), int(parts[1])
    except (subprocess.SubprocessError, FileNotFoundError, ValueError):
        pass

    return 1920, 1080


def get_pane_geometry(screen_w, screen_h):
    """Query tmux for pane positions and sizes. Returns dict of pane_id -> (left, top, width, height)."""
    try:
        out = subprocess.check_output(
            ["tmux", "list-panes", "-F",
             "#{pane_id} #{pane_left} #{pane_top} #{pane_width} #{pane_height}"],
            stderr=subprocess.DEVNULL, timeout=2,
        ).decode().strip()
    except (subprocess.SubprocessError, FileNotFoundError):
        return {}

    panes = {}
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 5:
            pane_id = parts[0]
            left, top, width, height = int(parts[1]), int(parts[2]), int(parts[3]), int(parts[4])
            panes[pane_id] = (left, top, width, height)
    return panes


def find_pane_at(x, y, panes):
    """Find which pane contains the given screen coordinates."""
    for pane_id, (left, top, width, height) in panes.items():
        if left <= x < left + width and top <= y < top + height:
            return pane_id
    return None


def get_current_pane():
    """Get the currently focused pane ID."""
    try:
        return subprocess.check_output(
            ["tmux", "display-message", "-p", "#{pane_id}"],
            stderr=subprocess.DEVNULL, timeout=2,
        ).decode().strip()
    except (subprocess.SubprocessError, FileNotFoundError):
        return None


def select_pane(pane_id):
    """Select the given tmux pane."""
    try:
        subprocess.run(
            ["tmux", "select-pane", "-t", pane_id],
            stderr=subprocess.DEVNULL, timeout=2,
        )
    except (subprocess.SubprocessError, FileNotFoundError):
        pass


def main():
    mouse_dev = find_mouse_device()
    if not mouse_dev:
        print("No mouse/touchpad device found.", file=sys.stderr)
        sys.exit(1)

    screen_w, screen_h = get_screen_size()
    print(f"hover-select: screen {screen_w}x{screen_h}, device {mouse_dev}", file=sys.stderr)

    # Open mouse device
    fd = os.open(mouse_dev, os.O_RDONLY | os.O_NONBLOCK)

    # Exclusive access to prevent other readers from consuming events
    try:
        fcntl.ioctl(fd, 0x40044501, 1)  # EVIOCGRAB
    except OSError:
        pass  # Non-critical, continue without exclusive grab

    # Track absolute position (start at center of screen)
    cur_x = screen_w // 2
    cur_y = screen_h // 2
    last_pane = None
    event_size = struct.calcsize(INPUT_EVENT_FORMAT)
    debounce = 0

    # Cache pane geometry, refresh periodically
    pane_cache = get_pane_geometry(screen_w, screen_h)
    pane_cache_time = time.monotonic()
    pane_refresh_interval = 0.5  # seconds

    def cleanup(*_):
        try:
            fcntl.ioctl(fd, 0x40044501, 0)  # EVIOCRELEASE
        except OSError:
            pass
        try:
            os.close(fd)
        except OSError:
            pass
        sys.exit(0)

    signal.signal(signal.SIGTERM, cleanup)
    signal.signal(signal.SIGINT, cleanup)

    while True:
        try:
            data = os.read(fd, event_size * 32)
        except BlockingIOError:
            time.sleep(0.005)
            continue
        except OSError:
            break

        # Refresh pane geometry periodically
        now = time.monotonic()
        if now - pane_cache_time > pane_refresh_interval:
            pane_cache = get_pane_geometry(screen_w, screen_h)
            pane_cache_time = now

        # Parse events from the read buffer
        i = 0
        while i + event_size <= len(data):
            _, _, ev_type, ev_code, ev_value = struct.unpack_from(INPUT_EVENT_FORMAT, data, i)
            i += event_size

            if ev_type == EV_REL:
                if ev_code == REL_X:
                    cur_x = max(0, min(screen_w - 1, cur_x + ev_value))
                elif ev_code == REL_Y:
                    cur_y = max(0, min(screen_h - 1, cur_y + ev_value))
            elif ev_type == EV_SYN and ev_code == SYN_REPORT:
                # Debounce: only act every 20ms
                if now - debounce < 0.02:
                    continue
                debounce = now

                if not pane_cache:
                    continue

                pane_id = find_pane_at(cur_x, cur_y, pane_cache)
                if pane_id and pane_id != last_pane:
                    current = get_current_pane()
                    if pane_id != current:
                        select_pane(pane_id)
                    last_pane = pane_id


if __name__ == "__main__":
    main()

# tmux-hover-select

Automatically select tmux panes by hovering the mouse over them.

No click required -- move your mouse over a pane and it gains focus instantly.

## How it works

- Reads evdev mouse/touchpad events to track cursor position
- Maps screen coordinates to tmux pane geometry
- Selects the pane under the cursor when it changes

## Requirements

- Python 3.6+
- Linux with evdev input support
- A Wayland compositor (Sway, Hyprland, etc.) or X11
- tmux 3.0+

## Usage

### Auto-start with tmux

Add to your `~/.tmux.conf`:

```tmux
run-shell "python3 /path/to/hover_select.py &"
```

### Manual start

```bash
python3 hover_select.py &
```

### Stop

```bash
pkill -f hover_select.py
```

## Finding your mouse device

The script auto-detects the mouse device from `/dev/input/event*`.
It prefers devices named "mouse" over "touchpad".

To check your devices:

```bash
for dev in /dev/input/event*; do
  echo "$dev: $(cat /sys/class/input/$(basename $dev)/device/name)"
done
```

## Limitations

- Only works in the current tmux session/window
- Requires read access to `/dev/input/event*` (typically the `input` group)
- Screen resolution is detected at startup; change `get_screen_size()` if incorrect
- Pane geometry is refreshed every 0.5 seconds

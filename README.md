# tmux-hover-select

Select tmux panes by hovering the mouse over them. No click required.

Cross-platform: Linux (evdev), macOS (CGEventTap), FreeBSD.

## Install

### Build from source

```bash
git clone https://github.com/hautlythird211/tmux-hover-select.git
cd tmux-hover-select
cargo build --release
```

Binary: `target/release/hover-select`

### Requirements

- Rust 1.70+ (for building)
- tmux 3.0+
- Linux: read access to `/dev/input/event*` (typically the `input` group)
- macOS: Accessibility permission (System Preferences > Privacy & Security)

## Usage

### Quick start

```bash
./hover-select.sh
```

### With tmux

Add to `~/.tmux.conf`:

```tmux
run-shell "/path/to/hover-select.sh &"
```

### Toggle with a key

```tmux
bind S run-shell "pgrep -f hover-select >/dev/null && pkill -f hover-select || /path/to/hover-select.sh &"
```

### Manual

```bash
./target/release/hover-select --screen 1920x1080 --display wayland
```

## How it works

1. **Shell wrapper** (`hover-select.sh`) detects OS, display server, mouse device, and screen resolution
2. **Rust binary** reads platform-native mouse events (evdev on Linux, CGEventTap on macOS)
3. Maps cursor coordinates to tmux pane geometry via `tmux list-panes`
4. Selects the pane under the cursor when it changes

## Architecture

```
hover-select.sh          # Cross-platform launcher (bash)
  |
  +-- hover-select       # Core engine (Rust)
       |
       +-- [Linux]   evdev crate -> /dev/input/event*
       +-- [macOS]   core-graphics crate -> CGEventTap
       +-- [FreeBSD] evdev crate -> /dev/input/event*
       |
       +-- tmux list-panes -> pane geometry
       +-- tmux select-pane -> switch focus
```

## Configuration

The shell wrapper auto-detects everything. Override manually:

```bash
./hover-select --screen 2560x1440 --device /dev/input/event9 --display x11
```

## License

MIT OR Apache-2.0

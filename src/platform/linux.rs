use evdev::{Device, EventSummary, RelativeAxisCode};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::common::{find_pane_at, get_current_pane, get_pane_geometry, select_pane};

pub struct PlatformMouse {
    device: Device,
    cur_x: i32,
    cur_y: i32,
    screen_w: i32,
    screen_h: i32,
    last_pane: Option<String>,
    pane_cache: std::collections::HashMap<String, super::common::Pane>,
    pane_cache_time: Instant,
}

impl PlatformMouse {
    pub fn new(device_path: &str, screen_w: i32, screen_h: i32) -> io::Result<Self> {
        let device = Device::open(device_path)
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Cannot open {}: {}", device_path, e)))?;

        eprintln!("hover-select: device={}, screen={}x{}", device_path, screen_w, screen_h);

        Ok(Self {
            device,
            cur_x: screen_w / 2,
            cur_y: screen_h / 2,
            screen_w,
            screen_h,
            last_pane: None,
            pane_cache: get_pane_geometry(),
            pane_cache_time: Instant::now(),
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut last_event_time = Instant::now();
        let debounce = Duration::from_millis(16);

        loop {
            let events = self.device.fetch_events().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to read events: {}", e))
            })?;

            let now = Instant::now();

            if now.duration_since(self.pane_cache_time) > Duration::from_millis(500) {
                self.pane_cache = get_pane_geometry();
                self.pane_cache_time = now;
            }

            for event in events {
                match event.destructure() {
                    EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, delta) => {
                        self.cur_x = (self.cur_x + delta as i32).clamp(0, self.screen_w - 1);
                    }
                    EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, delta) => {
                        self.cur_y = (self.cur_y + delta as i32).clamp(0, self.screen_h - 1);
                    }
                    EventSummary::Synchronization(_, _, _) => {
                        if now.duration_since(last_event_time) < debounce {
                            continue;
                        }
                        last_event_time = now;

                        if let Some(pane_id) = find_pane_at(self.cur_x, self.cur_y, &self.pane_cache) {
                            if self.last_pane.as_ref() != Some(&pane_id) {
                                if let Some(current) = get_current_pane() {
                                    if pane_id != current {
                                        select_pane(&pane_id);
                                    }
                                }
                                self.last_pane = Some(pane_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn find_mouse_device() -> io::Result<String> {
    let mut candidates: Vec<PathBuf> = fs::read_dir("/dev/input/")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Cannot read /dev/input: {}", e)))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name().map_or(false, |n| n.to_string_lossy().starts_with("event")))
        .collect();

    candidates.sort();

    // Prefer "mouse" devices
    for dev in &candidates {
        if name_contains(dev, "mouse") {
            return Ok(dev.to_string_lossy().into_owned());
        }
    }

    // Fallback: touchpad/trackpoint
    for dev in &candidates {
        let name = get_device_name(dev);
        if name.contains("touchpad") || name.contains("trackpoint") {
            return Ok(dev.to_string_lossy().into_owned());
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "No mouse/touchpad device found"))
}

fn get_device_name(path: &Path) -> String {
    let basename = path.file_name().unwrap_or_default().to_string_lossy();
    let name_path = format!("/sys/class/input/{}/device/name", basename);
    fs::read_to_string(&name_path)
        .unwrap_or_default()
        .trim()
        .to_lowercase()
}

fn name_contains(path: &Path, needle: &str) -> bool {
    get_device_name(path).contains(needle)
}

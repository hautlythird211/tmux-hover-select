use core_graphics::display::CGDisplay;
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::common::{find_pane_at, get_current_pane, get_pane_geometry, select_pane};

pub struct PlatformMouse {
    screen_w: i32,
    screen_h: i32,
}

impl PlatformMouse {
    pub fn new(_device: &str, screen_w: i32, screen_h: i32) -> Self {
        eprintln!("hover-select: screen={}x{} (macOS CGEventTap)", screen_w, screen_h);
        Self { screen_w, screen_h }
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let (tx, rx) = mpsc::channel::<(f64, f64)>();

        let event_mask = (1 << CGEventType::MouseMoved.as_u32())
            | (1 << CGEventType::LeftMouseDragged.as_u32())
            | (1 << CGEventType::RightMouseDragged.as_u32());

        let _tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            event_mask,
            |_proxy, _event_type, event| {
                let x = event.get_field(EventField::MouseEventPointX);
                let y = event.get_field(EventField::MouseEventPointY);
                let _ = tx.send((x, y));
                core_graphics::event::CallbackResult::Keep
            },
        )
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Failed to create CGEventTap. Grant Accessibility permission in System Preferences > Privacy & Security.",
            )
        })?;

        let mut pane_cache = get_pane_geometry();
        let mut pane_cache_time = Instant::now();
        let mut last_pane: Option<String> = None;

        loop {
            if let Ok((x, y)) = rx.recv_timeout(Duration::from_millis(16)) {
                let ix = x as i32;
                let iy = y as i32;

                if ix < 0 || ix >= self.screen_w || iy < 0 || iy >= self.screen_h {
                    continue;
                }

                if pane_cache_time.elapsed() > Duration::from_millis(500) {
                    pane_cache = get_pane_geometry();
                    pane_cache_time = Instant::now();
                }

                if let Some(pane_id) = find_pane_at(ix, iy, &pane_cache) {
                    if last_pane.as_ref() != Some(&pane_id) {
                        if let Some(current) = get_current_pane() {
                            if pane_id != current {
                                select_pane(&pane_id);
                            }
                        }
                        last_pane = Some(pane_id);
                    }
                }
            }
        }
    }
}

pub fn get_screen_resolution() -> (i32, i32) {
    let display = CGDisplay::main();
    (display.pixels_wide() as i32, display.pixels_high() as i32)
}

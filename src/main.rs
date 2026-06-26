use clap::Parser;
use std::process;

mod platform;

#[derive(Parser)]
#[command(name = "hover-select", about = "Select tmux panes on mouse hover")]
struct Cli {
    /// Screen resolution as WxH (e.g., 1920x1080)
    #[arg(long, value_name = "WxH", default_value = "1920x1080")]
    screen: String,

    /// Mouse/touchpad device path (Linux/FreeBSD only)
    #[arg(long, value_name = "PATH")]
    device: Option<String>,

    /// Display server: wayland, x11, macos
    #[arg(long, value_name = "TYPE")]
    display: String,
}

fn parse_resolution(s: &str) -> (i32, i32) {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        eprintln!("Invalid resolution format '{}'. Expected WxH (e.g., 1920x1080)", s);
        process::exit(1);
    }
    let w: i32 = parts[0].parse().unwrap_or_else(|_| {
        eprintln!("Invalid width: {}", parts[0]);
        process::exit(1);
    });
    let h: i32 = parts[1].parse().unwrap_or_else(|_| {
        eprintln!("Invalid height: {}", parts[1]);
        process::exit(1);
    });
    (w, h)
}

fn main() {
    let cli = Cli::parse();
    let (screen_w, screen_h) = parse_resolution(&cli.screen);

    #[cfg(target_os = "linux")]
    {
        let device = cli.device.unwrap_or_else(|| {
            match platform::linux::find_mouse_device() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        });

        let mut mouse = platform::PlatformMouse::new(&device, screen_w, screen_h)
            .unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                process::exit(1);
            });

        if let Err(e) = mouse.run() {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut mouse = platform::PlatformMouse::new(
            cli.device.as_deref().unwrap_or(""),
            screen_w,
            screen_h,
        );

        if let Err(e) = mouse.run() {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        eprintln!("Unsupported platform: {}", std::env::consts::OS);
        process::exit(1);
    }
}

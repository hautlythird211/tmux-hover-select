use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn get_pane_geometry() -> HashMap<String, Pane> {
    let output = Command::new("tmux")
        .args(["list-panes", "-F", "#{pane_id} #{pane_left} #{pane_top} #{pane_width} #{pane_height}"])
        .output()
        .expect("Failed to run tmux list-panes");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 5 {
            panes.insert(
                parts[0].to_string(),
                Pane {
                    id: parts[0].to_string(),
                    left: parts[1].parse().unwrap_or(0),
                    top: parts[2].parse().unwrap_or(0),
                    width: parts[3].parse().unwrap_or(0),
                    height: parts[4].parse().unwrap_or(0),
                },
            );
        }
    }

    panes
}

pub fn get_current_pane() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{pane_id}"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let id = stdout.trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}

pub fn select_pane(pane_id: &str) -> bool {
    Command::new("tmux")
        .args(["select-pane", "-t", pane_id])
        .output()
        .is_ok()
}

pub fn find_pane_at(x: i32, y: i32, panes: &HashMap<String, Pane>) -> Option<String> {
    for pane in panes.values() {
        if x >= pane.left && x < pane.left + pane.width && y >= pane.top && y < pane.top + pane.height {
            return Some(pane.id.clone());
        }
    }
    None
}

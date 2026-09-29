use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::config;
use crate::herdr::{self, call, field, list, DOCK_LABEL, DOCK_WIDTH};

const STALE_LOCK: Duration = Duration::from_secs(30);

struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn lock() -> Option<Lock> {
    let dir = std::env::var_os("HERDR_PLUGIN_STATE_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("ensure.lock");
    let stale = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age > STALE_LOCK);
    if stale {
        let _ = fs::remove_file(&path);
    }
    OpenOptions::new().write(true).create_new(true).open(&path).ok().map(|_| Lock(path))
}

/// Adds a dock to every tab in the session that doesn't have one.
pub fn run() -> i32 {
    config::write_default();
    let Some(_lock) = lock() else { return 0 };

    for workspace in list(call(&["workspace", "list"]), "workspaces") {
        let workspace_id = field(&workspace, "workspace_id");
        let mut tabs: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for pane in list(call(&["pane", "list", "--workspace", workspace_id]), "panes") {
            tabs.entry(field(&pane, "tab_id").to_string()).or_default().push(pane);
        }
        for panes in tabs.values() {
            if panes.iter().any(|p| field(p, "label") == DOCK_LABEL) {
                continue;
            }
            add_dock(field(&panes[0], "pane_id"));
        }
    }
    0
}

fn add_dock(any_pane: &str) {
    let Some((_, panes)) = herdr::layout(any_pane) else { return };
    let Some((target, _)) = panes
        .iter()
        .max_by_key(|(_, r)| (r.x + r.width, r.height, -r.y))
        .cloned()
    else {
        return;
    };

    let plugin = std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-dock".to_string());
    let opened = call(&[
        "plugin", "pane", "open", "--plugin", &plugin, "--entrypoint", "dock", "--placement", "split",
        "--target-pane", &target, "--direction", "right", "--no-focus",
    ]);
    let Some(dock) = opened
        .as_ref()
        .and_then(|r| r.get("plugin_pane"))
        .and_then(|p| p.get("pane"))
        .and_then(|p| p.get("pane_id"))
        .and_then(Value::as_str)
    else {
        return;
    };

    let Some((area, panes)) = herdr::layout(dock) else { return };
    let Some((_, rect)) = panes.iter().find(|(id, _)| id == dock) else { return };
    let excess = rect.width - DOCK_WIDTH as i64;
    if excess > 0 && area.width > 0 {
        let amount = format!("{:.4}", excess as f64 / area.width as f64);
        call(&["pane", "resize", "--pane", &target, "--direction", "right", "--amount", &amount]);
    }
}

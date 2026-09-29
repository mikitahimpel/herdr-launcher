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
            if panes.iter().any(is_dock) {
                continue;
            }
            add_dock(field(&panes[0], "pane_id"));
        }
    }
    0
}

/// Docks clear their label, so recognize them by running from the plugin root.
fn is_dock(pane: &Value) -> bool {
    let root = std::env::var("HERDR_PLUGIN_ROOT").unwrap_or_default();
    field(pane, "label") == DOCK_LABEL || (!root.is_empty() && field(pane, "cwd") == root)
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

    fit(dock, &target);
}

/// Shrinks the dock to `DOCK_WIDTH`; resize amounts are relative to the split, so converge in a few steps.
fn fit(dock: &str, neighbor: &str) {
    for _ in 0..4 {
        let Some((_, panes)) = herdr::layout(dock) else { return };
        let width = |id: &str| panes.iter().find(|(p, _)| p == id).map(|(_, r)| r.width);
        let (Some(dock_width), Some(neighbor_width)) = (width(dock), width(neighbor)) else { return };
        let excess = dock_width - DOCK_WIDTH as i64;
        if excess <= 0 {
            return;
        }
        let amount = format!("{:.4}", excess as f64 / (dock_width + neighbor_width) as f64);
        call(&["pane", "resize", "--pane", neighbor, "--direction", "right", "--amount", &amount]);
    }
}

/// Fits existing docks to their left neighbor.
pub fn fit_all() -> i32 {
    for workspace in list(call(&["workspace", "list"]), "workspaces") {
        for pane in list(call(&["pane", "list", "--workspace", field(&workspace, "workspace_id")]), "panes") {
            if !is_dock(&pane) {
                continue;
            }
            let dock = field(&pane, "pane_id");
            let Some((_, panes)) = herdr::layout(dock) else { continue };
            let Some((_, rect)) = panes.iter().find(|(p, _)| p == dock) else { continue };
            let left = panes
                .iter()
                .filter(|(p, r)| p != dock && r.x + r.width == rect.x && r.y <= rect.y && r.y + r.height >= rect.y + 1)
                .max_by_key(|(_, r)| r.height)
                .map(|(p, _)| p.clone());
            if let Some(left) = left {
                fit(dock, &left);
            }
        }
    }
    0
}

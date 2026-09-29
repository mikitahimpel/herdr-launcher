use std::process::Command;

use serde_json::Value;

pub const DOCK_LABEL: &str = "herdr-dock";
pub const DOCK_WIDTH: u16 = 10;

pub fn bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

pub fn call(args: &[&str]) -> Option<Value> {
    let output = Command::new(bin()).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice::<Value>(&output.stdout).ok()?.get("result").cloned()
}

pub fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn list(result: Option<Value>, key: &str) -> Vec<Value> {
    result.and_then(|r| r.get(key).cloned()).and_then(|v| v.as_array().cloned()).unwrap_or_default()
}

#[derive(Clone)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

pub fn layout(pane_id: &str) -> Option<(Rect, Vec<(String, Rect)>)> {
    let layout = call(&["pane", "layout", "--pane", pane_id])?.get("layout")?.clone();
    let rect = |v: &Value| Rect {
        x: v.get("x").and_then(Value::as_i64).unwrap_or(0),
        y: v.get("y").and_then(Value::as_i64).unwrap_or(0),
        width: v.get("width").and_then(Value::as_i64).unwrap_or(0),
        height: v.get("height").and_then(Value::as_i64).unwrap_or(0),
    };
    let area = rect(layout.get("area")?);
    let panes = layout
        .get("panes")?
        .as_array()?
        .iter()
        .filter_map(|p| Some((p.get("pane_id")?.as_str()?.to_string(), rect(p.get("rect")?))))
        .collect();
    Some((area, panes))
}

/// Panes sharing the tab of `pane_id`, excluding it.
pub fn tab_neighbors(pane_id: &str) -> Option<Vec<Value>> {
    let pane = call(&["pane", "get", pane_id])?.get("pane")?.clone();
    let tab = field(&pane, "tab_id").to_string();
    let workspace = field(&pane, "workspace_id").to_string();
    Some(
        list(call(&["pane", "list", "--workspace", &workspace]), "panes")
            .into_iter()
            .filter(|p| field(p, "tab_id") == tab && field(p, "pane_id") != pane_id)
            .collect(),
    )
}

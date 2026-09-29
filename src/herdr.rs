use std::process::Command;

use serde_json::Value;

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

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn context() -> Value {
    std::env::var("HERDR_PLUGIN_CONTEXT_JSON").ok().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or(Value::Null)
}

/// The agent pane the launcher was opened from: the focused pane if it runs an agent,
/// otherwise the first agent in the same tab.
pub fn agent_pane() -> Option<String> {
    if let Ok(pane) = std::env::var("LAUNCHER_AGENT_PANE") {
        return Some(pane).filter(|p| !p.is_empty());
    }
    let context = context();
    let focused = field(&context, "focused_pane_id");
    if focused.is_empty() {
        return None;
    }
    if !field(&context, "focused_pane_agent").is_empty() {
        return Some(focused.to_string());
    }
    let pane = call(&["pane", "get", focused])?.get("pane")?.clone();
    let tab = field(&pane, "tab_id");
    call(&["pane", "list", "--workspace", field(&pane, "workspace_id")])?
        .get("panes")?
        .as_array()?
        .iter()
        .find(|p| field(p, "tab_id") == tab && !field(p, "agent").is_empty())
        .map(|p| field(p, "pane_id").to_string())
}

/// Opens the launcher popup, remembering which agent it was opened from.
pub fn open_menu() -> i32 {
    let plugin = std::env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "launcher".to_string());
    let mut args = vec!["plugin".to_string(), "pane".into(), "open".into(), "--plugin".into(), plugin, "--entrypoint".into(), "menu".into()];
    if let Some(pane) = agent_pane() {
        args.extend(["--env".into(), format!("LAUNCHER_AGENT_PANE={pane}")]);
    }
    Command::new(bin()).args(&args).output().ok().and_then(|o| o.status.code()).unwrap_or(1)
}

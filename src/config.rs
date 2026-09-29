use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Button {
    pub icon: String,
    pub color: Option<String>,
    pub plugin: Option<String>,
    pub entrypoint: Option<String>,
    pub placement: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub command: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    buttons: Vec<Button>,
}

pub const DEFAULT: &str = r##"# Buttons shown in the dock, top to bottom.
#
# A button opens a plugin pane (plugin + entrypoint) or runs a command.
# placement: overlay (full tab, default here), popup, split, tab or zoomed.
# color: icon color as "#rrggbb".
# "{agent_pane}" in env values or command arguments becomes the agent pane in the dock's tab.

[[buttons]]
icon = "◆"
color = "#cba6f7"
plugin = "agent-tokens"
entrypoint = "dashboard"
placement = "overlay"
env = { AGENT_TOKENS_PANE = "{agent_pane}" }

[[buttons]]
icon = "◷"
color = "#89b4fa"
plugin = "agent-history"
entrypoint = "search"
placement = "overlay"
"##;

fn path() -> Option<PathBuf> {
    std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(|dir| PathBuf::from(dir).join("dock.toml"))
}

pub fn write_default() {
    if let Some(path) = path().filter(|p| !p.exists()) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, DEFAULT);
    }
}

pub fn load() -> Result<Vec<Button>, String> {
    let raw = path().and_then(|p| fs::read_to_string(p).ok()).unwrap_or_else(|| DEFAULT.to_string());
    toml::from_str::<Config>(&raw).map(|c| c.buttons).map_err(|e| e.message().to_string())
}

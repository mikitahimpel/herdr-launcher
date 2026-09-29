use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Panel {
    pub icon: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
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
    panels: Vec<Panel>,
}

pub const DEFAULT: &str = r##"# Panels listed in the launcher, top to bottom.
#
# A panel opens a plugin pane (plugin + entrypoint) or runs a command.
# placement: overlay (whole tab, the default), popup, split, tab or zoomed.
# color: icon color as "#rrggbb".
# "{agent_pane}" in env values or command arguments becomes the agent pane the launcher was opened from.

[[panels]]
icon = "◆"
name = "Token usage"
description = "context, output, prompts and subagents"
color = "#cba6f7"
plugin = "agent-tokens"
entrypoint = "dashboard"
env = { AGENT_TOKENS_PANE = "{agent_pane}" }

[[panels]]
icon = "◷"
name = "Agent history"
description = "search and resume Claude Code and Codex sessions"
color = "#89b4fa"
plugin = "agent-history"
entrypoint = "search"
"##;

fn path() -> Option<PathBuf> {
    std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(|dir| PathBuf::from(dir).join("launcher.toml"))
}

pub fn load() -> Result<Vec<Panel>, String> {
    let raw = match path() {
        Some(path) if path.exists() => fs::read_to_string(path).map_err(|e| e.to_string())?,
        Some(path) => {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(path, DEFAULT);
            DEFAULT.to_string()
        }
        None => DEFAULT.to_string(),
    };
    toml::from_str::<Config>(&raw).map(|c| c.panels).map_err(|e| e.message().to_string())
}

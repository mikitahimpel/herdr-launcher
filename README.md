# herdr-launcher

A [Herdr](https://herdr.dev) plugin: one keybinding opens a small floating menu of your plugin panels; pick one and it opens over the whole tab.

```
╭ ◆ Panels ──────────────────────────────────────────╮
│                                                    │
│ ▌ ◆  Token usage                                1  │
│ ▌    context, output, prompts and subagents        │
│                                                    │
│   ◷  Agent history                              2  │
│      search and resume Claude Code and Codex …     │
│                                                    │
│ ↑↓ select · enter open · 1-9 · esc close           │
╰────────────────────────────────────────────────────╯
```

Pick with the mouse, arrows and Enter, or the number keys. `esc` closes the menu.

## Install

Requires Herdr 0.9.0+ and a Rust toolchain (`cargo`); the install builds the binary.

```sh
herdr plugin install mikitahimpel/herdr-launcher
```

Bind the menu in `~/.config/herdr/config.toml`, then run `herdr server reload-config`:

```toml
[[keys.command]]
key = "prefix+space"
type = "plugin_action"
command = "launcher.menu"
description = "panels"
```

## Panels

Panels live in `launcher.toml` in the plugin's config directory (`herdr plugin config-dir launcher`), created on first use:

```toml
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
```

A panel opens a plugin pane (`plugin`, `entrypoint`, optional `placement` — `overlay` by default — `width`, `height`, `env`) or runs `command = ["argv", ...]`. `{agent_pane}` becomes the agent pane the launcher was opened from: the focused pane if it runs an agent, otherwise the first agent in its tab.

Pairs with [herdr-agent-tokens](https://github.com/mikitahimpel/herdr-agent-tokens).

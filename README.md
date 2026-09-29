# herdr-dock

A [Herdr](https://herdr.dev) plugin that puts a narrow icon dock on the right edge of every tab. Click an icon to open a plugin panel over the whole tab — no keybindings to remember.

```
┌ your panes ───────────────────────────┐┌───────────┐
│                                       ││ ┌───────┐ │
│                                       ││ │   ◆   │ │
│                                       ││ └───────┘ │
│                                       ││ ┌───────┐ │
│                                       ││ │   ◷   │ │
│                                       ││ └───────┘ │
└───────────────────────────────────────┘└───────────┘
```

Every icon sits on a tile of the same size, so glyphs from different fonts still line up. The dock is as narrow as Herdr allows: splits are limited to 10% of their parent, so it takes 10% of the tab width.

- The dock adds itself to every tab: new tabs and workspaces get one automatically, and existing tabs get one when the session starts.
- In a tab with several panes, it's split off the rightmost pane.
- When the dock is the only pane left in a tab, it closes itself.
- Close it in a tab if you don't want it there; it comes back only when the session restarts.

## Install

Requires Herdr 0.9.0+ and a Rust toolchain (`cargo`); the install builds the binary.

```sh
herdr plugin install mikitahimpel/herdr-dock
herdr plugin action invoke herdr-dock.ensure   # add the dock to the tabs you already have
```

## Buttons

Buttons live in `dock.toml` in the plugin's config directory (`herdr plugin config-dir herdr-dock`). It's created on first run with:

```toml
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
```

A button either opens a plugin pane (`plugin`, `entrypoint`, and optional `placement`, `width`, `height`, `env`, `color`) or runs `command = ["argv", ...]`. `{agent_pane}` becomes the agent pane in the dock's tab. The dock rereads the file every few seconds and shows a red `!` if it can't parse it.

Pairs with [herdr-agent-tokens](https://github.com/mikitahimpel/herdr-agent-tokens).

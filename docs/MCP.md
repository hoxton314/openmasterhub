# AI agents over MCP

`masterhubd mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) server, so an
AI agent such as Claude can read your hub and change its config: create and edit profiles,
pages, keys, dial, knob, fader and roller actions and setups, switch setups and profiles, and
pin a profile. It is **off by default**.

## Turn it on

In the app: **Settings › AI agents › Run MCP server**. That saves `mcp = true` in the `[app]`
table of your config, which you can also write by hand:

```toml
[app]
mcp = true
```

While it is off, `masterhubd mcp` exits at once with an error the agent shows you, and a
server that is already running answers every tool call with that error. Turning it back off
takes effect on the next call.

## Connect an agent

The agent starts `masterhubd mcp` itself and talks to it over stdin and stdout. The service
(`masterhubd`) has to be running; the MCP server is a thin client of its local socket.

**Claude Code**, once, for every project:

```sh
claude mcp add --scope user openmasterhub -- masterhubd mcp
```

**Claude Desktop** (Settings › Developer › Edit Config, `claude_desktop_config.json`) and other
clients with an `mcpServers` file:

```json
{
  "mcpServers": {
    "openmasterhub": { "command": "masterhubd", "args": ["mcp"] }
  }
}
```

On Windows the installer does not put `masterhubd.exe` on `PATH`; use its full path in the
folder the app is installed in. Settings › AI agents shows both snippets with the right path
for your install, and a copy button.

Then ask, for example: *"Make an OBS profile: scene keys on the first row, mute the mic on the
dial press, and use it in my Stream setup."*

## Tools

Read:

| Tool | What it returns |
|---|---|
| `get_hub` | Connected modules (kind, socket, placement, firmware, needs replug; no serial numbers), live state (setup, profile, page, focused app, why, pin), capabilities, version |
| `list_profiles` | Every profile: apps, base, colour, pages and folders with their set keys, other controls, the setups that use it |
| `get_profile` | One profile exactly as stored |
| `get_controls` | Every control of a profile on one page after pinned keys and base profiles, with where each comes from |
| `list_setups` | The setups (or the implicit "Default") and the active one |
| `get_config` | The whole config as TOML |
| `validate_config` | `{valid, error}` for a TOML text, without saving it |
| `list_open_apps` | Window classes of open windows, for a profile's `apps` |

Edit (each change is saved at once):

| Tool | What it does |
|---|---|
| `add_profile` | New profile with an empty page "Main", or a copy of another; optionally add it to setups |
| `update_profile` | Change apps, base, colour, description, game flag |
| `delete_profile` | Remove a profile and its name from setup lists and pinned-key exceptions |
| `add_page`, `delete_page` | Keypad pages and folders of a profile |
| `set_key` | A key's face (label, image, colours, widget) and press/release actions; `null` clears it |
| `set_dial`, `set_knob`, `set_fader`, `set_roller` | The other controls of a profile |
| `set_pinned_key` | A key on every page of every profile, with exceptions |
| `put_setup`, `delete_setup` | Add, replace or (with `merge`) update a setup; remove one |
| `put_config` | Replace the whole config (the structured tools are preferred) |

`set_*` and `put_setup` replace the piece they name; with `merge: true` they lay the given
fields over the current ones and a `null` field removes it. Controls and actions use the same
shapes as the config file ([`examples/config.toml`](../examples/config.toml)), as JSON, e.g.
`{"label": "Mute", "press": {"keys": "ctrl+shift+m"}}`; the tool schemas list every field.

Live (change what the hub shows, not the config): `switch_setup`, `switch_profile`, `show_page`,
`pin_profile`, `release_pin`.

## Safety

- **Same checks as the app.** Every edit runs the full check the app runs before saving
  (unknown key names, missing pages, folders, profiles, setups and audio groups, base cycles,
  holds over 3 s, …), and the service checks again before it writes the file. An agent cannot
  save a config the service would refuse or fail to start with; a refused edit changes nothing
  and the agent gets the reason.
- **App settings stay yours.** `put_config` keeps your `[app]` table, so an agent cannot turn
  the MCP server (or anything else in the app's settings) on or off.
- **No hardware mode.** No tool reaches anything that switches modules to hardware mode, and
  none flashes, claims or redraws modules directly.
- **No serial numbers.** `get_hub` leaves the modules' serial numbers out.
- **Shell commands.** An agent can bind `cmd` actions, which run a shell command when *you*
  press the key, turn the knob and so on. No tool runs an action by itself. Review what an agent
  set up, as you would a script it wrote: `get_profile` and the app's editor show every binding.
- Saving rewrites the file the same way the app does: comments in a hand-edited config are not
  kept. Back it up first (Settings › Config file › Back up now) if they matter.

## How it works

`masterhubd mcp` speaks JSON-RPC 2.0, one message per line (MCP's stdio transport, protocol
revisions 2024-11-05 to 2025-11-25). For every tool call it checks `[app] mcp`, then makes
requests to the running service over its local socket (`get_config`, `set_config`,
`set_setup`, `pin`, …; see `masterhub_core::ipc`), the same API the app uses. Edits load the
config, change one piece, check it, send it with `set_config`, and wait until the service has
applied it, so the next call sees the change. The process holds no state and opens no device,
so several agents can use it at once.

To try it without touching your own config, run a second service with its own socket, state
and config, and no modules:

```sh
mkdir -p /tmp/omh/run /tmp/omh/state && chmod 700 /tmp/omh/run
printf '[app]\nmcp = true\n' > /tmp/omh/config.toml
XDG_RUNTIME_DIR=/tmp/omh/run XDG_STATE_HOME=/tmp/omh/state masterhubd --no-modules --config /tmp/omh/config.toml &
XDG_RUNTIME_DIR=/tmp/omh/run masterhubd mcp   # then type JSON-RPC lines, or point an MCP inspector at it
```

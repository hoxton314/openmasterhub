# Claude Code status on the Base LED and keys

`masterhubd` can show what your [Claude Code](https://claude.com/claude-code) sessions are doing
on the Base LED, and give every open session its own key (see [Session keys](#session-keys)).
On the Base, with several sessions open, the most urgent one wins:

| Status | Default | When |
| --- | --- | --- |
| attention | `#ff4000`, **flashing** | a session waits for you: permission prompt, question, plan approval |
| ready | `#00ff40` | a session finished its turn and waits for a new prompt |
| working | `#0060ff` | every session is busy |
| (none) | your `[leds] base` colour | no Claude Code session is open |

Works on Linux and Windows. The service forgets a session whose `claude` process has exited,
even if it never sent `SessionEnd`.

## Set up

1. Make sure `masterhubd` is running (Linux: `systemctl --user status masterhubd`, Windows: the
   installer starts it at login) and, on Linux, is on your `PATH`.
2. Add the hooks to `~/.claude/settings.json` (merge with any `hooks` you already have):

```json
{
  "hooks": {
    "SessionStart":      [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "UserPromptSubmit":  [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "PreToolUse":        [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "PostToolUse":       [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "Notification":      [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "Elicitation":       [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "ElicitationResult": [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "Stop":              [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "StopFailure":       [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }],
    "SessionEnd":        [{ "hooks": [{ "type": "command", "command": "masterhubd claude-hook", "timeout": 5 }] }]
  }
}
```

   **Windows:** the installer does not put `masterhubd.exe` on `PATH`, so give its full path.
   Claude Code runs hooks through Git Bash, so in every hook above use:

   ```json
   "command": "\"$LOCALAPPDATA/OpenMasterHUB/masterhubd.exe\" claude-hook"
   ```

   (`%LOCALAPPDATA%\OpenMasterHUB` is where the installer puts the app and the service.)

3. Start a new Claude Code session.

The hook prints nothing and always exits successfully, so it never blocks Claude Code, even
when the service is not running.

## Session keys

A `claude_session` widget shows one open session: its name (the title in the Claude app's
sidebar, or set with `/rename`; otherwise the folder name) under a small terracotta mascot. The
key's background is tinted with the session's status colour and the mascot's pose follows it:

| Status | Key |
| --- | --- |
| working | `working` background, mascot typing (animated) |
| ready | `ready` background, mascot standing |
| attention | `attention` background flashing with the Base, mascot waving |

Sessions fill the slots oldest first, so keys don't move around as sessions come and go; an
empty slot shows the key as configured (blank, unless you gave it a label). The `session` action
brings a session up:

- **Claude desktop app** sessions open in the app (`claude://code/continue?session=local_…`)
  and its window comes to the front.
- **Terminal** sessions focus the window of the terminal `claude` runs in (Hyprland on Linux,
  the foreground window on Windows). With several tabs, or tmux, in one terminal window, that
  window comes up but not the tab.

```toml
[[profile.page.key]]
index = 0
widget = { kind = "claude_session", slot = 0 }
press = { session = 0 }
```

`examples/config.toml` has a ready-made **Claude** profile: thirteen session keys, *New*
(`claude://code/new`) and *Back*, which returns to the profile shown before. It
activates while the Claude app is focused, and a pinned *Claude* key on every page of every
profile switches to it from anywhere:

```toml
[[pinned]]
index = 9
label = "Claude"
press = { profile = "Claude" }
except = ["Claude"]   # the Claude profile shows its own key there
``` In the app, pick the **Claude**
widget and the **Claude** action and choose the same session number.

Session keys use the same hooks as the Base LED (set up above), and the names come from the
files Claude Code keeps in `~/.claude/sessions/` (`$CLAUDE_CONFIG_DIR/sessions/`).

## Colours

The same colours are used on the Base and for the background of session keys.

```toml
[status]
attention = "#ff4000"
ready = "#00ff40"
working = "#0060ff"
# enabled = false   # keep the normal Base colour (session keys still work)
```

## How it works

Each hook runs `masterhubd claude-hook`, which reads the event JSON from stdin, maps it to a
status and sends `set_status` over the service's local socket (`docs/app-architecture.md`):

| Hook event | Status |
| --- | --- |
| `SessionStart`, `Stop`, `StopFailure` (turn ended on an API error) | ready |
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `ElicitationResult` | working |
| `PreToolUse` of `AskUserQuestion` or `ExitPlanMode`, `PermissionRequest`, `Elicitation` (MCP server asks for input) | attention |
| `Notification`: `permission_prompt`, `elicitation_dialog` | attention |
| `Notification`: `idle_prompt` | ready |
| `SessionEnd` | cleared |

Tool events from subagents are ignored, so a background agent does not stop the flash while
the main session waits for you.

Other tools can use the same request, one JSON line on the socket:

```json
{"id":1,"method":"set_status","params":{"source":"my-tool","status":"attention"}}
{"id":2,"method":"set_status","params":{"source":"my-tool","status":null}}
```

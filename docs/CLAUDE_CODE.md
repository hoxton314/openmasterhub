# Claude Code status on the Base LED

`masterhubd` can show what your [Claude Code](https://claude.com/claude-code) sessions are doing
on the Base LED. With several sessions open, the most urgent one wins:

| Status | Default | When |
| --- | --- | --- |
| attention | `#ff4000`, **flashing** | a session waits for you: permission prompt, question, plan approval |
| ready | `#00ff40` | a session finished its turn and waits for a new prompt |
| working | `#0060ff` | every session is busy |
| (none) | your `[leds] base` colour | no Claude Code session is open |

Linux only for now: the service forgets a session whose `claude` process has exited, even if
it never sent `SessionEnd`.

## Set up

1. Make sure `masterhubd` is running (`systemctl --user status masterhubd`) and is on your `PATH`.
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

3. Start a new Claude Code session.

The hook prints nothing and always exits successfully, so it never blocks Claude Code, even
when the service is not running.

## Colours

```toml
[status]
attention = "#ff4000"
ready = "#00ff40"
working = "#0060ff"
# enabled = false   # ignore status reports and keep the normal Base colour
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

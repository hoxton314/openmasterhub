//! `masterhubd claude-hook`: a Claude Code hook that reports the session's status to the
//! running service, which shows it on the Base LED (see `docs/CLAUDE_CODE.md`).
//!
//! Claude Code passes the hook event as JSON on stdin. The hook prints nothing (stdout of some
//! hooks becomes model context) and always succeeds, so it never gets in Claude's way.

use std::io::Read;

use masterhub_core::ipc::{Request, Status, call_once};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HookInput {
    session_id: String,
    hook_event_name: String,
    tool_name: Option<String>,
    notification_type: Option<String>,
    /// Set for events from a subagent.
    agent_id: Option<String>,
}

/// What the session's status becomes after this event: `Some(None)` clears it, `None` keeps it.
fn status_for(input: &HookInput) -> Option<Option<Status>> {
    let s = match input.hook_event_name.as_str() {
        // StopFailure: the turn ended on an API error, without a Stop.
        "SessionStart" | "Stop" | "StopFailure" => Status::Ready,
        "UserPromptSubmit" | "ElicitationResult" => Status::Working,
        "PermissionRequest" | "Elicitation" => Status::Attention,
        "PreToolUse" if matches!(input.tool_name.as_deref(), Some("AskUserQuestion" | "ExitPlanMode")) => {
            Status::Attention
        }
        // Subagents keep running while the main session waits on a prompt; don't let their
        // tool calls hide it.
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" if input.agent_id.is_none() => Status::Working,
        "Notification" => match input.notification_type.as_deref() {
            Some("permission_prompt" | "elicitation_dialog") => Status::Attention,
            Some("idle_prompt") => Status::Ready,
            _ => return None,
        },
        "SessionEnd" => return Some(None),
        _ => return None,
    };
    Some(Some(s))
}

pub fn hook() {
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        return;
    }
    let Ok(input) = serde_json::from_str::<HookInput>(&text) else { return };
    let Some(status) = status_for(&input) else { return };
    let request = Request::SetStatus { source: format!("claude:{}", input.session_id), status, pid: claude_pid() };
    // The service may not be running; that's fine.
    let _ = call_once(request);
}

/// The Claude Code process: the first ancestor that is not a shell.
#[cfg(target_os = "linux")]
fn claude_pid() -> Option<u32> {
    const SHELLS: [&str; 6] = ["sh", "bash", "dash", "zsh", "fish", "ksh"];
    let mut pid = std::os::unix::process::parent_id();
    for _ in 0..8 {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // "pid (comm) state ppid ...", where comm may itself contain parentheses.
        let (head, tail) = stat.rsplit_once(')')?;
        let comm = head.split_once('(')?.1;
        if !SHELLS.contains(&comm) {
            return Some(pid);
        }
        pid = tail.split_whitespace().nth(1)?.parse().ok()?;
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn claude_pid() -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(json: &str) -> Option<Option<Status>> {
        status_for(&serde_json::from_str(json).unwrap())
    }

    #[test]
    fn events_map_to_statuses() {
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"UserPromptSubmit","prompt":"hi"}"#), Some(Some(Status::Working)));
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"Stop"}"#), Some(Some(Status::Ready)));
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"StopFailure"}"#), Some(Some(Status::Ready)));
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"Elicitation"}"#), Some(Some(Status::Attention)));
        assert_eq!(
            status(r#"{"session_id":"a","hook_event_name":"Notification","notification_type":"permission_prompt"}"#),
            Some(Some(Status::Attention))
        );
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"Notification","notification_type":"auth_success"}"#), None);
        assert_eq!(
            status(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"AskUserQuestion"}"#),
            Some(Some(Status::Attention))
        );
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"PostToolUse","tool_name":"Bash"}"#), Some(Some(Status::Working)));
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"PostToolUse","tool_name":"Bash","agent_id":"x"}"#), None);
        assert_eq!(status(r#"{"session_id":"a","hook_event_name":"SessionEnd","reason":"exit"}"#), Some(None));
    }
}

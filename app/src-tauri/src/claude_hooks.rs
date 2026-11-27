//! Claude Code hooks for the Integrations page: whether `~/.claude/settings.json` reports to
//! masterhubd (`docs/CLAUDE_CODE.md`), and adding or removing exactly those hooks, leaving the
//! user's other settings and hooks as they are.

use std::path::PathBuf;

use masterhub_core::ipc::SessionInfo;
use serde::Serialize;
use serde_json::{Map, Value, json};
use tauri::State;

use crate::{AppState, err};

/// The hook events the status LED and session keys use.
const EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Notification",
    "Elicitation",
    "ElicitationResult",
    "Stop",
    "StopFailure",
    "SessionEnd",
];

/// What marks a hook as ours.
const MARKER: &str = "masterhubd";
const SUBCOMMAND: &str = "claude-hook";

#[derive(Serialize)]
pub struct HookStatus {
    installed: bool,
    /// Shown in the UI.
    path: String,
    /// Events that report to masterhubd.
    hooks: usize,
    /// Seconds since a session last changed state, a stand-in for the last hook event.
    last_event_secs: Option<u64>,
}

fn settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or("no home directory")?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

/// The hook command: on Windows the service's full path in Git Bash form (the installer does
/// not put it on `PATH`), else `masterhubd` (on `PATH` from the package).
fn command_line() -> String {
    #[cfg(windows)]
    if let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("masterhubd.exe"))).filter(|p| p.exists()) {
        return format!("\"{}\" {SUBCOMMAND}", exe.display().to_string().replace('\\', "/"));
    }
    format!("{MARKER} {SUBCOMMAND}")
}

fn read(path: &PathBuf) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{} is not valid JSON ({e}); fix it first", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn is_ours(hook: &Value) -> bool {
    let cmd = hook.get("command").and_then(Value::as_str).unwrap_or("");
    cmd.contains(MARKER) && cmd.contains(SUBCOMMAND)
}

/// Events whose hooks include ours.
fn installed_events(settings: &Value) -> usize {
    let Some(hooks) = settings.get("hooks").and_then(Value::as_object) else { return 0 };
    EVENTS.iter().filter(|e| hooks.get(**e).and_then(Value::as_array).is_some_and(|groups| groups.iter().any(group_has_ours))).count()
}

fn group_has_ours(group: &Value) -> bool {
    group.get("hooks").and_then(Value::as_array).is_some_and(|h| h.iter().any(is_ours))
}

/// Add our hook to every event (updating the command of an existing one).
fn install(settings: &mut Value, command: &str) -> Result<(), String> {
    let root = settings.as_object_mut().ok_or("settings.json is not an object")?;
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in settings.json is not an object")?;
    for event in EVENTS {
        let groups = hooks.entry(*event).or_insert_with(|| json!([]));
        let groups = groups.as_array_mut().ok_or_else(|| format!("hooks.{event} in settings.json is not a list"))?;
        let mut found = false;
        for group in groups.iter_mut() {
            for hook in group.get_mut("hooks").and_then(Value::as_array_mut).into_iter().flatten().filter(|h| is_ours(h)) {
                hook["command"] = json!(command);
                found = true;
            }
        }
        if !found {
            groups.push(json!({ "hooks": [{ "type": "command", "command": command, "timeout": 5 }] }));
        }
    }
    Ok(())
}

/// Remove our hooks, and the groups, events and `hooks` itself that this empties.
fn remove(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else { return };
    for groups in hooks.values_mut().filter_map(Value::as_array_mut) {
        for group in groups.iter_mut() {
            if let Some(list) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                list.retain(|h| !is_ours(h));
            }
        }
        // A group we emptied goes; a group that never had hooks stays as the user wrote it.
        groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|h| !h.is_empty()));
    }
    hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));
    if hooks.is_empty() {
        settings.as_object_mut().expect("it had hooks").shift_remove("hooks");
    }
}

fn write(path: &PathBuf, settings: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    // Keep the user's file next to the change the first time, in case something looks wrong.
    let backup = path.with_extension("json.omh-backup");
    if path.exists() && !backup.exists() {
        let _ = std::fs::copy(path, &backup);
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings).map_err(err)? + "\n").map_err(err)?;
    std::fs::rename(&tmp, path).map_err(err)
}

#[tauri::command]
pub fn claude_hooks_status(state: State<AppState>) -> HookStatus {
    let path = settings_path().ok();
    let hooks = path.as_ref().and_then(|p| read(p).ok()).map_or(0, |s| installed_events(&s));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let sessions: Vec<SessionInfo> = state.client.call(masterhub_core::ipc::Request::GetSessions).ok().and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    HookStatus {
        installed: hooks > 0,
        path: if cfg!(windows) { "~\\.claude\\settings.json" } else { "~/.claude/settings.json" }.to_owned(),
        hooks,
        last_event_secs: sessions.iter().map(|s| now.saturating_sub(s.since) / 1000).min(),
    }
}

/// Add (or update) the hooks in `~/.claude/settings.json`.
#[tauri::command]
pub fn claude_hooks_install() -> Result<(), String> {
    let path = settings_path()?;
    let mut settings = read(&path)?;
    install(&mut settings, &command_line())?;
    write(&path, &settings)
}

/// Remove the hooks `claude_hooks_install` added.
#[tauri::command]
pub fn claude_hooks_remove() -> Result<(), String> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(());
    }
    let mut settings = read(&path)?;
    remove(&mut settings);
    write(&path, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_keeps_the_rest_and_is_idempotent() {
        let mut s = json!({
            "model": "opus",
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "notify-send done" }] }], "Custom": [] }
        });
        install(&mut s, "masterhubd claude-hook").unwrap();
        assert_eq!(installed_events(&s), EVENTS.len());
        assert_eq!(s["model"], "opus");
        // The user's own Stop hook stays, ours is added as a group of its own.
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        let once = s.clone();
        install(&mut s, "masterhubd claude-hook").unwrap();
        assert_eq!(s, once);
        // A changed command updates the existing hook instead of adding another.
        install(&mut s, "\"C:/x/masterhubd.exe\" claude-hook").unwrap();
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(s["hooks"]["Stop"][1]["hooks"][0]["command"], "\"C:/x/masterhubd.exe\" claude-hook");
    }

    #[test]
    fn remove_leaves_user_hooks() {
        let mut s = json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "notify-send done" }] }] }, "model": "opus" });
        install(&mut s, "masterhubd claude-hook").unwrap();
        remove(&mut s);
        assert_eq!(s, json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "notify-send done" }] }] }, "model": "opus" }));
        assert_eq!(installed_events(&s), 0);

        let mut only = json!({ "hooks": {} });
        install(&mut only, "masterhubd claude-hook").unwrap();
        remove(&mut only);
        assert_eq!(only, json!({}));
    }

    #[test]
    fn rejects_odd_settings() {
        assert!(install(&mut json!([]), "x").is_err());
        assert!(install(&mut json!({ "hooks": { "Stop": 3 } }), "masterhubd claude-hook").is_err());
    }
}

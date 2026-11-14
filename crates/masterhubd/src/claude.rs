//! `masterhubd claude-hook`: a Claude Code hook that reports the session's status to the
//! running service, which shows it on the Base LED and on session keys (see
//! `docs/CLAUDE_CODE.md`). Also what the service knows about a session beyond its status: its
//! name and how to bring it to the front.
//!
//! Claude Code passes the hook event as JSON on stdin. The hook prints nothing (stdout of some
//! hooks becomes model context) and always succeeds, so it never gets in Claude's way.

use std::io::Read;
use std::path::PathBuf;

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

/// The Claude Code process: the first ancestor that is not a shell (hooks run through Git Bash,
/// `cmd` or PowerShell).
#[cfg(windows)]
fn claude_pid() -> Option<u32> {
    const SHELLS: [&str; 8] = ["sh", "bash", "dash", "zsh", "fish", "cmd", "powershell", "pwsh"];
    let procs = process_table();
    let mut pid = procs.get(&std::process::id())?.0;
    for _ in 0..8 {
        let (parent, name) = procs.get(&pid)?;
        if !SHELLS.contains(&name.as_str()) {
            return Some(pid);
        }
        pid = *parent;
    }
    None
}

/// Every process: pid → (parent pid, exe name without `.exe`, lower-case).
#[cfg(windows)]
pub fn process_table() -> std::collections::HashMap<u32, (u32, String)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};

    let mut procs = std::collections::HashMap::new();
    // SAFETY: plain Win32 calls; the entry is plain data with dwSize set, the snapshot is closed.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return procs;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            let name = String::from_utf16_lossy(&e.szExeFile[..len]).to_ascii_lowercase();
            procs.insert(e.th32ProcessID, (e.th32ParentProcessID, name.trim_end_matches(".exe").to_owned()));
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
    }
    procs
}

#[cfg(not(any(target_os = "linux", windows)))]
fn claude_pid() -> Option<u32> {
    None
}

/// What Claude Code writes about a running session to `~/.claude/sessions/<pid>.json`. Not a
/// documented format, so every field is optional.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionFile {
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    /// `busy` or `idle`.
    pub status: Option<String>,
    /// Start time of the process in clock ticks since boot (Linux), to tell a reused pid apart.
    pub proc_start: Option<String>,
    /// The session's title, as shown in the desktop app's sidebar or set with `/rename`.
    pub name: Option<String>,
    pub cwd: Option<PathBuf>,
    /// `local_<id>` for a session of the Claude desktop app.
    pub host_session_id: Option<String>,
    /// Milliseconds since the epoch.
    pub started_at: Option<u64>,
}

impl SessionFile {
    /// Read the file of the `claude` process `pid`, if it wrote one.
    pub fn of(pid: u32) -> Option<Self> {
        let text = std::fs::read_to_string(claude_dir()?.join("sessions").join(format!("{pid}.json"))).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Sessions whose `claude` process is still running, for those that have not sent a hook
    /// event yet (open before the service started, or without the hooks set up).
    pub fn running() -> Vec<(u32, Self)> {
        let Some(dir) = claude_dir().map(|d| d.join("sessions")) else { return Vec::new() };
        let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
        entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| serde_json::from_str::<Self>(&std::fs::read_to_string(e.path()).ok()?).ok())
            .filter_map(|f| Some((f.pid?, f)))
            .filter(|(pid, f)| f.session_id.is_some() && same_process(*pid, f.proc_start.as_deref()))
            .collect()
    }

    /// The status the file reports; it knows nothing about questions waiting for an answer.
    pub fn status(&self) -> Status {
        if self.status.as_deref() == Some("busy") { Status::Working } else { Status::Ready }
    }

    /// What a key shows: the session name, else the name of its folder.
    pub fn title(&self) -> Option<String> {
        let name = self.name.as_deref().map(str::trim).filter(|n| !n.is_empty());
        name.map(str::to_owned).or_else(|| Some(self.cwd.as_ref()?.file_name()?.to_string_lossy().into_owned()))
    }

    /// `claude://` link that opens this session in the desktop app.
    pub fn desktop_link(&self) -> Option<String> {
        let id = self.host_session_id.as_deref()?;
        // The same shape the desktop app accepts; anything else is not passed to a shell.
        let valid = id.strip_prefix("local_").is_some_and(|r| {
            (1..=64).contains(&r.len()) && r.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
        valid.then(|| format!("claude://code/continue?session={id}"))
    }
}

/// Whether `pid` is still the process that wrote a session file.
#[cfg(target_os = "linux")]
fn same_process(pid: u32, proc_start: Option<&str>) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { return false };
    // Field 22, the start time, counted from the end of "(comm)", which may contain spaces.
    let start = stat.rsplit_once(')').and_then(|(_, rest)| rest.split_whitespace().nth(19));
    proc_start.is_none_or(|p| start == Some(p))
}

#[cfg(not(target_os = "linux"))]
fn same_process(pid: u32, _proc_start: Option<&str>) -> bool {
    crate::controller::process_alive(pid)
}

/// `$CLAUDE_CONFIG_DIR`, else `~/.claude`.
fn claude_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    Some(PathBuf::from(home).join(".claude"))
}

/// Bring a session to the front: open it in the desktop app when it lives there, then focus
/// the window that holds its process (the desktop app, or the terminal it runs in). Blocks
/// briefly; run it off the controller thread.
pub fn bring_up(pid: Option<u32>, file: Option<&SessionFile>) {
    if let Some(link) = file.and_then(SessionFile::desktop_link) {
        log::info!("opening {link}");
        if let Err(e) = open_link(&link) {
            log::error!("opening {link}: {e:#}");
        }
    }
    // Claude Code puts the session's topic in the terminal title.
    let title = file.and_then(SessionFile::title);
    if let Some(pid) = pid
        && let Err(e) = crate::focus::raise(pid, title.as_deref())
    {
        log::warn!("focusing Claude Code session {pid}: {e:#}");
    }
}

#[cfg(not(windows))]
fn open_link(link: &str) -> anyhow::Result<()> {
    std::process::Command::new("xdg-open").arg(link).spawn()?;
    Ok(())
}

#[cfg(windows)]
fn open_link(link: &str) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // `link` is validated by `desktop_link`, so it holds nothing `cmd` would interpret.
    std::process::Command::new("cmd").args(["/C", "start", "", link]).creation_flags(CREATE_NO_WINDOW).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_session_files() {
        let f: SessionFile = serde_json::from_str(
            r#"{"pid":1,"sessionId":"x","cwd":"/home/demo/git/openmasterhub","startedAt":1792000000000,
                "entrypoint":"claude-desktop","hostSessionId":"local_0000demo-0001","name":"Claude preset","status":"busy"}"#,
        )
        .unwrap();
        assert_eq!(f.title().as_deref(), Some("Claude preset"));
        assert_eq!(f.status(), Status::Working);
        assert_eq!(f.started_at, Some(1792000000000));
        assert_eq!(f.desktop_link().as_deref(), Some("claude://code/continue?session=local_0000demo-0001"));

        let cli: SessionFile = serde_json::from_str(r#"{"cwd":"/home/demo/git/openmasterhub","name":" "}"#).unwrap();
        assert_eq!(cli.title().as_deref(), Some("openmasterhub"));
        assert_eq!(cli.desktop_link(), None);

        let odd = SessionFile { host_session_id: Some("local_a&calc".into()), ..Default::default() };
        assert_eq!(odd.desktop_link(), None);
    }

    fn status(json: &str) -> Option<Option<Status>> {
        status_for(&serde_json::from_str(json).unwrap())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn tells_reused_pids_apart() {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
        let start = stat.rsplit_once(')').unwrap().1.split_whitespace().nth(19).unwrap().to_owned();
        assert!(same_process(std::process::id(), Some(&start)));
        assert!(!same_process(std::process::id(), Some("1")));
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

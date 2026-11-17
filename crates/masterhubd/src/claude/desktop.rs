//! Claude Code sessions that the Claude desktop app runs on another machine over SSH. Their
//! `claude` process and hooks run on that machine, so nothing reaches this service from them;
//! the desktop app keeps a file per session here instead, which says when a turn started and
//! ended: `<config>/Claude/claude-code-sessions/<account>/<org>/local_<id>.json`.
//!
//! Not a documented format: every field is optional, and a file that does not parse just leaves
//! its session out. Sessions on this machine are left out too; their hooks report them.
//!
//! A turn can end while background subagents keep running on the other machine. The app syncs
//! the session's transcript here (`~/.claude/projects/ssh-<session id>/`), with a file per
//! subagent, so a session stays working while an agent it launched in the background has not
//! reported back.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use masterhub_core::ipc::Status;
use serde::Deserialize;

use super::SessionFile;

/// How long a remote session stays on its key after its last activity.
const RECENT: Duration = Duration::from_secs(12 * 3600);
/// A turn whose file has not changed for this long counts as over (the app quit mid-turn).
const STALE_TURN: Duration = Duration::from_secs(3 * 3600);
/// A background agent whose transcript has not changed for this long counts as gone (it never
/// reported back: the remote `claude` crashed or the sync stopped).
const STALE_AGENT: Duration = Duration::from_secs(3 * 3600);

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DesktopFile {
    /// `local_<id>`, the desktop app's id.
    session_id: Option<String>,
    /// The Claude Code session id, as hooks report it.
    cli_session_id: Option<String>,
    title: Option<String>,
    cwd: Option<PathBuf>,
    /// Milliseconds since the epoch, like the other times.
    created_at: Option<u64>,
    last_activity_at: Option<u64>,
    /// When the last prompt was sent, and when the last turn ended.
    latest_user_frame_at: Option<u64>,
    last_turn_edge_at: Option<u64>,
    /// When the session was last on screen in the app.
    last_focused_at: Option<u64>,
    is_archived: bool,
    /// Set for a session that runs over SSH.
    ssh_config: Option<serde_json::Value>,
    /// The session that started this one, for sessions started from another.
    spawned_from: Option<SpawnedFrom>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SpawnedFrom {
    session_id: Option<String>,
}

impl DesktopFile {
    /// A turn is running: the last prompt came after the last turn ended.
    fn mid_turn(&self, modified: SystemTime, now: SystemTime) -> bool {
        let fresh = now.duration_since(modified).is_ok_and(|age| age < STALE_TURN);
        fresh && self.latest_user_frame_at.unwrap_or(0) > self.last_turn_edge_at.unwrap_or(0)
    }
}

/// A remote session to show on a key.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteSession {
    /// `claude:<session id>`, the same source its hooks would use.
    pub source: String,
    pub status: Status,
    /// Name, start time and desktop link, as for a local session.
    pub file: SessionFile,
}

/// The desktop app's session files, re-read only when they change.
/// When a session of the desktop app was last on screen, and when its last turn ended
/// (milliseconds since the epoch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub focused: u64,
    pub turn_ended: Option<u64>,
}

#[derive(Default)]
pub struct DesktopSessions {
    files: HashMap<PathBuf, (SystemTime, Option<DesktopFile>)>,
    /// What the synced transcripts say about background agents, by Claude Code session id.
    transcripts: HashMap<String, Transcript>,
}

impl DesktopSessions {
    /// Open remote sessions, working while a turn runs in them or in a session they started,
    /// or while an agent they launched in the background is still running.
    pub fn remote(&mut self) -> Vec<RemoteSession> {
        let Some(dir) = sessions_dir() else { return Vec::new() };
        self.scan(&dir);
        let now = SystemTime::now();
        let mut v = sessions(self.files.values().filter_map(|(m, f)| Some((*m, f.as_ref()?))), now);
        for s in &mut v {
            if s.status == Status::Ready && s.file.session_id.as_deref().is_some_and(|cli| self.agents_running(cli, now)) {
                s.status = Status::Working;
            }
        }
        self.transcripts.retain(|cli, _| v.iter().any(|s| s.file.session_id.as_deref() == Some(cli)));
        v
    }

    /// Whether a background agent of session `cli` is still running, from the transcript the
    /// app syncs here.
    /// When each of the app's sessions (local or remote, by Claude Code session id) was last on
    /// screen and when its last turn ended, from the files read by the last [`Self::remote`].
    pub fn views(&self) -> HashMap<String, View> {
        self.files
            .values()
            .filter_map(|(_, f)| f.as_ref())
            .filter_map(|f| {
                let view = View { focused: f.last_focused_at?, turn_ended: f.last_turn_edge_at };
                Some((f.cli_session_id.clone()?, view))
            })
            .collect()
    }

    fn agents_running(&mut self, cli: &str, now: SystemTime) -> bool {
        // The id ends up in a path.
        if cli.is_empty() || !cli.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
        let Some(dir) = super::claude_dir().map(|d| d.join("projects").join(format!("ssh-{cli}"))) else { return false };
        let t = self.transcripts.entry(cli.to_owned()).or_default();
        t.update(&dir.join(format!("{cli}.jsonl")));
        t.running().any(|id| {
            let modified = std::fs::metadata(dir.join(format!("agent-{id}.jsonl"))).and_then(|m| m.modified());
            modified.is_ok_and(|m| now.duration_since(m).map_or(true, |age| age < STALE_AGENT))
        })
    }

    fn scan(&mut self, dir: &Path) {
        let mut seen = Vec::new();
        for path in json_files(dir) {
            let Some(modified) = std::fs::metadata(&path).and_then(|m| m.modified()).ok() else { continue };
            if self.files.get(&path).is_none_or(|(m, _)| *m != modified) {
                let file = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok());
                self.files.insert(path.clone(), (modified, file));
            }
            seen.push(path);
        }
        self.files.retain(|p, _| seen.contains(p));
    }
}

/// Background agents a session launched and those that reported back, read from its transcript
/// as it grows.
#[derive(Debug, Default)]
struct Transcript {
    /// Bytes read so far, up to the end of the last whole line.
    read: u64,
    launched: HashSet<String>,
    finished: HashSet<String>,
}

impl Transcript {
    /// Read the lines added to the transcript at `path` since the last call.
    fn update(&mut self, path: &Path) {
        let Ok(mut f) = std::fs::File::open(path) else { return };
        let Ok(len) = f.metadata().map(|m| m.len()) else { return };
        if len < self.read {
            // Rewritten: start over.
            *self = Self::default();
        }
        if len == self.read || f.seek(SeekFrom::Start(self.read)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if f.take(len - self.read).read_to_end(&mut buf).is_err() {
            return;
        }
        let Some(end) = buf.iter().rposition(|&b| b == b'\n') else { return };
        for line in buf[..end].split(|&b| b == b'\n') {
            if let Ok(line) = std::str::from_utf8(line) {
                self.line(line);
            }
        }
        self.read += end as u64 + 1;
    }

    /// Note a background agent launched (the Agent tool's result) or reporting back (a
    /// `<task-notification>` queued for the main agent). Quotes are matched unescaped, so the
    /// same text inside a message or tool output does not count.
    fn line(&mut self, line: &str) {
        if let Some(i) = line.find(r#""text":"Async agent launched"#)
            && let Some(id) = after(&line[i..], "agentId: ")
        {
            self.launched.insert(id.to_owned());
        } else if line.starts_with(r#"{"type":"queue-operation","operation":"enqueue""#)
            && line.contains("<task-notification>")
            && let Some(id) = after(line, "<task-id>")
        {
            self.finished.insert(id.to_owned());
        }
    }

    /// Agents launched in the background that have not reported back yet.
    fn running(&self) -> impl Iterator<Item = &str> {
        self.launched.difference(&self.finished).map(String::as_str)
    }
}

/// The id that follows `prefix` in `text`.
fn after<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = &text[text.find(prefix)? + prefix.len()..];
    let id = &rest[..rest.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(rest.len())];
    (!id.is_empty()).then_some(id)
}

/// `local_*.json` two folders below `dir` (account, then organisation).
fn json_files(dir: &Path) -> Vec<PathBuf> {
    let subdirs = |d: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default()
    };
    subdirs(dir)
        .iter()
        .flat_map(|d| subdirs(d))
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("local_"))
        })
        .collect()
}

/// The remote sessions among `files` (with their modification times) at `now`.
fn sessions<'a>(files: impl Iterator<Item = (SystemTime, &'a DesktopFile)>, now: SystemTime) -> Vec<RemoteSession> {
    let files: Vec<(SystemTime, &DesktopFile)> = files.filter(|(_, f)| !f.is_archived).collect();
    let now_ms = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    // Sessions with a turn running, local or remote, by desktop id.
    let busy: Vec<&str> = files.iter().filter(|(m, f)| f.mid_turn(*m, now)).filter_map(|(_, f)| f.session_id.as_deref()).collect();
    files
        .iter()
        .filter(|(_, f)| f.ssh_config.is_some())
        .filter_map(|(modified, f)| {
            let id = f.session_id.as_deref()?;
            let cli = f.cli_session_id.as_deref()?;
            let own = f.mid_turn(*modified, now);
            // A session it started is still working on its behalf.
            let children = files.iter().any(|(_, c)| {
                c.spawned_from.as_ref().and_then(|s| s.session_id.as_deref()) == Some(id)
                    && c.session_id.as_deref().is_some_and(|cid| busy.contains(&cid))
            });
            let recent = now_ms.saturating_sub(f.last_activity_at.unwrap_or(0)) < RECENT.as_millis() as u64;
            if !(own || children || recent) {
                return None;
            }
            let file = SessionFile {
                session_id: Some(cli.to_owned()),
                name: f.title.clone(),
                cwd: f.cwd.clone(),
                host_session_id: Some(id.to_owned()),
                started_at: f.created_at,
                ..SessionFile::default()
            };
            let status = if own || children { Status::Working } else { Status::Ready };
            Some(RemoteSession { source: format!("claude:{cli}"), status, file })
        })
        .collect()
}

/// `claude-code-sessions` in the desktop app's config folder.
fn sessions_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library").join("Application Support");
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".config")))?;
    Some(base.join("Claude").join("claude-code-sessions"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: u64 = 3600 * 1000;

    fn file(json: &str) -> DesktopFile {
        serde_json::from_str(json).unwrap()
    }

    /// A session file at `now_ms`, `ago` ms after its last activity.
    fn remote(id: &str, cli: &str, ago: u64, now_ms: u64, mid_turn: bool) -> DesktopFile {
        let (user, edge) = if mid_turn { (now_ms - ago, now_ms - ago - 10) } else { (now_ms - ago - 10, now_ms - ago) };
        file(&format!(
            r#"{{"sessionId":"{id}","cliSessionId":"{cli}","title":"Vault","cwd":"/home/u","createdAt":1,
               "lastActivityAt":{},"latestUserFrameAt":{user},"lastTurnEdgeAt":{edge},
               "sshConfig":{{"sshHost":"u@nas"}},"someFutureField":[1,2]}}"#,
            now_ms - ago
        ))
    }

    fn run(files: &[DesktopFile], now: SystemTime) -> Vec<(String, Status)> {
        let mut v: Vec<_> = sessions(files.iter().map(|f| (now, f)), now).into_iter().map(|s| (s.source, s.status)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    #[test]
    fn remote_sessions_and_their_status() {
        let now = SystemTime::now();
        let ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let idle = remote("local_a", "aaa", HOUR, ms, false);
        let busy = remote("local_b", "bbb", 1000, ms, true);
        let old = remote("local_c", "ccc", 13 * HOUR, ms, false);
        let local = file(r#"{"sessionId":"local_d","cliSessionId":"ddd","lastActivityAt":1}"#);
        let mut archived = remote("local_e", "eee", 1000, ms, true);
        archived.is_archived = true;
        assert_eq!(
            run(&[idle, busy, old, local, archived], now),
            [("claude:aaa".to_owned(), Status::Ready), ("claude:bbb".to_owned(), Status::Working)]
        );
    }

    #[test]
    fn a_running_session_it_started_keeps_it_working() {
        let now = SystemTime::now();
        let ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        // An old parent: shown only because its child works for it.
        let parent = remote("local_p", "ppp", 20 * HOUR, ms, false);
        let mut child = remote("local_k", "kkk", 1000, ms, true);
        child.spawned_from = Some(SpawnedFrom { session_id: Some("local_p".into()) });
        assert_eq!(run(&[parent.clone(), child.clone()], now), [("claude:kkk".into(), Status::Working), ("claude:ppp".into(), Status::Working)]);
        // The child finished: the parent is old, so it goes.
        let done = remote("local_k", "kkk", 1000, ms, false);
        let done = DesktopFile { spawned_from: child.spawned_from.clone(), ..done };
        assert_eq!(run(&[parent, done], now), [("claude:kkk".into(), Status::Ready)]);
    }

    #[test]
    fn a_turn_left_running_by_a_closed_app_goes_stale() {
        let now = SystemTime::now();
        let ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let f = remote("local_a", "aaa", 1000, ms, true);
        assert!(f.mid_turn(now, now));
        assert!(!f.mid_turn(now - STALE_TURN - Duration::from_secs(1), now));
    }

    #[test]
    fn the_key_shows_the_title_and_opens_the_desktop_app() {
        let now = SystemTime::now();
        let ms = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let s = sessions([(now, &remote("local_abc-1", "aaa", 1000, ms, false))].into_iter(), now).remove(0);
        assert_eq!(s.file.title().as_deref(), Some("Vault"));
        assert_eq!(s.file.desktop_link().as_deref(), Some("claude://code/continue?session=local_abc-1"));
        assert_eq!(s.file.pid, None);
    }

    /// Lines as Claude Code writes them, shortened.
    const LAUNCH: &str = r#"{"parentUuid":"p","isSidechain":false,"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_1","type":"tool_result","content":[{"type":"text","text":"Async agent launched successfully. (internal)\nagentId: a8ee2d055bc747ee8 (internal ID)\nThe agent is working in the background."}]}]}}"#;
    const DONE: &str = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-11-16T14:49:26.288Z","sessionId":"s","content":"<task-notification>\n<task-id>a8ee2d055bc747ee8</task-id>\n<tool-use-id>toolu_1</tool-use-id>\n<status>completed</status>\n</task-notification>"}"#;
    /// The same text quoted in a tool result (grep output, say) is not a launch.
    const QUOTED: &str = r#"{"parentUuid":"q","type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_2","type":"tool_result","content":"{\"type\":\"text\",\"text\":\"Async agent launched successfully. agentId: bbbb1111 \"} {\"type\":\"queue-operation\",\"operation\":\"enqueue\"} <task-notification><task-id>a8ee2d055bc747ee8</task-id>"}]}}"#;

    #[test]
    fn background_agents_run_until_they_report_back() {
        let mut t = Transcript::default();
        t.line(LAUNCH);
        assert_eq!(t.running().collect::<Vec<_>>(), ["a8ee2d055bc747ee8"]);
        t.line(QUOTED);
        assert_eq!(t.running().collect::<Vec<_>>(), ["a8ee2d055bc747ee8"]);
        t.line(DONE);
        assert_eq!(t.running().count(), 0);
    }

    #[test]
    fn reads_the_transcript_as_it_grows() {
        let path = std::env::temp_dir().join(format!("omh-transcript-{}.jsonl", std::process::id()));
        let mut t = Transcript::default();
        // A line still being written is read once it is whole.
        std::fs::write(&path, format!("{LAUNCH}\n{}", &DONE[..40])).unwrap();
        t.update(&path);
        assert_eq!(t.running().count(), 1);
        std::fs::write(&path, format!("{LAUNCH}\n{DONE}\n")).unwrap();
        t.update(&path);
        assert_eq!(t.running().count(), 0);
        // Rewritten shorter: read again from the start.
        std::fs::write(&path, format!("{LAUNCH}\n")).unwrap();
        t.update(&path);
        assert_eq!(t.running().count(), 1);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn finds_files_two_folders_down() {
        let dir = std::env::temp_dir().join(format!("omh-desktop-{}", std::process::id()));
        let leaf = dir.join("acct").join("org");
        std::fs::create_dir_all(&leaf).unwrap();
        std::fs::write(leaf.join("local_x.json"), "{}").unwrap();
        std::fs::write(leaf.join("other.json"), "{}").unwrap();
        std::fs::write(dir.join("local_top.json"), "{}").unwrap();
        let mut found = json_files(&dir);
        found.sort();
        assert_eq!(found, [leaf.join("local_x.json")]);
        let mut d = DesktopSessions::default();
        d.scan(&dir);
        assert_eq!(d.files.len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
        d.scan(&dir);
        assert!(d.files.is_empty());
    }
}


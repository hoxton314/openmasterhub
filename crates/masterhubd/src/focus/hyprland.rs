//! Follows the focused window on Hyprland through its event socket (`.socket2.sock`).

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::controller::Msg;

pub fn spawn(controller: Sender<Msg>) {
    thread::Builder::new()
        .name("hyprland".into())
        .spawn(move || {
            let mut warned = false;
            loop {
                match instance_dir() {
                    Some(dir) => {
                        warned = false;
                        if let Some(class) = active_class(&dir) {
                            let _ = controller.send(Msg::ActiveApp(class));
                        }
                        if let Err(e) = watch(&dir, &controller) {
                            log::debug!("hyprland socket: {e}");
                        }
                    }
                    None if !warned => {
                        log::info!("Hyprland not detected; per-app profiles disabled until it appears");
                        warned = true;
                    }
                    None => {}
                }
                thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("spawn hyprland thread");
}

/// `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE`, or the newest instance directory
/// when the variable is missing (e.g. when started by systemd before Hyprland exported it).
fn instance_dir() -> Option<PathBuf> {
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("hypr");
    if let Some(sig) = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE") {
        let dir = runtime.join(sig);
        if dir.join(".socket2.sock").exists() {
            return Some(dir);
        }
    }
    std::fs::read_dir(&runtime)
        .ok()?
        .flatten()
        .filter(|e| e.path().join(".socket2.sock").exists())
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

fn watch(dir: &std::path::Path, controller: &Sender<Msg>) -> std::io::Result<()> {
    let stream = UnixStream::connect(dir.join(".socket2.sock"))?;
    log::info!("following focused window via Hyprland");
    for line in BufReader::new(stream).lines() {
        let line = line?;
        if let Some(class) = parse_active_window(&line)
            && controller.send(Msg::ActiveApp(class.to_owned())).is_err()
        {
            return Ok(());
        }
    }
    Ok(())
}

/// `activewindow>>kitty,~/src` → `kitty`. An empty class (desktop focused) is reported too.
fn parse_active_window(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("activewindow>>")?;
    Some(rest.split_once(',').map_or(rest, |(class, _)| class))
}

/// Send one command to the request socket (what `hyprctl` does) and return the answer.
fn request(dir: &std::path::Path, command: &str) -> std::io::Result<String> {
    let mut s = UnixStream::connect(dir.join(".socket.sock"))?;
    s.set_read_timeout(Some(Duration::from_secs(1)))?;
    s.write_all(command.as_bytes())?;
    let mut buf = String::new();
    s.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Ask the request socket for the current window: `j/activewindow` returns JSON.
fn active_class(dir: &std::path::Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&request(dir, "j/activewindow").ok()?).ok()?;
    v.get("class")?.as_str().map(str::to_owned)
}

/// Focus the window of `pid`, or of its nearest ancestor that has one (the terminal a CLI
/// runs in), switching workspace if needed. When that process has several windows, prefer the
/// one whose title contains `title`, else the one focused last.
pub fn raise(pid: u32, title: Option<&str>) -> Result<()> {
    let dir = instance_dir().context("Hyprland is not running")?;
    let clients: Vec<Client> = serde_json::from_str(&request(&dir, "j/clients")?)?;
    let mut p = pid;
    for _ in 0..16 {
        if let Some(c) = pick_window(&clients, p, title) {
            // Hyprland 0.55+ with a Lua config takes Lua dispatchers; older ones the classic form.
            let classic = request(&dir, &format!("dispatch focuswindow address:{}", c.address))?;
            if classic.trim() == "ok" {
                return Ok(());
            }
            let lua = request(&dir, &format!("dispatch hl.dsp.focus({{ window = \"address:{}\" }})", c.address))?;
            if lua.trim() != "ok" {
                bail!("hyprland: {}", classic.trim());
            }
            return Ok(());
        }
        p = match parent_pid(p) {
            Some(pp) if pp > 1 => pp,
            _ => break,
        };
    }
    bail!("no window found for process {pid} or its parents")
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Client {
    pid: u32,
    address: String,
    #[serde(default)]
    title: String,
    /// 0 = focused now.
    #[serde(default)]
    focus_history_id: i64,
}

fn pick_window<'a>(clients: &'a [Client], pid: u32, title: Option<&str>) -> Option<&'a Client> {
    let mut own: Vec<&Client> = clients.iter().filter(|c| c.pid == pid).collect();
    own.sort_by_key(|c| c.focus_history_id);
    let title = title.map(str::to_lowercase).filter(|t| !t.is_empty());
    title.and_then(|t| own.iter().find(|c| c.title.to_lowercase().contains(&t)).copied()).or(own.first().copied())
}

fn parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // "pid (comm) state ppid ...", where comm may itself contain parentheses.
    stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_events() {
        assert_eq!(parse_active_window("activewindow>>kitty,~/src"), Some("kitty"));
        assert_eq!(parse_active_window("activewindow>>firefox,a, b, c"), Some("firefox"));
        assert_eq!(parse_active_window("activewindow>>,"), Some(""));
        assert_eq!(parse_active_window("workspace>>2"), None);
    }

    #[test]
    fn picks_the_session_window() {
        let c = |pid, address: &str, title: &str, focus| Client { pid, address: address.into(), title: title.into(), focus_history_id: focus };
        let clients = [c(1, "a", "✳ Fix the tests", 2), c(1, "b", "vim", 1), c(2, "x", "other", 0)];
        assert_eq!(pick_window(&clients, 1, Some("fix the tests")).unwrap().address, "a");
        assert_eq!(pick_window(&clients, 1, Some("nothing")).unwrap().address, "b");
        assert_eq!(pick_window(&clients, 1, None).unwrap().address, "b");
        assert!(pick_window(&clients, 3, None).is_none());
    }

    #[test]
    fn finds_parent_process() {
        assert_eq!(parent_pid(std::process::id()), Some(std::os::unix::process::parent_id()));
    }
}

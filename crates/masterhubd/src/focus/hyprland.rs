//! Follows the focused window on Hyprland through its event socket (`.socket2.sock`).

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

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

/// Ask the request socket for the current window: `j/activewindow` returns JSON.
fn active_class(dir: &std::path::Path) -> Option<String> {
    let mut s = UnixStream::connect(dir.join(".socket.sock")).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    s.write_all(b"j/activewindow").ok()?;
    let mut buf = String::new();
    s.read_to_string(&mut buf).ok()?;
    let v: serde_json::Value = serde_json::from_str(&buf).ok()?;
    v.get("class")?.as_str().map(str::to_owned)
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
}

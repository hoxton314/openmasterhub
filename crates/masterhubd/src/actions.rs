//! Executes actions on a dedicated thread: shell commands and virtual key presses.

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, KeyCode, KeyEvent};

use masterhub_core::config::Action;

pub struct Request {
    pub action: Action,
    /// Substituted for `{value}` in commands.
    pub value: Option<u8>,
    /// Requests with the same key are coalesced: while a command for that key is still
    /// running, only the newest pending request is kept (fader drags send many updates).
    pub coalesce: Option<String>,
}

pub fn spawn() -> Sender<Request> {
    let (tx, rx) = channel();
    thread::Builder::new().name("actions".into()).spawn(move || run(rx)).expect("spawn actions thread");
    tx
}

fn run(rx: Receiver<Request>) {
    let mut keyboard: Option<VirtualDevice> = None;
    let mut running: HashMap<String, Child> = HashMap::new();
    let mut pending: HashMap<String, String> = HashMap::new();
    let mut detached: Vec<Child> = Vec::new();

    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(req) => match req.action {
                Action::Keys(combo) => {
                    if let Err(e) = send_keys(&mut keyboard, &combo) {
                        log::error!("keys {combo:?}: {e:#}");
                    }
                }
                // Navigation is handled by the controller and never reaches this thread.
                Action::Page(_) | Action::Folder(_) | Action::Back(_) | Action::Profile(_) => {}
                Action::Cmd(cmd) => {
                    let cmd = match req.value {
                        Some(v) => cmd.replace("{value}", &v.to_string()),
                        None => cmd,
                    };
                    match req.coalesce {
                        Some(key) => {
                            pending.insert(key, cmd);
                        }
                        None => detached.extend(spawn_shell(&cmd)),
                    }
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        running.retain(|_, child| !matches!(child.try_wait(), Ok(Some(_))));
        detached.retain_mut(|child| !matches!(child.try_wait(), Ok(Some(_))));
        let ready: Vec<String> = pending.keys().filter(|k| !running.contains_key(*k)).cloned().collect();
        for key in ready {
            let cmd = pending.remove(&key).expect("key from pending");
            if let Some(child) = spawn_shell(&cmd) {
                running.insert(key, child);
            }
        }
    }
}

fn spawn_shell(cmd: &str) -> Option<Child> {
    log::debug!("run: {cmd}");
    match Command::new("sh").args(["-c", cmd]).stdin(Stdio::null()).spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            log::error!("spawning {cmd:?}: {e}");
            None
        }
    }
}

fn send_keys(keyboard: &mut Option<VirtualDevice>, combo: &str) -> Result<()> {
    let keys = parse_combo(combo)?;
    if keyboard.is_none() {
        *keyboard = Some(create_keyboard()?);
    }
    let kb = keyboard.as_mut().expect("keyboard just created");
    for k in &keys {
        kb.emit(&[*KeyEvent::new(*k, 1)])?;
    }
    for k in keys.iter().rev() {
        kb.emit(&[*KeyEvent::new(*k, 0)])?;
    }
    Ok(())
}

fn create_keyboard() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    // KEY_ESC .. KEY_MICMUTE covers the keyboard and media keys.
    for code in 1..=248 {
        keys.insert(KeyCode::new(code));
    }
    let dev = VirtualDevice::builder()?.name("openmasterhub virtual keyboard").with_keys(&keys)?.build()?;
    // Give the compositor a moment to pick up the new device before the first key.
    thread::sleep(Duration::from_millis(200));
    Ok(dev)
}

/// `"ctrl+shift+m"` -> [KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_M]
pub fn parse_combo(combo: &str) -> Result<Vec<KeyCode>> {
    let keys = combo
        .split('+')
        .map(|part| {
            let name = part.trim().to_ascii_lowercase();
            let canonical = match name.as_str() {
                "ctrl" | "control" => "leftctrl",
                "shift" => "leftshift",
                "alt" => "leftalt",
                "altgr" => "rightalt",
                "super" | "meta" | "win" | "cmd" => "leftmeta",
                "return" => "enter",
                "del" => "delete",
                "pgup" => "pageup",
                "pgdn" => "pagedown",
                other => other,
            };
            KeyCode::from_str(&format!("KEY_{}", canonical.to_ascii_uppercase()))
                .map_err(|_| anyhow!("unknown key {part:?} in {combo:?}"))
        })
        .collect::<Result<Vec<_>>>()?;
    if keys.is_empty() {
        bail!("empty key combination");
    }
    Ok(keys)
}

/// Check all key names in the config up front, so typos are reported at startup.
pub fn validate(action: &Action) -> Result<()> {
    match action {
        Action::Keys(k) => parse_combo(k).map(|_| ()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos() {
        assert_eq!(
            parse_combo("ctrl+Shift+m").unwrap(),
            vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_M]
        );
        assert_eq!(parse_combo("playpause").unwrap(), vec![KeyCode::KEY_PLAYPAUSE]);
        assert_eq!(parse_combo("super+f13").unwrap(), vec![KeyCode::KEY_LEFTMETA, KeyCode::KEY_F13]);
        assert!(parse_combo("ctrl+nosuchkey").is_err());
    }
}

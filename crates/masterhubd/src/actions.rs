//! Executes actions on a dedicated thread: shell commands and virtual key presses.

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use anyhow::Result;

use masterhub_core::config::Action;

use crate::keyboard::Keyboard;

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
    let mut keyboard = Keyboard::new();
    let mut running: HashMap<String, Child> = HashMap::new();
    let mut pending: HashMap<String, String> = HashMap::new();
    let mut detached: Vec<Child> = Vec::new();

    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(req) => match req.action {
                Action::Keys(combo) => {
                    if let Err(e) = keyboard.send(&combo) {
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
    match shell(cmd).stdin(Stdio::null()).spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            log::error!("spawning {cmd:?}: {e}");
            None
        }
    }
}

/// `sh -c` on Unix, `cmd /C` without a console window on Windows.
fn shell(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = Command::new("cmd");
        c.arg("/C").raw_arg(cmd).creation_flags(CREATE_NO_WINDOW);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    }
}

/// Check all key names in the config up front, so typos are reported at startup.
pub fn validate(action: &Action) -> Result<()> {
    match action {
        Action::Keys(k) => crate::keyboard::validate(k),
        _ => Ok(()),
    }
}

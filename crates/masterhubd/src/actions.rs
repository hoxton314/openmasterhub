//! Executes actions on a dedicated thread: shell commands, virtual key presses and volume.

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use anyhow::Result;

use masterhub_core::config::{Action, Config, VolumeChange};

use crate::audio;
use crate::keyboard::Keyboard;

pub struct Request {
    pub action: Action,
    /// Substituted for `{value}` in commands.
    pub value: Option<u8>,
    /// Requests with the same key are coalesced: while a command for that key is still
    /// running, only the newest pending request is kept (fader drags send many updates).
    pub coalesce: Option<String>,
    /// For `volume` and `mute`: their target, with audio groups resolved by the controller.
    pub audio: Option<audio::Target>,
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
                Action::Keys(keys) => {
                    let combo = keys.combo();
                    if let Err(e) = keyboard.send(&combo) {
                        log::error!("keys {combo:?}: {e:#}");
                    }
                }
                Action::Volume(v) => match (VolumeChange::parse(&v.level, req.value), &req.audio) {
                    (Some(change), Some(target)) => {
                        if let Err(e) = audio::set_volume(target, change) {
                            log::error!("volume {v:?}: {e:#}");
                        }
                    }
                    (None, _) => log::error!("volume {v:?}: invalid"),
                    (_, None) => log::error!("volume {v:?}: unknown audio group, or no focused app"),
                },
                Action::Mute(m) => match &req.audio {
                    Some(target) => {
                        if let Err(e) = audio::set_mute(target, m.state) {
                            log::error!("mute {m:?}: {e:#}");
                        }
                    }
                    None => log::error!("mute {m:?}: unknown audio group, or no focused app"),
                },
                // Navigation and sessions are handled by the controller and never reach this thread.
                Action::Page(_)
                | Action::Folder(_)
                | Action::Back(_)
                | Action::Profile(_)
                | Action::Setup(_)
                | Action::Pin(_)
                | Action::Session(_)
                | Action::Sessions(_) => {}
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
        Action::Keys(k) => crate::keyboard::validate(&k.combo()),
        _ => Ok(()),
    }
}

/// Every binding the service cannot run, as one message each: `keys` actions (profiles and
/// pinned keys) and setup shortcuts whose key names or hold do not parse.
pub fn binding_errors(config: &Config) -> Vec<String> {
    let mut errors = Vec::new();
    for profile in &config.profiles {
        for action in profile.actions() {
            if let Err(e) = validate(action) {
                errors.push(format!("profile {:?}: {e:#}", profile.name));
            }
        }
    }
    for pinned in &config.pinned {
        for action in [&pinned.key.press, &pinned.key.release].into_iter().flatten() {
            if let Err(e) = validate(action) {
                errors.push(format!("pinned key {}: {e:#}", pinned.key.index));
            }
        }
    }
    for setup in &config.setups {
        for shortcut in setup.shortcuts() {
            if let Err(e) = crate::keyboard::validate(shortcut) {
                errors.push(format!("setup {:?}: shortcut {shortcut:?}: {e:#}", setup.name));
            }
        }
    }
    errors
}

/// The full check for a config that is about to be saved or reloaded: [`Config::validate`]
/// plus [`binding_errors`]. Startup uses the same check but only logs binding errors.
pub fn validate_config(config: &Config) -> Result<()> {
    config.validate()?;
    config.validate_holds()?;
    let errors = binding_errors(config);
    if !errors.is_empty() {
        anyhow::bail!("{}", errors.join("; "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(toml: &str) -> Config {
        Config::parse_unchecked(toml, Path::new(".")).unwrap()
    }

    #[test]
    fn full_validation_covers_bindings() {
        let ok = parse("[[profile]]\nname='A'\n[[profile.page]]\nname='P'\n[[profile.page.key]]\nindex=0\npress={keys='ctrl+c'}\n");
        assert!(validate_config(&ok).is_ok());

        let typo = parse("[[profile]]\nname='A'\n[profile.dial]\npress={keys='ctrl+nosuchkey'}\n");
        let e = validate_config(&typo).unwrap_err().to_string();
        assert!(e.contains("nosuchkey") && e.contains("\"A\""), "{e}");

        let pinned = parse("[[profile]]\nname='A'\n[[pinned]]\nindex=1\npress={keys='ctrl+nosuchkey'}\n");
        assert!(validate_config(&pinned).unwrap_err().to_string().contains("pinned key 1"));

        let shortcut = parse("[[profile]]\nname='A'\n[[setup]]\nname='S'\nprofiles=[]\nfallback='A'\nshortcut='ctrl+nosuchkey'\n");
        assert!(validate_config(&shortcut).unwrap_err().to_string().contains("shortcut"));
    }

    #[test]
    fn hold_over_the_limit_is_rejected_everywhere() {
        let over = masterhub_core::config::MAX_HOLD_MS + 1;
        let dial = format!("[[profile]]\nname='A'\n[profile.dial]\npress={{keys={{combo='o', hold={over}}}}}\n");
        // Already by the core check, which the app runs too.
        assert!(parse(&dial).validate_holds().is_err());
        assert!(validate_config(&parse(&dial)).is_err());
        let at_limit = dial.replace(&over.to_string(), &masterhub_core::config::MAX_HOLD_MS.to_string());
        assert!(validate_config(&parse(&at_limit)).is_ok());
    }
}

//! masterhubd: background service for the Cooler Master MasterHUB.

// On Windows the service starts at login; as a GUI-subsystem program it opens no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod audio;
mod claude;
mod controller;
mod focus;
mod hid;
mod ipc;
mod keyboard;
mod resume;
mod setups;
mod state;
mod stats;
mod surface;
mod worker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use masterhub_core::config::{self, Config};

use crate::controller::{Controller, Snapshot};

#[cfg(windows)]
const DEFAULT_CONFIG: &str = r"%APPDATA%\openmasterhub\config.toml";
#[cfg(not(windows))]
const DEFAULT_CONFIG: &str = "$XDG_CONFIG_HOME/openmasterhub/config.toml";

/// Written on first run on Windows, where the installer leaves no config behind.
#[cfg(windows)]
const STARTER_CONFIG: &str = include_str!("../../../examples/starter.toml");

fn usage() -> String {
    format!(
        "usage: masterhubd [--config PATH] [--check] [--list]
       masterhubd claude-hook   (Claude Code hook, see docs/CLAUDE_CODE.md)
       masterhubd ctl setup NAME|next|prev|pick
       masterhubd ctl profile NAME|next|prev|pick
       masterhubd ctl pin [toggle|release]
                                (tell the running service, e.g. from a compositor keybind)

  --config PATH  config file (default: {DEFAULT_CONFIG})
  --check        validate the config and exit
  --list         list connected modules and exit

Logging: RUST_LOG=debug masterhubd"
    )
}

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("claude-hook") {
        claude::hook();
        return Ok(());
    }
    if std::env::args().nth(1).as_deref() == Some("ctl") {
        let args: Vec<String> = std::env::args().skip(2).collect();
        let action = ctl_action(&args).with_context(usage)?;
        masterhub_core::ipc::call_once(masterhub_core::ipc::Request::RunAction { action, value: None })?;
        return Ok(());
    }
    init_logging();

    let mut config_path = None;
    let (mut check, mut list) = (false, false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_path = Some(PathBuf::from(args.next().context("--config needs a path")?)),
            "--check" => check = true,
            "--list" => list = true,
            "-h" | "--help" => {
                println!("{}", usage());
                return Ok(());
            }
            other => bail!("unknown argument {other:?}\n\n{}", usage()),
        }
    }

    if list {
        for m in hid::enumerate() {
            let ports = m.usb_ports.as_ref().map_or("?".into(), |p| p.iter().map(u8::to_string).collect::<Vec<_>>().join("."));
            println!("{:<8} {:<22} {:<12} {}", m.kind.name(), m.serial, ports, m.path);
        }
        return Ok(());
    }

    #[cfg(windows)]
    let first_run = config_path.is_none() && !check;
    let path = config_path.unwrap_or_else(config::default_path);
    #[cfg(windows)]
    if first_run && !path.exists() {
        match path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, STARTER_CONFIG)) {
            Ok(()) => log::info!("first run: wrote the starter config to {}", path.display()),
            Err(e) => log::warn!("writing the starter config to {}: {e}", path.display()),
        }
    }
    let (config, text) = if path.exists() {
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        (Config::load(&path)?, text)
    } else {
        log::warn!("{} not found, starting with an empty config", path.display());
        (Config::default(), String::new())
    };
    // The app and hot reload reject such bindings when saving; one that still gets here (a
    // hand-edited file, an older version's save) is reported but must not keep the service
    // from starting: the action is skipped with an error when pressed.
    let errors = actions::binding_errors(&config);
    if check && !errors.is_empty() {
        bail!("{}", errors.join("\n"));
    }
    for e in &errors {
        log::error!("ignoring binding: {e}");
    }
    if check {
        println!("{}: ok", path.display());
        return Ok(());
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let shutdown = shutdown.clone();
        ctrlc::set_handler(move || shutdown.store(true, Ordering::Relaxed)).context("installing signal handler")?;
    }

    let (ctl_tx, ctl_rx) = channel();
    let snapshot = Arc::new(Mutex::new(Snapshot::default()));
    let events = Arc::new(ipc::Broadcaster::default());
    ipc::serve(ctl_tx.clone(), snapshot.clone(), events.clone())?;

    let actions = actions::spawn();
    {
        let snapshot = snapshot.clone();
        let path = path.clone();
        thread::Builder::new()
            .name("controller".into())
            .spawn(move || Controller::new(config, text, path, actions, snapshot, events, state::default_path()).run(ctl_rx))
            .context("spawning controller")?;
    }
    focus::spawn(ctl_tx.clone());
    audio::spawn_watcher();

    log::info!("masterhubd started, config {}", path.display());
    let mut workers: HashMap<String, JoinHandle<()>> = HashMap::new();
    // Bumped after each resume from suspend; workers then reconnect to re-initialise their module.
    let resumes = Arc::new(AtomicU64::new(0));
    let mut sleep_watch = resume::SleepWatch::new();
    while !shutdown.load(Ordering::Relaxed) {
        if sleep_watch.poll() {
            resumes.fetch_add(1, Ordering::Relaxed);
        }
        workers.retain(|_, h| !h.is_finished());
        for info in hid::enumerate() {
            if workers.contains_key(&info.serial) {
                continue;
            }
            let serial = info.serial.clone();
            let (tx, shutdown, resumes) = (ctl_tx.clone(), shutdown.clone(), resumes.clone());
            let handle = thread::Builder::new()
                .name(format!("{}-{}", info.kind.name(), serial))
                .spawn(move || worker::run(info, tx, shutdown, resumes))
                .context("spawning worker")?;
            workers.insert(serial, handle);
        }
        thread::sleep(Duration::from_millis(500));
    }

    log::info!("shutting down");
    // A module that stopped answering holds its worker in a request until the kernel times it
    // out (5 s); name it, so a slow stop can be traced to a module.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while workers.values().any(|h| !h.is_finished()) && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    let slow: Vec<&str> = workers.iter().filter(|(_, h)| !h.is_finished()).map(|(s, _)| s.as_str()).collect();
    if !slow.is_empty() {
        log::warn!("still waiting for {} to finish a request", slow.join(", "));
    }
    for (_, h) in workers {
        let _ = h.join();
    }
    log::info!("stopped");
    Ok(())
}

/// The action a `masterhubd ctl …` command line asks for.
fn ctl_action(args: &[String]) -> Result<config::Action> {
    use config::{Action, PinAction};
    let arg = |i: usize| args.get(i).map(String::as_str);
    Ok(match (arg(0), arg(1)) {
        (Some("setup"), Some(name)) => Action::Setup(name.to_owned()),
        (Some("profile"), Some(name)) => Action::Profile(name.to_owned()),
        (Some("pin"), None | Some("toggle")) => Action::Pin(PinAction::Toggle),
        (Some("pin"), Some("release")) => Action::Pin(PinAction::Release),
        _ => bail!("unknown ctl command {:?}", args.join(" ")),
    })
}

/// Log to stderr, or on Windows without a console (started at login) to
/// `%LOCALAPPDATA%\openmasterhub\masterhubd.log` (see `open_log`).
fn init_logging() {
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        // SAFETY: plain Win32 call. Succeeds when started from a terminal, so `--list` and the
        // log are visible there.
        let has_console = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0 || cfg!(debug_assertions);
        if !has_console
            && let Some(dir) = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("openmasterhub"))
            && std::fs::create_dir_all(&dir).is_ok()
            && let Ok(file) = open_log(&dir)
        {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        }
    }
    builder.init();
}

/// `masterhubd.log` in `dir`, appended to. Past 1 MiB it becomes `masterhubd.old.log` (replacing
/// the previous one), so the log cannot grow without bound.
#[cfg(windows)]
fn open_log(dir: &std::path::Path) -> std::io::Result<std::fs::File> {
    const MAX_LOG: u64 = 1 << 20;
    let log = dir.join("masterhubd.log");
    if std::fs::metadata(&log).is_ok_and(|m| m.len() > MAX_LOG) {
        let _ = std::fs::rename(&log, dir.join("masterhubd.old.log"));
    }
    std::fs::OpenOptions::new().create(true).append(true).open(log)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ctl_commands() {
        use masterhub_core::config::{Action, PinAction};
        let a = |s: &str| super::ctl_action(&s.split_whitespace().map(str::to_owned).collect::<Vec<_>>()).ok();
        assert_eq!(a("setup Stream"), Some(Action::Setup("Stream".into())));
        assert_eq!(a("profile next"), Some(Action::Profile("next".into())));
        assert_eq!(a("pin"), Some(Action::Pin(PinAction::Toggle)));
        assert_eq!(a("pin release"), Some(Action::Pin(PinAction::Release)));
        assert_eq!(a("setup"), None);
        assert_eq!(a("reboot now"), None);
    }
}

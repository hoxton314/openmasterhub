//! masterhubd: background service for the Cooler Master MasterHUB.

// On Windows the service starts at login; as a GUI-subsystem program it opens no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod claude;
mod controller;
mod focus;
mod hid;
mod ipc;
mod keyboard;
mod stats;
mod worker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use masterhub_core::config::{self, Config};

use crate::controller::{Controller, Snapshot};

const USAGE: &str = "usage: masterhubd [--config PATH] [--check] [--list]
       masterhubd claude-hook   (Claude Code hook, see docs/CLAUDE_CODE.md)

  --config PATH  config file (default: $XDG_CONFIG_HOME/openmasterhub/config.toml)
  --check        validate the config and exit
  --list         list connected modules and exit

Logging: RUST_LOG=debug masterhubd";

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("claude-hook") {
        claude::hook();
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
                println!("{USAGE}");
                return Ok(());
            }
            other => bail!("unknown argument {other:?}\n\n{USAGE}"),
        }
    }

    if list {
        for m in hid::enumerate() {
            let ports = m.usb_ports.as_ref().map_or("?".into(), |p| p.iter().map(u8::to_string).collect::<Vec<_>>().join("."));
            println!("{:<8} {:<22} {:<12} {}", m.kind.name(), m.serial, ports, m.path);
        }
        return Ok(());
    }

    let path = config_path.unwrap_or_else(config::default_path);
    let (config, text) = if path.exists() {
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        (Config::load(&path)?, text)
    } else {
        log::warn!("{} not found, starting with an empty config", path.display());
        (Config::default(), String::new())
    };
    for action in config.profiles.iter().flat_map(|p| p.actions()) {
        actions::validate(action)?;
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
            .spawn(move || Controller::new(config, text, path, actions, snapshot, events).run(ctl_rx))
            .context("spawning controller")?;
    }
    focus::spawn(ctl_tx.clone());

    log::info!("masterhubd started, config {}", path.display());
    let mut workers: HashMap<String, JoinHandle<()>> = HashMap::new();
    while !shutdown.load(Ordering::Relaxed) {
        workers.retain(|_, h| !h.is_finished());
        for info in hid::enumerate() {
            if workers.contains_key(&info.serial) {
                continue;
            }
            let serial = info.serial.clone();
            let (tx, shutdown) = (ctl_tx.clone(), shutdown.clone());
            let handle = thread::Builder::new()
                .name(format!("{}-{}", info.kind.name(), serial))
                .spawn(move || worker::run(info, tx, shutdown))
                .context("spawning worker")?;
            workers.insert(serial, handle);
        }
        thread::sleep(Duration::from_millis(500));
    }

    log::info!("shutting down");
    for (_, h) in workers {
        let _ = h.join();
    }
    Ok(())
}

/// Log to stderr, or on Windows without a console (started at login) to
/// `%LOCALAPPDATA%\openmasterhub\masterhubd.log`.
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
            && let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("masterhubd.log"))
        {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        }
    }
    builder.init();
}

//! masterhubd: Linux service for the Cooler Master MasterHUB.

mod actions;
mod controller;
mod dbus;
mod focus;
mod hid;
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
use masterhub_core::dbus::{BUS_NAME, OBJECT_PATH};

use crate::controller::{Controller, Snapshot};

const USAGE: &str = "usage: masterhubd [--config PATH] [--check] [--list] [--no-dbus]

  --config PATH  config file (default: $XDG_CONFIG_HOME/openmasterhub/config.toml)
  --check        validate the config and exit
  --list         list connected modules and exit
  --no-dbus      don't register on the session bus (no GUI control)

Logging: RUST_LOG=debug masterhubd";

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut config_path = None;
    let (mut check, mut list, mut no_dbus) = (false, false, false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_path = Some(PathBuf::from(args.next().context("--config needs a path")?)),
            "--check" => check = true,
            "--list" => list = true,
            "--no-dbus" => no_dbus = true,
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
    // Windows has no session bus; the GUI link there comes with the cross-platform IPC.
    let no_dbus = no_dbus || cfg!(windows);
    let bus = if no_dbus {
        None
    } else {
        let api = dbus::Api { controller: ctl_tx.clone(), snapshot: snapshot.clone() };
        let conn = zbus::blocking::connection::Builder::session()
            .and_then(|b| b.name(BUS_NAME))
            .and_then(|b| b.serve_at(OBJECT_PATH, api))
            .and_then(|b| b.build())
            .with_context(|| format!("registering {BUS_NAME} on the session bus (is masterhubd already running?)"))?;
        log::info!("D-Bus API at {BUS_NAME}");
        Some(conn)
    };

    let actions = actions::spawn();
    {
        let (snapshot, bus) = (snapshot.clone(), bus.clone());
        let path = path.clone();
        thread::Builder::new()
            .name("controller".into())
            .spawn(move || Controller::new(config, text, path, actions, snapshot, bus).run(ctl_rx))
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

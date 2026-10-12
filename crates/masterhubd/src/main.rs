//! masterhubd: Linux service for the Cooler Master MasterHUB.

mod actions;
mod config;
mod hid;
mod render;
mod worker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::config::Config;
use crate::worker::Shared;

const USAGE: &str = "usage: masterhubd [--config PATH] [--check] [--list]

  --config PATH  config file (default: $XDG_CONFIG_HOME/openmasterhub/config.toml)
  --check        validate the config and exit
  --list         list connected modules and exit

Logging: RUST_LOG=debug masterhubd";

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
            println!("{:<8} {:<22} {}", m.kind.name(), m.serial, m.devnode.display());
        }
        return Ok(());
    }

    let path = config_path.unwrap_or_else(config::default_path);
    let config = if path.exists() {
        Config::load(&path)?
    } else {
        log::warn!("{} not found, running with an empty config", path.display());
        Config::default()
    };
    validate_actions(&config)?;
    if check {
        println!("{}: ok", path.display());
        return Ok(());
    }

    let shared = Arc::new(Shared {
        renderer: render::Renderer::new(config.font.as_deref()),
        config,
        actions: actions::spawn(),
        shutdown: AtomicBool::new(false),
    });
    {
        let shared = shared.clone();
        ctrlc::set_handler(move || shared.shutdown.store(true, Ordering::Relaxed)).context("installing signal handler")?;
    }

    log::info!("masterhubd started, config {}", path.display());
    let mut workers: HashMap<String, JoinHandle<()>> = HashMap::new();
    while !shared.shutdown.load(Ordering::Relaxed) {
        workers.retain(|_, h| !h.is_finished());
        for info in hid::enumerate() {
            if workers.contains_key(&info.serial) {
                continue;
            }
            let serial = info.serial.clone();
            let shared = shared.clone();
            let handle = thread::Builder::new()
                .name(format!("{}-{}", info.kind.name(), serial))
                .spawn(move || worker::run(info, shared))
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

fn validate_actions(cfg: &Config) -> Result<()> {
    let all = cfg
        .keys
        .iter()
        .flat_map(|k| [&k.press, &k.release])
        .chain(cfg.dial.iter().flat_map(|d| [&d.cw, &d.ccw, &d.press, &d.release]))
        .chain(cfg.faders.iter().map(|f| &f.change))
        .chain(cfg.knobs.iter().flat_map(|k| [&k.cw, &k.ccw, &k.press, &k.release]))
        .chain(cfg.rollers.iter().flat_map(|r| [&r.away, &r.toward]))
        .flatten();
    for action in all {
        actions::validate(action)?;
    }
    Ok(())
}

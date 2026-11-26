//! masterhub-games: opt-in game modes for the MasterHUB (from PULPIT by Dawid Gorczyca). While a
//! game that is switched on has focus, its mode takes the keypad, the dial and the rollers
//! through masterhubd's live surface (docs/LIVE_SURFACE.md) and gives them back after.
//!
//! Usage (the app's Game modes section uses the same commands):
//!   masterhub-games [--config PATH]               run the modes of the games switched on;
//!                                                 reloads when games.toml changes
//!   masterhub-games list [--json]                 games in this build and their state
//!   masterhub-games plan GAME [--path DIR]        what setup would do, as JSON
//!   masterhub-games setup GAME [--path DIR] [--no-extract]
//!                                                 install the game-side files, extract the
//!                                                 look from your install, switch it on
//!   masterhub-games disable GAME
//!   masterhub-games sheet GAME OUT.png            a game's keys as one picture
//!   masterhub-games --check                       validate the config

// No console window when started at login on Windows; commands attach to the terminal's.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// A build with only some games leaves parts of the engine unused.
#![cfg_attr(not(all(feature = "cyberpunk", feature = "deadlock", feature = "wow", feature = "crimson")), allow(dead_code))]

mod bridge;
mod config;
mod ctx;
mod game;
mod games;
mod hub;
mod live;
mod setup;
mod steam;
mod vpk;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    init_logging();
    let mut args = std::env::args().skip(1);
    let mut path = config::Config::default_path();
    let mut check = false;
    let mut command: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => path = args.next().context("--config needs a path")?.into(),
            "--check" => check = true,
            _ if command.is_none() && !a.starts_with('-') => command = Some(a),
            _ => rest.push(a),
        }
    }
    let flag = |name: &str| rest.iter().any(|a| a == name);
    let value = |name: &str| rest.iter().position(|a| a == name).and_then(|i| rest.get(i + 1)).cloned();
    let positional = |i: usize| rest.iter().filter(|a| !a.starts_with('-')).nth(i).cloned();
    match command.as_deref() {
        None => {}
        Some("list") => return setup::list(&path, flag("--json")),
        Some("plan") => {
            let game = positional(0).context("plan needs a game name")?;
            let info = games::by_name(&game).with_context(|| format!("unknown game {game:?}"))?;
            let cfg = config::Config::load(&path)?;
            let plan = setup::plan(info, &cfg, value("--path").map(PathBuf::from), !flag("--no-extract"));
            println!("{}", serde_json::to_string(&plan)?);
            return Ok(());
        }
        Some("setup") => {
            let game = positional(0).context("setup needs a game name (see `masterhub-games list`)")?;
            return setup::setup(&path, &game, value("--path").map(PathBuf::from), !flag("--no-extract"));
        }
        Some("disable") => return setup::disable(&path, &positional(0).context("disable needs a game name")?),
        Some("sheet") => {
            let name = positional(0).context("sheet needs a game name and a PNG path")?;
            let out = positional(1).context("sheet needs a PNG path")?;
            let info = games::by_name(&name).with_context(|| format!("no game {name:?}"))?;
            let cfg = config::Config::load(&path)?;
            let game = games::make(info, &cfg);
            game::sheet(&game, &masterhub_games_render::Renderer::new()?).save(&out)?;
            return Ok(());
        }
        Some(other) => bail!("unknown command {other:?} (list, setup, disable, sheet; --config PATH, --check)"),
    }
    let cfg = config::Config::load(&path)?;
    if check {
        println!("{}: ok", path.display());
        return Ok(());
    }
    let Some(_lock) = single_instance()? else {
        log::info!("masterhub-games is already running");
        return Ok(());
    };
    let games = games::enabled(&cfg);
    if games.is_empty() {
        log::info!("no game is switched on in {}; waiting for one (the app's Game modes)", path.display());
    } else {
        log::info!("game modes: {}", games.iter().map(|g| g.title).collect::<Vec<_>>().join(", "));
    }
    game::spawn_all(games);
    // Game modes are threads with their own claims; a changed config starts over cleanly.
    let stamp = |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let start = stamp(&path);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if stamp(&path) != start {
            log::info!("{} changed: restarting", path.display());
            drop(_lock);
            return restart();
        }
    }
}

/// Set for the copy `restart` starts.
const RESTARTED: &str = "MASTERHUB_GAMES_RESTARTED";

/// Hold a lock for this user's running copy; `None` when another one holds it.
fn single_instance() -> Result<Option<std::fs::File>> {
    let dir = ctx::data_dir("");
    std::fs::create_dir_all(&dir)?;
    let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(dir.join("masterhub-games.lock"))?;
    // Started by `restart` on Windows: the old copy may still be on its way out.
    let tries = if std::env::var_os(RESTARTED).is_some() { 50 } else { 1 };
    for _ in 0..tries {
        match f.try_lock() {
            Ok(()) => return Ok(Some(f)),
            Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(std::fs::TryLockError::Error(e)) => return Err(e.into()),
        }
    }
    Ok(None)
}

/// Start this program again with the same arguments: in place on Unix (systemd keeps the
/// same process), as a new detached process on Windows.
fn restart() -> Result<()> {
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.args(std::env::args_os().skip(1)).env(RESTARTED, "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(cmd.exec().into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW).spawn()?;
        Ok(())
    }
}

/// Log to stderr, or on Windows without a console (started at login) to
/// `%LOCALAPPDATA%\\openmasterhub\\masterhub-games.log` (cut back to empty past 1 MiB).
fn init_logging() {
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
        // SAFETY: plain Win32 call; succeeds when started from a terminal.
        let has_console = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0 || cfg!(debug_assertions);
        if !has_console && let Some(dir) = dirs::data_local_dir().map(|d| d.join("openmasterhub")) {
            let path = dir.join("masterhub-games.log");
            let big = std::fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20);
            let file = std::fs::create_dir_all(&dir)
                .and_then(|_| std::fs::OpenOptions::new().create(true).append(!big).write(true).truncate(big).open(&path));
            if let Ok(file) = file {
                builder.target(env_logger::Target::Pipe(Box::new(file)));
            }
        }
    }
    builder.init();
}

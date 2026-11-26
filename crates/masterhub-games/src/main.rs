//! masterhub-games: opt-in game modes for the MasterHUB (from PULPIT by Dawid Gorczyca). While a
//! game that is switched on has focus, its mode takes the keypad, the dial and the rollers
//! through masterhubd's live surface (docs/LIVE_SURFACE.md) and gives them back after.
//!
//! Usage:
//!   masterhub-games [--config PATH]               run the modes of the games switched on
//!   masterhub-games list                          games in this build and their state
//!   masterhub-games setup GAME [--path DIR] [--no-extract]
//!                                                 install the game-side files, extract the
//!                                                 look from your install, switch it on
//!   masterhub-games disable GAME
//!   masterhub-games sheet GAME OUT.png            a game's keys as one picture
//!   masterhub-games --check                       validate the config

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
        Some("list") => return setup::list(&path),
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
    let games = games::enabled(&cfg);
    if games.is_empty() {
        log::info!("no game is switched on in {} (masterhub-games setup GAME); nothing to do", path.display());
        return Ok(());
    }
    log::info!("game modes: {}", games.iter().map(|g| g.title).collect::<Vec<_>>().join(", "));
    game::spawn_all(games);
    loop {
        std::thread::park();
    }
}

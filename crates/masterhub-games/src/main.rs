//! PULPIT: the KREATURA layer for the MasterHUB. Takes over the hub through masterhubd's live
//! surface and draws daily mode in the look of docs/STYLE.md.
//!
//! Usage: `pulpit [--config PATH] [--check]`, `pulpit gallery` (compare looks on the hub), `pulpit sheet GAME OUT.png` (a game's keys as a picture), or `pulpit --knob-test APP`: 10 fast clicks
//! down and 10 up on APP's volume through the knob worker; must end where it started.

mod app;
mod bridge;
mod claude;
mod config;
mod game;
mod games;
mod gallery;
mod hub;
mod icons;
mod kiln;
mod live;
mod notify;
mod sources;
mod state;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let mut args = std::env::args().skip(1);
    let mut path = config::Config::default_path();
    let mut check = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => path = args.next().context("--config needs a path")?.into(),
            "--check" => check = true,
            "gallery" => return gallery::run(),
            "sheet" => {
                let name = args.next().context("sheet needs a game name and a PNG path")?;
                let out = args.next().context("sheet needs a PNG path")?;
                let game = games::by_name(&name).with_context(|| format!("no game {name:?}"))?;
                game::sheet(&game, &pulpit_render::Renderer::new()?).save(&out)?;
                return Ok(());
            }
            "--knob-test" => return knob_test(args.next().context("--knob-test needs an app glob")?),
            other => anyhow::bail!("unknown argument {other:?} (--config PATH, --check, gallery, sheet GAME PNG, --knob-test APP)"),
        }
    }
    let cfg = config::Config::load(&path)?;
    if check {
        println!("{}: ok", path.display());
        return Ok(());
    }
    log::info!("config {}", path.display());
    let renderer = pulpit_render::Renderer::new()?;
    game::spawn_all(games::enabled(&cfg.games));
    app::App::new(cfg, renderer).run()
}

/// Hammer the knob worker like fast turning does and check it lands where it started.
fn knob_test(app: String) -> Result<()> {
    use std::time::{Duration, Instant};
    let hub = hub::Hub::start();
    let start = Instant::now();
    while !hub.online() {
        anyhow::ensure!(start.elapsed() < Duration::from_secs(3), "masterhubd not reachable");
        std::thread::sleep(Duration::from_millis(20));
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let knobs = hub.knob_worker(tx);
    let click = |delta| hub::KnobCmd { knob: 0, apps: vec![app.clone()], delta, mute: false };
    knobs.send(click(0))?;
    let before = rx.recv_timeout(Duration::from_secs(5))?;
    anyhow::ensure!(before.name.is_some(), "{app}: no audio stream");
    // Paced like a hand turning the knob: a click every 35 ms.
    for d in [-2; 10].into_iter().chain([2; 10]) {
        knobs.send(click(d))?;
        std::thread::sleep(Duration::from_millis(35));
    }
    knobs.send(click(0))?;
    let mut last = before.clone();
    while let Ok(v) = rx.recv_timeout(Duration::from_millis(1500)) {
        println!("  {:?} {:.2}", v.name, v.volume);
        last = v;
    }
    println!("before {:.2}, after {:.2}", before.volume, last.volume);
    anyhow::ensure!((before.volume - last.volume).abs() < 0.011, "drifted");
    println!("ok");
    Ok(())
}

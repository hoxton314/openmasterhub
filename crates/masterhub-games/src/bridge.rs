//! The game side of a game mode: a CET mod (games/cyberpunk/pulpit_bridge) that writes
//! `stats.json` a few times a second and runs the commands we append to `cmd.txt`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

/// Stats older than this mean the bridge is not running (game closed, paused or CET broken).
const STALE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Weapon {
    pub name: Option<String>,
    pub mag: Option<f64>,
    pub mag_left: Option<f64>,
    pub reserve: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub ingame: bool,
    pub hp: Option<f64>,
    pub ram: Option<f64>,
    pub ram_max: Option<f64>,
    pub eddies: Option<f64>,
    pub level: Option<f64>,
    pub cred: Option<f64>,
    pub district: Option<String>,
    pub wanted: Option<f64>,
    pub speed: Option<f64>,
    pub weapon: Option<Weapon>,
    pub time: Option<String>,
    #[serde(default)]
    pub combat: bool,
    pub sandy: Option<f64>,
    pub berserk: Option<f64>,
    pub heal_charge: Option<f64>,
    #[serde(default)]
    pub god: bool,
    #[serde(default)]
    pub stamina_inf: bool,
}

/// How the app shows a game's bridge: `connected` (stats are fresh), `disconnected` (the mod is
/// installed, the game is not running) or `error` (its folder is missing). `None`: no bridge mod.
pub fn state(dir: Option<&std::path::Path>) -> Option<&'static str> {
    let dir = dir?;
    Some(if Bridge::new(dir.to_owned()).stats().is_some() {
        "connected"
    } else if dir.is_dir() {
        "disconnected"
    } else {
        "error"
    })
}

pub struct Bridge {
    dir: PathBuf,
    last_seq: u128,
}

impl Bridge {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, last_seq: 0 }
    }

    /// The latest stats, or `None` when the bridge is not running.
    pub fn stats(&self) -> Option<Stats> {
        let path = self.dir.join("stats.json");
        let age = std::fs::metadata(&path).ok()?.modified().ok()?.elapsed().unwrap_or_default();
        if age > STALE {
            return None;
        }
        // The mod rewrites the file in place: a half-written read just waits for the next one.
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Queue a command for the mod (`cmd arg`). The file keeps the last 20 lines; the mod runs
    /// every line with a newer sequence number.
    pub fn send(&mut self, cmd: &str, arg: &str) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let seq = now.max(self.last_seq + 1);
        self.last_seq = seq;
        let path = self.dir.join("cmd.txt");
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        let mut lines: Vec<&str> = old.lines().collect();
        let start = lines.len().saturating_sub(20);
        lines.drain(..start);
        let new = format!("{seq}\t{cmd}\t{arg}");
        lines.push(&new);
        let tmp = self.dir.join("cmd.tmp");
        let r = std::fs::write(&tmp, lines.join("\n") + "\n").and_then(|_| std::fs::rename(&tmp, &path));
        match r {
            Ok(()) => log::info!("bridge: {cmd} {arg}"),
            Err(e) => log::warn!("bridge {cmd}: {e}"),
        }
    }
}

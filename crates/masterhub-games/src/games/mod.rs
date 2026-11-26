//! The games that have a game mode: one file each, compiled in by its Cargo feature and
//! switched on per user in `games.toml` (every game is off until then).

use anyhow::{Result, bail};

use crate::config::Config;
use crate::ctx::Ctx;
use crate::game::Game;

#[cfg(feature = "crimson")]
mod crimson_desert;
#[cfg(feature = "cyberpunk")]
mod cyberpunk;
#[cfg(feature = "deadlock")]
mod deadlock;
#[cfg(feature = "wow")]
mod wow_forever;

/// A game this build knows.
pub struct Info {
    pub name: &'static str,
    pub title: &'static str,
    /// Its Steam app id, to find the install through Steam.
    pub steam: Option<u32>,
    pub make: fn(&Ctx) -> Game,
}

pub const ALL: &[Info] = &[
    #[cfg(feature = "cyberpunk")]
    Info { name: "cyberpunk", title: "Cyberpunk 2077", steam: Some(1091500), make: cyberpunk::game },
    #[cfg(feature = "crimson")]
    Info { name: "crimson_desert", title: "Crimson Desert", steam: Some(3321460), make: crimson_desert::game },
    #[cfg(feature = "deadlock")]
    Info { name: "deadlock", title: "Deadlock", steam: Some(1422450), make: deadlock::game },
    #[cfg(feature = "wow")]
    Info { name: "wow_forever", title: "World of Warcraft (WoW Forever)", steam: None, make: wow_forever::game },
];

/// The CET bridge's folder inside the Cyberpunk install.
#[cfg(feature = "cyberpunk")]
pub fn cyberpunk_bridge() -> &'static str {
    cyberpunk::BRIDGE
}

#[cfg(not(feature = "cyberpunk"))]
pub fn cyberpunk_bridge() -> &'static str {
    ""
}

pub fn by_name(name: &str) -> Option<&'static Info> {
    ALL.iter().find(|g| g.name == name)
}

/// Refuse game names this build does not know.
pub fn check<'a>(names: impl IntoIterator<Item = &'a String>) -> Result<()> {
    for name in names {
        if by_name(name).is_none() {
            let known: Vec<&str> = ALL.iter().map(|g| g.name).collect();
            bail!("unknown game {name:?} (this build knows: {})", known.join(", "));
        }
    }
    Ok(())
}

/// The install folder of a game: the config's `path`, else found through Steam.
pub fn install(info: &Info, cfg: &Config) -> Option<std::path::PathBuf> {
    cfg.games.get(info.name).and_then(|g| g.path.clone()).or_else(|| info.steam.and_then(crate::steam::find_app))
}

/// A game's mode with this machine's paths and the config's overrides.
pub fn make(info: &Info, cfg: &Config) -> Game {
    let ctx = Ctx::new(info.name, install(info, cfg), &cfg.clips);
    let mut game = (info.make)(&ctx);
    if let Some(class) = cfg.games.get(info.name).and_then(|g| g.class.clone()) {
        game.class = class;
    }
    game
}

/// The games switched on in the config.
pub fn enabled(cfg: &Config) -> Vec<Game> {
    ALL.iter().filter(|g| cfg.games.get(g.name).is_some_and(|c| c.enabled)).map(|g| make(g, cfg)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Game> {
        let cfg = Config::default();
        ALL.iter().map(|g| make(g, &cfg)).collect()
    }

    #[test]
    fn fixed_pages_fill_the_keypad() {
        for g in all() {
            for p in g.pages.iter().filter(|p| !p.favs && !p.loadouts) {
                assert_eq!(p.keys.len(), 15, "{} {}", g.title, p.name);
            }
        }
    }

    #[test]
    fn off_unless_enabled() {
        let mut cfg = Config::default();
        assert!(enabled(&cfg).is_empty());
        cfg.games.insert(ALL[0].name.into(), crate::config::GameConfig { enabled: true, ..Default::default() });
        assert_eq!(enabled(&cfg).len(), 1);
        assert!(check([&"tetris".to_string()]).is_err());
    }
}

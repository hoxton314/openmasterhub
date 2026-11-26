//! The games that have a game mode: one file each, switched in the config's `[games]` table
//! (`name = false` turns one off). To remove a game for good: delete its file, its line in
//! `ALL` and its folder under `games/` in the repo.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::game::Game;

mod crimson_desert;
mod cyberpunk;
mod deadlock;
mod wow_forever;

const ALL: &[(&str, fn() -> Game)] = &[("cyberpunk", cyberpunk::game), ("crimson_desert", crimson_desert::game), ("deadlock", deadlock::game), ("wow_forever", wow_forever::game)];

/// Refuse names in `[games]` that no game has.
pub fn check(switches: &BTreeMap<String, bool>) -> Result<()> {
    for name in switches.keys() {
        if !ALL.iter().any(|(n, _)| n == name) {
            let known: Vec<&str> = ALL.iter().map(|(n, _)| *n).collect();
            bail!("[games]: unknown game {name:?} (known: {})", known.join(", "));
        }
    }
    Ok(())
}

/// A game by its `[games]` name.
pub fn by_name(name: &str) -> Option<Game> {
    ALL.iter().find(|(n, _)| *n == name).map(|(_, make)| make())
}

/// The games switched on: every known one unless the config says `name = false`.
pub fn enabled(switches: &BTreeMap<String, bool>) -> Vec<Game> {
    ALL.iter().filter(|(n, _)| switches.get(*n).copied().unwrap_or(true)).map(|(_, make)| make()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_pages_fill_the_keypad() {
        for g in enabled(&BTreeMap::new()) {
            for p in g.pages.iter().filter(|p| !p.favs && !p.loadouts) {
                assert_eq!(p.keys.len(), 15, "{} {}", g.title, p.name);
            }
        }
    }

    #[test]
    fn switches() {
        let off = BTreeMap::from([("crimson_desert".to_string(), false)]);
        let on: Vec<&str> = enabled(&off).iter().map(|g| g.title).collect();
        assert_eq!(on, ["CYBERPUNK", "DEADLOCK", "WOW FOREVER"]);
        assert!(check(&off).is_ok());
        assert!(check(&BTreeMap::from([("tetris".to_string(), true)])).is_err());
    }
}

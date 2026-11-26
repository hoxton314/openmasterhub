//! The game modes' config, `games.toml` next to the service's `config.toml`
//! (`~/.config/openmasterhub/games.toml`, `%APPDATA%\openmasterhub\games.toml`). Every game is
//! off until it is switched on here, by `masterhub-games setup <game>` or by the installer.
//!
//! ```toml
//! [deadlock]
//! enabled = true
//! path = "/games/SteamLibrary/steamapps/common/Deadlock"   # optional: found through Steam
//! class = "steam_app_1422450"                               # optional: the window to follow
//!
//! [clips]   # optional: commands for the SAVE CLIP, REPLAY and RECORD keys
//! save = "obs-cmd replay save"
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Unknown tables are caught by [`crate::games::check`] (serde can't deny them next to `flatten`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Clips::is_empty")]
    pub clips: Clips,
    /// Per game, by its name in `games::ALL`.
    #[serde(flatten)]
    pub games: BTreeMap<String, GameConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConfig {
    #[serde(default)]
    pub enabled: bool,
    /// The game's install folder; found through Steam when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// The window to follow (class glob, `class@title`, or an exe name on Windows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

/// Commands for the clip keys; a key without one shows that it is not set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clips {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<String>,
}

impl Clips {
    fn is_empty(&self) -> bool {
        self.save.is_none() && self.replay.is_none() && self.record.is_none()
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir().unwrap_or_default().join("openmasterhub").join("games.toml")
    }

    /// The config at `path`; a missing file is an empty config (every game off).
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let cfg: Self = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        crate::games::check(cfg.games.keys())?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, toml::to_string(self)?).with_context(|| format!("writing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_are_off_unless_enabled() {
        let cfg: Config = toml::from_str("[clips]\nsave = 'x'\n[deadlock]\nenabled = true\n[cyberpunk]\npath = '/g'\n").unwrap();
        assert!(cfg.games["deadlock"].enabled);
        assert!(!cfg.games["cyberpunk"].enabled);
        assert_eq!(cfg.clips.save.as_deref(), Some("x"));
        assert!(toml::from_str::<Config>("[deadlock]\nenabeld = true\n").is_err());
    }
}

//! Runtime state that outlives the service but is not configuration: the active setup and the
//! profile chosen by hand in each manual setup. Kept in `state.json` in the user's state
//! directory, so switching setups never rewrites `config.toml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// The active setup; the first one when unset or gone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<String>,
    /// Manual setups: the profile last chosen in each, by setup name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub manual: BTreeMap<String, String>,
}

/// `$XDG_STATE_HOME/openmasterhub/state.json` (`~/.local/state/…`), on Windows
/// `%LOCALAPPDATA%\openmasterhub\state.json`.
pub fn default_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let dir = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")));
    dir.map(|d| d.join("openmasterhub").join("state.json"))
}

impl State {
    /// The saved state; empty when there is none or it cannot be read.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("ignoring {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Write atomically (temp file, then rename).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_junk() {
        let dir = std::env::temp_dir().join(format!("omh-state-{}", std::process::id()));
        let path = dir.join("state.json");
        assert_eq!(State::load(&path), State::default());
        let s = State { setup: Some("Stream".into()), manual: [("Stream".into(), "Live".into())].into() };
        s.save(&path).unwrap();
        assert_eq!(State::load(&path), s);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(State::load(&path), State::default());
        // Unknown fields from a newer service are ignored.
        std::fs::write(&path, r#"{"setup":"Work","later":1}"#).unwrap();
        assert_eq!(State::load(&path).setup.as_deref(), Some("Work"));
        let _ = std::fs::remove_dir_all(dir);
    }
}

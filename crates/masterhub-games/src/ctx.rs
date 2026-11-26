//! What a game's definition needs from this machine: where the game and its extracted assets
//! are, and the user's clip commands.

use std::path::PathBuf;

use crate::config::Clips;
use crate::game::Press;

pub struct Ctx {
    /// The game's install folder, when known.
    pub install: Option<PathBuf>,
    /// Our folder for this game: assets extracted from the user's install, panel files.
    pub data: PathBuf,
    clips: [Option<&'static str>; 3],
}

#[derive(Clone, Copy)]
pub enum Clip {
    Save = 0,
    Replay = 1,
    Record = 2,
}

impl Ctx {
    pub fn new(name: &str, install: Option<PathBuf>, clips: &Clips) -> Self {
        let leak = |s: &Option<String>| s.clone().map(|s| &*Box::leak(s.into_boxed_str()));
        Self { install, data: data_dir(name), clips: [leak(&clips.save), leak(&clips.replay), leak(&clips.record)] }
    }

    /// A clip key's press: the configured command, or nothing when none is set.
    pub fn clip(&self, c: Clip) -> Press {
        self.clips[c as usize].map_or(Press::Unset, Press::Cmd)
    }
}

/// `~/.local/share/openmasterhub/games/<name>` (`%APPDATA%\openmasterhub\games\<name>`).
pub fn data_dir(name: &str) -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("openmasterhub").join("games").join(name)
}

/// The window class of a Steam game: `steam_app_<id>` under Proton, the exe name on Windows.
pub fn steam_class(app: u32, exe: &str) -> String {
    if cfg!(windows) { exe.to_owned() } else { format!("steam_app_{app}") }
}

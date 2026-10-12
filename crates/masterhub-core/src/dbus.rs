//! The service's D-Bus API on the session bus. Structured values travel as JSON strings so
//! the GUI and service share these Rust types without a D-Bus type mapping for each.
//!
//! Interface `org.openmasterhub.Daemon1` at `/org/openmasterhub/Daemon`:
//!
//! | Member | Signature | Meaning |
//! |---|---|---|
//! | `GetConfig()` | `→ s` | current config as TOML |
//! | `SetConfig(toml)` | `s →` | validate, save and apply; errors on invalid config |
//! | `GetModules()` | `→ s` | JSON `Vec<ModuleState>` |
//! | `GetState()` | `→ s` | JSON [`HubState`] |
//! | `GetDisplays()` | `→ a(syay)` | current JPEG of every display: (serial, target, jpeg) |
//! | `SetProfile(name)` | `s →` | switch profile (until the focused app changes) |
//! | `ShowPage(name)` | `s →` | show a keypad page or folder of the active profile |
//! | signal `Input(json)` | `s` | JSON [`InputMsg`] for every press/turn/move |
//! | signal `StateChanged(json)` | `s` | JSON [`HubState`] |
//! | signal `ModulesChanged(json)` | `s` | JSON `Vec<ModuleState>` |
//! | signal `DisplayChanged(serial, target, jpeg)` | `syay` | a display was redrawn |
//! | signal `ConfigChanged()` | | the config was replaced (by the GUI or on disk) |
//!
//! Display targets: 0..=14 keypad keys, [`TARGET_DIAL`] the dial.

use serde::{Deserialize, Serialize};

use crate::layout::Placement;

pub const BUS_NAME: &str = "org.openmasterhub.Daemon";
pub const OBJECT_PATH: &str = "/org/openmasterhub/Daemon";
pub const INTERFACE: &str = "org.openmasterhub.Daemon1";
pub const TARGET_DIAL: u8 = 255;
/// Application ID (and Wayland app_id / window class) of the GUI. Focusing it does not
/// switch profiles, so a profile can be edited while another app was focused before.
pub const GUI_APP_ID: &str = "org.openmasterhub.OpenMasterHub";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    /// `ModuleKind::name()`: base, keypad, fader, knob, roller, dial.
    pub kind: String,
    pub serial: String,
    /// USB socket below the Base's top hub, e.g. "22".
    pub socket: Option<String>,
    pub placement: Option<Placement>,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HubState {
    pub profile: String,
    /// Name of the keypad page or folder on screen.
    pub page: String,
    pub in_folder: bool,
    /// Base orientation reported by its sensor.
    pub horizontal: bool,
    /// Window class of the focused app (Hyprland), if known.
    pub active_app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMsg {
    pub serial: String,
    /// key, fader, knob, roller, dial
    pub control: String,
    /// Key 0..=14, fader 0..=4, knob 0..=2, roller 0 = left / 1 = right, dial 0.
    pub index: u8,
    /// down, up, cw, ccw, away, toward, move
    pub event: String,
    /// Fader position 0..=100.
    pub value: Option<u8>,
}

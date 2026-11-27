//! What the shell knows about the hub right now: the config, the live state and what the
//! service supports. The tray, the shortcuts and the switch overlay read it; it is refreshed
//! when the service connects, the config changes or the live profile does.

use std::time::Instant;

use masterhub_core::config::{Config, SetupMode};
use masterhub_core::ipc::{HubState, Reason};
use tauri::{AppHandle, Manager};

use crate::AppState;

#[derive(Default)]
pub struct Live {
    pub config: Option<Config>,
    pub state: Option<HubState>,
    pub capabilities: Vec<String>,
    /// How the last setup switch was asked for (a shortcut, the tray) and when.
    pub trigger: Option<(Instant, String)>,
}

/// `[app] overlay`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayMode {
    /// Only switches the user made (default).
    Switches,
    /// Also follow-mode changes caused by the focused window.
    All,
    Off,
}

impl Live {
    pub fn has(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// The `[app]` feedback switch, on unless the user turned it off.
    pub fn feedback(&self, pick: impl Fn(&masterhub_core::config::Feedback) -> Option<bool>) -> Option<bool> {
        self.config.as_ref().and_then(|c| c.app.feedback.as_ref()).and_then(pick)
    }

    /// `[app] overlay`: which switches get the toast. Unknown values mean the default.
    pub fn overlay_mode(&self) -> OverlayMode {
        match self.config.as_ref().and_then(|c| c.app.overlay.as_deref()) {
            Some("all") => OverlayMode::All,
            Some("off") => OverlayMode::Off,
            _ => OverlayMode::Switches,
        }
    }

    /// `[app] close_to_tray = false` keeps the window's close button quitting the app.
    pub fn close_to_tray(&self) -> bool {
        self.config.as_ref().and_then(|c| c.app.other.get("close_to_tray")).and_then(|v| v.as_bool()) != Some(false)
    }
}

/// "Work · Blender" and why it is live ("because blender is focused").
pub struct Description {
    pub title: String,
    pub reason: String,
}

pub fn describe(config: &Config, state: &HubState) -> Description {
    let setup = state.setup.clone().unwrap_or_default();
    let title = if setup.is_empty() { state.profile.clone() } else { format!("{setup} · {}", state.profile) };
    let setups = config.effective_setups();
    let active = setups.iter().find(|s| s.name == setup);
    let reason = match &state.reason {
        Some(Reason::Rule { pattern, .. }) => format!("because {} is focused", state.active_app.as_deref().unwrap_or(pattern)),
        Some(Reason::Fallback) => "because no rule matches".to_owned(),
        Some(Reason::Manual) => match active.filter(|s| s.mode() == SetupMode::Manual).and_then(|s| s.profiles.iter().position(|p| *p == state.profile).map(|i| (i, s.profiles.len()))) {
            Some((i, n)) => format!("switched by hand · {} of {n}", i + 1),
            None => "switched by hand".to_owned(),
        },
        Some(Reason::Pin) => "pinned".to_owned(),
        Some(Reason::Action) => "chosen by an action".to_owned(),
        None => String::new(),
    };
    Description { title, reason }
}

/// Fetch the config the way the UI's snapshot does.
pub fn fetch_config(client: &masterhub_core::ipc::Client) -> Option<Config> {
    let path = client.config_path().ok()?;
    let dir = std::path::Path::new(&path).parent().map(std::path::Path::to_owned).unwrap_or_else(|| std::path::PathBuf::from("."));
    let text = client.config().ok()?;
    Config::parse(&text, &dir).map_err(|e| log::warn!("config: {e:#}")).ok()
}

/// Reload what the service tells (config, state, capabilities) and bring the tray and the
/// shortcuts up to date.
pub fn refresh(app: &AppHandle) {
    let state = app.state::<AppState>();
    let online = state.client.is_online();
    let (config, hub, caps) = if online { (fetch_config(&state.client), state.client.state().ok(), state.client.capabilities()) } else { (None, None, Vec::new()) };
    {
        let mut live = state.live.lock().unwrap();
        // Keep the last good config when only the fetch failed.
        if config.is_some() || !online {
            live.config = config;
        }
        live.state = hub;
        live.capabilities = caps;
    }
    crate::shortcuts::sync(app);
    crate::tray::update(app);
}

/// The live state changed: show the switch if it was one, and refresh the tray.
pub fn on_state(app: &AppHandle, new: HubState) {
    let state = app.state::<AppState>();
    let prev = state.live.lock().unwrap().state.replace(new.clone());
    crate::overlay::on_change(app, prev.as_ref(), &new);
    crate::tray::update(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::parse(
            r#"
[[profile]]
name = "Desktop"
[[profile]]
name = "Live"
[[profile]]
name = "BRB"
[[setup]]
name = "Work"
fallback = "Desktop"
[[setup]]
name = "Stream"
mode = "manual"
profiles = ["Live", "BRB"]
"#,
            std::path::Path::new("."),
        )
        .unwrap()
    }

    #[test]
    fn overlay_mode_reads_the_app_table() {
        let mut live = Live::default();
        assert_eq!(live.overlay_mode(), OverlayMode::Switches);
        let with = |v: &str| Config::parse(&format!("[app]\noverlay = \"{v}\"\n"), std::path::Path::new(".")).ok();
        for (v, want) in [("all", OverlayMode::All), ("off", OverlayMode::Off), ("switches", OverlayMode::Switches), ("bogus", OverlayMode::Switches)] {
            live.config = with(v);
            assert_eq!(live.overlay_mode(), want, "{v}");
        }
    }

    #[test]
    fn describes_why_a_profile_is_live() {
        let c = config();
        let rule = HubState { profile: "Blender".into(), setup: Some("Work".into()), active_app: Some("blender".into()), reason: Some(Reason::Rule { pattern: "blender*".into(), index: 0 }), ..Default::default() };
        let d = describe(&c, &rule);
        assert_eq!((d.title.as_str(), d.reason.as_str()), ("Work · Blender", "because blender is focused"));
        let manual = HubState { profile: "BRB".into(), setup: Some("Stream".into()), reason: Some(Reason::Manual), ..Default::default() };
        assert_eq!(describe(&c, &manual).reason, "switched by hand · 2 of 2");
        let fallback = HubState { profile: "Desktop".into(), setup: Some("Work".into()), reason: Some(Reason::Fallback), ..Default::default() };
        assert_eq!(describe(&c, &fallback).reason, "because no rule matches");
        let legacy = HubState { profile: "Desktop".into(), ..Default::default() };
        assert_eq!(describe(&c, &legacy).title, "Desktop");
    }
}

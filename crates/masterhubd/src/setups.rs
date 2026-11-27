//! Setups engine: which profile the active setup makes live, and why. Pure logic; the
//! controller applies the result. Semantics (docs: `.design/PLAN.md` §4.2, IA-Precedence):
//!
//! - **follow**: the first listed profile whose `apps` match the focused window, else the
//!   fallback. A profile chosen by hand or by an action stays until the focus changes.
//! - **manual**: the profile chosen by hand (remembered per setup), else the first listed;
//!   focus changes do nothing.
//! - **hybrid**: follow, until a profile is pinned (by hand, a `profile` or `pin` action, or
//!   `pin`); the pin ends by its release policy.
//!
//! A config without `[[setup]]` has one implicit follow setup that picks exactly like
//! `Config::profile_for_app` (see `Config::effective_setups`).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use masterhub_core::config::{Config, ReleasePolicy, Setup, SetupMode, glob_match};
use masterhub_core::ipc::{PinState, Reason};

use crate::state::State;

/// Minutes a `release = "timeout"` pin lasts without `release_after`.
const DEFAULT_RELEASE_MINUTES: u32 = 10;

/// A hybrid setup's pin.
#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub setup: String,
    pub profile: String,
    /// Unix seconds.
    pub since: u64,
    pub at: Instant,
    /// The focused app when it was pinned (`release = "next_app"`).
    pub app: Option<String>,
}

#[derive(Debug, Default)]
pub struct Engine {
    /// The active setup as saved; resolve it with [`Engine::active`].
    pub setup: Option<String>,
    /// Manual setups: the profile chosen by hand, by setup.
    pub manual: BTreeMap<String, String>,
    pub pin: Option<Pin>,
}

impl Engine {
    pub fn from_state(state: &State) -> Self {
        Self { setup: state.setup.clone(), manual: state.manual.clone(), pin: None }
    }

    pub fn state(&self) -> State {
        State { setup: self.setup.clone(), manual: self.manual.clone() }
    }

    /// The active setup: the saved one if it exists, else the first.
    pub fn active(&self, config: &Config) -> Setup {
        let mut all = config.effective_setups();
        let i = all.iter().position(|s| Some(&s.name) == self.setup.as_ref()).unwrap_or(0);
        all.swap_remove(i)
    }

    /// After a config change: drop a pin whose setup or profile is gone.
    pub fn sync(&mut self, config: &Config) {
        let active = self.active(config);
        if self.pin.as_ref().is_some_and(|p| p.setup != active.name || active.mode() != SetupMode::Hybrid || config.profile(&p.profile).is_none()) {
            self.pin = None;
        }
    }

    /// The pin of the active setup, if it is hybrid and pinned.
    pub fn pin_of(&self, setup: &Setup) -> Option<&Pin> {
        self.pin.as_ref().filter(|p| p.setup == setup.name && setup.mode() == SetupMode::Hybrid)
    }

    pub fn pin_state(&self, config: &Config) -> Option<PinState> {
        let p = self.pin_of(&self.active(config))?;
        Some(PinState { profile: p.profile.clone(), since: Some(p.since) })
    }

    /// The profile the active setup makes live now, and why; `None` when it names none.
    pub fn decide(&self, config: &Config, app: Option<&str>) -> Option<(String, Reason)> {
        let s = self.active(config);
        match s.mode() {
            SetupMode::Manual => {
                let chosen = self.manual.get(&s.name).filter(|p| config.profile(p).is_some());
                let name = chosen.or_else(|| s.profiles.first())?;
                Some((name.clone(), Reason::Manual))
            }
            SetupMode::Hybrid if self.pin_of(&s).is_some() => Some((self.pin_of(&s)?.profile.clone(), Reason::Pin)),
            SetupMode::Follow | SetupMode::Hybrid => follow(config, &s, app),
        }
    }

    /// Whether a focus change to `app` ends the pin (`release = "next_app"`).
    pub fn focus_releases(&self, config: &Config, app: Option<&str>) -> bool {
        let s = self.active(config);
        self.pin_of(&s).is_some_and(|p| release_policy(&s) == ReleasePolicy::NextApp && p.app.as_deref() != app)
    }

    /// Whether the pin has run out (`release = "timeout"`).
    pub fn pin_expired(&self, config: &Config, now: Instant) -> bool {
        let s = self.active(config);
        self.pin_of(&s).is_some_and(|p| {
            release_policy(&s) == ReleasePolicy::Timeout
                && now.duration_since(p.at) >= Duration::from_secs(u64::from(s.release_after.unwrap_or(DEFAULT_RELEASE_MINUTES)) * 60)
        })
    }
}

fn release_policy(s: &Setup) -> ReleasePolicy {
    s.release.unwrap_or_default()
}

/// Follow and hybrid setups: the first listed profile with a pattern matching `app`, else the
/// fallback. With `app` unknown, the fallback.
pub fn follow(config: &Config, s: &Setup, app: Option<&str>) -> Option<(String, Reason)> {
    if let Some(app) = app {
        for (index, name) in s.profiles.iter().enumerate() {
            let Some(p) = config.profile(name) else { continue };
            if let Some(pattern) = p.apps.iter().find(|pat| glob_match(pat, app)) {
                return Some((name.clone(), Reason::Rule { pattern: pattern.clone(), index }));
            }
        }
    }
    s.fallback.clone().filter(|f| config.profile(f).is_some()).map(|f| (f, Reason::Fallback))
}

/// The profiles `profile next|prev|pick` steps through: the list, then the fallback.
pub fn cycle_list(s: &Setup) -> Vec<String> {
    let mut v = s.profiles.clone();
    if s.mode() != SetupMode::Manual
        && let Some(f) = &s.fallback
        && !v.contains(f)
    {
        v.push(f.clone());
    }
    v
}

/// The item `step` (+1 next, -1 prev) away from `current` in `list`; from outside the list,
/// next is the first and prev the last. `None` past either end without `wrap`.
pub fn step(list: &[String], current: &str, step: i32, wrap: bool) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i64;
    let i = match list.iter().position(|x| x == current) {
        Some(i) => i as i64 + i64::from(step),
        None if step > 0 => 0,
        None => n - 1,
    };
    let i = if wrap { i.rem_euclid(n) } else if (0..n).contains(&i) { i } else { return None };
    Some(list[i as usize].clone())
}

/// Whether `profile next|prev` wraps in this setup: `wrap`, default on except in manual setups.
pub fn wraps(s: &Setup) -> bool {
    s.wrap.unwrap_or(s.mode() != SetupMode::Manual)
}

/// The setup `step` away from `current` (always wrapping).
pub fn step_setup(config: &Config, current: &str, n: i32) -> Option<String> {
    let names: Vec<String> = config.effective_setups().into_iter().map(|s| s.name).collect();
    step(&names, current, n, true)
}

/// A short "why" for cards and logs, e.g. "by app", "fallback".
pub fn reason_text(r: Option<&Reason>) -> &'static str {
    match r {
        Some(Reason::Rule { .. }) => "by app",
        Some(Reason::Fallback) => "fallback",
        Some(Reason::Manual) => "by hand",
        Some(Reason::Pin) => "pinned",
        Some(Reason::Action) | None => "switched",
    }
}

pub fn mode_title(m: SetupMode) -> &'static str {
    match m {
        SetupMode::Follow => "Follow",
        SetupMode::Manual => "Manual",
        SetupMode::Hybrid => "Hybrid",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use masterhub_core::config::{ControlRef, PageView, Side, Source};

    /// The IA-Precedence scenarios, as `app/src/lib/model/fixtures.ts`.
    pub(crate) const FIXTURE: &str = r##"
backlight = 80

[[pinned]]
index = 8
label = "Claude"
widget = { kind = "claude_session" }
except_setups = ["Play"]

[[setup]]
name = "Work"
mode = "follow"
profiles = ["Blender", "Godot", "Ghostty", "Claude"]
fallback = "Desktop"

[[setup]]
name = "Stream"
mode = "manual"
profiles = ["Scenes", "Desktop"]
wrap = true

[[setup]]
name = "Play"
mode = "hybrid"
profiles = ["Cyberpunk 2077", "Discord"]
fallback = "Desktop light"
release = "next_app"

[[profile]]
name = "Desktop"
dial = { label = "Volume", cw = { volume = "+2" } }
[[profile.page]]
name = "Main"
key = [{ index = 0, label = "Files", press = { keys = "ctrl+s" } }, { index = 14, label = "Apps", press = { keys = "ctrl+s" } }]
[[profile.knob]]
index = 0
cw = { volume = "+2" }
[[profile.knob]]
index = 1
cw = { volume = "+2" }
[[profile.knob]]
index = 2
cw = { volume = "+2" }
[[profile.roller]]
side = "left"
away = { page = "next" }
[[profile.roller]]
side = "right"
away = { keys = "ctrl+z" }

[[profile]]
name = "Desktop light"
base = "Desktop"
[[profile.page]]
name = "Main"
[[profile.roller]]
side = "left"
away = { page = "prev" }

[[profile]]
name = "Blender"
apps = ["blender"]
base = "Desktop"
dial = { label = "Zoom view", cw = { keys = "ctrl+plus" } }
[[profile.page]]
name = "Views"
key = [
  { index = 0, label = "B1", press = { keys = "ctrl+s" } }, { index = 1, label = "B2", press = { keys = "ctrl+s" } },
  { index = 2, label = "B3", press = { keys = "ctrl+s" } }, { index = 3, label = "B4", press = { keys = "ctrl+s" } },
  { index = 4, label = "B5", press = { keys = "ctrl+s" } }, { index = 5, label = "B6", press = { keys = "ctrl+s" } },
  { index = 6, label = "B7", press = { keys = "ctrl+s" } }, { index = 7, label = "B8", press = { keys = "ctrl+s" } },
  { index = 8, label = "B9", press = { keys = "ctrl+s" } }, { index = 9, label = "B10", press = { keys = "ctrl+s" } },
  { index = 10, label = "B11", press = { keys = "ctrl+s" } }, { index = 11, label = "B12", press = { keys = "ctrl+s" } },
  { index = 12, label = "B13", press = { keys = "ctrl+s" } }, { index = 13, label = "B14", press = { keys = "ctrl+s" } },
  { index = 14, label = "B15", press = { keys = "ctrl+s" } },
]
[[profile.page]]
name = "Modeling"

[[profile]]
name = "Godot"
apps = ["godot", "Godot*.exe"]
base = "Desktop"
[[profile.page]]
name = "Main"

[[profile]]
name = "Ghostty"
apps = ["com.mitchellh.ghostty"]
[[profile.page]]
name = "Main"

[[profile]]
name = "Claude"
apps = ["claude-*", "com.mitchellh.ghostty"]
[[profile.page]]
name = "Main"

[[profile]]
name = "Scenes"
apps = ["obs"]
[[profile.page]]
name = "Scenes"
key = [{ index = 0, label = "Live", press = { keys = "ctrl+s" } }]
[[profile.knob]]
index = 0
cw = { volume = "+2" }

[[profile]]
name = "Cyberpunk 2077"
apps = ["steam_app_1091500"]
base = "Desktop light"
[[profile.page]]
name = "Combat"
[[profile.knob]]
index = 0
cw = { keys = "f" }
[[profile.knob]]
index = 1
cw = { keys = "f" }
[[profile.knob]]
index = 2
cw = { keys = "f" }

[[profile]]
name = "Discord"
apps = ["discord", "Discord.exe"]
[[profile.page]]
name = "Main"
"##;

    pub(crate) fn fixture() -> Config {
        Config::parse(FIXTURE, std::path::Path::new(".")).unwrap()
    }

    fn engine(setup: &str) -> Engine {
        Engine { setup: Some(setup.into()), ..Default::default() }
    }

    fn pin(e: &mut Engine, setup: &str, profile: &str, app: Option<&str>) {
        e.pin = Some(Pin { setup: setup.into(), profile: profile.into(), since: 0, at: Instant::now(), app: app.map(Into::into) });
    }

    fn name(s: &Source) -> Option<String> {
        match s {
            Source::Own(p) | Source::Base { profile: p, .. } => Some(p.name.clone()),
            Source::Pinned(p) => p.key.face.label.clone(),
            Source::None => None,
        }
    }

    #[test]
    fn legacy_config_picks_like_profile_for_app() {
        let c = Config::parse(
            r#"
[[profile]]
name = "Default"
[[profile]]
name = "Streaming"
apps = ["obs", "com.obsproject.Studio"]
[[profile]]
name = "Shell"
apps = ["*ghostty"]
"#,
            std::path::Path::new("."),
        )
        .unwrap();
        let e = Engine::default();
        assert_eq!(e.active(&c).name, "Default");
        for app in ["obs", "OBS", "kitty", "com.mitchellh.ghostty", ""] {
            let (name, _) = e.decide(&c, Some(app)).unwrap();
            assert_eq!(name, c.profiles[c.profile_for_app(app)].name, "{app}");
        }
        assert_eq!(e.decide(&c, Some("com.mitchellh.ghostty")).unwrap().1, Reason::Rule { pattern: "*ghostty".into(), index: 1 });
        assert_eq!(e.decide(&c, None), Some(("Default".into(), Reason::Fallback)));
        // A saved setup that does not exist (any more) falls back to the first.
        assert_eq!(engine("Gone").active(&c).name, "Default");
    }

    #[test]
    fn follow_first_match_wins_then_fallback() {
        let c = fixture();
        let e = engine("Work");
        assert_eq!(e.decide(&c, Some("blender")), Some(("Blender".into(), Reason::Rule { pattern: "blender".into(), index: 0 })));
        // Ghostty shadows Claude (both match ghostty).
        assert_eq!(e.decide(&c, Some("com.mitchellh.ghostty")).unwrap().0, "Ghostty");
        assert_eq!(e.decide(&c, Some("claude-code")).unwrap().1, Reason::Rule { pattern: "claude-*".into(), index: 3 });
        assert_eq!(e.decide(&c, Some("godot_v4.3.exe")).unwrap().0, "Godot");
        // Discord is only in Play.
        assert_eq!(e.decide(&c, Some("discord")), Some(("Desktop".into(), Reason::Fallback)));
    }

    #[test]
    fn manual_ignores_focus_and_remembers() {
        let c = fixture();
        let mut e = engine("Stream");
        assert_eq!(e.decide(&c, Some("obs")), Some(("Scenes".into(), Reason::Manual)));
        e.manual.insert("Stream".into(), "Desktop".into());
        assert_eq!(e.decide(&c, Some("obs")), Some(("Desktop".into(), Reason::Manual)));
        // A remembered profile that was deleted: the first again.
        e.manual.insert("Stream".into(), "Gone".into());
        assert_eq!(e.decide(&c, None).unwrap().0, "Scenes");
        let s = e.active(&c);
        let list = cycle_list(&s);
        assert_eq!(list, ["Scenes", "Desktop"]);
        assert!(wraps(&s));
        assert_eq!(step(&list, "Desktop", 1, true).as_deref(), Some("Scenes"));
        assert_eq!(step(&list, "Scenes", -1, false), None);
        assert_eq!(step(&list, "Claude", 1, false).as_deref(), Some("Scenes"));
        assert_eq!(step(&list, "Claude", -1, false).as_deref(), Some("Desktop"));
    }

    #[test]
    fn hybrid_pins_and_releases() {
        let c = fixture();
        let mut e = engine("Play");
        assert_eq!(e.decide(&c, Some("discord")).unwrap().0, "Discord");
        assert_eq!(e.decide(&c, Some("kitty")), Some(("Desktop light".into(), Reason::Fallback)));
        pin(&mut e, "Play", "Discord", Some("kitty"));
        assert_eq!(e.decide(&c, Some("steam_app_1091500")), Some(("Discord".into(), Reason::Pin)));
        assert_eq!(e.pin_state(&c).unwrap().profile, "Discord");
        // next_app: the same app does not end it, another one does.
        assert!(!e.focus_releases(&c, Some("kitty")));
        assert!(e.focus_releases(&c, Some("steam_app_1091500")));
        assert!(!e.pin_expired(&c, Instant::now() + Duration::from_secs(86400)));
        // A pin of another setup does not apply.
        e.setup = Some("Work".into());
        assert_eq!(e.decide(&c, Some("blender")).unwrap().0, "Blender");
        assert!(e.pin_state(&c).is_none());
        e.sync(&c);
        assert!(e.pin.is_none());
        // The cycle list ends with the fallback.
        assert_eq!(cycle_list(&engine("Play").active(&c)), ["Cyberpunk 2077", "Discord", "Desktop light"]);
    }

    #[test]
    fn timeout_and_same_trigger_policies() {
        let mut c = fixture();
        c.setups[2].release = Some(ReleasePolicy::Timeout);
        c.setups[2].release_after = Some(2);
        let mut e = engine("Play");
        pin(&mut e, "Play", "Discord", None);
        let at = e.pin.as_ref().unwrap().at;
        assert!(!e.pin_expired(&c, at + Duration::from_secs(119)));
        assert!(e.pin_expired(&c, at + Duration::from_secs(120)));
        assert!(!e.focus_releases(&c, Some("anything")));
        c.setups[2].release = None; // same_trigger: only the trigger (or release_pin) ends it
        assert!(!e.pin_expired(&c, at + Duration::from_secs(86400)));
        assert!(!e.focus_releases(&c, Some("anything")));
    }

    #[test]
    fn setups_cycle() {
        let c = fixture();
        assert_eq!(step_setup(&c, "Play", 1).as_deref(), Some("Work"));
        assert_eq!(step_setup(&c, "Work", -1).as_deref(), Some("Play"));
    }

    // The IA-Precedence scenarios (app/src/lib/model/model.test.ts).

    #[test]
    fn precedence_work_blender_focused() {
        let c = fixture();
        let src = |ctl| c.source_of("Blender", PageView::Page(0), Some("Work"), ctl);
        for k in [0, 1, 7, 9, 14] {
            assert!(matches!(src(ControlRef::Key(k)), Source::Own(p) if p.name == "Blender"));
        }
        assert!(matches!(src(ControlRef::Key(8)), Source::Pinned(_)));
        assert!(matches!(src(ControlRef::Dial), Source::Own(p) if p.name == "Blender"));
        assert_eq!(name(&src(ControlRef::Knob(1))).as_deref(), Some("Desktop"));
        assert_eq!(name(&src(ControlRef::Roller(Side::Right))).as_deref(), Some("Desktop"));
        // Resolved configs: pinned key, own dial, inherited knob.
        assert_eq!(c.key_at("Blender", PageView::Page(0), Some("Work"), 8).unwrap().face.label.as_deref(), Some("Claude"));
        assert_eq!(c.dial_of("Blender").unwrap().face.label.as_deref(), Some("Zoom view"));
        assert!(c.knob_of("Blender", 1).unwrap().cw.is_some());
        // Page 2 (Modeling) sets nothing: page 2 of Desktop does not exist, so nothing.
        assert_eq!(c.key_at("Blender", PageView::Page(1), Some("Work"), 0), None);
    }

    #[test]
    fn precedence_play_cyberpunk_focused() {
        let c = fixture();
        let src = |ctl| c.source_of("Cyberpunk 2077", PageView::Page(0), Some("Play"), ctl);
        // Key 9 is excluded from Play and nothing below sets it.
        assert_eq!(src(ControlRef::Key(8)), Source::None);
        assert_eq!(c.key_at("Cyberpunk 2077", PageView::Page(0), Some("Play"), 8), None);
        // But it is pinned in the same profile seen from another setup.
        assert!(matches!(c.source_of("Cyberpunk 2077", PageView::Page(0), Some("Work"), ControlRef::Key(8)), Source::Pinned(_)));
        assert!(matches!(src(ControlRef::Knob(2)), Source::Own(p) if p.name == "Cyberpunk 2077"));
        assert_eq!(name(&src(ControlRef::Roller(Side::Left))).as_deref(), Some("Desktop light"));
        match src(ControlRef::Roller(Side::Right)) {
            Source::Base { profile, via } => {
                assert_eq!(profile.name, "Desktop");
                assert_eq!(via, ["Desktop light", "Desktop"]);
            }
            s => panic!("{s:?}"),
        }
        // Inherited by page position: Combat (page 1) shows Desktop's Main keys.
        assert_eq!(c.key_at("Cyberpunk 2077", PageView::Page(0), Some("Play"), 0).unwrap().face.label.as_deref(), Some("Files"));
        assert_eq!(c.dial_of("Cyberpunk 2077").unwrap().face.label.as_deref(), Some("Volume"));
    }

    #[test]
    fn precedence_stream_chosen_by_hand() {
        let c = fixture();
        let e = engine("Stream");
        assert_eq!(e.decide(&c, Some("blender")).unwrap(), ("Scenes".into(), Reason::Manual));
        assert!(matches!(c.source_of("Scenes", PageView::Page(0), Some("Stream"), ControlRef::Key(0)), Source::Own(p) if p.name == "Scenes"));
        // No base: unset controls are nobody's.
        assert_eq!(c.source_of("Scenes", PageView::Page(0), Some("Stream"), ControlRef::Dial), Source::None);
    }

    #[test]
    fn leds_inherit_per_field() {
        let mut c = fixture();
        use masterhub_core::config::{Color, Leds};
        c.leds.brightness = Some(50);
        c.profiles[0].leds = Some(Leds { base: Color::parse("#ff0000"), ..Default::default() });
        c.profiles[1].leds = Some(Leds { brightness: Some(20), ..Default::default() });
        let l = c.leds_of("Cyberpunk 2077");
        assert_eq!((l.base, l.brightness), (Color::parse("#ff0000"), Some(20)));
        assert_eq!(c.leds_of("Scenes").brightness, Some(50));
    }
}

//! TOML configuration, shared by the service (reads it) and the GUI (edits it).
//! See `examples/config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use masterhub_proto::Rgb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Settings of the desktop app (theme, density, …). The service ignores them.
    #[serde(default, skip_serializing_if = "AppSettings::is_empty")]
    pub app: AppSettings,
    /// TTF/OTF font for labels. Defaults to `fc-match sans:bold`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<PathBuf>,
    /// LCD backlight of keypad and dial, 0..=100.
    #[serde(default = "default_backlight")]
    pub backlight: u8,
    #[serde(default, skip_serializing_if = "Leds::is_empty")]
    pub leds: Leds,
    /// Base LED colours for tool status, such as Claude Code sessions (`masterhubd claude-hook`).
    #[serde(default, skip_serializing_if = "StatusLeds::is_default")]
    pub status: StatusLeds,
    /// How the Base is placed on the desk. `auto` follows its tilt sensor.
    #[serde(default, skip_serializing_if = "Orientation::is_auto")]
    pub orientation: Orientation,
    /// Turn key and dial images so they read upright from the user's side, whatever way the
    /// Base and the modules are rotated.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub upright: bool,
    /// Keys shown at the same place on every keypad page and folder of every profile, over
    /// the page's own key there, e.g. a key that opens the Claude profile from anywhere.
    #[serde(default, rename = "pinned", skip_serializing_if = "Vec::is_empty")]
    pub pinned: Vec<PinnedKey>,
    /// Sets of profiles the user switches between (see [`Setup`]). None: one implicit setup,
    /// see [`Config::effective_setups`].
    #[serde(default, rename = "setup", skip_serializing_if = "Vec::is_empty")]
    pub setups: Vec<Setup>,
    /// The first profile is the default; it is used when no other profile matches the
    /// focused application.
    #[serde(default, rename = "profile", skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<Profile>,
    /// Named sets of apps that one `volume` or `mute` action controls together.
    #[serde(default, rename = "audio_group", skip_serializing_if = "Vec::is_empty")]
    pub audio_groups: Vec<AudioGroup>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppSettings::default(),
            font: None,
            backlight: default_backlight(),
            leds: Leds::default(),
            status: StatusLeds::default(),
            orientation: Orientation::Auto,
            upright: true,
            pinned: Vec::new(),
            setups: Vec::new(),
            profiles: Vec::new(),
            audio_groups: Vec::new(),
        }
    }
}

fn default_backlight() -> u8 {
    80
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

/// `[app]`: settings of the desktop app, kept in the same file so they travel with it. The
/// service parses and ignores them; keys it does not know are kept as they are.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    /// A theme id (bundled or user), or `"system"`: follow the OS between `theme_dark` and
    /// `theme_light`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_dark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_light: Option<String>,
    /// `compact` or `comfortable`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<String>,
    /// `system`, `on` or `off`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_motion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_run_done: Option<bool>,
    /// How setup and profile switches are shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<Feedback>,
    /// Which switches get the toast: `switches` (default; the ones the user made), `all`
    /// (also follow-mode changes caused by the focused window) or `off`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<String>,
    /// Settings of newer app versions, kept unchanged.
    #[serde(flatten)]
    pub other: toml::Table,
}

impl AppSettings {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// `[app.feedback]`: what the app shows on a switch.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Feedback {
    /// A toast near the tray (off unless `true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toast: Option<bool>,
    /// A card on the dial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dial: Option<bool>,
    /// The Base LED blinks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub led: Option<bool>,
    /// Also while a fullscreen window is focused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_fullscreen: Option<bool>,
    #[serde(flatten)]
    pub other: toml::Table,
}

/// The name of the setup a config without `[[setup]]` has.
pub const IMPLICIT_SETUP: &str = "Default";

/// `[[setup]]`: a set of profiles the user switches between, e.g. Work, Stream, Play. A profile
/// listed in several setups is the same profile (linked).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub name: String,
    /// Default: follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<SetupMode>,
    /// Profiles in priority (follow, hybrid) or cycle (manual) order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
    /// Follow and hybrid: the profile used when no rule matches; never one of `profiles`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
    /// LED blink, dial card ring and UI colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Global shortcut that switches to this setup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    /// Show a dial card on profile changes in this setup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_on_dial: Option<bool>,
    /// Manual: after the last profile comes the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    /// Manual: global shortcut for the next profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// Manual: global shortcut for the previous profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    /// Hybrid: global shortcut that pins the live profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    /// Hybrid: when a pin ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<ReleasePolicy>,
    /// Hybrid, `release = "timeout"`: minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_after: Option<u32>,
}

impl Setup {
    pub fn mode(&self) -> SetupMode {
        self.mode.unwrap_or_default()
    }

    /// Global shortcuts this setup defines (empty strings left out).
    pub fn shortcuts(&self) -> impl Iterator<Item = &str> {
        [&self.shortcut, &self.next, &self.prev, &self.pin].into_iter().flatten().map(String::as_str).filter(|s| !s.trim().is_empty())
    }
}

/// How a setup picks its live profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupMode {
    /// The first profile whose apps match the focused window, else the fallback.
    #[default]
    Follow,
    /// The user steps through the profiles.
    Manual,
    /// Follow, until the user pins a profile.
    Hybrid,
}

/// When a hybrid setup's pin ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePolicy {
    /// The pin shortcut or action again.
    #[default]
    SameTrigger,
    /// After `release_after` minutes.
    Timeout,
    /// When another app is focused.
    NextApp,
}

/// How the Base lies on the desk, as seen by the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Follow the Base's tilt sensor when it reports; landscape until then. The sensor is
    /// silent while the Base lies flat, so in practice this is often plain landscape.
    #[default]
    Auto,
    Portrait,
    Landscape,
    PortraitFlipped,
    LandscapeFlipped,
}

impl Orientation {
    pub const ALL: [Orientation; 5] =
        [Self::Auto, Self::Portrait, Self::Landscape, Self::PortraitFlipped, Self::LandscapeFlipped];

    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Portrait => "Portrait",
            Self::Landscape => "Landscape",
            Self::PortraitFlipped => "Portrait, upside down",
            Self::LandscapeFlipped => "Landscape, upside down",
        }
    }

    /// Clockwise rotation (0/90/180/270) from the Base's vertical frame to the user's view.
    /// `sensor` is the Base's "horizontal" report, if it sent one.
    pub fn rotation(self, sensor: Option<bool>) -> u16 {
        match self {
            Self::Portrait => 0,
            Self::Landscape => 90,
            Self::PortraitFlipped => 180,
            Self::LandscapeFlipped => 270,
            Self::Auto => match sensor {
                Some(false) => 0,
                _ => 90,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    /// Window classes that activate this profile (case-insensitive, `*` wildcards),
    /// e.g. `["obs", "steam_app_*"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<String>,
    /// Controls this profile does not set come from this profile (by page position, folder
    /// name, index or side).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Avatar colour in the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Description shown in the app (templates, cards).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Listed under Games in the app (`None`: when one of `apps` is a Steam game,
    /// `steam_app_*`). Only for the app; the service treats every profile alike.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<bool>,
    /// Overrides the global LED colours while this profile is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leds: Option<Leds>,
    /// Keypad pages, cycled with `page = "next"` / `"prev"`.
    #[serde(default, rename = "page", skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<Page>,
    /// Keypad folders, opened with `folder = "<name>"` and left with `back`.
    #[serde(default, rename = "folder", skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<Page>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dial: Option<DialConfig>,
    #[serde(default, rename = "fader", skip_serializing_if = "Vec::is_empty")]
    pub faders: Vec<FaderConfig>,
    #[serde(default, rename = "knob", skip_serializing_if = "Vec::is_empty")]
    pub knobs: Vec<KnobConfig>,
    #[serde(default, rename = "roller", skip_serializing_if = "Vec::is_empty")]
    pub rollers: Vec<RollerConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub name: String,
    #[serde(default, rename = "key", skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<KeyConfig>,
}

impl Page {
    pub fn key(&self, index: u8) -> Option<&KeyConfig> {
        self.keys.iter().find(|k| k.index == index)
    }
}

/// Static LED colours. One colour applies to every LED of that module; a list sets them
/// individually (fader: top to bottom, knob: bottom-left, bottom-right, top).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fader: Option<OneOrMany<Color>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knob: Option<OneOrMany<Color>>,
    /// 1..=100
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    /// Knob rings show the volume level of their turn action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knob_volume: Option<bool>,
    /// Knob rings turn red while their volume target is muted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knob_mute_red: Option<bool>,
}

impl Leds {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Base LED colours while a tool reports a status (see `ipc::Status`). The most urgent status
/// of all sources wins; with none, the Base shows its normal colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusLeds {
    pub enabled: bool,
    /// Something waits for an answer; flashes.
    pub attention: Color,
    /// Finished, waiting for a new prompt.
    pub ready: Color,
    /// Busy.
    pub working: Color,
    /// How session keys are ordered.
    #[serde(skip_serializing_if = "SessionSort::is_default")]
    pub sort: SessionSort,
    /// The Base LED blinks twice in the new setup's colour when the setup changes.
    #[serde(skip_serializing_if = "is_true")]
    pub blink_on_setup: bool,
}

/// The order Claude Code sessions fill the `claude_session` slots in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSort {
    /// Oldest first, so keys don't move around.
    Started,
    /// Most recent status change first.
    Changed,
    /// Waiting for you first, then ready, then working; most recent change first within each.
    #[default]
    Urgent,
}

impl SessionSort {
    pub const ALL: [SessionSort; 3] = [Self::Started, Self::Changed, Self::Urgent];

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for StatusLeds {
    fn default() -> Self {
        Self {
            enabled: true,
            attention: Color(Rgb::new(0xff, 0x40, 0x00)),
            ready: Color(Rgb::new(0x00, 0xff, 0x40)),
            working: Color(Rgb::new(0x00, 0x60, 0xff)),
            sort: SessionSort::Urgent,
            blink_on_setup: true,
        }
    }
}

impl StatusLeds {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T: Clone> OneOrMany<T> {
    pub fn get(&self, i: usize) -> Option<T> {
        match self {
            Self::One(v) => Some(v.clone()),
            Self::Many(v) => v.get(i).cloned(),
        }
    }
}

/// Look of a display: background colour, optional picture or animated GIF, optional label,
/// optional live widget.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// PNG/JPEG, or a GIF which is animated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<FaceWidget>,
    /// How the image is placed on the screen (default: fit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<ImageFit>,
    /// Draw the label over the image instead of below it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_over_image: Option<bool>,
}

/// How a face's image is placed on its screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    /// Whole image visible.
    #[default]
    Fit,
    /// Cover the screen, cropping.
    Fill,
    /// Unscaled, centred.
    Center,
}

/// A face's live widget: `{ kind = "cpu_usage", refresh = 2 }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaceWidget {
    #[serde(flatten)]
    pub widget: Widget,
    /// Seconds between updates (default: the widget's own rate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<u16>,
}

impl From<Widget> for FaceWidget {
    fn from(widget: Widget) -> Self {
        Self { widget, refresh: None }
    }
}

impl std::ops::Deref for FaceWidget {
    type Target = Widget;
    fn deref(&self) -> &Widget {
        &self.widget
    }
}

/// Live data drawn on a key or the dial.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    /// strftime-style format, default `%H:%M`.
    Clock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
    CpuUsage,
    CpuTemp,
    RamUsage,
    GpuUsage,
    GpuTemp,
    /// Default sink volume.
    Volume,
    /// MPRIS: title, artist and cover art of the active player.
    NowPlaying,
    /// The Claude Code session in this slot (0 = the first in `[status] sort` order): its name and status.
    /// Empty slots show the face as configured.
    ClaudeSession {
        #[serde(default)]
        slot: u8,
    },
}

impl Widget {
    pub const ALL: [Widget; 9] = [
        Widget::Clock { format: None },
        Widget::CpuUsage,
        Widget::CpuTemp,
        Widget::RamUsage,
        Widget::GpuUsage,
        Widget::GpuTemp,
        Widget::Volume,
        Widget::NowPlaying,
        Widget::ClaudeSession { slot: 0 },
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Self::Clock { .. } => "Clock",
            Self::CpuUsage => "CPU usage",
            Self::CpuTemp => "CPU temperature",
            Self::RamUsage => "RAM usage",
            Self::GpuUsage => "GPU usage",
            Self::GpuTemp => "GPU temperature",
            Self::Volume => "Volume",
            Self::NowPlaying => "Now playing",
            Self::ClaudeSession { .. } => "Claude Code session",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyConfig {
    /// 0..=14, row-major from the top-left.
    pub index: u8,
    #[serde(flatten)]
    pub face: Face,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Action>,
}

impl KeyConfig {
    pub fn new(index: u8) -> Self {
        Self { index, face: Face::default(), press: None, release: None }
    }
}

/// A key on every page (`Config::pinned`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinnedKey {
    #[serde(flatten)]
    pub key: KeyConfig,
    /// Profiles that show their own key there instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
    /// Setups whose profiles all show their own key there instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except_setups: Vec<String>,
}

impl Config {
    /// The key at `index` on a page of `profile`: a pinned key, else the page's own.
    pub fn key<'a>(&'a self, profile: &str, page: &'a Page, index: u8) -> Option<&'a KeyConfig> {
        self.pinned
            .iter()
            .find(|p| p.key.index == index && !p.except.iter().any(|e| e == profile))
            .map(|p| &p.key)
            .or_else(|| page.key(index))
    }
}

// ───────────────────────────── precedence ─────────────────────────────
//
// Which config a control uses, per IA-Precedence (top wins): a live-surface claim (the
// service's business), a pinned key unless the profile or its setup is excluded, the live
// profile if it sets the control, then its `base` profiles, nearest first. Mirrors
// `ownerOf` in app/src/lib/model/precedence.ts; keep the two in step.

/// One control of the hub, for [`Config::source_of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRef {
    Key(u8),
    Dial,
    Knob(u8),
    Fader(u8),
    Roller(Side),
}

/// Which keypad page of a profile is on screen: by position among its pages, or a folder by
/// name. A base profile shows its page at the same position, or its folder of that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageView<'a> {
    Page(usize),
    Folder(&'a str),
}

/// Where a control's config comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source<'a> {
    /// A pinned key.
    Pinned(&'a PinnedKey),
    /// The profile itself.
    Own(&'a Profile),
    /// A base profile; `via` are the profiles walked, nearest first, ending with it.
    Base { profile: &'a Profile, via: Vec<&'a str> },
    /// Nothing sets it.
    None,
}

impl Face {
    /// Any field set (an empty label counts as unset).
    pub fn is_set(&self) -> bool {
        self.label.as_deref().is_some_and(|l| !l.is_empty())
            || self.image.is_some()
            || self.background.is_some()
            || self.text_color.is_some()
            || self.widget.is_some()
            || self.fit.is_some()
            || self.label_over_image.is_some()
    }
}

impl KeyConfig {
    /// Sets anything beyond its index; an entry that does not falls through to the base.
    pub fn is_set(&self) -> bool {
        self.press.is_some() || self.release.is_some() || self.face.is_set()
    }
}

impl DialConfig {
    pub fn is_set(&self) -> bool {
        self.face.is_set() || [&self.cw, &self.ccw, &self.press, &self.release, &self.long_press].iter().any(|a| a.is_some())
    }
}

impl PinnedKey {
    /// Shown in `profile` while `setup` is active.
    pub fn applies(&self, profile: &str, setup: Option<&str>) -> bool {
        !self.except.iter().any(|e| e == profile) && !setup.is_some_and(|s| self.except_setups.iter().any(|e| e == s))
    }
}

impl Profile {
    /// Page `view` in this profile: the page at that position, or the folder of that name.
    pub fn page_for(&self, view: PageView) -> Option<&Page> {
        match view {
            PageView::Folder(name) => self.folders.iter().find(|f| f.name == name),
            PageView::Page(n) => self.pages.get(n),
        }
    }

    /// Whether this profile sets `ctl` (keys on page `view`).
    fn sets(&self, view: PageView, ctl: ControlRef) -> bool {
        match ctl {
            ControlRef::Key(i) => self.page_for(view).and_then(|p| p.key(i)).is_some_and(KeyConfig::is_set),
            ControlRef::Dial => self.dial.as_ref().is_some_and(DialConfig::is_set),
            ControlRef::Knob(i) => self
                .knobs
                .iter()
                .any(|k| k.index == i && [&k.cw, &k.ccw, &k.press, &k.release].iter().any(|a| a.is_some())),
            ControlRef::Fader(i) => self.faders.iter().any(|f| f.index == i && f.change.is_some()),
            ControlRef::Roller(side) => self.rollers.iter().any(|r| r.side == side && (r.away.is_some() || r.toward.is_some())),
        }
    }
}

impl Config {
    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// The base profiles of `profile`, nearest first, stopping at cycles and missing names.
    pub fn bases(&self, profile: &str) -> Vec<&Profile> {
        let mut out: Vec<&Profile> = Vec::new();
        let mut next = self.profile(profile).and_then(|p| p.base.as_deref());
        while let Some(name) = next {
            if name == profile || out.iter().any(|p| p.name == name) {
                break;
            }
            let Some(p) = self.profile(name) else { break };
            out.push(p);
            next = p.base.as_deref();
        }
        out
    }

    /// The pinned key at `index` shown in `profile` while `setup` is active.
    pub fn pinned_at(&self, index: u8, profile: &str, setup: Option<&str>) -> Option<&PinnedKey> {
        self.pinned.iter().find(|p| p.key.index == index && p.applies(profile, setup))
    }

    /// Where control `ctl` of `profile` comes from while `setup` is active, keys on page `view`.
    pub fn source_of<'a>(&'a self, profile: &str, view: PageView, setup: Option<&str>, ctl: ControlRef) -> Source<'a> {
        if let ControlRef::Key(i) = ctl
            && let Some(p) = self.pinned_at(i, profile, setup)
        {
            return Source::Pinned(p);
        }
        let Some(own) = self.profile(profile) else { return Source::None };
        if own.sets(view, ctl) {
            return Source::Own(own);
        }
        let mut via = Vec::new();
        for b in self.bases(profile) {
            via.push(b.name.as_str());
            if b.sets(view, ctl) {
                return Source::Base { profile: b, via };
            }
        }
        Source::None
    }

    /// The profile that supplies `ctl`: the profile itself or a base; `None` for pinned keys
    /// and controls nothing sets.
    fn supplier<'a>(&'a self, profile: &str, view: PageView, setup: Option<&str>, ctl: ControlRef) -> Option<&'a Profile> {
        match self.source_of(profile, view, setup, ctl) {
            Source::Own(p) | Source::Base { profile: p, .. } => Some(p),
            Source::Pinned(_) | Source::None => None,
        }
    }

    /// The key at `index` of `profile` on page `view`: pinned, the profile's own, inherited from
    /// a base, else the page's own (unset) entry.
    pub fn key_at<'a>(&'a self, profile: &str, view: PageView, setup: Option<&str>, index: u8) -> Option<&'a KeyConfig> {
        let ctl = ControlRef::Key(index);
        if let Source::Pinned(p) = self.source_of(profile, view, setup, ctl) {
            return Some(&p.key);
        }
        let p = self.supplier(profile, view, setup, ctl).or_else(|| self.profile(profile))?;
        p.page_for(view)?.key(index)
    }

    /// The dial of `profile`, inherited from a base if it sets none.
    pub fn dial_of(&self, profile: &str) -> Option<&DialConfig> {
        match self.supplier(profile, PageView::Page(0), None, ControlRef::Dial) {
            Some(p) => p.dial.as_ref(),
            None => self.profile(profile)?.dial.as_ref(),
        }
    }

    pub fn knob_of(&self, profile: &str, index: u8) -> Option<&KnobConfig> {
        let p = self.supplier(profile, PageView::Page(0), None, ControlRef::Knob(index)).or_else(|| self.profile(profile))?;
        p.knobs.iter().find(|k| k.index == index)
    }

    pub fn fader_of(&self, profile: &str, index: u8) -> Option<&FaderConfig> {
        let p = self.supplier(profile, PageView::Page(0), None, ControlRef::Fader(index)).or_else(|| self.profile(profile))?;
        p.faders.iter().find(|f| f.index == index)
    }

    pub fn roller_of(&self, profile: &str, side: Side) -> Option<&RollerConfig> {
        let p = self.supplier(profile, PageView::Page(0), None, ControlRef::Roller(side)).or_else(|| self.profile(profile))?;
        p.rollers.iter().find(|r| r.side == side)
    }

    /// Global LED colours with the overrides of `profile`'s bases (farthest first) and then of
    /// `profile` applied per field.
    pub fn leds_of(&self, profile: &str) -> Leds {
        let mut leds = self.leds.clone();
        let chain = self.profile(profile).into_iter().chain(self.bases(profile)).collect::<Vec<_>>();
        for o in chain.iter().rev().filter_map(|p| p.leds.as_ref()) {
            leds.base = o.base.or(leds.base);
            leds.fader = o.fader.clone().or(leds.fader);
            leds.knob = o.knob.clone().or(leds.knob);
            leds.brightness = o.brightness.or(leds.brightness);
            leds.knob_volume = o.knob_volume.or(leds.knob_volume);
            leds.knob_mute_red = o.knob_mute_red.or(leds.knob_mute_red);
        }
        leds
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialConfig {
    #[serde(flatten)]
    pub face: Face,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ccw: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Action>,
    /// Held down for a while.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_press: Option<Action>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaderConfig {
    /// 0 = top.
    pub index: u8,
    /// Run with `{value}` replaced by the position 0..=100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<Action>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnobConfig {
    /// 0 = bottom-left, 1 = bottom-right, 2 = top.
    pub index: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cw: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ccw: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Action>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollerConfig {
    pub side: Side,
    /// Rolled away from you (towards the keys).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub away: Option<Action>,
    /// Rolled towards you.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toward: Option<Action>,
}

/// What an input does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Action {
    /// Shell command run with `sh -c`. `{value}` is replaced for faders.
    Cmd(String),
    /// Key combination sent through a virtual keyboard, e.g. `"ctrl+shift+m"` or `"playpause"`,
    /// or `{ combo, hold }` to hold it `hold` ms.
    Keys(KeysAction),
    /// Keypad page: `"next"`, `"prev"` or a page name.
    Page(String),
    /// Open a keypad folder by name.
    Folder(String),
    /// Leave the current folder.
    Back(bool),
    /// Switch to a profile by name, or `"next"`, `"prev"`, `"pick"` within the active setup.
    Profile(String),
    /// Switch setup: a setup name, or `"next"`, `"prev"`, `"pick"`.
    Setup(String),
    /// Hybrid setups: pin the live profile (`toggle`) or release the pin.
    Pin(PinAction),
    /// Volume: `"+2"`/`"-2"` change it by percentage points, `"40"` sets it, `"{value}"`
    /// follows a fader. Of the default output, or of `{ level, output | input | app | group }`.
    Volume(VolumeAction),
    /// Mute: `toggle`, `on` or `off`. Of the default output, or of
    /// `{ state, output | input | app | group }`.
    Mute(MuteAction),
    /// Bring up the Claude Code session in this slot (see `Widget::ClaudeSession`).
    Session(u8),
    /// Scroll the Claude Code session keys by a page: `"next"` or `"prev"`.
    Sessions(String),
}

/// A `keys` action: a combination tapped (`"ctrl+c"`, `"o@700"`) or held (`{ combo, hold }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeysAction {
    Tap(String),
    Hold(KeysHold),
}

/// `keys` as a table: `combo` held for `hold` ms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeysHold {
    pub combo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<u32>,
}

/// The longest `keys` hold accepted (ms): the service waits it out on its actions thread.
pub const MAX_HOLD_MS: u64 = 3000;

/// Split `combo` or `combo@ms` into the key names and the hold in ms (0 for a tap), checking
/// the hold against [`MAX_HOLD_MS`]. Key names are not checked here (the service does that).
pub fn split_hold(combo: &str) -> Result<(&str, u64)> {
    let Some((keys, ms)) = combo.rsplit_once('@') else { return Ok((combo, 0)) };
    let ms: u64 = ms.trim().parse().map_err(|_| anyhow!("bad hold time in {combo:?}"))?;
    if ms > MAX_HOLD_MS {
        bail!("hold of {ms} ms in {combo:?} is over {MAX_HOLD_MS} ms");
    }
    Ok((keys, ms))
}

impl KeysAction {
    /// The combination in the keyboard's `combo` / `combo@ms` syntax.
    pub fn combo(&self) -> String {
        match self {
            Self::Tap(c) => c.clone(),
            Self::Hold(KeysHold { combo, hold: Some(ms) }) => format!("{combo}@{ms}"),
            Self::Hold(KeysHold { combo, hold: None }) => combo.clone(),
        }
    }
}

impl From<&str> for KeysAction {
    fn from(combo: &str) -> Self {
        Self::Tap(combo.to_owned())
    }
}

impl From<String> for KeysAction {
    fn from(combo: String) -> Self {
        Self::Tap(combo)
    }
}

/// A `pin` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinAction {
    /// Pin the live profile, or release the pin.
    Toggle,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mute {
    Toggle,
    On,
    Off,
}

/// What a `volume` or `mute` action applies to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AudioTarget {
    /// The default output device.
    #[default]
    Output,
    /// An output device by name or description (case-insensitive, `*` wildcards).
    OutputDevice(String),
    /// The default input device (microphone).
    Input,
    /// An input device by name or description (case-insensitive, `*` wildcards).
    InputDevice(String),
    /// Apps playing audio, by program or app name (case-insensitive, `*` wildcards, `.exe`
    /// optional), e.g. `"discord"`.
    App(String),
    /// A named `[[audio_group]]`.
    Group(String),
}

/// A `volume` action: the change (`"+2"`, `"40"`, `"{value}"`) and what it applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "VolumeRepr", into = "VolumeRepr")]
pub struct VolumeAction {
    pub level: String,
    pub target: AudioTarget,
}

/// A `mute` action: the new state and what it applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MuteRepr", into = "MuteRepr")]
pub struct MuteAction {
    pub state: Mute,
    pub target: AudioTarget,
}

impl From<&str> for VolumeAction {
    fn from(level: &str) -> Self {
        Self { level: level.to_owned(), target: AudioTarget::Output }
    }
}

impl From<Mute> for MuteAction {
    fn from(state: Mute) -> Self {
        Self { state, target: AudioTarget::Output }
    }
}

/// The target keys of a volume or mute table; exactly one may be set.
#[derive(Debug, Clone, Default)]
struct TargetKeys {
    output: Option<String>,
    input: Option<String>,
    app: Option<String>,
    group: Option<String>,
}

impl TryFrom<TargetKeys> for AudioTarget {
    type Error = String;
    fn try_from(t: TargetKeys) -> Result<Self, String> {
        let device = |d: String, default, named: fn(String) -> AudioTarget| {
            if d.is_empty() || d.eq_ignore_ascii_case("default") { default } else { named(d) }
        };
        let mut targets = [
            t.output.map(|d| device(d, AudioTarget::Output, AudioTarget::OutputDevice)),
            t.input.map(|d| device(d, AudioTarget::Input, AudioTarget::InputDevice)),
            t.app.map(AudioTarget::App),
            t.group.map(AudioTarget::Group),
        ]
        .into_iter()
        .flatten();
        let target = targets.next().unwrap_or_default();
        if targets.next().is_some() {
            return Err("use only one of output, input, app and group".into());
        }
        Ok(target)
    }
}

impl From<AudioTarget> for TargetKeys {
    fn from(t: AudioTarget) -> Self {
        let mut k = Self::default();
        match t {
            AudioTarget::Output => k.output = Some("default".into()),
            AudioTarget::OutputDevice(d) => k.output = Some(d),
            AudioTarget::Input => k.input = Some("default".into()),
            AudioTarget::InputDevice(d) => k.input = Some(d),
            AudioTarget::App(a) => k.app = Some(a),
            AudioTarget::Group(g) => k.group = Some(g),
        }
        k
    }
}

/// TOML/JSON shapes of volume and mute actions: the bare value for the default output, else
/// a table with the value and one target key.
macro_rules! target_repr {
    ($repr:ident, $table:ident, $action:ident, $field:ident: $ty:ty) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(untagged)]
        enum $repr {
            Short($ty),
            Long($table),
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $table {
            $field: $ty,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            output: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            input: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            app: Option<String>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            group: Option<String>,
        }

        impl TryFrom<$repr> for $action {
            type Error = String;
            fn try_from(r: $repr) -> Result<Self, String> {
                Ok(match r {
                    $repr::Short($field) => Self { $field, target: AudioTarget::Output },
                    $repr::Long(t) => {
                        let keys = TargetKeys { output: t.output, input: t.input, app: t.app, group: t.group };
                        Self { $field: t.$field, target: keys.try_into()? }
                    }
                })
            }
        }

        impl From<$action> for $repr {
            fn from(a: $action) -> Self {
                if a.target == AudioTarget::Output {
                    return Self::Short(a.$field);
                }
                let k = TargetKeys::from(a.target);
                Self::Long($table { $field: a.$field, output: k.output, input: k.input, app: k.app, group: k.group })
            }
        }
    };
}

target_repr!(VolumeRepr, VolumeTable, VolumeAction, level: String);
target_repr!(MuteRepr, MuteTable, MuteAction, state: Mute);

/// Apps whose volume one fader or knob controls together, e.g. all games.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioGroup {
    pub name: String,
    /// Program or app names, as for `app` targets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<String>,
    /// Also every program started by Steam (games and Proton).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub steam: bool,
}

/// A parsed `volume` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeChange {
    /// Change by percentage points.
    By(i32),
    /// Set to a percentage.
    To(u8),
}

impl VolumeChange {
    /// Parse a `volume` action, with `value` (a fader position) substituted for `{value}`.
    pub fn parse(s: &str, value: Option<u8>) -> Option<Self> {
        let s = s.trim().replace("{value}", &value.unwrap_or(0).to_string());
        if let Some(n) = s.strip_prefix('+') {
            return Some(Self::By(n.trim().parse().ok()?));
        }
        if let Some(n) = s.strip_prefix('-') {
            return Some(Self::By(-n.trim().parse::<i32>().ok()?));
        }
        s.parse::<u8>().ok().filter(|v| *v <= 100).map(Self::To)
    }

    /// The new level (0.0..=1.0) from the current one.
    pub fn apply(self, current: f32) -> f32 {
        match self {
            Self::By(d) => (current + d as f32 / 100.0).clamp(0.0, 1.0),
            Self::To(v) => v as f32 / 100.0,
        }
    }
}

/// `"#rrggbb"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(pub Rgb);

impl Color {
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0.r, self.0.g, self.0.b)
    }

    pub fn parse(s: &str) -> Option<Self> {
        let h = s.strip_prefix('#')?;
        if h.len() != 6 {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Color(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8)))
    }
}

impl Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid colour {s:?}, expected #rrggbb")))
    }
}

impl Config {
    /// Parse and validate, resolving relative paths against `base_dir`.
    pub fn parse(text: &str, base_dir: &Path) -> Result<Self> {
        let cfg = Self::parse_unchecked(text, base_dir)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Parse without [`Config::validate`], e.g. profiles imported from a file that refer to
    /// profiles or groups only the target config has.
    pub fn parse_unchecked(text: &str, base_dir: &Path) -> Result<Self> {
        let mut cfg: Config = toml::from_str(text)?;
        for profile in &mut cfg.profiles {
            let faces = profile
                .pages
                .iter_mut()
                .chain(profile.folders.iter_mut())
                .flat_map(|p| p.keys.iter_mut().map(|k| &mut k.face))
                .chain(profile.dial.iter_mut().map(|d| &mut d.face));
            for face in faces {
                if let Some(img) = &mut face.image {
                    *img = expand(base_dir, img);
                }
            }
        }
        for p in &mut cfg.pinned {
            if let Some(img) = &mut p.key.face.image {
                *img = expand(base_dir, img);
            }
        }
        if let Some(font) = &mut cfg.font {
            *font = expand(base_dir, font);
        }
        Ok(cfg)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text, path.parent().unwrap_or(Path::new("."))).with_context(|| format!("in {}", path.display()))
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write atomically (temp file + rename), creating the directory if needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml()?).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        if self.backlight > 100 {
            bail!("backlight must be 0..=100");
        }
        let mut names = std::collections::HashSet::new();
        for p in &self.profiles {
            if !names.insert(p.name.as_str()) {
                bail!("profile {:?} defined twice", p.name);
            }
            for page in p.pages.iter().chain(&p.folders) {
                let mut seen = [false; 15];
                for k in &page.keys {
                    if k.index > 14 {
                        bail!("{}/{}: key index {} out of range 0..=14", p.name, page.name, k.index);
                    }
                    if std::mem::replace(&mut seen[k.index as usize], true) {
                        bail!("{}/{}: key {} configured twice", p.name, page.name, k.index);
                    }
                }
            }
            if let Some(f) = p.faders.iter().find(|f| f.index > 4) {
                bail!("{}: fader index {} out of range 0..=4", p.name, f.index);
            }
            if let Some(k) = p.knobs.iter().find(|k| k.index > 2) {
                bail!("{}: knob index {} out of range 0..=2", p.name, k.index);
            }
            for action in p.actions() {
                match action {
                    Action::Folder(f) if !p.folders.iter().any(|x| &x.name == f) => {
                        bail!("{}: folder {f:?} does not exist", p.name)
                    }
                    Action::Page(pg)
                        if pg != "next" && pg != "prev" && !p.pages.iter().any(|x| &x.name == pg) =>
                    {
                        bail!("{}: page {pg:?} does not exist", p.name)
                    }
                    Action::Volume(v) if VolumeChange::parse(&v.level, None).is_none() => {
                        bail!("{}: volume {:?} must be like \"+2\", \"-2\", \"40\" or \"{{value}}\"", p.name, v.level)
                    }
                    _ => {}
                }
            }
        }
        let mut groups = std::collections::HashSet::new();
        for g in &self.audio_groups {
            if !groups.insert(g.name.as_str()) {
                bail!("audio group {:?} defined twice", g.name);
            }
        }
        let mut pinned = std::collections::HashSet::new();
        for p in &self.pinned {
            if p.key.index > 14 {
                bail!("pinned key index {} out of range 0..=14", p.key.index);
            }
            if !pinned.insert(p.key.index) {
                bail!("pinned key {} configured twice", p.key.index);
            }
            if let Some(e) = p.except.iter().find(|e| !self.profiles.iter().any(|x| &x.name == *e)) {
                bail!("pinned key {}: profile {e:?} does not exist", p.key.index);
            }
            for action in [&p.key.press, &p.key.release].into_iter().flatten() {
                if matches!(action, Action::Folder(_) | Action::Page(_)) {
                    bail!("pinned key {}: folder and page actions differ per profile; use profile, back or session", p.key.index);
                }
            }
        }
        let pinned_actions = self.pinned.iter().flat_map(|p| [&p.key.press, &p.key.release]).flatten();
        let setups = self.effective_setups();
        for action in self.profiles.iter().flat_map(Profile::actions).chain(pinned_actions) {
            if let Action::Profile(name) = action
                && !is_step(name)
                && !self.profiles.iter().any(|p| &p.name == name)
            {
                bail!("profile {name:?} does not exist");
            }
            if let Action::Setup(name) = action
                && !is_step(name)
                && !setups.iter().any(|s| &s.name == name)
            {
                bail!("setup {name:?} does not exist");
            }
            if let Action::Sessions(d) = action
                && d != "next"
                && d != "prev"
            {
                bail!("sessions {d:?} must be \"next\" or \"prev\"");
            }
            let target = match action {
                Action::Volume(v) => &v.target,
                Action::Mute(m) => &m.target,
                _ => continue,
            };
            if let AudioTarget::Group(g) = target
                && !groups.contains(g.as_str())
            {
                bail!("audio group {g:?} does not exist");
            }
        }
        self.validate_setups()?;
        for p in &self.pinned {
            if let Some(s) = p.except_setups.iter().find(|s| !setups.iter().any(|x| &x.name == *s)) {
                bail!("pinned key {}: setup {s:?} does not exist", p.key.index);
            }
        }
        for p in &self.profiles {
            let faces = p.pages.iter().chain(&p.folders).flat_map(|pg| &pg.keys).map(|k| &k.face).chain(p.dial.iter().map(|d| &d.face));
            if faces.filter_map(|f| f.widget.as_ref()).any(|w| w.refresh == Some(0)) {
                bail!("{}: widget refresh must be at least 1 second", p.name);
            }
        }
        if self.pinned.iter().filter_map(|p| p.key.face.widget.as_ref()).any(|w| w.refresh == Some(0)) {
            bail!("pinned key: widget refresh must be at least 1 second");
        }
        Ok(())
    }

    /// The hold of every `keys` action and setup shortcut is within [`MAX_HOLD_MS`]. Kept apart
    /// from [`Config::validate`] so the service can still start on a file that breaks it (the
    /// binding is then skipped); everything that saves a config runs both.
    pub fn validate_holds(&self) -> Result<()> {
        for p in &self.profiles {
            for action in p.actions() {
                if let Action::Keys(k) = action {
                    split_hold(&k.combo()).with_context(|| format!("{}: keys action", p.name))?;
                }
            }
        }
        for p in &self.pinned {
            for action in [&p.key.press, &p.key.release].into_iter().flatten() {
                if let Action::Keys(k) = action {
                    split_hold(&k.combo()).with_context(|| format!("pinned key {}: keys action", p.key.index))?;
                }
            }
        }
        for s in &self.setups {
            for shortcut in s.shortcuts() {
                split_hold(shortcut).with_context(|| format!("setup {:?}: shortcut {shortcut:?}", s.name))?;
            }
        }
        Ok(())
    }

    /// Setups and profile bases: names unique, references exist, no cycles.
    fn validate_setups(&self) -> Result<()> {
        let profile = |name: &str| self.profiles.iter().any(|p| p.name == name);
        let mut names = std::collections::HashSet::new();
        for s in &self.setups {
            if s.name.trim().is_empty() {
                bail!("a setup has no name");
            }
            if !names.insert(s.name.as_str()) {
                bail!("setup {:?} defined twice", s.name);
            }
            let mut listed = std::collections::HashSet::new();
            for p in &s.profiles {
                if !profile(p) {
                    bail!("setup {:?}: profile {p:?} does not exist", s.name);
                }
                if !listed.insert(p.as_str()) {
                    bail!("setup {:?}: profile {p:?} listed twice", s.name);
                }
            }
            if let Some(f) = &s.fallback {
                if !profile(f) {
                    bail!("setup {:?}: fallback {f:?} does not exist", s.name);
                }
                if listed.contains(f.as_str()) {
                    bail!("setup {:?}: fallback {f:?} is also in its profiles", s.name);
                }
            }
            if s.release_after == Some(0) {
                bail!("setup {:?}: release_after must be at least 1 minute", s.name);
            }
        }
        for p in &self.profiles {
            let mut seen = vec![p.name.as_str()];
            let mut next = p.base.as_deref();
            while let Some(b) = next {
                let Some(base) = self.profiles.iter().find(|x| x.name == b) else {
                    bail!("{}: base profile {b:?} does not exist", p.name);
                };
                if seen.contains(&b) {
                    bail!("{}: base profiles form a cycle ({} → {b})", p.name, seen.join(" → "));
                }
                seen.push(b);
                next = base.base.as_deref();
            }
        }
        Ok(())
    }

    /// The setups, or for a config without `[[setup]]` the implicit one that behaves exactly
    /// like [`Config::profile_for_app`]: [`IMPLICIT_SETUP`], follow, every profile but the
    /// first, the first as fallback.
    pub fn effective_setups(&self) -> Vec<Setup> {
        if !self.setups.is_empty() {
            return self.setups.clone();
        }
        let names: Vec<String> = self.profiles.iter().map(|p| p.name.clone()).collect();
        vec![Setup {
            name: IMPLICIT_SETUP.to_owned(),
            mode: Some(SetupMode::Follow),
            profiles: names.iter().skip(1).cloned().collect(),
            fallback: names.first().cloned(),
            ..Default::default()
        }]
    }

    /// The profile to use for a focused window class: the first non-default profile whose
    /// `apps` match, otherwise the default (first) profile.
    pub fn profile_for_app(&self, class: &str) -> usize {
        self.profiles
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, p)| p.apps.iter().any(|pat| glob_match(pat, class)))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

impl Profile {
    /// Every action configured in this profile.
    pub fn actions(&self) -> impl Iterator<Item = &Action> {
        self.pages
            .iter()
            .chain(&self.folders)
            .flat_map(|p| &p.keys)
            .flat_map(|k| [&k.press, &k.release])
            .chain(self.dial.iter().flat_map(|d| [&d.cw, &d.ccw, &d.press, &d.release, &d.long_press]))
            .chain(self.faders.iter().map(|f| &f.change))
            .chain(self.knobs.iter().flat_map(|k| [&k.cw, &k.ccw, &k.press, &k.release]))
            .chain(self.rollers.iter().flat_map(|r| [&r.away, &r.toward]))
            .flatten()
    }
}

/// `"next"`, `"prev"` or `"pick"` in a `profile` or `setup` action.
fn is_step(name: &str) -> bool {
    matches!(name, "next" | "prev" | "pick")
}

/// Case-insensitive match with `*` wildcards.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.to_lowercase(), text.to_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == t;
    }
    let mut rest = t.as_str();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            let Some(r) = rest.strip_prefix(part) else { return false };
            rest = r;
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            let Some(pos) = rest.find(part) else { return false };
            rest = &rest[pos + part.len()..];
        }
    }
    true
}

/// Expand `~/` and make relative paths relative to `base`.
fn expand(base: &Path, p: &Path) -> PathBuf {
    if let Ok(rest) = p.strip_prefix("~")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    if p.is_relative() { base.join(p) } else { p.to_owned() }
}

/// `%APPDATA%\openmasterhub\config.toml` on Windows.
#[cfg(windows)]
pub fn default_path() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("openmasterhub").join("config.toml")
}

/// `$XDG_CONFIG_HOME/openmasterhub/config.toml`
#[cfg(not(windows))]
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("openmasterhub/config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses_and_roundtrips() {
        let text = include_str!("../../../examples/config.toml");
        let cfg = Config::parse(text, Path::new("/x")).unwrap();
        assert!(cfg.profiles.len() >= 2);
        assert!(!cfg.profiles[0].pages.is_empty());
        let again = Config::parse(&cfg.to_toml().unwrap(), Path::new("/x")).unwrap();
        assert_eq!(cfg, again);
        let claude = cfg.profiles.iter().find(|p| p.name == "Claude").unwrap();
        let pin = &cfg.pinned[0];
        assert_eq!(pin.key.press, Some(Action::Profile("Claude".into())));
        let main = &cfg.profiles[0].pages[0];
        assert_eq!(cfg.key("Default", main, pin.key.index), Some(&pin.key));
        assert_ne!(cfg.key("Claude", &claude.pages[0], pin.key.index), Some(&pin.key));
        let key = claude.pages[0].key(2).unwrap();
        assert_eq!(key.face.widget, Some(Widget::ClaudeSession { slot: 2 }.into()));
        assert_eq!(key.press, Some(Action::Session(2)));
        let left = claude.rollers.iter().find(|r| r.side == Side::Left).unwrap();
        assert_eq!(left.toward, Some(Action::Sessions("next".into())));
    }

    #[test]
    fn colors_actions_widgets() {
        assert_eq!(Color::parse("#ff8000"), Some(Color(Rgb::new(255, 128, 0))));
        assert_eq!(Color::parse("ff8000"), None);
        let cfg = Config::parse(
            r##"
            [[profile]]
            name = "Default"
            [[profile.page]]
            name = "Main"
            [[profile.page.key]]
            index = 3
            label = "Mute"
            background = "#c81e1e"
            widget = { kind = "clock", format = "%H:%M:%S" }
            press = { keys = "ctrl+shift+m" }
            [[profile.page.key]]
            index = 4
            press = { page = "next" }
            "##,
            Path::new("/"),
        )
        .unwrap();
        let key = &cfg.profiles[0].pages[0].keys[0];
        assert_eq!(key.press, Some(Action::Keys("ctrl+shift+m".into())));
        assert_eq!(key.face.widget, Some(Widget::Clock { format: Some("%H:%M:%S".into()) }.into()));
    }

    #[test]
    fn rejects_dangling_references() {
        let bad_folder = "[[profile]]\nname='A'\n[[profile.page]]\nname='P'\n[[profile.page.key]]\nindex=0\npress={folder='Nope'}\n";
        assert!(Config::parse(bad_folder, Path::new("/")).is_err());
        let bad_index = "[[profile]]\nname='A'\n[[profile.page]]\nname='P'\n[[profile.page.key]]\nindex=15\n";
        assert!(Config::parse(bad_index, Path::new("/")).is_err());
        let bad_volume = "[[profile]]\nname='A'\n[profile.dial]\ncw={volume='up'}\n";
        assert!(Config::parse(bad_volume, Path::new("/")).is_err());
        let sessions = |dir: &str| format!("[[profile]]\nname='A'\n[[profile.roller]]\nside='left'\naway={{sessions='{dir}'}}\n");
        assert!(Config::parse(&sessions("prev"), Path::new("/")).is_ok());
        assert!(Config::parse(&sessions("up"), Path::new("/")).is_err());
        let sort = |s: &str| Config::parse(&format!("[status]\nsort='{s}'\n"), Path::new("/")).map(|c| c.status.sort);
        assert_eq!(sort("urgent").unwrap(), SessionSort::Urgent);
        assert_eq!(sort("changed").unwrap(), SessionSort::Changed);
        assert_eq!(sort("started").unwrap(), SessionSort::Started);
        assert_eq!(Config::parse("", Path::new("/")).unwrap().status.sort, SessionSort::Urgent);
        assert!(sort("newest").is_err());
        let pin = |extra: &str| format!("[[pinned]]\nindex=4\n{extra}\n[[profile]]\nname='A'\n");
        assert!(Config::parse(&pin("press={profile='A'}\nexcept=['A']"), Path::new("/")).is_ok());
        assert!(Config::parse(&pin("except=['Nope']"), Path::new("/")).is_err());
        assert!(Config::parse(&pin("press={folder='F'}"), Path::new("/")).is_err());
        assert!(Config::parse(&pin("press={profile='Nope'}"), Path::new("/")).is_err());
        assert!(Config::parse(&pin("labl='typo'"), Path::new("/")).is_err());
    }

    #[test]
    fn starter_config_parses() {
        let cfg = Config::parse(include_str!("../../../examples/starter.toml"), Path::new("/x")).unwrap();
        assert_eq!(cfg.profiles[0].dial.as_ref().unwrap().press, Some(Action::Mute(Mute::Toggle.into())));
    }

    #[test]
    fn volume_changes() {
        assert_eq!(VolumeChange::parse("+2", None), Some(VolumeChange::By(2)));
        assert_eq!(VolumeChange::parse("-5", None), Some(VolumeChange::By(-5)));
        assert_eq!(VolumeChange::parse("40", None), Some(VolumeChange::To(40)));
        assert_eq!(VolumeChange::parse("{value}", Some(73)), Some(VolumeChange::To(73)));
        assert_eq!(VolumeChange::parse("101", None), None);
        assert_eq!(VolumeChange::parse("up", None), None);
        assert_eq!(VolumeChange::By(5).apply(0.98), 1.0);
        assert_eq!(VolumeChange::By(-10).apply(0.05), 0.0);
        assert_eq!(VolumeChange::To(50).apply(0.9), 0.5);
    }

    #[test]
    fn app_matching() {
        assert!(glob_match("obs", "OBS"));
        assert!(glob_match("steam_app_*", "steam_app_1234"));
        assert!(glob_match("*firefox*", "org.mozilla.firefox"));
        assert!(!glob_match("obs", "obsidian"));
        let cfg = Config {
            profiles: vec![
                Profile { name: "Default".into(), ..Default::default() },
                Profile { name: "Stream".into(), apps: vec!["obs".into()], ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(cfg.profile_for_app("obs"), 1);
        assert_eq!(cfg.profile_for_app("kitty"), 0);
    }

    #[test]
    fn audio_targets() {
        let text = r#"
            [[audio_group]]
            name = "Games"
            apps = ["cs2"]
            steam = true

            [[profile]]
            name = "Default"
            [[profile.fader]]
            index = 0
            change = { volume = "{value}" }
            [[profile.fader]]
            index = 1
            change = { volume = { level = "{value}", app = "discord" } }
            [[profile.fader]]
            index = 2
            change = { volume = { level = "{value}", group = "Games" } }
            [[profile.fader]]
            index = 3
            change = { volume = { level = "{value}", input = "default" } }
            [[profile.fader]]
            index = 4
            change = { volume = { level = "{value}", output = "*headphones*" } }
            [[profile.knob]]
            index = 0
            press = { mute = { state = "toggle", input = "Blue Yeti" } }
        "#;
        let cfg = Config::parse(text, Path::new("/x")).unwrap();
        let p = &cfg.profiles[0];
        let target = |i: usize| match &p.faders[i].change {
            Some(Action::Volume(v)) => v.target.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(target(0), AudioTarget::Output);
        assert_eq!(target(1), AudioTarget::App("discord".into()));
        assert_eq!(target(2), AudioTarget::Group("Games".into()));
        assert_eq!(target(3), AudioTarget::Input);
        assert_eq!(target(4), AudioTarget::OutputDevice("*headphones*".into()));
        assert_eq!(
            p.knobs[0].press,
            Some(Action::Mute(MuteAction { state: Mute::Toggle, target: AudioTarget::InputDevice("Blue Yeti".into()) }))
        );
        assert!(cfg.audio_groups[0].steam);

        let toml = cfg.to_toml().unwrap();
        assert!(toml.contains(r#"volume = "{value}""#), "{toml}");
        assert!(toml.contains(r#"state = "toggle""#), "{toml}");
        assert_eq!(Config::parse(&toml, Path::new("/x")).unwrap(), cfg);

        let json = serde_json::to_string(&p.faders[1].change).unwrap();
        assert_eq!(json, r#"{"volume":{"level":"{value}","app":"discord"}}"#);

        let bad = |change: &str| Config::parse(&format!("[[profile]]\nname = \"a\"\n[[profile.fader]]\nindex = 0\nchange = {change}\n"), Path::new("/x"));
        assert!(bad(r#"{ volume = { level = "1", app = "a", group = "b" } }"#).is_err());
        assert!(bad(r#"{ volume = { level = "1", group = "nope" } }"#).is_err());
        assert!(bad(r#"{ volume = { level = "loud", app = "a" } }"#).is_err());
        assert!(bad(r#"{ volume = { level = "1", device = "a" } }"#).is_err());
    }

    /// No `[[setup]]`, `[app]` or other new key appears when a config does not set them.
    #[test]
    fn existing_configs_serialise_unchanged() {
        for text in [include_str!("../../../examples/config.toml"), include_str!("../../../examples/starter.toml")] {
            let cfg = Config::parse(text, Path::new("/x")).unwrap();
            let toml = cfg.to_toml().unwrap();
            for key in ["[app", "[[setup]]", "about =", "except_setups", "blink_on_setup", "knob_volume", "knob_mute_red", "long_press", "fit =", "label_over_image", "refresh ="] {
                assert!(!toml.contains(key), "{key} in\n{toml}");
            }
            assert_eq!(Config::parse(&toml, Path::new("/x")).unwrap(), cfg);
            // The app sends configs back as JSON.
            let json = serde_json::to_string(&cfg).unwrap();
            assert_eq!(serde_json::from_str::<Config>(&json).unwrap(), cfg);
            assert!(!json.contains("\"app\"") && !json.contains("\"setup\""), "{json}");
        }
    }

    const SETUPS: &str = r##"
        [app]
        theme = "system"
        theme_dark = "orchid"
        density = "compact"
        first_run_done = true
        overlay = "all"
        future_setting = [1, 2]
        [app.feedback]
        toast = true
        in_fullscreen = false

        [status]
        blink_on_setup = false

        [leds]
        knob_volume = true
        knob_mute_red = true

        [[setup]]
        name = "Work"
        mode = "follow"
        profiles = ["Blender", "Claude"]
        fallback = "Desktop"
        color = "#5e63eb"
        shortcut = "ctrl+alt+1"
        show_on_dial = true

        [[setup]]
        name = "Stream"
        mode = "manual"
        profiles = ["Live", "Desktop"]
        wrap = true
        next = "ctrl+alt+f12"

        [[setup]]
        name = "Play"
        mode = "hybrid"
        profiles = ["Blender"]
        fallback = "Desktop"
        pin = "ctrl+alt+p"
        release = "timeout"
        release_after = 10

        [[pinned]]
        index = 8
        except = ["Claude"]
        except_setups = ["Stream"]
        press = { setup = "next" }

        [[profile]]
        name = "Desktop"
        color = "#c26a2b"
        about = "Everything else"
        [[profile.page]]
        name = "Main"
        [[profile.page.key]]
        index = 0
        press = { keys = { combo = "ctrl+shift+t", hold = 250 } }
        [[profile.page.key]]
        index = 1
        press = { profile = "next" }
        [[profile.page.key]]
        index = 2
        press = { pin = "toggle" }
        fit = "fill"
        label_over_image = true
        widget = { kind = "cpu_usage", refresh = 2 }
        [[profile.page.key]]
        index = 3
        press = { setup = "Play" }
        widget = { kind = "clock", format = "%H", refresh = 1 }
        [profile.dial]
        long_press = { profile = "pick" }

        [[profile]]
        name = "Blender"
        apps = ["blender"]
        base = "Desktop"

        [[profile]]
        name = "Claude"
        apps = ["*claude*"]

        [[profile]]
        name = "Live"
        base = "Blender"
    "##;

    #[test]
    fn setups_and_new_shapes_roundtrip() {
        let cfg = Config::parse(SETUPS, Path::new("/x")).unwrap();
        assert_eq!(cfg.setups.len(), 3);
        assert_eq!(cfg.setups[1].mode(), SetupMode::Manual);
        assert_eq!(cfg.setups[2].release, Some(ReleasePolicy::Timeout));
        assert_eq!(cfg.effective_setups(), cfg.setups);
        assert_eq!(cfg.app.theme.as_deref(), Some("system"));
        assert!(cfg.app.other.contains_key("future_setting"));
        assert!(!cfg.status.blink_on_setup);
        assert_eq!(cfg.pinned[0].except_setups, ["Stream"]);
        let keys = &cfg.profiles[0].pages[0].keys;
        let hold = KeysAction::Hold(KeysHold { combo: "ctrl+shift+t".into(), hold: Some(250) });
        assert_eq!(keys[0].press, Some(Action::Keys(hold.clone())));
        assert_eq!(hold.combo(), "ctrl+shift+t@250");
        assert_eq!(keys[2].press, Some(Action::Pin(PinAction::Toggle)));
        assert_eq!(keys[2].face.fit, Some(ImageFit::Fill));
        let w = keys[2].face.widget.as_ref().unwrap();
        assert_eq!((&w.widget, w.refresh), (&Widget::CpuUsage, Some(2)));
        assert_eq!(cfg.profiles[0].dial.as_ref().unwrap().long_press, Some(Action::Profile("pick".into())));
        assert_eq!(cfg.profiles[1].base.as_deref(), Some("Desktop"));

        let toml = cfg.to_toml().unwrap();
        assert_eq!(Config::parse(&toml, Path::new("/x")).unwrap(), cfg, "{toml}");
        for key in ["[[setup]]", "[app]", "future_setting", "[app.feedback]", "overlay = \"all\"", "blink_on_setup = false", "except_setups", "hold = 250"] {
            assert!(toml.contains(key), "{key} missing in\n{toml}");
        }
        // Unset setup fields stay unset.
        assert!(!toml.contains("show_on_dial = false") && !toml.contains("mode = \"follow\"\nprofiles = [\"Blender\"]"));

        // The app's JSON shapes (app/src/lib/types.ts).
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["setup"][0]["name"], "Work");
        assert_eq!(json["profile"][0]["page"][0]["key"][0]["press"], serde_json::json!({"keys": {"combo": "ctrl+shift+t", "hold": 250}}));
        assert_eq!(json["profile"][0]["page"][0]["key"][2]["widget"], serde_json::json!({"kind": "cpu_usage", "refresh": 2}));
        assert_eq!(json["profile"][0]["page"][0]["key"][2]["press"], serde_json::json!({"pin": "toggle"}));
        assert_eq!(json["app"]["feedback"]["toast"], true);
        assert_eq!(json["app"]["overlay"], "all");
        assert_eq!(serde_json::from_value::<Config>(json).unwrap(), cfg);
    }

    #[test]
    fn rejects_bad_setups() {
        let with = |find: &str, replace: &str| {
            assert!(SETUPS.contains(find), "{find}");
            Config::parse(&SETUPS.replacen(find, replace, 1), Path::new("/x"))
        };
        assert!(with("name = \"Stream\"", "name = \"Work\"").is_err(), "duplicate setup");
        assert!(with("profiles = [\"Blender\", \"Claude\"]", "profiles = [\"Blender\", \"Nope\"]").is_err());
        assert!(with("profiles = [\"Blender\", \"Claude\"]", "profiles = [\"Blender\", \"Blender\"]").is_err());
        assert!(with("profiles = [\"Blender\", \"Claude\"]", "profiles = [\"Desktop\"]").is_err(), "fallback in list");
        assert!(with("fallback = \"Desktop\"", "fallback = \"Nope\"").is_err());
        assert!(with("release_after = 10", "release_after = 0").is_err());
        assert!(with("release = \"timeout\"", "release = \"later\"").is_err());
        assert!(with("mode = \"hybrid\"", "mode = \"auto\"").is_err());
        assert!(with("except_setups = [\"Stream\"]", "except_setups = [\"Nope\"]").is_err());
        assert!(with("base = \"Desktop\"", "base = \"Nope\"").is_err());
        assert!(with("base = \"Desktop\"", "base = \"Live\"").is_err(), "cycle");
        assert!(with("base = \"Desktop\"", "base = \"Blender\"").is_err(), "own base");
        assert!(with("press = { setup = \"Play\" }", "press = { setup = \"Nope\" }").is_err());
        assert!(with("press = { profile = \"next\" }", "press = { profile = \"later\" }").is_err());
        assert!(with("press = { pin = \"toggle\" }", "press = { pin = \"hold\" }").is_err());
        assert!(with("refresh = 2", "refresh = 0").is_err());
        assert!(with("hold = 250", "hold = 250, tap = 1").is_err());
        assert!(with("wrap = true", "wrapp = true").is_err(), "unknown setup key");
        assert!(with("toast = true", "toast = 1").is_err());
        // A manual setup keeps a fallback from when it followed apps (the app leaves it).
        assert!(with("profiles = [\"Live\", \"Desktop\"]", "profiles = [\"Live\"]\nfallback = \"Desktop\"").is_ok());
    }

    /// Without `[[setup]]` the implicit setup picks exactly what `profile_for_app` picks.
    #[test]
    fn holds_are_limited() {
        assert_eq!(split_hold("o").unwrap(), ("o", 0));
        assert_eq!(split_hold("o@700").unwrap(), ("o", 700));
        assert!(split_hold("o@3001").is_err());
        assert!(split_hold("o@x").is_err());
        let toml = |ms: u64| format!("[[profile]]\nname='A'\n[profile.dial]\npress={{keys={{combo='o', hold={ms}}}}}\n[[pinned]]\nindex=0\nrelease={{keys='p@{ms}'}}\n");
        let ok = Config::parse(&toml(MAX_HOLD_MS), Path::new(".")).unwrap();
        assert!(ok.validate_holds().is_ok());
        // `validate` leaves it to `validate_holds`, so the service can start on such a file.
        let over = Config::parse(&toml(MAX_HOLD_MS + 1), Path::new(".")).unwrap();
        assert!(over.validate_holds().is_err());
    }

    #[test]
    fn implicit_setup_matches_legacy() {
        let cfg = Config::parse(include_str!("../../../examples/config.toml"), Path::new("/x")).unwrap();
        let setups = cfg.effective_setups();
        assert_eq!(setups.len(), 1);
        let s = &setups[0];
        assert_eq!((s.name.as_str(), s.mode()), (IMPLICIT_SETUP, SetupMode::Follow));
        assert_eq!(s.fallback.as_ref(), Some(&cfg.profiles[0].name));
        let names: Vec<&String> = cfg.profiles.iter().skip(1).map(|p| &p.name).collect();
        assert_eq!(s.profiles.iter().collect::<Vec<_>>(), names);
        let pick = |class: &str| {
            s.profiles
                .iter()
                .find(|n| cfg.profiles.iter().any(|p| &p.name == *n && p.apps.iter().any(|a| glob_match(a, class))))
                .or(s.fallback.as_ref())
                .and_then(|n| cfg.profiles.iter().position(|p| &p.name == n))
                .unwrap()
        };
        let classes = cfg.profiles.iter().flat_map(|p| &p.apps).map(|a| a.replace('*', "x")).chain(["kitty".to_owned(), String::new()]);
        for class in classes {
            assert_eq!(pick(&class), cfg.profile_for_app(&class), "{class}");
        }
        assert_eq!(Config::default().effective_setups()[0].fallback, None);
        // An implicit setup's name works in setup actions and except_setups.
        assert!(Config::parse("[[pinned]]\nindex=0\npress={setup='Default'}\nexcept_setups=['Default']\n", Path::new("/")).is_ok());
    }
}

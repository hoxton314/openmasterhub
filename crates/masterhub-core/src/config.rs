//! TOML configuration, shared by the service (reads it) and the GUI (edits it).
//! See `examples/config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use masterhub_proto::Rgb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// TTF/OTF font for labels. Defaults to `fc-match sans:bold`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<PathBuf>,
    /// LCD backlight of keypad and dial, 0..=100.
    #[serde(default = "default_backlight")]
    pub backlight: u8,
    #[serde(default, skip_serializing_if = "Leds::is_empty")]
    pub leds: Leds,
    /// How the Base is placed on the desk. `auto` follows its tilt sensor.
    #[serde(default, skip_serializing_if = "Orientation::is_auto")]
    pub orientation: Orientation,
    /// The first profile is the default; it is used when no other profile matches the
    /// focused application.
    #[serde(default, rename = "profile", skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<Profile>,
}

fn default_backlight() -> u8 {
    80
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    /// Follow the Base's sensor; landscape until it reports.
    #[default]
    Auto,
    Landscape,
    Portrait,
}

impl Orientation {
    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }

    /// Resolve to "is the Base horizontal (landscape)?".
    pub fn horizontal(self, sensor: Option<bool>) -> bool {
        match self {
            Self::Landscape => true,
            Self::Portrait => false,
            Self::Auto => sensor.unwrap_or(true),
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
}

impl Leds {
    pub fn is_empty(&self) -> bool {
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
    pub widget: Option<Widget>,
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
}

impl Widget {
    pub const ALL: [Widget; 8] = [
        Widget::Clock { format: None },
        Widget::CpuUsage,
        Widget::CpuTemp,
        Widget::RamUsage,
        Widget::GpuUsage,
        Widget::GpuTemp,
        Widget::Volume,
        Widget::NowPlaying,
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
    /// Key combination sent through a virtual keyboard, e.g. `"ctrl+shift+m"` or `"playpause"`.
    Keys(String),
    /// Keypad page: `"next"`, `"prev"` or a page name.
    Page(String),
    /// Open a keypad folder by name.
    Folder(String),
    /// Leave the current folder.
    Back(bool),
    /// Switch to a profile by name.
    Profile(String),
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
        let mut cfg: Config = toml::from_str(text)?;
        cfg.validate()?;
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
                    _ => {}
                }
            }
        }
        for action in self.profiles.iter().flat_map(Profile::actions) {
            if let Action::Profile(name) = action
                && !self.profiles.iter().any(|p| &p.name == name)
            {
                bail!("profile {name:?} does not exist");
            }
        }
        Ok(())
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
            .chain(self.dial.iter().flat_map(|d| [&d.cw, &d.ccw, &d.press, &d.release]))
            .chain(self.faders.iter().map(|f| &f.change))
            .chain(self.knobs.iter().flat_map(|k| [&k.cw, &k.ccw, &k.press, &k.release]))
            .chain(self.rollers.iter().flat_map(|r| [&r.away, &r.toward]))
            .flatten()
    }
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

/// `$XDG_CONFIG_HOME/openmasterhub/config.toml`
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
        assert_eq!(key.face.widget, Some(Widget::Clock { format: Some("%H:%M:%S".into()) }));
    }

    #[test]
    fn rejects_dangling_references() {
        let bad_folder = "[[profile]]\nname='A'\n[[profile.page]]\nname='P'\n[[profile.page.key]]\nindex=0\npress={folder='Nope'}\n";
        assert!(Config::parse(bad_folder, Path::new("/")).is_err());
        let bad_index = "[[profile]]\nname='A'\n[[profile.page]]\nname='P'\n[[profile.page.key]]\nindex=15\n";
        assert!(Config::parse(bad_index, Path::new("/")).is_err());
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
}

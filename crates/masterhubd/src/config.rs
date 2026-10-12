//! TOML configuration. See `examples/config.toml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use masterhub_proto::Rgb;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// TTF/OTF font for key labels. Defaults to `fc-match sans:bold`.
    pub font: Option<PathBuf>,
    /// LCD backlight of keypad and dial, 0..=100.
    #[serde(default = "default_backlight")]
    pub backlight: u8,
    #[serde(default)]
    pub leds: Leds,
    #[serde(default, rename = "key")]
    pub keys: Vec<KeyConfig>,
    #[serde(default)]
    pub dial: Option<DialConfig>,
    #[serde(default, rename = "fader")]
    pub faders: Vec<FaderConfig>,
    #[serde(default, rename = "knob")]
    pub knobs: Vec<KnobConfig>,
    #[serde(default, rename = "roller")]
    pub rollers: Vec<RollerConfig>,
}

fn default_backlight() -> u8 {
    80
}

/// Static LED colours. One colour applies to every LED of that module; a list sets them
/// individually (fader: top to bottom, knob: bottom-left, bottom-right, top).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leds {
    pub base: Option<Color>,
    pub fader: Option<OneOrMany<Color>>,
    pub knob: Option<OneOrMany<Color>>,
    /// 1..=100
    pub brightness: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
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

/// Look of a display: background colour, optional picture, optional label.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    pub label: Option<String>,
    pub image: Option<PathBuf>,
    pub background: Option<Color>,
    pub text_color: Option<Color>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyConfig {
    /// 0..=14, row-major from the top-left.
    pub index: u8,
    #[serde(flatten)]
    pub face: Face,
    pub press: Option<Action>,
    pub release: Option<Action>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialConfig {
    #[serde(flatten)]
    pub face: Face,
    pub cw: Option<Action>,
    pub ccw: Option<Action>,
    pub press: Option<Action>,
    pub release: Option<Action>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaderConfig {
    /// 0 = top.
    pub index: u8,
    /// Run with `{value}` replaced by the position 0..=100.
    pub change: Option<Action>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnobConfig {
    /// 0 = bottom-left, 1 = bottom-right, 2 = top.
    pub index: u8,
    pub cw: Option<Action>,
    pub ccw: Option<Action>,
    pub press: Option<Action>,
    pub release: Option<Action>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollerConfig {
    pub side: Side,
    /// Rolled away from you (towards the keys).
    pub away: Option<Action>,
    /// Rolled towards you.
    pub toward: Option<Action>,
}

/// What an input does.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Action {
    /// Shell command run with `sh -c`. `{value}` is replaced for faders.
    Cmd(String),
    /// Key combination sent through a virtual keyboard, e.g. `"ctrl+shift+m"` or `"playpause"`.
    Keys(String),
}

/// `"#rrggbb"`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub Rgb);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_color(&s).map(Color).ok_or_else(|| serde::de::Error::custom(format!("invalid colour {s:?}, expected #rrggbb")))
    }
}

fn parse_color(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        cfg.validate()?;
        // Relative image paths are relative to the config file.
        let base = path.parent().unwrap_or(Path::new("."));
        for face in cfg.keys.iter_mut().map(|k| &mut k.face).chain(cfg.dial.iter_mut().map(|d| &mut d.face)) {
            if let Some(img) = &mut face.image {
                *img = expand(base, img);
            }
        }
        if let Some(font) = &mut cfg.font {
            *font = expand(base, font);
        }
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        let mut seen = HashMap::new();
        for k in &self.keys {
            if k.index > 14 {
                bail!("key index {} out of range 0..=14", k.index);
            }
            if seen.insert(k.index, ()).is_some() {
                bail!("key {} configured twice", k.index);
            }
        }
        if let Some(f) = self.faders.iter().find(|f| f.index > 4) {
            bail!("fader index {} out of range 0..=4", f.index);
        }
        if let Some(k) = self.knobs.iter().find(|k| k.index > 2) {
            bail!("knob index {} out of range 0..=2", k.index);
        }
        if self.backlight > 100 {
            bail!("backlight must be 0..=100");
        }
        Ok(())
    }
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
    fn example_config_parses() {
        let text = include_str!("../../../examples/config.toml");
        let cfg: Config = toml::from_str(text).unwrap();
        cfg.validate().unwrap();
        assert!(!cfg.keys.is_empty());
        assert!(cfg.dial.is_some());
    }

    #[test]
    fn colors_and_actions() {
        assert_eq!(parse_color("#ff8000"), Some(Rgb::new(255, 128, 0)));
        assert_eq!(parse_color("ff8000"), None);
        let cfg: Config = toml::from_str(
            r##"
            [[key]]
            index = 3
            label = "Mute"
            background = "#c81e1e"
            press = { keys = "ctrl+shift+m" }
            "##,
        )
        .unwrap();
        assert_eq!(cfg.keys[0].press, Some(Action::Keys("ctrl+shift+m".into())));
        assert_eq!(cfg.keys[0].face.background, Some(Color(Rgb::new(0xc8, 0x1e, 0x1e))));
    }

    #[test]
    fn rejects_bad_index() {
        let cfg: Config = toml::from_str("[[key]]\nindex = 15\n").unwrap();
        assert!(cfg.validate().is_err());
    }
}

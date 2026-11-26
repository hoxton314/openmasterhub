//! PULPIT's own config (`~/.config/pulpit/pulpit.toml`, see `config/pulpit.example.toml` in the repo).
//! Actions use the engine's action syntax and run through its `run_action` request.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use masterhub_core::config::Action;
use serde::Deserialize;

use crate::icons;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub dial: DialConfig,
    #[serde(default)]
    pub pages: PagesConfig,
    /// The status row: keys shown on every daily page (normally 0..=4).
    #[serde(default)]
    pub status: Vec<KeyConfig>,
    #[serde(default, rename = "page")]
    pub pages_list: Vec<Page>,
    /// The three knobs: volume per app.
    #[serde(default, rename = "knob")]
    pub knobs: Vec<KnobConfig>,
    #[serde(default)]
    pub look: LookConfig,
    #[serde(default)]
    pub lights: LightsConfig,
    #[serde(default)]
    pub screens: ScreensConfig,
    /// Game modes by name (see `games`): `false` switches one off; a game not named is on.
    #[serde(default)]
    pub games: std::collections::BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ScreensConfig {
    /// Dim the key and dial screens after this many minutes without touching the hub (0 = never).
    pub idle_minutes: u32,
    /// Also dim while the desktop is locked (hyprlock).
    pub when_locked: bool,
    /// Backlight while dimmed, 0..=100.
    pub dim: u8,
}

impl Default for ScreensConfig {
    fn default() -> Self {
        Self { idle_minutes: 5, when_locked: true, dim: 5 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LightsConfig {
    /// quiet: knob rings only while turning, the Base edge dark unless something needs you
    /// (a Claude session waits, a KILN job failed). full: rings glow at rest, the Base follows
    /// every Claude / KILN status (masterhubd's colours). off: no lights at all.
    pub mode: String,
    /// Brightness of the lights when they are on, 1..=100.
    pub brightness: u8,
    /// The Base colour when something needs you (it breathes slowly).
    pub attention: String,
}

impl Default for LightsConfig {
    fn default() -> Self {
        Self { mode: "quiet".into(), brightness: 70, attention: "#ff4000".into() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LookConfig {
    /// Colour set for key categories (see `pulpit gallery`): kreatura, ember, neon, daylight, ink.
    pub palette: String,
}

impl Default for LookConfig {
    fn default() -> Self {
        Self { palette: "kreatura".into() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnobConfig {
    pub index: u8,
    /// Shown on the dial while turning, e.g. "MUSIC".
    pub label: String,
    /// App targets in order of preference (engine syntax: "focused", "chrome*",
    /// "node:proton-game"); the first one with an audio stream is used.
    pub apps: Vec<String>,
    /// Ring colour, "#rrggbb".
    pub color: String,
    /// Volume change per click, percent.
    #[serde(default = "default_step")]
    pub step: u8,
}

fn default_step() -> u8 {
    2
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DialConfig {
    /// Master volume change per dial click, in percent.
    pub volume_step: u8,
}

impl Default for DialConfig {
    fn default() -> Self {
        Self { volume_step: 2 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PagesConfig {
    /// The roller that flips daily pages: "left" or "right".
    pub roller: String,
}

impl Default for PagesConfig {
    fn default() -> Self {
        Self { roller: "left".into() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub name: String,
    /// Only in the rotation when: "more-sessions" (a Claude session beyond the status row).
    pub when: Option<String>,
    #[serde(default, rename = "key")]
    pub keys: Vec<KeyConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyConfig {
    pub index: u8,
    #[serde(default)]
    pub label: String,
    /// Icon name from [`icons`].
    pub icon: Option<String>,
    /// App logo (pulpit_render::logo_names), drawn instead of the icon.
    pub logo: Option<String>,
    /// Ground and stripe colour: status, apps, media, tools, capture.
    pub category: Option<String>,
    pub minor: Option<String>,
    /// Window class glob of this key's app: focusing it clears the key's notifications.
    pub class: Option<String>,
    /// Notification app-name / desktop-entry glob: such notifications put an orb on this key.
    pub notify: Option<String>,
    /// For `live = "claude"`: which session (0 = oldest open one).
    pub slot: Option<u8>,
    /// Notifications whose summary matches this glob are not counted (e.g. "*(#*" skips
    /// Discord server channels, so only private messages count).
    pub notify_skip: Option<String>,
    /// Tray item Id glob (e.g. "discord*"): its red unread badge puts an UNREAD orb on the key.
    pub tray: Option<String>,
    /// Live content: "replay" (buffer state), "record" (recording state), "player" (play /
    /// pause icon, mint orb while playing, the track as minor line), "karma-dispatch" (lit
    /// while the Karma dispatcher session runs).
    pub live: Option<String>,
    pub press: Option<Action>,
}

impl KeyConfig {
    pub fn icon_char(&self) -> Option<char> {
        self.icon.as_deref().and_then(icons::glyph)
    }
}

pub const CATEGORIES: [&str; 5] = ["status", "apps", "media", "tools", "capture"];

type Pair = ([u8; 3], [u8; 3]);

/// Colour sets: ground and bottom-stripe per category, in [`CATEGORIES`] order.
pub const PALETTES: &[(&str, [Pair; 5])] = &[
    // Dark tints of the brand colours: kind at a glance, cream text stays legible.
    ("kreatura", [
        ([0x1E, 0x10, 0x29], [0xAF, 0x92, 0xB5]),
        ([0x10, 0x19, 0x30], [0x6A, 0x9B, 0xD8]),
        ([0x0D, 0x21, 0x19], [0x4B, 0xD4, 0x86]),
        ([0x26, 0x1A, 0x08], [0xE8, 0xAA, 0x17]),
        ([0x2A, 0x0E, 0x0C], [0xFF, 0x5A, 0x3C]),
    ]),
    // Warmer and deeper.
    ("ember", [
        ([0x24, 0x0F, 0x22], [0xD4, 0x7F, 0xC8]),
        ([0x0A, 0x20, 0x26], [0x3F, 0xB6, 0xC9]),
        ([0x15, 0x22, 0x0A], [0x9C, 0xCF, 0x4F]),
        ([0x2E, 0x15, 0x05], [0xFF, 0x8A, 0x2B]),
        ([0x30, 0x0A, 0x15], [0xFF, 0x46, 0x70]),
    ]),
    // Near-black grounds; the colour lives in bright stripes.
    ("neon", [
        ([0x10, 0x08, 0x16], [0xC0, 0x8B, 0xFF]),
        ([0x07, 0x0D, 0x16], [0x4F, 0xC3, 0xFF]),
        ([0x06, 0x12, 0x0C], [0x4D, 0xFF, 0xA6]),
        ([0x14, 0x10, 0x04], [0xFF, 0xD2, 0x3F]),
        ([0x16, 0x05, 0x08], [0xFF, 0x3B, 0x5C]),
    ]),
    // Lighter mid-tone grounds: categories tell apart from further away.
    ("daylight", [
        ([0x3A, 0x2A, 0x4A], [0xD9, 0xB8, 0xFF]),
        ([0x24, 0x38, 0x5A], [0x9C, 0xC4, 0xFF]),
        ([0x1F, 0x42, 0x35], [0x8E, 0xF0, 0xB8]),
        ([0x4A, 0x34, 0x12], [0xFF, 0xCF, 0x6B]),
        ([0x4A, 0x1C, 0x1A], [0xFF, 0x9A, 0x85]),
    ]),
    // One ground for all; only the stripe says the category. The calmest.
    ("ink", [
        ([0x12, 0x09, 0x1F], [0xAF, 0x92, 0xB5]),
        ([0x12, 0x09, 0x1F], [0x6A, 0x9B, 0xD8]),
        ([0x12, 0x09, 0x1F], [0x4B, 0xD4, 0x86]),
        ([0x12, 0x09, 0x1F], [0xE8, 0xAA, 0x17]),
        ([0x12, 0x09, 0x1F], [0xFF, 0x5A, 0x3C]),
    ]),
];

/// Ground and stripe of `category` in colour set `palette`.
pub fn colors(palette: &str, category: &str) -> Option<Pair> {
    let i = CATEGORIES.iter().position(|c| *c == category)?;
    PALETTES.iter().find(|(n, _)| *n == palette).map(|(_, p)| p[i])
}

/// Ground and stripe of `category` in the default set.
pub fn category(name: &str) -> Option<Pair> {
    colors("kreatura", name)
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let c: Config = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        c.check()?;
        Ok(c)
    }

    fn check(&self) -> Result<()> {
        crate::games::check(&self.games)?;
        let keys = self.status.iter().chain(self.pages_list.iter().flat_map(|p| p.keys.iter()));
        for k in keys {
            if k.index >= 15 {
                bail!("key index {} out of range 0..=14", k.index);
            }
            if let Some(i) = &k.icon
                && icons::glyph(i).is_none()
            {
                bail!("unknown icon {i:?} (known: {})", icons::names().join(", "));
            }
            if let Some(l) = &k.logo
                && !pulpit_render::logo_names().contains(&l.as_str())
            {
                bail!("unknown logo {l:?} (known: {})", pulpit_render::logo_names().join(", "));
            }
            if let Some(c) = &k.category
                && category(c).is_none()
            {
                bail!("unknown category {c:?} (status, apps, media, tools, capture)");
            }
            if let Some(l) = &k.live
                && !matches!(l.as_str(), "replay" | "record" | "player" | "claude" | "claude-new" | "kiln" | "karma-dispatch")
            {
                bail!("unknown live content {l:?} (replay, record, player, claude, claude-new, kiln, karma-dispatch)");
            }
        }
        for k in &self.knobs {
            if k.index >= 3 {
                bail!("knob index {} out of range 0..=2", k.index);
            }
            if k.apps.is_empty() {
                bail!("knob {} has no apps", k.index);
            }
            if masterhub_core::config::Color::parse(&k.color).is_none() {
                bail!("knob {}: bad color {:?}", k.index, k.color);
            }
        }
        if self.screens.dim > 100 {
            bail!("screens.dim must be 0..=100");
        }
        if !matches!(self.lights.mode.as_str(), "quiet" | "full" | "off") {
            bail!("lights.mode must be quiet, full or off");
        }
        if !(1..=100).contains(&self.lights.brightness) {
            bail!("lights.brightness must be 1..=100");
        }
        if masterhub_core::config::Color::parse(&self.lights.attention).is_none() {
            bail!("lights.attention: bad color {:?}", self.lights.attention);
        }
        if !PALETTES.iter().any(|(n, _)| *n == self.look.palette) {
            bail!("unknown look.palette {:?}", self.look.palette);
        }
        if !matches!(self.pages.roller.as_str(), "left" | "right") {
            bail!("pages.roller must be left or right");
        }
        for p in &self.pages_list {
            if let Some(w) = &p.when
                && w != "more-sessions"
            {
                bail!("page {:?}: unknown when {w:?} (more-sessions)", p.name);
            }
        }
        if self.pages_list.is_empty() {
            bail!("no [[page]]");
        }
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
        base.join("pulpit/pulpit.toml")
    }

    /// The key at `index` on daily page `page`: the status row wins.
    pub fn key(&self, page: usize, index: u8) -> Option<&KeyConfig> {
        self.status
            .iter()
            .find(|k| k.index == index)
            .or_else(|| self.pages_list.get(page)?.keys.iter().find(|k| k.index == index))
    }

    pub fn is_status(&self, index: u8) -> bool {
        self.status.iter().any(|k| k.index == index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_config_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/pulpit.example.toml");
        let c = Config::load(&path).unwrap();
        assert!(!c.pages_list.is_empty());
        assert!(c.is_status(0));
        // the leftmost status key is the Karma dispatcher; the Claude keys after it start at slot 0
        let k0 = c.status.iter().find(|k| k.index == 0).unwrap();
        assert_eq!(k0.live.as_deref(), Some("karma-dispatch"));
        let mut slots: Vec<u8> = c
            .status
            .iter()
            .chain(c.pages_list.iter().flat_map(|p| p.keys.iter()))
            .filter(|k| k.live.as_deref() == Some("claude"))
            .filter_map(|k| k.slot)
            .collect();
        slots.sort();
        assert_eq!(slots, (0..slots.len() as u8).collect::<Vec<_>>(), "no session slot is skipped");
    }
}

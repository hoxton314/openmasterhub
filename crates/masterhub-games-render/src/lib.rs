//! Faces of the game modes (from PULPIT by Dawid Gorczyca): what a key or the dial shows, in
//! each game's look.
//!
//! The app describes a face ([`KeyFace`], [`DialFace`]) and asks the [`Renderer`] for a frame at
//! time `t` (seconds, drives scrolling text and pulsing orbs) with press intensity `press`
//! (1.0 right at the press, falling to 0.0 over [`PRESS_SECS`]).

use image::RgbImage;

mod art;
mod canvas;
mod logos;
mod face;
mod intro;
mod kern;
mod text;

/// The face glyphs used by the sample sheet (Font Awesome Free, solid).
pub mod glyphs {
    pub const SPARKLES: char = '\u{e2ca}';
    pub const OVEN: char = '\u{e4f1}';
    pub const GLOBE: char = '\u{f0ac}';
    pub const REC: char = '\u{f111}';
    pub const EYE_DROPPER: char = '\u{f1fb}';
}

/// The five sample keys and the dial of the style board (`tools/style-samples/samples.py`), for
/// the contact sheet, tests and benches.
pub mod samples {
    use super::{DialFace, KeyFace, Orb, glyphs};

    pub fn keys() -> Vec<KeyFace> {
        let k = |label: &str, icon, ring, orb, minor: &str| KeyFace {
            label: label.into(),
            icon: Some(icon),
            ring,
            orb,
            minor: (!minor.is_empty()).then(|| minor.into()),
            ..Default::default()
        };
        vec![
            k("GROOVER FIX · EDIT RENDER.RS", glyphs::SPARKLES, None, Some(Orb::Working), "4 MIN"),
            k("KILN", glyphs::OVEN, Some(0.62), None, "62% · SHAPE"),
            k("CHROME", glyphs::GLOBE, None, None, ""),
            k("SAVE CLIP", glyphs::REC, None, Some(Orb::Recording), "BUFFER 30S"),
            k("COLOUR", glyphs::EYE_DROPPER, None, None, ""),
        ]
    }

    pub fn dial() -> DialFace {
        DialFace {
            card: None,
            value: 0.62,
            big: "62".into(),
            name: "VOLUME".into(),
            status: Some("CLAUDE WAITS".into()),
        }
    }
}

pub use face::crimson::{CrimsonFont, CrimsonFrame, CrimsonIcons, CrimsonPress, CrimsonRing, CrimsonStyle};
pub use face::deadlock::{DeadlockFrame, DeadlockPress, DeadlockRing, DeadlockStyle};
pub use face::wow::{WowFrame, WowPress, WowRing, WowStyle};
pub use face::cyber::{CyberStyle, Frame, IconFx, Numbers, PressFx, Ring};
pub use art::set_mask_dir;
pub use intro::{GAP as INTRO_GAP, IntroStyle};
pub use text::{set_deadlock_font, set_wow_src};

pub const KEY_SIZE: u32 = 72;
pub const DIAL_SIZE: u32 = 240;
/// How long the press animation lasts.
pub const PRESS_SECS: f32 = 0.35;

/// The status orb at a key's top right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orb {
    /// Marigold pulse.
    Working,
    /// Red breath (replay buffer running, recording).
    Recording,
    /// Steady mint.
    Ready,
    /// Red flash.
    Attention,
    /// Red, breathing slowly: unread notifications in that app.
    Notify,
}

/// How a key answers a press (see [`press_styles`] for the gallery names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PressStyle {
    /// The approved look: ground lifts, a lavender glare sweeps across, the icon grows marigold.
    #[default]
    Glare,
    /// Rings expand from the icon past the key's edge; the icon brightens.
    Ripple,
    /// The ground flashes cream and the face inverts to plum-deep, then fades back.
    Flash,
    /// The icon squashes, overshoots and settles on a damped spring; the ground stays.
    Spring,
    /// An anime impact frame: a held ink frame, a burst and sharp shards that fly out.
    Impact,
    /// Slats in the accent colour cover the key and slide off in turn.
    Shutter,
}

/// Every press style with a short upper-case name for labels.
pub fn press_styles() -> &'static [(PressStyle, &'static str)] {
    &[
        (PressStyle::Glare, "GLARE"),
        (PressStyle::Ripple, "RIPPLE"),
        (PressStyle::Flash, "FLASH"),
        (PressStyle::Spring, "SPRING"),
        (PressStyle::Impact, "IMPACT"),
        (PressStyle::Shutter, "SHUTTER"),
    ]
}

/// What a key shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeyFace {
    /// Top line, the important text. Scrolls when it does not fit.
    pub label: String,
    /// A glyph from Font Awesome Free (solid), e.g. `'\u{f0ac}'` (globe).
    pub icon: Option<char>,
    /// Ground colour (sRGB) for the key's category; plum-deep when `None`.
    pub ground: Option<[u8; 3]>,
    /// A 2 px stripe along the bottom edge in the category's accent colour.
    pub accent: Option<[u8; 3]>,
    /// An app logo by name (see [`logo_names`]); drawn instead of `icon`.
    pub logo: Option<String>,
    /// Progress ring 0.0..=1.0 around the icon; only for things with progress.
    pub ring: Option<f32>,
    pub orb: Option<Orb>,
    /// Bottom line, minor info.
    pub minor: Option<String>,
    /// The press animation; its effect colour is `accent` when set, else marigold.
    pub press_style: PressStyle,
}

/// What the dial shows (the Gauge scene).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialFace {
    /// Gauge (a value: volume) or a status card.
    pub card: Option<DialCard>,
    /// 0.0..=1.0, the marigold arc.
    pub value: f32,
    /// The big number or word in the middle, e.g. "62".
    pub big: String,
    /// What the value is, e.g. "VOLUME".
    pub name: String,
    /// The most urgent status, pulsing, e.g. "CLAUDE WAITS".
    pub status: Option<String>,
}

/// A live-status card on the dial: a thin ring around the edge (progress or breathing), a
/// small label on top, a big value or title (scrolls when long), a subtitle and a facts line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialCard {
    pub eyebrow: String,
    pub big: String,
    pub sub: Option<String>,
    pub line: Option<String>,
    /// Ring colour (sRGB); `None` = only the quiet track.
    pub ring: Option<[u8; 3]>,
    /// Progress 0..=1 drawn clockwise from the top; `None` = the whole ring.
    pub progress: Option<f32>,
    /// The ring breathes (something wants you).
    pub breathe: bool,
    /// A small dot running round the track once a minute (the resting clock).
    pub seconds_dot: bool,
    /// A map picture under the card, for looks that draw one (WoW: a `maps/` stem of the
    /// extracted game files, shown as the minimap).
    pub map: Option<String>,
}

/// A whole look for keys and the dial: daily mode's, or a game's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Kreatura,
    /// Cyberpunk 2077, assembled from [`CyberStyle`] parts.
    Cyber(CyberStyle),
    /// Crimson Desert, assembled from [`CrimsonStyle`] parts.
    Crimson(CrimsonStyle),
    /// Deadlock, assembled from [`DeadlockStyle`] parts.
    Deadlock(DeadlockStyle),
    /// WoW Forever, assembled from [`WowStyle`] parts (pictures and fonts from the game's files).
    Wow(WowStyle),
}

/// Fonts and caches; create once.
pub struct Renderer {
    painter: face::Painter,
}

impl Renderer {
    /// Load the fonts (see docs/STYLE.md).
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self { painter: face::Painter::new()? })
    }

    /// A 72×72 key frame.
    pub fn key(&self, face: &KeyFace, t: f32, press: f32) -> RgbImage {
        self.painter.key(face, t, press).into_image()
    }

    /// A 72×72 key frame in `theme`.
    pub fn key_in(&self, theme: Theme, face: &KeyFace, t: f32, press: f32) -> RgbImage {
        match theme {
            Theme::Kreatura => self.key(face, t, press),
            Theme::Cyber(st) => self.painter.key_cyber(&st, face, t, press).into_image(),
            Theme::Crimson(st) => self.painter.key_crimson(&st, face, t, press).into_image(),
            Theme::Deadlock(st) => self.painter.key_deadlock(&st, face, t, press).into_image(),
            Theme::Wow(st) => self.painter.key_wow(&st, face, t, press).into_image(),
        }
    }

    /// A 240×240 dial frame in `theme`.
    pub fn dial_in(&self, theme: Theme, face: &DialFace, t: f32) -> RgbImage {
        match theme {
            Theme::Kreatura => self.dial(face, t),
            Theme::Cyber(st) => self.painter.dial_cyber(&st, face, t).into_image(),
            Theme::Crimson(st) => self.painter.dial_crimson(&st, face, t).into_image(),
            Theme::Deadlock(st) => self.painter.dial_deadlock(&st, face, t).into_image(),
            Theme::Wow(st) => self.painter.dial_wow(&st, face, t).into_image(),
        }
    }

    /// Whether `face` changes over time when not pressed (scrolling text, orb, …), so the app
    /// knows which keys to redraw every frame.
    pub fn key_moves(&self, face: &KeyFace) -> bool {
        face.orb.is_some() || self.painter.label_scrolls(face) || self.painter.minor_scrolls(face)
    }

    /// Frame `t` of a `dur`-second game intro: 15 key images (row-major) and the dial, cut
    /// from one picture spread over the whole keypad.
    pub fn intro(&self, style: IntroStyle, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        self.painter.intro(style, t, dur)
    }

    /// A 240×240 dial frame (black outside the circle).
    pub fn dial(&self, face: &DialFace, t: f32) -> RgbImage {
        self.painter.dial(face, t).into_image()
    }
}

#[cfg(test)]
mod tests;

/// Drawn icon names a game look can use through [`KeyFace::logo`].
pub fn art_names() -> Vec<&'static str> {
    art::names()
}

/// Logo names a [`KeyFace`] can use.
pub fn logo_names() -> Vec<&'static str> {
    logos::names()
}

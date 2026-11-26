//! The drawing itself, after `tools/style-samples/samples.py` (`key_mix` variant 3 and
//! `dial_gauge`). Pillow places shapes by pixel boxes, so a shape "centred on pixel 36" has its
//! continuous centre at 36.5 while text anchors sit at 36.0; both are kept as in the prototype.

use tiny_skia::FillRule;

use crate::canvas::{Canvas, Clip, Cover, Rgb, annulus, arc, circle, mix, rgb};
use crate::text::{Anchor, FontId, Fonts};
use crate::{DialFace, KeyFace, Orb, PressStyle};

pub mod crimson;
pub mod cyber;
pub mod deadlock;
pub mod wow;
mod press;
use press::Fx;

pub const PLUM_DEEP: Rgb = rgb(0x12091f);
pub const PLUM: Rgb = rgb(0x231b2c);
pub const RING: Rgb = rgb(0x47355e);
pub const CREAM: Rgb = rgb(0xded19f);
pub const MARIGOLD: Rgb = rgb(0xe8aa17);
pub const LAVENDER: Rgb = rgb(0xaf92b5);
pub const MINT: Rgb = rgb(0x4bd486);
pub const REC: Rgb = rgb(0xff5a3c);
pub const PIX_DIM: Rgb = rgb(0x6a7b9e);
/// The engraved label's shadow.
const SHADOW: Rgb = rgb(0x050208);
const MARIGOLD_DIM: Rgb = rgb(0x5a4008);
const REC_DIM: Rgb = rgb(0x602018);

const K: usize = 72;
const LABEL_SIZE: f32 = 11.0;
const MINOR_SIZE: f32 = 10.0;
const SCROLL_SPEED: f32 = 24.0;
const SCROLL_GAP: f32 = 24.0;

/// The strip a line of text lives in: clip, centre when it fits, widest text that stays still,
/// scroll start padding.
struct Strip {
    x0: f32,
    width: f32,
    pad: f32,
    fit: f32,
    centre: f32,
}

const TOP_WITH_ORB: Strip = Strip { x0: 1.0, width: 58.0, pad: 2.0, fit: 54.0, centre: 31.0 };
const TOP_FULL: Strip = Strip { x0: 0.0, width: 72.0, pad: 4.0, fit: 64.0, centre: 36.0 };

impl Strip {
    fn fits(&self, w: f32) -> bool {
        w <= self.fit
    }
}

pub struct Painter {
    pub fonts: Fonts,
    /// The orb's blurred glow, unsunk and sunk by 1 px.
    orb_glow: [Cover; 2],
    dial_disc: Cover,
}

impl Painter {
    pub fn new() -> anyhow::Result<Self> {
        let glow = |oy: f32| {
            let mut c = Cover::new(K, K);
            if let Some(p) = circle(64.5, 8.5 + oy, 4.5) {
                c.fill_path(&p, FillRule::Winding, 1.0);
            }
            c.blur(3.0);
            c
        };
        let d = crate::DIAL_SIZE as usize;
        let disc = circle(d as f32 / 2.0, d as f32 / 2.0, d as f32 / 2.0)
            .map(|p| Cover::from_path(d, d, &p, FillRule::Winding))
            .unwrap_or_else(|| Cover::new(d, d));
        Ok(Self { fonts: Fonts::load()?, orb_glow: [glow(0.0), glow(1.0)], dial_disc: disc })
    }

    pub(crate) fn dial_disc(&self) -> &Cover {
        &self.dial_disc
    }

    fn label_strip(face: &KeyFace) -> &'static Strip {
        if face.orb.is_some() { &TOP_WITH_ORB } else { &TOP_FULL }
    }

    pub fn label_scrolls(&self, face: &KeyFace) -> bool {
        !face.label.is_empty()
            && !Self::label_strip(face).fits(self.fonts.width(FontId::Chakra, LABEL_SIZE, &face.label))
    }

    pub fn minor_scrolls(&self, face: &KeyFace) -> bool {
        face.minor.as_deref().is_some_and(|m| {
            !m.is_empty() && !TOP_FULL.fits(self.fonts.width(FontId::Mono, MINOR_SIZE, m))
        })
    }

    pub fn key(&self, face: &KeyFace, t: f32, press: f32) -> Canvas {
        let p = if press.is_finite() { press.clamp(0.0, 1.0) } else { 0.0 };
        let ground = face.ground.map_or(PLUM_DEEP, rgbf);
        let effect = face.accent.map_or(MARIGOLD, rgbf);
        let fx = Fx::new(face.press_style, p, ground, effect);
        let oy = fx.oy;
        let mut c = Canvas::new(K, K, fx.ground);
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };

        let has_top = !face.label.is_empty();
        let logo = face.logo.as_deref().filter(|l| crate::logos::exists(l));
        let has_mid = face.icon.is_some() || logo.is_some() || face.ring.is_some();
        let minor = face.minor.as_deref().filter(|m| !m.is_empty());
        // Where the ring and icon sit.
        let cy = if has_top {
            39.0
        } else if minor.is_some() {
            34.0
        } else {
            36.0
        } + oy;

        // Effects on the ground, under everything else (Glare's band, ripples, impact shards).
        if p > 0.0 {
            match face.press_style {
                PressStyle::Glare => glare_band(&mut c, p),
                PressStyle::Ripple => press::ripple(&mut c, p, effect, cy),
                PressStyle::Impact => press::impact(&mut c, p, effect, cy),
                PressStyle::Flash | PressStyle::Spring | PressStyle::Shutter => {}
            }
        }

        // Status orb, top right, with a soft glow.
        if let Some(orb) = face.orb {
            let col = orb_color(orb, t);
            c.add(&self.orb_glow[oy as usize], col);
            if let Some(dot) = circle(64.5, 8.5 + oy, 3.5) {
                c.paint(&Cover::from_path(K, K, &dot, FillRule::Winding), col, 1.0);
            }
        }

        // Top line: engraved label, scrolling when it does not fit.
        if has_top {
            let y = if has_mid {
                9.0
            } else if minor.is_some() {
                32.0
            } else {
                36.0
            } + oy;
            let strip = Self::label_strip(face);
            // The engraved shadow only on still labels, as in the prototype's marquee.
            if fx.shadow && !self.label_scrolls(face) {
                self.line(&mut c, FontId::Chakra, LABEL_SIZE, &face.label, strip, y + 1.0, SHADOW, t);
            }
            self.line(&mut c, FontId::Chakra, LABEL_SIZE, &face.label, strip, y, fx.text, t);
        }

        // Ring and icon.
        if let Some(v) = face.ring {
            if let Some(path) = annulus(36.5, cy + 0.5, 15.5, 18.5) {
                c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), fx.track, 1.0);
            }
            let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
            if let Some(path) = arc(36.5, cy + 0.5, 17.0, 3.0, -90.0, -90.0 + 360.0 * v) {
                c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), MINT, 1.0);
            }
        }
        if let Some(name) = logo {
            // Logos fill their whole box; a bit smaller than the glyph size reads the same.
            let size = fx.icon_size(if face.ring.is_some() { 16.0 } else { 22.0 });
            if let Some(cov) = crate::logos::cover(name, K, K, size, 36.0, cy) {
                if let Some(g) = fx.glow {
                    let mut glow = cov.clone();
                    glow.blur(5.0);
                    c.add(&glow, g);
                }
                c.paint(&cov, fx.icon, 1.0);
            }
        } else if let Some(icon) = face.icon.filter(|&i| self.fonts.has_glyph(FontId::Icons, i)) {
            let size = fx.icon_size(if face.ring.is_some() { 18.0 } else { 24.0 });
            let s = icon.encode_utf8(&mut [0; 4]).to_owned();
            if let Some(g) = fx.glow {
                let mut glow = self.fonts.cover(K, K, FontId::Icons, size, &s, 36.0, cy);
                glow.blur(5.0);
                c.add(&glow, g);
            }
            self.fonts.draw(&mut c, FontId::Icons, size, &s, 36.0, cy, Anchor::Middle, fx.icon, full);
        }

        // Category stripe along the bottom edge.
        if let Some(a) = face.accent
            && let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 70.0, K as f32, 2.0)
        {
            c.paint(&Cover::from_path(K, K, &tiny_skia::PathBuilder::from_rect(rect), FillRule::Winding), rgbf(a), 1.0);
        }

        // Bottom line: minor info.
        if let Some(m) = minor {
            self.line(&mut c, FontId::Mono, MINOR_SIZE, m, &TOP_FULL, 66.0 + oy, fx.minor, t);
        }

        // Effects over the whole face.
        if p > 0.0 && face.press_style == PressStyle::Shutter {
            press::shutter(&mut c, p, effect);
        }
        c
    }

    /// One line of text in `strip`: centred when it fits, else a marquee clipped to the strip.
    #[allow(clippy::too_many_arguments)]
    fn line(&self, c: &mut Canvas, font: FontId, size: f32, text: &str, strip: &Strip, y: f32, col: Rgb, t: f32) {
        let w = self.fonts.width(font, size, text);
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };
        if strip.fits(w) {
            self.fonts.draw(c, font, size, text, strip.centre, y, Anchor::Middle, col, full);
            return;
        }
        let clip = Clip {
            x0: strip.x0 as i32,
            y0: 0,
            x1: (strip.x0 + strip.width) as i32,
            y1: K as i32,
        };
        let span = w + SCROLL_GAP;
        let x = strip.x0 + strip.pad - (t.max(0.0) * SCROLL_SPEED) % span;
        for dx in [0.0, span] {
            self.fonts.draw(c, font, size, text, x + dx, y, Anchor::Left, col, clip);
        }
    }

    pub fn dial(&self, face: &DialFace, t: f32) -> Canvas {
        if let Some(card) = &face.card {
            return self.dial_card(card, t);
        }
        let d = crate::DIAL_SIZE as usize;
        let mut c = Canvas::new(d, d, PLUM);
        let all = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        let v = if face.value.is_finite() { face.value.clamp(0.0, 1.0) } else { 0.0 };
        // Pillow box (22, 22, 218, 218), width 14: outer radius 98.5 around 120.5.
        if let Some(path) = arc(120.5, 120.5, 91.5, 14.0, 135.0, 405.0) {
            c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), RING, 1.0);
        }
        if let Some(path) = arc(120.5, 120.5, 91.5, 14.0, 135.0, 135.0 + 270.0 * v) {
            c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), MARIGOLD, 1.0);
        }
        let f = &self.fonts;
        f.draw(&mut c, FontId::Bebas, 78.0, &face.big, 120.0, 108.0, Anchor::Middle, CREAM, all);
        f.draw(&mut c, FontId::Chakra, 15.0, &face.name, 120.0, 152.0, Anchor::Middle, LAVENDER, all);
        if let Some(s) = face.status.as_deref() {
            let k = 0.55 + 0.45 * (t * 3.0).sin();
            f.draw(&mut c, FontId::Bebas, 20.0, s, 120.0, 190.0, Anchor::Middle, mix(PLUM, MARIGOLD, k), all);
        }
        c.mask_black(&self.dial_disc);
        c
    }
}

impl Painter {
    fn dial_card(&self, card: &crate::DialCard, t: f32) -> Canvas {
        let d = crate::DIAL_SIZE as usize;
        let mut c = Canvas::new(d, d, PLUM);
        let all = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        // The quiet track, then the coloured part.
        if let Some(path) = arc(120.5, 120.5, 104.0, 6.0, 0.0, 360.0) {
            c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), RING, 1.0);
        }
        if let Some(col) = card.ring.map(rgbf) {
            let col = if card.breathe { mix(mix(PLUM, col, 0.25), col, 0.5 + 0.5 * (t * 2.2).sin()) } else { col };
            let (a0, a1) = match card.progress {
                Some(p) => (-90.0, -90.0 + 360.0 * if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.0 }),
                None => (0.0, 360.0),
            };
            if a1 > a0
                && let Some(path) = arc(120.5, 120.5, 104.0, 6.0, a0, a1)
            {
                c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), col, 1.0);
            }
        }
        if card.seconds_dot {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |x| x.as_secs_f32());
            let a = (secs % 60.0) / 60.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            if let Some(dot) = circle(120.5 + 104.0 * a.cos(), 120.5 + 104.0 * a.sin(), 5.0) {
                c.paint(&Cover::from_path(d, d, &dot, FillRule::Winding), MARIGOLD, 1.0);
            }
        }
        let f = &self.fonts;
        f.draw(&mut c, FontId::Chakra, 15.0, &card.eyebrow, 120.0, 66.0, Anchor::Middle, LAVENDER, all);
        // Short values big (a clock, a percentage), words smaller; long ones scroll.
        let big_size = if card.big.chars().count() <= 5 { 78.0 } else { 46.0 };
        self.dial_line(&mut c, FontId::Bebas, big_size, &card.big, 112.0, CREAM, t, 176.0);
        if let Some(sub) = card.sub.as_deref() {
            self.dial_line(&mut c, FontId::Chakra, 15.0, sub, 160.0, CREAM, t, 170.0);
        }
        if let Some(line) = card.line.as_deref() {
            self.dial_line(&mut c, FontId::Mono, 12.0, line, 186.0, PIX_DIM, t, 140.0);
        }
        c.mask_black(&self.dial_disc);
        c
    }

    /// Centred text on the dial; wider than `max` it scrolls inside a centred window.
    #[allow(clippy::too_many_arguments)]
    fn dial_line(&self, c: &mut Canvas, font: FontId, size: f32, text: &str, y: f32, col: Rgb, t: f32, max: f32) {
        let f = &self.fonts;
        let w = f.width(font, size, text);
        let all = Clip { x0: 0, y0: 0, x1: crate::DIAL_SIZE as i32, y1: crate::DIAL_SIZE as i32 };
        if w <= max {
            f.draw(c, font, size, text, 120.0, y, Anchor::Middle, col, all);
            return;
        }
        let x0 = 120.0 - max / 2.0;
        let clip = Clip { x0: x0 as i32, y0: 0, x1: (x0 + max) as i32, y1: crate::DIAL_SIZE as i32 };
        let span = w + 40.0;
        let off = (t * 30.0) % span;
        for k in [0.0, span] {
            f.draw(c, font, size, text, x0 - off + k, y, Anchor::Left, col, clip);
        }
    }
}

fn rgbf(c: [u8; 3]) -> Rgb {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

/// Glare, the approved default: a soft lavender band sweeping from top left to bottom right.
fn glare_band(c: &mut Canvas, p: f32) {
    let x = -40.0 + (1.0 - p) * 150.0;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x, 0.0);
    pb.line_to(x + 22.0, 0.0);
    pb.line_to(x - 50.0, 72.0);
    pb.line_to(x - 72.0, 72.0);
    pb.close();
    if let Some(path) = pb.finish() {
        let mut band = Cover::from_path(K, K, &path, FillRule::Winding);
        band.scale(110.0 / 255.0);
        band.blur(5.0);
        c.paint(&band, LAVENDER, 1.0);
    }
}

pub fn orb_color(orb: Orb, t: f32) -> Rgb {
    match orb {
        Orb::Working => mix(MARIGOLD_DIM, MARIGOLD, 0.5 + 0.5 * (t * 6.0).sin()),
        Orb::Recording => mix(REC_DIM, REC, 0.5 + 0.5 * (t * 4.0).sin()),
        Orb::Ready => MINT,
        // Red, breathing slowly (slower and brighter at rest than Recording): a message waits.
        Orb::Notify => mix(REC_DIM, REC, 0.7 + 0.3 * (t * 2.0).sin()),
        // On/off at about 2 Hz.
        Orb::Attention => {
            if (t * 4.0).floor().rem_euclid(2.0) < 1.0 {
                REC
            } else {
                REC_DIM
            }
        }
    }
}

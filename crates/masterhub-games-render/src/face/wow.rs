//! The game look for WoW Forever, built from the game's own material: its spell and item icons
//! (full colour), Friz Quadrata and Morpheus, the XP bar's purple, the minimap's bronze and the
//! zone map itself. Everything is read at run time from the files extracted by
//! the user's own install by `masterhub-games setup wow_forever` (`set_wow_src`
//! overrides); it is the game's artwork, for this hub only, never published or committed.
//! Parts have variants ([`WowStyle`]) for a proposals board.
//!
//! A key's `logo` names a picture as `wow:<icon file stem>` (e.g. `wow:spell_holy_holybolt`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use image::RgbaImage;
use tiny_skia::FillRule;

use super::{Painter, rgbf};
use crate::canvas::{Canvas, Clip, Cover, Rgb, arc, circle, mix, rgb};
use crate::text::{Anchor, FontId};
use crate::{DialCard, DialFace, KeyFace};

const K: usize = 72;
const D: usize = crate::DIAL_SIZE as usize;
pub(crate) const INK: Rgb = rgb(0x0b0a0d);
/// Tooltip navy and its grey-blue edge.
const NAVY: Rgb = rgb(0x0a0d1f);
const EDGE: Rgb = rgb(0x8a93a8);
/// The interface's gold text (`NORMAL_FONT_COLOR`, 1.0 / 0.82 / 0).
pub(crate) const GOLD: Rgb = rgb(0xffd100);
pub(crate) const WHITE: Rgb = rgb(0xf4f1ea);
const PARCH: Rgb = rgb(0xd8c39a);
const PARCH_DARK: Rgb = rgb(0x8a6a3c);
pub(crate) const BROWN: Rgb = rgb(0x2b1a0b);
/// The XP bar: earned purple, rested blue.
pub(crate) const XP: Rgb = rgb(0x9a4dff);
const RESTED: Rgb = rgb(0x2f6bff);
pub(crate) const BRONZE: Rgb = rgb(0xb08a4a);
const BRONZE_HI: Rgb = rgb(0xf1d79a);
const BRONZE_LO: Rgb = rgb(0x4a3418);
pub(crate) const HOLY: Rgb = rgb(0xffe6a0);
pub(crate) const ARCANE: Rgb = rgb(0x6fd0ff);

/// The key's build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WowFrame {
    /// An action button: the icon fills the key, the name on a dark band, the key's bind
    /// small in the top corner.
    #[default]
    Button,
    /// A tooltip: navy glass, grey-blue edge, the icon in the middle, the name in gold.
    Tooltip,
    /// A page of the quest log: parchment, the icon above, the name in Morpheus ink.
    Parchment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WowPress {
    /// The button's own feedback: the yellow pushed highlight, then a cooldown sweep.
    #[default]
    Cast,
    /// Holy light: golden rays burst from the icon and a white bloom fades.
    Holy,
    /// A spell alert: the golden marching glow around the edge flares and fades.
    Proc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WowRing {
    /// The XP bar bent into a ring: purple fill, twenty notches, the rested part in blue ahead.
    #[default]
    Xp,
    /// The minimap: a bronze rim with its north marker round the zone's own map.
    Minimap,
    /// The hearthstone: dark stone with blue runes that light up as the ring fills.
    Rune,
}

/// One assembled WoW look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WowStyle {
    pub frame: WowFrame,
    pub press: WowPress,
    pub ring: WowRing,
}

fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Smooth value noise in 0..1.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (x - xi, y - yi);
    let s = |v: f32| v * v * (3.0 - 2.0 * v);
    let g = |dx: f32, dy: f32| rnd((xi + dx) as i32 as u32 ^ seed, ((yi + dy) as i32 as u32).wrapping_mul(31));
    let a = g(0.0, 0.0) + (g(1.0, 0.0) - g(0.0, 0.0)) * s(fx);
    let b = g(0.0, 1.0) + (g(1.0, 1.0) - g(0.0, 1.0)) * s(fx);
    a + (b - a) * s(fy)
}

/// The folder of extracted game files.
pub(crate) fn src_dir() -> Option<PathBuf> {
    crate::text::wow_src()
}

/// A picture from the extracted files (`icons/<name>.png`, `maps/<name>.png`), resized to
/// `w`×`h`, cached.
pub(crate) fn picture(folder: &str, name: &str, w: u32, h: u32) -> Option<RgbaImage> {
    type Cache = Mutex<HashMap<(String, u32, u32), Option<RgbaImage>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let key = (format!("{folder}/{name}"), w, h);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().ok()?.get(&key) {
        return v.clone();
    }
    let img = image::open(src_dir()?.join(folder).join(format!("{name}.png")))
        .ok()
        .map(|i| image::imageops::resize(&i.to_rgba8(), w, h, image::imageops::FilterType::Lanczos3));
    let mut c = cache.lock().ok()?;
    if c.len() > 512 {
        c.clear();
    }
    c.insert(key, img.clone());
    img
}


/// Composite `img` with its top left at (x, y), scaled by `alpha`, multiplied by `tint`.
pub(crate) fn blit(c: &mut Canvas, img: &RgbaImage, x: i32, y: i32, alpha: f32, tint: Rgb) {
    for (ix, iy, p) in img.enumerate_pixels() {
        let (tx, ty) = (x + ix as i32, y + iy as i32);
        if tx < 0 || ty < 0 || tx as usize >= c.w || ty as usize >= c.h {
            continue;
        }
        let a = p.0[3] as f32 / 255.0 * alpha;
        if a <= 0.0 {
            continue;
        }
        let src = [p.0[0] as f32 * tint[0] / 255.0, p.0[1] as f32 * tint[1] / 255.0, p.0[2] as f32 * tint[2] / 255.0];
        let d = &mut c.px[ty as usize * c.w + tx as usize];
        *d = mix(*d, src, a.min(1.0));
    }
}

/// A rectangle's coverage (whole pixels).
fn rect_cover(w: usize, h: usize, x0: f32, y0: f32, x1: f32, y1: f32) -> Cover {
    let mut c = Cover::new(w, h);
    for y in (y0.max(0.0) as usize)..(y1.min(h as f32) as usize) {
        for x in (x0.max(0.0) as usize)..(x1.min(w as f32) as usize) {
            c.v[y * w + x] = 1.0;
        }
    }
    c
}

/// Parchment: warm paper with blotches, fibres and burnt edges.
pub(crate) fn parchment(c: &mut Canvas, seed: u32) {
    let (w, h) = (c.w as f32, c.h as f32);
    for y in 0..c.h {
        for x in 0..c.w {
            let (fx, fy) = (x as f32, y as f32);
            let n = noise(fx / 9.0, fy / 9.0, seed) * 0.6 + noise(fx / 3.0, fy / 3.0, seed + 7) * 0.25 + rnd(x as u32 + seed, y as u32) * 0.15;
            let edge = {
                let dx = (fx.min(w - 1.0 - fx) / (w * 0.22)).min(1.0);
                let dy = (fy.min(h - 1.0 - fy) / (h * 0.22)).min(1.0);
                1.0 - dx.min(dy)
            };
            let col = mix(PARCH, PARCH_DARK, (n * 0.55 + edge * edge * 0.8).min(1.0));
            c.px[y * c.w + x] = mix(col, BROWN, (edge.powi(4) * 0.9).min(1.0));
        }
    }
}

impl Painter {
    /// A key in the WoW look. `ground` / `accent` tint the frame for keys without a game icon.
    pub(crate) fn key_wow(&self, st: &WowStyle, face: &KeyFace, t: f32, press: f32) -> Canvas {
        let p = if press.is_finite() { press.clamp(0.0, 1.0) } else { 0.0 };
        let ground = face.ground.map_or(rgb(0x1c1a22), rgbf);
        let accent = face.accent.map_or(GOLD, rgbf);
        let mut c = Canvas::new(K, K, INK);
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };
        let icon = face.logo.as_deref().and_then(|l| l.split('|').find_map(|n| n.strip_prefix("wow:").map(str::to_owned)));
        let readout = icon.is_none() && face.icon.is_none_or(|i| i == ' ') && face.minor.as_deref().is_some_and(|m| !m.is_empty());

        // (icon box: x, y, size), label baseline middle, label colour, label face.
        let (ix, iy, is, label_y, label_ink, font);
        match st.frame {
            WowFrame::Button => {
                c = Canvas::new(K, K, mix(INK, ground, 0.9));
                (ix, iy, is) = (0, 0, K as u32);
                (label_y, label_ink, font) = (63.0, WHITE, FontId::Friz);
            }
            WowFrame::Tooltip => {
                // Glass: navy, a touch lighter at the top, the grey-blue edge.
                for y in 0..K {
                    let col = mix(mix(NAVY, ground, 0.25), NAVY, y as f32 / K as f32);
                    for x in 0..K {
                        c.px[y * K + x] = col;
                    }
                }
                if let Some(path) = rounded_ring(1.0, 1.0, 71.0, 71.0, 5.0, 1.6) {
                    c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), EDGE, 1.0);
                }
                (ix, iy, is) = (18, 9, 36);
                (label_y, label_ink, font) = (59.0, mix(GOLD, accent, 0.15), FontId::Friz);
            }
            WowFrame::Parchment => {
                parchment(&mut c, 3);
                (ix, iy, is) = (19, 7, 34);
                (label_y, label_ink, font) = (58.0, BROWN, FontId::Morpheus);
            }
        }

        // The picture: the game's icon, else a drawn glyph on the key's colours.
        let pop = if st.press == WowPress::Cast { 1.0 } else { 1.0 + 0.12 * (p * std::f32::consts::PI).sin() };
        let ps = (is as f32 * pop).round() as u32;
        let (px0, py0) = (ix as i32 - (ps as i32 - is as i32) / 2, iy as i32 - (ps as i32 - is as i32) / 2);
        if let Some(img) = icon.as_deref().and_then(|n| picture("icons", n, ps, ps)) {
            blit(&mut c, &img, px0, py0, 1.0, [255.0; 3]);
            if st.frame != WowFrame::Button {
                // A thin dark rim so the icon sits in its frame.
                if let Some(path) = rounded_ring(px0 as f32 - 1.0, py0 as f32 - 1.0, (px0 + ps as i32) as f32 + 1.0, (py0 + ps as i32) as f32 + 1.0, 3.0, 1.2) {
                    c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), INK, 0.8);
                }
            }
        } else if icon.is_some() {
            // A slot whose picture is not there (yet): the game's empty action slot.
            if let Some(img) = picture("ui", "ui-quickslot2", 112, 112) {
                blit(&mut c, &img, -20, -20, 1.0, [255.0; 3]);
            } else {
                let r = rect_cover(K, K, 6.0, 6.0, 66.0, 66.0);
                c.paint(&r, rgb(0x050407), 0.8);
            }
        } else if readout {
            let v = face.minor.clone().unwrap_or_default();
            let col = if st.frame == WowFrame::Parchment { BROWN } else { mix(WHITE, accent, 0.5) };
            self.wow_line(&mut c, FontId::Friz, 22.0, &v, 36.0, if st.frame == WowFrame::Button { 30.0 } else { 27.0 }, col, 64.0, t);
        } else if let Some(g) = face.icon.filter(|&i| i != ' ' && self.fonts.has_glyph(FontId::Icons, i)) {
            if st.frame == WowFrame::Button {
                // A plain key: a gem of the key's colour like an empty bar slot.
                if let Some(path) = circle(36.0, 28.0, 20.0) {
                    let mut cv = Cover::from_path(K, K, &path, FillRule::Winding);
                    c.paint(&cv, mix(ground, accent, 0.25), 1.0);
                    cv.blur(4.0);
                    c.add(&cv, mix([0.0; 3], accent, 0.15));
                }
            }
            let s = g.encode_utf8(&mut [0; 4]).to_owned();
            let ink = if st.frame == WowFrame::Parchment { BROWN } else { accent };
            let cy = if st.frame == WowFrame::Button { 28.0 } else { iy as f32 + is as f32 / 2.0 };
            self.fonts.draw(&mut c, FontId::Icons, 22.0 * pop, &s, 36.0, cy, Anchor::Middle, ink, full);
        }

        // A spell still cooling down: the dark sweep for what is left (`ring` = 0..1 left).
        if let Some(left) = face.ring.filter(|r| *r > 0.0 && r.is_finite()) {
            let (cx, cy) = (ix as f32 + is as f32 / 2.0, iy as f32 + is as f32 / 2.0);
            for y in iy as usize..(iy + is) as usize {
                for x in ix as usize..(ix + is) as usize {
                    let a = ((x as f32 + 0.5 - cx).atan2(-(y as f32 + 0.5 - cy)) / std::f32::consts::TAU).rem_euclid(1.0);
                    if a > 1.0 - left.min(1.0) {
                        let d = &mut c.px[y * K + x];
                        *d = mix(*d, INK, 0.62);
                    }
                }
            }
        }

        if st.frame == WowFrame::Button {
            // The button's bevel: dark inner edge, a bronze rim.
            if let Some(path) = rounded_ring(0.5, 0.5, 71.5, 71.5, 4.0, 2.4) {
                c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), mix(BRONZE_LO, accent, 0.25), 1.0);
            }
            // The name on a dark band; it fades up into the icon.
            if !face.label.is_empty() {
                for y in 46..K {
                    let a = ((y as f32 - 46.0) / 10.0).min(1.0) * 0.82;
                    for x in 0..K {
                        let d = &mut c.px[y * K + x];
                        *d = mix(*d, INK, a);
                    }
                }
            }
        }

        // The pushed-down look of the button press (all frames): yellow wash at the start.
        if p > 0.0 && st.press == WowPress::Cast {
            let wash = rect_cover(K, K, ix as f32, iy as f32, (ix + is) as f32, (iy + is) as f32);
            c.paint(&wash, rgb(0xfff2a0), (p - 0.6).max(0.0) * 1.2);
        }

        if !face.label.is_empty() {
            let size = if font == FontId::Morpheus { 12.5 } else { 10.0 };
            self.wow_line(&mut c, font, size, &face.label.to_uppercase_if(font != FontId::Morpheus), 36.0, label_y, label_ink, 64.0, t);
        }
        // The bind, top right like the bar's hotkey text (only on game icons; readouts use minor).
        if st.frame == WowFrame::Button
            && icon.is_some()
            && let Some(m) = face.minor.as_deref().filter(|m| !m.is_empty())
        {
            let w = self.fonts.width(FontId::Friz, 9.0, m);
            self.fonts.draw(&mut c, FontId::Friz, 9.0, m, 68.0 - w, 8.0, Anchor::Left, rgb(0xd0d0d0), full);
        } else if !readout
            && let Some(m) = face.minor.as_deref().filter(|m| !m.is_empty())
        {
            // Under the label for the other frames, small and quiet ("NO BIND").
            let col = if st.frame == WowFrame::Parchment { PARCH_DARK } else { EDGE };
            self.wow_line(&mut c, FontId::Friz, 7.5, m, 36.0, if st.frame == WowFrame::Button { 8.0 } else { 68.0 }, col, 64.0, t);
        }

        if let Some(orb) = face.orb
            && let Some(path) = circle(63.0, 9.0, 4.0)
        {
            c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), super::orb_color(orb, t), 1.0);
        }

        if p > 0.0 {
            match st.press {
                WowPress::Cast => {
                    // The cooldown sweep: a dark clock hand wipes round the icon, shrinking.
                    let (cx, cy) = (ix as f32 + is as f32 / 2.0, iy as f32 + is as f32 / 2.0);
                    let left = p.min(0.85) / 0.85;
                    for y in iy as usize..(iy + is) as usize {
                        for x in ix as usize..(ix + is) as usize {
                            let a = ((x as f32 + 0.5 - cx).atan2(-(y as f32 + 0.5 - cy)) / std::f32::consts::TAU).rem_euclid(1.0);
                            let d = &mut c.px[y * K + x];
                            if a > 1.0 - left {
                                *d = mix(*d, INK, 0.62);
                            } else if a > 1.0 - left - 0.012 {
                                *d = mix(*d, HOLY, 0.9);
                            }
                        }
                    }
                }
                WowPress::Holy => {
                    let (cx, cy) = (ix as f32 + is as f32 / 2.0, iy as f32 + is as f32 / 2.0);
                    let spin = t * 0.7;
                    let reach = 14.0 + (1.0 - p) * 60.0;
                    for y in 0..K {
                        for x in 0..K {
                            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                            let r = (dx * dx + dy * dy).sqrt();
                            let ang = dy.atan2(dx) + spin;
                            let ray = ((ang * 6.0).cos().max(0.0)).powf(6.0) + 0.6 * ((ang * 11.0 + 1.3).cos().max(0.0)).powf(10.0);
                            let fall = (1.0 - r / reach).max(0.0);
                            let bloom = (1.0 - r / 30.0).max(0.0).powi(2) * p * p;
                            let v = (ray * fall * p * 1.4 + bloom * 0.9).min(1.0);
                            let d = &mut c.px[y * K + x];
                            for k in 0..3 {
                                d[k] = (d[k] + HOLY[k] * v).min(255.0);
                            }
                        }
                    }
                }
                WowPress::Proc => {
                    // Marching ants of light round the edge, flaring with the press.
                    let mut ring = Cover::new(K, K);
                    let ph = t * 1.6;
                    for y in 0..K {
                        for x in 0..K {
                            let edge = (x.min(K - 1 - x).min(y).min(K - 1 - y)) as f32;
                            if edge < 5.0 {
                                let (dx, dy) = (x as f32 - 35.5, y as f32 - 35.5);
                                let a = dy.atan2(dx) / std::f32::consts::TAU;
                                let ant = 0.5 + 0.5 * ((a + ph) * std::f32::consts::TAU * 8.0).sin();
                                ring.v[y * K + x] = (1.0 - edge / 5.0) * (0.45 + 0.55 * ant);
                            }
                        }
                    }
                    let mut glow = ring.clone();
                    glow.blur(4.0);
                    c.add(&glow, mix([0.0; 3], rgb(0xffc640), p));
                    c.paint(&ring, rgb(0xfff0b0), p);
                }
            }
        }
        c
    }

    /// Centred text shrunk to fit `max` (down to 70 %), then scrolling.
    #[allow(clippy::too_many_arguments)]
    fn wow_line(&self, c: &mut Canvas, font: FontId, size: f32, text: &str, cx: f32, y: f32, col: Rgb, max: f32, t: f32) {
        let w = self.fonts.width(font, size, text);
        let size2 = if w > max { (size * max / w).max(size * 0.7) } else { size };
        let w2 = self.fonts.width(font, size2, text);
        let clip = Clip { x0: (cx - max / 2.0 - 2.0) as i32, y0: 0, x1: (cx + max / 2.0 + 2.0) as i32, y1: c.h as i32 };
        // A soft dark halo keeps light text readable on icons.
        if w2 <= max + 1.0 {
            self.fonts.draw(c, font, size2, text, cx, y + 0.8, Anchor::Middle, INK, clip);
            self.fonts.draw(c, font, size2, text, cx, y, Anchor::Middle, col, clip);
            return;
        }
        let span = w2 + 20.0;
        let x = clip.x0 as f32 + 2.0 - (t.max(0.0) * 20.0) % span;
        for dx in [0.0, span] {
            self.fonts.draw(c, font, size2, text, x + dx, y, Anchor::Left, col, clip);
        }
    }

    /// The dial in the WoW look. `map` (a `maps/` stem) shows under the Minimap ring.
    pub(crate) fn dial_wow(&self, st: &WowStyle, face: &DialFace, t: f32) -> Canvas {
        let mut c = Canvas::new(D, D, rgb(0x0d0b10));
        let (eyebrow, big, sub, line, col, progress, breathe, gauge, dot) = match &face.card {
            Some(DialCard { eyebrow, big, sub, line, ring, progress, breathe, seconds_dot, .. }) => {
                (eyebrow.clone(), big.clone(), sub.clone(), line.clone(), ring.map_or(XP, rgbf), *progress, *breathe, false, *seconds_dot)
            }
            None => (face.name.clone(), face.big.clone(), face.status.clone(), None, XP, Some(face.value), false, true, false),
        };
        let pulse = if breathe { 0.5 + 0.5 * (t * 2.6).sin() } else { 1.0 };
        let lit = progress.map_or(1.0, |p| if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.0 });
        let (a0, span) = if gauge { (135.0f32, 270.0f32) } else { (-90.0, 360.0) };
        let paint_arc = |c: &mut Canvas, r: f32, w: f32, s: f32, e: f32, col: Rgb, a: f32| {
            if e > s
                && let Some(path) = arc(120.5, 120.5, r, w, s, e)
            {
                c.paint(&Cover::from_path(D, D, &path, FillRule::Winding), col, a);
            }
        };
        let disc = |r: f32| circle(120.5, 120.5, r).map(|p| Cover::from_path(D, D, &p, FillRule::Winding));
        let mut text_main = WHITE;
        let mut text_eye = GOLD;
        // A card with a map shows it as the minimap, whatever the ring.
        let map = face.card.as_ref().and_then(|c| c.map.clone());
        let ring = if map.is_some() { WowRing::Minimap } else { st.ring };

        match ring {
            WowRing::Xp => {
                // Dark leather ground, the bar in its frame with twenty notches.
                for y in 0..D {
                    for x in 0..D {
                        let n = noise(x as f32 / 14.0, y as f32 / 14.0, 9) * 0.5 + rnd(x as u32, y as u32 + 5) * 0.2;
                        c.px[y * D + x] = mix(rgb(0x0f0c12), rgb(0x241c2a), n);
                    }
                }
                paint_arc(&mut c, 104.0, 16.0, a0, a0 + span, rgb(0x150f1c), 1.0);
                paint_arc(&mut c, 104.0, 12.0, a0, a0 + span * lit, mix(INK, col, pulse), 1.0);
                // The rested part, ahead of the fill (a quarter of a level at most).
                if col == XP && !gauge {
                    paint_arc(&mut c, 104.0, 12.0, a0 + span * lit, a0 + span * (lit + 0.12).min(1.0), RESTED, 0.55);
                }
                // A sheen along the bar.
                paint_arc(&mut c, 108.0, 2.0, a0, a0 + span * lit, mix(col, WHITE, 0.45), 0.6 * pulse);
                for i in 0..20 {
                    let a = a0 + span * i as f32 / 20.0;
                    paint_arc(&mut c, 104.0, 14.0, a - 0.5, a + 0.5, INK, 0.85);
                }
                paint_arc(&mut c, 113.5, 2.0, a0, a0 + span, BRONZE, 0.9);
                paint_arc(&mut c, 94.5, 1.5, a0, a0 + span, BRONZE_LO, 1.0);
            }
            WowRing::Minimap => {
                // The zone's map, darkened, turning very slowly like the minimap does.
                // The zone in play (from the feed); Elwynn Forest stands in.
                if let Some(img) = picture("maps", map.as_deref().unwrap_or("elwynn_forest"), 420, 280) {
                    let ang = t * 0.01;
                    let (s, co) = ang.sin_cos();
                    for y in 0..D {
                        for x in 0..D {
                            let (dx, dy) = (x as f32 - 120.0, y as f32 - 120.0);
                            let (mx, my) = (210.0 + dx * co - dy * s, 140.0 + dx * s + dy * co);
                            if let Some(p) = img.get_pixel_checked(mx as u32, my as u32) {
                                c.px[y * D + x] = mix([p.0[0] as f32, p.0[1] as f32, p.0[2] as f32], INK, 0.62);
                            }
                        }
                    }
                }
                // Bevelled bronze rim.
                for (r, w, colr) in [(112.0, 9.0, BRONZE_LO), (110.0, 5.0, BRONZE), (108.0, 1.5, BRONZE_HI)] {
                    paint_arc(&mut c, r, w, 0.0, 360.0, colr, 1.0);
                }
                paint_arc(&mut c, 101.0, 4.0, a0, a0 + span * lit, mix(INK, col, pulse), 0.95);
                // North marker.
                if let Some(path) = circle(120.5, 10.0, 7.0) {
                    c.paint(&Cover::from_path(D, D, &path, FillRule::Winding), BRONZE_HI, 1.0);
                }
                self.fonts.draw(&mut c, FontId::Friz, 9.0, "N", 120.5, 10.5, Anchor::Middle, BROWN, Clip { x0: 0, y0: 0, x1: D as i32, y1: D as i32 });
            }
            WowRing::Rune => {
                for y in 0..D {
                    for x in 0..D {
                        let n = noise(x as f32 / 10.0, y as f32 / 10.0, 21) * 0.6 + rnd(x as u32 + 3, y as u32) * 0.25;
                        c.px[y * D + x] = mix(rgb(0x1a1d24), rgb(0x3a3f48), n);
                    }
                }
                paint_arc(&mut c, 104.0, 18.0, 0.0, 360.0, rgb(0x262a33), 1.0);
                let n = 16;
                for i in 0..n {
                    let frac = (i as f32 + 0.5) / n as f32;
                    let on = frac <= lit;
                    let a = (a0 + span * frac).to_radians();
                    let (x, y) = (120.5 + 104.0 * a.cos(), 120.5 + 104.0 * a.sin());
                    if let Some(path) = diamond(x, y, 5.5) {
                        let mut cv = Cover::from_path(D, D, &path, FillRule::Winding);
                        let colr = if on { mix(ARCANE, WHITE, 0.3) } else { rgb(0x4a505c) };
                        c.paint(&cv, colr, 1.0);
                        if on {
                            cv.blur(5.0);
                            c.add(&cv, mix([0.0; 3], ARCANE, 0.9 * pulse));
                        }
                    }
                }
                text_eye = mix(ARCANE, WHITE, 0.2);
            }
        }
        if dot {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |x| x.as_secs_f32());
            let a = (secs % 60.0) / 60.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            if let Some(path) = circle(120.5 + 104.0 * a.cos(), 120.5 + 104.0 * a.sin(), 4.0) {
                c.paint(&Cover::from_path(D, D, &path, FillRule::Winding), HOLY, 1.0);
            }
        }
        if ring == WowRing::Minimap {
            // Text needs a quiet middle over the map.
            if let Some(mut cv) = disc(78.0) {
                cv.blur(10.0);
                c.paint(&cv, INK, 0.55);
            }
            text_main = WHITE;
        }

        let all = Clip { x0: 0, y0: 0, x1: D as i32, y1: D as i32 };
        let draw = |c: &mut Canvas, font: FontId, size: f32, s: &str, y: f32, col: Rgb, max: f32| {
            let w = self.fonts.width(font, size, s);
            let size = if w > max { size * max / w } else { size };
            self.fonts.draw(c, font, size, s, 120.5, y + 1.2, Anchor::Middle, INK, all);
            self.fonts.draw(c, font, size, s, 120.5, y, Anchor::Middle, col, all);
        };
        draw(&mut c, FontId::Morpheus, 19.0, &title_case(&eyebrow), 70.0, text_eye, 130.0);
        draw(&mut c, FontId::Friz, 46.0, &big, 114.0, text_main, 150.0);
        if let Some(s) = sub.as_deref() {
            draw(&mut c, FontId::Friz, 13.0, s, 152.0, mix(WHITE, GOLD, 0.25), 140.0);
        }
        if let Some(l) = line.as_deref() {
            draw(&mut c, FontId::Friz, 10.5, l, 174.0, rgb(0xb8b0a0), 120.0);
        }
        c.mask_black(self.dial_disc());
        c
    }
}

/// Zone names and eyebrows read as titles in Morpheus ("ELWYNN FOREST" → "Elwynn Forest").
fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                Some(f) => f.to_uppercase().chain(ch.flat_map(|c| c.to_lowercase())).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn diamond(x: f32, y: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x, y - r);
    pb.line_to(x + r * 0.7, y);
    pb.line_to(x, y + r);
    pb.line_to(x - r * 0.7, y);
    pb.close();
    pb.finish()
}

/// A rounded rectangle's outline `w` thick (even-odd).
fn rounded_ring(x0: f32, y0: f32, x1: f32, y1: f32, r: f32, w: f32) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    rr(&mut pb, x0, y0, x1, y1, r);
    rr(&mut pb, x0 + w, y0 + w, x1 - w, y1 - w, (r - w).max(0.5));
    pb.finish()
}

fn rr(pb: &mut tiny_skia::PathBuilder, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) {
    let k = 0.5523 * r;
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0);
    pb.close();
}

trait UpperIf {
    fn to_uppercase_if(&self, yes: bool) -> String;
}

impl UpperIf for String {
    fn to_uppercase_if(&self, yes: bool) -> String {
        if yes { self.to_uppercase() } else { title_case(self) }
    }
}

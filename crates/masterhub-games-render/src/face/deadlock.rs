//! The game look for Deadlock, built from the game's own material: colours from its style sheets
//! (`offWhite`, `soulColor`, the weapon / vitality / spirit sets), its display face (Retail
//! Demo, read from the install) and its vector icons (art/game/dl-*). Keys are drawn like the
//! shop's item cards. Parts have variants ([`DeadlockStyle`]) for a proposals board.

use tiny_skia::{FillRule, PathBuilder, Rect};

use super::{Painter, rgbf};
use crate::canvas::{Canvas, Clip, Cover, Rgb, annulus, arc, circle, mix, rgb};
use crate::text::{Anchor, FontId};
use crate::{DialCard, DialFace, KeyFace};

const K: usize = 72;
pub(crate) const BG: Rgb = rgb(0x0e0e0e);
const TILE: Rgb = rgb(0x1a1a1a);
const GRAY: Rgb = rgb(0x2e2c27);
pub(crate) const OFF_WHITE: Rgb = rgb(0xffefd7);
const INK: Rgb = rgb(0x1a1a1a);
pub(crate) const SOUL: Rgb = rgb(0x70f8c1);
const GREEN_LIGHT: Rgb = rgb(0xa8ffc9);
const DIM: Rgb = rgb(0xadadad);
const GOLD: Rgb = rgb(0xffed79);

/// The key's build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeadlockFrame {
    /// A shop item card: the icon on a colour field, the name on an off-white plate below.
    #[default]
    Card,
    /// A dark tile like the main menu: light icon, name below, a colour bar under it.
    Nav,
    /// The icon on a round colour coin like the top bar's hero coins, name below.
    Coin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeadlockPress {
    /// A soul-green ring bursts out from the icon and the key glows green.
    #[default]
    Soul,
    /// The whole card flashes off-white, like buying an item.
    Buy,
    /// Diagonal stripes in the key's colour sweep across.
    Stripes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeadlockRing {
    /// One broad arc, like a soul counter filling.
    #[default]
    Souls,
    /// Art-deco ticks round the edge with a thin gold line inside.
    Deco,
}

/// One assembled Deadlock look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeadlockStyle {
    pub frame: DeadlockFrame,
    pub press: DeadlockPress,
    pub ring: DeadlockRing,
}

fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Film grain, the same on every frame.
pub(crate) fn grain(c: &mut Canvas, amp: f32) {
    let w = c.w;
    for (i, p) in c.px.iter_mut().enumerate() {
        let n = (rnd((i % w) as u32 * 7 + 3, (i / w) as u32 * 13 + 1) - 0.5) * 2.0 * amp;
        for k in p.iter_mut() {
            *k = (*k + n).clamp(0.0, 255.0);
        }
    }
}

fn rounded(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_ltrb(x0 + r, y0, x1 - r, y1)?);
    pb.push_rect(Rect::from_ltrb(x0, y0 + r, x1, y1 - r)?);
    for (x, y) in [(x0 + r, y0 + r), (x1 - r, y0 + r), (x0 + r, y1 - r), (x1 - r, y1 - r)] {
        pb.push_circle(x, y, r);
    }
    pb.finish()
}

fn cover_of(w: usize, h: usize, path: Option<tiny_skia::Path>) -> Cover {
    path.map(|p| Cover::from_path(w, h, &p, FillRule::Winding)).unwrap_or_else(|| Cover::new(w, h))
}

impl Painter {
    /// A key in the Deadlock look. `ground` is the card's colour field, `accent` its light tone.
    pub(crate) fn key_deadlock(&self, st: &DeadlockStyle, face: &KeyFace, t: f32, press: f32) -> Canvas {
        let p = if press.is_finite() { press.clamp(0.0, 1.0) } else { 0.0 };
        let field = face.ground.map_or(GRAY, rgbf);
        let light = face.accent.map_or(OFF_WHITE, rgbf);
        let mut c = Canvas::new(K, K, BG);
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };
        let card = cover_of(K, K, rounded(2.0, 2.0, 70.0, 70.0, 4.0));

        let (icon_y, icon_size, mut icon_ink, label_y, mut label_ink);
        match st.frame {
            DeadlockFrame::Card => {
                // The field, darker towards the top like the shop's cards, then the name plate.
                for y in 0..K {
                    let col = mix(mix(field, [0.0; 3], 0.28), field, (y as f32 / 50.0).min(1.0));
                    for x in 0..K {
                        let a = card.v[y * K + x];
                        if a > 0.0 {
                            let col = if y >= 51 { OFF_WHITE } else { col };
                            c.px[y * K + x] = mix(c.px[y * K + x], col, a);
                        }
                    }
                }
                (icon_y, icon_size, icon_ink, label_y, label_ink) = (27.0, 38.0, light, 61.0, INK);
            }
            DeadlockFrame::Nav => {
                c.paint(&card, TILE, 1.0);
                let bar = cover_of(K, K, rounded(16.0, 65.0, 56.0, 68.0, 1.5));
                c.paint(&bar, mix(field, light, 0.55), 1.0);
                (icon_y, icon_size, icon_ink, label_y, label_ink) = (27.0, 36.0, light, 55.0, OFF_WHITE);
            }
            DeadlockFrame::Coin => {
                c.paint(&card, TILE, 1.0);
                if let Some(path) = circle(36.0, 28.0, 22.0) {
                    c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), field, 1.0);
                }
                if let Some(path) = annulus(36.0, 28.0, 21.0, 23.0) {
                    c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), mix(field, light, 0.7), 1.0);
                }
                (icon_y, icon_size, icon_ink, label_y, label_ink) = (28.0, 27.0, light, 60.0, OFF_WHITE);
            }
        }
        grain(&mut c, 3.0);

        // Press colours.
        match st.press {
            DeadlockPress::Soul => {
                icon_ink = mix(icon_ink, SOUL, p);
                if st.frame != DeadlockFrame::Card {
                    label_ink = mix(label_ink, SOUL, p);
                }
            }
            DeadlockPress::Buy if p > 0.0 => {
                c.paint(&card, OFF_WHITE, p * 0.92);
                icon_ink = mix(icon_ink, INK, p);
                label_ink = mix(label_ink, INK, p);
            }
            _ => {}
        }

        // Label: shrinks to fit before it has to scroll.
        if !face.label.is_empty() {
            let size = {
                let w = self.fonts.width(FontId::Retail, 11.0, &face.label);
                if w > 60.0 { (11.0 * 60.0 / w).max(7.5) } else { 11.0 }
            };
            self.deadlock_line(&mut c, size, &face.label, label_y, label_ink, t);
        }
        if let Some(orb) = face.orb
            && let Some(path) = circle(63.0, 9.0, 4.0)
        {
            c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), super::orb_color(orb, t), 1.0);
        }

        // Icon: the game's own (`logo` lists pictures, best first), else the glyph.
        let pop = 1.0 + 0.2 * (p * std::f32::consts::PI).sin();
        let art = face.logo.as_deref().and_then(|l| l.split('|').find(|n| crate::art::exists(n)));
        if art.is_none() && face.icon.is_none() {
            // A readout: the value big where the icon would be.
            if let Some(v) = face.minor.as_deref().filter(|m| !m.is_empty()) {
                self.deadlock_mixed(&mut c, 27.0, v, 36.0, icon_y, icon_ink, 60.0, full);
            }
        } else if let Some(cover) = art.and_then(|n| crate::art::cover(n, K, K, icon_size * pop, 36.0, icon_y)) {
            c.paint(&cover, icon_ink, 1.0);
        } else if let Some(icon) = face.icon.filter(|&i| self.fonts.has_glyph(FontId::Icons, i)) {
            let s = icon.encode_utf8(&mut [0; 4]).to_owned();
            self.fonts.draw(&mut c, FontId::Icons, icon_size * 0.72 * pop, &s, 36.0, icon_y, Anchor::Middle, icon_ink, full);
        }

        if p > 0.0 {
            match st.press {
                DeadlockPress::Soul => {
                    let r = 8.0 + (1.0 - p) * 44.0;
                    if let Some(path) = annulus(36.0, icon_y, (r - 1.0 - 4.0 * p).max(0.5), r) {
                        let mut ring = Cover::from_path(K, K, &path, FillRule::EvenOdd);
                        for (v, m) in ring.v.iter_mut().zip(&card.v) {
                            *v *= m;
                        }
                        c.paint(&ring, SOUL, p);
                        ring.blur(3.0);
                        c.add(&ring, mix([0.0; 3], SOUL, p * 0.7));
                    }
                }
                DeadlockPress::Stripes => {
                    let x0 = (1.0 - p) * 130.0 - 40.0;
                    for y in 2..70usize {
                        for x in 2..70usize {
                            let d = x as f32 + y as f32 * 0.6 - x0;
                            if (0.0..34.0).contains(&d) && ((x + y) / 4) % 2 == 0 {
                                let a = card.v[y * K + x] * 0.8 * (1.0 - (d / 34.0 - 0.5).abs() * 2.0).max(0.0).sqrt();
                                c.px[y * K + x] = mix(c.px[y * K + x], light, a);
                            }
                        }
                    }
                }
                DeadlockPress::Buy => {}
            }
        }
        c
    }

    /// Centred line in the game's display face; too wide even when shrunk, it scrolls.
    fn deadlock_line(&self, c: &mut Canvas, size: f32, text: &str, y: f32, col: Rgb, t: f32) {
        let w = self.fonts.width(FontId::Retail, size, text);
        let clip = Clip { x0: 4, y0: 0, x1: 68, y1: K as i32 };
        if w <= 62.0 {
            self.fonts.draw(c, FontId::Retail, size, text, 36.0, y, Anchor::Middle, col, clip);
            return;
        }
        let span = w + 22.0;
        let x = 6.0 - (t.max(0.0) * 22.0) % span;
        for dx in [0.0, span] {
            self.fonts.draw(c, FontId::Retail, size, text, x + dx, y, Anchor::Left, col, clip);
        }
    }

    /// A centred dial line, shrunk to fit `max`. Retail Demo is a demo cut without punctuation
    /// (":" and "·" come out as a placeholder), so anything but letters, digits and spaces is
    /// drawn in Chakra Petch at the same size.
    fn deadlock_dial_line(&self, c: &mut Canvas, size: f32, text: &str, y: f32, col: Rgb, max: f32) {
        let all = Clip { x0: 0, y0: 0, x1: crate::DIAL_SIZE as i32, y1: crate::DIAL_SIZE as i32 };
        self.deadlock_mixed(c, size, text, 120.0, y, col, max, all);
    }

    /// Text centred on `cx`, shrunk to fit `max`: letters, digits and spaces in Retail Demo,
    /// everything else in Chakra Petch.
    #[allow(clippy::too_many_arguments)]
    fn deadlock_mixed(&self, c: &mut Canvas, size: f32, text: &str, cx: f32, y: f32, col: Rgb, max: f32, clip: Clip) {
        let own = |ch: char| ch.is_ascii_alphanumeric() || ch == ' ';
        let mut runs: Vec<(FontId, String)> = Vec::new();
        for ch in text.chars() {
            let font = if own(ch) { FontId::Retail } else { FontId::Chakra };
            match runs.last_mut() {
                Some((f, s)) if *f == font => s.push(ch),
                _ => runs.push((font, ch.to_string())),
            }
        }
        let width = |size: f32| runs.iter().map(|(f, s)| self.fonts.width(*f, size, s)).sum::<f32>();
        let w = width(size);
        let size = if w > max { size * max / w } else { size };
        let mut x = cx - width(size) / 2.0;
        for (font, s) in &runs {
            self.fonts.draw(c, *font, size, s, x.round(), y, Anchor::Left, col, clip);
            x += self.fonts.width(*font, size, s);
        }
    }

    /// The dial in the Deadlock look.
    pub(crate) fn dial_deadlock(&self, st: &DeadlockStyle, face: &DialFace, t: f32) -> Canvas {
        let d = crate::DIAL_SIZE as usize;
        let mut c = Canvas::new(d, d, rgb(0x121212));
        let (eyebrow, big, sub, line, col, progress, breathe, gauge, dot) = match &face.card {
            Some(DialCard { eyebrow, big, sub, line, ring, progress, breathe, seconds_dot, .. }) => {
                (eyebrow.clone(), big.clone(), sub.clone(), line.clone(), ring.map_or(SOUL, rgbf), *progress, *breathe, false, *seconds_dot)
            }
            None => (face.name.clone(), face.big.clone(), face.status.clone(), None, SOUL, Some(face.value), false, true, false),
        };
        let col = if breathe { mix(mix(BG, col, 0.3), col, 0.5 + 0.5 * (t * 2.6).sin()) } else { col };
        let lit = progress.map_or(1.0, |p| if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.0 });
        let (a0, span) = if gauge { (135.0f32, 270.0f32) } else { (-90.0, 360.0) };
        let paint_arc = |c: &mut Canvas, r: f32, w: f32, s: f32, e: f32, col: Rgb| {
            if e > s
                && let Some(path) = arc(120.5, 120.5, r, w, s, e)
            {
                c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), col, 1.0);
            }
        };

        // The game's emblem as a faint watermark.
        if let Some(em) = crate::art::cover("dl-logo", d, d, 176.0, 120.0, 120.0) {
            c.paint(&em, OFF_WHITE, 0.05);
        }
        grain(&mut c, 3.0);
        match st.ring {
            DeadlockRing::Souls => {
                paint_arc(&mut c, 104.0, 11.0, a0, a0 + span, GRAY);
                paint_arc(&mut c, 104.0, 11.0, a0, a0 + span * lit, col);
                paint_arc(&mut c, 113.0, 1.0, a0, a0 + span, mix(BG, OFF_WHITE, 0.35));
            }
            DeadlockRing::Deco => {
                let n = 60;
                for i in 0..n {
                    let frac = (i as f32 + 0.5) / n as f32;
                    let a = a0 + span * frac;
                    let long = i % 5 == 0;
                    let (r, w) = if long { (104.0, 18.0) } else { (107.0, 10.0) };
                    paint_arc(&mut c, r, w, a - 1.1, a + 1.1, if frac <= lit { col } else { GRAY });
                }
                paint_arc(&mut c, 92.0, 1.2, a0, a0 + span, mix(BG, GOLD, 0.6));
            }
        }
        if dot {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |x| x.as_secs_f32());
            let a = (secs % 60.0) / 60.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            if let Some(path) = circle(120.5 + 104.0 * a.cos(), 120.5 + 104.0 * a.sin(), 4.5) {
                c.paint(&Cover::from_path(d, d, &path, FillRule::Winding), OFF_WHITE, 1.0);
            }
        }

        self.deadlock_dial_line(&mut c, 16.0, &eyebrow, 68.0, mix(col, GREEN_LIGHT, 0.4), 120.0);
        self.deadlock_dial_line(&mut c, 64.0, &big, 116.0, OFF_WHITE, 150.0);
        if let Some(s) = sub.as_deref() {
            self.deadlock_dial_line(&mut c, 15.0, s, 158.0, mix(OFF_WHITE, DIM, 0.4), 140.0);
        }
        if let Some(l) = line.as_deref() {
            self.deadlock_dial_line(&mut c, 12.0, l, 180.0, DIM, 130.0);
        }
        let _ = t;
        c.mask_black(self.dial_disc());
        c
    }
}

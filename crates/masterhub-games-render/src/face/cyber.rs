//! The game look for Cyberpunk 2077, after the feel of the game's HUD (our own drawing; nothing
//! taken from the game). Every part has variants ([`CyberStyle`]) so a look can be assembled from
//! a proposals board.

use tiny_skia::{FillRule, PathBuilder};

use super::{Painter, Strip, rgbf};
use crate::canvas::{Canvas, Clip, Cover, Rgb, arc, mix};
use crate::text::{Anchor, FontId};
use crate::{DialCard, DialFace, KeyFace, Orb};

const K: usize = 72;
const BG: Rgb = [7.0, 7.0, 11.0];
const SCAN: Rgb = [13.0, 13.0, 19.0];
const SALMON: Rgb = [255.0, 96.0, 84.0];
const CYAN: Rgb = [2.0, 215.0, 242.0];
const WHITE: Rgb = [232.0, 236.0, 240.0];
const DIM: Rgb = [110.0, 118.0, 130.0];

/// The key's outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Frame {
    /// A panel with the top-right corner cut, thin neon outline, a tab on the cut.
    #[default]
    Cut,
    /// Only four corner brackets; the body stays dark.
    Brackets,
    /// A solid neon header bar carrying the label in dark ink.
    Header,
    /// A slanted label tab at the top left, outline running from it round the right side.
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconFx {
    /// Neon glow behind the icon.
    #[default]
    Glow,
    /// Plain, no glow.
    Flat,
    /// Inside a small cut-corner box.
    Boxed,
    /// Larger, no glow.
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressFx {
    /// The panel flashes neon and inverts, then fades.
    #[default]
    Flash,
    /// RGB split and sliced rows that settle.
    Glitch,
    /// A bright band sweeps down the key.
    Scan,
    /// The outline thickens and the whole key pulses brighter.
    Pulse,
    /// Flash and Glitch together (the picked look).
    FlashGlitch,
}

impl PressFx {
    fn flashes(self) -> bool {
        matches!(self, PressFx::Flash | PressFx::FlashGlitch)
    }

    fn glitches(self) -> bool {
        matches!(self, PressFx::Glitch | PressFx::FlashGlitch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ring {
    /// Separate segments round the dial.
    #[default]
    Segmented,
    /// One thin solid arc with tick marks outside.
    Thin,
    /// Two thin rings, progress on the outer with notches every 30°.
    Double,
    /// Four bracket arcs at the diagonals, progress as an inner arc.
    Brackets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Numbers {
    /// Tall condensed (Bebas).
    #[default]
    Tall,
    /// Terminal mono.
    Mono,
    /// Squared tech (Chakra Petch).
    Tech,
}

/// One assembled game look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CyberStyle {
    pub frame: Frame,
    pub icon: IconFx,
    pub press: PressFx,
    pub ring: Ring,
    pub numbers: Numbers,
    pub scanlines: bool,
}

impl Default for CyberStyle {
    fn default() -> Self {
        Self {
            frame: Frame::Cut,
            icon: IconFx::Glow,
            press: PressFx::Flash,
            ring: Ring::Segmented,
            numbers: Numbers::Tall,
            scanlines: true,
        }
    }
}

const LABEL: Strip = Strip { x0: 6.0, width: 52.0, pad: 1.0, fit: 52.0, centre: 0.0 };
const MINOR: Strip = Strip { x0: 6.0, width: 62.0, pad: 1.0, fit: 62.0, centre: 0.0 };

fn poly(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (x, y) = points[0];
    pb.move_to(x, y);
    for &(x, y) in &points[1..] {
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

fn fill(c: &mut Canvas, pts: &[(f32, f32)], col: Rgb, a: f32) {
    if let Some(p) = poly(pts) {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::Winding), col, a);
    }
}

fn rect(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, col: Rgb, a: f32) {
    fill(c, &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], col, a);
}

/// A closed outline `w` wide along `outer` (inner = `inner`).
fn outline(c: &mut Canvas, outer: &[(f32, f32)], inner: &[(f32, f32)], col: Rgb, a: f32) {
    let mut pb = PathBuilder::new();
    for pts in [outer, inner] {
        pb.move_to(pts[0].0, pts[0].1);
        for &(x, y) in &pts[1..] {
            pb.line_to(x, y);
        }
        pb.close();
    }
    if let Some(p) = pb.finish() {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::EvenOdd), col, a);
    }
}

/// The cut-corner panel, inset by `i`.
fn panel(i: f32) -> Vec<(f32, f32)> {
    vec![
        (2.0 + i, 2.0 + i),
        (58.0 - i * 0.4, 2.0 + i),
        (70.0 - i, 14.0 + i * 0.4),
        (70.0 - i, 70.0 - i),
        (10.0 + i * 0.4, 70.0 - i),
        (2.0 + i, 62.0 - i * 0.4),
    ]
}

fn scanlines(c: &mut Canvas) {
    for y in (0..c.h).step_by(3) {
        for x in 0..c.w {
            c.px[y * c.w + x] = SCAN;
        }
    }
}

fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

impl Painter {
    /// A key in the game look. `ground` tints the panel, `accent` is the neon colour.
    pub(crate) fn key_cyber(&self, st: &CyberStyle, face: &KeyFace, t: f32, press: f32) -> Canvas {
        let p = if press.is_finite() { press.clamp(0.0, 1.0) } else { 0.0 };
        let accent = face.accent.map_or(CYAN, rgbf);
        let tint = face.ground.map_or(BG, rgbf);
        let mut c = Canvas::new(K, K, BG);
        if st.scanlines {
            scanlines(&mut c);
        }
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };
        // Flash inverts the face while strong.
        let inverted = st.press.flashes() && p > 0.45;
        let pulse = if st.press == PressFx::Pulse { p } else { 0.0 };

        // Frame.
        let mut label_ink = accent;
        let mut label_y = 13.0;
        match st.frame {
            Frame::Cut => {
                fill(&mut c, &panel(0.0), mix(BG, tint, 0.7), 0.8);
                outline(&mut c, &panel(0.0), &panel(1.5 + pulse * 1.5), accent, 0.9);
                fill(&mut c, &[(52.0, 4.5), (58.0, 4.5), (67.5, 14.0), (67.5, 20.0)], accent, 0.9);
                rect(&mut c, 6.0, 65.0, 10.0, 2.0, accent, 0.9);
            }
            Frame::Brackets => {
                fill(&mut c, &[(4.0, 4.0), (68.0, 4.0), (68.0, 68.0), (4.0, 68.0)], mix(BG, tint, 0.5), 0.6);
                let (l, w) = (14.0, 2.0 + pulse * 1.5);
                for (x, y, sx, sy) in [(2.0, 2.0, 1.0, 1.0), (70.0, 2.0, -1.0, 1.0), (2.0, 70.0, 1.0, -1.0), (70.0, 70.0, -1.0, -1.0)] {
                    let xx = if sx > 0.0 { x } else { x - l };
                    let yy = if sy > 0.0 { y } else { y - w };
                    rect(&mut c, xx, yy, l, w, accent, 1.0);
                    let xx = if sx > 0.0 { x } else { x - w };
                    let yy = if sy > 0.0 { y } else { y - l };
                    rect(&mut c, xx, yy, w, l, accent, 1.0);
                }
                label_y = 14.0;
            }
            Frame::Header => {
                fill(&mut c, &[(2.0, 20.0), (70.0, 20.0), (70.0, 70.0), (2.0, 70.0)], mix(BG, tint, 0.6), 0.7);
                fill(&mut c, &[(2.0, 2.0), (62.0, 2.0), (70.0, 10.0), (70.0, 20.0), (2.0, 20.0)], accent, 0.92);
                outline(
                    &mut c,
                    &[(2.0, 20.0), (70.0, 20.0), (70.0, 70.0), (2.0, 70.0)],
                    &[(3.0, 20.0), (69.0 - pulse, 20.0), (69.0 - pulse, 69.0 - pulse), (3.0 + pulse, 69.0 - pulse)],
                    accent,
                    0.7,
                );
                label_ink = BG;
                label_y = 11.5;
            }
            Frame::Tab => {
                let lw = self.fonts.width(FontId::Chakra, 9.5, &face.label).min(52.0);
                let tab_r = 6.0 + lw + 8.0;
                fill(&mut c, &[(2.0, 2.0), (tab_r, 2.0), (tab_r + 8.0, 10.0), (tab_r + 8.0, 19.0), (2.0, 19.0)], accent, 0.92);
                fill(&mut c, &[(2.0, 19.0), (70.0, 19.0), (70.0, 70.0), (2.0, 70.0)], mix(BG, tint, 0.6), 0.6);
                let w = 1.5 + pulse * 1.5;
                rect(&mut c, tab_r + 8.0, 9.0 + 0.0, 70.0 - tab_r - 8.0, w, accent, 0.9);
                rect(&mut c, 70.0 - w, 9.0, w, 61.0, accent, 0.9);
                rect(&mut c, 30.0, 70.0 - w, 40.0, w, accent, 0.9);
                label_ink = BG;
                label_y = 11.0;
            }
        }
        if inverted {
            fill(&mut c, &[(0.0, 0.0), (72.0, 0.0), (72.0, 72.0), (0.0, 72.0)], accent, p * 0.85);
        } else if st.press.flashes() && p > 0.0 {
            fill(&mut c, &[(0.0, 0.0), (72.0, 0.0), (72.0, 72.0), (0.0, 72.0)], accent, p * 0.6);
        }
        if pulse > 0.0 {
            fill(&mut c, &[(0.0, 0.0), (72.0, 0.0), (72.0, 72.0), (0.0, 72.0)], accent, pulse * 0.25);
        }
        let ink = if inverted { BG } else { label_ink };
        let text = if inverted { BG } else { WHITE };

        if !face.label.is_empty() {
            self.cyber_line(&mut c, FontId::Chakra, 9.5, &face.label, &LABEL, label_y, ink, t);
        }
        if let Some(orb) = face.orb {
            let col = if matches!(orb, Orb::Ready) { CYAN } else { super::orb_color(orb, t) };
            rect(&mut c, 59.0, 25.0, 6.0, 6.0, col, 1.0);
        }

        // Icon.
        if let Some(icon) = face.icon.filter(|&i| self.fonts.has_glyph(FontId::Icons, i)) {
            let s = icon.encode_utf8(&mut [0; 4]).to_owned();
            let (x, y) = (36.0, if st.frame == Frame::Header || st.frame == Frame::Tab { 42.0 } else { 39.0 });
            // The icon pops on a press (grows, then settles) and shakes while the glitch runs.
            let pop = 1.0 + 0.3 * (p * std::f32::consts::PI).sin();
            let size = pop
                * match st.icon {
                    IconFx::Big => 27.0,
                    IconFx::Boxed => 17.0,
                    _ => 24.0,
                };
            let shake = if p > 0.35 && st.press.glitches() { (rnd(77, (t * 30.0) as u32) - 0.5) * 6.0 * p } else { 0.0 };
            let x = x + shake.round();
            if st.icon == IconFx::Boxed {
                let b = |i: f32| vec![(23.0 + i, y - 13.0 + i), (45.0 - i, y - 13.0 + i), (49.0 - i, y - 9.0 + i), (49.0 - i, y + 13.0 - i), (23.0 + i, y + 13.0 - i)];
                fill(&mut c, &b(0.0), mix(BG, accent, 0.15), 1.0);
                outline(&mut c, &b(0.0), &b(1.2), if inverted { BG } else { accent }, 0.9);
            }
            if st.icon == IconFx::Glow && !inverted {
                let mut glow = self.fonts.cover(K, K, FontId::Icons, size, &s, x, y);
                glow.blur(3.0);
                c.add(&glow, mix([0.0; 3], accent, 0.3));
            }
            let col = if inverted { BG } else { mix(accent, WHITE, 0.25) };
            self.fonts.draw(&mut c, FontId::Icons, size, &s, x, y, Anchor::Middle, col, full);
        }
        if let Some(m) = face.minor.as_deref().filter(|m| !m.is_empty()) {
            let col = if inverted { BG } else { mix(DIM, text, 0.35) };
            self.cyber_line(&mut c, FontId::Mono, 9.0, m, &MINOR, 60.0, col, t);
        }

        // Effects over the whole key.
        match st.press {
            fx if fx.glitches() && p > 0.0 => glitch(&mut c, p, accent, t),
            PressFx::Scan if p > 0.0 => {
                let y = (1.0 - p) * 84.0 - 8.0;
                rect(&mut c, 0.0, y - 6.0, 72.0, 12.0, accent, 0.35);
                rect(&mut c, 0.0, y, 72.0, 2.0, WHITE, 0.8);
            }
            _ => {}
        }
        c
    }

    /// Left-aligned line that scrolls when it does not fit its strip.
    #[allow(clippy::too_many_arguments)]
    fn cyber_line(&self, c: &mut Canvas, font: FontId, size: f32, text: &str, strip: &Strip, y: f32, col: Rgb, t: f32) {
        let w = self.fonts.width(font, size, text);
        let clip = Clip { x0: strip.x0 as i32, y0: 0, x1: (strip.x0 + strip.width) as i32, y1: K as i32 };
        if w <= strip.fit {
            self.fonts.draw(c, font, size, text, strip.x0, y, Anchor::Left, col, clip);
            return;
        }
        let span = w + 20.0;
        let x = strip.x0 + strip.pad - (t.max(0.0) * 22.0) % span;
        for dx in [0.0, span] {
            self.fonts.draw(c, font, size, text, x + dx, y, Anchor::Left, col, clip);
        }
    }

    /// The dial in the game look.
    pub(crate) fn dial_cyber(&self, st: &CyberStyle, face: &DialFace, t: f32) -> Canvas {
        let d = crate::DIAL_SIZE as usize;
        let mut c = Canvas::new(d, d, BG);
        if st.scanlines {
            scanlines(&mut c);
        }
        let all = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        let f = &self.fonts;

        let (eyebrow, big, sub, line, col, progress, breathe, gauge) = match &face.card {
            Some(DialCard { eyebrow, big, sub, line, ring, progress, breathe, .. }) => {
                (eyebrow.clone(), big.clone(), sub.clone(), line.clone(), ring.map_or(CYAN, rgbf), *progress, *breathe, false)
            }
            None => (face.name.clone(), face.big.clone(), face.status.clone(), None, rgbf([0xFC, 0xEE, 0x0A]), Some(face.value), false, true),
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

        match st.ring {
            Ring::Segmented => {
                let n = if gauge { 45 } else { 60 };
                let step = span / n as f32;
                for i in 0..n {
                    let s = a0 + step * i as f32;
                    let on = (i as f32 + 0.5) / n as f32 <= lit;
                    paint_arc(&mut c, 103.0, 9.0, s + 0.6, s + step - 1.4, if on { col } else { mix(BG, col, 0.16) });
                }
                paint_arc(&mut c, 90.0, 1.0, 0.0, 360.0, mix(BG, col, 0.45));
            }
            Ring::Thin => {
                paint_arc(&mut c, 100.0, 3.0, a0, a0 + span, mix(BG, col, 0.2));
                paint_arc(&mut c, 100.0, 3.0, a0, a0 + span * lit, col);
                for i in 0..24 {
                    let a = (i as f32 * 15.0 - 90.0).to_radians();
                    let (r0, r1) = (106.0, if i % 6 == 0 { 116.0 } else { 111.0 });
                    let (ca, sa) = (a.cos(), a.sin());
                    let w = if i % 6 == 0 { 2.0 } else { 1.2 };
                    let (nx, ny) = (-sa * w / 2.0, ca * w / 2.0);
                    fill(
                        &mut c,
                        &[
                            (120.5 + ca * r0 + nx, 120.5 + sa * r0 + ny),
                            (120.5 + ca * r1 + nx, 120.5 + sa * r1 + ny),
                            (120.5 + ca * r1 - nx, 120.5 + sa * r1 - ny),
                            (120.5 + ca * r0 - nx, 120.5 + sa * r0 - ny),
                        ],
                        if i % 6 == 0 { SALMON } else { mix(BG, col, 0.5) },
                        1.0,
                    );
                }
            }
            Ring::Double => {
                paint_arc(&mut c, 108.0, 2.0, a0, a0 + span, mix(BG, col, 0.25));
                paint_arc(&mut c, 108.0, 5.0, a0, a0 + span * lit, col);
                paint_arc(&mut c, 96.0, 1.5, 0.0, 360.0, mix(BG, col, 0.5));
                for i in 0..12 {
                    let s = i as f32 * 30.0 - 90.0;
                    paint_arc(&mut c, 102.0, 6.0, s - 0.8, s + 0.8, SALMON);
                }
            }
            Ring::Brackets => {
                for q in 0..4 {
                    let m = q as f32 * 90.0 - 45.0;
                    paint_arc(&mut c, 108.0, 4.0, m - 22.0, m + 22.0, mix(BG, col, 0.85));
                }
                paint_arc(&mut c, 96.0, 2.0, a0, a0 + span, mix(BG, col, 0.18));
                paint_arc(&mut c, 96.0, 4.0, a0, a0 + span * lit, col);
            }
        }
        for x in [22.0, 206.0] {
            rect(&mut c, x, 119.0, 12.0, 2.0, SALMON, 1.0);
        }

        f.draw(&mut c, FontId::Mono, 13.0, &eyebrow, 120.0, 64.0, Anchor::Middle, SALMON, all);
        let short = big.chars().count() <= 5;
        let (font, size) = match st.numbers {
            Numbers::Tall => (FontId::Bebas, if short { 76.0 } else { 46.0 }),
            Numbers::Mono => (FontId::Mono, if short { 54.0 } else { 34.0 }),
            Numbers::Tech => (FontId::Chakra, if short { 52.0 } else { 32.0 }),
        };
        self.dial_line(&mut c, font, size, &big, 112.0, col, t, 170.0);
        if let Some(s) = sub.as_deref() {
            self.dial_line(&mut c, FontId::Chakra, 14.0, s, 158.0, WHITE, t, 160.0);
        }
        if let Some(l) = line.as_deref() {
            self.dial_line(&mut c, FontId::Mono, 12.0, l, 182.0, DIM, t, 136.0);
        }
        c.mask_black(self.dial_disc());
        c
    }
}

/// Glitch press: rows slide sideways, the red channel splits off, a white bar or two.
fn glitch(c: &mut Canvas, p: f32, accent: Rgb, t: f32) {
    let tick = (t * 30.0) as u32;
    let amp = p * 8.0;
    let w = c.w;
    for band in 0..5u32 {
        if rnd(band, tick) > 0.3 + (1.0 - p) * 0.5 {
            continue;
        }
        let y0 = (rnd(band + 10, tick) * c.h as f32) as usize;
        let h = 2 + (rnd(band + 20, tick) * 8.0) as usize;
        let dx = ((rnd(band + 30, tick) - 0.5) * 2.0 * amp) as isize;
        for y in y0..(y0 + h).min(c.h) {
            let row: Vec<Rgb> = c.px[y * w..(y + 1) * w].to_vec();
            for x in 0..w {
                let sx = (x as isize - dx).clamp(0, w as isize - 1) as usize;
                c.px[y * w + x] = row[sx];
            }
        }
    }
    // Red channel offset.
    let off = (p * 3.0).round() as usize;
    if off > 0 {
        for y in 0..c.h {
            for x in (0..w).rev() {
                if x >= off {
                    c.px[y * w + x][0] = c.px[y * w + x - off][0];
                }
            }
        }
    }
    if p > 0.5 {
        let y = (rnd(99, tick) * c.h as f32) as usize;
        for x in 0..w {
            if y < c.h {
                c.px[y * w + x] = mix(c.px[y * w + x], mix(accent, WHITE, 0.5), 0.8);
            }
        }
    }
}

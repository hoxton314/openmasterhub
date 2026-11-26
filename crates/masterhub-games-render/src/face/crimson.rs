//! The game look for Crimson Desert, after the feel of the game's menus: near-black warm
//! ground, hatched panels, gold bands, cream serif text, bone-white key stamps (our own
//! drawing; colours measured from the user's screenshots, nothing taken from the game).
//! Every part has variants ([`CrimsonStyle`]) so a look can be assembled from a proposals board.

use tiny_skia::{FillRule, PathBuilder};

use super::{Painter, rgbf};
use crate::canvas::{Canvas, Clip, Cover, Rgb, annulus, arc, circle, mix, rgb};
use crate::text::{Anchor, FontId};
use crate::{DialCard, DialFace, KeyFace};

const K: usize = 72;
pub(crate) const BG: Rgb = rgb(0x12100f);
const PANEL: Rgb = rgb(0x2a2927);
pub(crate) const GOLD: Rgb = rgb(0xc8962e);
pub(crate) const GOLD_HI: Rgb = rgb(0xffe39d);
pub(crate) const CREAM: Rgb = rgb(0xefe6d2);
const BONE: Rgb = rgb(0xe8e5e0);
const INK: Rgb = rgb(0x15110c);
const DIM: Rgb = rgb(0x8f877a);
pub(crate) const EMBER: Rgb = rgb(0xff7a2e);

/// The key's build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrimsonFrame {
    /// A dark plaque with a thin line frame, corner studs and a rule under the label.
    #[default]
    Plaque,
    /// A hatched colour band across the top carrying the label in dark ink (the game's
    /// selected menu row).
    Banner,
    /// The icon on a worn bone-white stamp (the game's key prompts), label below.
    Stamp,
    /// The icon on a soft dark medallion with a thin ring (the game's journal cards).
    Seal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrimsonFont {
    /// IM Fell English SC: old and slightly rough, closest to the game's menus.
    #[default]
    Fell,
    /// Cinzel Bold: clean Roman capitals.
    Cinzel,
    /// Cormorant SC Bold: sharp, high contrast.
    Cormorant,
}

impl CrimsonFont {
    fn id(self) -> FontId {
        match self {
            CrimsonFont::Fell => FontId::Fell,
            CrimsonFont::Cinzel => FontId::Cinzel,
            CrimsonFont::Cormorant => FontId::Cormorant,
        }
    }

    /// Label size on a key: the faces differ in how big they draw at the same size.
    fn label(self) -> f32 {
        match self {
            CrimsonFont::Fell => 12.5,
            CrimsonFont::Cinzel => 10.0,
            CrimsonFont::Cormorant => 12.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrimsonIcons {
    /// Icons cut from the game's own menus where we have one, drawn ones for the rest.
    #[default]
    Game,
    /// Drawn fantasy icons (game-icons.net) on every key.
    Drawn,
    /// The plain rounded glyphs of daily mode.
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrimsonPress {
    /// A glow rises from the bottom edge and sparks fly up.
    #[default]
    Ember,
    /// The whole key turns into a hatched gold band and fades back.
    Gild,
    /// The key stamps bone-white and the ink wears off in speckles.
    Ink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrimsonRing {
    /// Two thin lines with studs at the quarters; progress as an arc between them.
    #[default]
    Studs,
    /// A broad hatched band.
    Band,
    /// Sun rays: short and long ticks round the edge, lit up to the value.
    Rays,
}

/// One assembled Crimson Desert look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CrimsonStyle {
    pub frame: CrimsonFrame,
    pub font: CrimsonFont,
    pub icons: CrimsonIcons,
    pub press: CrimsonPress,
    pub ring: CrimsonRing,
}

fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

fn rect(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, col: Rgb, a: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);
    pb.line_to(x + w, y);
    pb.line_to(x + w, y + h);
    pb.line_to(x, y + h);
    pb.close();
    if let Some(p) = pb.finish() {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::Winding), col, a);
    }
}

fn disc(c: &mut Canvas, cx: f32, cy: f32, r: f32, col: Rgb, a: f32) {
    if let Some(p) = circle(cx, cy, r) {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::Winding), col, a);
    }
}

fn ring(c: &mut Canvas, cx: f32, cy: f32, r_in: f32, r_out: f32, col: Rgb, a: f32) {
    if let Some(p) = annulus(cx, cy, r_in, r_out) {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::EvenOdd), col, a);
    }
}

/// A small diamond stud.
fn stud(c: &mut Canvas, cx: f32, cy: f32, r: f32, col: Rgb) {
    let mut pb = PathBuilder::new();
    pb.move_to(cx, cy - r);
    pb.line_to(cx + r, cy);
    pb.line_to(cx, cy + r);
    pb.line_to(cx - r, cy);
    pb.close();
    if let Some(p) = pb.finish() {
        c.paint(&Cover::from_path(c.w, c.h, &p, FillRule::Winding), col, 1.0);
    }
}

/// The diagonal hatching of the game's menu rows: every fourth diagonal darker by `a`.
pub(crate) fn hatch(c: &mut Canvas, x0: usize, y0: usize, x1: usize, y1: usize, a: f32) {
    for y in y0..y1.min(c.h) {
        for x in x0..x1.min(c.w) {
            if (x + y) % 4 == 0 {
                let d = &mut c.px[y * c.w + x];
                *d = mix(*d, [0.0; 3], a);
            }
        }
    }
}

/// Fine grain, the same on every frame, so panels read as worn slate instead of flat colour.
pub(crate) fn grain(c: &mut Canvas, x0: usize, y0: usize, x1: usize, y1: usize, amp: f32) {
    for y in y0..y1.min(c.h) {
        for x in x0..x1.min(c.w) {
            let n = (rnd(x as u32 * 7 + 3, y as u32 * 13 + 1) - 0.5) * 2.0 * amp;
            for k in c.px[y * c.w + x].iter_mut() {
                *k = (*k + n).clamp(0.0, 255.0);
            }
        }
    }
}

/// A rounded square (the stamp).
fn rounded(cx: f32, cy: f32, half: f32, r: f32) -> Option<tiny_skia::Path> {
    let rect = tiny_skia::Rect::from_ltrb(cx - half, cy - half, cx + half, cy + half)?;
    let mut pb = PathBuilder::new();
    pb.push_rect(tiny_skia::Rect::from_ltrb(rect.left() + r, rect.top(), rect.right() - r, rect.bottom())?);
    pb.push_rect(tiny_skia::Rect::from_ltrb(rect.left(), rect.top() + r, rect.right(), rect.bottom() - r)?);
    for (x, y) in [(rect.left() + r, rect.top() + r), (rect.right() - r, rect.top() + r), (rect.left() + r, rect.bottom() - r), (rect.right() - r, rect.bottom() - r)] {
        pb.push_circle(x, y, r);
    }
    pb.finish()
}

impl Painter {
    /// A key in the Crimson Desert look. `ground` tints the panel, `accent` is the key's colour.
    pub(crate) fn key_crimson(&self, st: &CrimsonStyle, face: &KeyFace, t: f32, press: f32) -> Canvas {
        let p = if press.is_finite() { press.clamp(0.0, 1.0) } else { 0.0 };
        let accent = face.accent.map_or(GOLD, rgbf);
        let tint = mix(PANEL, face.ground.map_or(PANEL, rgbf), 0.45);
        let font = st.font.id();
        // The label shrinks to fit before it has to scroll.
        let size = {
            let s = st.font.label();
            let w = self.fonts.width(font, s, &face.label);
            if w > 58.0 { (s * 58.0 / w).max(s * 0.6) } else { s }
        };
        let mut c = Canvas::new(K, K, BG);
        let full = Clip { x0: 0, y0: 0, x1: K as i32, y1: K as i32 };

        // The panel under everything.
        rect(&mut c, 2.0, 2.0, 68.0, 68.0, tint, 1.0);
        hatch(&mut c, 2, 2, 70, 70, 0.1);
        grain(&mut c, 2, 2, 70, 70, 5.0);

        let mut label_ink = mix(accent, GOLD_HI, 0.45);
        let mut icon_ink = BONE;
        let (label_y, icon_y, icon_size);
        match st.frame {
            CrimsonFrame::Plaque => {
                for (x, y, w, h) in [(3.0, 3.0, 66.0, 1.0), (3.0, 68.0, 66.0, 1.0), (3.0, 3.0, 1.0, 66.0), (68.0, 3.0, 1.0, 66.0)] {
                    rect(&mut c, x, y, w, h, accent, 0.55);
                }
                for (x, y) in [(3.5, 3.5), (68.5, 3.5), (3.5, 68.5), (68.5, 68.5)] {
                    stud(&mut c, x, y, 2.2, mix(accent, GOLD_HI, 0.3));
                }
                rect(&mut c, 12.0, 22.0, 48.0, 1.0, accent, 0.6);
                disc(&mut c, 36.0, 22.5, 3.0, tint, 1.0);
                ring(&mut c, 36.0, 22.5, 1.6, 2.8, accent, 1.0);
                (label_y, icon_y, icon_size) = (12.5, 46.0, 30.0);
            }
            CrimsonFrame::Banner => {
                rect(&mut c, 2.0, 2.0, 68.0, 20.0, mix(BG, accent, 0.82), 1.0);
                hatch(&mut c, 2, 2, 70, 22, 0.12);
                rect(&mut c, 2.0, 22.0, 68.0, 1.0, mix(accent, GOLD_HI, 0.5), 0.9);
                rect(&mut c, 2.0, 69.0, 68.0, 1.0, accent, 0.6);
                label_ink = INK;
                (label_y, icon_y, icon_size) = (12.0, 46.0, 32.0);
            }
            CrimsonFrame::Stamp => {
                if let Some(path) = rounded(36.0, 29.0, 17.0, 5.0) {
                    c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), BONE, 1.0);
                }
                // Wear: speckles and a nibbled edge, the same on every frame.
                for y in 12..46usize {
                    for x in 19..53usize {
                        let edge = x < 21 || x > 50 || y < 14 || y > 43;
                        if rnd(x as u32 * 131 + y as u32, 7) > if edge { 0.7 } else { 0.985 } {
                            let d = &mut c.px[y * K + x];
                            *d = mix(*d, tint, 0.55);
                        }
                    }
                }
                rect(&mut c, 20.0, 67.0, 32.0, 1.0, accent, 0.9);
                icon_ink = INK;
                label_ink = mix(accent, CREAM, 0.5);
                (label_y, icon_y, icon_size) = (57.0, 29.0, 25.0);
            }
            CrimsonFrame::Seal => {
                if let Some(path) = circle(36.0, 44.0, 21.0) {
                    let mut soft = Cover::from_path(K, K, &path, FillRule::Winding);
                    soft.blur(1.6);
                    c.paint(&soft, mix(BG, [0.0; 3], 0.35), 0.92);
                }
                ring(&mut c, 36.0, 44.0, 21.6, 22.6, accent, 0.75);
                (label_y, icon_y, icon_size) = (11.5, 44.0, 27.0);
            }
        }

        // Label and minor line.
        if !face.label.is_empty() {
            self.crimson_line(&mut c, font, size, &face.label, label_y, label_ink, t, 64.0);
        }
        if let Some(m) = face.minor.as_deref().filter(|m| !m.is_empty()) {
            let y = if st.frame == CrimsonFrame::Stamp { 47.0 } else { 64.0 };
            if st.frame != CrimsonFrame::Stamp && st.frame != CrimsonFrame::Seal {
                self.crimson_line(&mut c, font, size - 2.5, m, y, DIM, t, 60.0);
            }
        }
        if let Some(orb) = face.orb {
            stud(&mut c, 62.0, if st.frame == CrimsonFrame::Banner { 30.0 } else { 11.0 }, 4.0, super::orb_color(orb, t));
        }

        // Icon: a drawing when the key names one, else the glyph. It grows a little on a press.
        let pop = 1.0 + 0.22 * (p * std::f32::consts::PI).sin();
        let ink = if st.press == CrimsonPress::Ember { mix(icon_ink, GOLD_HI, p * 0.8) } else { icon_ink };
        // `logo` lists the key's pictures, best first ("cd-backpack|knapsack"): a cut-out from
        // the game, then a drawing.
        let drawn = face.logo.as_deref().and_then(|l| {
            l.split('|').find(|n| match st.icons {
                CrimsonIcons::Game => crate::art::exists(n),
                CrimsonIcons::Drawn => !crate::art::from_game(n) && crate::art::exists(n),
                CrimsonIcons::Plain => false,
            })
        });
        // The game's cut-outs carry their own margin: a little larger to match.
        let icon_size = if drawn.is_some_and(crate::art::from_game) { icon_size * 1.12 } else { icon_size };
        if let Some(cover) = drawn.and_then(|n| crate::art::cover(n, K, K, icon_size * pop, 36.0, icon_y)) {
            c.paint(&cover, ink, 1.0);
        } else if let Some(icon) = face.icon.filter(|&i| self.fonts.has_glyph(FontId::Icons, i)) {
            let s = icon.encode_utf8(&mut [0; 4]).to_owned();
            self.fonts.draw(&mut c, FontId::Icons, icon_size * 0.78 * pop, &s, 36.0, icon_y, Anchor::Middle, ink, full);
        }

        if p > 0.0 {
            match st.press {
                CrimsonPress::Ember => embers(&mut c, p, accent),
                CrimsonPress::Gild => {
                    // A gold band sweeping left to right, hatched like a selected row.
                    let x = (1.0 - p) * 110.0 - 30.0;
                    for py in 2..70usize {
                        for px in 2..70usize {
                            let dx = (px as f32 + py as f32 * 0.35) - x;
                            let a = (1.0 - (dx / 26.0).abs()).clamp(0.0, 1.0) * 0.85 + p * 0.18;
                            let d = &mut c.px[py * K + px];
                            let band = if (px + py) % 4 == 0 { mix(GOLD, [0.0; 3], 0.2) } else { mix(GOLD, GOLD_HI, 0.25) };
                            *d = mix(*d, band, a.min(1.0));
                        }
                    }
                }
                CrimsonPress::Ink => {
                    for py in 2..70usize {
                        for px in 2..70usize {
                            if rnd(px as u32 * 73 + 5, py as u32 * 31 + 1) < p * 1.25 {
                                let d = &mut c.px[py * K + px];
                                // Light where it was dark and the other way: a stamped print.
                                let lum = (d[0] + d[1] + d[2]) / 3.0;
                                *d = mix(*d, if lum > 120.0 { INK } else { BONE }, 0.9);
                            }
                        }
                    }
                }
            }
        }
        c
    }

    /// Centred line; wider than `max` it scrolls.
    #[allow(clippy::too_many_arguments)]
    fn crimson_line(&self, c: &mut Canvas, font: FontId, size: f32, text: &str, y: f32, col: Rgb, t: f32, max: f32) {
        let w = self.fonts.width(font, size, text);
        let x0 = 36.0 - max / 2.0;
        let clip = Clip { x0: x0 as i32, y0: 0, x1: (x0 + max) as i32, y1: K as i32 };
        if w <= max {
            self.fonts.draw(c, font, size, text, 36.0, y, Anchor::Middle, col, clip);
            return;
        }
        let span = w + 22.0;
        let x = x0 + 2.0 - (t.max(0.0) * 22.0) % span;
        for dx in [0.0, span] {
            self.fonts.draw(c, font, size, text, x + dx, y, Anchor::Left, col, clip);
        }
    }

    /// The dial in the Crimson Desert look.
    pub(crate) fn dial_crimson(&self, st: &CrimsonStyle, face: &DialFace, t: f32) -> Canvas {
        let d = crate::DIAL_SIZE as usize;
        let mut c = Canvas::new(d, d, BG);
        let font = st.font.id();
        let (eyebrow, big, sub, line, col, progress, breathe, gauge, dot) = match &face.card {
            Some(DialCard { eyebrow, big, sub, line, ring, progress, breathe, seconds_dot, .. }) => {
                (eyebrow.clone(), big.clone(), sub.clone(), line.clone(), ring.map_or(GOLD, rgbf), *progress, *breathe, false, *seconds_dot)
            }
            None => (face.name.clone(), face.big.clone(), face.status.clone(), None, GOLD, Some(face.value), false, true, false),
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

        // A faint hatched disc inside the ring, like the game's panels.
        disc(&mut c, 120.5, 120.5, 94.0, mix(BG, PANEL, 0.55), 1.0);
        hatch(&mut c, 0, 0, d, d, 0.1);
        grain(&mut c, 0, 0, d, d, 4.0);
        // The game's serpent ring as a faint watermark, as on its settings screens.
        if st.icons == CrimsonIcons::Game
            && let Some(em) = crate::art::cover("cd-emblem", d, d, 170.0, 120.0, 120.0)
        {
            c.paint(&em, mix(PANEL, CREAM, 0.25), 0.22);
        }
        match st.ring {
            CrimsonRing::Studs => {
                paint_arc(&mut c, 111.0, 1.5, a0, a0 + span, mix(BG, col, 0.6));
                paint_arc(&mut c, 99.0, 1.0, a0, a0 + span, mix(BG, col, 0.45));
                paint_arc(&mut c, 105.0, 5.0, a0, a0 + span, mix(BG, col, 0.14));
                paint_arc(&mut c, 105.0, 5.0, a0, a0 + span * lit, col);
                let n = if gauge { 3 } else { 4 };
                for i in 0..=n {
                    if !gauge && i == n {
                        break;
                    }
                    let a = (a0 + span * i as f32 / n as f32).to_radians();
                    let (x, y) = (120.5 + 105.0 * a.cos(), 120.5 + 105.0 * a.sin());
                    stud(&mut c, x, y, 8.0, BG);
                    stud(&mut c, x, y, 6.0, mix(col, GOLD_HI, 0.35));
                }
            }
            CrimsonRing::Band => {
                paint_arc(&mut c, 104.0, 14.0, a0, a0 + span, mix(BG, col, 0.2));
                paint_arc(&mut c, 104.0, 14.0, a0, a0 + span * lit, mix(BG, col, 0.9));
                // Hatch only the band.
                for y in 0..d {
                    for x in 0..d {
                        let r = ((x as f32 - 120.0).powi(2) + (y as f32 - 120.0).powi(2)).sqrt();
                        if (97.0..111.0).contains(&r) && (x + y) % 4 == 0 {
                            let p = &mut c.px[y * d + x];
                            *p = mix(*p, [0.0; 3], 0.25);
                        }
                    }
                }
                paint_arc(&mut c, 112.0, 1.0, a0, a0 + span, mix(col, GOLD_HI, 0.5));
                paint_arc(&mut c, 96.0, 1.0, a0, a0 + span, mix(BG, col, 0.6));
            }
            CrimsonRing::Rays => {
                let n = 72;
                for i in 0..n {
                    let frac = (i as f32 + 0.5) / n as f32;
                    let a = a0 + span * frac;
                    let long = i % 6 == 0;
                    let (r, w) = if long { (104.0, 16.0) } else { (106.0, 9.0) };
                    let on = frac <= lit;
                    paint_arc(&mut c, r, w, a - if long { 1.0 } else { 0.6 }, a + if long { 1.0 } else { 0.6 }, if on { if long { mix(col, GOLD_HI, 0.4) } else { col } } else { mix(BG, col, 0.2) });
                }
                paint_arc(&mut c, 94.0, 1.0, a0, a0 + span, mix(BG, col, 0.5));
            }
        }
        if dot {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |x| x.as_secs_f32());
            let a = (secs % 60.0) / 60.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            disc(&mut c, 120.5 + 105.0 * a.cos(), 120.5 + 105.0 * a.sin(), 4.0, GOLD_HI, 1.0);
        }

        // A rule with a ring stud between the eyebrow and the value, as under the game's tabs.
        rect(&mut c, 76.0, 80.0, 88.0, 1.0, col, 0.5);
        disc(&mut c, 120.5, 80.5, 4.0, BG, 1.0);
        ring(&mut c, 120.5, 80.5, 2.0, 3.4, mix(col, GOLD_HI, 0.3), 1.0);

        // Lines shrink to fit before they have to scroll.
        let fit = |font: FontId, size: f32, text: &str, max: f32, min: f32| {
            let w = self.fonts.width(font, size, text);
            if w > max { (size * max / w).max(min) } else { size }
        };
        self.dial_line(&mut c, font, fit(font, 15.0, &eyebrow, 118.0, 11.0), &eyebrow, 62.0, mix(col, GOLD_HI, 0.5), t, 122.0);
        self.dial_line(&mut c, FontId::Cinzel, fit(FontId::Cinzel, 54.0, &big, 150.0, 26.0), &big, 118.0, CREAM, t, 156.0);
        if let Some(s) = sub.as_deref() {
            self.dial_line(&mut c, font, 15.0, s, 158.0, mix(CREAM, DIM, 0.25), t, 160.0);
        }
        if let Some(l) = line.as_deref() {
            self.dial_line(&mut c, font, 13.0, l, 180.0, DIM, t, 136.0);
        }
        c.mask_black(self.dial_disc());
        c
    }
}

/// Ember press: a glow rising from the bottom edge and sparks flying up as it fades.
fn embers(c: &mut Canvas, p: f32, accent: Rgb) {
    let (w, h) = (c.w, c.h);
    let glow = mix(accent, EMBER, 0.5);
    for y in 0..h {
        let k = ((y as f32 - 26.0) / 46.0).clamp(0.0, 1.0);
        let a = k * k * p * 0.75;
        if a > 0.0 {
            for x in 0..w {
                let d = &mut c.px[y * w + x];
                for i in 0..3 {
                    d[i] = (d[i] + glow[i] * a).min(255.0);
                }
            }
        }
    }
    let q = 1.0 - p;
    for i in 0..14u32 {
        let x = 6.0 + rnd(i, 11) * 60.0 + (q * 9.0 + i as f32).sin() * 3.0;
        let y = 70.0 - q * (26.0 + rnd(i, 12) * 52.0);
        let r = 0.8 + rnd(i, 13) * 1.1;
        if let Some(path) = circle(x, y, r) {
            c.paint(&Cover::from_path(w, h, &path, FillRule::Winding), mix(EMBER, GOLD_HI, rnd(i, 14)), (p * 1.6).min(1.0));
        }
    }
}

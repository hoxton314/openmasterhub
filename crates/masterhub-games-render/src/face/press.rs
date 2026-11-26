//! The press looks ([`PressStyle`]). `p` is the press intensity, 1.0 on the press frame down to
//! 0.0 after `PRESS_SECS`; `u = 1 - p` is the time through the press. Glare is the look approved
//! on the hardware and keeps the prototype's numbers exactly; the others are gallery candidates.

use std::f32::consts::TAU;

use tiny_skia::{FillRule, PathBuilder, Rect};

use super::{CREAM, K, MARIGOLD, PIX_DIM, PLUM_DEEP, RING};
use crate::PressStyle;
use crate::canvas::{Canvas, Cover, Rgb, annulus, mix};

/// Near-black of Impact's held frame.
const INK: Rgb = [7.0, 4.0, 11.0];
/// A hotter cream for icons at the peak of a press.
const HOT: Rgb = [255.0, 246.0, 214.0];

/// What a press does to the parts of the face; the shapes drawn on top come from the functions
/// below.
pub struct Fx {
    pub ground: Rgb,
    /// Content sink in px (0 or 1).
    pub oy: f32,
    /// Icon scale, and whether the size snaps to whole px (Glare, like Python's round()).
    scale: Option<(f32, bool)>,
    /// Colour of the blurred icon glow, added.
    pub glow: Option<Rgb>,
    pub icon: Rgb,
    pub text: Rgb,
    pub minor: Rgb,
    /// The label's engraved shadow.
    pub shadow: bool,
    /// The progress ring's track.
    pub track: Rgb,
}

impl Fx {
    pub fn new(style: PressStyle, p: f32, ground: Rgb, effect: Rgb) -> Self {
        // Idle (p = 0) is Glare's formulas at p = 0, for every style.
        let mut fx = Fx {
            ground: mix(ground, RING, 0.6 * p),
            oy: if p > 0.0 { 1.0 } else { 0.0 },
            scale: None,
            glow: None,
            icon: mix(CREAM, MARIGOLD, p),
            text: CREAM,
            minor: PIX_DIM,
            shadow: true,
            track: mix(RING, MARIGOLD, p),
        };
        if p <= 0.0 {
            return fx;
        }
        let u = 1.0 - p;
        match style {
            PressStyle::Glare => {
                fx.scale = Some((1.0 + 0.25 * p, true));
                fx.glow = Some(mix([0.0; 3], MARIGOLD, p));
            }
            PressStyle::Ripple => {
                fx.ground = mix(ground, effect, 0.22 * p * p);
                let b = p * p;
                fx.scale = Some((1.0 + 0.12 * b, false));
                fx.icon = mix(CREAM, HOT, b);
                fx.glow = Some(mix([0.0; 3], effect, 0.9 * b));
                fx.track = mix(RING, effect, p);
            }
            PressStyle::Flash => {
                // Held near cream for the first quarter, then falling off fast.
                let f = if p > 0.75 { 0.93 } else { 0.93 * (p / 0.75).powi(3) };
                fx.ground = mix(ground, CREAM, f);
                let inverted = f > 0.45;
                if inverted {
                    fx.icon = PLUM_DEEP;
                    fx.text = PLUM_DEEP;
                    fx.minor = PLUM_DEEP;
                    fx.track = mix(CREAM, PLUM_DEEP, 0.45);
                    fx.shadow = false;
                } else {
                    fx.icon = CREAM;
                    fx.track = RING;
                }
                fx.scale = Some((1.0 + 0.1 * p * p, false));
            }
            PressStyle::Spring => {
                fx.ground = ground;
                fx.oy = 0.0;
                fx.scale = Some((spring(u), false));
                fx.icon = mix(CREAM, HOT, p);
                fx.glow = Some(mix([0.0; 3], effect, 0.45 * p));
                fx.track = RING;
            }
            PressStyle::Impact => {
                if p > IMPACT_HOLD {
                    fx.ground = INK;
                    fx.scale = Some((1.3, false));
                    fx.icon = HOT;
                    fx.glow = Some(mix([0.0; 3], effect, 0.8));
                    fx.track = effect;
                } else {
                    let k = p / IMPACT_HOLD; // 1 → 0 after the hold
                    fx.ground = mix(ground, INK, k * k);
                    fx.scale = Some((1.0 + 0.3 * k * k * k, false));
                    fx.icon = mix(CREAM, HOT, k);
                    fx.glow = Some(mix([0.0; 3], effect, 0.6 * k * k));
                    fx.track = mix(RING, effect, k);
                }
            }
            PressStyle::Shutter => {
                fx.ground = ground;
                fx.icon = CREAM;
                fx.track = RING;
            }
        }
        fx
    }

    /// The icon size for an idle `base` size.
    pub fn icon_size(&self, base: f32) -> f32 {
        match self.scale {
            None => base,
            Some((k, true)) => (base * k).round_ties_even(),
            // Half-pixel steps keep the glyph raster cache small.
            Some((k, false)) => (base * k * 2.0).round() / 2.0,
        }
    }
}

/// Spring's icon scale: squashed to 0.8 for a couple of frames, overshooting to ~1.17, settling.
fn spring(u: f32) -> f32 {
    let u = ((u - 0.06) / 0.94).max(0.0);
    let w = TAU * 1.6;
    1.0 + (-4.5 * u).exp() * (-0.2 * (w * u).cos() + 0.35 * (w * u).sin())
}

fn ease_in_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x < 0.5 { 4.0 * x * x * x } else { 1.0 - (-2.0 * x + 2.0).powi(3) / 2.0 }
}

/// Ripple: two rings expanding from the icon past the key's corners, thinning and fading.
pub fn ripple(c: &mut Canvas, p: f32, effect: Rgb, cy: f32) {
    let u = 1.0 - p;
    let col = mix(effect, CREAM, 0.4);
    for (delay, strength) in [(0.0, 1.0), (0.22, 0.75)] {
        let k = (u - delay) / (1.0 - delay);
        if !(0.0..1.0).contains(&k) {
            continue;
        }
        let e = 1.0 - (1.0 - k).powf(1.6);
        let r = 10.0 + e * 58.0;
        let w = 5.0 - 3.0 * e;
        let a = strength * (1.0 - k).powf(0.9);
        if let Some(path) = annulus(36.0, cy, (r - w / 2.0).max(0.0), r + w / 2.0) {
            c.paint(&Cover::from_path(K, K, &path, FillRule::EvenOdd), col, a);
        }
    }
}

/// Impact's held frame: the first 12 % (two frames at 40 fps).
const IMPACT_HOLD: f32 = 0.88;

/// Impact: an anime impact frame. On the hold a spiky ink burst sits behind the icon with sharp
/// shards round it on a near-black ground; then the shards fly out and thin to nothing.
pub fn impact(c: &mut Canvas, p: f32, effect: Rgb, cy: f32) {
    const N: usize = 11;
    const JITTER: [f32; N] = [-7.0, 6.0, -3.0, 9.0, -6.0, 2.0, -9.0, 5.0, -2.0, 8.0, -4.0];
    const LEN: [f32; N] = [54.0, 40.0, 60.0, 45.0, 56.0, 38.0, 52.0, 62.0, 42.0, 50.0, 46.0];
    const HALF: [f32; N] = [3.4, 2.4, 3.8, 2.8, 3.2, 2.2, 3.0, 4.0, 2.4, 3.2, 2.6];
    let held = p > IMPACT_HOLD;
    // 0 during the hold, then 0 → 1.
    let k = if held { 0.0 } else { 1.0 - p / IMPACT_HOLD };
    let (cx, cy) = (36.0, cy);

    // The burst: a jagged star, on the hold and one beat after, growing as it goes.
    let burst_a = if held { 1.0 } else { (1.0 - k / 0.12).max(0.0) };
    if burst_a > 0.0 {
        let grow = 1.0 + 0.6 * k;
        let mut pb = PathBuilder::new();
        let pts = 14;
        for i in 0..pts * 2 {
            let a = (i as f32 / (pts * 2) as f32) * TAU + 0.2;
            let r = if i % 2 == 0 { [27.0, 21.0, 24.0][i / 2 % 3] } else { 13.0 } * grow;
            let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
            if i == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
        }
        pb.close();
        if let Some(path) = pb.finish() {
            c.paint(&Cover::from_path(K, K, &path, FillRule::Winding), effect, burst_a);
        }
    }

    // The shards: thin diamonds pointing at the icon.
    let mut cov = Cover::new(K, K);
    for i in 0..N {
        let a = (i as f32 * 360.0 / N as f32 + JITTER[i] - 90.0).to_radians();
        let r_out = 15.0 + LEN[i] + 24.0 * k;
        let r_in = 15.0 + k * (r_out - 15.0);
        let len = r_out - r_in;
        if len < 1.0 {
            continue;
        }
        let hw = HALF[i] * (1.0 - 0.6 * k);
        let r_mid = r_in + 0.7 * len;
        let (dx, dy) = (a.cos(), a.sin());
        let (nx, ny) = (-dy, dx);
        let mut pb = PathBuilder::new();
        pb.move_to(cx + dx * r_in, cy + dy * r_in);
        pb.line_to(cx + dx * r_mid + nx * hw, cy + dy * r_mid + ny * hw);
        pb.line_to(cx + dx * r_out, cy + dy * r_out);
        pb.line_to(cx + dx * r_mid - nx * hw, cy + dy * r_mid - ny * hw);
        pb.close();
        if let Some(path) = pb.finish() {
            cov.fill_path(&path, FillRule::Winding, 1.0);
        }
    }
    let col = if held { mix(effect, CREAM, 0.35) } else { effect };
    c.paint(&cov, col, (1.0 - k).powf(0.6));
}

/// Shutter: four slats in the effect colour cover the key on the press and slide off in turn,
/// alternating sides, each with a bright leading edge.
pub fn shutter(c: &mut Canvas, p: f32, effect: Rgb) {
    const N: usize = 4;
    let u = 1.0 - p;
    let h = K as f32 / N as f32;
    let edge = mix(effect, CREAM, 0.6);
    let seam = mix(effect, [0.0; 3], 0.45);
    for i in 0..N {
        let k = ease_in_out((u - 0.1 * i as f32) / 0.55);
        if k >= 1.0 {
            continue;
        }
        let shift = k * (K as f32 + 4.0);
        let y = i as f32 * h;
        // Even slats leave to the right, odd ones to the left.
        let (x0, x1) = if i % 2 == 0 { (shift, K as f32 + 4.0) } else { (-4.0, K as f32 - shift) };
        let lead = if i % 2 == 0 { x0 } else { x1 - 2.0 };
        let fill = |c: &mut Canvas, x0: f32, x1: f32, y0: f32, y1: f32, col: Rgb| {
            if let Some(r) = Rect::from_ltrb(x0, y0, x1, y1) {
                c.paint(&Cover::from_path(K, K, &PathBuilder::from_rect(r), FillRule::Winding), col, 1.0);
            }
        };
        fill(c, x0, x1, y, y + h, effect);
        // A darker seam under each slat.
        fill(c, x0, x1, y + h - 1.0, y + h, seam);
        let fill = |c: &mut Canvas, x0: f32, x1: f32, col: Rgb| fill(c, x0, x1, y, y + h, col);
        if k > 0.0 {
            fill(c, lead, lead + 2.0, edge);
        }
    }
}

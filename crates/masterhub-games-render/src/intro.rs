//! Game intros: one picture spread over all 15 key screens (the gaps between keys are part of
//! the picture, so it reads as one screen behind a grille) plus the dial.

use image::RgbImage;

use crate::canvas::{Canvas, Clip, Rgb, mix};
use crate::text::{Anchor, FontId};
use crate::{DIAL_SIZE, KEY_SIZE};

/// Pixels between two key screens on the virtual canvas (about the real gap at key scale).
pub const GAP: u32 = 16;
const K: usize = KEY_SIZE as usize;
const VW: usize = 5 * K + 4 * GAP as usize;
const VH: usize = 3 * K + 2 * GAP as usize;

const BLACK: Rgb = [5.0, 4.0, 8.0];
const YELLOW: Rgb = [252.0, 238.0, 10.0];
const CYAN: Rgb = [2.0, 215.0, 242.0];
const RED: Rgb = [255.0, 59.0, 59.0];

/// The intro styles a game mode can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntroStyle {
    /// Scanline sweep, yellow and red glitch blocks, LINK / ESTABLISHED sliced across the keys.
    #[default]
    Glitch,
    /// Crimson Desert: a gold rule draws out from the middle, the title glows in, embers rise.
    Ember,
    /// Deadlock: the game's emblem assembles in soul green over film grain, then the name.
    Emblem,
    /// WoW: a hearthstone cast. Blue arcane light spirals in under orbiting runes, the dial
    /// fills like the cast bar, a flash, then the keys.
    Hearth,
    /// WoW: a level-up. A pillar of golden light rises through the keypad, gold dust, the
    /// title in Morpheus.
    Ding,
}

/// Small deterministic hash → 0..1.
fn rnd(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

fn rect(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, col: Rgb, a: f32) {
    let (x0, y0) = (x.max(0.0) as usize, y.max(0.0) as usize);
    let (x1, y1) = (((x + w).max(0.0) as usize).min(c.w), ((y + h).max(0.0) as usize).min(c.h));
    for py in y0..y1 {
        for px in x0..x1 {
            let d = &mut c.px[py * c.w + px];
            *d = mix(*d, col, a);
        }
    }
}

impl crate::face::Painter {
    /// Frame at `t` of a `dur`-second intro: the 15 key images (row-major) and the dial.
    pub fn intro(&self, style: IntroStyle, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        if style == IntroStyle::Ember {
            return self.intro_ember(t, dur);
        }
        if style == IntroStyle::Emblem {
            return self.intro_emblem(t, dur);
        }
        if style == IntroStyle::Hearth {
            return self.intro_hearth(t, dur);
        }
        if style == IntroStyle::Ding {
            return self.intro_ding(t, dur);
        }
        let p = (t / dur).clamp(0.0, 1.0);
        let f = &self.fonts;
        // Blocks and text fade out over the last quarter, where the caller blends in the keys.
        let fade = if p < 0.75 { 1.0 } else { ((1.0 - p) * 4.0).max(0.0) };
        let tick = (t * 14.0) as u32;

        let mut v = Canvas::new(VW, VH, BLACK);
        let all = Clip { x0: 0, y0: 0, x1: VW as i32, y1: VH as i32 };
        // Faint scanlines.
        for y in (0..VH).step_by(3) {
            rect(&mut v, 0.0, y as f32, VW as f32, 1.0, [18.0, 14.0, 26.0], 1.0);
        }
        // A cyan band sweeping down.
        let sy = (p * 1.6).fract() * (VH as f32 + 80.0) - 40.0;
        rect(&mut v, 0.0, sy - 18.0, VW as f32, 36.0, CYAN, 0.16 * fade);
        rect(&mut v, 0.0, sy, VW as f32, 2.0, CYAN, 0.6 * fade);
        // Glitch blocks, reshuffled 14 times a second.
        for i in 0..16 {
            let r = rnd(i, tick);
            let col = if i % 3 == 0 { RED } else { YELLOW };
            let x = rnd(i + 100, tick) * VW as f32 - 20.0;
            let y = rnd(i + 200, tick) * VH as f32;
            rect(&mut v, x, y, 24.0 + r * 150.0, 3.0 + r * 12.0, col, 0.85 * fade);
        }
        // LINK / ESTABLISHED, jittering, with a red split behind.
        if (0.2..0.88).contains(&p) {
            let j = (t * 60.0).sin() * 4.0;
            let on = rnd(7, tick) > 0.08; // the odd dropped frame
            if on {
                let (x, y1, y2) = (VW as f32 / 2.0, VH as f32 * 0.30, VH as f32 * 0.72);
                f.draw(&mut v, FontId::Bebas, 120.0, "LINK", x + j + 3.0, y1, Anchor::Middle, mix(BLACK, RED, fade), all);
                f.draw(&mut v, FontId::Bebas, 120.0, "LINK", x + j, y1, Anchor::Middle, mix(BLACK, YELLOW, fade), all);
                f.draw(&mut v, FontId::Bebas, 92.0, "ESTABLISHED", x - j - 3.0, y2, Anchor::Middle, mix(BLACK, RED, fade), all);
                f.draw(&mut v, FontId::Bebas, 92.0, "ESTABLISHED", x - j, y2, Anchor::Middle, mix(BLACK, CYAN, fade), all);
            }
        }
        // Slice glitch: whole rows shifted sideways now and then.
        if rnd(3, tick) > 0.55 && fade > 0.0 {
            let y = (rnd(4, tick) * VH as f32) as usize;
            let h = 4 + (rnd(5, tick) * 14.0) as usize;
            let dx = ((rnd(6, tick) - 0.5) * 60.0) as isize;
            for py in y..(y + h).min(VH) {
                let row: Vec<Rgb> = v.px[py * VW..(py + 1) * VW].to_vec();
                for px in 0..VW {
                    let sx = (px as isize - dx).rem_euclid(VW as isize) as usize;
                    v.px[py * VW + px] = row[sx];
                }
            }
        }

        let keys = cut_keys(v);

        // The dial: a yellow ring filling, the band, and the word flickering.
        let d = DIAL_SIZE as usize;
        let mut dc = Canvas::new(d, d, BLACK);
        let dall = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        if let Some(path) = crate::canvas::arc(120.5, 120.5, 100.0, 8.0, -90.0, -90.0 + 360.0 * (p / 0.75).min(1.0)) {
            dc.paint(&crate::canvas::Cover::from_path(d, d, &path, tiny_skia::FillRule::Winding), mix(BLACK, YELLOW, fade.max(0.3)), 1.0);
        }
        let dy = (p * 2.3).fract() * 260.0 - 10.0;
        rect(&mut dc, 0.0, dy, d as f32, 2.0, CYAN, 0.7 * fade);
        if rnd(9, tick) > 0.1 {
            let word = if p < 0.5 { "BREACH" } else { "ONLINE" };
            f.draw(&mut dc, FontId::Bebas, 58.0, word, 120.0, 118.0, Anchor::Middle, mix(BLACK, YELLOW, fade), dall);
            f.draw(&mut dc, FontId::Mono, 13.0, "PULPIT // NIGHT CITY", 120.0, 160.0, Anchor::Middle, mix(BLACK, CYAN, fade), dall);
        }
        dc.mask_black(self.dial_disc());
        (keys, dc.into_image())
    }

    fn intro_ember(&self, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        use crate::face::crimson::{BG, CREAM, EMBER, GOLD, GOLD_HI, hatch};
        const CRIMSON: Rgb = [150.0, 22.0, 18.0];
        let p = (t / dur).clamp(0.0, 1.0);
        let f = &self.fonts;
        let fade = if p < 0.75 { 1.0 } else { ((1.0 - p) * 4.0).max(0.0) };
        let ease = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        let sparks = |c: &mut Canvas, n: u32, w: f32, h: f32| {
            for i in 0..n {
                let life = (t * (0.25 + rnd(i, 3) * 0.3) + rnd(i, 4)).fract();
                let x = rnd(i, 1) * w + (t * 2.0 + i as f32).sin() * 6.0;
                let y = h + 6.0 - life * (h + 20.0);
                let a = (1.0 - life) * fade * ease(p * 6.0);
                if let Some(path) = crate::canvas::circle(x, y, 0.9 + rnd(i, 2) * 1.6) {
                    c.paint(&crate::canvas::Cover::from_path(c.w, c.h, &path, tiny_skia::FillRule::Winding), mix(EMBER, GOLD_HI, rnd(i, 5)), a);
                }
            }
        };

        let mut v = Canvas::new(VW, VH, BG);
        let all = Clip { x0: 0, y0: 0, x1: VW as i32, y1: VH as i32 };
        hatch(&mut v, 0, 0, VW, VH, 0.3);
        let (cx, mid) = (VW as f32 / 2.0, VH as f32 / 2.0);
        // The title glows in behind its letters, then the letters.
        let show = ease((p - 0.12) / 0.3) * fade;
        if show > 0.0 {
            for (word, y, size) in [("CRIMSON", mid - 50.0, 74.0), ("DESERT", mid + 54.0, 74.0)] {
                let mut glow = f.cover(VW, VH, FontId::Fell, size, word, cx, y);
                glow.blur(9.0);
                v.add(&glow, mix([0.0; 3], CRIMSON, show * 1.6));
                f.draw(&mut v, FontId::Fell, size, word, cx, y, Anchor::Middle, mix(BG, mix(GOLD_HI, CREAM, 0.4), show), all);
            }
        }
        // The rule draws out from the middle, with a ring stud.
        let half = ease(p / 0.3) * (VW as f32 / 2.0 - 24.0);
        rect(&mut v, cx - half, mid - 1.0, half * 2.0, 2.0, GOLD, fade);
        if let Some(path) = crate::canvas::annulus(cx, mid, 4.0, 7.0) {
            rect(&mut v, cx - 9.0, mid - 9.0, 18.0, 18.0, BG, 1.0);
            v.paint(&crate::canvas::Cover::from_path(VW, VH, &path, tiny_skia::FillRule::EvenOdd), GOLD_HI, fade);
        }
        sparks(&mut v, 46, VW as f32, VH as f32);
        let keys = cut_keys(v);

        let d = DIAL_SIZE as usize;
        let mut dc = Canvas::new(d, d, BG);
        let dall = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        hatch(&mut dc, 0, 0, d, d, 0.3);
        for (r, w, k) in [(111.0, 1.5, 0.6), (105.0, 5.0, 1.0), (99.0, 1.0, 0.45)] {
            if let Some(path) = crate::canvas::arc(120.5, 120.5, r, w, -90.0, -90.0 + 360.0 * ease(p / 0.6)) {
                dc.paint(&crate::canvas::Cover::from_path(d, d, &path, tiny_skia::FillRule::Winding), mix(BG, GOLD, k * fade.max(0.3)), 1.0);
            }
        }
        let show = ease((p - 0.2) / 0.3) * fade;
        f.draw(&mut dc, FontId::Fell, 16.0, "THE CONTINENT OF", 120.0, 88.0, Anchor::Middle, mix(BG, GOLD_HI, show), dall);
        f.draw(&mut dc, FontId::Cinzel, 44.0, "PYWEL", 120.0, 124.0, Anchor::Middle, mix(BG, CREAM, show), dall);
        sparks(&mut dc, 16, d as f32, d as f32);
        dc.mask_black(self.dial_disc());
        (keys, dc.into_image())
    }
}

impl crate::face::Painter {
    fn intro_emblem(&self, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        use crate::face::deadlock::{BG, OFF_WHITE, SOUL, grain};
        let p = (t / dur).clamp(0.0, 1.0);
        let f = &self.fonts;
        let fade = if p < 0.75 { 1.0 } else { ((1.0 - p) * 4.0).max(0.0) };
        let ease = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        // An old projector: the light dips for a frame now and then.
        let flicker = if rnd(11, (t * 18.0) as u32) > 0.9 { 0.75 } else { 1.0 } * fade;

        let mut v = Canvas::new(VW, VH, BG);
        let all = Clip { x0: 0, y0: 0, x1: VW as i32, y1: VH as i32 };
        let (cx, cy) = (104.0, VH as f32 / 2.0);
        // The outer ring grows in, glowing soul green; the inner emblem follows.
        let grow = ease(p / 0.35);
        if let Some(outer) = crate::art::cover("dl-logo-outer", VW, VH, 130.0 + 60.0 * grow, cx, cy) {
            let mut glow = outer.clone();
            glow.blur(8.0);
            v.add(&glow, mix([0.0; 3], SOUL, 0.55 * grow * flicker));
            v.paint(&outer, mix(SOUL, OFF_WHITE, ease((p - 0.3) / 0.2)), grow * flicker);
        }
        let inner = ease((p - 0.22) / 0.25);
        if let Some(em) = crate::art::cover("dl-logo-inner", VW, VH, 190.0, cx, cy) {
            v.paint(&em, OFF_WHITE, inner * flicker);
        }
        // The name, letter by letter.
        let word = "DEADLOCK";
        let shown = ((ease((p - 0.3) / 0.3) * word.len() as f32).ceil() as usize).min(word.len());
        // From the fourth key column on, so no letter starts in a gap.
        let x0 = 3.0 * (K as f32 + GAP as f32) + 2.0;
        if shown > 0 {
            f.draw(&mut v, FontId::Retail, 29.0, &word[..shown], x0, cy - 6.0, Anchor::Left, mix(BG, OFF_WHITE, flicker), all);
            rect(&mut v, x0, cy + 15.0, 142.0 * shown as f32 / word.len() as f32, 3.0, SOUL, flicker);
        }
        grain(&mut v, 7.0);
        let keys = cut_keys(v);

        let d = DIAL_SIZE as usize;
        let mut dc = Canvas::new(d, d, BG);
        if let Some(path) = crate::canvas::arc(120.5, 120.5, 104.0, 11.0, -90.0, -90.0 + 360.0 * ease(p / 0.6)) {
            dc.paint(&crate::canvas::Cover::from_path(d, d, &path, tiny_skia::FillRule::Winding), mix(BG, SOUL, fade.max(0.3)), 1.0);
        }
        if let Some(em) = crate::art::cover("dl-logo", d, d, 150.0, 120.0, 120.0) {
            dc.paint(&em, OFF_WHITE, inner * flicker);
        }
        grain(&mut dc, 7.0);
        dc.mask_black(self.dial_disc());
        (keys, dc.into_image())
    }
}

impl crate::face::Painter {
    fn intro_hearth(&self, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        use crate::face::wow::{ARCANE, INK, WHITE};
        let p = (t / dur).clamp(0.0, 1.0);
        let f = &self.fonts;
        let fade = if p < 0.75 { 1.0 } else { ((1.0 - p) * 4.0).max(0.0) };
        let ease = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        let build = ease(p / 0.6);
        // The flash peaks at 0.62 and is gone by 0.8.
        let flash = (1.0 - ((p - 0.62) / 0.12).abs()).max(0.0).powi(2);
        let (cx, cy) = (VW as f32 / 2.0, VH as f32 / 2.0);

        let mut v = Canvas::new(VW, VH, INK);
        for y in 0..VH {
            for x in 0..VW {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                let r = (dx * dx + dy * dy).sqrt();
                let a = dy.atan2(dx);
                // Spiral arms winding inward as the cast builds.
                let arm = ((a * 3.0 + r * 0.045 - t * 5.0).sin() * 0.5 + 0.5).powf(3.0);
                let reach = 40.0 + build * 220.0;
                let fall = (1.0 - r / reach).max(0.0);
                let core = (1.0 - r / (18.0 + 40.0 * build)).max(0.0).powi(2);
                let glow = (arm * fall * 0.85 + core * 0.9) * build * fade;
                let d = &mut v.px[y * VW + x];
                *d = mix(*d, mix(ARCANE, WHITE, core * 0.6), glow.min(1.0));
                let fl = flash * (1.0 - r / 320.0).max(0.0);
                *d = mix(*d, WHITE, fl.min(1.0));
            }
        }
        // Runes on a ring, turning, brightening one after another.
        let n = 12;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + t * 0.9;
            let rr = 96.0 - 30.0 * build;
            let (x, y) = (cx + rr * 1.5 * a.cos(), cy + rr * a.sin());
            let on = ease((build * n as f32 - i as f32).clamp(0.0, 1.0));
            if let Some(path) = crate::canvas::circle(x, y, 2.5 + 1.5 * on) {
                let mut cv = crate::canvas::Cover::from_path(VW, VH, &path, tiny_skia::FillRule::Winding);
                v.paint(&cv, mix(ARCANE, WHITE, on * 0.5), (0.3 + 0.7 * on) * fade);
                cv.blur(5.0);
                v.add(&cv, mix([0.0; 3], ARCANE, on * fade));
            }
        }
        // Motes drawn in towards the middle.
        for i in 0..40u32 {
            let life = (t * (0.4 + rnd(i, 1) * 0.4) + rnd(i, 2)).fract();
            let a = rnd(i, 3) * std::f32::consts::TAU + life * 2.0;
            let rr = (1.0 - life) * 230.0;
            let (x, y) = (cx + rr * a.cos(), cy + rr * 0.6 * a.sin());
            if let Some(path) = crate::canvas::circle(x, y, 1.2) {
                v.paint(&crate::canvas::Cover::from_path(VW, VH, &path, tiny_skia::FillRule::Winding), mix(ARCANE, WHITE, 0.5), life * build * fade);
            }
        }
        let keys = cut_keys(v);

        // The dial: the cast bar bent round, the spell's name.
        let d = DIAL_SIZE as usize;
        let mut dc = Canvas::new(d, d, INK);
        let dall = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        let cast = ease(p / 0.62);
        if let Some(path) = crate::canvas::arc(120.5, 120.5, 104.0, 14.0, -90.0, 270.0) {
            dc.paint(&crate::canvas::Cover::from_path(d, d, &path, tiny_skia::FillRule::Winding), [26.0, 22.0, 30.0], 1.0);
        }
        if let Some(path) = crate::canvas::arc(120.5, 120.5, 104.0, 10.0, -90.0, -90.0 + 360.0 * cast) {
            let mut cv = crate::canvas::Cover::from_path(d, d, &path, tiny_skia::FillRule::Winding);
            dc.paint(&cv, mix([255.0, 200.0, 40.0], ARCANE, (p * 1.6).min(1.0)), fade.max(0.2));
            cv.blur(4.0);
            dc.add(&cv, mix([0.0; 3], ARCANE, 0.6 * fade));
        }
        f.draw(&mut dc, FontId::Morpheus, 26.0, "Hearthstone", 120.0, 112.0, Anchor::Middle, mix(INK, WHITE, fade), dall);
        let left = (dur * 0.62 - t).max(0.0);
        let label = if left > 0.0 { format!("{left:.1}") } else { "Home".into() };
        f.draw(&mut dc, FontId::Friz, 15.0, &label, 120.0, 142.0, Anchor::Middle, mix(INK, ARCANE, fade), dall);
        for y in 0..d {
            for x in 0..d {
                let (dx, dy) = (x as f32 - 120.0, y as f32 - 120.0);
                let fl = flash * (1.0 - (dx * dx + dy * dy).sqrt() / 170.0).max(0.0);
                let px = &mut dc.px[y * d + x];
                *px = mix(*px, WHITE, fl);
            }
        }
        dc.mask_black(self.dial_disc());
        (keys, dc.into_image())
    }

    fn intro_ding(&self, t: f32, dur: f32) -> (Vec<RgbImage>, RgbImage) {
        use crate::face::wow::{GOLD, HOLY, INK, WHITE};
        let p = (t / dur).clamp(0.0, 1.0);
        let f = &self.fonts;
        let fade = if p < 0.75 { 1.0 } else { ((1.0 - p) * 4.0).max(0.0) };
        let ease = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        let rise = ease(p / 0.35);
        let (cx, cy) = (VW as f32 / 2.0, VH as f32 / 2.0);
        let mut v = Canvas::new(VW, VH, INK);
        let all = Clip { x0: 0, y0: 0, x1: VW as i32, y1: VH as i32 };
        // The pillar: rises from the bottom, widens, shimmers.
        let top = VH as f32 * (1.0 - rise * 1.1);
        let width = 18.0 + 70.0 * ease((p - 0.15) / 0.3);
        for y in 0..VH {
            if (y as f32) < top {
                continue;
            }
            for x in 0..VW {
                let dx = (x as f32 - cx).abs();
                let shimmer = 0.85 + 0.15 * ((y as f32 * 0.15 - t * 9.0).sin());
                let core = (1.0 - dx / width).max(0.0).powf(1.6) * shimmer;
                let halo = (1.0 - dx / (width * 2.6)).max(0.0).powi(3) * 0.5;
                let head = (1.0 - (y as f32 - top) / 30.0).max(0.0) * 0.6;
                let a = ((core + halo + head * core) * fade).min(1.0);
                let d = &mut v.px[y * VW + x];
                *d = mix(*d, mix(HOLY, WHITE, core * 0.7), a);
            }
        }
        // Rays fanning out once the pillar stands.
        let fan = ease((p - 0.3) / 0.25) * fade;
        if fan > 0.0 {
            for y in 0..VH {
                for x in 0..VW {
                    let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                    let a = dy.atan2(dx) + t * 0.25;
                    let ray = ((a * 9.0).cos().max(0.0)).powf(14.0);
                    let r = (dx * dx + dy * dy).sqrt();
                    let k = ray * (1.0 - r / 260.0).max(0.0) * fan * 0.5;
                    let d = &mut v.px[y * VW + x];
                    for i in 0..3 {
                        d[i] = (d[i] + HOLY[i] * k).min(255.0);
                    }
                }
            }
        }
        // Gold dust rising.
        for i in 0..60u32 {
            let life = (t * (0.3 + rnd(i, 1) * 0.4) + rnd(i, 2)).fract();
            let x = cx + (rnd(i, 3) - 0.5) * width * 3.0 + (t * 3.0 + i as f32).sin() * 4.0;
            let y = VH as f32 - life * VH as f32 * 1.1;
            if let Some(path) = crate::canvas::circle(x, y, 0.8 + rnd(i, 4) * 1.4) {
                v.paint(&crate::canvas::Cover::from_path(VW, VH, &path, tiny_skia::FillRule::Winding), mix(GOLD, WHITE, rnd(i, 5)), (1.0 - life) * rise * fade);
            }
        }
        // LEVEL across the middle row, one letter per key (no letter sits in a gap), one by one.
        for (i, ch) in "LEVEL".chars().enumerate() {
            let show = ease((p - 0.38 - i as f32 * 0.05) / 0.12) * fade;
            if show <= 0.0 {
                continue;
            }
            let x = (i as f32 * (K as f32 + GAP as f32)) + K as f32 / 2.0;
            let s = ch.to_string();
            f.draw(&mut v, FontId::Morpheus, 50.0, &s, x, cy + 2.0, Anchor::Middle, INK, all);
            f.draw(&mut v, FontId::Morpheus, 50.0, &s, x, cy, Anchor::Middle, mix(INK, GOLD, show), all);
        }
        let keys = cut_keys(v);

        let d = DIAL_SIZE as usize;
        let mut dc = Canvas::new(d, d, INK);
        let dall = Clip { x0: 0, y0: 0, x1: d as i32, y1: d as i32 };
        for y in 0..d {
            for x in 0..d {
                let (dx, dy) = ((x as f32 - 120.0), (y as f32 - 120.0));
                let r = (dx * dx + dy * dy).sqrt();
                let a = dy.atan2(dx) - t * 0.4;
                let ray = ((a * 7.0).cos().max(0.0)).powf(8.0) * (1.0 - r / 140.0).max(0.0);
                let core = (1.0 - r / (30.0 + 80.0 * rise)).max(0.0).powi(2);
                let px = &mut dc.px[y * d + x];
                *px = mix(*px, HOLY, ((ray * 0.6 + core * 0.7) * rise * fade).min(1.0));
            }
        }
        let show = ease((p - 0.62) / 0.12) * fade;
        if show > 0.0 {
            f.draw(&mut dc, FontId::Friz, 72.0, "60", 120.0, 112.0, Anchor::Middle, mix(INK, WHITE, show), dall);
            f.draw(&mut dc, FontId::Morpheus, 22.0, "Forever", 120.0, 164.0, Anchor::Middle, mix(INK, GOLD, show), dall);
        }
        dc.mask_black(self.dial_disc());
        (keys, dc.into_image())
    }
}

/// Cut the virtual keypad picture into the 15 key images (row-major).
fn cut_keys(v: Canvas) -> Vec<RgbImage> {
    let img = v.into_image();
    (0..15u32)
        .map(|i| {
            let (c, r) = (i % 5, i / 5);
            image::imageops::crop_imm(&img, c * (KEY_SIZE + GAP), r * (KEY_SIZE + GAP), KEY_SIZE, KEY_SIZE).to_image()
        })
        .collect()
}

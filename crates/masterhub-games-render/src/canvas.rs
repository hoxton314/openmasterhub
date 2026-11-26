//! A tiny float RGB canvas with the few operations the faces need: coverage blending, additive
//! glows, a Pillow-compatible Gaussian blur, and shape coverage masks from tiny-skia.

use image::RgbImage;
use tiny_skia::{FillRule, LineCap, Mask, Path, PathBuilder, Stroke, Transform};

pub type Rgb = [f32; 3];

pub const fn rgb(hex: u32) -> Rgb {
    [((hex >> 16) & 0xff) as f32, ((hex >> 8) & 0xff) as f32, (hex & 0xff) as f32]
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Integer clip rectangle, `x0..x1` × `y0..y1`.
#[derive(Clone, Copy, Debug)]
pub struct Clip {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// A single-channel coverage buffer, 0.0..=1.0.
#[derive(Clone)]
pub struct Cover {
    pub w: usize,
    pub h: usize,
    pub v: Vec<f32>,
}

impl Cover {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, v: vec![0.0; w * h] }
    }

    /// Coverage of a filled path (anti-aliased).
    pub fn from_path(w: usize, h: usize, path: &Path, rule: FillRule) -> Self {
        let mut c = Self::new(w, h);
        c.fill_path(path, rule, 1.0);
        c
    }

    /// Add the coverage of a path, scaled by `k`, clamped to 1.
    pub fn fill_path(&mut self, path: &Path, rule: FillRule, k: f32) {
        let Some(mut m) = Mask::new(self.w as u32, self.h as u32) else { return };
        m.fill_path(path, rule, true, Transform::identity());
        for (d, &s) in self.v.iter_mut().zip(m.data()) {
            *d = (*d + s as f32 / 255.0 * k).min(1.0);
        }
    }

    pub fn scale(&mut self, k: f32) {
        self.v.iter_mut().for_each(|x| *x *= k);
    }

    /// Pillow's `GaussianBlur(radius)`: three extended box passes per axis, edges repeated.
    pub fn blur(&mut self, sigma: f32) {
        let r = box_radius(sigma, 3);
        let mut tmp = vec![0.0f32; self.w.max(self.h)];
        let mut out = vec![0.0f32; self.w.max(self.h)];
        for _ in 0..3 {
            for y in 0..self.h {
                let row = &mut self.v[y * self.w..(y + 1) * self.w];
                tmp[..self.w].copy_from_slice(row);
                box_line(&tmp[..self.w], &mut out[..self.w], r);
                row.copy_from_slice(&out[..self.w]);
            }
        }
        for _ in 0..3 {
            for x in 0..self.w {
                for (y, t) in tmp[..self.h].iter_mut().enumerate() {
                    *t = self.v[y * self.w + x];
                }
                box_line(&tmp[..self.h], &mut out[..self.h], r);
                for (y, &o) in out[..self.h].iter().enumerate() {
                    self.v[y * self.w + x] = o;
                }
            }
        }
    }
}

/// Pillow's `_gaussian_blur_radius`.
fn box_radius(sigma: f32, passes: u32) -> f32 {
    let s2 = sigma * sigma / passes as f32;
    let l = ((12.0 * s2 + 1.0).sqrt() - 1.0) / 2.0;
    let l = l.floor();
    let a = (2.0 * l + 1.0) * (l * (l + 1.0) - 3.0 * s2) / (6.0 * (s2 - (l + 1.0) * (l + 1.0)));
    l + a
}

/// One extended box pass: integer core of `2l+1` taps plus fractional edge taps.
fn box_line(src: &[f32], dst: &mut [f32], r: f32) {
    let n = src.len() as i32;
    let l = r.floor() as i32;
    let a = r - l as f32;
    let norm = 1.0 / (2.0 * r + 1.0);
    let at = |i: i32| src[i.clamp(0, n - 1) as usize];
    let mut sum: f32 = (-l..=l).map(at).sum();
    for i in 0..n {
        dst[i as usize] = (sum + a * (at(i - l - 1) + at(i + l + 1))) * norm;
        sum += at(i + l + 1) - at(i - l);
    }
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, fill: Rgb) -> Self {
        Self { w, h, px: vec![fill; w * h] }
    }

    /// Paint `color` through `cover` (scaled by `alpha`).
    pub fn paint(&mut self, cover: &Cover, color: Rgb, alpha: f32) {
        for (d, &c) in self.px.iter_mut().zip(&cover.v) {
            let a = c * alpha;
            if a > 0.0 {
                *d = mix(*d, color, a.min(1.0));
            }
        }
    }

    /// Saturating add of `color * cover` (Pillow's `ImageChops.add` of a glow layer).
    pub fn add(&mut self, cover: &Cover, color: Rgb) {
        for (d, &c) in self.px.iter_mut().zip(&cover.v) {
            if c > 0.0 {
                for k in 0..3 {
                    d[k] = (d[k] + color[k] * c).min(255.0);
                }
            }
        }
    }

    /// Paint a small coverage bitmap whose top left is at (`x`, `y`), clipped.
    pub fn paint_at(&mut self, cov: &[f32], bw: usize, x: i32, y: i32, color: Rgb, clip: Clip) {
        if bw == 0 {
            return;
        }
        let bh = cov.len() / bw;
        let cx0 = clip.x0.max(0).max(x);
        let cy0 = clip.y0.max(0).max(y);
        let cx1 = clip.x1.min(self.w as i32).min(x + bw as i32);
        let cy1 = clip.y1.min(self.h as i32).min(y + bh as i32);
        for py in cy0..cy1 {
            let srow = ((py - y) as usize) * bw;
            let drow = py as usize * self.w;
            for px in cx0..cx1 {
                let a = cov[srow + (px - x) as usize];
                if a > 0.0 {
                    let d = &mut self.px[drow + px as usize];
                    *d = mix(*d, color, a);
                }
            }
        }
    }

    /// Keep what is under `cover`, black elsewhere.
    pub fn mask_black(&mut self, cover: &Cover) {
        for (d, &c) in self.px.iter_mut().zip(&cover.v) {
            for k in d.iter_mut() {
                *k *= c;
            }
        }
    }

    pub fn into_image(self) -> RgbImage {
        let mut buf = Vec::with_capacity(self.w * self.h * 3);
        for p in &self.px {
            for &k in p {
                buf.push((k + 0.5).clamp(0.0, 255.0) as u8);
            }
        }
        RgbImage::from_raw(self.w as u32, self.h as u32, buf).expect("canvas size")
    }
}

/// A filled circle.
pub fn circle(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r)
}

/// An annulus between `r_in` and `r_out` (fill with EvenOdd).
pub fn annulus(cx: f32, cy: f32, r_in: f32, r_out: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r_out);
    pb.push_circle(cx, cy, r_in);
    pb.finish()
}

/// A stroked arc (flat, radial ends) with angles in degrees, 0 = 3 o'clock, clockwise on
/// screen — Pillow's `ImageDraw.arc` convention. `r` is the centre line.
pub fn arc(cx: f32, cy: f32, r: f32, width: f32, a0: f32, a1: f32) -> Option<Path> {
    if a1 <= a0 {
        return None;
    }
    let mut pb = PathBuilder::new();
    let segs = ((a1 - a0) / 90.0).ceil().max(1.0) as usize;
    let step = (a1 - a0) / segs as f32;
    let pt = |a: f32| (cx + r * a.to_radians().cos(), cy + r * a.to_radians().sin());
    let (x, y) = pt(a0);
    pb.move_to(x, y);
    for i in 0..segs {
        let t0 = (a0 + step * i as f32).to_radians();
        let t1 = (a0 + step * (i + 1) as f32).to_radians();
        let k = 4.0 / 3.0 * ((t1 - t0) / 4.0).tan() * r;
        let (x0, y0) = (cx + r * t0.cos(), cy + r * t0.sin());
        let (x3, y3) = (cx + r * t1.cos(), cy + r * t1.sin());
        pb.cubic_to(
            x0 - k * t0.sin(),
            y0 + k * t0.cos(),
            x3 + k * t1.sin(),
            y3 - k * t1.cos(),
            x3,
            y3,
        );
    }
    let path = pb.finish()?;
    let stroke = Stroke { width, line_cap: LineCap::Butt, ..Stroke::default() };
    path.stroke(&stroke, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_radius_matches_pillow() {
        assert!((box_radius(5.0, 3) - 4.45).abs() < 1e-3);
        assert!((box_radius(3.0, 3) - 2.4167).abs() < 1e-3);
    }

    #[test]
    fn blur_keeps_mass() {
        let mut c = Cover::new(40, 40);
        c.v[20 * 40 + 20] = 1.0;
        c.blur(3.0);
        let s: f32 = c.v.iter().sum();
        assert!((s - 1.0).abs() < 1e-3, "{s}");
    }
}

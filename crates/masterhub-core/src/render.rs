//! Renders key and dial faces to the baseline JPEGs the modules decode.

use std::f32::consts::{PI, TAU};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, Rgb, RgbImage, RgbaImage};
use masterhub_proto::Rgb as Color;

use crate::config::Face;

/// Size of a keypad key image.
pub const KEY_SIZE: u32 = 72;
/// Size of the dial image.
pub const DIAL_SIZE: u32 = 240;

/// Live data for a widget, produced by the service.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveData {
    /// A 0..=1 value drawn as a ring, e.g. CPU load.
    Gauge { fraction: f32, text: String, caption: String },
    /// Plain text, e.g. a clock.
    Text { text: String, caption: Option<String> },
    /// Now playing.
    Media { title: String, artist: String, art: Option<RgbaImage> },
}

/// A decoded image or animation from a face's `image` path.
pub enum Picture {
    Still(RgbaImage),
    Animated(Vec<(RgbaImage, Duration)>),
}

/// Load a picture; GIFs with more than one frame become animations.
pub fn load_picture(path: &Path, size: u32) -> Result<Picture> {
    let is_gif = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"));
    if is_gif {
        let file = std::io::BufReader::new(std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?);
        let frames = image::codecs::gif::GifDecoder::new(file)?.into_frames().collect_frames()?;
        if frames.len() > 1 {
            return Ok(Picture::Animated(
                frames
                    .into_iter()
                    .map(|f| {
                        let (num, den) = f.delay().numer_denom_ms();
                        // Browsers treat delays under 20 ms as 100 ms; do the same.
                        let ms = if den == 0 { 100 } else { (num / den).max(20) };
                        let ms = if ms <= 20 { 100 } else { ms };
                        (fit(DynamicImage::ImageRgba8(f.into_buffer()), size), Duration::from_millis(ms as u64))
                    })
                    .collect(),
            ));
        }
    }
    let img = image::open(path).with_context(|| format!("loading {}", path.display()))?;
    Ok(Picture::Still(fit(img, size)))
}

fn fit(img: DynamicImage, size: u32) -> RgbaImage {
    img.resize(size, size, FilterType::Lanczos3).to_rgba8()
}

pub struct Renderer {
    font: Option<FontVec>,
}

impl Renderer {
    pub fn new(font: Option<&Path>) -> Self {
        let path = font.map(Path::to_owned).or_else(find_default_font);
        let font = path.as_deref().and_then(|p| match std::fs::read(p).ok().and_then(|d| FontVec::try_from_vec(d).ok()) {
            Some(f) => Some(f),
            None => {
                log::warn!("could not load font {}; labels disabled", p.display());
                None
            }
        });
        if path.is_none() {
            log::warn!("no font found; labels disabled (set `font` in the config)");
        }
        Self { font }
    }

    /// Render a face to RGB pixels. `picture` is the (current frame of the) face's image.
    pub fn render_rgb(&self, face: &Face, size: u32, picture: Option<&RgbaImage>, live: Option<&LiveData>) -> RgbImage {
        let bg = face.background.map(|c| c.0).unwrap_or(Color::new(0, 0, 0));
        let fg = face.text_color.map(|c| c.0).unwrap_or(Color::new(255, 255, 255));
        let mut img = RgbImage::from_pixel(size, size, Rgb([bg.r, bg.g, bg.b]));
        let s = size as f32;

        match live {
            Some(LiveData::Gauge { fraction, text, caption }) => {
                let accent = face.background.map(|_| fg).unwrap_or(gauge_color(*fraction));
                draw_ring(&mut img, *fraction, accent);
                self.text(&mut img, text, s * 0.47, s * 0.26, s * 0.62, fg);
                let caption = face.label.as_deref().unwrap_or(caption);
                self.text(&mut img, caption, s * 0.70, s * 0.12, s * 0.5, dim(fg));
            }
            Some(LiveData::Text { text, caption }) => {
                let caption = face.label.as_deref().or(caption.as_deref());
                let y = if caption.is_some() { s * 0.42 } else { s * 0.5 };
                self.text(&mut img, text, y, s * 0.34, s * 0.9, fg);
                if let Some(c) = caption {
                    self.text(&mut img, c, s * 0.75, s * 0.15, s * 0.85, dim(fg));
                }
            }
            Some(LiveData::Media { title, artist, art }) => {
                if let Some(art) = art {
                    let cover = image::imageops::resize(art, size, size, FilterType::Triangle);
                    blend_over(&mut img, &cover, 0, 0, 1.0);
                    darken_bottom(&mut img);
                }
                self.text(&mut img, title, s * 0.72, s * 0.15, s * 0.9, fg);
                self.text(&mut img, artist, s * 0.87, s * 0.11, s * 0.9, dim(fg));
            }
            None => {
                if let Some(pic) = picture {
                    let (x, y) = ((size.saturating_sub(pic.width())) / 2, (size.saturating_sub(pic.height())) / 2);
                    if face.label.is_some() {
                        // Shrink the picture to leave room for the label underneath.
                        let small = image::imageops::resize(pic, size * 2 / 3, size * 2 / 3, FilterType::Triangle);
                        blend_over(&mut img, &small, (size - small.width()) / 2, size / 12, 1.0);
                    } else {
                        blend_over(&mut img, pic, x, y, 1.0);
                    }
                }
                if let Some(label) = &face.label {
                    let y = if picture.is_some() { s * 0.86 } else { s * 0.5 };
                    let px = if picture.is_some() { s * 0.2 } else { s * 0.28 };
                    self.text(&mut img, label, y, px, s * 0.9, fg);
                }
            }
        }
        img
    }

    /// Render and encode as the baseline JFIF JPEG the modules expect, turned clockwise by
    /// `turn` degrees (0/90/180/270) so it reads upright on a rotated module.
    pub fn render(&self, face: &Face, size: u32, picture: Option<&RgbaImage>, live: Option<&LiveData>, turn: u16) -> Result<Vec<u8>> {
        let img = self.render_rgb(face, size, picture, live);
        let img = match turn % 360 {
            90 => image::imageops::rotate90(&img),
            180 => image::imageops::rotate180(&img),
            270 => image::imageops::rotate270(&img),
            _ => img,
        };
        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, 85).encode_image(&img)?;
        Ok(out)
    }

    /// Draw `text` centred at `center_y`, starting at `px` and shrinking to fit `max_width`.
    fn text(&self, img: &mut RgbImage, text: &str, center_y: f32, px: f32, max_width: f32, fg: Color) {
        let Some(font) = &self.font else { return };
        let width = img.width() as f32;
        let measure = |px: f32| {
            let f = font.as_scaled(PxScale::from(px));
            text.chars().map(|c| f.h_advance(f.glyph_id(c))).sum::<f32>()
        };
        let mut px = px;
        while measure(px) > max_width && px > 7.0 {
            px -= 1.0;
        }
        let scaled = font.as_scaled(PxScale::from(px));
        let baseline = center_y + (scaled.ascent() + scaled.descent()) / 2.0;
        let mut x = (width - measure(px)) / 2.0;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            let glyph = id.with_scale_and_position(px, ab_glyph::point(x, baseline));
            x += scaled.h_advance(id);
            let Some(outline) = font.outline_glyph(glyph) else { continue };
            let b = outline.px_bounds();
            outline.draw(|gx, gy, cov| {
                let (ix, iy) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                if ix < 0 || iy < 0 || ix as u32 >= img.width() || iy as u32 >= img.height() {
                    return;
                }
                let d = img.get_pixel_mut(ix as u32, iy as u32);
                let a = cov.clamp(0.0, 1.0);
                for (ch, v) in [fg.r, fg.g, fg.b].into_iter().enumerate() {
                    d[ch] = (v as f32 * a + d[ch] as f32 * (1.0 - a)) as u8;
                }
            });
        }
    }
}

fn dim(c: Color) -> Color {
    Color::new((c.r as u16 * 7 / 10) as u8, (c.g as u16 * 7 / 10) as u8, (c.b as u16 * 7 / 10) as u8)
}

/// Green → amber → red as the value rises.
fn gauge_color(f: f32) -> Color {
    match f {
        f if f < 0.6 => Color::new(64, 200, 120),
        f if f < 0.85 => Color::new(240, 170, 40),
        _ => Color::new(235, 70, 60),
    }
}

/// A 270° ring gauge open at the bottom, anti-aliased.
fn draw_ring(img: &mut RgbImage, fraction: f32, accent: Color) {
    let size = img.width() as f32;
    let (cx, cy) = (size / 2.0, size / 2.0);
    let outer = size * 0.46;
    let thickness = (size * 0.075).max(4.0);
    let inner = outer - thickness;
    let start = 0.75 * PI; // bottom-left, clockwise to bottom-right
    let sweep = 1.5 * PI;
    let filled = sweep * fraction.clamp(0.0, 1.0);
    let track = Color::new(60, 60, 66);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let r = (dx * dx + dy * dy).sqrt();
            let cov = ((r - inner + 0.5).clamp(0.0, 1.0)) * ((outer - r + 0.5).clamp(0.0, 1.0));
            if cov <= 0.0 {
                continue;
            }
            let a = (dy.atan2(dx) - start).rem_euclid(TAU);
            if a > sweep {
                continue;
            }
            let c = if a < filled { accent } else { track };
            let d = img.get_pixel_mut(x, y);
            for (ch, v) in [c.r, c.g, c.b].into_iter().enumerate() {
                d[ch] = (v as f32 * cov + d[ch] as f32 * (1.0 - cov)) as u8;
            }
        }
    }
}

fn darken_bottom(img: &mut RgbImage) {
    let h = img.height() as f32;
    for (_, y, p) in img.enumerate_pixels_mut() {
        let t = ((y as f32 / h - 0.45) / 0.55).clamp(0.0, 1.0) * 0.8;
        for c in p.0.iter_mut() {
            *c = (*c as f32 * (1.0 - t)) as u8;
        }
    }
}

fn blend_over(dst: &mut RgbImage, src: &RgbaImage, x0: u32, y0: u32, opacity: f32) {
    for (x, y, p) in src.enumerate_pixels() {
        let (dx, dy) = (x0 + x, y0 + y);
        if dx >= dst.width() || dy >= dst.height() {
            continue;
        }
        let a = p[3] as f32 / 255.0 * opacity;
        let d = dst.get_pixel_mut(dx, dy);
        for c in 0..3 {
            d[c] = (p[c] as f32 * a + d[c] as f32 * (1.0 - a)) as u8;
        }
    }
}

fn find_default_font() -> Option<PathBuf> {
    let out = std::process::Command::new("fc-match").args(["-f", "%{file}", "sans:bold"]).output().ok()?;
    let p = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    p.exists().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_baseline_jpeg_of_right_size() {
        let r = Renderer { font: None };
        let face = Face { label: Some("x".into()), ..Default::default() };
        let jpeg = r.render(&face, KEY_SIZE, None, None, 90).unwrap();
        // JFIF header the firmware checks for.
        assert_eq!(&jpeg[..4], &[0xFF, 0xD8, 0xFF, 0xE0]);
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (72, 72));
    }

    #[test]
    fn gauge_fills_proportionally() {
        let r = Renderer { font: None };
        let lit = |f: f32| {
            let img = r.render_rgb(
                &Face::default(),
                DIAL_SIZE,
                None,
                Some(&LiveData::Gauge { fraction: f, text: String::new(), caption: String::new() }),
            );
            img.pixels().filter(|p| p[1] > 150).count()
        };
        assert_eq!(lit(0.0), 0);
        assert!(lit(0.5) > 0 && lit(0.5) < lit(0.55) * 2);
    }
}

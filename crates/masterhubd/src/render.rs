//! Renders key and dial faces to the baseline JPEGs the modules decode.

use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{Rgb, RgbImage};
use masterhub_proto::Rgb as Color;

use crate::config::Face;

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
        if font.is_none() && path.is_none() {
            log::warn!("no font found; labels disabled (set `font` in the config)");
        }
        Self { font }
    }

    /// Render a square face of `size`×`size` pixels and encode it as JPEG.
    pub fn render(&self, face: &Face, size: u32) -> Result<Vec<u8>> {
        let bg = face.background.map(|c| c.0).unwrap_or(Color::new(0, 0, 0));
        let mut img = RgbImage::from_pixel(size, size, Rgb([bg.r, bg.g, bg.b]));

        if let Some(path) = &face.image {
            let pic = image::open(path).with_context(|| format!("loading {}", path.display()))?;
            // Leave room for the label under the picture.
            let box_size = if face.label.is_some() { size * 2 / 3 } else { size };
            let pic = pic.resize(box_size, box_size, FilterType::Lanczos3).to_rgba8();
            let x = (size - pic.width()) / 2;
            let y = if face.label.is_some() { size / 12 } else { (size - pic.height()) / 2 };
            blend_over(&mut img, &pic, x, y);
        }

        if let (Some(text), Some(font)) = (&face.label, &self.font) {
            let fg = face.text_color.map(|c| c.0).unwrap_or(Color::new(255, 255, 255));
            let center_y = if face.image.is_some() { size as f32 * 0.85 } else { size as f32 / 2.0 };
            draw_label(&mut img, font, text, center_y, fg);
        }

        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, 85).encode_image(&img)?;
        Ok(out)
    }
}

fn blend_over(dst: &mut RgbImage, src: &image::RgbaImage, x0: u32, y0: u32) {
    for (x, y, p) in src.enumerate_pixels() {
        let (dx, dy) = (x0 + x, y0 + y);
        if dx >= dst.width() || dy >= dst.height() {
            continue;
        }
        let a = p[3] as u32;
        let d = dst.get_pixel_mut(dx, dy);
        for c in 0..3 {
            d[c] = ((p[c] as u32 * a + d[c] as u32 * (255 - a)) / 255) as u8;
        }
    }
}

/// Draw `text` centred horizontally at `center_y`, shrinking it to fit the width.
fn draw_label(img: &mut RgbImage, font: &FontVec, text: &str, center_y: f32, fg: Color) {
    let width = img.width() as f32;
    let mut px = img.height() as f32 * 0.28;
    let layout = |px: f32| {
        let f = font.as_scaled(PxScale::from(px));
        let w: f32 = text.chars().map(|c| f.h_advance(f.glyph_id(c))).sum();
        (f, w)
    };
    let (mut scaled, mut w) = layout(px);
    while w > width * 0.9 && px > 8.0 {
        px -= 1.0;
        (scaled, w) = layout(px);
    }
    let baseline = center_y + (scaled.ascent() + scaled.descent()) / 2.0;
    let mut x = (width - w) / 2.0;
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
        let face = Face { label: Some("x".into()), background: None, image: None, text_color: None };
        let jpeg = r.render(&face, 72).unwrap();
        // JFIF header the firmware checks for.
        assert_eq!(&jpeg[..4], &[0xFF, 0xD8, 0xFF, 0xE0]);
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (72, 72));
    }
}

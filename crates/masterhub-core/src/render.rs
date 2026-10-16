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
use crate::ipc::Status;

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
    /// A Claude Code session: its name and the mascot in the pose of its status, on a
    /// background tinted with the status `color`. `frame` advances the working animation and
    /// flashes the attention background (odd = dim).
    Session { name: String, status: Status, color: Color, frame: u32 },
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
                        let ms = num.checked_div(den).filter(|&ms| ms > 20).unwrap_or(100);
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
            Some(LiveData::Session { name, status, color, frame }) => {
                let tint = match status {
                    Status::Attention if frame % 2 == 1 => 0.2,
                    Status::Attention => 0.5,
                    _ => 0.4,
                };
                let back = mix(bg, *color, tint);
                img = RgbImage::from_pixel(size, size, Rgb([back.r, back.g, back.b]));
                draw_mascot(&mut img, mascot(*status, *frame), s * 0.05, s * 0.6);
                self.two_lines(&mut img, name, s * 0.83, s * 0.15, s * 0.92, fg);
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

    /// Width of `text` at `px`.
    fn measure(&self, text: &str, px: f32) -> f32 {
        let Some(font) = &self.font else { return 0.0 };
        let f = font.as_scaled(PxScale::from(px));
        text.chars().map(|c| f.h_advance(f.glyph_id(c))).sum()
    }

    /// Draw `text` word-wrapped into at most two lines of `px` centred around `center_y`; a
    /// single line may shrink a little first, the second line ends in `…` if cut.
    fn two_lines(&self, img: &mut RgbImage, text: &str, center_y: f32, px: f32, max_width: f32, fg: Color) {
        let text = text.trim();
        if self.font.is_none() || text.is_empty() {
            return;
        }
        if self.measure(text, px * 0.8) <= max_width {
            self.text(img, text, center_y - px * 0.1, px, max_width, fg);
            return;
        }
        let lines = wrap(text, 2, |t| self.measure(t, px) <= max_width);
        let step = px * 1.05;
        let top = center_y - step * (lines.len() as f32 - 1.0) / 2.0;
        for (i, line) in lines.iter().enumerate() {
            self.text(img, line, top + step * i as f32, px, max_width, fg);
        }
    }

    /// Draw `text` centred at `center_y`, starting at `px` and shrinking to fit `max_width`.
    fn text(&self, img: &mut RgbImage, text: &str, center_y: f32, px: f32, max_width: f32, fg: Color) {
        let Some(font) = &self.font else { return };
        let width = img.width() as f32;
        let measure = |px: f32| self.measure(text, px);
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

/// Split `text` into at most `max_lines` lines that `fits`, breaking at spaces (or inside a
/// word too long for a line); the last line ends in `…` when text is left over.
fn wrap(text: &str, max_lines: usize, fits: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut rest: Vec<char> = text.chars().collect();
    while !rest.is_empty() && lines.len() < max_lines {
        let last = lines.len() + 1 == max_lines;
        let all: String = rest.iter().collect();
        if fits(&all) {
            lines.push(all);
            rest.clear();
            break;
        }
        // Longest prefix that fits, with room for the ellipsis on the last line.
        let mut n = rest.len();
        while n > 1 {
            let mut cand: String = rest[..n].iter().collect();
            if last {
                cand = format!("{}…", cand.trim_end());
            }
            if fits(&cand) {
                break;
            }
            n -= 1;
        }
        // Prefer breaking after a space unless that wastes most of the line.
        if !last && let Some(sp) = rest[..n].iter().rposition(|c| *c == ' ').filter(|&sp| sp * 2 >= n && n < rest.len()) {
            n = sp + 1;
        }
        let line: String = rest[..n].iter().collect::<String>().trim().to_owned();
        rest.drain(..n);
        while rest.first() == Some(&' ') {
            rest.remove(0);
        }
        lines.push(if last && !rest.is_empty() { format!("{line}…") } else { line });
    }
    lines
}

/// Claude Code mascot poses, 14×11 cells: `#` body, `o` eye.
const MASCOT_READY: [&str; 11] = [
    "..............",
    "...########...",
    "..##########..",
    "..##o####o##..",
    "..##o####o##..",
    "..##########..",
    ".############.",
    "#.##########.#",
    "#.##########.#",
    "...##....##...",
    "...##....##...",
];
/// Working, two frames: eyes down on the work, arms typing.
const MASCOT_WORKING: [[&str; 11]; 2] = [
    [
        "..............",
        "...########...",
        "..##########..",
        "..##########..",
        "..##oo##oo##..",
        "..##########..",
        "..##########..",
        "##############",
        "..##########..",
        "...##....##...",
        "...##....##...",
    ],
    [
        "..............",
        "...########...",
        "..##########..",
        "..##########..",
        "..##oo##oo##..",
        "..##########..",
        "##############",
        "..##########..",
        "..##########..",
        "...##....##...",
        "...##....##...",
    ],
];
/// Waiting for you: waving, eyes wide.
const MASCOT_ATTENTION: [&str; 11] = [
    ".............#",
    "...########..#",
    "..##########.#",
    "..##oo##oo####",
    "..##oo##oo##..",
    "..##########..",
    ".###########..",
    "#.##########..",
    "#.##########..",
    "...##....##...",
    "...##....##...",
];

fn mascot(status: Status, frame: u32) -> &'static [&'static str; 11] {
    match status {
        Status::Ready => &MASCOT_READY,
        Status::Working => &MASCOT_WORKING[frame as usize % 2],
        Status::Attention => &MASCOT_ATTENTION,
    }
}

/// Claude's terracotta, the mascot's own colour.
const MASCOT_BODY: Color = Color::new(0xd9, 0x77, 0x57);
const MASCOT_EYES: Color = Color::new(0x14, 0x14, 0x14);

/// Draw a sprite centred horizontally, `top` from the top edge and at most `height` tall.
fn draw_mascot(img: &mut RgbImage, sprite: &[&str; 11], top: f32, height: f32) {
    let size = img.width();
    let cell = ((height / sprite.len() as f32) as u32).max(1);
    let w = sprite[0].len() as u32 * cell;
    let (x0, y0) = ((size.saturating_sub(w)) / 2, top as u32);
    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let c = match ch {
                '#' => MASCOT_BODY,
                'o' => MASCOT_EYES,
                _ => continue,
            };
            for dy in 0..cell {
                for dx in 0..cell {
                    let (x, y) = (x0 + col as u32 * cell + dx, y0 + row as u32 * cell + dy);
                    if x < size && y < img.height() {
                        img.put_pixel(x, y, Rgb([c.r, c.g, c.b]));
                    }
                }
            }
        }
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color::new(m(a.r, b.r), m(a.g, b.g), m(a.b, b.b))
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

#[cfg(windows)]
fn find_default_font() -> Option<PathBuf> {
    let fonts = PathBuf::from(std::env::var_os("WINDIR")?).join("Fonts");
    ["segoeuib.ttf", "arialbd.ttf", "segoeui.ttf"].into_iter().map(|f| fonts.join(f)).find(|p| p.exists())
}

#[cfg(not(windows))]
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

    #[test]
    fn wraps_long_names() {
        let fits = |t: &str| t.chars().count() <= 10;
        assert_eq!(wrap("openmasterhub", 2, fits), ["openmaster", "hub"]);
        assert_eq!(wrap("openmasterhub-protocol", 2, fits), ["openmaster", "hub-proto…"]);
        assert_eq!(wrap("short", 2, fits), ["short"]);
        assert_eq!(wrap("Wire the CLI sandbox into", 2, fits), ["Wire the", "CLI sandb…"]);
        assert_eq!(wrap("Take a look at", 2, fits), ["Take a", "look at"]);
    }

    #[test]
    fn session_background_takes_status_colour() {
        let r = Renderer { font: None };
        let color = Color::new(0, 200, 100);
        for status in [Status::Working, Status::Ready, Status::Attention] {
            let live = LiveData::Session { name: "x".into(), status, color, frame: 0 };
            let img = r.render_rgb(&Face::default(), KEY_SIZE, None, Some(&live));
            let back = mix(Color::new(0, 0, 0), color, if status == Status::Attention { 0.5 } else { 0.4 });
            assert_eq!(img.get_pixel(0, KEY_SIZE - 1).0, [back.r, back.g, back.b], "{status:?}");
            let body = MASCOT_BODY;
            assert!(img.pixels().filter(|p| p.0 == [body.r, body.g, body.b]).count() > 300, "{status:?}");
        }
    }
}

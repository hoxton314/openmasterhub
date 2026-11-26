//! Embedded fonts, text measuring and cached text rasters (ab_glyph).
//!
//! Positions follow Pillow's `ImageDraw.text` with anchor `"mm"` / `"lm"`: horizontally the
//! advance width, vertically the middle between the font's ascender and descender.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ab_glyph::{Font, FontRef, GlyphId, PxScale, PxScaleFont, ScaleFont, point};

use crate::canvas::{Canvas, Clip, Cover, Rgb};
use crate::kern::Kerning;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontId {
    Chakra = 0,
    Mono = 1,
    Bebas = 2,
    Icons = 3,
    /// IM Fell English SC: an old, slightly rough book face.
    Fell = 4,
    /// Cinzel Bold: Roman capitals.
    Cinzel = 5,
    /// Cormorant SC Bold: a sharp display serif.
    Cormorant = 6,
    /// Retail Demo Bold, Deadlock's own display face, read from the game's install when the
    /// renderer starts (it is not ours to bundle); Chakra Petch stands in when the game is gone.
    Retail = 7,
    /// Friz Quadrata, WoW's interface face, read from the extracted game files
    /// (`~/.local/share/pulpit/wow-src/fonts`); Cinzel stands in without them.
    Friz = 8,
    /// Morpheus, WoW's title face (quest titles, zone names); IM Fell stands in.
    Morpheus = 9,
}

const N: usize = 10;

const BUILTIN: [&[u8]; 7] = [
    include_bytes!("../fonts/ChakraPetch-Bold.ttf"),
    include_bytes!("../fonts/ShareTechMono-Regular.ttf"),
    include_bytes!("../fonts/BebasNeue-Regular.ttf"),
    include_bytes!("../fonts/uicons-solid-rounded.ttf"),
    include_bytes!("../fonts/IMFellEnglishSC.ttf"),
    include_bytes!("../fonts/Cinzel-Bold.ttf"),
    include_bytes!("../fonts/CormorantSC-Bold.ttf"),
];

/// Deadlock's display face from the game's own files (`PULPIT_DEADLOCK_FONT` overrides the path).
fn game_font() -> &'static [u8] {
    let path = std::env::var_os("PULPIT_DEADLOCK_FONT").map(std::path::PathBuf::from).or_else(|| {
        let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
        Some(home.join(".local/share/Steam/steamapps/common/Deadlock/game/citadel/panorama/fonts/retaildemo-bold.otf"))
    });
    match path.and_then(|p| std::fs::read(p).ok()) {
        Some(bytes) => Box::leak(bytes.into_boxed_slice()),
        None => BUILTIN[0],
    }
}

/// A WoW font from the files extracted by games/wow_forever/tools/extract.py
/// (`PULPIT_WOW_SRC` overrides the folder); `None` when they are not there.
fn wow_font(file: &str) -> Option<&'static [u8]> {
    let dir = std::env::var_os("PULPIT_WOW_SRC").map(std::path::PathBuf::from).or_else(|| {
        Some(std::path::PathBuf::from(std::env::var_os("HOME")?).join(".local/share/pulpit/wow-src"))
    })?;
    std::fs::read(dir.join("fonts").join(file)).ok().map(|b| &*Box::leak(b.into_boxed_slice()))
}

/// Text faces get their cap height snapped to whole pixels (see [`Fonts::scaled`]); the icon
/// font keeps its shape. All glyphs land on whole pixels, as Pillow + FreeType place them.
fn hinted(font: FontId) -> bool {
    font != FontId::Icons
}

/// Sub-pixel steps the cache keeps per axis.
const SUB: f32 = 4.0;
/// Drop the raster cache when it grows past this (labels change over a long run).
const CACHE_MAX: usize = 1024;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    font: FontId,
    size: u32,
    text: String,
    fx: u8,
    fy: u8,
}

/// A text raster: coverage `w`×`h` whose top left sits at (`x0`, `y0`) from the integer pen
/// origin on the baseline.
pub struct Raster {
    pub x0: i32,
    pub y0: i32,
    pub w: usize,
    pub cov: Vec<f32>,
}

pub struct Fonts {
    fonts: [FontRef<'static>; N],
    /// Cap height (top of 'H') in font units.
    caps: [f32; N],
    kerning: [Kerning; N],
    cache: Mutex<HashMap<Key, Arc<Raster>>>,
    widths: Mutex<HashMap<(FontId, u32, String), f32>>,
}

#[derive(Clone, Copy)]
pub enum Anchor {
    /// Left edge at x.
    Left,
    /// Centre of the advance width at x.
    Middle,
}

type Scaled<'a> = PxScaleFont<&'a FontRef<'static>>;

impl Fonts {
    pub fn load() -> anyhow::Result<Self> {
        let data: [&'static [u8]; N] = std::array::from_fn(|i| match i {
            7 => game_font(),
            8 => wow_font("frizqt__.ttf").unwrap_or(BUILTIN[5]),
            9 => wow_font("morpheus.ttf").unwrap_or(BUILTIN[4]),
            _ => BUILTIN[i],
        });
        let f = |i: usize| FontRef::try_from_slice(data[i]);
        // A game font that does not parse falls back like a missing one.
        let fonts = [f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?, f(6)?, f(7).or_else(|_| f(0))?, f(8).or_else(|_| f(5))?, f(9).or_else(|_| f(4))?];
        // ab_glyph's unscaled outline bounds keep y_max in `min.y`.
        let caps = fonts.each_ref().map(|f| f.outline(f.glyph_id('H')).map_or(0.0, |o| o.bounds.min.y));
        let kerning = data.map(Kerning::new);
        Ok(Self {
            fonts,
            caps,
            kerning,
            cache: Mutex::new(HashMap::new()),
            widths: Mutex::new(HashMap::new()),
        })
    }

    /// The font at `size` px per em (Pillow's size; ab_glyph's scale is ascent − descent).
    /// Text faces get their vertical scale nudged so the cap height lands on whole pixels — a
    /// stand-in for the vertical hinting FreeType gives Pillow, which keeps small caps crisp.
    fn scaled(&self, font: FontId, size: f32) -> Scaled<'_> {
        let f = &self.fonts[font as usize];
        let em = f.units_per_em().unwrap_or(1000.0);
        let x = size * f.height_unscaled() / em;
        let mut y = x;
        if hinted(font) {
            let cap = self.caps[font as usize] * size / em;
            if cap > 0.0 {
                y = x * cap.round().max(1.0) / cap;
            }
        }
        f.as_scaled(PxScale { x, y })
    }

    fn ids(&self, font: FontId, text: &str) -> Vec<GlyphId> {
        let f = &self.fonts[font as usize];
        text.chars().map(|c| f.glyph_id(c)).collect()
    }

    /// Kerning before `g` (GPOS, else the legacy table) and the advance of `g`, in pixels.
    fn advance(&self, font: FontId, sf: &Scaled<'_>, prev: Option<GlyphId>, g: GlyphId) -> (f32, f32) {
        let k = prev.map_or(0.0, |p| {
            let gpos = self.kerning[font as usize].pair(p.0, g.0);
            if gpos != 0.0 {
                gpos * sf.h_scale_factor()
            } else {
                sf.kern(p, g)
            }
        });
        (k, sf.h_advance(g))
    }

    /// Advance width of `text` in pixels (cached: the app asks every frame).
    pub fn width(&self, font: FontId, size: f32, text: &str) -> f32 {
        let key = (font, size.to_bits(), text.to_owned());
        if let Some(&w) = self.widths.lock().expect("width cache").get(&key) {
            return w;
        }
        let sf = self.scaled(font, size);
        let ids = self.ids(font, text);
        let mut w = 0.0;
        for (i, &g) in ids.iter().enumerate() {
            let (k, a) = self.advance(font, &sf, i.checked_sub(1).map(|j| ids[j]), g);
            w += k + a;
        }
        let mut widths = self.widths.lock().expect("width cache");
        if widths.len() >= CACHE_MAX {
            widths.clear();
        }
        widths.insert(key, w);
        w
    }

    /// Offset from the vertical anchor "middle" down to the baseline.
    fn mid_to_baseline(&self, font: FontId, size: f32) -> f32 {
        let f = &self.fonts[font as usize];
        let k = size / f.units_per_em().unwrap_or(1000.0);
        let (mut asc, mut desc) = (f.ascent_unscaled() * k, f.descent_unscaled() * k);
        if hinted(font) {
            // FreeType rounds the hinted ascender up and the descender down.
            (asc, desc) = (asc.ceil(), desc.floor());
        }
        (asc + desc) / 2.0
    }

    fn raster(&self, font: FontId, size: f32, text: &str, fx: u8, fy: u8) -> Arc<Raster> {
        let key = Key { font, size: size.to_bits(), text: text.to_owned(), fx, fy };
        if let Some(r) = self.cache.lock().expect("font cache").get(&key) {
            return r.clone();
        }
        let r = Arc::new(self.rasterize(font, size, text, fx as f32 / SUB, fy as f32 / SUB));
        let mut cache = self.cache.lock().expect("font cache");
        if cache.len() >= CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, r.clone());
        r
    }

    fn rasterize(&self, font: FontId, size: f32, text: &str, fx: f32, fy: f32) -> Raster {
        let sf = self.scaled(font, size);
        let ids = self.ids(font, text);
        let mut pen = 0.0f32;
        let mut outlines = Vec::with_capacity(ids.len());
        for (i, &g) in ids.iter().enumerate() {
            let (k, a) = self.advance(font, &sf, i.checked_sub(1).map(|j| ids[j]), g);
            pen += k;
            // Like Pillow + FreeType: every glyph lands on a whole pixel, so stems stay sharp.
            let gx = (fx + pen).round();
            let glyph = g.with_scale_and_position(sf.scale(), point(gx, fy));
            pen += a;
            if let Some(o) = sf.outline_glyph(glyph) {
                outlines.push(o);
            }
        }
        if outlines.is_empty() {
            return Raster { x0: 0, y0: 0, w: 0, cov: Vec::new() };
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for o in &outlines {
            let b = o.px_bounds();
            x0 = x0.min(b.min.x);
            y0 = y0.min(b.min.y);
            x1 = x1.max(b.max.x);
            y1 = y1.max(b.max.y);
        }
        let (x0, y0) = (x0.floor() as i32, y0.floor() as i32);
        let w = (x1.ceil() as i32 - x0).max(0) as usize;
        let h = (y1.ceil() as i32 - y0).max(0) as usize;
        let mut cov = vec![0.0f32; w * h];
        for o in &outlines {
            let b = o.px_bounds();
            let (ox, oy) = (b.min.x as i32 - x0, b.min.y as i32 - y0);
            o.draw(|x, y, c| {
                let (px, py) = (ox + x as i32, oy + y as i32);
                if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                    let d = &mut cov[py as usize * w + px as usize];
                    *d = (*d + c).min(1.0);
                }
            });
        }
        Raster { x0, y0, w, cov }
    }

    /// Draw `text` with its vertical middle at `y` and `anchor` at `x`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        font: FontId,
        size: f32,
        text: &str,
        x: f32,
        y: f32,
        anchor: Anchor,
        color: Rgb,
        clip: Clip,
    ) {
        if text.is_empty() {
            return;
        }
        // Like Pillow: the anchor offset is whole pixels, the position's fraction is kept.
        let left = match anchor {
            Anchor::Left => x,
            Anchor::Middle => x - (self.width(font, size, text) / 2.0).round(),
        };
        let base = (y + self.mid_to_baseline(font, size)).round();
        let (ix, iy) = (left.floor(), base.floor());
        let fx = (((left - ix) * SUB).round() as u8).min(SUB as u8 - 1);
        let fy = (((base - iy) * SUB).round() as u8).min(SUB as u8 - 1);
        let r = self.raster(font, size, text, fx, fy);
        canvas.paint_at(&r.cov, r.w, ix as i32 + r.x0, iy as i32 + r.y0, color, clip);
    }

    /// Coverage of `text` drawn like [`Fonts::draw`] (anchor middle) into a fresh `w`×`h` cover,
    /// for glows.
    #[allow(clippy::too_many_arguments)]
    pub fn cover(&self, w: usize, h: usize, font: FontId, size: f32, text: &str, x: f32, y: f32) -> Cover {
        let mut c = Canvas::new(w, h, [0.0; 3]);
        let all = Clip { x0: 0, y0: 0, x1: w as i32, y1: h as i32 };
        self.draw(&mut c, font, size, text, x, y, Anchor::Middle, [1.0, 0.0, 0.0], all);
        Cover { w, h, v: c.px.iter().map(|p| p[0]).collect() }
    }

    pub fn has_glyph(&self, font: FontId, c: char) -> bool {
        self.fonts[font as usize].glyph_id(c).0 != 0
    }
}

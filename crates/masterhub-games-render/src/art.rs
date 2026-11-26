//! Drawn icons for game looks (game-icons.net, CC BY 3.0, see `art/CREDITS.md`): 512×512 SVGs
//! in `art/` whose last path is the drawing. Filled like logos, so they take colours and pop.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tiny_skia::{FillRule, Path, PathBuilder, Transform};

use crate::canvas::Cover;

macro_rules! art {
    ($($name:literal),* $(,)?) => {
        const ART: &[(&str, &str)] = &[$(($name, include_str!(concat!("../art/", $name, ".svg")))),*];
    };
}

art!(
    "treasure-map", "knapsack", "scroll-unfurled", "skills", "star-swirl", "photo-camera", "horse-head",
    "crossed-swords", "broadsword", "chicken-leg", "meat", "flame", "fire-ring", "hamburger-menu", "compass",
    "film-strip", "backward-time", "video-camera", "speedometer", "open-book", "dragon-spiral", "laurel-crown",
);

macro_rules! masks {
    ($($name:literal),* $(,)?) => {
        const MASKS: &[(&str, &[u8])] = &[$(($name, include_bytes!(concat!("../art/game/", $name, ".png")))),*];
    };
}

// Icons cut from the user's own screenshots of Crimson Desert (see `art/CREDITS.md`): grey
// PNG masks, 128 px. For this hub only; they must not be published.
masks!(
    "cd-backpack", "cd-sword", "cd-emblem", "cd-discovery", "cd-photo", "cd-gear", "cd-book", "cd-tab-pot",
    "cd-tab-scroll", "cd-star", "cd-save", "cd-compass", "cd-horseshoes", "cd-skill", "cd-flame",
    // Deadlock: the game's own vector icons, cut by games/deadlock/tools/icons.py.
    "dl-ap", "dl-binoculars", "dl-book", "dl-cart", "dl-chat", "dl-check", "dl-cursor", "dl-down", "dl-download", "dl-eye", "dl-eye-off", "dl-mic-off", "dl-faster", "dl-gear", "dl-globe", "dl-graph", "dl-grid", "dl-gun", "dl-keyboard", "dl-left", "dl-list", "dl-logo-inner", "dl-logo-outer", "dl-logo", "dl-meter", "dl-pause", "dl-play", "dl-player", "dl-quickbuy", "dl-refresh", "dl-right", "dl-slower", "dl-soul", "dl-sound", "dl-sound-off", "dl-spirit", "dl-star", "dl-survival", "dl-team", "dl-ult", "dl-up", "dl-watch", "dl-x",
);

pub fn names() -> Vec<&'static str> {
    ART.iter().map(|(n, _)| *n).chain(MASKS.iter().map(|(n, _)| *n)).collect()
}

/// Whether `name` is a cut-out from the game (a mask), not a drawing.
pub fn from_game(name: &str) -> bool {
    MASKS.iter().any(|(n, _)| *n == name)
}

fn mask_of(name: &str) -> Option<&'static image::GrayImage> {
    static CACHE: OnceLock<HashMap<&'static str, image::GrayImage>> = OnceLock::new();
    CACHE
        .get_or_init(|| MASKS.iter().filter_map(|(n, png)| Some((*n, image::load_from_memory(png).ok()?.into_luma8()))).collect())
        .get(name)
}

fn mask_cover(mask: &image::GrayImage, w: usize, h: usize, size: f32, cx: f32, cy: f32) -> Cover {
    let px = size.round().max(1.0) as u32;
    let small = image::imageops::resize(mask, px, px, image::imageops::FilterType::CatmullRom);
    let mut c = Cover::new(w, h);
    let (x0, y0) = ((cx - px as f32 / 2.0).round() as i32, (cy - px as f32 / 2.0).round() as i32);
    for (x, y, p) in small.enumerate_pixels() {
        let (tx, ty) = (x0 + x as i32, y0 + y as i32);
        if tx >= 0 && ty >= 0 && (tx as usize) < w && (ty as usize) < h {
            c.v[ty as usize * w + tx as usize] = p.0[0] as f32 / 255.0;
        }
    }
    c
}

fn path_of(name: &str) -> Option<Path> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Path>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(p) = cache.lock().unwrap().get(name) {
        return p.clone();
    }
    let p = ART.iter().find(|(n, _)| *n == name).and_then(|(_, svg)| parse(svg));
    cache.lock().unwrap().insert(name.to_owned(), p.clone());
    p
}

fn parse(svg: &str) -> Option<Path> {
    let d = svg.split(" d=\"").last()?.split('"').next()?;
    let mut pb = PathBuilder::new();
    for seg in svgtypes::SimplifyingPathParser::from(d) {
        match seg.ok()? {
            svgtypes::SimplePathSegment::MoveTo { x, y } => pb.move_to(x as f32, y as f32),
            svgtypes::SimplePathSegment::LineTo { x, y } => pb.line_to(x as f32, y as f32),
            svgtypes::SimplePathSegment::Quadratic { x1, y1, x, y } => pb.quad_to(x1 as f32, y1 as f32, x as f32, y as f32),
            svgtypes::SimplePathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                pb.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32)
            }
            svgtypes::SimplePathSegment::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

pub fn exists(name: &str) -> bool {
    from_game(name) || path_of(name).is_some()
}

/// Coverage of drawing `name`, `size` px square, centred on (cx, cy), in a w×h buffer.
pub fn cover(name: &str, w: usize, h: usize, size: f32, cx: f32, cy: f32) -> Option<Cover> {
    if let Some(m) = mask_of(name) {
        return Some(mask_cover(m, w, h, size, cx, cy));
    }
    let path = path_of(name)?;
    let k = size / 512.0;
    let path = path.transform(Transform::from_row(k, 0.0, 0.0, k, cx - size / 2.0, cy - size / 2.0))?;
    Some(Cover::from_path(w, h, &path, FillRule::Winding))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_drawing_parses() {
        for n in super::names() {
            assert!(super::exists(n), "{n}");
        }
    }
}

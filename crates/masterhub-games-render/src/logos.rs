//! App logos: single-path 24×24 SVGs in `logos/`, filled like icon glyphs. The game modes use
//! none, so none are bundled (brand marks are not ours to ship); the hook stays for the faces.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tiny_skia::{FillRule, Path, PathBuilder, Transform};

use crate::canvas::Cover;

macro_rules! logos {
    ($($name:literal),* $(,)?) => {
        const LOGOS: &[(&str, &str)] = &[$(($name, include_str!(concat!("../logos/", $name, ".svg")))),*];
    };
}

logos!();

/// Names of the logos a [`crate::KeyFace`] can use.
pub fn names() -> Vec<&'static str> {
    LOGOS.iter().map(|(n, _)| *n).collect()
}

fn path_of(name: &str) -> Option<Path> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Path>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(p) = cache.lock().unwrap().get(name) {
        return p.clone();
    }
    let p = LOGOS.iter().find(|(n, _)| *n == name).and_then(|(_, svg)| parse(svg));
    cache.lock().unwrap().insert(name.to_owned(), p.clone());
    p
}

/// The `d` of the SVG's path, in its 24×24 viewBox.
fn parse(svg: &str) -> Option<Path> {
    let d = svg.split(" d=\"").nth(1)?.split('"').next()?;
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
    path_of(name).is_some()
}

/// Coverage of logo `name`, `size` px square, centred on (cx, cy), in a w×h buffer.
pub fn cover(name: &str, w: usize, h: usize, size: f32, cx: f32, cy: f32) -> Option<Cover> {
    let path = path_of(name)?;
    let k = size / 24.0;
    let path = path.transform(Transform::from_row(k, 0.0, 0.0, k, cx - size / 2.0, cy - size / 2.0))?;
    Some(Cover::from_path(w, h, &path, FillRule::Winding))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_logo_parses() {
        for n in super::names() {
            assert!(super::exists(n), "{n}");
        }
    }
}

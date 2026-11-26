//! GPOS pair kerning (the `kern` feature), which ab_glyph does not read. Pillow lays text out
//! with HarfBuzz, so without this pairs like "AV" sit a pixel wider than in the prototype.

use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
use ttf_parser::opentype_layout::LayoutTable;
use ttf_parser::{Face, GlyphId, Tag};

pub struct Kerning {
    face: Option<Face<'static>>,
    /// Lookup indices of the `kern` feature.
    lookups: Vec<u16>,
}

impl Kerning {
    pub fn new(data: &'static [u8]) -> Self {
        let face = Face::parse(data, 0).ok();
        let mut lookups = Vec::new();
        if let Some(gpos) = face.as_ref().and_then(|f| f.tables().gpos) {
            let kern = Tag::from_bytes(b"kern");
            for i in 0..gpos.features.len() {
                if let Some(feature) = gpos.features.get(i).filter(|f| f.tag == kern) {
                    for l in feature.lookup_indices {
                        if !lookups.contains(&l) {
                            lookups.push(l);
                        }
                    }
                }
            }
        }
        Self { face, lookups }
    }

    /// Horizontal adjustment between `a` and `b`, in font units.
    pub fn pair(&self, a: u16, b: u16) -> f32 {
        let Some(gpos) = self.face.as_ref().and_then(|f| f.tables().gpos) else { return 0.0 };
        self.lookups.iter().find_map(|&l| pair_in(&gpos, l, GlyphId(a), GlyphId(b))).unwrap_or(0) as f32
    }
}

fn pair_in(gpos: &LayoutTable<'_>, lookup: u16, a: GlyphId, b: GlyphId) -> Option<i16> {
    let lookup = gpos.lookups.get(lookup)?;
    for i in 0..lookup.subtables.len() {
        let Some(PositioningSubtable::Pair(pair)) = lookup.subtables.get::<PositioningSubtable>(i) else {
            continue;
        };
        let found = match pair {
            PairAdjustment::Format1 { coverage, sets } => {
                coverage.get(a).and_then(|idx| sets.get(idx)).and_then(|set| set.get(b))
            }
            PairAdjustment::Format2 { coverage, classes, matrix } => coverage
                .get(a)
                .and_then(|_| matrix.get((classes.0.get(a), classes.1.get(b)))),
        };
        if let Some((first, _)) = found {
            return Some(first.x_advance);
        }
    }
    None
}

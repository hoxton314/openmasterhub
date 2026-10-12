//! Where a module sits on the Base.
//!
//! The Base has a 4×6 grid of cells (in its *vertical* orientation, col 0/row 0 at the
//! top-left) and 27 USB sockets. Each socket ends in a 4-pin contact **pad** at a fixed point
//! of the Base, with a fixed orientation; each module kind has its **pins** at a fixed point
//! of its underside. A module's USB port path tells which pad it sits on, and from pad + pins
//! + orientation its cells follow, including placements that overhang the Base.
//!
//! The pad/pin geometry was fitted to the vendor app's per-socket CSS tables (all entries
//! agree except a few sloppy CSS entries) and checked against six real arrangements on
//! hardware; pads "34" and "121" were measured on hardware. Coordinates are in half cells to stay exact.

use masterhub_proto::ModuleKind;
use serde::{Deserialize, Serialize};

pub const GRID_COLS: u8 = 4;
pub const GRID_ROWS: u8 = 6;

/// Cells covered by a module, in the Base's vertical frame. A module may overhang the Base,
/// so `col`/`row` can be negative and the area can extend past the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub col: i8,
    pub row: i8,
    /// Width and height in cells, after rotation.
    pub cols: u8,
    pub rows: u8,
    /// Clockwise rotation of the module relative to its natural orientation: 0, 90, 180, 270.
    pub rotation: u16,
}

pub const BASE_PLACEMENT: Placement = Placement { col: 0, row: 0, cols: GRID_COLS, rows: GRID_ROWS, rotation: 0 };

/// Natural footprint (width × height in cells) of each module.
pub fn footprint(kind: ModuleKind) -> (u8, u8) {
    match kind {
        ModuleKind::Base => (GRID_COLS, GRID_ROWS),
        ModuleKind::Keypad => (4, 3),
        ModuleKind::Fader => (4, 2),
        ModuleKind::Knob | ModuleKind::Dial => (2, 2),
        ModuleKind::Roller => (4, 1),
    }
}

/// Pin position on the module's underside (half cells, natural orientation) and the extra
/// rotation the module gets relative to the pad's orientation.
fn pins(kind: ModuleKind) -> Option<((i8, i8), u16)> {
    Some(match kind {
        ModuleKind::Keypad => ((8, 3), 0),
        ModuleKind::Fader => ((8, 1), 0),
        ModuleKind::Knob | ModuleKind::Dial => ((4, 1), 0),
        ModuleKind::Roller => ((5, 0), 90),
        ModuleKind::Base => return None,
    })
}

/// (socket, pad x, pad y in half cells, pad orientation in degrees).
const PADS: &[(&str, i8, i8, u16)] = &[
    ("111", 1, 4, 270),
    ("112", 5, 2, 270),
    ("113", 8, 1, 0),
    ("114", 6, 7, 180),
    // Measured on hardware (fader, arrangement 6); agrees with the vendor's roller entry.
    ("121", 5, 10, 270),
    ("122", 8, 9, 0),
    ("123", 4, 9, 0),
    ("124", 1, 8, 270),
    ("131", 4, 3, 180),
    ("132", 3, 0, 270),
    ("133", 6, 3, 180),
    ("134", 5, 0, 270),
    ("141", 0, 3, 180),
    ("142", 2, 3, 180),
    ("143", 1, 0, 270),
    ("21", 4, 7, 180),
    ("22", 3, 4, 270),
    ("23", 5, 4, 270),
    ("24", 8, 3, 0),
    ("31", 4, 5, 0),
    ("32", 2, 7, 180),
    ("33", 0, 7, 180),
    // Not in the vendor data; measured on hardware (roller, arrangement 5).
    ("34", 4, 1, 0),
    ("41", 8, 7, 0),
    ("42", 8, 5, 0),
    ("43", 5, 6, 270),
    ("44", 5, 8, 270),
    // "144" is the Base MCU itself.
];

/// How far (clockwise) to turn a module's screen images so they read upright for the user,
/// given the module's rotation on the Base and the Base's rotation on the desk.
pub fn upright_turn(module_rotation: u16, base_rotation: u16) -> u16 {
    (360 - (module_rotation + base_rotation) % 360) % 360
}

/// The socket a module is plugged into: its USB port path below the Base's top hub as a digit
/// string ("22", "134", …). `module_ports` and `base_ports` are the dotted port lists from
/// sysfs (e.g. `3-1.2.2` → `[1, 2, 2]`). The Base MCU itself sits at socket "144".
pub fn socket(module_ports: &[u8], base_ports: &[u8]) -> Option<String> {
    let top_hub = base_ports.len().checked_sub(3)?;
    let prefix = &base_ports[..top_hub];
    let rest = module_ports.strip_prefix(prefix)?;
    (!rest.is_empty()).then(|| rest.iter().map(|p| p.to_string()).collect())
}

/// Parse the port list from a USB sysfs name like `3-1.2.2`.
pub fn ports_from_sysfs(name: &str) -> Option<Vec<u8>> {
    let (_bus, ports) = name.split_once('-')?;
    ports.split('.').map(|p| p.parse().ok()).collect()
}

/// Cells and rotation for a module of `kind` in `socket`. `None` for an unknown socket.
pub fn place(kind: ModuleKind, socket: &str) -> Option<Placement> {
    if kind == ModuleKind::Base {
        return Some(BASE_PLACEMENT);
    }
    let &(_, px, py, orientation) = PADS.iter().find(|p| p.0 == socket)?;
    let ((u, v), extra) = pins(kind)?;
    let rotation = (orientation + extra) % 360;
    let (w, h) = footprint(kind);
    let (w2, h2) = (2 * w as i8, 2 * h as i8);
    // Pins in the placed (rotated) module, relative to its top-left corner.
    let (x, y) = match rotation {
        0 => (u, v),
        90 => (h2 - v, u),
        180 => (w2 - u, h2 - v),
        _ => (v, w2 - u),
    };
    let (col2, row2) = (px - x, py - y);
    if col2 % 2 != 0 || row2 % 2 != 0 {
        return None;
    }
    let (cols, rows) = if rotation % 180 == 90 { (h, w) } else { (w, h) };
    Some(Placement { col: col2 / 2, row: row2 / 2, cols, rows, rotation })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sockets_from_real_hub() {
        // sysfs names observed on the user's hub
        let base = ports_from_sysfs("3-1.1.4.4").unwrap();
        let sock = |n: &str| socket(&ports_from_sysfs(n).unwrap(), &base).unwrap();
        assert_eq!(sock("3-1.1.4.4"), "144");
        assert_eq!(sock("3-1.2.2"), "22");
        assert_eq!(sock("3-1.1.1.4"), "114");
        assert_eq!(sock("3-1.1.3.4"), "134");
        assert_eq!(sock("3-1.1.4.3"), "143");
    }

    #[test]
    fn every_pad_places_every_module() {
        for &(s, ..) in PADS {
            for kind in [ModuleKind::Keypad, ModuleKind::Fader, ModuleKind::Knob, ModuleKind::Dial, ModuleKind::Roller] {
                let p = place(kind, s).unwrap_or_else(|| panic!("{kind:?} at {s}"));
                let (w, h) = footprint(kind);
                let (w, h) = if p.rotation % 180 == 90 { (h, w) } else { (w, h) };
                assert_eq!((p.cols, p.rows), (w, h));
            }
        }
    }

    /// Place modules, check that nothing overlaps; returns which Base cells are covered.
    fn tile(mods: &[(ModuleKind, &str)]) -> [[bool; GRID_COLS as usize]; GRID_ROWS as usize] {
        let mut covered = std::collections::HashSet::new();
        let mut grid = [[false; GRID_COLS as usize]; GRID_ROWS as usize];
        for &(kind, s) in mods {
            let p = place(kind, s).unwrap_or_else(|| panic!("{kind:?} at {s} unknown"));
            for r in p.row..p.row + p.rows as i8 {
                for c in p.col..p.col + p.cols as i8 {
                    assert!(covered.insert((c, r)), "{kind:?} overlaps at {c},{r}");
                    if (0..GRID_COLS as i8).contains(&c) && (0..GRID_ROWS as i8).contains(&r) {
                        grid[r as usize][c as usize] = true;
                    }
                }
            }
        }
        grid
    }

    fn free(grid: &[[bool; GRID_COLS as usize]; GRID_ROWS as usize]) -> Vec<(usize, usize)> {
        (0..GRID_ROWS as usize).flat_map(|r| (0..GRID_COLS as usize).map(move |c| (c, r))).filter(|&(c, r)| !grid[r][c]).collect()
    }

    fn at(kind: ModuleKind, s: &str) -> (i8, i8, u16) {
        let p = place(kind, s).unwrap();
        (p.col, p.row, p.rotation)
    }

    // The arrangements below were plugged on real hardware and photographed.

    #[test]
    fn arrangement_1_flat() {
        let grid = tile(&[(ModuleKind::Keypad, "22"), (ModuleKind::Roller, "114"), (ModuleKind::Dial, "134"), (ModuleKind::Knob, "143")]);
        assert!(free(&grid).is_empty());
        assert_eq!(at(ModuleKind::Keypad, "22"), (0, 2, 270));
    }

    #[test]
    fn arrangement_2_keypad_overhangs() {
        let grid = tile(&[(ModuleKind::Keypad, "113"), (ModuleKind::Fader, "111"), (ModuleKind::Knob, "42"), (ModuleKind::Dial, "122")]);
        assert!(free(&grid).is_empty());
        assert_eq!(at(ModuleKind::Keypad, "113"), (0, -1, 0));
    }

    #[test]
    fn arrangement_3_two_overhangs() {
        let grid = tile(&[
            (ModuleKind::Keypad, "124"),
            (ModuleKind::Fader, "143"),
            (ModuleKind::Dial, "42"),
            (ModuleKind::Knob, "122"),
            (ModuleKind::Roller, "113"),
        ]);
        assert_eq!(free(&grid), vec![(2, 0), (2, 1)]); // bare strip next to the dial
        assert_eq!(at(ModuleKind::Keypad, "124"), (-1, 4, 270));
        assert_eq!(at(ModuleKind::Roller, "113"), (3, -2, 90));
    }

    #[test]
    fn arrangement_4_everything_overhangs() {
        let grid = tile(&[
            (ModuleKind::Dial, "133"),
            (ModuleKind::Fader, "21"),
            (ModuleKind::Keypad, "123"),
            (ModuleKind::Roller, "132"),
            (ModuleKind::Knob, "122"),
        ]);
        assert_eq!(free(&grid), vec![(0, 1), (1, 1), (2, 1), (0, 2), (1, 2)]); // bare middle of the Base
        assert_eq!(at(ModuleKind::Dial, "133"), (3, 0, 180));
        assert_eq!(at(ModuleKind::Fader, "21"), (2, 2, 180));
        assert_eq!(at(ModuleKind::Keypad, "123"), (-2, 3, 0));
        assert_eq!(at(ModuleKind::Roller, "132"), (-1, 0, 0));
    }

    #[test]
    fn arrangement_5_roller_on_pad_34() {
        let grid = tile(&[
            (ModuleKind::Dial, "133"),
            (ModuleKind::Fader, "21"),
            (ModuleKind::Keypad, "123"),
            (ModuleKind::Roller, "34"),
            (ModuleKind::Knob, "122"),
        ]);
        assert_eq!(at(ModuleKind::Roller, "34"), (1, -2, 90));
        // bare Base: the strip above the roller and the patch below it, as in the photo
        assert_eq!(free(&grid), vec![(0, 0), (2, 0), (0, 1), (2, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn arrangement_6_fader_below() {
        // Base upside down in portrait: keypad at the top of the Base frame, fader hanging off
        // the bottom edge.
        let grid = tile(&[(ModuleKind::Keypad, "24"), (ModuleKind::Fader, "121")]);
        assert_eq!(at(ModuleKind::Fader, "121"), (2, 5, 270));
        assert_eq!(free(&grid).len(), 4 * 6 - 12 - 2);
    }

    #[test]
    fn upright_turns_match_photos() {
        // Keypad at "24" on a Base lying portrait upside down: keys looked upside down.
        assert_eq!(upright_turn(place(ModuleKind::Keypad, "24").unwrap().rotation, 180), 180);
        // Dial at "133" on a landscape Base: its text read bottom-to-top (turned 270°).
        assert_eq!(upright_turn(place(ModuleKind::Dial, "133").unwrap().rotation, 90), 90);
        // Keypad at "22" on a landscape Base was already upright.
        assert_eq!(upright_turn(place(ModuleKind::Keypad, "22").unwrap().rotation, 90), 0);
    }

    /// The vendor app's CSS tables (docs/app-architecture.md §2.3): the model must reproduce
    /// them, except a few entries that the doc already flags as sloppy.
    #[test]
    fn agrees_with_vendor_tables() {
        type E = (&'static str, i8, i8, i8, i8, u16);
        let vendor: [(ModuleKind, &[E]); 4] = [
            (ModuleKind::Keypad, &[("24", 0, 3, 0, 2, 0), ("42", 0, 3, 1, 3, 0), ("41", 0, 3, 2, 4, 0), ("122", 0, 3, 3, 5, 0), ("141", 0, 3, 0, 2, 180), ("33", 0, 3, 2, 4, 180), ("132", 0, 2, 0, 3, 270), ("134", 1, 3, 0, 3, 270), ("112", 1, 3, 1, 4, 270), ("22", 0, 2, 2, 5, 270), ("23", 1, 3, 2, 5, 270)]),
            (ModuleKind::Fader, &[("113", 0, 3, 0, 1, 0), ("24", 0, 3, 1, 2, 0), ("42", 0, 3, 2, 3, 0), ("41", 0, 3, 3, 4, 0), ("122", 0, 3, 4, 5, 0), ("141", 0, 3, 0, 1, 180), ("33", 0, 3, 2, 3, 180), ("143", 0, 1, 0, 3, 270), ("132", 1, 2, 0, 3, 270), ("134", 2, 3, 0, 3, 270), ("32", 2, 3, 0, 3, 270), ("112", 2, 3, 1, 4, 270), ("111", 0, 1, 2, 5, 270), ("22", 1, 2, 2, 5, 270), ("23", 2, 3, 2, 5, 270), ("31", 2, 3, 2, 5, 270)]),
            (ModuleKind::Knob, &[("24", 2, 3, 1, 2, 0), ("31", 0, 1, 2, 3, 0), ("42", 2, 3, 2, 3, 0), ("41", 2, 3, 3, 4, 0), ("123", 0, 1, 4, 5, 0), ("122", 2, 3, 4, 5, 0), ("121", 0, 1, 2, 3, 0), ("141", 0, 1, 0, 1, 180), ("142", 1, 2, 0, 1, 180), ("131", 2, 3, 0, 1, 180), ("33", 0, 1, 2, 3, 180), ("32", 1, 2, 2, 3, 180), ("21", 2, 3, 2, 3, 180), ("143", 0, 1, 0, 1, 270), ("132", 1, 2, 0, 1, 270), ("134", 2, 3, 0, 1, 270), ("112", 2, 3, 1, 2, 270), ("111", 0, 1, 2, 3, 270), ("22", 1, 2, 2, 3, 270), ("23", 2, 3, 2, 3, 270), ("43", 2, 3, 3, 4, 270), ("124", 0, 1, 4, 5, 270), ("44", 2, 3, 4, 5, 270)]),
            (ModuleKind::Roller, &[("134", 0, 3, 0, 0, 0), ("112", 0, 3, 1, 1, 0), ("23", 0, 3, 2, 2, 0), ("43", 0, 3, 3, 3, 0), ("44", 0, 3, 4, 4, 0), ("121", 0, 3, 5, 5, 0), ("31", 1, 1, 0, 3, 90), ("42", 3, 3, 0, 3, 90), ("123", 1, 1, 2, 5, 90), ("122", 3, 3, 2, 5, 90), ("141", 0, 0, 0, 3, 270), ("142", 1, 1, 0, 3, 270), ("124", 2, 2, 0, 3, 270), ("131", 2, 2, 0, 3, 270), ("133", 3, 3, 0, 3, 270), ("33", 0, 0, 2, 5, 270), ("32", 1, 1, 2, 5, 270), ("21", 2, 2, 2, 5, 270), ("114", 3, 3, 2, 5, 270)]),
        ];
        // Sloppy vendor CSS: duplicated/odd offsets noted in the doc.
        // The knob/dial "121" entry contradicts hardware (arrangement 6) and the roller entry.
        let known_bad = [(ModuleKind::Fader, "31"), (ModuleKind::Fader, "32"), (ModuleKind::Knob, "121"), (ModuleKind::Roller, "124")];
        for (kind, table) in vendor {
            for &(s, c0, _, r0, _, rot) in table {
                if known_bad.contains(&(kind, s)) {
                    continue;
                }
                assert_eq!(at(kind, s), (c0, r0, rot), "{kind:?} at {s}");
            }
        }
    }
}

//! Where a module sits on the Base: the Base has a 4×6 grid of cells (in its *vertical*
//! orientation, col 0/row 0 at the top-left) and 27 USB sockets. A module's socket is known
//! from its USB port path, and (module kind, socket) determines the cells it covers and its
//! rotation. Tables come from the vendor app's CSS (see docs/app-architecture.md §2.3).

use masterhub_proto::ModuleKind;
use serde::{Deserialize, Serialize};

pub const GRID_COLS: u8 = 4;
pub const GRID_ROWS: u8 = 6;

/// Cells covered by a module, in the Base's vertical frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub col: u8,
    pub row: u8,
    /// Width and height in cells, after rotation.
    pub cols: u8,
    pub rows: u8,
    /// Clockwise rotation of the module relative to its natural orientation: 0, 90, 180, 270.
    pub rotation: u16,
}

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

/// Cells and rotation for a module of `kind` in `socket`. `None` for unknown combinations.
pub fn place(kind: ModuleKind, socket: &str) -> Option<Placement> {
    let table: &[(&str, u8, u8, u8, u8, u16)] = match kind {
        ModuleKind::Base => return Some(Placement { col: 0, row: 0, cols: GRID_COLS, rows: GRID_ROWS, rotation: 0 }),
        ModuleKind::Keypad => KEYPAD,
        ModuleKind::Fader => FADER,
        ModuleKind::Knob | ModuleKind::Dial => KNOB,
        ModuleKind::Roller => ROLLER,
    };
    let &(_, c0, c1, r0, r1, rotation) = table.iter().find(|e| e.0 == socket)?;
    Some(Placement { col: c0, row: r0, cols: c1 - c0 + 1, rows: r1 - r0 + 1, rotation })
}

// (socket, col_from, col_to, row_from, row_to, rotation), inclusive ranges.
const KEYPAD: &[(&str, u8, u8, u8, u8, u16)] = &[
    ("24", 0, 3, 0, 2, 0),
    ("42", 0, 3, 1, 3, 0),
    ("41", 0, 3, 2, 4, 0),
    ("122", 0, 3, 3, 5, 0),
    ("141", 0, 3, 0, 2, 180),
    ("33", 0, 3, 2, 4, 180),
    ("132", 0, 2, 0, 3, 270),
    ("134", 1, 3, 0, 3, 270),
    ("112", 1, 3, 1, 4, 270),
    ("22", 0, 2, 2, 5, 270),
    ("23", 1, 3, 2, 5, 270),
];

const FADER: &[(&str, u8, u8, u8, u8, u16)] = &[
    ("113", 0, 3, 0, 1, 0),
    ("24", 0, 3, 1, 2, 0),
    ("42", 0, 3, 2, 3, 0),
    ("41", 0, 3, 3, 4, 0),
    ("122", 0, 3, 4, 5, 0),
    ("141", 0, 3, 0, 1, 180),
    ("33", 0, 3, 2, 3, 180),
    ("143", 0, 1, 0, 3, 270),
    ("132", 1, 2, 0, 3, 270),
    ("134", 2, 3, 0, 3, 270),
    ("32", 2, 3, 0, 3, 270),
    ("112", 2, 3, 1, 4, 270),
    ("111", 0, 1, 2, 5, 270),
    ("22", 1, 2, 2, 5, 270),
    ("23", 2, 3, 2, 5, 270),
    ("31", 2, 3, 2, 5, 270),
];

const KNOB: &[(&str, u8, u8, u8, u8, u16)] = &[
    ("24", 2, 3, 1, 2, 0),
    ("31", 0, 1, 2, 3, 0),
    ("42", 2, 3, 2, 3, 0),
    ("41", 2, 3, 3, 4, 0),
    ("123", 0, 1, 4, 5, 0),
    ("122", 2, 3, 4, 5, 0),
    ("121", 0, 1, 2, 3, 0),
    ("141", 0, 1, 0, 1, 180),
    ("142", 1, 2, 0, 1, 180),
    ("131", 2, 3, 0, 1, 180),
    ("33", 0, 1, 2, 3, 180),
    ("32", 1, 2, 2, 3, 180),
    ("21", 2, 3, 2, 3, 180),
    ("143", 0, 1, 0, 1, 270),
    ("132", 1, 2, 0, 1, 270),
    ("134", 2, 3, 0, 1, 270),
    ("112", 2, 3, 1, 2, 270),
    ("111", 0, 1, 2, 3, 270),
    ("22", 1, 2, 2, 3, 270),
    ("23", 2, 3, 2, 3, 270),
    ("43", 2, 3, 3, 4, 270),
    ("124", 0, 1, 4, 5, 270),
    ("44", 2, 3, 4, 5, 270),
];

const ROLLER: &[(&str, u8, u8, u8, u8, u16)] = &[
    ("134", 0, 3, 0, 0, 0),
    ("112", 0, 3, 1, 1, 0),
    ("23", 0, 3, 2, 2, 0),
    ("43", 0, 3, 3, 3, 0),
    ("44", 0, 3, 4, 4, 0),
    ("121", 0, 3, 5, 5, 0),
    ("31", 1, 1, 0, 3, 90),
    ("42", 3, 3, 0, 3, 90),
    ("123", 1, 1, 2, 5, 90),
    ("122", 3, 3, 2, 5, 90),
    ("141", 0, 0, 0, 3, 270),
    ("142", 1, 1, 0, 3, 270),
    ("124", 2, 2, 0, 3, 270),
    ("131", 2, 2, 0, 3, 270),
    ("133", 3, 3, 0, 3, 270),
    ("33", 0, 0, 2, 5, 270),
    ("32", 1, 1, 2, 5, 270),
    ("21", 2, 2, 2, 5, 270),
    ("114", 3, 3, 2, 5, 270),
];

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
    fn real_layout_tiles_without_overlap() {
        // keypad + roller + dial + knob as plugged on the user's hub
        let mods = [
            (ModuleKind::Keypad, "22"),
            (ModuleKind::Roller, "114"),
            (ModuleKind::Dial, "134"),
            (ModuleKind::Knob, "143"),
        ];
        let mut grid = [[false; GRID_COLS as usize]; GRID_ROWS as usize];
        for (kind, s) in mods {
            let p = place(kind, s).unwrap();
            let (w, h) = footprint(kind);
            let (w, h) = if p.rotation % 180 == 90 { (h, w) } else { (w, h) };
            assert_eq!((p.cols, p.rows), (w, h), "{kind:?} footprint");
            for r in p.row..p.row + p.rows {
                for c in p.col..p.col + p.cols {
                    assert!(!grid[r as usize][c as usize], "{kind:?} overlaps at {c},{r}");
                    grid[r as usize][c as usize] = true;
                }
            }
        }
        assert!(grid.iter().flatten().all(|&x| x), "the four modules fill the base");
    }

    #[test]
    fn all_table_entries_match_footprints() {
        for (kind, table) in [
            (ModuleKind::Keypad, KEYPAD),
            (ModuleKind::Fader, FADER),
            (ModuleKind::Knob, KNOB),
            (ModuleKind::Roller, ROLLER),
        ] {
            for e in table {
                let p = place(kind, e.0).unwrap();
                let (w, h) = footprint(kind);
                let (w, h) = if p.rotation % 180 == 90 { (h, w) } else { (w, h) };
                assert_eq!((p.cols, p.rows), (w, h), "{kind:?} socket {}", e.0);
                assert!(p.col + p.cols <= GRID_COLS && p.row + p.rows <= GRID_ROWS, "{kind:?} {}", e.0);
            }
        }
    }
}

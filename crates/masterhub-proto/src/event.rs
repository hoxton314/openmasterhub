use crate::ModuleKind;

/// Number of keys on the keypad.
pub const KEY_COUNT: usize = 15;

/// Rotation direction from the user's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Clockwise,
    CounterClockwise,
}

/// Roller direction: away from the user (towards the keypad) or towards the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roll {
    Away,
    Toward,
}

/// A decoded input report. Stateless: snapshots (keys, faders, knob buttons) are reported as
/// is; turn them into edges with [`KeyTracker`] or by comparing against the last value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputReport {
    /// Full state of the 15 keys, `true` = pressed. Index is row-major from the top-left.
    Keys([bool; KEY_COUNT]),
    /// Absolute fader positions 0 (left) ..= 100 (right). Index 0 is the top fader.
    Faders([u8; 5]),
    /// One detent on one or both rollers.
    Rollers { left: Option<Roll>, right: Option<Roll> },
    /// One detent on the knobs. Index 0 = bottom-left, 1 = bottom-right, 2 = top.
    KnobTurn([Option<Rotation>; 3]),
    /// Knob push-button state, same indexing as [`InputReport::KnobTurn`].
    KnobButtons([bool; 3]),
    /// One step of the dial (about 36 per revolution).
    DialTurn(Rotation),
    DialButton { pressed: bool },
    /// Base orientation, sent periodically.
    BaseOrientation { horizontal: bool },
    /// Anything not understood yet; kept for logging.
    Unknown { kind: u8, payload: Vec<u8> },
}

/// Decode one input report (as read from hidraw, starting with report ID `0x01`).
/// Returns `None` if the buffer is not an input report at all.
pub fn decode_input(module: ModuleKind, report: &[u8]) -> Option<InputReport> {
    if report.len() < 4 || report[0] != 0x01 {
        return None;
    }
    let kind = report[1];
    let len = u16::from_le_bytes([report[2], report[3]]) as usize;
    let p = &report[4..(4 + len).min(report.len())];
    let unknown = || InputReport::Unknown { kind, payload: p.to_vec() };

    let decoded = match (module, kind) {
        // 02 0f s0..s14
        (ModuleKind::Keypad, 0x34) if p.len() >= 2 + KEY_COUNT && p[1] as usize == KEY_COUNT => {
            InputReport::Keys(std::array::from_fn(|i| p[2 + i] != 0))
        }
        // 02 05 f0..f4
        (ModuleKind::Fader, 0x2A) if p.len() >= 7 => InputReport::Faders(std::array::from_fn(|i| p[2 + i])),
        // 01 02 R L
        (ModuleKind::Roller, 0x2A) if p.len() >= 4 => {
            let roll = |b: u8| match b {
                0 => Some(Roll::Away),
                1 => Some(Roll::Toward),
                _ => None,
            };
            InputReport::Rollers { right: roll(p[2]), left: roll(p[3]) }
        }
        // <sub> 03 k0 k1 k2
        (ModuleKind::Knob, 0x2A) if p.len() >= 5 => match p[0] {
            0 => InputReport::KnobTurn(std::array::from_fn(|i| match p[2 + i] {
                1 => Some(Rotation::Clockwise),
                0 => Some(Rotation::CounterClockwise),
                _ => None,
            })),
            3 => InputReport::KnobButtons(std::array::from_fn(|i| p[2 + i] == 1)),
            _ => unknown(),
        },
        // <d0> 01 <d2>
        (ModuleKind::Dial, 0x44) if p.len() >= 3 => match p[0] {
            3 => InputReport::DialTurn(if p[2] == 0 { Rotation::Clockwise } else { Rotation::CounterClockwise }),
            4 => InputReport::DialButton { pressed: p[2] == 1 },
            _ => unknown(),
        },
        (ModuleKind::Base, 0x2B) if !p.is_empty() => InputReport::BaseOrientation { horizontal: p[0] == 1 },
        _ => unknown(),
    };
    Some(decoded)
}

/// Turns state snapshots of `N` buttons into press/release edges.
#[derive(Debug, Clone)]
pub struct KeyTracker<const N: usize> {
    state: [bool; N],
}

impl<const N: usize> Default for KeyTracker<N> {
    fn default() -> Self {
        Self { state: [false; N] }
    }
}

impl<const N: usize> KeyTracker<N> {
    /// Returns `(index, pressed)` for every button whose state changed.
    pub fn update(&mut self, new: [bool; N]) -> Vec<(usize, bool)> {
        let edges = (0..N).filter(|&i| self.state[i] != new[i]).map(|i| (i, new[i])).collect();
        self.state = new;
        edges
    }

    /// Forget the state, e.g. after a reconnect.
    pub fn reset(&mut self) {
        self.state = [false; N];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(hex: &str) -> Vec<u8> {
        let mut v: Vec<u8> = hex.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect();
        v.resize(64, 0);
        v
    }

    // All vectors below were captured from real hardware.

    #[test]
    fn keypad() {
        let r = decode_input(ModuleKind::Keypad, &report("01 34 11 00 02 0f 00 01 00")).unwrap();
        let mut want = [false; 15];
        want[1] = true;
        assert_eq!(r, InputReport::Keys(want));
    }

    #[test]
    fn fader() {
        let r = decode_input(ModuleKind::Fader, &report("01 2a 07 00 02 05 00 56 64 64 64")).unwrap();
        assert_eq!(r, InputReport::Faders([0, 86, 100, 100, 100]));
    }

    #[test]
    fn roller() {
        let r = decode_input(ModuleKind::Roller, &report("01 2a 04 00 01 02 ff 01")).unwrap();
        assert_eq!(r, InputReport::Rollers { left: Some(Roll::Toward), right: None });
        let r = decode_input(ModuleKind::Roller, &report("01 2a 04 00 01 02 00 ff")).unwrap();
        assert_eq!(r, InputReport::Rollers { left: None, right: Some(Roll::Away) });
    }

    #[test]
    fn knob() {
        let r = decode_input(ModuleKind::Knob, &report("01 2a 05 00 00 03 ff ff 01")).unwrap();
        assert_eq!(r, InputReport::KnobTurn([None, None, Some(Rotation::Clockwise)]));
        let r = decode_input(ModuleKind::Knob, &report("01 2a 05 00 03 03 00 01 00")).unwrap();
        assert_eq!(r, InputReport::KnobButtons([false, true, false]));
    }

    #[test]
    fn dial() {
        let r = decode_input(ModuleKind::Dial, &report("01 44 03 00 03 01 00")).unwrap();
        assert_eq!(r, InputReport::DialTurn(Rotation::Clockwise));
        let r = decode_input(ModuleKind::Dial, &report("01 44 03 00 03 01 01")).unwrap();
        assert_eq!(r, InputReport::DialTurn(Rotation::CounterClockwise));
        let r = decode_input(ModuleKind::Dial, &report("01 44 03 00 04 01 01")).unwrap();
        assert_eq!(r, InputReport::DialButton { pressed: true });
    }

    #[test]
    fn base_and_garbage() {
        let r = decode_input(ModuleKind::Base, &report("01 2b 01 00 01")).unwrap();
        assert_eq!(r, InputReport::BaseOrientation { horizontal: true });
        assert!(decode_input(ModuleKind::Base, &[0x02, 0x00]).is_none());
        assert!(matches!(
            decode_input(ModuleKind::Fader, &report("01 2d 01 00 07")),
            Some(InputReport::Unknown { kind: 0x2d, .. })
        ));
    }

    #[test]
    fn tracker_edges() {
        let mut t = KeyTracker::<3>::default();
        assert_eq!(t.update([false, true, false]), vec![(1, true)]);
        assert_eq!(t.update([true, true, false]), vec![(0, true)]);
        assert_eq!(t.update([false, false, false]), vec![(0, false), (1, false)]);
    }
}

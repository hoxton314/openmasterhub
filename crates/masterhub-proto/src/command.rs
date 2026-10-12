use crate::{FEATURE_COMMAND, FEATURE_LEN, ModuleKind};

/// A feature SET report (`03 <cmd> args…`), zero-padded to [`FEATURE_LEN`].
///
/// On display modules the host must read feature report `0x03` back after sending it,
/// otherwise the command is acknowledged but not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureCommand(pub [u8; FEATURE_LEN]);

impl FeatureCommand {
    fn new(args: &[u8]) -> Self {
        let mut buf = [0u8; FEATURE_LEN];
        buf[0] = FEATURE_COMMAND;
        buf[1..=args.len()].copy_from_slice(args);
        Self(buf)
    }

    /// Switch a display module between software mode (host drives keys and screens, input is
    /// reported) and hardware mode (standalone animation, no input). Returns `None` for
    /// modules without a mode switch.
    ///
    /// Firmware quirk: once hardware mode (`false`) has been sent, the module ignores further
    /// switches to software mode until it is power-cycled (unplugged).
    pub fn sw_mode(kind: ModuleKind, software: bool) -> Option<Self> {
        let cmd = match kind {
            ModuleKind::Keypad => 0x3A,
            ModuleKind::Dial => 0x49,
            _ => return None,
        };
        Some(Self::new(&[cmd, software as u8]))
    }

    /// LCD backlight, 0..=100 (clamped).
    pub fn backlight(kind: ModuleKind, level: u8) -> Option<Self> {
        let cmd = match kind {
            ModuleKind::Keypad => 0x34,
            ModuleKind::Dial => 0x44,
            _ => return None,
        };
        Some(Self::new(&[cmd, level.min(100)]))
    }

    /// Fill one keypad key with a solid colour.
    pub fn key_color(key: u8, rgb: Rgb) -> Self {
        Self::new(&[0x37, key, rgb.r, rgb.g, rgb.b])
    }

    /// Fill a whole display (keypad panel or dial) with a solid colour.
    pub fn screen_color(kind: ModuleKind, rgb: Rgb) -> Option<Self> {
        let cmd = match kind {
            ModuleKind::Keypad => 0x36,
            ModuleKind::Dial => 0x46,
            _ => return None,
        };
        Some(Self::new(&[cmd, rgb.r, rgb.g, rgb.b]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// LED effect modes of the M0 modules (Base, Fader, Knob).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LedMode {
    Off = 0,
    Static = 1,
    Breathing = 2,
    Strobe = 3,
    Spring = 4,
    Cycling = 5,
    Random = 6,
    Wave = 7,
    Water = 8,
    Rainbow = 9,
}

/// Which LED of a module to address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedTarget {
    All,
    Index(u8),
}

impl LedTarget {
    fn byte(self) -> u8 {
        match self {
            Self::All => 0xFF,
            Self::Index(i) => i,
        }
    }
}

/// Output reports for the LED-capable M0 modules (64 bytes incl. ID). `None` if the module
/// has no LEDs. Nothing is persisted to flash (save flag 0).
pub fn led_mode(kind: ModuleKind, target: LedTarget, mode: LedMode) -> Option<Vec<u8>> {
    let g = kind.led_group()?;
    Some(output(kind, &[0x2C, g, target.byte(), mode as u8, 0]))
}

pub fn led_color(kind: ModuleKind, target: LedTarget, rgb: Rgb) -> Option<Vec<u8>> {
    let g = kind.led_group()?;
    Some(output(kind, &[0x2D, g, target.byte(), rgb.r, rgb.g, rgb.b, 0]))
}

/// LED brightness, 1..=100 (clamped).
pub fn led_brightness(kind: ModuleKind, target: LedTarget, level: u8) -> Option<Vec<u8>> {
    let g = kind.led_group()?;
    Some(output(kind, &[0x2E, g, target.byte(), level.clamp(1, 100)]))
}

fn output(kind: ModuleKind, args: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; kind.output_len()];
    buf[0] = 0x02;
    buf[1..=args.len()].copy_from_slice(args);
    buf
}

/// Where an image goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTarget {
    /// A keypad key, 0..=14 (row-major from top-left). Image must be a 72×72 JPEG.
    Key(u8),
    /// The dial screen. Image must be a 240×240 JPEG.
    Dial,
}

/// Bytes of image data carried by one image packet.
pub const IMAGE_CHUNK: usize = 312;

/// Split a JPEG into the 320-byte output reports that display it:
/// `02 <cmd> seq:u16le len:u16le <key> <last> data[..312]`.
pub fn image_packets(target: ImageTarget, jpeg: &[u8]) -> Vec<[u8; 320]> {
    let (cmd, key) = match target {
        ImageTarget::Key(k) => (0x36, k),
        ImageTarget::Dial => (0x45, 0),
    };
    let count = jpeg.len().div_ceil(IMAGE_CHUNK).max(1);
    (0..count)
        .map(|seq| {
            let part = &jpeg[(seq * IMAGE_CHUNK).min(jpeg.len())..((seq + 1) * IMAGE_CHUNK).min(jpeg.len())];
            let mut pkt = [0u8; 320];
            pkt[0] = 0x02;
            pkt[1] = cmd;
            pkt[2..4].copy_from_slice(&(seq as u16).to_le_bytes());
            pkt[4..6].copy_from_slice(&(part.len() as u16).to_le_bytes());
            pkt[6] = key;
            pkt[7] = (seq + 1 == count) as u8;
            pkt[8..8 + part.len()].copy_from_slice(part);
            pkt
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sw_mode_bytes() {
        let c = FeatureCommand::sw_mode(ModuleKind::Keypad, true).unwrap();
        assert_eq!(&c.0[..4], &[0x03, 0x3A, 0x01, 0x00]);
        assert_eq!(c.0.len(), 32);
        let c = FeatureCommand::sw_mode(ModuleKind::Dial, true).unwrap();
        assert_eq!(&c.0[..3], &[0x03, 0x49, 0x01]);
        assert!(FeatureCommand::sw_mode(ModuleKind::Fader, true).is_none());
    }

    #[test]
    fn led_bytes_match_hardware_test() {
        // Sent on hardware: fader top LED red, base red.
        let m = led_mode(ModuleKind::Fader, LedTarget::Index(0), LedMode::Static).unwrap();
        assert_eq!(&m[..6], &[0x02, 0x2C, 1, 0, 1, 0]);
        assert_eq!(m.len(), 64);
        let c = led_color(ModuleKind::Base, LedTarget::Index(0), Rgb::new(255, 0, 0)).unwrap();
        assert_eq!(&c[..8], &[0x02, 0x2D, 0, 0, 255, 0, 0, 0]);
        let b = led_brightness(ModuleKind::Knob, LedTarget::All, 200).unwrap();
        assert_eq!(&b[..5], &[0x02, 0x2E, 2, 0xFF, 100]);
        assert!(led_color(ModuleKind::Roller, LedTarget::All, Rgb::default()).is_none());
    }

    #[test]
    fn image_chunking() {
        let jpeg: Vec<u8> = (0..1680u32).map(|i| i as u8).collect();
        let p = image_packets(ImageTarget::Key(3), &jpeg);
        assert_eq!(p.len(), 6); // matches the 6 packets seen on hardware
        assert_eq!(&p[0][..8], &[0x02, 0x36, 0, 0, 0x38, 0x01, 3, 0]);
        let last = &p[5];
        assert_eq!(&last[..8], &[0x02, 0x36, 5, 0, 120, 0, 3, 1]);
        assert_eq!(last[8], jpeg[1560]);
        assert!(last[8 + 120..].iter().all(|&b| b == 0));

        let d = image_packets(ImageTarget::Dial, &[0xFF; 10863]);
        assert_eq!(d.len(), 35); // matches hardware test
        assert_eq!(&d[0][..2], &[0x02, 0x45]);
        assert_eq!(d[34][7], 1);
    }
}

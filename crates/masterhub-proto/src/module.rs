/// The kinds of module the MasterHUB can host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// The base board itself (one RGB LED zone, orientation sensor).
    Base,
    /// 15 keys with a 72×72 LCD each ("15xKey w/ IPS", SDK M4315).
    Keypad,
    /// 5 linear faders with one LED each ("5xFader").
    Fader,
    /// 3 rotary knobs with push and one LED each, in a triangle ("3xKnob").
    Knob,
    /// 2 rollers ("2xRoller").
    Roller,
    /// Big rotary dial with push and a 240×240 LCD ("1xDial w/ IPS", SDK M131E).
    Dial,
}

impl ModuleKind {
    /// Map a USB product ID to a module kind. Includes the vendor's alternate `0x99xx` IDs.
    pub fn from_pid(pid: u16) -> Option<Self> {
        Some(match pid {
            0x01F6 | 0x9999 => Self::Base,
            0x01FE | 0x9997 => Self::Keypad,
            0x0200 | 0x9991 => Self::Fader,
            0x0202 | 0x9996 => Self::Knob,
            0x0204 | 0x9995 => Self::Roller,
            0x0206 | 0x9994 => Self::Dial,
            _ => return None,
        })
    }

    /// Size of an output report including the report ID.
    pub fn output_len(self) -> usize {
        match self {
            Self::Keypad | Self::Dial => 320,
            _ => 64,
        }
    }

    /// LED group used by the M0 LED commands, if the module has addressable LEDs.
    pub fn led_group(self) -> Option<u8> {
        match self {
            Self::Base => Some(0),
            Self::Fader => Some(1),
            Self::Knob => Some(2),
            _ => None,
        }
    }

    /// Number of individually addressable LEDs.
    pub fn led_count(self) -> u8 {
        match self {
            Self::Base => 1,
            Self::Fader => 5,
            Self::Knob => 3,
            _ => 0,
        }
    }

    /// Whether the module needs software mode before it reports input and accepts images.
    pub fn has_sw_mode(self) -> bool {
        matches!(self, Self::Keypad | Self::Dial)
    }

    /// Feature report ID of the device-info query, for display modules.
    pub fn device_info_report(self) -> Option<u8> {
        match self {
            Self::Keypad => Some(0x3B),
            Self::Dial => Some(0x4B),
            _ => None,
        }
    }

    /// Feature report ID returning the application firmware version.
    pub fn firmware_report(self) -> u8 {
        match self {
            Self::Keypad => 0x38,
            Self::Dial => 0x48,
            _ => 0x2E,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Keypad => "keypad",
            Self::Fader => "fader",
            Self::Knob => "knob",
            Self::Roller => "roller",
            Self::Dial => "dial",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pids() {
        assert_eq!(ModuleKind::from_pid(0x01FE), Some(ModuleKind::Keypad));
        assert_eq!(ModuleKind::from_pid(0x0206), Some(ModuleKind::Dial));
        assert_eq!(ModuleKind::from_pid(0x9991), Some(ModuleKind::Fader));
        assert_eq!(ModuleKind::from_pid(0x1234), None);
    }
}

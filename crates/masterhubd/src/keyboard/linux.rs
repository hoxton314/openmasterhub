//! uinput virtual keyboard.

use std::str::FromStr;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, KeyCode, KeyEvent};

pub type Key = KeyCode;

/// Canonical name → evdev key code (`playpause` → `KEY_PLAYPAUSE`).
pub fn key(name: &str) -> Option<Key> {
    KeyCode::from_str(&format!("KEY_{}", name.to_ascii_uppercase())).ok()
}

pub struct Device(VirtualDevice);

impl Device {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        // KEY_ESC .. KEY_MICMUTE covers the keyboard and media keys.
        for code in 1..=248 {
            keys.insert(KeyCode::new(code));
        }
        let dev = VirtualDevice::builder()?.name("openmasterhub virtual keyboard").with_keys(&keys)?.build()?;
        // Give the compositor a moment to pick up the new device before the first key.
        thread::sleep(Duration::from_millis(200));
        Ok(Self(dev))
    }

    pub fn tap(&mut self, keys: &[Key]) -> Result<()> {
        for k in keys {
            self.0.emit(&[*KeyEvent::new(*k, 1)])?;
        }
        for k in keys.iter().rev() {
            self.0.emit(&[*KeyEvent::new(*k, 0)])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        assert_eq!(key("leftctrl"), Some(KeyCode::KEY_LEFTCTRL));
        assert_eq!(key("playpause"), Some(KeyCode::KEY_PLAYPAUSE));
        assert_eq!(key("f13"), Some(KeyCode::KEY_F13));
        assert_eq!(key("nosuchkey"), None);
    }
}

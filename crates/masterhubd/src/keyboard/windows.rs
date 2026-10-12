//! `SendInput` keyboard. Key names map to Windows virtual-key codes.

use std::mem::size_of;

use anyhow::{Result, bail};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput,
};

/// Virtual-key code and whether it is an "extended" key (arrows, navigation block, right-hand
/// modifiers), which Windows needs flagged to tell it from the numpad equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    vk: u16,
    extended: bool,
}

pub fn key(name: &str) -> Option<Key> {
    let k = |vk: u16| Some(Key { vk, extended: false });
    let ext = |vk: u16| Some(Key { vk, extended: true });
    let b = name.as_bytes();
    match name {
        // a..z, 0..9
        _ if b.len() == 1 && b[0].is_ascii_lowercase() => k(b[0].to_ascii_uppercase() as u16),
        _ if b.len() == 1 && b[0].is_ascii_digit() => k(b[0] as u16),
        // f1..f24
        _ if name.starts_with('f') && name[1..].parse::<u16>().is_ok_and(|n| (1..=24).contains(&n)) => {
            k(0x70 + name[1..].parse::<u16>().ok()? - 1)
        }
        // kp0..kp9
        _ if name.starts_with("kp") && name.len() == 3 && b[2].is_ascii_digit() => k(0x60 + (b[2] - b'0') as u16),
        "leftctrl" => k(0xA2),
        "rightctrl" => ext(0xA3),
        "leftshift" => k(0xA0),
        "rightshift" => k(0xA1),
        "leftalt" => k(0xA4),
        "rightalt" => ext(0xA5),
        "leftmeta" => ext(0x5B),
        "rightmeta" => ext(0x5C),
        "compose" | "menu" => ext(0x5D),
        "esc" => k(0x1B),
        "enter" => k(0x0D),
        "kpenter" => ext(0x0D),
        "tab" => k(0x09),
        "space" => k(0x20),
        "backspace" => k(0x08),
        "capslock" => k(0x14),
        "numlock" => ext(0x90),
        "scrolllock" => k(0x91),
        "pause" => k(0x13),
        "sysrq" | "print" => ext(0x2C),
        "insert" => ext(0x2D),
        "delete" => ext(0x2E),
        "home" => ext(0x24),
        "end" => ext(0x23),
        "pageup" => ext(0x21),
        "pagedown" => ext(0x22),
        "up" => ext(0x26),
        "down" => ext(0x28),
        "left" => ext(0x25),
        "right" => ext(0x27),
        "minus" => k(0xBD),
        "equal" => k(0xBB),
        "leftbrace" => k(0xDB),
        "rightbrace" => k(0xDD),
        "backslash" => k(0xDC),
        "semicolon" => k(0xBA),
        "apostrophe" => k(0xDE),
        "grave" => k(0xC0),
        "comma" => k(0xBC),
        "dot" => k(0xBE),
        "slash" => k(0xBF),
        "kpplus" => k(0x6B),
        "kpminus" => k(0x6D),
        "kpasterisk" => k(0x6A),
        "kpslash" => ext(0x6F),
        "kpdot" => k(0x6E),
        // Media and volume keys.
        "mute" => k(0xAD),
        "volumedown" => k(0xAE),
        "volumeup" => k(0xAF),
        "nextsong" => k(0xB0),
        "previoussong" => k(0xB1),
        "stopcd" => k(0xB2),
        "playpause" => k(0xB3),
        "mail" => k(0xB4),
        "www" | "homepage" => k(0xAC),
        "calc" => k(0xB7),
        _ => None,
    }
}

pub struct Device;

impl Device {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn tap(&mut self, keys: &[Key]) -> Result<()> {
        let event = |key: &Key, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key.vk,
                    wScan: 0,
                    dwFlags: if key.extended { KEYEVENTF_EXTENDEDKEY } else { 0 } | if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let inputs: Vec<INPUT> =
            keys.iter().map(|k| event(k, false)).chain(keys.iter().rev().map(|k| event(k, true))).collect();
        // SAFETY: `inputs` is a valid array of INPUT structures of the given length and size.
        let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            bail!("SendInput sent {sent} of {} events (blocked by a higher-privileged window?)", inputs.len());
        }
        Ok(())
    }
}

//! Virtual keyboard for `keys` actions. Key names are the same on every OS: the Linux
//! `input-event-codes.h` names without `KEY_`, lower-case (`a`, `f13`, `leftctrl`, `playpause`,
//! `volumeup`, …), plus the aliases below.

use anyhow::{Result, bail};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;

/// Sends key combinations; the OS device is created on first use.
pub struct Keyboard {
    device: Option<imp::Device>,
}

impl Keyboard {
    pub fn new() -> Self {
        Self { device: None }
    }

    /// Press the keys of `combo` in order, then release them in reverse.
    pub fn send(&mut self, combo: &str) -> Result<()> {
        let keys = parse(combo)?;
        if self.device.is_none() {
            self.device = Some(imp::Device::new()?);
        }
        self.device.as_mut().expect("just created").tap(&keys)
    }
}

/// Check a combination without sending it (config validation).
pub fn validate(combo: &str) -> Result<()> {
    parse(combo).map(|_| ())
}

fn parse(combo: &str) -> Result<Vec<imp::Key>> {
    let keys = combo
        .split('+')
        .map(|part| {
            let name = canonical(part);
            imp::key(&name).ok_or_else(|| anyhow::anyhow!("unknown key {part:?} in {combo:?}"))
        })
        .collect::<Result<Vec<_>>>()?;
    if keys.is_empty() {
        bail!("empty key combination");
    }
    Ok(keys)
}

/// Lower-case and resolve aliases to the canonical key name.
fn canonical(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    match name.as_str() {
        "ctrl" | "control" => "leftctrl",
        "shift" => "leftshift",
        "alt" => "leftalt",
        "altgr" => "rightalt",
        "super" | "meta" | "win" | "cmd" => "leftmeta",
        "return" => "enter",
        "del" => "delete",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        "escape" => "esc",
        other => other,
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases() {
        assert_eq!(canonical(" Ctrl"), "leftctrl");
        assert_eq!(canonical("WIN"), "leftmeta");
        assert_eq!(canonical("playpause"), "playpause");
    }

    #[test]
    fn combos() {
        assert_eq!(parse("ctrl+Shift+m").unwrap().len(), 3);
        assert!(parse("playpause").is_ok());
        assert!(parse("super+f13").is_ok());
        assert!(parse("ctrl+nosuchkey").is_err());
        assert!(parse("").is_err());
    }
}

//! Global shortcuts from `[[setup]]` (`shortcut`, `next`, `prev`, `pin`), through
//! tauri-plugin-global-shortcut: X11 and Windows. Wayland has no global shortcuts for apps, so
//! there the user binds `masterhubd ctl setup|profile|pin ...` in the compositor (the UI says so).
//!
//! Key names are the service's (`ctrl+alt+1`, `f13`, `playpause`), not the plugin's, so the
//! combination means the same here as in a `keys` action.

use std::collections::HashMap;

use masterhub_core::config::{Action, PinAction, Setup};
use masterhub_core::ipc::Request;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::AppState;

/// What a shortcut does.
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// Switch to this setup.
    Setup(String),
    /// Next or previous profile, while this (manual) setup is active.
    Cycle { setup: String, next: bool },
    /// Pin the live profile, or end the pin, while this (hybrid) setup is active.
    Pin(String),
}

/// A parsed combination and, when the tray can show it natively, its accelerator text.
#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub shortcut: Shortcut,
    pub accelerator: Option<String>,
}

fn modifier(name: &str) -> Option<(Modifiers, &'static str)> {
    Some(match name {
        "ctrl" | "control" | "leftctrl" | "rightctrl" => (Modifiers::CONTROL, "Ctrl"),
        "shift" | "leftshift" | "rightshift" => (Modifiers::SHIFT, "Shift"),
        "alt" | "leftalt" => (Modifiers::ALT, "Alt"),
        "super" | "meta" | "win" | "cmd" | "leftmeta" | "rightmeta" => (Modifiers::SUPER, "Super"),
        _ => return None,
    })
}

/// The service's key name as a physical key, with the token the tray's accelerator parser knows
/// (none for keys it does not).
fn key(name: &str) -> Option<(Code, Option<String>)> {
    let b = name.as_bytes();
    if b.len() == 1 && b[0].is_ascii_lowercase() {
        let code = [Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI, Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM, Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR, Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ][(b[0] - b'a') as usize];
        return Some((code, Some(name.to_ascii_uppercase())));
    }
    if b.len() == 1 && b[0].is_ascii_digit() {
        let code = [Code::Digit0, Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5, Code::Digit6, Code::Digit7, Code::Digit8, Code::Digit9][(b[0] - b'0') as usize];
        return Some((code, Some(name.to_owned())));
    }
    if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=24).contains(n)) {
        let codes = [Code::F1, Code::F2, Code::F3, Code::F4, Code::F5, Code::F6, Code::F7, Code::F8, Code::F9, Code::F10, Code::F11, Code::F12, Code::F13, Code::F14, Code::F15, Code::F16, Code::F17, Code::F18, Code::F19, Code::F20, Code::F21, Code::F22, Code::F23, Code::F24];
        return Some((codes[n - 1], Some(format!("F{n}"))));
    }
    if let Some(n) = name.strip_prefix("kp").and_then(|n| n.parse::<usize>().ok()).filter(|n| *n <= 9) {
        let codes = [Code::Numpad0, Code::Numpad1, Code::Numpad2, Code::Numpad3, Code::Numpad4, Code::Numpad5, Code::Numpad6, Code::Numpad7, Code::Numpad8, Code::Numpad9];
        return Some((codes[n], None));
    }
    let (code, token) = match name {
        "esc" | "escape" => (Code::Escape, Some("Escape")),
        "enter" | "return" => (Code::Enter, Some("Enter")),
        "tab" => (Code::Tab, Some("Tab")),
        "space" => (Code::Space, Some("Space")),
        "backspace" => (Code::Backspace, Some("Backspace")),
        "delete" | "del" => (Code::Delete, Some("Delete")),
        "insert" => (Code::Insert, Some("Insert")),
        "home" => (Code::Home, Some("Home")),
        "end" => (Code::End, Some("End")),
        "pageup" | "pgup" => (Code::PageUp, Some("PageUp")),
        "pagedown" | "pgdn" => (Code::PageDown, Some("PageDown")),
        "up" => (Code::ArrowUp, Some("Up")),
        "down" => (Code::ArrowDown, Some("Down")),
        "left" => (Code::ArrowLeft, Some("Left")),
        "right" => (Code::ArrowRight, Some("Right")),
        "minus" => (Code::Minus, None),
        "equal" => (Code::Equal, None),
        "leftbrace" => (Code::BracketLeft, None),
        "rightbrace" => (Code::BracketRight, None),
        "backslash" => (Code::Backslash, None),
        "semicolon" => (Code::Semicolon, None),
        "apostrophe" => (Code::Quote, None),
        "grave" => (Code::Backquote, None),
        "comma" => (Code::Comma, None),
        "dot" => (Code::Period, None),
        "slash" => (Code::Slash, None),
        "capslock" => (Code::CapsLock, None),
        "pause" => (Code::Pause, None),
        "print" | "sysrq" => (Code::PrintScreen, None),
        "playpause" => (Code::MediaPlayPause, None),
        "nextsong" => (Code::MediaTrackNext, None),
        "previoussong" => (Code::MediaTrackPrevious, None),
        "stopcd" => (Code::MediaStop, None),
        "volumeup" => (Code::AudioVolumeUp, None),
        "volumedown" => (Code::AudioVolumeDown, None),
        "mute" => (Code::AudioVolumeMute, None),
        _ => return None,
    };
    Some((code, token.map(str::to_owned)))
}

/// `ctrl+alt+1` and the like: modifiers, then exactly one other key.
pub fn parse(text: &str) -> Result<Parsed, String> {
    let mut mods = Modifiers::empty();
    let mut accel: Vec<String> = Vec::new();
    let mut main: Option<(Code, Option<String>)> = None;
    for part in text.split('+') {
        let name = part.trim().to_ascii_lowercase();
        if let Some((m, label)) = modifier(&name) {
            mods |= m;
            accel.push(label.to_owned());
        } else if main.is_some() {
            return Err(format!("{text:?} has more than one key"));
        } else {
            main = Some(key(&name).ok_or_else(|| format!("unknown key {part:?} in {text:?}"))?);
        }
    }
    let (code, token) = main.ok_or_else(|| format!("{text:?} has no key, only modifiers"))?;
    if mods.is_empty() && !matches!(code, Code::F13 | Code::F14 | Code::F15 | Code::F16 | Code::F17 | Code::F18 | Code::F19 | Code::F20 | Code::F21 | Code::F22 | Code::F23 | Code::F24 | Code::MediaPlayPause | Code::MediaTrackNext | Code::MediaTrackPrevious | Code::MediaStop | Code::AudioVolumeUp | Code::AudioVolumeDown | Code::AudioVolumeMute) {
        // A bare letter would be taken from every program.
        return Err(format!("{text:?} needs a modifier (Ctrl, Alt, ...)"));
    }
    let accelerator = token.map(|t| {
        accel.push(t);
        accel.join("+")
    });
    Ok(Parsed { shortcut: Shortcut::new((!mods.is_empty()).then_some(mods), code), accelerator })
}

/// "Ctrl Alt F12", as the UI writes shortcuts.
pub fn pretty(text: &str) -> String {
    text.split('+')
        .map(|p| {
            let p = p.trim();
            match modifier(&p.to_ascii_lowercase()) {
                Some((_, label)) => label.to_owned(),
                None if p.len() == 1 => p.to_ascii_uppercase(),
                None => {
                    let mut c = p.chars();
                    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The shortcuts of every setup, in config order; the first of two equal combinations wins.
pub fn bindings(setups: &[Setup]) -> Vec<(String, Binding)> {
    let mut out: Vec<(String, Binding)> = Vec::new();
    let mut add = |text: &Option<String>, binding: Binding| {
        if let Some(t) = text.as_deref().filter(|t| !t.trim().is_empty()) {
            out.push((t.to_owned(), binding));
        }
    };
    for s in setups {
        add(&s.shortcut, Binding::Setup(s.name.clone()));
        add(&s.next, Binding::Cycle { setup: s.name.clone(), next: true });
        add(&s.prev, Binding::Cycle { setup: s.name.clone(), next: false });
        add(&s.pin, Binding::Pin(s.name.clone()));
    }
    out
}

/// The session has no way to grab keys for every program.
pub fn wayland() -> bool {
    cfg!(target_os = "linux") && (std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland") || std::env::var_os("WAYLAND_DISPLAY").is_some())
}

/// Install the plugin (not on Wayland: grabbing keys there fails, and the plugin's own setup can
/// refuse to start).
pub fn init(app: &AppHandle) {
    if wayland() {
        log::info!("Wayland: no global shortcuts; bind `masterhubd ctl setup|profile|pin` in your compositor");
        return;
    }
    let plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let (app, id) = (app.clone(), shortcut.id());
                std::thread::spawn(move || fire(&app, id));
            }
        })
        .build();
    match app.plugin(plugin) {
        Ok(()) => app.state::<AppState>().shortcuts_ready.store(true, std::sync::atomic::Ordering::Relaxed),
        Err(e) => log::warn!("global shortcuts are not available: {e}"),
    }
}

/// Register what the config's setups define, replacing what was registered.
pub fn sync(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.shortcuts_ready.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let setups = state.live.lock().unwrap().config.as_ref().map(|c| c.effective_setups()).unwrap_or_default();
    let gs = app.global_shortcut();
    if let Err(e) = gs.unregister_all() {
        log::warn!("shortcuts: {e}");
    }
    let mut map: HashMap<u32, (String, Binding)> = HashMap::new();
    for (text, binding) in bindings(&setups) {
        let parsed = match parse(&text) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("shortcut: {e}");
                continue;
            }
        };
        let id = parsed.shortcut.id();
        if map.contains_key(&id) {
            log::warn!("shortcut {text:?} is used twice; the first one wins");
            continue;
        }
        match gs.register(parsed.shortcut) {
            Ok(()) => {
                map.insert(id, (text, binding));
            }
            // Often another program has it.
            Err(e) => log::warn!("shortcut {text:?} could not be registered: {e}"),
        }
    }
    *state.bindings.lock().unwrap() = map;
}

fn fire(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    let Some((text, binding)) = state.bindings.lock().unwrap().get(&id).cloned() else { return };
    let active = state.live.lock().unwrap().state.as_ref().and_then(|s| s.setup.clone());
    match binding {
        Binding::Setup(name) => {
            state.live.lock().unwrap().trigger = Some((std::time::Instant::now(), pretty(&text)));
            crate::system::fire(&state, Request::SetSetup { name }, "setup shortcut");
        }
        Binding::Cycle { setup, next } => {
            if active.as_deref() == Some(setup.as_str()) {
                let dir = if next { "next" } else { "prev" };
                crate::system::fire(&state, Request::RunAction { action: Action::Profile(dir.into()), value: None }, "profile shortcut");
            }
        }
        // `pin = "toggle"` pins the live profile and, with the `same_trigger` release policy,
        // lets the same shortcut release it again.
        Binding::Pin(setup) => {
            if active.as_deref() == Some(setup.as_str()) {
                crate::system::fire(&state, Request::RunAction { action: Action::Pin(PinAction::Toggle), value: None }, "pin shortcut");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_service_key_names() {
        let p = parse("ctrl+alt+1").unwrap();
        assert_eq!(p.shortcut, Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit1));
        assert_eq!(p.accelerator.as_deref(), Some("Ctrl+Alt+1"));
        assert_eq!(parse("Ctrl+Shift+F12").unwrap().shortcut, Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F12));
        assert_eq!(parse("super+leftbrace").unwrap().accelerator, None);
        assert_eq!(parse("f13").unwrap().shortcut, Shortcut::new(None, Code::F13));
        assert!(parse("playpause").is_ok());
    }

    #[test]
    fn rejects_unusable_combinations() {
        assert!(parse("ctrl+nosuchkey").is_err());
        assert!(parse("ctrl+alt").is_err());
        assert!(parse("ctrl+a+b").is_err());
        assert!(parse("a").is_err());
    }

    #[test]
    fn writes_like_the_ui() {
        assert_eq!(pretty("ctrl+alt+f12"), "Ctrl Alt F12");
        assert_eq!(pretty("super+1"), "Super 1");
    }

    #[test]
    fn collects_the_shortcuts_of_every_setup() {
        let setups = vec![
            Setup { name: "Work".into(), shortcut: Some("ctrl+alt+1".into()), ..Default::default() },
            Setup { name: "Stream".into(), shortcut: Some("ctrl+alt+2".into()), next: Some("ctrl+alt+f12".into()), prev: Some(" ".into()), pin: None, ..Default::default() },
        ];
        let b = bindings(&setups);
        assert_eq!(b.len(), 3);
        assert_eq!(b[2], ("ctrl+alt+f12".to_owned(), Binding::Cycle { setup: "Stream".into(), next: true }));
    }
}

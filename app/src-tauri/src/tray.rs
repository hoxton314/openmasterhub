//! The tray icon and its menu (S30): what is live and why, the setups (with their shortcuts), the
//! profiles of a manual setup, pause, open and quit. Rebuilt whenever the live state, the config
//! or the connection changes. Clicking the icon opens the window; closing the window leaves the
//! app in the tray (`[app] close_to_tray = false` turns that off).
//!
//! Needs the system tray library on Linux (`libayatana-appindicator`); without it the app runs
//! without a tray and the window's close button quits.

use std::sync::atomic::Ordering;

use masterhub_core::config::SetupMode;
use masterhub_core::ipc::Reason;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::AppState;
use crate::live::{Live, describe};

const ID: &str = "main";

/// Create the icon, if the platform lets us.
pub fn create(app: &AppHandle) {
    let Some(icon) = app.default_window_icon().cloned() else { return };
    // The Linux tray library is loaded at run time and panics when it is missing.
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let menu = build_menu(app).ok();
        let mut b = TrayIconBuilder::with_id(ID)
            .icon(icon)
            .tooltip("OpenMasterHUB")
            .show_menu_on_left_click(false)
            .on_menu_event(on_menu_event)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    show_main(tray.app_handle());
                }
            });
        if let Some(menu) = &menu {
            b = b.menu(menu);
        }
        b.build(app)
    }));
    match built {
        Ok(Ok(_)) => app.state::<AppState>().tray_ready.store(true, Ordering::Relaxed),
        Ok(Err(e)) => log::warn!("tray icon: {e}"),
        Err(_) => log::warn!("tray icon: the system tray library is missing (libayatana-appindicator); running without a tray"),
    }
}

/// Bring the menu up to date.
pub fn update(app: &AppHandle) {
    if !app.state::<AppState>().tray_ready.load(Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    // Menus belong to the main thread.
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(ID) else { return };
        match build_menu(&handle) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(e) => log::warn!("tray menu: {e}"),
        }
        let state = handle.state::<AppState>();
        let live = state.live.lock().unwrap();
        let tip = match (&live.config, &live.state) {
            (Some(c), Some(s)) => format!("OpenMasterHUB: {}", describe(c, s).title),
            _ => "OpenMasterHUB".to_owned(),
        };
        let _ = tray.set_tooltip(Some(tip));
    });
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// One line of the menu: an item that can be disabled and shows a shortcut.
struct Line {
    id: String,
    text: String,
    enabled: bool,
    checked: Option<bool>,
    accelerator: Option<String>,
}

fn line(id: impl Into<String>, text: impl Into<String>) -> Line {
    Line { id: id.into(), text: text.into(), enabled: true, checked: None, accelerator: None }
}

fn note(text: impl Into<String>) -> Line {
    Line { enabled: false, ..line("", text) }
}

/// What the menu shows, apart from the widgets: easy to test, and the same for every platform.
enum Entry {
    Line(Line),
    Separator,
}

fn entries(live: &Live, online: bool) -> Vec<Entry> {
    use Entry::{Line as L, Separator};
    let mut out = Vec::new();
    match (&live.config, &live.state, online) {
        (Some(config), Some(state), true) => {
            let d = describe(config, state);
            out.push(L(note("Live on the hub")));
            out.push(L(note(d.title)));
            if !d.reason.is_empty() {
                out.push(L(note(d.reason)));
            }
            out.push(Separator);
            out.push(L(note("Setup")));
            let setups = config.effective_setups();
            let active = state.setup.clone().unwrap_or_default();
            for s in &setups {
                let accelerator = s.shortcut.as_deref().and_then(|t| crate::shortcuts::parse(t).ok()).and_then(|p| p.accelerator);
                out.push(L(Line { checked: Some(s.name == active), accelerator, ..line(format!("setup:{}", s.name), s.name.clone()) }));
            }
            out.push(Separator);
            let current = setups.iter().find(|s| s.name == active);
            match current.map(|s| (s, s.mode())) {
                Some((s, SetupMode::Manual)) => {
                    out.push(L(note("Profile")));
                    for (i, p) in s.profiles.iter().enumerate() {
                        let accelerator = (i < 9).then(|| (i + 1).to_string());
                        out.push(L(Line { checked: Some(*p == state.profile), accelerator, ..line(format!("profile:{p}"), p.clone()) }));
                    }
                }
                other => {
                    out.push(L(note("Profile (manual setups only)")));
                    out.push(L(note(format!("{} follows apps", other.map_or(active.as_str(), |(s, _)| s.name.as_str())))));
                    if matches!(other, Some((_, SetupMode::Hybrid))) && live.has("pin") {
                        if state.pinned.is_some() || state.reason == Some(Reason::Pin) {
                            out.push(L(line("release", "Release the pinned profile")));
                        } else {
                            out.push(L(line("pin", format!("Pin {}", state.profile))));
                        }
                    }
                }
            }
            out.push(Separator);
            out.push(L(Line { checked: Some(state.paused), enabled: live.has("pause"), ..line("pause", "Pause hub actions") }));
        }
        (_, _, true) => out.push(L(note("Reading the hub..."))),
        _ => {
            out.push(L(note("The service is not running")));
            out.push(L(line("start", "Start the service")));
        }
    }
    out.push(L(line("open", "Open OpenMasterHUB")));
    out.push(L(line("quit", "Quit")));
    out
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let online = state.client.is_online();
    let items = entries(&state.live.lock().unwrap(), online);
    let mut menu = MenuBuilder::new(app);
    for e in items {
        match e {
            Entry::Separator => menu = menu.separator(),
            Entry::Line(l) => match l.checked {
                Some(checked) => {
                    let mut b = CheckMenuItemBuilder::with_id(&l.id, &l.text).checked(checked).enabled(l.enabled);
                    if let Some(a) = &l.accelerator {
                        b = b.accelerator(a);
                    }
                    // An accelerator the menu cannot parse must not lose the item.
                    let item = b.build(app).or_else(|_| CheckMenuItemBuilder::with_id(&l.id, &l.text).checked(checked).enabled(l.enabled).build(app))?;
                    menu = menu.item(&item);
                }
                None => {
                    let item = MenuItemBuilder::with_id(&l.id, &l.text).enabled(l.enabled).build(app)?;
                    menu = menu.item(&item);
                }
            },
        }
    }
    menu.build()
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_owned();
    match id.as_str() {
        "open" => return show_main(app),
        "quit" => return app.exit(0),
        _ => {}
    }
    // The service answers over a socket: never wait for it on the main thread.
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let client = &state.client;
        let r = match id.split_once(':') {
            Some(("setup", name)) => {
                state.live.lock().unwrap().trigger = Some((std::time::Instant::now(), "tray".into()));
                client.set_setup(name)
            }
            Some(("profile", name)) => client.set_profile(name),
            _ => match id.as_str() {
                "pin" => client.pin(None),
                "release" => client.release_pin(),
                "pause" => {
                    let paused = state.live.lock().unwrap().state.as_ref().is_some_and(|s| s.paused);
                    client.pause(!paused)
                }
                "start" => crate::launch_service(),
                _ => Ok(()),
            },
        };
        if let Err(e) = r {
            log::warn!("tray {id}: {e:#}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use masterhub_core::config::Config;
    use masterhub_core::ipc::HubState;

    fn live(setup: &str, profile: &str, reason: Reason) -> Live {
        let config = Config::parse(
            r#"
[[profile]]
name = "Desktop"
[[profile]]
name = "Live"
[[profile]]
name = "BRB"
[[setup]]
name = "Work"
fallback = "Desktop"
shortcut = "ctrl+alt+1"
[[setup]]
name = "Stream"
mode = "manual"
profiles = ["Live", "BRB"]
shortcut = "ctrl+alt+f12"
"#,
            std::path::Path::new("."),
        )
        .unwrap();
        Live {
            config: Some(config),
            state: Some(HubState { profile: profile.into(), setup: Some(setup.into()), reason: Some(reason), ..Default::default() }),
            capabilities: vec!["pause".into()],
            trigger: None,
        }
    }

    fn texts(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| match e {
                Entry::Separator => "---".to_owned(),
                Entry::Line(l) => format!("{}{}", match l.checked { Some(true) => "[x] ", Some(false) => "[ ] ", None => "" }, l.text),
            })
            .collect()
    }

    #[test]
    fn follow_setup_menu() {
        let t = texts(&entries(&live("Work", "Desktop", Reason::Fallback), true));
        assert_eq!(t[..4], ["Live on the hub", "Work · Desktop", "because no rule matches", "---"]);
        assert!(t.contains(&"[x] Work".to_owned()) && t.contains(&"[ ] Stream".to_owned()));
        assert!(t.contains(&"Work follows apps".to_owned()));
        assert_eq!(t[t.len() - 3..], ["[ ] Pause hub actions", "Open OpenMasterHUB", "Quit"]);
    }

    #[test]
    fn manual_setup_lists_its_profiles() {
        let t = texts(&entries(&live("Stream", "Live", Reason::Manual), true));
        assert!(t.contains(&"[x] Live".to_owned()) && t.contains(&"[ ] BRB".to_owned()), "{t:?}");
        assert!(t.contains(&"switched by hand · 1 of 2".to_owned()));
    }

    #[test]
    fn offline_menu_offers_to_start() {
        let t = texts(&entries(&Live::default(), false));
        assert_eq!(t, ["The service is not running", "Start the service", "Open OpenMasterHUB", "Quit"]);
    }
}

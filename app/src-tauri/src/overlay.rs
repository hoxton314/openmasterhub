//! The switch toast (S08): a small always-on-top window near the tray that says which profile
//! or setup just became live. It never takes focus, is skipped while a fullscreen window has
//! the foreground (unless `[app.feedback] in_fullscreen = true`) and is drawn by the same UI
//! and theme engine as the app (`overlay.html`). By default it only shows switches the user
//! made, not follow-mode changes caused by the focused window (`[app] overlay`).
//!
//! The window is exactly as big as the toast: the page measures its card and reports it with
//! `overlay_fit`, which sizes, places and shows the window. On Wayland a normal window cannot
//! be placed or kept on top, so there it is a wlr layer-shell surface (`layer.rs`); without
//! layer-shell (GNOME) no window is shown at all, the dial card and the LED still report.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use masterhub_core::ipc::HubState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;
use crate::live::{OverlayMode, describe};

const LABEL: &str = "overlay";
/// Logical size until the page reports the card's.
const WIDTH: f64 = 320.0;
const HEIGHT: f64 = 60.0;
const MARGIN: f64 = 12.0;
const PROFILE_SECS: f64 = 1.5;
const SETUP_SECS: f64 = 5.0;

/// How the window is shown, decided once in `create`.
const WINDOW: u8 = 0;
/// A wlr layer-shell surface (Wayland).
#[cfg(target_os = "linux")]
const LAYER: u8 = 1;
/// Wayland without layer-shell: nothing is shown.
const NONE: u8 = 2;
static BACKEND: AtomicU8 = AtomicU8::new(WINDOW);

#[derive(Serialize, Clone, Debug)]
struct Toast {
    /// Echoed by `overlay_fit`, so a late answer for an older toast is ignored.
    generation: u64,
    /// `profile` or `setup`.
    kind: &'static str,
    title: String,
    /// The line above the title.
    line: String,
    color: Option<String>,
    /// The setup to go back to, for a setup switch.
    undo: Option<String>,
    secs: f64,
    /// `[app]` theme settings, so the toast looks like the app.
    app: Option<serde_json::Value>,
}

/// Create the (hidden) window once at start, so a toast shows without waiting for a page load.
pub fn create(app: &AppHandle) {
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("OpenMasterHUB switch")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build();
    match built {
        Ok(window) => init_backend(&window),
        Err(e) => {
            log::warn!("switch overlay: {e}");
            BACKEND.store(NONE, Ordering::Relaxed);
        }
    }
}

/// On Wayland turn the (still unmapped) window into a layer-shell surface.
fn init_backend(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let gtk_window = match window.gtk_window() {
            Ok(w) => w,
            Err(e) => {
                log::warn!("switch overlay: {e}");
                BACKEND.store(NONE, Ordering::Relaxed);
                return;
            }
        };
        if !gtk_window.display().type_().name().contains("Wayland") {
            return;
        }
        match crate::layer::init(&gtk_window, MARGIN as i32) {
            Ok(()) => {
                BACKEND.store(LAYER, Ordering::Relaxed);
                warm_up(window);
            }
            Err(e) => {
                log::info!("switch overlay: not shown, no layer-shell here ({e}); the dial card and the LED still show switches");
                BACKEND.store(NONE, Ordering::Relaxed);
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = window;
}

/// A layer-shell surface that is mapped for the first time with its toast already in the page
/// stays blank (the webview paints nothing). Mapping it empty and transparent once at start,
/// long enough for a few frames, avoids that.
#[cfg(target_os = "linux")]
fn warm_up(window: &tauri::WebviewWindow) {
    let window = window.clone();
    std::thread::spawn(move || {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            let _ = w.show();
        });
        std::thread::sleep(Duration::from_millis(400));
        let w = window.clone();
        let _ = window.run_on_main_thread(move || {
            let _ = w.hide();
        });
    });
}

/// Whether a state change is a switch the toast is for, under `[app] overlay`.
fn wanted(mode: OverlayMode, prev: &HubState, new: &HubState, setup_changed: bool) -> bool {
    use masterhub_core::ipc::Reason;
    match mode {
        OverlayMode::Off => false,
        OverlayMode::All => true,
        OverlayMode::Switches => {
            // The user chose a setup, or let go of a pin.
            if setup_changed || (prev.pinned.is_some() && new.pinned.is_none()) {
                return true;
            }
            // Rules and the fallback are follow mode reacting to the focused window.
            !matches!(new.reason, Some(Reason::Rule { .. } | Reason::Fallback))
        }
    }
}

/// A live state change: show it when it was a switch the user should see.
pub fn on_change(app: &AppHandle, prev: Option<&HubState>, new: &HubState) {
    let Some(prev) = prev else { return };
    let setup_changed = new.setup.is_some() && prev.setup != new.setup;
    let profile_changed = !new.profile.is_empty() && prev.profile != new.profile;
    let pin_changed = prev.pinned.is_some() != new.pinned.is_some();
    if !setup_changed && !profile_changed && !pin_changed {
        return;
    }
    if BACKEND.load(Ordering::Relaxed) == NONE {
        return;
    }
    let state = app.state::<AppState>();
    let (toast, in_fullscreen) = {
        let live = state.live.lock().unwrap();
        if live.feedback(|f| f.toast) == Some(false) || !wanted(live.overlay_mode(), prev, new, setup_changed) {
            return;
        }
        let Some(config) = live.config.as_ref() else { return };
        let d = describe(config, new);
        let app_settings = config.app.clone();
        let theme = serde_json::to_value(&app_settings).ok();
        let toast = if setup_changed {
            let name = new.setup.clone().unwrap_or_default();
            let how = live.trigger.as_ref().filter(|(at, _)| at.elapsed() < Duration::from_secs(2)).map(|(_, t)| t.clone());
            Toast {
                generation: 0,
                kind: "setup",
                color: config.effective_setups().iter().find(|s| s.name == name).and_then(|s| s.color).map(|c| c.to_hex()),
                title: name,
                line: match how {
                    Some(how) => format!("Setup switched · {how}"),
                    None => "Setup switched".to_owned(),
                },
                undo: prev.setup.clone(),
                secs: SETUP_SECS,
                app: theme,
            }
        } else {
            let setup = new.setup.as_deref().unwrap_or_default();
            Toast {
                generation: 0,
                kind: "profile",
                color: config.profile(&new.profile).and_then(|p| p.color).map(|c| c.to_hex()),
                title: new.profile.clone(),
                line: [setup, d.reason.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · "),
                undo: None,
                secs: PROFILE_SECS,
                app: theme,
            }
        };
        (toast, live.feedback(|f| f.in_fullscreen) == Some(true))
    };
    // The app's own window is in front: it shows the change itself.
    if app.get_webview_window("main").is_some_and(|w| w.is_focused().unwrap_or(false)) {
        return;
    }
    if !in_fullscreen && fullscreen_in_front() {
        return;
    }
    show(app, toast);
}

fn show(app: &AppHandle, mut toast: Toast) {
    if app.get_webview_window(LABEL).is_none() {
        return;
    }
    let generation = app.state::<AppState>().overlay_generation.fetch_add(1, Ordering::Relaxed) + 1;
    toast.generation = generation;
    let secs = toast.secs;
    // The page draws the card, measures it and answers with `overlay_fit`, which shows the window.
    if let Err(e) = app.emit_to(LABEL, "overlay:show", toast) {
        log::warn!("switch overlay: {e}");
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f64(secs + 0.4));
        // A newer toast owns the window now.
        if handle.state::<AppState>().overlay_generation.load(Ordering::Relaxed) == generation {
            hide(&handle);
        }
    });
}

/// The page measured its card (logical px): size the window to it, place it and show it.
#[tauri::command]
pub fn overlay_fit(app: AppHandle, state: tauri::State<AppState>, generation: u64, width: f64, height: f64) {
    if state.overlay_generation.load(Ordering::Relaxed) != generation || !(8.0..=1200.0).contains(&width) || !(8.0..=1200.0).contains(&height) {
        return;
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(LABEL) else { return };
        let _ = window.set_size(LogicalSize::new(width.ceil(), height.ceil()));
        // The compositor places a layer-shell surface by its anchor and margins.
        if BACKEND.load(Ordering::Relaxed) == WINDOW {
            place(&handle, &window, width.ceil(), height.ceil());
        }
        let _ = window.show();
    });
}

pub fn hide(app: &AppHandle) {
    // A late `overlay_fit` for the toast that just went away must not show the window again.
    app.state::<AppState>().overlay_generation.fetch_add(1, Ordering::Relaxed);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(LABEL) {
            let _ = w.hide();
        }
    });
}

/// Bottom-right of the primary monitor's work area (above the Windows taskbar).
fn place(app: &AppHandle, window: &tauri::WebviewWindow, width: f64, height: f64) {
    let Ok(Some(monitor)) = app.primary_monitor() else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (w, h, m) = ((width * scale) as i32, (height * scale) as i32, (MARGIN * scale) as i32);
    let x = area.position.x + area.size.width as i32 - w - m;
    let y = area.position.y + area.size.height as i32 - h - m;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// "Undo" on a setup toast: go back to the setup it replaced.
#[tauri::command]
pub fn overlay_undo(app: AppHandle, state: tauri::State<AppState>, setup: String) -> Result<(), String> {
    hide(&app);
    state.client.set_setup(&setup).map_err(crate::err)
}

/// The toast's own close (it hides itself when its time is up).
#[tauri::command]
pub fn overlay_hide(app: AppHandle) {
    hide(&app);
}

/// Whether the program in front covers its whole monitor (a fullscreen game or video).
fn fullscreen_in_front() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect};
        // SAFETY: plain Win32 calls on the foreground window handle; the buffers outlive them.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return false;
            }
            let mut class = [0u16; 64];
            let n = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
            // The desktop covers the screen too.
            if matches!(String::from_utf16_lossy(&class[..n]).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
                return false;
            }
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetWindowRect(hwnd, &mut rect) == 0 || GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) == 0 {
                return false;
            }
            let m = info.rcMonitor;
            rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
        }
    }
    #[cfg(target_os = "linux")]
    {
        linux_fullscreen()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn linux_fullscreen() -> bool {
    use crate::system::command;
    // Hyprland: the active window's `fullscreen` is 0 when it is not.
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
        && let Ok(out) = command("hyprctl").args(["-j", "activewindow"]).output()
        && let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout)
    {
        return match v.get("fullscreen") {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
            _ => false,
        };
    }
    // X11 (and XWayland games): `_NET_WM_STATE_FULLSCREEN` on the active window.
    if std::env::var_os("DISPLAY").is_some()
        && let Ok(root) = command("xprop").args(["-root", "_NET_ACTIVE_WINDOW"]).output()
        && let Some(id) = String::from_utf8_lossy(&root.stdout).split_whitespace().last().filter(|id| id.starts_with("0x") && *id != "0x0").map(str::to_owned)
        && let Ok(win) = command("xprop").args(["-id", &id, "_NET_WM_STATE"]).output()
    {
        return String::from_utf8_lossy(&win.stdout).contains("_NET_WM_STATE_FULLSCREEN");
    }
    false
}

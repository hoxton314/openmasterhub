//! The switch toast (S08): a small always-on-top window near the tray that says which profile
//! or setup just became live. It never takes focus, is skipped while a fullscreen window has
//! the foreground (unless `[app.feedback] in_fullscreen = true`) and is drawn by the same UI
//! and theme engine as the app (`overlay.html`). Wayland compositors place the window
//! themselves, so there it appears wherever they put new windows.

use std::sync::atomic::Ordering;
use std::time::Duration;

use masterhub_core::ipc::HubState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;
use crate::live::describe;

const LABEL: &str = "overlay";
/// Logical size; the card inside is 320 wide.
const WIDTH: f64 = 352.0;
const HEIGHT: f64 = 84.0;
const MARGIN: f64 = 12.0;
const PROFILE_SECS: f64 = 1.5;
const SETUP_SECS: f64 = 5.0;

#[derive(Serialize, Clone, Debug)]
struct Toast {
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
    if let Err(e) = built {
        log::warn!("switch overlay: {e}");
    }
}

/// A live state change: show it when it was a switch the user should see.
pub fn on_change(app: &AppHandle, prev: Option<&HubState>, new: &HubState) {
    let Some(prev) = prev else { return };
    let setup_changed = new.setup.is_some() && prev.setup != new.setup;
    let profile_changed = !new.profile.is_empty() && prev.profile != new.profile;
    if !setup_changed && !profile_changed {
        return;
    }
    let state = app.state::<AppState>();
    let (toast, in_fullscreen) = {
        let live = state.live.lock().unwrap();
        if live.feedback(|f| f.toast) == Some(false) {
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

fn show(app: &AppHandle, toast: Toast) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let generation = app.state::<AppState>().overlay_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let secs = toast.secs;
    if let Err(e) = window.emit("overlay:show", toast) {
        log::warn!("switch overlay: {e}");
        return;
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(LABEL) else { return };
        place(&handle, &window);
        let _ = window.show();
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs_f64(secs + 0.4));
        // A newer toast owns the window now.
        if handle.state::<AppState>().overlay_generation.load(Ordering::Relaxed) == generation {
            hide(&handle);
        }
    });
}

pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(LABEL) {
            let _ = w.hide();
        }
    });
}

/// Bottom-right of the primary monitor's work area (above the Windows taskbar).
fn place(app: &AppHandle, window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = app.primary_monitor() else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (w, h, m) = ((WIDTH * scale) as i32, (HEIGHT * scale) as i32, (MARGIN * scale) as i32);
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

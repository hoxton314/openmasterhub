//! OpenMasterHUB desktop app: a Tauri shell around the Svelte UI. The Rust side talks to the
//! masterhubd service and forwards its live events to the UI.

mod claude_hooks;
mod files;
mod games;
mod live;
mod overlay;
mod shortcuts;
mod system;
mod themes;
mod tray;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64};

use base64::Engine;
use masterhub_core::config::{self, Config, Face, Widget};
use masterhub_core::ipc::{Client, ClientEvent, DisplayImage, Event, HubState, ModuleState, SessionInfo, Status};
use masterhub_core::render::{KEY_SIZE, LiveData, Picture, Renderer, load_picture_as};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};


struct AppState {
    client: Client,
    renderer: Renderer,
    /// TOML last sent to the service, to recognise our own ConfigChanged echoes.
    last_sent: Mutex<Option<String>>,
    /// What the tray, shortcuts and overlay show (see `live.rs`).
    live: Mutex<live::Live>,
    /// Registered global shortcuts by id.
    bindings: Mutex<HashMap<u32, (String, shortcuts::Binding)>>,
    shortcuts_ready: AtomicBool,
    tray_ready: AtomicBool,
    /// Bumped by every toast, so an old one's timer does not hide a newer toast.
    overlay_generation: AtomicU64,
}

#[derive(Clone, Serialize)]
struct Display {
    serial: String,
    target: u8,
    src: String,
}

#[derive(Serialize)]
struct Snapshot {
    online: bool,
    config: Option<Config>,
    config_path: Option<String>,
    modules: Vec<ModuleState>,
    state: Option<HubState>,
    displays: Vec<Display>,
    /// What the service implements beyond 0.8 (`get_capabilities`); empty for older services.
    capabilities: Vec<String>,
}

fn data_url(jpeg: &[u8]) -> String {
    // Local previews only; device images arrive base64-encoded already.
    format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg))
}

pub(crate) fn err(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

/// Everything the UI needs to draw, fetched from the service.
#[tauri::command]
fn snapshot(state: State<AppState>) -> Snapshot {
    let c = &state.client;
    if !c.is_online() {
        return Snapshot {
            online: false,
            config: None,
            config_path: None,
            modules: Vec::new(),
            state: None,
            displays: Vec::new(),
            capabilities: Vec::new(),
        };
    }
    Snapshot {
        online: true,
        config: live::fetch_config(c),
        config_path: c.config_path().ok(),
        modules: c.modules().unwrap_or_default(),
        state: c.state().ok(),
        displays: c
            .displays()
            .unwrap_or_default()
            .into_iter()
            .map(Display::from)
            .collect(),
        capabilities: c.capabilities(),
    }
}

/// Validate and send a new config; the service saves it and applies it to the hardware.
#[tauri::command]
fn save_config(state: State<AppState>, config: Config) -> Result<(), String> {
    config.validate().map_err(err)?;
    config.validate_holds().map_err(err)?;
    let text = config.to_toml().map_err(err)?;
    *state.last_sent.lock().unwrap() = Some(text.clone());
    state.client.set_config(&text).map_err(err)
}

#[tauri::command]
fn set_profile(state: State<AppState>, name: String) -> Result<(), String> {
    state.client.set_profile(&name).map_err(err)
}

/// Make a setup active (services with capability `setups`).
#[tauri::command]
fn set_setup(state: State<AppState>, name: String) -> Result<(), String> {
    state.client.set_setup(&name).map_err(err)
}

/// End any client's claim (services with capability `release_claim`).
#[tauri::command]
fn release_claim(state: State<AppState>, claim: u64) -> Result<(), String> {
    state.client.release_claim(claim).map_err(err)
}

/// Hybrid setups: pin a profile, the live one without `name` (capability `pin`).
#[tauri::command]
fn pin_profile(state: State<AppState>, name: Option<String>) -> Result<(), String> {
    state.client.pin(name.as_deref()).map_err(err)
}

/// End the hybrid pin (capability `pin`).
#[tauri::command]
fn release_pin(state: State<AppState>) -> Result<(), String> {
    state.client.release_pin().map_err(err)
}

/// A Claude Code session as the Integrations page lists it.
#[derive(Serialize)]
struct ClaudeSession {
    name: String,
    status: Status,
}

/// The open Claude Code sessions the service knows, in slot order.
#[tauri::command]
fn claude_sessions(state: State<AppState>) -> Vec<ClaudeSession> {
    let sessions = state.client.call(masterhub_core::ipc::Request::GetSessions).and_then(|v| Ok(serde_json::from_value::<Vec<SessionInfo>>(v)?));
    sessions.unwrap_or_default().into_iter().map(|s| ClaudeSession { name: s.name, status: s.status }).collect()
}

#[tauri::command]
fn show_page(state: State<AppState>, name: String) -> Result<(), String> {
    state.client.show_page(&name).map_err(err)
}

/// Render a key face locally with placeholder data for live widgets.
#[tauri::command]
fn preview_face(state: State<AppState>, face: Face) -> Result<String, String> {
    let picture = face.image.as_ref().and_then(|p| load_picture_as(p, KEY_SIZE, face.fit.unwrap_or_default()).ok());
    let frame = picture.as_ref().map(|p| match p {
        Picture::Still(i) => i,
        Picture::Animated(f) => &f[0].0,
    });
    let live = face.widget.as_ref().map(|w| match &w.widget {
        Widget::Clock { .. } => LiveData::Text { text: "12:34".into(), caption: Some("Mon 12".into()) },
        Widget::NowPlaying => LiveData::Media { title: "Song title".into(), artist: "Artist".into(), art: None },
        Widget::ClaudeSession { slot } => LiveData::Session {
            name: format!("Session {}", slot + 1),
            status: Status::Ready,
            color: config::StatusLeds::default().ready.0,
            frame: 0,
            checked: false,
            above: None,
            below: None,
        },
        other => LiveData::Gauge { fraction: 0.42, text: "42%".into(), caption: other.title().split(' ').next().unwrap_or("").to_owned() },
    });
    let jpeg = state.renderer.render(&face, KEY_SIZE, frame, live.as_ref(), 0).map_err(err)?;
    Ok(data_url(&jpeg))
}

/// Window classes (`.exe` names on Windows) of open windows, suggested when assigning apps to a
/// profile. The service knows them on every desktop it follows.
#[tauri::command]
fn open_window_classes(state: State<AppState>) -> Vec<String> {
    state.client.open_apps().unwrap_or_else(|e| {
        log::debug!("open windows: {e:#}");
        Vec::new()
    })
}

/// Apps playing audio and the audio devices, suggested as targets of volume and mute actions.
#[tauri::command]
fn audio_info(state: State<AppState>) -> masterhub_core::ipc::AudioInfo {
    state.client.audio().unwrap_or_else(|e| {
        log::debug!("audio info: {e:#}");
        Default::default()
    })
}

#[tauri::command]
fn start_service() -> Result<(), String> {
    launch_service().map_err(err)
}

#[tauri::command]
fn restart_service() -> Result<(), String> {
    stop_service();
    launch_service().map_err(err)
}

/// End a running masterhubd so a new one can take the pipe. On Linux, `launch_service`
/// restarts the systemd unit itself.
fn stop_service() {
    #[cfg(windows)]
    {
        *system::CHILD.lock().unwrap() = None;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/IM", "masterhubd.exe", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Start masterhubd: through systemd on Linux, or the service bundled next to the app
/// elsewhere (detached, so it keeps running when the app closes).
pub(crate) fn launch_service() -> anyhow::Result<()> {
    system::note_launched();
    #[cfg(target_os = "linux")]
    {
        // Restart rather than start: the unit can be active yet unreachable, e.g. the old
        // binary still running after a package upgrade changed the IPC.
        let ok = std::process::Command::new("systemctl").args(["--user", "restart", "masterhubd"]).status().is_ok_and(|s| s.success());
        anyhow::ensure!(ok, "could not restart masterhubd.service; run masterhubd in a terminal instead");
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let exe = std::env::current_exe()?;
        let name = if cfg!(windows) { "masterhubd.exe" } else { "masterhubd" };
        let service = exe.parent().map(|d| d.join(name)).filter(|p| p.exists());
        let service = service.ok_or_else(|| anyhow::anyhow!("masterhubd is not installed next to the app"))?;
        let mut cmd = std::process::Command::new(service);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        }
        // Kept to learn how it ended (`service_status`).
        *system::CHILD.lock().unwrap() = Some(cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?);
        Ok(())
    }
}

impl From<DisplayImage> for Display {
    fn from(d: DisplayImage) -> Self {
        Self { serial: d.serial, target: d.target, src: format!("data:image/jpeg;base64,{}", d.jpeg) }
    }
}

/// Forward service events to the UI.
fn forward_events(app: AppHandle, rx: std::sync::mpsc::Receiver<ClientEvent>) {
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            let r = match event {
                ClientEvent::Online(online) => {
                    system::note_online(online);
                    let r = app.emit("hub:online", online);
                    live::refresh(&app);
                    r
                }
                ClientEvent::Event(Event::Input(i)) => app.emit("hub:input", i),
                ClientEvent::Event(Event::State(s)) => {
                    let r = app.emit("hub:state", s.clone());
                    live::on_state(&app, s);
                    r
                }
                ClientEvent::Event(Event::Modules(m)) => app.emit("hub:modules", m),
                ClientEvent::Event(Event::Display(d)) => app.emit("hub:display", Display::from(d)),
                ClientEvent::Event(Event::ConfigChanged) => {
                    // The tray and the shortcuts follow every change, ours or not.
                    live::refresh(&app);
                    // Ignore the echo of our own save; pass on edits made elsewhere.
                    let state = app.state::<AppState>();
                    let ours = state.client.config().ok().is_some_and(|t| state.last_sent.lock().unwrap().as_deref() == Some(t.as_str()));
                    if ours { Ok(()) } else { app.emit("hub:config-changed", ()) }
                }
                ClientEvent::Event(Event::Claim(c)) => app.emit("hub:claim", c),
                ClientEvent::Event(Event::Claims(c)) => app.emit("hub:claims", c),
                ClientEvent::Event(Event::Sessions(s)) => app.emit("hub:sessions", s),
            };
            if let Err(e) = r {
                log::warn!("emit: {e}");
            }
        }
    });
}

/// WebKitGTK workarounds, as in git-twig: DMA-BUF rendering is broken on NVIDIA + Wayland.
#[cfg(target_os = "linux")]
fn linux_webkit_env() {
    let wayland = std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland");
    if !wayland {
        return;
    }
    // SAFETY: called before any threads are started.
    unsafe {
        if std::path::Path::new("/proc/driver/nvidia").exists() && std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        if std::env::var("GDK_GL").is_err() {
            std::env::set_var("GDK_GL", "gles");
        }
    }
}

pub fn run() {
    #[cfg(target_os = "linux")]
    linux_webkit_env();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (tx, rx) = std::sync::mpsc::channel();
    let client = Client::start(move |e| {
        let _ = tx.send(e);
    });
    tauri::Builder::default()
        // First: a second launch (the app lives in the tray) shows the running window instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            client,
            renderer: Renderer::new(None),
            last_sent: Mutex::new(None),
            live: Mutex::new(live::Live::default()),
            bindings: Mutex::new(HashMap::new()),
            shortcuts_ready: AtomicBool::new(false),
            tray_ready: AtomicBool::new(false),
            overlay_generation: AtomicU64::new(0),
        })
        .on_window_event(|window, event| {
            // Close to the tray, when there is one.
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if state.tray_ready.load(std::sync::atomic::Ordering::Relaxed) && state.live.lock().unwrap().close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            shortcuts::init(app.handle());
            tray::create(app.handle());
            overlay::create(app.handle());
            forward_events(app.handle().clone(), rx);
            // Windows/macOS: the service is bundled with the app; start it if it isn't running
            // (on Linux systemd starts it).
            #[cfg(not(target_os = "linux"))]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    if !handle.state::<AppState>().client.is_online()
                        && let Err(e) = launch_service()
                    {
                        log::warn!("starting masterhubd: {e:#}");
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_config,
            set_profile,
            show_page,
            set_setup,
            release_claim,
            pin_profile,
            release_pin,
            claude_sessions,
            preview_face,
            open_window_classes,
            audio_info,
            start_service,
            restart_service,
            system::service_status,
            system::service_log,
            system::service_log_dir,
            system::service_autostart,
            system::set_service_autostart,
            system::system_checks,
            system::system_info,
            system::vendor_app,
            system::close_vendor_app,
            system::run_command_test,
            system::open_window_titles,
            system::identify,
            claude_hooks::claude_hooks_status,
            claude_hooks::claude_hooks_install,
            claude_hooks::claude_hooks_remove,
            overlay::overlay_undo,
            overlay::overlay_hide,
            games::games_bridge,
            games::games_list,
            games::games_plan,
            games::games_setup,
            games::games_disable,
            themes::themes_list,
            themes::themes_dir,
            themes::open_themes_dir,
            files::open_path,
            files::import_toml,
            files::export_toml,
            files::template_save,
            files::templates_list,
            files::backup_config,
            files::backups_list,
            files::restore_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}

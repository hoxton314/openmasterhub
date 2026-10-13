//! OpenMasterHUB desktop app: a Tauri shell around the Svelte UI. The Rust side talks to the
//! masterhubd service and forwards its live events to the UI.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine;
use masterhub_core::config::{Config, Face, Widget};
use masterhub_core::ipc::{Client, ClientEvent, DisplayImage, Event, HubState, ModuleState};
use masterhub_core::render::{KEY_SIZE, LiveData, Picture, Renderer, load_picture};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};


struct AppState {
    client: Client,
    renderer: Renderer,
    /// TOML last sent to the service, to recognise our own ConfigChanged echoes.
    last_sent: Mutex<Option<String>>,
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
}

fn data_url(jpeg: &[u8]) -> String {
    // Local previews only; device images arrive base64-encoded already.
    format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpeg))
}

fn err(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

/// Everything the UI needs to draw, fetched from the service.
#[tauri::command]
fn snapshot(state: State<AppState>) -> Snapshot {
    let c = &state.client;
    if !c.is_online() {
        return Snapshot { online: false, config: None, config_path: None, modules: Vec::new(), state: None, displays: Vec::new() };
    }
    let path = c.config_path().ok();
    let dir = path.as_deref().and_then(|p| Path::new(p).parent()).map(Path::to_owned).unwrap_or_else(|| PathBuf::from("."));
    let config = c.config().ok().and_then(|text| Config::parse(&text, &dir).map_err(|e| log::warn!("config: {e:#}")).ok());
    Snapshot {
        online: true,
        config,
        config_path: path,
        modules: c.modules().unwrap_or_default(),
        state: c.state().ok(),
        displays: c
            .displays()
            .unwrap_or_default()
            .into_iter()
            .map(Display::from)
            .collect(),
    }
}

/// Validate and send a new config; the service saves it and applies it to the hardware.
#[tauri::command]
fn save_config(state: State<AppState>, config: Config) -> Result<(), String> {
    config.validate().map_err(err)?;
    let text = config.to_toml().map_err(err)?;
    *state.last_sent.lock().unwrap() = Some(text.clone());
    state.client.set_config(&text).map_err(err)
}

#[tauri::command]
fn set_profile(state: State<AppState>, name: String) -> Result<(), String> {
    state.client.set_profile(&name).map_err(err)
}

#[tauri::command]
fn show_page(state: State<AppState>, name: String) -> Result<(), String> {
    state.client.show_page(&name).map_err(err)
}

/// Render a key face locally with placeholder data for live widgets.
#[tauri::command]
fn preview_face(state: State<AppState>, face: Face) -> Result<String, String> {
    let picture = face.image.as_ref().and_then(|p| load_picture(p, KEY_SIZE).ok());
    let frame = picture.as_ref().map(|p| match p {
        Picture::Still(i) => i,
        Picture::Animated(f) => &f[0].0,
    });
    let live = face.widget.as_ref().map(|w| match w {
        Widget::Clock { .. } => LiveData::Text { text: "12:34".into(), caption: Some("Mon 12".into()) },
        Widget::NowPlaying => LiveData::Media { title: "Song title".into(), artist: "Artist".into(), art: None },
        other => LiveData::Gauge { fraction: 0.42, text: "42%".into(), caption: other.title().split(' ').next().unwrap_or("").to_owned() },
    });
    let jpeg = state.renderer.render(&face, KEY_SIZE, frame, live.as_ref(), 0).map_err(err)?;
    Ok(data_url(&jpeg))
}

/// Window classes of open windows (Hyprland), suggested when assigning apps to a profile.
#[tauri::command]
fn open_window_classes() -> Vec<String> {
    let Ok(out) = std::process::Command::new("hyprctl").args(["clients", "-j"]).output() else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else { return Vec::new() };
    let mut classes: Vec<String> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("class")?.as_str().map(str::to_owned))
        .filter(|c| !c.is_empty() && c != masterhub_core::ipc::GUI_APP_ID && !c.starts_with("openmasterhub"))
        .collect();
    classes.sort();
    classes.dedup();
    classes
}

#[tauri::command]
fn start_service() -> Result<(), String> {
    launch_service().map_err(err)
}

/// Start masterhubd: through systemd on Linux, or the service bundled next to the app
/// elsewhere (detached, so it keeps running when the app closes).
fn launch_service() -> anyhow::Result<()> {
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
        cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?;
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
                ClientEvent::Online(online) => app.emit("hub:online", online),
                ClientEvent::Event(Event::Input(i)) => app.emit("hub:input", i),
                ClientEvent::Event(Event::State(s)) => app.emit("hub:state", s),
                ClientEvent::Event(Event::Modules(m)) => app.emit("hub:modules", m),
                ClientEvent::Event(Event::Display(d)) => app.emit("hub:display", Display::from(d)),
                ClientEvent::Event(Event::ConfigChanged) => {
                    // Ignore the echo of our own save; pass on edits made elsewhere.
                    let state = app.state::<AppState>();
                    let ours = state.client.config().ok().is_some_and(|t| state.last_sent.lock().unwrap().as_deref() == Some(t.as_str()));
                    if ours { Ok(()) } else { app.emit("hub:config-changed", ()) }
                }
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
        if Path::new("/proc/driver/nvidia").exists() && std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
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
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { client, renderer: Renderer::new(None), last_sent: Mutex::new(None) })
        .setup(move |app| {
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
            preview_face,
            open_window_classes,
            start_service
        ])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}

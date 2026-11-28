//! The service's local API, used by the desktop app. One JSON message per line over a local
//! socket: a Unix socket in the user's runtime directory (Linux, macOS) or a named pipe that
//! only the current user may open (Windows).
//!
//! Client → service: [`RequestMsg`] (`{"id":1,"method":"get_state"}`).
//! Service → client: [`ServerMsg`], either a response to a request or a pushed [`Event`].
//!
//! | Method | Params | Result |
//! |---|---|---|
//! | `get_config` | | config TOML (string) |
//! | `set_config` | `toml` | validate, save and apply |
//! | `get_config_path` | | path (string) |
//! | `get_modules` | | `Vec<ModuleState>` |
//! | `get_state` | | [`HubState`] |
//! | `get_displays` | | `Vec<DisplayImage>` |
//! | `set_profile` | `name` | switch profile (until the focused app changes) |
//! | `show_page` | `name` | show a keypad page or folder of the active profile |
//! | `set_status` | `source`, `status`, `pid`, `background`, `detail` | set (or clear, `status: null`) a tool's [`Status`] |
//! | `get_open_apps` | | window classes (`.exe` names on Windows) of open windows, sorted |
//! | `get_audio` | | [`AudioInfo`]: apps playing audio, output and input devices |
//! | `get_volume` | | `{level, muted}` of the default output (level 0..=1) |
//! | `get_sessions` | | open Claude Code sessions, `Vec<SessionInfo>` (also the `sessions` event) |
//! | `claim` | `name`, `controls`, `apps`, `priority` | take over controls (live surface), `{"claim": id}` |
//! | `release` | `claim` | give them back |
//! | `backlight` | `claim`, `level` (0..=100, null = the config's) | dim the key and dial screens while the claim lives |
//! | `run_action` | `action` (as in the config, e.g. `{"keys":"ctrl+c"}`), `value`? | run it now |
//! | `frame` | `claim`, `target`, `jpeg` (base64) | show an image on a claimed key (0..=14) or the dial (255) |
//! | `led` | `claim`, `zone`, `mode`, `color`, `brightness` | set a claimed LED |
//! | `get_capabilities` | | what this service implements beyond 0.8, `Vec<String>` (see [`capability`]) |
//! | `set_setup` | `name` | make a setup active (capability `setups`) |
//! | `pin` | `profile`? | hybrid setups: pin a profile, default the live one (capability `pin`) |
//! | `release_pin` | | end the pin (capability `pin`) |
//! | `identify` | `serial` | flash a module so the user can find it (capability `identify`) |
//! | `preview_profile` | `name`, `secs` | show a profile on the hub for a while (capability `preview_profile`) |
//! | `release_claim` | `claim` | end another client's claim (capability `release_claim`) |
//! | `get_system_info` | | [`SystemInfo`] (capability `system_info`) |
//! | `get_claims` | | every claim, `Vec<ClaimState>` (also the `claims` event; capability `claim_details`) |
//! | `get_open_windows` | | open windows with their titles, `Vec<`[`WindowInfo`]`>` (capability `window_titles`) |
//! | `pause` | `paused` | stop (or resume) running the actions bound to controls; `run_action` still works (capability `pause`) |
//!
//! Services without a capability answer its requests with an error; services older than
//! `get_capabilities` answer that with an error too, which clients treat as an empty list.
//!
//! Events: `input`, `state`, `modules`, `display`, `config_changed`, `claim`, `claims` (the whole
//! list, when a claim is added or removed), `sessions`.
//!
//! ## Live surface
//!
//! A client claims [`Controls`] while any of `apps` (window class globs, as for profiles) is
//! focused, or always when `apps` is empty. Where claims overlap, the higher `priority` wins,
//! then the newer claim. While a client owns a control the service stops drawing it, sends
//! no action for it, and marks its `input` events with the claim id; the client draws keys
//! and the dial with `frame` and sets LEDs with `led`. Frames are kept, so an owner that gets
//! a control back (focus returns) shows its last frame again. A claim ends with `release` or
//! when the client disconnects. `claim` events tell each client what it owns right now and
//! how far to turn images clockwise so they read upright.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{Name, SendHalf, Stream};
use serde::{Deserialize, Serialize};

use crate::layout::Placement;

/// Display target of the dial (keys are 0..=14).
pub const TARGET_DIAL: u8 = 255;
/// Application ID (and Wayland app_id) of the GUI. Focusing the GUI does not switch profiles.
pub const GUI_APP_ID: &str = "org.openmasterhub.OpenMasterHub";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    /// `ModuleKind::name()`: base, keypad, fader, knob, roller, dial.
    pub kind: String,
    pub serial: String,
    /// USB socket below the Base's top hub, e.g. "22".
    pub socket: Option<String>,
    pub placement: Option<Placement>,
    pub firmware: Option<String>,
    /// The module ignores software mode until it is unplugged and plugged in again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_replug: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HubState {
    pub profile: String,
    /// Name of the keypad page or folder on screen.
    pub page: String,
    pub in_folder: bool,
    /// Base orientation: true when lying landscape.
    pub horizontal: bool,
    /// Clockwise rotation (0/90/180/270) from the Base's vertical frame to the user's view.
    #[serde(default)]
    pub rotation: u16,
    /// Focused application (window class or exe name), if known.
    pub active_app: Option<String>,
    /// The active setup (services with capability `setups`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<String>,
    /// Why the live profile is live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// Hybrid setups: the pinned profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<PinState>,
    /// Control actions are paused (capability `pause`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub paused: bool,
}

/// Why the live profile is live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    /// Pattern `pattern` of the profile at `index` (0-based) in the setup's list matched the
    /// focused app.
    Rule { pattern: String, index: usize },
    /// No rule matched.
    Fallback,
    /// Chosen by the user (manual setups, `set_profile`).
    Manual,
    /// Pinned (hybrid setups).
    Pin,
    /// A `profile` action.
    Action,
}

/// A hybrid setup's pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinState {
    pub profile: String,
    /// Unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
}

/// Names reported by `get_capabilities`: behaviour beyond the 0.8 baseline. Clients hide or
/// disable controls whose capability is missing; unknown names are ignored.
pub mod capability {
    pub const SETUPS: &str = "setups";
    pub const BASE: &str = "base";
    /// `keys = { combo, hold }`.
    pub const HOLD: &str = "hold";
    pub const LONG_PRESS: &str = "long_press";
    pub const SETUP_ACTION: &str = "setup_action";
    /// `profile = "next" | "prev" | "pick"`.
    pub const PROFILE_CYCLE: &str = "profile_cycle";
    pub const PIN: &str = "pin";
    pub const EXCEPT_SETUPS: &str = "except_setups";
    pub const BLINK_ON_SETUP: &str = "blink_on_setup";
    pub const KNOB_VOLUME: &str = "knob_volume";
    pub const FACE_FIT: &str = "face_fit";
    pub const WIDGET_REFRESH: &str = "widget_refresh";
    pub const IDENTIFY: &str = "identify";
    pub const NEEDS_REPLUG: &str = "needs_replug";
    pub const SYSTEM_INFO: &str = "system_info";
    pub const PREVIEW_PROFILE: &str = "preview_profile";
    pub const RELEASE_CLAIM: &str = "release_claim";
    /// `priority`, `apps`, `pid` and `program` in [`super::ClaimState`].
    pub const CLAIM_DETAILS: &str = "claim_details";
    /// `get_open_windows`.
    pub const WINDOW_TITLES: &str = "window_titles";
    /// `pause` and `HubState::paused`.
    pub const PAUSE: &str = "pause";
}

/// One open window, for `get_open_windows`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Window class (the `.exe` name on Windows), as profiles match it.
    pub class: String,
    pub title: String,
}

/// `get_system_info`: what the service runs on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemInfo {
    /// `linux`, `windows`, `macos`.
    pub platform: String,
    /// How the focused window is found, e.g. `hyprland`, `kwin`, `x11`, `win32`; none if unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMsg {
    pub serial: String,
    /// key, fader, knob, roller, dial
    pub control: String,
    /// Key 0..=14, fader 0..=4, knob 0..=2, roller 0 = left / 1 = right, dial 0.
    pub index: u8,
    /// down, up, cw, ccw, away, toward, move
    pub event: String,
    /// Fader position 0..=100.
    pub value: Option<u8>,
    /// The live-surface claim that owns this control, if any (it got no config action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<u64>,
}

/// An LED zone a client can claim: the Base light, one knob ring, one fader light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedZone {
    Base,
    Knob(u8),
    Fader(u8),
}

/// Controls taken over by a live-surface client (see the module docs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    /// Keypad keys 0..=14.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<u8>,
    /// The dial: its screen, turns and press.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dial: bool,
    /// Knobs 0..=2: turns and press (their LEDs are `leds`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub knobs: Vec<u8>,
    /// Rollers: 0 = left, 1 = right.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rollers: Vec<u8>,
    /// Faders 0..=4.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub faders: Vec<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub leds: Vec<LedZone>,
}

impl Controls {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What a live-surface claim owns right now (event `claim`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimState {
    pub claim: u64,
    pub name: String,
    /// One of its `apps` is focused (or it has none).
    pub active: bool,
    /// The claimed controls it owns now (none while inactive or outranked).
    pub owns: Controls,
    /// Turn key images this far clockwise (0/90/180/270) to read upright.
    pub key_turn: u16,
    /// Same for the dial.
    pub dial_turn: u16,
    /// Claim priority (capability `claim_details`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// The window class globs it is active for; empty: always.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apps: Vec<String>,
    /// Process of the claiming client, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
}

/// A rendered display: JPEG as base64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayImage {
    pub serial: String,
    pub target: u8,
    pub jpeg: String,
}

impl DisplayImage {
    pub fn new(serial: &str, target: u8, jpeg: &[u8]) -> Self {
        use base64::Engine;
        Self { serial: serial.to_owned(), target, jpeg: base64::engine::general_purpose::STANDARD.encode(jpeg) }
    }
}

/// What a tool (e.g. a Claude Code session) is doing, shown on the Base LED. Ordered by urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Working,
    Ready,
    Attention,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    GetConfig,
    SetConfig { toml: String },
    GetConfigPath,
    GetModules,
    GetState,
    GetDisplays,
    SetProfile { name: String },
    ShowPage { name: String },
    /// `source` identifies the reporter, e.g. `claude:<session id>`. The service forgets it when
    /// `pid` exits (Linux).
    SetStatus {
        source: String,
        status: Option<Status>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pid: Option<u32>,
        /// Reported for work running in the background of the source (a Claude Code subagent):
        /// a `working` report then turns `ready` into `working` but never hides `attention`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        background: bool,
        /// What the session is doing, from the main agent's hook events; replaces the last.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<SessionDetail>,
    },
    GetOpenApps,
    GetAudio,
    /// The default output's volume: `{"level": 0.4, "muted": false}`.
    GetVolume,
    /// The open Claude Code sessions, in slot order (`Vec<SessionInfo>`).
    GetSessions,
    /// Take over controls; the result is `{"claim": id}`. See the module docs.
    Claim {
        name: String,
        controls: Controls,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        apps: Vec<String>,
        #[serde(default)]
        priority: i32,
        /// The claiming program's name, for the claim list (else taken from its process).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        program: Option<String>,
    },
    Release { claim: u64 },
    /// Key and dial screen backlight, 0..=100, or `None` for the config's level. Ends with
    /// the claim, so a client that quits never leaves the screens dark.
    Backlight { claim: u64, level: Option<u8> },
    /// Run a config action now, as if a control bound to it fired (`value`: a fader position).
    RunAction {
        action: crate::config::Action,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<u8>,
    },
    /// A 72×72 (key) or 240×240 (dial) JPEG, base64, for a claimed key or the dial (255).
    Frame { claim: u64, target: u8, jpeg: String },
    /// Set a claimed LED. Fields left out keep their value.
    Led {
        claim: u64,
        zone: LedZone,
        /// off, static, breathing, strobe, spring, cycling, random, wave, water, rainbow
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<crate::config::Color>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        brightness: Option<u8>,
    },
    /// `Vec<String>` of [`capability`] names.
    GetCapabilities,
    SetSetup { name: String },
    /// Pin `profile`, or the live profile when `None`.
    Pin {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
    },
    ReleasePin,
    Identify { serial: String },
    PreviewProfile { name: String, secs: u32 },
    /// End any client's claim (admin; the claim list in the app).
    ReleaseClaim { claim: u64 },
    GetSystemInfo,
    /// Every claim, `Vec<ClaimState>`.
    GetClaims,
    /// Open windows with their titles, `Vec<WindowInfo>`.
    GetOpenWindows,
    /// Stop or resume running the actions bound to controls (`run_action` and claims still work).
    Pause { paused: bool },
}

/// Suggestions for the targets of volume and mute actions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioInfo {
    /// Names of the apps playing audio now.
    pub apps: Vec<String>,
    /// Output device descriptions.
    pub outputs: Vec<String>,
    /// Input device descriptions, without the monitors of outputs.
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestMsg {
    pub id: u64,
    #[serde(flatten)]
    pub request: Request,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
    Input(InputMsg),
    State(HubState),
    Modules(Vec<ModuleState>),
    Display(DisplayImage),
    ConfigChanged,
    Claim(ClaimState),
    /// Every claim, sent when one is added or removed (a removed claim is missing from it).
    Claims(Vec<ClaimState>),
    /// The open Claude Code sessions changed (see `get_sessions`).
    Sessions(Vec<SessionInfo>),
}

/// What a Claude Code session is doing, beyond its status (from its hooks).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionDetail {
    /// e.g. "Edit render.rs", "Bash cargo test", "Thinking"; none while waiting for a prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doing: Option<String>,
    /// The session's transcript (JSONL), for its last words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// One open Claude Code session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Its slot: 0 = the oldest open session (the `session` action and widget use it).
    pub slot: u8,
    pub source: String,
    pub name: String,
    pub status: Status,
    /// When the status last changed, Unix milliseconds.
    pub since: u64,
    #[serde(flatten)]
    pub detail: SessionDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Response(Response),
    Event(Event),
}

// ───────────────────────────── socket ─────────────────────────────

/// Where the service listens, per user.
pub fn socket_name() -> Result<Name<'static>> {
    #[cfg(windows)]
    {
        use interprocess::local_socket::GenericNamespaced;
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
        let name: String = format!("openmasterhub-{user}").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        Ok(name.to_ns_name::<GenericNamespaced>()?.into_owned())
    }
    #[cfg(not(windows))]
    {
        use interprocess::local_socket::GenericFilePath;
        Ok(socket_path().to_fs_name::<GenericFilePath>()?.into_owned())
    }
}

/// Unix socket path: `$XDG_RUNTIME_DIR/openmasterhub.sock` (a directory only the user can
/// enter), else the user's temp directory.
#[cfg(not(windows))]
pub fn socket_path() -> std::path::PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    dir.join("openmasterhub.sock")
}

/// Write one message as a JSON line.
pub fn write_line<W: Write, T: Serialize>(w: &mut W, msg: &T) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(msg)?;
    line.push(b'\n');
    w.write_all(&line)?;
    w.flush()
}

// ───────────────────────────── client ─────────────────────────────

/// Connect, send one request and wait for its answer. For short-lived callers such as hooks.
pub fn call_once(request: Request) -> Result<serde_json::Value> {
    let stream = Stream::connect(socket_name()?).context("the masterhubd service is not running")?;
    let (recv, mut send) = stream.split();
    write_line(&mut send, &RequestMsg { id: 1, request }).context("sending to masterhubd")?;
    for line in BufReader::new(recv).lines() {
        // Events are broadcast to every client; skip them.
        if let Ok(ServerMsg::Response(r)) = serde_json::from_str::<ServerMsg>(&line?) {
            return match r.error {
                Some(e) => Err(anyhow!(e)),
                None => Ok(r.result.unwrap_or(serde_json::Value::Null)),
            };
        }
    }
    Err(anyhow!("masterhubd closed the connection"))
}

/// What the client reports to the app.
#[derive(Debug, Clone)]
// One at a time through a channel; boxing would only complicate matching on it.
#[allow(clippy::large_enum_variant)]
pub enum ClientEvent {
    /// Connected to (true) or lost (false) the service.
    Online(bool),
    Event(Event),
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Response>>>>;

/// Connection to the service. Reconnects in the background; calls fail while offline.
#[derive(Clone)]
pub struct Client {
    writer: Arc<Mutex<Option<SendHalf>>>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl Client {
    /// Start connecting; `on_event` is called from a background thread.
    pub fn start(on_event: impl Fn(ClientEvent) + Send + 'static) -> Self {
        let client = Self { writer: Arc::default(), pending: Arc::default(), next_id: Arc::new(AtomicU64::new(1)) };
        let c = client.clone();
        std::thread::Builder::new()
            .name("ipc-client".into())
            .spawn(move || c.run(on_event))
            .expect("spawn ipc client thread");
        client
    }

    fn run(&self, on_event: impl Fn(ClientEvent)) {
        loop {
            if let Ok(stream) = socket_name().and_then(|n| Stream::connect(n).map_err(Into::into)) {
                let (recv, send) = stream.split();
                *self.writer.lock().unwrap() = Some(send);
                on_event(ClientEvent::Online(true));
                for line in BufReader::new(recv).lines() {
                    let Ok(line) = line else { break };
                    match serde_json::from_str::<ServerMsg>(&line) {
                        Ok(ServerMsg::Response(r)) => {
                            if let Some(tx) = self.pending.lock().unwrap().remove(&r.id) {
                                let _ = tx.send(r);
                            }
                        }
                        Ok(ServerMsg::Event(e)) => on_event(ClientEvent::Event(e)),
                        Err(e) => log::warn!("ipc: bad message: {e}"),
                    }
                }
                *self.writer.lock().unwrap() = None;
                self.pending.lock().unwrap().clear();
                on_event(ClientEvent::Online(false));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    pub fn is_online(&self) -> bool {
        self.writer.lock().unwrap().is_some()
    }

    /// Send a request and wait for its result.
    pub fn call(&self, request: Request) -> Result<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);
        {
            let mut w = self.writer.lock().unwrap();
            let w = w.as_mut().ok_or_else(|| anyhow!("the masterhubd service is not running"))?;
            write_line(w, &RequestMsg { id, request }).context("sending to masterhubd")?;
        }
        let r = rx.recv_timeout(Duration::from_secs(10)).map_err(|_| anyhow!("no answer from masterhubd"))?;
        match r.error {
            Some(e) => Err(anyhow!(e)),
            None => Ok(r.result.unwrap_or(serde_json::Value::Null)),
        }
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, request: Request) -> Result<T> {
        Ok(serde_json::from_value(self.call(request)?)?)
    }

    pub fn config(&self) -> Result<String> {
        self.get(Request::GetConfig)
    }
    pub fn config_path(&self) -> Result<String> {
        self.get(Request::GetConfigPath)
    }
    pub fn set_config(&self, toml: &str) -> Result<()> {
        self.call(Request::SetConfig { toml: toml.to_owned() }).map(|_| ())
    }
    pub fn modules(&self) -> Result<Vec<ModuleState>> {
        self.get(Request::GetModules)
    }
    pub fn state(&self) -> Result<HubState> {
        self.get(Request::GetState)
    }
    pub fn displays(&self) -> Result<Vec<DisplayImage>> {
        self.get(Request::GetDisplays)
    }
    pub fn set_profile(&self, name: &str) -> Result<()> {
        self.call(Request::SetProfile { name: name.to_owned() }).map(|_| ())
    }
    pub fn show_page(&self, name: &str) -> Result<()> {
        self.call(Request::ShowPage { name: name.to_owned() }).map(|_| ())
    }
    pub fn open_apps(&self) -> Result<Vec<String>> {
        self.get(Request::GetOpenApps)
    }
    pub fn audio(&self) -> Result<AudioInfo> {
        self.get(Request::GetAudio)
    }
    /// What the service implements; empty for services that predate the request.
    pub fn capabilities(&self) -> Vec<String> {
        self.get(Request::GetCapabilities).unwrap_or_default()
    }
    pub fn set_setup(&self, name: &str) -> Result<()> {
        self.call(Request::SetSetup { name: name.to_owned() }).map(|_| ())
    }
    /// Hybrid setups: pin `profile`, or the live one.
    pub fn pin(&self, profile: Option<&str>) -> Result<()> {
        self.call(Request::Pin { profile: profile.map(str::to_owned) }).map(|_| ())
    }
    pub fn release_pin(&self) -> Result<()> {
        self.call(Request::ReleasePin).map(|_| ())
    }
    pub fn identify(&self, serial: &str) -> Result<()> {
        self.call(Request::Identify { serial: serial.to_owned() }).map(|_| ())
    }
    pub fn preview_profile(&self, name: &str, secs: u32) -> Result<()> {
        self.call(Request::PreviewProfile { name: name.to_owned(), secs }).map(|_| ())
    }
    pub fn release_claim(&self, claim: u64) -> Result<()> {
        self.call(Request::ReleaseClaim { claim }).map(|_| ())
    }
    pub fn system_info(&self) -> Result<SystemInfo> {
        self.get(Request::GetSystemInfo)
    }
    /// Every claim; empty for services that predate the request.
    pub fn claims(&self) -> Vec<ClaimState> {
        self.get(Request::GetClaims).unwrap_or_default()
    }
    /// Open windows with titles (capability `window_titles`).
    pub fn open_windows(&self) -> Result<Vec<WindowInfo>> {
        self.get(Request::GetOpenWindows)
    }
    /// Pause or resume control actions (capability `pause`).
    pub fn pause(&self, paused: bool) -> Result<()> {
        self.call(Request::Pause { paused }).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn run_action_takes_config_actions() {
        let r: RequestMsg = serde_json::from_str(r#"{"id":3,"method":"run_action","params":{"action":{"volume":{"level":"+2","app":"discord"}}}}"#).unwrap();
        let Request::RunAction { action, value } = r.request else { panic!("not run_action") };
        assert_eq!(value, None);
        assert!(matches!(action, crate::config::Action::Volume(v) if v.target == crate::config::AudioTarget::App("discord".into())));
        let r: RequestMsg = serde_json::from_str(r#"{"id":4,"method":"run_action","params":{"action":{"keys":"ctrl+c"}}}"#).unwrap();
        assert!(matches!(r.request, Request::RunAction { action: crate::config::Action::Keys(_), .. }));
    }

    use super::*;

    #[test]
    fn wire_format() {
        let r = RequestMsg { id: 7, request: Request::ShowPage { name: "Main".into() } };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":7,"method":"show_page","params":{"name":"Main"}}"#);
        let r = RequestMsg { id: 1, request: Request::GetState };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":1,"method":"get_state"}"#);
        assert_eq!(serde_json::from_str::<RequestMsg>(r#"{"id":1,"method":"get_state"}"#).unwrap(), r);
        let r = RequestMsg { id: 4, request: Request::GetOpenApps };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":4,"method":"get_open_apps"}"#);

        let r = RequestMsg {
            id: 2,
            request: Request::SetStatus {
                source: "claude:x".into(),
                status: Some(Status::Attention),
                pid: None,
                background: false,
                detail: None,
            },
        };
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            r#"{"id":2,"method":"set_status","params":{"source":"claude:x","status":"attention"}}"#
        );
        assert!(Status::Attention > Status::Ready && Status::Ready > Status::Working);

        let e = ServerMsg::Event(Event::ConfigChanged);
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"event","event":"config_changed"}"#);
        let resp = ServerMsg::Response(Response { id: 3, result: Some(serde_json::json!("x")), error: None });
        assert_eq!(serde_json::to_string(&resp).unwrap(), r#"{"type":"response","id":3,"result":"x"}"#);
    }

    #[test]
    fn new_fields_are_optional() {
        // An old service's state and modules parse, and new fields stay off the wire when unset.
        let old = r#"{"profile":"A","page":"Main","in_folder":false,"horizontal":true,"rotation":90,"active_app":null}"#;
        let s: HubState = serde_json::from_str(old).unwrap();
        assert_eq!(s.setup, None);
        assert_eq!(serde_json::to_string(&s).unwrap(), old);
        let m: ModuleState = serde_json::from_str(r#"{"kind":"dial","serial":"x","socket":null,"placement":null,"firmware":null}"#).unwrap();
        assert!(!m.needs_replug);
        assert!(!serde_json::to_string(&m).unwrap().contains("needs_replug"));
        let c = ClaimState { claim: 1, name: "x".into(), active: true, owns: Controls::default(), key_turn: 0, dial_turn: 0, priority: None, apps: vec![], pid: None, program: None };
        assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"claim":1,"name":"x","active":true,"owns":{},"key_turn":0,"dial_turn":0}"#);
    }

    #[test]
    fn reasons_and_new_requests() {
        let s = HubState {
            setup: Some("Work".into()),
            reason: Some(Reason::Rule { pattern: "blender".into(), index: 0 }),
            pinned: Some(PinState { profile: "Blender".into(), since: Some(5) }),
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["reason"], serde_json::json!({"kind":"rule","pattern":"blender","index":0}));
        assert_eq!(v["pinned"], serde_json::json!({"profile":"Blender","since":5}));
        assert_eq!(serde_json::to_value(Reason::Fallback).unwrap(), serde_json::json!({"kind":"fallback"}));
        let r = RequestMsg { id: 1, request: Request::GetCapabilities };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":1,"method":"get_capabilities"}"#);
        let r: RequestMsg = serde_json::from_str(r#"{"id":2,"method":"pin","params":{}}"#).unwrap();
        assert_eq!(r.request, Request::Pin { profile: None });
        let r: RequestMsg = serde_json::from_str(r#"{"id":2,"method":"set_setup","params":{"name":"Play"}}"#).unwrap();
        assert_eq!(r.request, Request::SetSetup { name: "Play".into() });
        // Old clients' claims parse; the claim list is an event of its own.
        let r: RequestMsg = serde_json::from_str(r#"{"id":3,"method":"claim","params":{"name":"x","controls":{"dial":true}}}"#).unwrap();
        assert!(matches!(r.request, Request::Claim { program: None, priority: 0, .. }));
        assert_eq!(serde_json::to_string(&Event::Claims(vec![])).unwrap(), r#"{"event":"claims","data":[]}"#);
    }
}

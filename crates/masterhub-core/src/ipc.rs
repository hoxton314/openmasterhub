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
//! | `set_status` | `source`, `status`, `pid` | set (or clear, `status: null`) a tool's [`Status`] |
//!
//! Events: `input`, `state`, `modules`, `display`, `config_changed`.

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
    },
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format() {
        let r = RequestMsg { id: 7, request: Request::ShowPage { name: "Main".into() } };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":7,"method":"show_page","params":{"name":"Main"}}"#);
        let r = RequestMsg { id: 1, request: Request::GetState };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"id":1,"method":"get_state"}"#);
        assert_eq!(serde_json::from_str::<RequestMsg>(r#"{"id":1,"method":"get_state"}"#).unwrap(), r);

        let r = RequestMsg {
            id: 2,
            request: Request::SetStatus { source: "claude:x".into(), status: Some(Status::Attention), pid: None },
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
}

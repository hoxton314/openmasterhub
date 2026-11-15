//! Follows the focused window on KDE Plasma (Wayland or X11) and brings windows to the front.
//!
//! Wayland apps cannot see other windows, but KWin can. masterhubd loads a small KWin script
//! over D-Bus (`org.kde.kwin.Scripting`) that calls `ActiveWindow(class)` on this service on
//! every focus change. It is loaded again whenever KWin or masterhubd restarts, so nothing has to
//! be installed. `raise` and `window_classes` run one-shot scripts the same way, which answer
//! over D-Bus.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use zbus::blocking::Connection;

use crate::controller::Msg;

const BUS_NAME: &str = "org.openmasterhub.Focus";
const OBJECT_PATH: &str = "/org/openmasterhub/Focus";
const KWIN: &str = "org.kde.KWin";
const FOCUS_PLUGIN: &str = "openmasterhub-focus";
const FOCUS_JS: &str = include_str!("kwin/focus.js");
const RAISE_JS: &str = include_str!("kwin/raise.js");
const WINDOWS_JS: &str = include_str!("kwin/windows.js");

/// What a one-shot script answers.
enum Reply {
    Raised(bool),
    Windows(Vec<String>),
}

/// One-shot scripts waiting for their answer, by token.
type Pending = Arc<Mutex<HashMap<String, Sender<Reply>>>>;

struct Service {
    conn: Connection,
    pending: Pending,
}

static SERVICE: OnceLock<Service> = OnceLock::new();

struct Focus {
    controller: Sender<Msg>,
    pending: Pending,
}

#[zbus::interface(name = "org.openmasterhub.Focus1")]
impl Focus {
    /// Called by the focus script. An empty class means no window is focused.
    fn active_window(&self, class: &str) {
        log::debug!("kde focus: {class:?}");
        let _ = self.controller.send(Msg::ActiveApp(class.to_owned()));
    }

    /// Called by a raise script when it is done.
    fn raised(&self, token: &str, found: bool) {
        self.reply(token, Reply::Raised(found));
    }

    /// Called by the window list script: the classes of the open windows, one per line.
    fn windows(&self, token: &str, classes: &str) {
        self.reply(token, Reply::Windows(classes.lines().map(str::to_owned).collect()));
    }
}

impl Focus {
    fn reply(&self, token: &str, reply: Reply) {
        if let Some(tx) = self.pending.lock().unwrap().remove(token) {
            let _ = tx.send(reply);
        }
    }
}

pub fn spawn(controller: Sender<Msg>) {
    thread::Builder::new()
        .name("kde-focus".into())
        .spawn(move || {
            let pending = Pending::default();
            let conn = match serve(controller, pending.clone()) {
                Ok(conn) => conn,
                Err(e) => return log::warn!("KDE focus service unavailable: {e}"),
            };
            let _ = SERVICE.set(Service { conn: conn.clone(), pending });
            follow_kwin(&conn);
        })
        .expect("spawn kde focus thread");
}

/// Owning the well-known name also keeps a second masterhubd from loading the script too.
fn serve(controller: Sender<Msg>, pending: Pending) -> zbus::Result<Connection> {
    zbus::blocking::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Focus { controller, pending })?
        .build()
}

/// Load the focus script into every KWin that appears on the bus (checked every few seconds).
fn follow_kwin(conn: &Connection) -> ! {
    let mut seen: Option<String> = None;
    loop {
        let owner = kwin_owner(conn);
        if owner.is_some() && owner != seen {
            match run_script(conn, &write_script("focus.js", FOCUS_JS), FOCUS_PLUGIN) {
                Ok(()) => log::info!("following focused window via KWin"),
                Err(e) => log::warn!("loading the KWin focus script: {e:#}"),
            }
        }
        seen = owner;
        thread::sleep(Duration::from_secs(3));
    }
}

/// The unique bus name of the running KWin.
fn kwin_owner(conn: &Connection) -> Option<String> {
    let reply = conn
        .call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "GetNameOwner", &(KWIN,))
        .ok()?;
    reply.body().deserialize::<String>().ok()
}

fn scripting(conn: &Connection, method: &str, body: &(impl serde::Serialize + zbus::zvariant::DynamicType)) -> zbus::Result<zbus::Message> {
    conn.call_method(Some(KWIN), "/Scripting", Some("org.kde.kwin.Scripting"), method, body)
}

/// Load `path` into KWin as `plugin` and start it, replacing a script of that name.
fn run_script(conn: &Connection, path: &std::path::Path, plugin: &str) -> Result<()> {
    // One left by an earlier masterhubd still holds the name; KWin would refuse a second.
    let _ = scripting(conn, "unloadScript", &(plugin,));
    let path = path.to_str().context("script path is not UTF-8")?;
    let id: i32 = scripting(conn, "loadScript", &(path, plugin))?.body().deserialize()?;
    if id < 0 {
        bail!("KWin refused to load {path}");
    }
    // Plasma 6 puts scripts under /Scripting/ScriptN, Plasma 5 under /N.
    for object in [format!("/Scripting/Script{id}"), format!("/{id}")] {
        if conn.call_method(Some(KWIN), object.as_str(), Some("org.kde.kwin.Script"), "run", &()).is_ok() {
            return Ok(());
        }
    }
    let _ = scripting(conn, "unloadScript", &(plugin,));
    bail!("could not start KWin script {id}")
}

/// KWin reads scripts from files: `$XDG_RUNTIME_DIR/openmasterhub/<name>`.
fn write_script(name: &str, text: &str) -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, PathBuf::from).join("openmasterhub");
    let path = dir.join(name);
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text)) {
        log::warn!("writing {}: {e}", path.display());
    }
    path
}

/// Focus the window of `pid`, or of its nearest ancestor that has one, switching virtual
/// desktop if needed. When that process has several windows, prefer the one whose title
/// contains `title`, else the topmost.
pub fn raise(pid: u32, title: Option<&str>) -> Result<()> {
    let pids = pids(pid);
    match one_shot("raise", |token| raise_script(&pids, token, title))? {
        Reply::Raised(true) => Ok(()),
        _ => bail!("no window found for process {pid} or its parents"),
    }
}

/// Classes of the open windows.
pub fn window_classes() -> Result<Vec<String>> {
    match one_shot("windows", |token| WINDOWS_JS.replace("__TOKEN__", &serde_json::Value::from(token).to_string()))? {
        Reply::Windows(classes) => Ok(classes),
        Reply::Raised(_) => bail!("unexpected answer from KWin"),
    }
}

/// Run a script made by `script(token)` once and wait for the answer it sends with `token`.
fn one_shot(kind: &str, script: impl FnOnce(&str) -> String) -> Result<Reply> {
    let svc = SERVICE.get().context("the KDE focus service is not running")?;
    kwin_owner(&svc.conn).context("neither Hyprland nor KWin is running")?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let token = format!("{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    let plugin = format!("openmasterhub-{kind}-{token}");
    let path = write_script(&format!("{kind}-{token}.js"), &script(&token));

    let (tx, rx) = mpsc::channel();
    svc.pending.lock().unwrap().insert(token.clone(), tx);
    let reply = run_script(&svc.conn, &path, &plugin).and_then(|()| rx.recv_timeout(Duration::from_secs(2)).context("KWin did not answer"));
    svc.pending.lock().unwrap().remove(&token);
    let _ = scripting(&svc.conn, "unloadScript", &(plugin.as_str(),));
    let _ = std::fs::remove_file(&path);
    reply
}

/// `pid` and up to 15 of its ancestors.
fn pids(pid: u32) -> Vec<u32> {
    let mut out = vec![pid];
    while out.len() < 16
        && let Some(parent) = super::parent_pid(*out.last().unwrap()).filter(|&p| p > 1)
    {
        out.push(parent);
    }
    out
}

/// Fill in the raise script. The title goes in last so its text is never substituted.
fn raise_script(pids: &[u32], token: &str, title: Option<&str>) -> String {
    let json = |v: &serde_json::Value| v.to_string();
    RAISE_JS
        .replace("__PIDS__", &json(&pids.into()))
        .replace("__TOKEN__", &json(&token.into()))
        .replace("__TITLE__", &json(&title.unwrap_or_default().to_lowercase().into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_the_raise_script() {
        let s = raise_script(&[42, 7], "1-0", Some("Fix \"__TOKEN__\" Tests"));
        assert!(s.contains("var PIDS = [42,7];"));
        assert!(s.contains("var TOKEN = \"1-0\";"));
        assert!(s.contains(r#"var TITLE = "fix \"__token__\" tests";"#));
        assert!(raise_script(&[1], "t", None).contains("var TITLE = \"\";"));
    }

    #[test]
    fn walks_up_to_the_parent() {
        let p = pids(std::process::id());
        assert_eq!(p[0], std::process::id());
        assert_eq!(p.get(1).copied(), Some(std::os::unix::process::parent_id()).filter(|&p| p > 1));
    }
}


//! Follows the focused window on KDE Plasma (Wayland or X11) and brings windows to the front.
//!
//! Wayland apps cannot see other windows, but KWin can. masterhubd loads a small KWin script
//! over D-Bus (`org.kde.kwin.Scripting`) that calls `ActiveWindow(class)` on this service on
//! every focus change. It is loaded again whenever KWin or masterhubd restarts, so nothing has to
//! be installed. `raise` and `window_classes` run one-shot scripts the same way, which answer
//! over D-Bus.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
/// Not `openmasterhub-focus`: that was the id of a KWin package installed by hand in an early
/// version, and `Scripting.start()` unloads a loaded script named like a disabled package.
const FOCUS_PLUGIN: &str = "openmasterhubd-focus";
const LEGACY_FOCUS_PLUGIN: &str = "openmasterhub-focus";
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

/// Set when the focus script reports, to tell that it really runs.
static REPORTED: AtomicBool = AtomicBool::new(false);

struct Focus {
    controller: Sender<Msg>,
    pending: Pending,
}

#[zbus::interface(name = "org.openmasterhub.Focus1")]
impl Focus {
    /// Called by the focus script. An empty class means no window is focused.
    fn active_window(&self, class: &str) {
        log::debug!("kde focus: {class:?}");
        if !REPORTED.swap(true, Ordering::Relaxed) {
            log::info!("following focused window via KWin");
        }
        let _ = self.controller.send(Msg::ActiveApp(class.to_owned()));
    }

    /// Called by the focus script when it throws, so the error lands in this log.
    fn script_error(&self, message: &str) {
        log::warn!("KWin focus script error: {message}");
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
            let Some(conn) = serve_when_free(controller, pending.clone()) else { return };
            let _ = SERVICE.set(Service { conn: conn.clone(), pending });
            follow_kwin(&conn);
        })
        .expect("spawn kde focus thread");
}

/// Serve the focus interface, waiting while another program (usually a second masterhubd)
/// owns the name: KWin would send our focus reports to it.
fn serve_when_free(controller: Sender<Msg>, pending: Pending) -> Option<Connection> {
    let mut warned = false;
    loop {
        match serve(controller.clone(), pending.clone()) {
            Ok(conn) => {
                if warned {
                    log::info!("{BUS_NAME} is free now");
                }
                return Some(conn);
            }
            Err(zbus::Error::NameTaken) => {
                if !warned {
                    let who = owner_pid(BUS_NAME).map_or_else(String::new, |pid| format!(" (process {pid})"));
                    log::warn!("another program{who} owns {BUS_NAME}, so focus reports from KWin go there; is a second masterhubd running? Waiting for it to exit");
                    warned = true;
                }
                thread::sleep(Duration::from_secs(5));
            }
            Err(e) => {
                log::warn!("KDE focus service unavailable: {e}");
                return None;
            }
        }
    }
}

/// The process that owns a bus name.
fn owner_pid(name: &str) -> Option<u32> {
    let conn = Connection::session().ok()?;
    let reply = conn
        .call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "GetConnectionUnixProcessID", &(name,))
        .ok()?;
    reply.body().deserialize().ok()
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
            load_focus_script(conn);
        }
        seen = owner;
        thread::sleep(Duration::from_secs(3));
    }
}

/// Load the focus script, then check that it reports: it does once as soon as it runs.
fn load_focus_script(conn: &Connection) {
    // An earlier masterhubd of this branch loaded it under the old name.
    log::info!("KWin is running; loading the focus script");
    unload(conn, LEGACY_FOCUS_PLUGIN);
    REPORTED.store(false, Ordering::Relaxed);
    let path = write_script("focus.js", FOCUS_JS);
    if let Err(e) = run_script(conn, &path, FOCUS_PLUGIN) {
        return log::warn!("loading the KWin focus script: {e:#}");
    }
    for _ in 0..50 {
        if REPORTED.load(Ordering::Relaxed) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    // Tell apart a script KWin dropped (it failed to read or run it) from one that runs silently.
    let loaded = scripting(conn, "isScriptLoaded", &(FOCUS_PLUGIN,)).and_then(|r| r.body().deserialize::<bool>());
    match loaded {
        Ok(true) => log::warn!(
            "KWin has the focus script ({}) loaded, but it never reported a window; focus changes will not switch profiles",
            path.display()
        ),
        Ok(false) => log::warn!(
            "KWin dropped the focus script ({}) without running it; look for its error with `journalctl --user -b | grep -i focus.js`",
            path.display()
        ),
        Err(e) => log::warn!("the KWin focus script never reported, and KWin did not answer: {e}"),
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
///
/// Scripts are started with `Scripting.start()`, which runs every loaded script that is not
/// running yet (KWin calls it itself when its config changes), rather than through their
/// `/Scripting/ScriptN` objects: KWin numbers a new script `scripts.size()`, so once a script
/// in the middle of the list has been unloaded, a new one gets the number (and D-Bus path) of a
/// script that is still running, and `run()` on that path does nothing.
fn run_script(conn: &Connection, path: &std::path::Path, plugin: &str) -> Result<()> {
    unload(conn, plugin);
    let path = path.to_str().context("script path is not UTF-8")?;
    let id: i32 = scripting(conn, "loadScript", &(path, plugin))?.body().deserialize()?;
    if id < 0 {
        bail!("KWin refused to load {path}");
    }
    log::debug!("KWin loaded {plugin} as script {id}");
    scripting(conn, "start", &()).context("starting the script")?;
    Ok(())
}

/// Unload the script named `plugin`, if loaded, and wait until it is gone: KWin deletes it
/// later, and refuses to load another script of that name until then.
fn unload(conn: &Connection, plugin: &str) {
    if !matches!(scripting(conn, "unloadScript", &(plugin,)).and_then(|r| r.body().deserialize::<bool>()), Ok(true)) {
        return;
    }
    for _ in 0..20 {
        let loaded = scripting(conn, "isScriptLoaded", &(plugin,)).and_then(|r| r.body().deserialize::<bool>());
        if !matches!(loaded, Ok(true)) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
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
    // A unique name: no need to wait until KWin has deleted it.
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




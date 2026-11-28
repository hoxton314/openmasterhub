//! Local IPC server for the desktop app (see `masterhub_core::ipc` for the protocol).
//!
//! One thread per connected client reads its requests. Requests are answered from the shared
//! snapshot; changes go to the controller without waiting for it. Events from the controller
//! are written to every client through the [`Broadcaster`].

use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result, bail};
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{ListenerOptions, SendHalf, Stream};
use masterhub_core::config::Config;
use masterhub_core::ipc::{
    DisplayImage, Event, Request, RequestMsg, Response, ServerMsg, SystemInfo, TARGET_DIAL, capability, socket_name, write_line,
};
use serde::Serialize;

use crate::controller::{Msg, Snapshot};

/// Writes events to every connected client.
#[derive(Default)]
pub struct Broadcaster {
    clients: Mutex<Vec<Arc<Mutex<SendHalf>>>>,
}

impl Broadcaster {
    pub fn send(&self, event: Event) {
        let msg = ServerMsg::Event(event);
        self.clients.lock().unwrap().retain(|c| write_line(&mut *c.lock().unwrap(), &msg).is_ok());
    }

    fn add(&self, client: Arc<Mutex<SendHalf>>) {
        self.clients.lock().unwrap().push(client);
    }
}

/// Start listening. Fails if another masterhubd already serves this user.
pub fn serve(controller: Sender<Msg>, snapshot: Arc<Mutex<Snapshot>>, broadcaster: Arc<Broadcaster>) -> Result<()> {
    let name = socket_name()?;
    if Stream::connect(name.clone()).is_ok() {
        bail!("masterhubd is already running for this user");
    }
    let opts = ListenerOptions::new().name(name).try_overwrite(true);
    #[cfg(windows)]
    let opts = {
        use interprocess::os::windows::local_socket::ListenerOptionsExt;
        use interprocess::os::windows::security_descriptor::SecurityDescriptor;
        // Only the pipe's owner (this user) and SYSTEM may connect: a client can set
        // shell-command actions, so other users must not reach the pipe.
        let sddl = widestring::U16CString::from_str("D:P(A;;GA;;;OW)(A;;GA;;;SY)").expect("static SDDL");
        opts.security_descriptor(SecurityDescriptor::deserialize(&sddl).context("pipe security descriptor")?)
    };
    let listener = opts.create_sync().context("creating the local socket")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = masterhub_core::ipc::socket_path();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).context("restricting the socket")?;
        log::info!("listening on {}", path.display());
    }
    #[cfg(windows)]
    log::info!("listening on a named pipe for this user");

    thread::Builder::new()
        .name("ipc".into())
        .spawn(move || {
            for conn in listener.incoming() {
                match conn {
                    Ok(stream) => {
                        let (controller, snapshot, broadcaster) = (controller.clone(), snapshot.clone(), broadcaster.clone());
                        let _ = thread::Builder::new()
                            .name("ipc-client".into())
                            .spawn(move || client(stream, controller, snapshot, broadcaster));
                    }
                    Err(e) => log::warn!("ipc accept: {e}"),
                }
            }
        })
        .context("spawning ipc thread")?;
    Ok(())
}

fn client(stream: Stream, controller: Sender<Msg>, snapshot: Arc<Mutex<Snapshot>>, broadcaster: Arc<Broadcaster>) {
    // The client's process, for the claim list.
    // `pid_t` on Unix, already `u32` on Windows.
    #[allow(clippy::useless_conversion)]
    let pid = stream.peer_creds().ok().and_then(|c| c.pid()).and_then(|p| u32::try_from(p).ok());
    let peer = Peer { pid, program: pid.and_then(crate::focus::program_name) };
    let (recv, send) = stream.split();
    let send = Arc::new(Mutex::new(send));
    broadcaster.add(send.clone());
    log::debug!("app connected");
    // Live-surface claims made on this connection; they end with it.
    let mut claims: Vec<u64> = Vec::new();
    for line in BufReader::new(recv).lines() {
        let Ok(line) = line else { break };
        let response = match serde_json::from_str::<RequestMsg>(&line) {
            Ok(RequestMsg { id, request }) => match handle(request, &controller, &snapshot, &mut claims, &peer) {
                Ok(result) => Response { id, result: Some(result), error: None },
                Err(e) => Response { id, result: None, error: Some(format!("{e:#}")) },
            },
            Err(e) => Response { id: 0, result: None, error: Some(format!("bad request: {e}")) },
        };
        if write_line(&mut *send.lock().unwrap(), &ServerMsg::Response(response)).is_err() {
            break;
        }
    }
    for claim in claims {
        let _ = controller.send(Msg::Release { claim });
    }
    log::debug!("app disconnected");
}

/// The process at the other end of a connection.
struct Peer {
    pid: Option<u32>,
    program: Option<String>,
}

/// What this service implements beyond the 0.8 baseline (`get_capabilities`). Add a name here
/// when its behaviour lands; the app hides the controls of missing ones.
pub const CAPABILITIES: &[&str] = &[
    capability::SETUPS,
    capability::BASE,
    // `keys = { combo, hold }` runs as `combo@hold`.
    capability::HOLD,
    capability::LONG_PRESS,
    capability::SETUP_ACTION,
    capability::PROFILE_CYCLE,
    capability::PIN,
    capability::EXCEPT_SETUPS,
    capability::BLINK_ON_SETUP,
    capability::FACE_FIT,
    capability::WIDGET_REFRESH,
    capability::IDENTIFY,
    capability::NEEDS_REPLUG,
    capability::SYSTEM_INFO,
    capability::PREVIEW_PROFILE,
    capability::RELEASE_CLAIM,
    capability::CLAIM_DETAILS,
    capability::WINDOW_TITLES,
    capability::PAUSE,
];

/// Live-surface claim ids, unique for the service's lifetime.
static NEXT_CLAIM: AtomicU64 = AtomicU64::new(1);
/// Largest frame accepted (a 240×240 JPEG at high quality is ~30 KB).
const MAX_FRAME: usize = 256 * 1024;

fn json<T: Serialize>(v: T) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(v)?)
}

/// Pins need a hybrid setup.
fn check_hybrid(snap: &Snapshot) -> Result<()> {
    let active = snap.state.setup.as_deref().and_then(|n| snap.config.effective_setups().into_iter().find(|s| s.name == n));
    match active {
        Some(s) if s.mode() == masterhub_core::config::SetupMode::Hybrid => Ok(()),
        Some(s) => bail!("setup {:?} is not hybrid; only hybrid setups pin profiles", s.name),
        None => bail!("no setup is active; only hybrid setups pin profiles"),
    }
}

/// Refuse an action from a client (`masterhubd ctl`, the app) that would do nothing, so the
/// caller hears why instead of only the service log.
fn check_action(snap: &Snapshot, action: &masterhub_core::config::Action) -> Result<()> {
    use masterhub_core::config::{Action, PinAction};
    let step = |name: &str| matches!(name, "next" | "prev" | "pick");
    match action {
        Action::Pin(PinAction::Toggle) if snap.state.pinned.is_none() => check_hybrid(snap),
        Action::Pin(PinAction::Release) if snap.state.pinned.is_none() => bail!("nothing is pinned"),
        Action::Setup(name) if !step(name) && !snap.config.effective_setups().iter().any(|s| s.name == *name) => bail!("no setup {name:?}"),
        // An exact profile name wins over next/prev/pick.
        Action::Profile(name) if snap.config.profile(name).is_none() && !step(name) => bail!("no profile {name:?}"),
        _ => Ok(()),
    }
}

fn handle(
    request: Request,
    controller: &Sender<Msg>,
    snapshot: &Mutex<Snapshot>,
    claims: &mut Vec<u64>,
    peer: &Peer,
) -> Result<serde_json::Value> {
    let own = |claim: u64| if claims.contains(&claim) { Ok(()) } else { Err(anyhow::anyhow!("no claim {claim} on this connection")) };
    match request {
        Request::GetConfig => json(&snapshot.lock().unwrap().config_text),
        Request::GetConfigPath => json(snapshot.lock().unwrap().config_path.display().to_string()),
        Request::GetModules => json(&snapshot.lock().unwrap().modules),
        Request::GetState => json(&snapshot.lock().unwrap().state),
        Request::GetDisplays => json(
            snapshot
                .lock()
                .unwrap()
                .displays
                .iter()
                .map(|((serial, target), jpeg)| DisplayImage::new(serial, *target, jpeg))
                .collect::<Vec<_>>(),
        ),
        Request::SetConfig { toml } => {
            let path = snapshot.lock().unwrap().config_path.clone();
            let dir = path.parent().unwrap_or(std::path::Path::new("."));
            let config = Config::parse_unchecked(&toml, dir)?;
            crate::actions::validate_config(&config)?;
            if let Some(d) = path.parent() {
                std::fs::create_dir_all(d)?;
            }
            let tmp = path.with_extension("toml.tmp");
            std::fs::write(&tmp, &toml)?;
            std::fs::rename(&tmp, &path)?;
            log::info!("config replaced by the app");
            controller.send(Msg::ApplyConfig { config: Box::new(config), text: toml })?;
            json(())
        }
        Request::SetProfile { name } => {
            if !snapshot.lock().unwrap().config.profiles.iter().any(|p| p.name == name) {
                bail!("no profile {name:?}");
            }
            controller.send(Msg::SetProfile(name))?;
            json(())
        }
        Request::ShowPage { name } => {
            controller.send(Msg::ShowPage(name))?;
            json(())
        }
        Request::SetStatus { source, status, pid, background, detail } => {
            controller.send(Msg::SetStatus { source, status, pid, background, detail })?;
            json(())
        }
        Request::GetOpenApps => json(crate::focus::open_apps()?),
        Request::GetAudio => json(crate::audio::info()),
        Request::GetVolume => {
            let (level, muted) = crate::audio::get().context("no default output")?;
            json(serde_json::json!({ "level": level, "muted": muted }))
        }
        Request::GetSessions => json(&snapshot.lock().unwrap().sessions),
        Request::Claim { name, controls, apps, priority, program } => {
            if controls.is_empty() {
                bail!("claim names no controls");
            }
            if controls.keys.iter().any(|&k| k >= 15)
                || controls.knobs.iter().any(|&k| k >= 3)
                || controls.rollers.iter().any(|&k| k >= 2)
                || controls.faders.iter().any(|&k| k >= 5)
            {
                bail!("claim names a control that does not exist");
            }
            let claim = NEXT_CLAIM.fetch_add(1, Ordering::Relaxed);
            claims.push(claim);
            let program = program.or_else(|| peer.program.clone());
            controller.send(Msg::Claim { claim, name, controls, apps, priority, pid: peer.pid, program })?;
            json(serde_json::json!({ "claim": claim }))
        }
        Request::RunAction { action, value } => {
            check_action(&snapshot.lock().unwrap(), &action)?;
            controller.send(Msg::RunAction { action, value })?;
            json(())
        }
        Request::Backlight { claim, level } => {
            own(claim)?;
            controller.send(Msg::Backlight { claim, level })?;
            json(())
        }
        Request::Release { claim } => {
            own(claim)?;
            claims.retain(|c| *c != claim);
            controller.send(Msg::Release { claim })?;
            json(())
        }
        Request::Frame { claim, target, jpeg } => {
            own(claim)?;
            if target >= 15 && target != TARGET_DIAL {
                bail!("no display target {target}");
            }
            use base64::Engine;
            let jpeg = base64::engine::general_purpose::STANDARD.decode(jpeg).context("jpeg is not base64")?;
            if jpeg.len() > MAX_FRAME || !jpeg.starts_with(&[0xFF, 0xD8]) {
                bail!("frame must be a JPEG of at most {MAX_FRAME} bytes");
            }
            controller.send(Msg::Frame { claim, target, jpeg: Arc::new(jpeg) })?;
            json(())
        }
        Request::Led { claim, zone, mode, color, brightness } => {
            own(claim)?;
            let mode = match mode.as_deref() {
                Some(m) => Some(crate::surface::parse_led_mode(m).ok_or_else(|| anyhow::anyhow!("unknown LED mode {m:?}"))?),
                None => None,
            };
            controller.send(Msg::Led { claim, zone, mode, color: color.map(|c| c.0), brightness })?;
            json(())
        }
        Request::GetCapabilities => json(CAPABILITIES),
        Request::SetSetup { name } => {
            if !snapshot.lock().unwrap().config.effective_setups().iter().any(|s| s.name == name) {
                bail!("no setup {name:?}");
            }
            controller.send(Msg::SetSetup(name))?;
            json(())
        }
        Request::Pin { profile } => {
            {
                let snap = snapshot.lock().unwrap();
                if let Some(p) = &profile
                    && snap.config.profile(p).is_none()
                {
                    bail!("no profile {p:?}");
                }
                check_hybrid(&snap)?;
            }
            controller.send(Msg::Pin(profile))?;
            json(())
        }
        Request::ReleasePin => {
            controller.send(Msg::ReleasePin)?;
            json(())
        }
        Request::Identify { serial } => {
            let kind = snapshot.lock().unwrap().modules.iter().find(|m| m.serial == serial).map(|m| m.kind.clone());
            match kind.as_deref() {
                None => bail!("no module {serial:?}"),
                Some("roller") => bail!("the roller has no screen or lights to flash"),
                Some(_) => controller.send(Msg::Identify(serial))?,
            }
            json(())
        }
        Request::PreviewProfile { name, secs } => {
            if snapshot.lock().unwrap().config.profile(&name).is_none() {
                bail!("no profile {name:?}");
            }
            controller.send(Msg::Preview { name, secs })?;
            json(())
        }
        Request::ReleaseClaim { claim } => {
            if !snapshot.lock().unwrap().claims.iter().any(|c| c.claim == claim) {
                bail!("no claim {claim}");
            }
            controller.send(Msg::Release { claim })?;
            json(())
        }
        Request::GetSystemInfo => json(SystemInfo {
            platform: std::env::consts::OS.to_owned(),
            focus_backend: crate::focus::backend().map(str::to_owned),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        Request::GetClaims => json(&snapshot.lock().unwrap().claims),
        Request::GetOpenWindows => json(crate::focus::open_windows()?),
        Request::Pause { paused } => {
            controller.send(Msg::Pause(paused))?;
            json(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_are_known_names() {
        // Only names the app knows (`Capability` in app/src/lib/types.ts; `window_titles` and `pause`
        // are used by the Tauri side only).
        let known = [
            capability::SETUPS, capability::BASE, capability::HOLD, capability::LONG_PRESS, capability::SETUP_ACTION,
            capability::PROFILE_CYCLE, capability::PIN, capability::EXCEPT_SETUPS, capability::BLINK_ON_SETUP,
            capability::KNOB_VOLUME, capability::FACE_FIT, capability::WIDGET_REFRESH, capability::IDENTIFY,
            capability::NEEDS_REPLUG, capability::SYSTEM_INFO, capability::PREVIEW_PROFILE, capability::RELEASE_CLAIM,
            capability::CLAIM_DETAILS, capability::WINDOW_TITLES, capability::PAUSE,
        ];
        assert!(CAPABILITIES.iter().all(|c| known.contains(c)));
    }

    #[test]
    fn actions_that_would_do_nothing_are_refused() {
        use masterhub_core::config::{Action, PinAction};
        let config = Config::parse(
            "[[profile]]\nname = \"Desktop\"\n[[setup]]\nname = \"Play\"\nfallback = \"Desktop\"\n[[setup]]\nname = \"Work\"\nmode = \"hybrid\"\nfallback = \"Desktop\"\n",
            std::path::Path::new("."),
        )
        .unwrap();
        let snap = |setup: &str| Snapshot {
            config: config.clone(),
            config_text: String::new(),
            config_path: Default::default(),
            modules: Vec::new(),
            state: masterhub_core::ipc::HubState { profile: "Desktop".into(), setup: Some(setup.into()), ..Default::default() },
            displays: Default::default(),
            sessions: Vec::new(),
            claims: Vec::new(),
        };
        let err = |s: &Snapshot, a: Action| check_action(s, &a).err().map(|e| e.to_string());
        let play = snap("Play");
        assert_eq!(err(&play, Action::Pin(PinAction::Toggle)).as_deref(), Some("setup \"Play\" is not hybrid; only hybrid setups pin profiles"));
        assert_eq!(err(&play, Action::Pin(PinAction::Release)).as_deref(), Some("nothing is pinned"));
        assert_eq!(err(&play, Action::Setup("Nope".into())).as_deref(), Some("no setup \"Nope\""));
        assert_eq!(err(&play, Action::Profile("Nope".into())).as_deref(), Some("no profile \"Nope\""));
        for ok in [Action::Setup("next".into()), Action::Setup("Work".into()), Action::Profile("pick".into()), Action::Profile("Desktop".into())] {
            assert_eq!(err(&play, ok), None);
        }
        assert_eq!(err(&snap("Work"), Action::Pin(PinAction::Toggle)), None);
    }
}

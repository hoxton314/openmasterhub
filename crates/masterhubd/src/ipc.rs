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
    DisplayImage, Event, Request, RequestMsg, Response, ServerMsg, TARGET_DIAL, socket_name, write_line,
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
    let (recv, send) = stream.split();
    let send = Arc::new(Mutex::new(send));
    broadcaster.add(send.clone());
    log::debug!("app connected");
    // Live-surface claims made on this connection; they end with it.
    let mut claims: Vec<u64> = Vec::new();
    for line in BufReader::new(recv).lines() {
        let Ok(line) = line else { break };
        let response = match serde_json::from_str::<RequestMsg>(&line) {
            Ok(RequestMsg { id, request }) => match handle(request, &controller, &snapshot, &mut claims) {
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

/// Live-surface claim ids, unique for the service's lifetime.
static NEXT_CLAIM: AtomicU64 = AtomicU64::new(1);
/// Largest frame accepted (a 240×240 JPEG at high quality is ~30 KB).
const MAX_FRAME: usize = 256 * 1024;

fn json<T: Serialize>(v: T) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(v)?)
}

fn handle(
    request: Request,
    controller: &Sender<Msg>,
    snapshot: &Mutex<Snapshot>,
    claims: &mut Vec<u64>,
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
            let config = Config::parse(&toml, dir)?;
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
        Request::SetStatus { source, status, pid, background } => {
            controller.send(Msg::SetStatus { source, status, pid, background })?;
            json(())
        }
        Request::GetOpenApps => json(crate::focus::open_apps()?),
        Request::GetAudio => json(crate::audio::info()),
        Request::Claim { name, controls, apps, priority } => {
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
            controller.send(Msg::Claim { claim, name, controls, apps, priority })?;
            json(serde_json::json!({ "claim": claim }))
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
    }
}

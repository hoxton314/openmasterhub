//! The connection to masterhubd's live surface (openmasterhub fork, docs/LIVE_SURFACE.md).

use std::io::Cursor;
use std::sync::mpsc::{Receiver, Sender, channel};

use anyhow::Result;
use base64::Engine;
use image::{ImageEncoder, RgbImage, codecs::jpeg::JpegEncoder};
use masterhub_core::config::{Action, Color};
use masterhub_core::ipc::{Client, ClientEvent, Controls, LedZone, Request};

/// A knob click for the knob worker: `delta` in percent, or a mute toggle.
pub struct KnobCmd {
    pub knob: u8,
    pub apps: Vec<String>,
    pub delta: i32,
    pub mute: bool,
}

fn first_stream(v: &serde_json::Value) -> Option<(Option<String>, f32, bool)> {
    let f = v.as_array()?.first()?;
    Some((f["name"].as_str().map(str::to_owned), f["volume"].as_f64()? as f32, f["muted"].as_bool().unwrap_or(false)))
}

/// Read the first of `apps` that has an audio stream, then set it to an exact volume.
fn apply(client: &Client, knob: u8, apps: &[String], delta: i32, mute: bool) -> AppVol {
    for app in apps {
        let Ok(now) = client.call(Request::AppVolume { app: app.clone(), volume: None, mute: None }) else { continue };
        let Some((_, vol, _)) = first_stream(&now) else { continue };
        let target = (delta != 0).then(|| {
            let pct = ((vol * 100.0).round() as i32 + delta).clamp(0, 100);
            pct.to_string()
        });
        let mute = mute.then(|| serde_json::from_value(serde_json::json!("toggle")).expect("mute toggle"));
        let after = client.call(Request::AppVolume { app: app.clone(), volume: target, mute }).ok();
        if let Some((name, volume, muted)) = after.as_ref().and_then(first_stream) {
            return AppVol { knob, name, volume, muted };
        }
    }
    AppVol { knob, name: None, volume: 0.0, muted: false }
}

/// The result of a knob's volume change: which app, its volume now, muted.
#[derive(Debug, Clone)]
pub struct AppVol {
    pub knob: u8,
    pub name: Option<String>,
    pub volume: f32,
    pub muted: bool,
}

pub struct Hub {
    client: Client,
    pub events: Receiver<ClientEvent>,
    pub claim: Option<u64>,
    /// Clockwise turn the service asks for (key images, dial image).
    pub key_turn: u16,
    pub dial_turn: u16,
}

impl Hub {
    pub fn start() -> Self {
        let (tx, rx) = channel();
        let client = Client::start(move |e| {
            let _ = tx.send(e);
        });
        Self { client, events: rx, claim: None, key_turn: 0, dial_turn: 0 }
    }

    /// Report a status for the Base light (`None` clears it), off the main loop.
    pub fn set_status(&self, source: &str, status: Option<masterhub_core::ipc::Status>) {
        let client = self.client.clone();
        let source = source.to_owned();
        std::thread::spawn(move || {
            let r = client.call(Request::SetStatus { source, status, pid: None, background: false, detail: None });
            if let Err(e) = r {
                log::warn!("status: {e:#}");
            }
        });
    }

    /// The open Claude Code sessions right now.
    pub fn sessions(&self) -> Vec<masterhub_core::ipc::SessionInfo> {
        self.client.call(Request::GetSessions).ok().and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    /// The focused window's class right now (state events only come when it changes).
    pub fn active_app(&self) -> Option<String> {
        let v = self.client.call(Request::GetState).ok()?;
        serde_json::from_value::<masterhub_core::ipc::HubState>(v).ok()?.active_app
    }

    pub fn release(&mut self) {
        if let Some(claim) = self.claim.take() {
            let _ = self.client.call(Request::Release { claim });
        }
    }

    pub fn online(&self) -> bool {
        self.client.is_online()
    }

    /// Claim the controls of daily mode (always, lowest priority, so app profiles win later).
    pub fn claim(&mut self, controls: Controls) -> Result<u64> {
        self.claim_as("pulpit", controls, 0)
    }

    /// Claim under another name and priority (the gallery sits above daily mode).
    pub fn claim_as(&mut self, name: &str, controls: Controls, priority: i32) -> Result<u64> {
        self.claim_apps(name, controls, vec![], priority)
    }

    /// A claim that applies only while one of `apps` (window-class globs) is focused.
    pub fn claim_apps(&mut self, name: &str, controls: Controls, apps: Vec<String>, priority: i32) -> Result<u64> {
        let v = self.client.call(Request::Claim { name: name.into(), controls, apps, priority })?;
        let id = v["claim"].as_u64().ok_or_else(|| anyhow::anyhow!("no claim id in {v}"))?;
        self.claim = Some(id);
        Ok(id)
    }

    pub fn frame(&self, target: u8, img: &RgbImage, turn: u16) -> Result<()> {
        let Some(claim) = self.claim else { return Ok(()) };
        let img = match turn % 360 {
            90 => image::imageops::rotate90(img),
            180 => image::imageops::rotate180(img),
            270 => image::imageops::rotate270(img),
            _ => img.clone(),
        };
        let mut jpeg = Cursor::new(Vec::new());
        JpegEncoder::new_with_quality(&mut jpeg, 88).write_image(&img, img.width(), img.height(), image::ExtendedColorType::Rgb8)?;
        let jpeg = base64::engine::general_purpose::STANDARD.encode(jpeg.into_inner());
        self.client.call(Request::Frame { claim, target, jpeg })?;
        Ok(())
    }

    /// Set a claimed LED (static colour).
    pub fn led(&self, zone: LedZone, color: &str, brightness: u8) {
        self.led_mode(zone, "static", Some(color), Some(brightness));
    }

    /// Key and dial backlight while our claim lives; `None` = the service's configured level.
    pub fn backlight(&self, level: Option<u8>) {
        let Some(claim) = self.claim else { return };
        if let Err(e) = self.client.call(Request::Backlight { claim, level }) {
            log::warn!("backlight: {e:#}");
        }
    }

    /// Set a claimed LED's firmware mode (static, breathing, off, …), colour and brightness.
    pub fn led_mode(&self, zone: LedZone, mode: &str, color: Option<&str>, brightness: Option<u8>) {
        let Some(claim) = self.claim else { return };
        let r = self.client.call(Request::Led {
            claim,
            zone,
            mode: Some(mode.into()),
            color: color.and_then(Color::parse),
            brightness,
        });
        if let Err(e) = r {
            log::warn!("led: {e:#}");
        }
    }

    /// Start the knob worker: one thread applies every knob change in order, summing the
    /// clicks that queued up meanwhile and setting an exact volume, so fast turning never
    /// races itself.
    pub fn knob_worker(&self, tx: Sender<AppVol>) -> Sender<KnobCmd> {
        // Its own connection: the service answers one connection's requests in order, so
        // volume changes on the frame connection would stall every frame behind them.
        let client = Client::start(|_| {});
        let (cmd_tx, cmd_rx) = channel::<KnobCmd>();
        std::thread::Builder::new()
            .name("knobs".into())
            .spawn(move || {
                let t = std::time::Instant::now();
                while !client.is_online() && t.elapsed() < std::time::Duration::from_secs(10) {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                while let Ok(first) = cmd_rx.recv() {
                    // Everything that queued up while the last change ran, per knob.
                    let mut batch = vec![first];
                    batch.extend(cmd_rx.try_iter());
                    let mut knobs: Vec<(u8, Vec<String>, i32, bool)> = Vec::new();
                    for c in batch {
                        match knobs.iter_mut().find(|k| k.0 == c.knob) {
                            Some(k) => {
                                k.2 += c.delta;
                                k.3 ^= c.mute;
                            }
                            None => knobs.push((c.knob, c.apps, c.delta, c.mute)),
                        }
                    }
                    for (knob, apps, delta, mute) in knobs {
                        let t = std::time::Instant::now();
                        let r = apply(&client, knob, &apps, delta, mute);
                        log::info!("knob {knob} {delta:+} mute={mute}: {:?} {:.2} in {} ms", r.name, r.volume, t.elapsed().as_millis());
                        let _ = tx.send(r);
                    }
                }
            })
            .expect("spawn knob thread");
        cmd_tx
    }

    /// Run an engine action off the main loop, so a slow action never stalls frames.
    pub fn run(&self, action: Action, value: Option<u8>) {
        let client = self.client.clone();
        std::thread::spawn(move || {
            if let Err(e) = client.call(Request::RunAction { action, value }) {
                log::warn!("action failed: {e:#}");
            }
        });
    }
}

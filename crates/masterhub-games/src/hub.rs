//! The connection to masterhubd's live surface (docs/LIVE_SURFACE.md).

use std::io::Cursor;
use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
use base64::Engine;
use image::{ImageEncoder, RgbImage, codecs::jpeg::JpegEncoder};
use masterhub_core::config::Action;
use masterhub_core::ipc::{Client, ClientEvent, Controls, Request};

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

    /// The default output's volume (0..=1) and mute, from the service.
    pub fn volume(&self) -> Option<(f32, bool)> {
        let v = self.client.call(Request::GetVolume).ok()?;
        Some((v["level"].as_f64()? as f32, v["muted"].as_bool().unwrap_or(false)))
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

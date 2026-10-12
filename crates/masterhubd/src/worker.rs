//! One thread per connected module: initialise it, then translate its input into actions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use masterhub_proto::{
    FeatureCommand, ImageTarget, InputReport, KEY_COUNT, KeyTracker, LedMode, LedTarget, ModuleKind, Rgb, Roll, Rotation,
    decode_input, image_packets, led_brightness, led_color, led_mode, parse_firmware_version,
};

use crate::actions::Request;
use crate::config::{Action, Config, Face, OneOrMany, Side};
use crate::hid::{HidDevice, ModuleInfo};
use crate::render::Renderer;

pub struct Shared {
    pub config: Config,
    pub renderer: Renderer,
    pub actions: Sender<Request>,
    pub shutdown: AtomicBool,
}

pub fn run(info: ModuleInfo, shared: Arc<Shared>) {
    let name = format!("{} {}", info.kind.name(), info.serial);
    match Worker::new(&info, shared).and_then(|mut w| w.run()) {
        Ok(()) => log::info!("{name}: stopped"),
        Err(e) => log::warn!("{name}: {e:#}"),
    }
}

struct Worker {
    dev: HidDevice,
    shared: Arc<Shared>,
    keys: KeyTracker<KEY_COUNT>,
    knob_buttons: KeyTracker<3>,
    faders: Option<[u8; 5]>,
}

impl Worker {
    fn new(info: &ModuleInfo, shared: Arc<Shared>) -> Result<Self> {
        Ok(Self {
            dev: HidDevice::open(info)?,
            shared,
            keys: KeyTracker::default(),
            knob_buttons: KeyTracker::default(),
            faders: None,
        })
    }

    fn kind(&self) -> ModuleKind {
        self.dev.info.kind
    }

    fn run(&mut self) -> Result<()> {
        self.init().context("initialising module")?;
        // Display modules only answer version queries once in software mode, so ask after init.
        let fw = self.dev.get_feature(self.kind().firmware_report()).ok().and_then(|r| parse_firmware_version(&r));
        log::info!("{} {} connected (fw {})", self.kind().name(), self.dev.info.serial, fw.as_deref().unwrap_or("?"));

        let mut buf = [0u8; 64];
        let result = loop {
            if self.shared.shutdown.load(Ordering::Relaxed) {
                break Ok(());
            }
            match self.dev.read_timeout(&mut buf, Duration::from_millis(250)) {
                Ok(Some(n)) => {
                    if let Some(report) = decode_input(self.kind(), &buf[..n]) {
                        self.handle(report);
                    }
                }
                Ok(None) => {}
                Err(e) => break Err(e),
            }
        };
        if self.shared.shutdown.load(Ordering::Relaxed) {
            self.release();
        }
        result
    }

    /// Everything resets on replug, so this runs on every connect.
    fn init(&mut self) -> Result<()> {
        let kind = self.kind();
        let shared = self.shared.clone();
        let cfg = &shared.config;
        if let Some(cmd) = FeatureCommand::sw_mode(kind, true)
            && self.dev.command(&cmd)?.is_empty()
        {
            // Firmware quirk: hardware mode was entered earlier (e.g. by the vendor app on exit)
            // and the module refuses software mode until power-cycled.
            log::error!(
                "{} {} ignored the switch to software mode; unplug and replug the MasterHUB",
                kind.name(),
                self.dev.info.serial
            );
        }
        if let Some(cmd) = FeatureCommand::backlight(kind, cfg.backlight) {
            self.dev.command(&cmd)?;
        }
        match kind {
            ModuleKind::Keypad => {
                for index in 0..KEY_COUNT as u8 {
                    match cfg.keys.iter().find(|k| k.index == index) {
                        Some(key) => self.show(ImageTarget::Key(index), &key.face, 72),
                        None => {
                            self.dev.command(&FeatureCommand::key_color(index, Rgb::default()))?;
                        }
                    }
                }
            }
            ModuleKind::Dial => {
                let face = cfg.dial.as_ref().map(|d| d.face.clone()).unwrap_or_default();
                self.show(ImageTarget::Dial, &face, 240);
            }
            ModuleKind::Base | ModuleKind::Fader | ModuleKind::Knob => self.init_leds()?,
            ModuleKind::Roller => {}
        }
        Ok(())
    }

    fn show(&mut self, target: ImageTarget, face: &Face, size: u32) {
        let jpeg = match self.shared.renderer.render(face, size) {
            Ok(j) => j,
            Err(e) => {
                log::error!("rendering {target:?}: {e:#}");
                return;
            }
        };
        for pkt in image_packets(target, &jpeg) {
            if let Err(e) = self.dev.write_output(&pkt) {
                log::error!("sending {target:?}: {e:#}");
                return;
            }
        }
    }

    fn init_leds(&mut self) -> Result<()> {
        let kind = self.kind();
        let shared = self.shared.clone();
        let leds = &shared.config.leds;
        let colors: Option<OneOrMany<_>> = match kind {
            ModuleKind::Base => leds.base.map(OneOrMany::One),
            ModuleKind::Fader => leds.fader.clone(),
            ModuleKind::Knob => leds.knob.clone(),
            _ => None,
        };
        let Some(colors) = colors else { return Ok(()) };
        let brightness = leds.brightness.unwrap_or(100);
        let mut reports = Vec::new();
        for i in 0..kind.led_count() {
            let Some(c) = colors.get(i as usize) else { continue };
            let t = LedTarget::Index(i);
            reports.extend(led_mode(kind, t, LedMode::Static));
            reports.extend(led_color(kind, t, c.0));
            reports.extend(led_brightness(kind, t, brightness));
        }
        for r in reports {
            self.dev.write_output(&r)?;
            // The full-speed modules poll their OUT endpoint every 3 ms.
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    /// Blank the screens when the service stops.
    ///
    /// Deliberately does NOT switch back to hardware mode: after `sw_mode(false)` the firmware
    /// ignores every later `sw_mode(true)` until the module is power-cycled, so the next
    /// service start would find dead keys and screens.
    fn release(&mut self) {
        if let Some(cmd) = FeatureCommand::screen_color(self.kind(), Rgb::default()) {
            let _ = self.dev.command(&cmd);
        }
    }

    fn handle(&mut self, report: InputReport) {
        let shared = self.shared.clone();
        let cfg = &shared.config;
        let mut fire: Vec<(Option<Action>, Option<u8>, Option<String>)> = Vec::new();
        match report {
            InputReport::Keys(state) => {
                for (i, pressed) in self.keys.update(state) {
                    log::debug!("key {i} {}", if pressed { "down" } else { "up" });
                    if let Some(k) = cfg.keys.iter().find(|k| k.index as usize == i) {
                        fire.push((if pressed { k.press.clone() } else { k.release.clone() }, None, None));
                    }
                }
            }
            InputReport::Faders(pos) => {
                let prev = self.faders.replace(pos);
                for i in 0..5 {
                    if prev.is_some_and(|p| p[i] == pos[i]) {
                        continue;
                    }
                    log::debug!("fader {i} = {}", pos[i]);
                    if let Some(f) = cfg.faders.iter().find(|f| f.index as usize == i) {
                        fire.push((f.change.clone(), Some(pos[i]), Some(format!("fader{i}"))));
                    }
                }
            }
            InputReport::Rollers { left, right } => {
                for (side, roll) in [(Side::Left, left), (Side::Right, right)] {
                    let Some(roll) = roll else { continue };
                    log::debug!("roller {side:?} {roll:?}");
                    if let Some(r) = cfg.rollers.iter().find(|r| r.side == side) {
                        fire.push((if roll == Roll::Away { r.away.clone() } else { r.toward.clone() }, None, None));
                    }
                }
            }
            InputReport::KnobTurn(turns) => {
                for (i, rot) in turns.into_iter().enumerate() {
                    let Some(rot) = rot else { continue };
                    log::debug!("knob {i} {rot:?}");
                    if let Some(k) = cfg.knobs.iter().find(|k| k.index as usize == i) {
                        fire.push((if rot == Rotation::Clockwise { k.cw.clone() } else { k.ccw.clone() }, None, None));
                    }
                }
            }
            InputReport::KnobButtons(state) => {
                for (i, pressed) in self.knob_buttons.update(state) {
                    log::debug!("knob {i} {}", if pressed { "down" } else { "up" });
                    if let Some(k) = cfg.knobs.iter().find(|k| k.index as usize == i) {
                        fire.push((if pressed { k.press.clone() } else { k.release.clone() }, None, None));
                    }
                }
            }
            InputReport::DialTurn(rot) => {
                log::debug!("dial {rot:?}");
                if let Some(d) = &cfg.dial {
                    fire.push((if rot == Rotation::Clockwise { d.cw.clone() } else { d.ccw.clone() }, None, None));
                }
            }
            InputReport::DialButton { pressed } => {
                log::debug!("dial {}", if pressed { "down" } else { "up" });
                if let Some(d) = &cfg.dial {
                    fire.push((if pressed { d.press.clone() } else { d.release.clone() }, None, None));
                }
            }
            InputReport::BaseOrientation { .. } => {}
            InputReport::Unknown { kind, payload } => {
                log::debug!("{}: unknown report {kind:#04x} {payload:02x?}", self.kind().name());
            }
        }
        for (action, value, coalesce) in fire {
            if let Some(action) = action {
                let _ = self.shared.actions.send(Request { action, value, coalesce });
            }
        }
    }
}

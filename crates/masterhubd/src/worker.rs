//! One thread per connected module. Pure I/O: it initialises the module, forwards decoded
//! input to the controller and executes display/LED commands from it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::Duration;

use anyhow::{Context, Result};
use masterhub_proto::{FeatureCommand, ImageTarget, InputReport, Rgb, decode_input, image_packets, parse_firmware_version};

use crate::controller::Msg;
use crate::hid::{HidDevice, ModuleInfo};

/// Commands from the controller to a worker.
pub enum DeviceCmd {
    Image { target: ImageTarget, jpeg: Arc<Vec<u8>> },
    /// Raw 64-byte output reports (LED commands).
    Output(Vec<Vec<u8>>),
    Backlight(u8),
}

/// Runs until the module disconnects, the service shuts down or `resumes` changes (the system
/// resumed from suspend); the main loop then starts a new worker, which initialises the module
/// again.
pub fn run(info: ModuleInfo, controller: Sender<Msg>, shutdown: Arc<AtomicBool>, resumes: Arc<AtomicU64>) {
    let name = format!("{} {}", info.kind.name(), info.serial);
    let (tx, rx) = channel();
    let result = HidDevice::open(&info).and_then(|dev| {
        let resumed = resumes.load(Ordering::Relaxed);
        let mut w = Worker { dev, controller: controller.clone(), shutdown, resumes, resumed };
        w.run(tx, rx)
    });
    if let Err(e) = result {
        log::warn!("{name}: {e:#}");
    }
    let _ = controller.send(Msg::Disconnected { serial: info.serial });
}

struct Worker {
    dev: HidDevice,
    controller: Sender<Msg>,
    shutdown: Arc<AtomicBool>,
    resumes: Arc<AtomicU64>,
    /// `resumes` when this worker started.
    resumed: u64,
}

impl Worker {
    fn run(&mut self, tx: Sender<DeviceCmd>, rx: Receiver<DeviceCmd>) -> Result<()> {
        let kind = self.dev.info.kind;
        let mut needs_replug = false;
        if let Some(cmd) = FeatureCommand::sw_mode(kind, true)
            && self.dev.command(&cmd)?.is_empty()
        {
            needs_replug = true;
            // Firmware quirk: hardware mode was entered earlier (e.g. by the vendor app on exit)
            // and the module refuses software mode until power-cycled.
            log::error!(
                "{} {} ignored the switch to software mode; unplug and replug the MasterHUB",
                kind.name(),
                self.dev.info.serial
            );
        }
        // Display modules only answer version queries once in software mode.
        let firmware = self.dev.get_feature(kind.firmware_report()).ok().and_then(|r| parse_firmware_version(&r));
        log::info!("{} {} connected (fw {})", kind.name(), self.dev.info.serial, firmware.as_deref().unwrap_or("?"));
        self.controller
            .send(Msg::Connected { info: self.dev.info.clone(), firmware, commands: tx, needs_replug })
            .context("controller gone")?;

        let mut buf = [0u8; 64];
        let result = loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break Ok(());
            }
            if self.resumes.load(Ordering::Relaxed) != self.resumed {
                log::info!("{} {}: reconnecting after resume", kind.name(), self.dev.info.serial);
                break Ok(());
            }
            if let Err(e) = self.drain(&rx) {
                break Err(e);
            }
            match self.dev.read_timeout(&mut buf, Duration::from_millis(15)) {
                Ok(Some(n)) => {
                    if let Some(report) = decode_input(kind, &buf[..n])
                        && !matches!(report, InputReport::Unknown { .. })
                    {
                        let _ = self.controller.send(Msg::Input { serial: self.dev.info.serial.clone(), report });
                    }
                }
                Ok(None) => {}
                Err(e) => break Err(e),
            }
        };
        if self.shutdown.load(Ordering::Relaxed) {
            // Blank the screens. Deliberately no switch back to hardware mode: the firmware
            // would then ignore software mode until power-cycled.
            if let Some(cmd) = FeatureCommand::screen_color(kind, Rgb::default()) {
                let _ = self.dev.command(&cmd);
            }
        }
        result
    }

    fn drain(&mut self, rx: &Receiver<DeviceCmd>) -> Result<()> {
        // Only the newest image per target: a client pushing frames faster than the module
        // takes them must not build up lag.
        let mut cmds: Vec<DeviceCmd> = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(DeviceCmd::Image { target, jpeg }) => {
                    cmds.retain(|c| !matches!(c, DeviceCmd::Image { target: t, .. } if *t == target));
                    cmds.push(DeviceCmd::Image { target, jpeg });
                }
                // LED reports: a newer one for the same command and LED replaces the older,
                // so a fast stream of LED changes can't back up behind the 4 ms pacing.
                Ok(DeviceCmd::Output(reports)) => {
                    for r in reports {
                        let key = |x: &[u8]| (x.get(1).copied(), x.get(2).copied(), x.get(3).copied());
                        for c in cmds.iter_mut() {
                            if let DeviceCmd::Output(old) = c {
                                old.retain(|o| key(o) != key(&r));
                            }
                        }
                        cmds.push(DeviceCmd::Output(vec![r]));
                    }
                }
                Ok(cmd) => cmds.push(cmd),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        for cmd in cmds {
            match cmd {
                DeviceCmd::Image { target, jpeg } => {
                    for pkt in image_packets(target, &jpeg) {
                        self.dev.write_output(&pkt)?;
                    }
                }
                DeviceCmd::Output(reports) => {
                    for r in reports {
                        self.dev.write_output(&r)?;
                        // The full-speed modules poll their OUT endpoint every 3 ms.
                        std::thread::sleep(Duration::from_millis(4));
                    }
                }
                DeviceCmd::Backlight(level) => {
                    if let Some(cmd) = FeatureCommand::backlight(self.dev.info.kind, level) {
                        self.dev.command(&cmd)?;
                    }
                }
            }
        }
        Ok(())
    }
}

//! One thread per connected module. Pure I/O: it initialises the module, forwards decoded
//! input to the controller and executes display/LED commands from it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

pub fn run(info: ModuleInfo, controller: Sender<Msg>, shutdown: Arc<AtomicBool>) {
    let name = format!("{} {}", info.kind.name(), info.serial);
    let (tx, rx) = channel();
    let result = HidDevice::open(&info).and_then(|dev| {
        let mut w = Worker { dev, controller: controller.clone(), shutdown };
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
}

impl Worker {
    fn run(&mut self, tx: Sender<DeviceCmd>, rx: Receiver<DeviceCmd>) -> Result<()> {
        let kind = self.dev.info.kind;
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
        // Display modules only answer version queries once in software mode.
        let firmware = self.dev.get_feature(kind.firmware_report()).ok().and_then(|r| parse_firmware_version(&r));
        log::info!("{} {} connected (fw {})", kind.name(), self.dev.info.serial, firmware.as_deref().unwrap_or("?"));
        self.controller
            .send(Msg::Connected { info: self.dev.info.clone(), firmware, commands: tx })
            .context("controller gone")?;

        let mut buf = [0u8; 64];
        let result = loop {
            if self.shutdown.load(Ordering::Relaxed) {
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
        loop {
            match rx.try_recv() {
                Ok(DeviceCmd::Image { target, jpeg }) => {
                    for pkt in image_packets(target, &jpeg) {
                        self.dev.write_output(&pkt)?;
                    }
                }
                Ok(DeviceCmd::Output(reports)) => {
                    for r in reports {
                        self.dev.write_output(&r)?;
                        // The full-speed modules poll their OUT endpoint every 3 ms.
                        std::thread::sleep(Duration::from_millis(4));
                    }
                }
                Ok(DeviceCmd::Backlight(level)) => {
                    if let Some(cmd) = FeatureCommand::backlight(self.dev.info.kind, level) {
                        self.dev.command(&cmd)?;
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }
}

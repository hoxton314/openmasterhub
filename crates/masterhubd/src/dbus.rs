//! D-Bus API (see `masterhub_core::dbus` for the contract). Methods answer from the shared
//! snapshot and hand changes to the controller without waiting for it, so the bus thread
//! never blocks on the controller (which emits signals through the same connection).

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use masterhub_core::config::Config;
use zbus::fdo;
use zbus::object_server::SignalEmitter;

use crate::controller::{Msg, Snapshot};

pub struct Api {
    pub controller: Sender<Msg>,
    pub snapshot: Arc<Mutex<Snapshot>>,
}

fn failed(e: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(e.to_string())
}

#[zbus::interface(name = "org.openmasterhub.Daemon1")]
impl Api {
    fn get_config(&self) -> String {
        self.snapshot.lock().unwrap().config_text.clone()
    }

    /// Validate, save and apply a new config.
    fn set_config(&self, toml: String) -> fdo::Result<()> {
        let path = self.snapshot.lock().unwrap().config_path.clone();
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let config = Config::parse(&toml, dir).map_err(|e| failed(format!("{e:#}")))?;
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(failed)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &toml).map_err(failed)?;
        std::fs::rename(&tmp, &path).map_err(failed)?;
        log::info!("config replaced over D-Bus");
        self.controller.send(Msg::ApplyConfig { config: Box::new(config), text: toml }).map_err(failed)
    }

    fn get_config_path(&self) -> String {
        self.snapshot.lock().unwrap().config_path.display().to_string()
    }

    fn get_modules(&self) -> String {
        serde_json::to_string(&self.snapshot.lock().unwrap().modules).unwrap_or_default()
    }

    fn get_state(&self) -> String {
        serde_json::to_string(&self.snapshot.lock().unwrap().state).unwrap_or_default()
    }

    fn get_displays(&self) -> Vec<(String, u8, Vec<u8>)> {
        self.snapshot
            .lock()
            .unwrap()
            .displays
            .iter()
            .map(|((serial, target), jpeg)| (serial.clone(), *target, jpeg.to_vec()))
            .collect()
    }

    fn set_profile(&self, name: String) -> fdo::Result<()> {
        if !self.snapshot.lock().unwrap().config.profiles.iter().any(|p| p.name == name) {
            return Err(fdo::Error::InvalidArgs(format!("no profile {name:?}")));
        }
        self.controller.send(Msg::SetProfile(name)).map_err(failed)
    }

    fn show_page(&self, name: String) -> fdo::Result<()> {
        self.controller.send(Msg::ShowPage(name)).map_err(failed)
    }

    #[zbus(signal)]
    async fn input(emitter: &SignalEmitter<'_>, json: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, json: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn modules_changed(emitter: &SignalEmitter<'_>, json: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn display_changed(emitter: &SignalEmitter<'_>, serial: &str, target: u8, jpeg: &[u8]) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn config_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

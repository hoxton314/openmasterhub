//! D-Bus client for masterhubd. Method calls are blocking (they are local and fast); signals
//! are read on background threads and forwarded to the GTK main loop as [`Event`]s.

use std::thread;

use anyhow::{Context, Result};
use masterhub_core::dbus::{BUS_NAME, HubState, INTERFACE, InputMsg, ModuleState, OBJECT_PATH};
use zbus::blocking::{Connection, Proxy};

pub enum Event {
    /// The service appeared on (true) or left (false) the bus.
    Online(bool),
    Input(InputMsg),
    State(HubState),
    Modules(Vec<ModuleState>),
    Display { serial: String, target: u8, jpeg: Vec<u8> },
    ConfigChanged,
}

#[derive(Clone)]
pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn connect(events: async_channel::Sender<Event>) -> Result<Self> {
        let conn = Connection::session().context("connecting to the session bus")?;
        spawn_signal_thread(conn.clone(), events.clone())?;
        spawn_owner_thread(conn.clone(), events)?;
        Ok(Self { conn })
    }

    fn proxy(&self) -> Result<Proxy<'static>> {
        Ok(Proxy::new(&self.conn, BUS_NAME, OBJECT_PATH, INTERFACE)?)
    }

    pub fn is_online(&self) -> bool {
        let Ok(proxy) = zbus::blocking::fdo::DBusProxy::new(&self.conn) else { return false };
        let Ok(name) = BUS_NAME.try_into() else { return false };
        proxy.name_has_owner(name).unwrap_or(false)
    }

    pub fn config(&self) -> Result<String> {
        Ok(self.proxy()?.call("GetConfig", &())?)
    }

    pub fn config_path(&self) -> Result<String> {
        Ok(self.proxy()?.call("GetConfigPath", &())?)
    }

    pub fn set_config(&self, toml: &str) -> Result<()> {
        self.proxy()?.call::<_, _, ()>("SetConfig", &(toml,)).map_err(|e| match e {
            zbus::Error::MethodError(_, Some(msg), _) => anyhow::anyhow!(msg),
            e => e.into(),
        })
    }

    pub fn modules(&self) -> Result<Vec<ModuleState>> {
        let json: String = self.proxy()?.call("GetModules", &())?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn state(&self) -> Result<HubState> {
        let json: String = self.proxy()?.call("GetState", &())?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn displays(&self) -> Result<Vec<(String, u8, Vec<u8>)>> {
        Ok(self.proxy()?.call("GetDisplays", &())?)
    }

    pub fn set_profile(&self, name: &str) -> Result<()> {
        Ok(self.proxy()?.call("SetProfile", &(name,))?)
    }

    pub fn show_page(&self, name: &str) -> Result<()> {
        Ok(self.proxy()?.call("ShowPage", &(name,))?)
    }
}

fn spawn_signal_thread(conn: Connection, events: async_channel::Sender<Event>) -> Result<()> {
    let proxy = Proxy::new(&conn, BUS_NAME, OBJECT_PATH, INTERFACE)?;
    let signals = proxy.receive_all_signals()?;
    thread::Builder::new().name("dbus-signals".into()).spawn(move || {
        let _proxy = proxy;
        for msg in signals {
            let header = msg.header();
            let Some(member) = header.member() else { continue };
            let body = msg.body();
            let event = match member.as_str() {
                "Input" => body.deserialize::<String>().ok().and_then(|j| serde_json::from_str(&j).ok()).map(Event::Input),
                "StateChanged" => body.deserialize::<String>().ok().and_then(|j| serde_json::from_str(&j).ok()).map(Event::State),
                "ModulesChanged" => body.deserialize::<String>().ok().and_then(|j| serde_json::from_str(&j).ok()).map(Event::Modules),
                "DisplayChanged" => body
                    .deserialize::<(String, u8, Vec<u8>)>()
                    .ok()
                    .map(|(serial, target, jpeg)| Event::Display { serial, target, jpeg }),
                "ConfigChanged" => Some(Event::ConfigChanged),
                _ => None,
            };
            if let Some(e) = event
                && events.send_blocking(e).is_err()
            {
                return;
            }
        }
    })?;
    Ok(())
}

/// Watch `NameOwnerChanged` for the service, so the GUI notices it starting and stopping.
fn spawn_owner_thread(conn: Connection, events: async_channel::Sender<Event>) -> Result<()> {
    let proxy = Proxy::new(&conn, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")?;
    let signals = proxy.receive_signal("NameOwnerChanged")?;
    thread::Builder::new().name("dbus-owner".into()).spawn(move || {
        let _proxy = proxy;
        for msg in signals {
            let Ok((name, _old, new)) = msg.body().deserialize::<(String, String, String)>() else { continue };
            if name == BUS_NAME && events.send_blocking(Event::Online(!new.is_empty())).is_err() {
                return;
            }
        }
    })?;
    Ok(())
}

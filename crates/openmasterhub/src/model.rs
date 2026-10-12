//! GUI state: the config being edited, live hub state and what is selected.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gtk4::cairo;
use masterhub_core::config::{Config, DialConfig, FaderConfig, KeyConfig, KnobConfig, Page, Profile, RollerConfig, Side};
use masterhub_core::dbus::{HubState, ModuleState};

/// What the inspector edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    /// A module itself (body click): info and lighting.
    Module(String),
    Key(u8),
    Dial,
    Knob(u8),
    Fader(u8),
    Roller(Side),
}

pub struct Model {
    pub config: Config,
    pub config_dir: PathBuf,
    /// TOML last sent to the service, to recognise our own ConfigChanged echoes.
    pub last_sent: Option<String>,
    pub modules: Vec<ModuleState>,
    pub hub: HubState,
    pub displays: HashMap<(String, u8), cairo::ImageSurface>,
    pub selection: Selection,
    /// Recent input per (serial, control, index), for the glow in the live view.
    pub flashes: HashMap<(String, String, u8), Instant>,
    pub fader_pos: HashMap<(String, u8), u8>,
    pub online: bool,
}

pub const FLASH: Duration = Duration::from_millis(280);

impl Model {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            config_dir: PathBuf::from("."),
            last_sent: None,
            modules: Vec::new(),
            hub: HubState::default(),
            displays: HashMap::new(),
            selection: Selection::None,
            flashes: HashMap::new(),
            fader_pos: HashMap::new(),
            online: false,
        }
    }

    pub fn profile_index(&self) -> Option<usize> {
        self.config.profiles.iter().position(|p| p.name == self.hub.profile).or((!self.config.profiles.is_empty()).then_some(0))
    }

    pub fn profile(&self) -> Option<&Profile> {
        self.config.profiles.get(self.profile_index()?)
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        if self.config.profiles.is_empty() {
            self.config.profiles.push(Profile { name: "Default".into(), ..Default::default() });
            self.hub.profile = "Default".into();
        }
        let i = self.profile_index().unwrap_or(0);
        &mut self.config.profiles[i]
    }

    /// The keypad page or folder shown on the hub (and edited in the GUI).
    pub fn page(&self) -> Option<&Page> {
        let p = self.profile()?;
        p.pages.iter().chain(&p.folders).find(|x| x.name == self.hub.page).or(p.pages.first())
    }

    pub fn page_mut(&mut self) -> &mut Page {
        let name = self.hub.page.clone();
        let p = self.profile_mut();
        if p.pages.is_empty() {
            p.pages.push(Page { name: "Main".into(), keys: Vec::new() });
        }
        let in_pages = p.pages.iter().position(|x| x.name == name);
        let in_folders = p.folders.iter().position(|x| x.name == name);
        match (in_pages, in_folders) {
            (Some(i), _) => &mut p.pages[i],
            (None, Some(i)) => &mut p.folders[i],
            (None, None) => &mut p.pages[0],
        }
    }

    pub fn key(&self, index: u8) -> Option<&KeyConfig> {
        self.page()?.key(index)
    }

    /// The key config for `index`, created if missing.
    pub fn key_mut(&mut self, index: u8) -> &mut KeyConfig {
        let page = self.page_mut();
        let pos = match page.keys.iter().position(|k| k.index == index) {
            Some(p) => p,
            None => {
                page.keys.push(KeyConfig::new(index));
                page.keys.sort_by_key(|k| k.index);
                page.keys.iter().position(|k| k.index == index).expect("just inserted")
            }
        };
        &mut page.keys[pos]
    }

    pub fn dial_mut(&mut self) -> &mut DialConfig {
        self.profile_mut().dial.get_or_insert_with(DialConfig::default)
    }

    pub fn knob_mut(&mut self, index: u8) -> &mut KnobConfig {
        let p = self.profile_mut();
        if !p.knobs.iter().any(|k| k.index == index) {
            p.knobs.push(KnobConfig { index, cw: None, ccw: None, press: None, release: None });
        }
        p.knobs.iter_mut().find(|k| k.index == index).expect("just inserted")
    }

    pub fn fader_mut(&mut self, index: u8) -> &mut FaderConfig {
        let p = self.profile_mut();
        if !p.faders.iter().any(|f| f.index == index) {
            p.faders.push(FaderConfig { index, change: None });
        }
        p.faders.iter_mut().find(|f| f.index == index).expect("just inserted")
    }

    pub fn roller_mut(&mut self, side: Side) -> &mut RollerConfig {
        let p = self.profile_mut();
        if !p.rollers.iter().any(|r| r.side == side) {
            p.rollers.push(RollerConfig { side, away: None, toward: None });
        }
        p.rollers.iter_mut().find(|r| r.side == side).expect("just inserted")
    }

    /// Drop empty entries so the saved TOML stays tidy.
    pub fn tidy(&mut self) {
        for p in &mut self.config.profiles {
            for page in p.pages.iter_mut().chain(p.folders.iter_mut()) {
                page.keys.retain(|k| k.face != Default::default() || k.press.is_some() || k.release.is_some());
            }
            p.knobs.retain(|k| k.cw.is_some() || k.ccw.is_some() || k.press.is_some() || k.release.is_some());
            p.faders.retain(|f| f.change.is_some());
            p.rollers.retain(|r| r.away.is_some() || r.toward.is_some());
            if p.dial.as_ref().is_some_and(|d| *d == DialConfig::default()) {
                p.dial = None;
            }
        }
    }

    pub fn flash(&mut self, serial: &str, control: &str, index: u8) {
        self.flashes.insert((serial.to_owned(), control.to_owned(), index), Instant::now());
    }

    pub fn is_flashing(&self, serial: &str, control: &str, index: u8) -> bool {
        self.flashes
            .get(&(serial.to_owned(), control.to_owned(), index))
            .is_some_and(|t| t.elapsed() < FLASH)
    }
}

/// Decode a JPEG from the service into a cairo surface for the live view.
pub fn jpeg_surface(jpeg: &[u8]) -> Option<cairo::ImageSurface> {
    let img = image::load_from_memory(jpeg).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w as i32, h as i32).ok()?;
    let stride = surface.stride() as usize;
    {
        let mut data = surface.data().ok()?;
        for (y, row) in img.rows().enumerate() {
            for (x, p) in row.enumerate() {
                let o = y * stride + x * 4;
                // ARGB32 is native-endian: B, G, R, A in memory on little-endian. Opaque, so no
                // premultiplication needed.
                data[o] = p[2];
                data[o + 1] = p[1];
                data[o + 2] = p[0];
                data[o + 3] = 255;
            }
        }
    }
    Some(surface)
}

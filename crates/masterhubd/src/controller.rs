//! The controller owns all hub state (profile, page, folder stack, what every display shows)
//! and is the only place that decides what happens on input. Workers do I/O, the IPC server
//! reads the shared [`Snapshot`].

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use masterhub_core::config::{Action, AudioTarget, Color, Config, Face, Leds, Page, SessionSort, Side, Widget};
use masterhub_core::ipc::{
    ClaimState, Controls, DisplayImage, Event, GUI_APP_ID, HubState, InputMsg, LedZone, ModuleState, Status, TARGET_DIAL,
};
use masterhub_core::layout;
use masterhub_core::render::{DIAL_SIZE, KEY_SIZE, LiveData, Picture, Renderer, load_picture};
use masterhub_proto::{
    ImageTarget, InputReport, KEY_COUNT, KeyTracker, LedMode, LedTarget, ModuleKind, Rgb, Roll, Rotation,
    led_brightness, led_color, led_mode,
};

use crate::actions::Request;
use crate::claude::SessionFile;
use crate::claude::desktop::View;
use crate::ipc::Broadcaster;
use crate::hid::ModuleInfo;
use crate::stats::Stats;
use crate::surface::{Claim, Claims, Control, LedState};
use crate::worker::DeviceCmd;

pub enum Msg {
    Connected { info: ModuleInfo, firmware: Option<String>, commands: Sender<DeviceCmd> },
    Disconnected { serial: String },
    Input { serial: String, report: InputReport },
    ActiveApp(String),
    /// The focused window's title; sent just before its `ActiveApp` (and when it changes).
    /// Only the Hyprland backend sends it so far.
    #[cfg_attr(windows, allow(dead_code))]
    ActiveTitle(String),
    /// A validated config from the app (IPC) (already saved to disk) with its TOML text.
    ApplyConfig { config: Box<Config>, text: String },
    SetProfile(String),
    ShowPage(String),
    /// A tool's status for the Base LED; `None` forgets the source. `background`: from work in
    /// the background of the source (see [`reported_status`]).
    SetStatus {
        source: String,
        status: Option<Status>,
        pid: Option<u32>,
        background: bool,
        detail: Option<masterhub_core::ipc::SessionDetail>,
    },
    /// Live surface (see [`crate::surface`]).
    Claim { claim: u64, name: String, controls: Controls, apps: Vec<String>, priority: i32 },
    Release { claim: u64 },
    Backlight { claim: u64, level: Option<u8> },
    /// An action from a client (`run_action`).
    RunAction { action: Action, value: Option<u8> },
    Frame { claim: u64, target: u8, jpeg: Arc<Vec<u8>> },
    Led { claim: u64, zone: LedZone, mode: Option<LedMode>, color: Option<Rgb>, brightness: Option<u8> },
}

/// State shared with the IPC server.
#[derive(Default)]
pub struct Snapshot {
    pub config: Config,
    pub config_text: String,
    pub config_path: PathBuf,
    pub modules: Vec<ModuleState>,
    pub state: HubState,
    pub displays: BTreeMap<(String, u8), Arc<Vec<u8>>>,
    pub sessions: Vec<masterhub_core::ipc::SessionInfo>,
}

struct Module {
    info: ModuleInfo,
    firmware: Option<String>,
    tx: Sender<DeviceCmd>,
    keys: KeyTracker<KEY_COUNT>,
    knob_buttons: KeyTracker<3>,
    faders: Option<[u8; 5]>,
    /// What each display target currently shows.
    shown: HashMap<u8, Arc<Vec<u8>>>,
    /// Where the module sits on the Base, once the Base is known.
    placement: Option<layout::Placement>,
}

struct Anim {
    frame: usize,
    due: Instant,
}

/// A tool that reports its status, such as a Claude Code session (`claude:<session id>`).
struct Reporter {
    status: Status,
    pid: Option<u32>,
    /// When it first reported, in `Controller::reports`.
    first: u64,
    /// When its status last changed, in `Controller::reports`.
    changed: u64,
    /// When its status last changed, in milliseconds since the epoch.
    changed_at: u64,
    /// When the Claude desktop app last showed it, for sessions of the app.
    view: Option<View>,
    /// When its key on the hub was last pressed, in milliseconds since the epoch (0: never).
    pressed: u64,
    /// Its session file said a prompt waits for the user at the last tick.
    waiting: bool,
    /// What Claude Code wrote about the session, re-read every tick (the name changes).
    file: Option<SessionFile>,
    /// Found in the session files rather than reported by a hook; its status follows the file
    /// until the first hook event.
    discovered: bool,
    /// Runs on another machine over SSH, known from the desktop app's session files only.
    remote: bool,
    /// When it last reported.
    reported: Instant,
    /// What it is doing (Claude Code sessions), from its last hook event that said so.
    detail: masterhub_core::ipc::SessionDetail,
}

pub struct Controller {
    config: Config,
    config_path: PathBuf,
    config_mtime: Option<SystemTime>,
    renderer: Renderer,
    pictures: HashMap<(PathBuf, u32), Option<Rc<Picture>>>,
    modules: BTreeMap<String, Module>,
    profile: usize,
    /// The profile shown before the current one, for `back` outside a folder.
    previous_profile: Option<usize>,
    page: usize,
    folders: Vec<String>,
    /// Last orientation reported by the Base's sensor, if any.
    sensor_horizontal: Option<bool>,
    active_app: Option<String>,
    active_title: Option<String>,
    anims: HashMap<(String, u8), Anim>,
    stats: Stats,
    live: HashMap<Widget, LiveData>,
    actions: Sender<Request>,
    snapshot: Arc<Mutex<Snapshot>>,
    events: Arc<Broadcaster>,
    /// Tool status per source, with the process that reported it.
    statuses: HashMap<String, Reporter>,
    /// The Claude desktop app's sessions, for those it runs over SSH.
    desktop: crate::claude::desktop::DesktopSessions,
    /// Counts status reports, to order sessions that started at the same time.
    reports: u64,
    /// The first session shown on the session keys, when there are more sessions than keys
    /// (scrolled a page at a time).
    session_offset: usize,
    /// Advances every tick; animates working sessions.
    ticks: u32,
    /// Status on the Base LED, whether a flash is lit and when it toggles next.
    shown_status: Option<Status>,
    blink_on: bool,
    blink_due: Option<Instant>,
    /// Live-surface claims and who owns which control.
    claims: Claims,
    /// Claimed frames change fast: their `display` events go out at most every
    /// [`DISPLAY_EVERY`]; the newest is sent on the next tick.
    display_sent: HashMap<(String, u8), Instant>,
    display_pending: BTreeMap<(String, u8), Arc<Vec<u8>>>,
    /// A client's backlight level and its claim; the config's level otherwise.
    backlight: Option<(u64, u8)>,
}

const DISPLAY_EVERY: Duration = Duration::from_millis(100);

const TICK: Duration = Duration::from_secs(1);
/// How long a Claude Code session stays `working` while its process is idle and nothing reports:
/// background agents report their tool calls, so a missed wake-up can't keep it blue forever.
const BACKGROUND_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Half period of the attention flash.
const BLINK: Duration = Duration::from_millis(500);

impl Controller {
    pub fn new(
        config: Config,
        config_text: String,
        config_path: PathBuf,
        actions: Sender<Request>,
        snapshot: Arc<Mutex<Snapshot>>,
        events: Arc<Broadcaster>,
    ) -> Self {
        {
            let mut s = snapshot.lock().unwrap();
            s.config = config.clone();
            s.config_text = config_text;
            s.config_path = config_path.clone();
        }
        if config.orientation.is_auto() {
            // The sensor only reports some changes and nothing while the Base lies flat.
            log::info!(
                "orientation = \"auto\": no tilt sensor reading yet, assuming landscape \
                 (set orientation in the config to pin it)"
            );
        }
        let c = Self {
            renderer: Renderer::new(config.font.as_deref()),
            config_mtime: mtime(&config_path),
            config,
            config_path,
            pictures: HashMap::new(),
            modules: BTreeMap::new(),
            profile: 0,
            previous_profile: None,
            page: 0,
            folders: Vec::new(),
            sensor_horizontal: None,
            active_app: None,
            active_title: None,
            anims: HashMap::new(),
            stats: Stats::new(),
            live: HashMap::new(),
            actions,
            snapshot,
            events,
            statuses: HashMap::new(),
            desktop: Default::default(),
            reports: 0,
            session_offset: 0,
            ticks: 0,
            shown_status: None,
            blink_on: true,
            blink_due: None,
            claims: Claims::default(),
            display_sent: HashMap::new(),
            display_pending: BTreeMap::new(),
            backlight: None,
        };
        c.publish_state();
        c
    }

    pub fn run(mut self, rx: Receiver<Msg>) {
        let mut next_tick = Instant::now();
        loop {
            let now = Instant::now();
            let next_anim = self.anims.values().map(|a| a.due).chain(self.blink_due).min();
            let deadline = next_anim.map_or(next_tick, |a| a.min(next_tick));
            match rx.recv_timeout(deadline.saturating_duration_since(now)) {
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            let now = Instant::now();
            if now >= next_tick {
                next_tick = now + TICK;
                self.ticks = self.ticks.wrapping_add(1);
                self.check_config_file();
                self.prune_statuses();
                self.sample_widgets();
                self.refresh_all();
                self.flush_displays();
            }
            self.advance_animations(now);
            self.advance_blink(now);
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Connected { info, firmware, commands } => {
                let serial = info.serial.clone();
                self.modules.insert(
                    serial.clone(),
                    Module {
                        info,
                        firmware,
                        tx: commands,
                        keys: KeyTracker::default(),
                        knob_buttons: KeyTracker::default(),
                        faders: None,
                        shown: HashMap::new(),
                        placement: None,
                    },
                );
                self.init_module(&serial);
                self.publish_modules();
            }
            Msg::Disconnected { serial } => {
                if self.modules.remove(&serial).is_some() {
                    log::info!("{serial} disconnected");
                    self.anims.retain(|(s, _), _| s != &serial);
                    self.snapshot.lock().unwrap().displays.retain(|(s, _), _| s != &serial);
                    self.publish_modules();
                }
            }
            Msg::Input { serial, report } => self.input(&serial, report),
            Msg::ActiveTitle(title) => {
                let changed = self.active_title.as_deref() != Some(&title);
                self.active_title = Some(title);
                if changed {
                    self.reclaim();
                }
            }
            // The GUI's own window: GTK uses the app ID, Tauri the binary name (openmasterhub[.exe]).
            Msg::ActiveApp(class) if class == GUI_APP_ID || class.to_ascii_lowercase().starts_with("openmasterhub") => {}
            Msg::ActiveApp(class) => {
                let target = self.config.profile_for_app(&class);
                self.active_app = Some(class);
                self.reclaim();
                if target != self.profile {
                    log::info!("focused {:?}: profile {:?}", self.active_app, self.config.profiles[target].name);
                    self.switch_profile(target);
                } else {
                    self.publish_state();
                }
            }
            Msg::ApplyConfig { config, text } => {
                self.config_mtime = mtime(&self.config_path);
                self.apply_config(*config, text);
            }
            Msg::SetProfile(name) => {
                if let Some(i) = self.config.profiles.iter().position(|p| p.name == name) {
                    self.switch_profile(i);
                }
            }
            Msg::ShowPage(name) => self.show_page(&name),
            Msg::SetStatus { source, status, pid, background, detail } => {
                log::debug!("status {source}: {status:?}{}", if background { " (background)" } else { "" });
                match status {
                    Some(status) => {
                        let file = pid.filter(|_| source.starts_with("claude:")).and_then(SessionFile::of);
                        let old = self.statuses.remove(&source);
                        let status = reported_status(old.as_ref().map(|r| r.status), status, background);
                        let mut r = old.unwrap_or_else(|| Reporter::new(status, pid, None, &mut self.reports));
                        r.set(status, &mut self.reports);
                        if let Some(d) = detail {
                            r.detail = d;
                        }
                        (r.pid, r.file, r.discovered, r.remote, r.reported) = (pid, file, false, false, Instant::now());
                        self.statuses.insert(source, r);
                    }
                    None => {
                        self.statuses.remove(&source);
                    }
                }
                self.update_status();
                self.refresh_sessions();
                self.publish_sessions();
            }
            Msg::Claim { claim, name, controls, apps, priority } => {
                log::info!("{name} claims {controls:?} (apps {apps:?}, priority {priority}) as #{claim}");
                let controls = crate::surface::expand(&controls);
                let c = Claim { name, controls, apps, priority, frames: HashMap::new(), leds: HashMap::new() };
                self.claims.insert(claim, c);
                self.reclaim();
                self.publish_claim(claim);
            }
            Msg::Backlight { claim, level } => {
                self.backlight = level.map(|l| (claim, l.min(100)));
                self.send_backlight();
            }
            Msg::Release { claim } => {
                if self.backlight.is_some_and(|(c, _)| c == claim) {
                    self.backlight = None;
                    self.send_backlight();
                }
                if self.claims.remove(claim) {
                    log::info!("claim #{claim} released");
                    self.reclaim();
                }
            }
            Msg::RunAction { action, value } => self.run_action(action, value, None),
            Msg::Frame { claim, target, jpeg } => {
                let Some(c) = self.claims.get_mut(claim) else { return };
                c.frames.insert(target, jpeg);
                if self.claims.owner(Control::of_target(target)) == Some(claim) {
                    let module = if target == TARGET_DIAL { ModuleKind::Dial } else { ModuleKind::Keypad };
                    for serial in self.serials_of(module) {
                        self.refresh_target(&serial, target);
                    }
                }
            }
            Msg::Led { claim, zone, mode, color, brightness } => {
                let Some(c) = self.claims.get_mut(claim) else { return };
                let st = c.leds.entry(zone).or_default();
                st.mode = mode.or(st.mode);
                st.color = color.or(st.color);
                st.brightness = brightness.or(st.brightness);
                if self.claims.owner(Control::Led(zone)) == Some(claim) {
                    // Only this LED, only what changed: LED reports are slow (one per 4 ms).
                    self.send_one_led(zone, mode, color, brightness);
                }
            }
        }
    }

    // ───────────────────────────── live surface ─────────────────────────────

    fn send_backlight(&self) {
        let level = self.backlight.map_or(self.config.backlight, |(_, l)| l);
        for m in self.modules.values().filter(|m| matches!(m.info.kind, ModuleKind::Keypad | ModuleKind::Dial)) {
            let _ = m.tx.send(DeviceCmd::Backlight(level));
        }
    }

    fn serials_of(&self, kind: ModuleKind) -> Vec<String> {
        self.modules.values().filter(|m| m.info.kind == kind).map(|m| m.info.serial.clone()).collect()
    }

    /// Decide again who owns which control and redraw what changed hands.
    fn reclaim(&mut self) {
        let changes = self.claims.recompute(self.active_app.as_deref(), self.active_title.as_deref());
        let mut zones = Vec::new();
        for ctl in &changes.controls {
            match *ctl {
                Control::Key(k) => {
                    for s in self.serials_of(ModuleKind::Keypad) {
                        self.anims.remove(&(s.clone(), k));
                        self.refresh_target(&s, k);
                    }
                }
                Control::Dial => {
                    for s in self.serials_of(ModuleKind::Dial) {
                        self.anims.remove(&(s.clone(), TARGET_DIAL));
                        self.refresh_target(&s, TARGET_DIAL);
                    }
                }
                Control::Led(z) => zones.push(z),
                // Inputs are routed as they come in.
                Control::Knob(_) | Control::Roller(_) | Control::Fader(_) => {}
            }
        }
        for z in zones {
            self.send_zone_leds(z);
        }
        if changes.controls.iter().any(|c| matches!(c, Control::Key(_) | Control::Dial)) {
            // A released key may show a widget again.
            self.sample_widgets();
        }
        for claim in changes.claims {
            self.publish_claim(claim);
        }
    }

    fn publish_claim(&self, claim: u64) {
        let Some(c) = self.claims.get(claim) else { return };
        let turn = |kind| self.serials_of(kind).first().map_or(0, |s| self.turn_for(s));
        self.events.send(Event::Claim(ClaimState {
            claim,
            name: c.name.clone(),
            active: self.claims.is_active(claim),
            owns: crate::surface::collect(&self.claims.owned_by(claim)),
            key_turn: turn(ModuleKind::Keypad),
            dial_turn: turn(ModuleKind::Dial),
            priority: Some(c.priority),
            apps: c.apps.clone(),
            pid: None,
            program: None,
        }));
    }

    fn publish_claims(&self) {
        for id in self.claims.ids() {
            self.publish_claim(id);
        }
    }

    /// The LED state a claim set for `zone`, if a claim owns it.
    fn claimed_led(&self, zone: LedZone) -> Option<LedState> {
        let owner = self.claims.owner(Control::Led(zone))?;
        Some(self.claims.get(owner)?.leds.get(&zone).copied().unwrap_or_default())
    }

    /// Re-send the LEDs of the module that has `zone`.
    fn send_zone_leds(&mut self, zone: LedZone) {
        let kind = match zone {
            LedZone::Base => ModuleKind::Base,
            LedZone::Knob(_) => ModuleKind::Knob,
            LedZone::Fader(_) => ModuleKind::Fader,
        };
        if zone == LedZone::Base {
            self.apply_base_led();
            return;
        }
        for s in self.serials_of(kind) {
            self.send_leds(&s);
        }
    }

    fn send_one_led(&self, zone: LedZone, mode: Option<LedMode>, color: Option<Rgb>, brightness: Option<u8>) {
        let (kind, idx) = match zone {
            LedZone::Base => (ModuleKind::Base, 0),
            LedZone::Knob(i) => (ModuleKind::Knob, i),
            LedZone::Fader(i) => (ModuleKind::Fader, i),
        };
        let t = LedTarget::Index(idx);
        let mut reports = Vec::new();
        reports.extend(mode.and_then(|m| led_mode(kind, t, m)));
        reports.extend(color.and_then(|c| led_color(kind, t, c)));
        reports.extend(brightness.and_then(|b| led_brightness(kind, t, b)));
        if reports.is_empty() {
            return;
        }
        for m in self.modules.values().filter(|m| m.info.kind == kind) {
            let _ = m.tx.send(DeviceCmd::Output(reports.clone()));
        }
    }

    /// Send the newest claimed frames the app has not seen yet.
    fn flush_displays(&mut self) {
        for ((serial, target), jpeg) in std::mem::take(&mut self.display_pending) {
            self.display_sent.insert((serial.clone(), target), Instant::now());
            self.events.send(Event::Display(DisplayImage::new(&serial, target, &jpeg)));
        }
    }

    // ───────────────────────────── config ─────────────────────────────

    fn check_config_file(&mut self) {
        let m = mtime(&self.config_path);
        if m == self.config_mtime {
            return;
        }
        self.config_mtime = m;
        match std::fs::read_to_string(&self.config_path).map_err(anyhow::Error::from).and_then(|text| {
            let dir = self.config_path.parent().unwrap_or(std::path::Path::new("."));
            Config::parse(&text, dir).map(|c| (c, text))
        }) {
            Ok((config, text)) => {
                log::info!("{} changed, reloading", self.config_path.display());
                self.apply_config(config, text);
            }
            Err(e) => log::error!("not reloading {}: {e:#}", self.config_path.display()),
        }
    }

    fn apply_config(&mut self, config: Config, text: String) {
        let old_profile = self.profile_name().map(str::to_owned);
        let old_page = self.page_ref().map(|p| p.name.clone());
        if config.font != self.config.font {
            self.renderer = Renderer::new(config.font.as_deref());
        }
        self.config = config;
        self.update_status();
        self.pictures.clear();
        self.anims.clear();
        // Keep the user where they were if that still exists.
        self.profile = old_profile.and_then(|n| self.config.profiles.iter().position(|p| p.name == n)).unwrap_or(0);
        self.previous_profile = None;
        let pages = self.profile_ref().map(|p| p.pages.as_slice()).unwrap_or_default();
        self.page = old_page.and_then(|n| pages.iter().position(|p| p.name == n)).unwrap_or(0);
        let folders = self.profile_ref().map(|p| p.folders.clone()).unwrap_or_default();
        self.folders.retain(|f| folders.iter().any(|x| &x.name == f));
        {
            let mut s = self.snapshot.lock().unwrap();
            s.config = self.config.clone();
            s.config_text = text;
        }
        let serials: Vec<String> = self.modules.keys().cloned().collect();
        for serial in serials {
            self.init_module(&serial);
        }
        self.events.send(Event::ConfigChanged);
        self.publish_state();
    }

    // ───────────────────────────── state ─────────────────────────────

    fn profile_ref(&self) -> Option<&masterhub_core::config::Profile> {
        self.config.profiles.get(self.profile)
    }

    fn profile_name(&self) -> Option<&str> {
        self.profile_ref().map(|p| p.name.as_str())
    }

    /// The keypad page or folder on screen.
    fn page_ref(&self) -> Option<&Page> {
        let p = self.profile_ref()?;
        match self.folders.last() {
            Some(f) => p.folders.iter().find(|x| &x.name == f),
            None => p.pages.get(self.page),
        }
    }

    /// The key at `index` on screen: pinned, or the page's own.
    fn key_ref(&self, index: u8) -> Option<&masterhub_core::config::KeyConfig> {
        self.config.key(self.profile_name()?, self.page_ref()?, index)
    }

    fn switch_profile(&mut self, index: usize) {
        if index >= self.config.profiles.len() {
            return;
        }
        if index != self.profile {
            self.previous_profile = Some(self.profile);
        }
        self.profile = index;
        self.page = 0;
        self.session_offset = 0;
        self.folders.clear();
        self.anims.clear();
        let serials: Vec<String> = self.modules.keys().cloned().collect();
        for s in &serials {
            self.send_leds(s);
        }
        self.refresh_all();
        self.publish_state();
    }

    fn show_page(&mut self, name: &str) {
        let Some(p) = self.profile_ref() else { return };
        if let Some(i) = p.pages.iter().position(|x| x.name == name) {
            self.page = i;
            self.folders.clear();
        } else if p.folders.iter().any(|x| x.name == name) {
            self.folders = vec![name.to_owned()];
        } else {
            return;
        }
        self.anims.retain(|_, _| false);
        self.refresh_all();
        self.publish_state();
    }

    fn navigate(&mut self, action: &Action) -> bool {
        let Some(p) = self.profile_ref() else { return false };
        let n = p.pages.len().max(1);
        match action {
            Action::Page(which) => {
                self.page = match which.as_str() {
                    "next" => (self.page + 1) % n,
                    "prev" => (self.page + n - 1) % n,
                    name => match p.pages.iter().position(|x| x.name == name) {
                        Some(i) => i,
                        None => return true,
                    },
                };
                self.folders.clear();
            }
            Action::Folder(name) => self.folders.push(name.clone()),
            // Outside a folder: back to the profile shown before this one (the app's profile
            // after a pinned Claude key, or the one before the Claude app was focused), else
            // the profile of the focused app, else the default.
            Action::Back(_) if self.folders.is_empty() => {
                let app = self.config.profile_for_app(self.active_app.as_deref().unwrap_or_default());
                if let Some(t) = back_target(self.profile, self.previous_profile, app, self.config.profiles.len()) {
                    self.switch_profile(t);
                }
                return true;
            }
            Action::Back(_) => {
                self.folders.pop();
            }
            Action::Profile(name) => {
                match self.config.profiles.iter().position(|p| &p.name == name) {
                    Some(i) => self.switch_profile(i),
                    None => log::warn!("profile {name:?}: not supported by this masterhubd yet"),
                }
                return true;
            }
            // Setups are not implemented yet: the config may already use them.
            Action::Setup(_) | Action::Pin(_) => {
                log::warn!("{action:?}: not supported by this masterhubd yet");
                return true;
            }
            Action::Cmd(_)
            | Action::Keys(_)
            | Action::Volume(_)
            | Action::Mute(_)
            | Action::Session(_)
            | Action::Sessions(_) => return false,
        }
        self.anims.clear();
        self.refresh_all();
        self.publish_state();
        true
    }

    // ───────────────────────────── output ─────────────────────────────

    fn init_module(&mut self, serial: &str) {
        let Some(m) = self.modules.get_mut(serial) else { return };
        m.shown.clear();
        let kind = m.info.kind;
        if matches!(kind, ModuleKind::Keypad | ModuleKind::Dial) {
            let _ = m.tx.send(DeviceCmd::Backlight(self.backlight.map_or(self.config.backlight, |(_, l)| l)));
        }
        self.send_leds(serial);
        self.refresh_module(serial);
    }

    /// Global LED colours, with the active profile's overrides applied per field.
    fn effective_leds(&self) -> Leds {
        let mut leds = self.config.leds.clone();
        if let Some(o) = self.profile_ref().and_then(|p| p.leds.as_ref()) {
            leds.base = o.base.or(leds.base);
            leds.fader = o.fader.clone().or(leds.fader);
            leds.knob = o.knob.clone().or(leds.knob);
            leds.brightness = o.brightness.or(leds.brightness);
        }
        leds
    }

    fn send_leds(&self, serial: &str) {
        let Some(m) = self.modules.get(serial) else { return };
        let kind = m.info.kind;
        let leds = self.effective_leds();
        let colors = match kind {
            ModuleKind::Base => self.status_color().or(leds.base).map(masterhub_core::config::OneOrMany::One),
            ModuleKind::Fader => leds.fader.clone(),
            ModuleKind::Knob => leds.knob.clone(),
            _ => return,
        };
        let brightness = leds.brightness.unwrap_or(100);
        let mut reports = Vec::new();
        for i in 0..kind.led_count() {
            let t = LedTarget::Index(i);
            let zone = match kind {
                ModuleKind::Base => LedZone::Base,
                ModuleKind::Knob => LedZone::Knob(i),
                _ => LedZone::Fader(i),
            };
            if let Some(st) = self.claimed_led(zone) {
                // Only what the claim set; a colour alone means a static colour.
                let mode = st.mode.or(st.color.map(|_| LedMode::Static));
                reports.extend(mode.and_then(|m| led_mode(kind, t, m)));
                reports.extend(st.color.and_then(|c| led_color(kind, t, c)));
                reports.extend(st.brightness.and_then(|b| led_brightness(kind, t, b)));
                continue;
            }
            let Some(c) = colors.as_ref().and_then(|c| c.get(i as usize)) else { continue };
            reports.extend(led_mode(kind, t, LedMode::Static));
            reports.extend(led_color(kind, t, c.0));
            reports.extend(led_brightness(kind, t, brightness));
        }
        if reports.is_empty() {
            return;
        }
        let _ = m.tx.send(DeviceCmd::Output(reports));
    }

    // ───────────────────────────── tool status ─────────────────────────────

    /// The status colour on the Base right now (dark in the off phase of a flash).
    fn status_color(&self) -> Option<Color> {
        let st = self.shown_status?;
        let c = &self.config.status;
        Some(match st {
            Status::Attention if !self.blink_on => Color(Rgb::default()),
            Status::Attention => c.attention,
            Status::Ready => c.ready,
            Status::Working => c.working,
        })
    }

    /// Show the most urgent status of all sources on the Base, and flash while any source
    /// needs attention (the Base and session keys flash together).
    fn update_status(&mut self) {
        let attention = self.statuses.values().any(|r| r.status == Status::Attention);
        // A finished session you have checked needs nothing from you.
        if attention != self.blink_due.is_some() {
            self.blink_on = true;
            self.blink_due = attention.then(|| Instant::now() + BLINK);
        }
        let top = if self.config.status.enabled { self.statuses.values().filter(|r| !r.checked()).map(|r| r.status).max() } else { None };
        if top == self.shown_status {
            return;
        }
        log::info!("base status: {top:?}");
        self.shown_status = top;
        self.apply_base_led();
    }

    /// Show the Base light: a claim's, the status colour, the configured colour, or the
    /// firmware's cycling.
    fn apply_base_led(&self) {
        let Some(serial) = self.base_serial() else { return };
        if self.shown_status.is_none() && self.effective_leds().base.is_none() && self.claimed_led(LedZone::Base).is_none() {
            // No Base colour configured: back to the firmware's colour cycling.
            if let Some(m) = self.modules.get(&serial) {
                let reports = led_mode(ModuleKind::Base, LedTarget::Index(0), LedMode::Cycling).into_iter().collect();
                let _ = m.tx.send(DeviceCmd::Output(reports));
            }
        } else {
            self.send_leds(&serial);
        }
    }

    fn advance_blink(&mut self, now: Instant) {
        let Some(due) = self.blink_due.filter(|d| *d <= now) else { return };
        self.blink_on = !self.blink_on;
        self.blink_due = Some((due + BLINK).max(now));
        if self.shown_status == Some(Status::Attention)
            && self.claims.owner(Control::Led(LedZone::Base)).is_none()
            && let (Some(c), Some(m)) = (self.status_color(), self.base_serial().and_then(|s| self.modules.get(&s)))
        {
            let reports = led_color(ModuleKind::Base, LedTarget::Index(0), c.0).into_iter().collect();
            let _ = m.tx.send(DeviceCmd::Output(reports));
        }
        self.refresh_sessions();
    }

    /// Forget sources whose process has exited without clearing its status, and re-read the
    /// session files of the others.
    fn prune_statuses(&mut self) {
        let before = self.statuses.len();
        self.statuses.retain(|source, r| {
            let alive = r.pid.is_none_or(process_alive);
            if !alive {
                log::info!("status {source}: process gone");
            }
            alive
        });
        let mut changed = self.statuses.len() != before;
        for (source, r) in &mut self.statuses {
            if let Some(pid) = r.pid.filter(|_| source.starts_with("claude:") && !r.discovered) {
                r.file = SessionFile::of(pid);
                let idle = r.file.as_ref().is_some_and(|f| f.status.as_deref() == Some("idle"));
                if waited_too_long(r.status, idle, r.reported.elapsed()) {
                    log::info!("status {source}: idle and silent for {BACKGROUND_TIMEOUT:?}, ready");
                    changed |= r.set(Status::Ready, &mut self.reports);
                }
                // Claude Code marks its file while a prompt or dialog waits for you, including
                // ones no hook reports; follow it until the next hook event says otherwise.
                let waiting = r.file.as_ref().is_some_and(SessionFile::waiting);
                if let Some(status) = file_wait(r.status, r.waiting, waiting, r.file.as_ref().map(SessionFile::status)) {
                    log::info!("status {source}: session file {}", if waiting { "waits for you" } else { "no longer waits" });
                    changed |= r.set(status, &mut self.reports);
                }
                r.waiting = waiting;
            }
        }
        for (pid, file) in SessionFile::running() {
            let source = format!("claude:{}", file.session_id.as_deref().unwrap_or_default());
            match self.statuses.get_mut(&source) {
                Some(r) if !r.discovered => {}
                Some(r) => {
                    changed |= r.set(file.status(), &mut self.reports);
                    r.file = Some(file);
                }
                None => {
                    log::info!("found Claude Code session {source} (pid {pid})");
                    let mut r = Reporter::new(file.status(), Some(pid), Some(file), &mut self.reports);
                    r.discovered = true;
                    self.statuses.insert(source, r);
                    changed = true;
                }
            }
        }
        changed |= self.follow_remote_sessions();
        // Sessions of the desktop app count as checked once it has shown them.
        let views = self.desktop.views();
        for (source, r) in &mut self.statuses {
            let view = source.strip_prefix("claude:").and_then(|id| views.get(id)).copied();
            if r.view != view {
                let was = r.checked();
                r.view = view;
                if r.checked() != was {
                    log::info!("status {source}: {}", if was { "not checked" } else { "checked" });
                    changed = true;
                }
            }
        }
        if changed {
            self.update_status();
        }
        // Names come from the session files and change at any time.
        self.publish_sessions();
    }

    /// Tell clients about the open sessions when anything about them changed.
    fn publish_sessions(&mut self) {
        let list: Vec<masterhub_core::ipc::SessionInfo> = self
            .sessions()
            .into_iter()
            .enumerate()
            .filter_map(|(slot, r)| {
                let source = self.statuses.iter().find(|(_, x)| std::ptr::eq(*x, r)).map(|(s, _)| s.clone())?;
                Some(masterhub_core::ipc::SessionInfo {
                    slot: slot as u8,
                    source,
                    name: r.file.as_ref().and_then(SessionFile::title).unwrap_or_else(|| "Claude".to_owned()),
                    status: r.status,
                    since: r.changed_at,
                    detail: r.detail.clone(),
                })
            })
            .collect();
        let mut snap = self.snapshot.lock().unwrap();
        if snap.sessions != list {
            snap.sessions = list.clone();
            drop(snap);
            self.events.send(Event::Sessions(list));
        }
    }

    /// Add, update and drop the sessions the desktop app runs over SSH; whether any changed.
    fn follow_remote_sessions(&mut self) -> bool {
        let remote = self.desktop.remote();
        let mut changed = false;
        for s in &remote {
            match self.statuses.get_mut(&s.source) {
                // Its hooks reach this service after all.
                Some(r) if !r.remote => {}
                Some(r) => {
                    changed |= r.set(s.status, &mut self.reports) || r.file.as_ref() != Some(&s.file);
                    r.file = Some(s.file.clone());
                }
                None => {
                    log::info!("found remote Claude Code session {} ({})", s.source, s.file.title().unwrap_or_default());
                    let mut r = Reporter::new(s.status, None, Some(s.file.clone()), &mut self.reports);
                    (r.discovered, r.remote) = (true, true);
                    self.statuses.insert(s.source.clone(), r);
                    changed = true;
                }
            }
        }
        let before = self.statuses.len();
        self.statuses.retain(|source, r| !r.remote || remote.iter().any(|s| &s.source == source));
        changed || self.statuses.len() != before
    }

    // ───────────────────────────── Claude Code sessions ─────────────────────────────

    /// Open Claude Code sessions in the configured order (`[status] sort`).
    fn sessions(&self) -> Vec<&Reporter> {
        let mut v: Vec<&Reporter> =
            self.statuses.iter().filter(|(source, _)| source.starts_with("claude:")).map(|(_, r)| r).collect();
        sort_sessions(&mut v, self.config.status.sort);
        v
    }

    /// How many session keys are on screen: one past the highest slot shown, 0 with none.
    fn session_slots(&self) -> usize {
        let slots = self.visible_widgets().into_iter().filter_map(|w| match w {
            Widget::ClaudeSession { slot } => Some(usize::from(slot) + 1),
            _ => None,
        });
        slots.max().unwrap_or(0)
    }

    /// What a session key shows; `None` for an empty slot.
    fn session_live(&self, slot: u8, slots: usize) -> Option<LiveData> {
        let sessions = self.sessions();
        let offset = page_start(self.session_offset, sessions.len(), slots);
        let r = *sessions.get(offset + usize::from(slot))?;
        let c = &self.config.status;
        let (color, frame) = match r.status {
            Status::Attention => (c.attention, u32::from(!self.blink_on)),
            Status::Ready => (c.ready, 0),
            Status::Working => (c.working, self.ticks),
        };
        let name = r.file.as_ref().and_then(SessionFile::title).unwrap_or_else(|| "Claude".to_owned());
        // Arrows on the first and last key when sessions are scrolled off; in the attention
        // colour when one of those waits for you.
        let arrow = |off: &[&Reporter]| {
            let urgent = off.iter().any(|r| r.status == Status::Attention);
            (!off.is_empty()).then_some(if urgent { c.attention.0 } else { Rgb::new(0xff, 0xff, 0xff) })
        };
        let above = if slot == 0 { arrow(&sessions[..offset]) } else { None };
        let last = usize::from(slot) + 1 == slots;
        let below = if last { arrow(&sessions[(offset + slots).min(sessions.len())..]) } else { None };
        Some(LiveData::Session { name, status: r.status, color: color.0, frame, checked: r.checked(), above, below })
    }

    /// Redraw session keys after a status change or a flash, if any are on screen.
    fn refresh_sessions(&mut self) {
        let slots: Vec<Widget> =
            self.visible_widgets().into_iter().filter(|w| matches!(w, Widget::ClaudeSession { .. })).collect();
        if slots.is_empty() {
            return;
        }
        let count = self.session_slots();
        for w in slots {
            self.set_session_live(w, count);
        }
        self.refresh_all();
    }

    /// `slots`: [`Self::session_slots`].
    fn set_session_live(&mut self, w: Widget, slots: usize) {
        let Widget::ClaudeSession { slot } = w else { return };
        match self.session_live(slot, slots) {
            Some(live) => self.live.insert(w, live),
            None => self.live.remove(&w),
        };
    }

    /// Scroll the session keys a page `next` or `prev`, stopping at either end.
    fn scroll_sessions(&mut self, dir: &str) {
        let (total, slots) = (self.sessions().len(), self.session_slots());
        let offset = page_start(self.session_offset, total, slots);
        let to = match dir {
            "prev" => offset.saturating_sub(slots),
            _ => page_start(offset + slots, total, slots),
        };
        if to != self.session_offset {
            log::debug!("sessions {}..{} of {total}", to + 1, (to + slots).min(total));
            self.session_offset = to;
            self.refresh_sessions();
        }
    }

    /// Bring up the session in `slot`, off the controller thread; that checks it.
    fn bring_up(&mut self, slot: u8) {
        let sessions = self.sessions();
        let offset = page_start(self.session_offset, sessions.len(), self.session_slots());
        let Some(r) = sessions.get(offset + usize::from(slot)).copied() else { return };
        let (pid, file) = (r.pid, r.file.clone());
        let source = self.statuses.iter().find(|(_, x)| std::ptr::eq(*x, r)).map(|(s, _)| s.clone());
        std::thread::spawn(move || crate::claude::bring_up(pid, file.as_ref()));
        if let Some(r) = source.and_then(|s| self.statuses.get_mut(&s)) {
            r.pressed = now_ms();
            self.update_status();
            self.refresh_sessions();
        }
    }

    fn base_serial(&self) -> Option<String> {
        self.modules.values().find(|m| m.info.kind == ModuleKind::Base).map(|m| m.info.serial.clone())
    }

    fn refresh_all(&mut self) {
        let serials: Vec<String> = self.modules.keys().cloned().collect();
        for s in serials {
            self.refresh_module(&s);
        }
    }

    fn refresh_module(&mut self, serial: &str) {
        let Some(kind) = self.modules.get(serial).map(|m| m.info.kind) else { return };
        match kind {
            ModuleKind::Keypad => {
                for i in 0..KEY_COUNT as u8 {
                    self.refresh_target(serial, i);
                }
            }
            ModuleKind::Dial => self.refresh_target(serial, TARGET_DIAL),
            _ => {}
        }
    }

    fn face_for(&self, target: u8) -> Option<Face> {
        if target == TARGET_DIAL {
            self.profile_ref()?.dial.as_ref().map(|d| d.face.clone())
        } else {
            self.key_ref(target).map(|k| k.face.clone())
        }
    }

    fn picture(&mut self, path: &std::path::Path, size: u32) -> Option<Rc<Picture>> {
        self.pictures
            .entry((path.to_owned(), size))
            .or_insert_with(|| match load_picture(path, size) {
                Ok(p) => Some(Rc::new(p)),
                Err(e) => {
                    log::error!("{e:#}");
                    None
                }
            })
            .clone()
    }

    /// Render one display target and send it if it changed.
    fn refresh_target(&mut self, serial: &str, target: u8) {
        let size = if target == TARGET_DIAL { DIAL_SIZE } else { KEY_SIZE };
        if let Some(owner) = self.claims.owner(Control::of_target(target)) {
            self.anims.remove(&(serial.to_owned(), target));
            let frame = self.claims.get(owner).and_then(|c| c.frames.get(&target).cloned());
            // Nothing pushed yet: blank, rather than the config's face under it.
            let jpeg = match frame {
                Some(f) => f,
                None => match self.renderer.render(&Face::default(), size, None, None, 0) {
                    Ok(j) => Arc::new(j),
                    Err(e) => return log::error!("rendering: {e:#}"),
                },
            };
            self.show(serial, target, jpeg, true);
            return;
        }
        let face = self.face_for(target).unwrap_or_default();
        let picture = face.image.as_ref().and_then(|p| self.picture(p, size));
        let frame = match picture.as_deref() {
            Some(Picture::Still(img)) => Some(img),
            Some(Picture::Animated(frames)) => {
                let anim = self
                    .anims
                    .entry((serial.to_owned(), target))
                    .or_insert_with(|| Anim { frame: 0, due: Instant::now() + frames[0].1 });
                Some(&frames[anim.frame % frames.len()].0)
            }
            None => None,
        };
        let live = face.widget.as_ref().and_then(|w| self.live.get(&w.widget));
        let turn = self.turn_for(serial);
        let jpeg = match self.renderer.render(&face, size, frame, live, turn) {
            Ok(j) => Arc::new(j),
            Err(e) => {
                log::error!("rendering: {e:#}");
                return;
            }
        };
        self.show(serial, target, jpeg, false);
    }

    /// Send an image to a display target if it changed. `throttle`: claimed frames, whose
    /// `display` events are rate-limited (see [`Controller::flush_displays`]).
    fn show(&mut self, serial: &str, target: u8, jpeg: Arc<Vec<u8>>, throttle: bool) {
        let Some(m) = self.modules.get_mut(serial) else { return };
        if m.shown.get(&target).is_some_and(|old| *old == jpeg) {
            return;
        }
        m.shown.insert(target, jpeg.clone());
        let it = if target == TARGET_DIAL { ImageTarget::Dial } else { ImageTarget::Key(target) };
        let _ = m.tx.send(DeviceCmd::Image { target: it, jpeg: jpeg.clone() });
        let key = (serial.to_owned(), target);
        self.snapshot.lock().unwrap().displays.insert(key.clone(), jpeg.clone());
        if throttle && self.display_sent.get(&key).is_some_and(|t| t.elapsed() < DISPLAY_EVERY) {
            self.display_pending.insert(key, jpeg);
            return;
        }
        self.display_pending.remove(&key);
        self.display_sent.insert(key, Instant::now());
        self.events.send(Event::Display(DisplayImage::new(serial, target, &jpeg)));
    }

    /// How far to turn a module's images (clockwise) so they read upright for the user: the
    /// inverse of the module's rotation on the Base plus the Base's rotation on the desk.
    fn turn_for(&self, serial: &str) -> u16 {
        if !self.config.upright {
            return 0;
        }
        let module = self.modules.get(serial).and_then(|m| m.placement).map_or(0, |p| p.rotation);
        layout::upright_turn(module, self.config.orientation.rotation(self.sensor_horizontal))
    }

    fn advance_animations(&mut self, now: Instant) {
        let due: Vec<(String, u8)> = self.anims.iter().filter(|(_, a)| a.due <= now).map(|(k, _)| k.clone()).collect();
        for (serial, target) in due {
            let size = if target == TARGET_DIAL { DIAL_SIZE } else { KEY_SIZE };
            let delay = self
                .face_for(target)
                .and_then(|f| f.image)
                .and_then(|p| self.picture(&p, size))
                .and_then(|pic| match &*pic {
                    Picture::Animated(frames) => {
                        let anim = self.anims.get_mut(&(serial.clone(), target))?;
                        anim.frame = (anim.frame + 1) % frames.len();
                        Some(frames[anim.frame].1)
                    }
                    Picture::Still(_) => None,
                });
            match delay {
                Some(d) => {
                    if let Some(a) = self.anims.get_mut(&(serial.clone(), target)) {
                        // Catch up without drifting, but don't spin after a long stall.
                        a.due = (a.due + d).max(now);
                    }
                    self.refresh_target(&serial, target);
                }
                None => {
                    self.anims.remove(&(serial, target));
                }
            }
        }
    }

    /// Sample live data for widgets visible on connected displays.
    fn sample_widgets(&mut self) {
        let needed = self.visible_widgets();
        self.live = self.stats.sample(&needed);
        let slots = self.session_slots();
        for w in needed {
            self.set_session_live(w, slots);
        }
    }

    /// Widgets on the connected displays right now.
    fn visible_widgets(&self) -> Vec<Widget> {
        let mut needed = Vec::new();
        for m in self.modules.values() {
            let targets: Vec<u8> = match m.info.kind {
                ModuleKind::Keypad => (0..KEY_COUNT as u8).collect(),
                ModuleKind::Dial => vec![TARGET_DIAL],
                _ => continue,
            };
            for t in targets {
                if self.claims.owner(Control::of_target(t)).is_some() {
                    continue;
                }
                if let Some(w) = self.face_for(t).and_then(|f| f.widget) {
                    needed.push(w.widget);
                }
            }
        }
        needed
    }

    // ───────────────────────────── input ─────────────────────────────

    fn input(&mut self, serial: &str, report: InputReport) {
        let mut events: Vec<(&'static str, u8, &'static str, Option<u8>)> = Vec::new();
        let Some(m) = self.modules.get_mut(serial) else { return };
        match report {
            InputReport::Keys(state) => {
                for (i, down) in m.keys.update(state) {
                    events.push(("key", i as u8, if down { "down" } else { "up" }, None));
                }
            }
            InputReport::Faders(pos) => {
                let prev = m.faders.replace(pos);
                for i in 0..5 {
                    if prev.is_none_or(|p| p[i] != pos[i]) {
                        events.push(("fader", i as u8, "move", Some(pos[i])));
                    }
                }
            }
            InputReport::Rollers { left, right } => {
                for (idx, roll) in [(0, left), (1, right)] {
                    if let Some(r) = roll {
                        events.push(("roller", idx, if r == Roll::Away { "away" } else { "toward" }, None));
                    }
                }
            }
            InputReport::KnobTurn(turns) => {
                for (i, rot) in turns.into_iter().enumerate() {
                    if let Some(r) = rot {
                        events.push(("knob", i as u8, if r == Rotation::Clockwise { "cw" } else { "ccw" }, None));
                    }
                }
            }
            InputReport::KnobButtons(state) => {
                for (i, down) in m.knob_buttons.update(state) {
                    events.push(("knob", i as u8, if down { "down" } else { "up" }, None));
                }
            }
            InputReport::DialTurn(r) => events.push(("dial", 0, if r == Rotation::Clockwise { "cw" } else { "ccw" }, None)),
            InputReport::DialButton { pressed } => events.push(("dial", 0, if pressed { "down" } else { "up" }, None)),
            InputReport::BaseOrientation { horizontal } => {
                if self.sensor_horizontal != Some(horizontal) {
                    log::info!("Base tilt sensor: {}", if horizontal { "landscape" } else { "portrait" });
                    self.sensor_horizontal = Some(horizontal);
                    self.publish_state();
                    // Turn the screens to the new orientation.
                    self.refresh_all();
                    self.publish_claims();
                }
            }
            InputReport::Unknown { .. } => {}
        }

        for (control, index, event, value) in events {
            log::debug!("{control} {index} {event} {value:?}");
            let claim = Control::of_input(control, index).and_then(|c| self.claims.owner(c));
            let msg = InputMsg {
                serial: serial.to_owned(),
                control: control.into(),
                index,
                event: event.into(),
                value,
                claim,
            };
            self.events.send(Event::Input(msg));
            if claim.is_some() {
                continue;
            }
            if let Some(action) = self.binding(control, index, event) {
                let coalesce = (control == "fader").then(|| format!("{serial}/fader{index}"));
                self.run_action(action, value, coalesce);
            }
        }
    }

    /// Resolve a volume or mute target; `app = "focused"` means the focused window's app.
    fn audio_target(&self, target: &AudioTarget) -> Option<crate::audio::Target> {
        match target {
            AudioTarget::App(a) if a.eq_ignore_ascii_case("focused") => {
                let class = self.active_app.clone().filter(|c| !c.is_empty())?;
                crate::audio::Target::resolve(&AudioTarget::App(class), &[])
            }
            t => crate::audio::Target::resolve(t, &self.config.audio_groups),
        }
    }

    /// Run an action from a control or a `run_action` request.
    fn run_action(&mut self, action: Action, value: Option<u8>, coalesce: Option<String>) {
        match &action {
            Action::Session(slot) => self.bring_up(*slot),
            Action::Sessions(dir) => self.scroll_sessions(dir),
            _ if self.navigate(&action) => {}
            _ => {
                let audio = match &action {
                    Action::Volume(v) => self.audio_target(&v.target),
                    Action::Mute(m) => self.audio_target(&m.target),
                    _ => None,
                };
                let _ = self.actions.send(Request { action, value, coalesce, audio });
            }
        }
    }

    /// The action bound to an input in the active profile.
    fn binding(&self, control: &str, index: u8, event: &str) -> Option<Action> {
        let p = self.profile_ref()?;
        let pick = |cw: &Option<Action>, ccw: &Option<Action>, press: &Option<Action>, release: &Option<Action>| {
            match event {
                "cw" => cw.clone(),
                "ccw" => ccw.clone(),
                "down" => press.clone(),
                "up" => release.clone(),
                _ => None,
            }
        };
        match control {
            "key" => {
                let k = self.key_ref(index)?;
                if event == "down" { k.press.clone() } else { k.release.clone() }
            }
            "fader" => p.faders.iter().find(|f| f.index == index)?.change.clone(),
            "knob" => {
                let k = p.knobs.iter().find(|k| k.index == index)?;
                pick(&k.cw, &k.ccw, &k.press, &k.release)
            }
            "dial" => {
                let d = p.dial.as_ref()?;
                pick(&d.cw, &d.ccw, &d.press, &d.release)
            }
            "roller" => {
                let side = if index == 0 { Side::Left } else { Side::Right };
                match p.rollers.iter().find(|r| r.side == side) {
                    Some(r) => if event == "away" { r.away.clone() } else { r.toward.clone() },
                    // Unbound, the left roller scrolls the session keys on screen.
                    None if side == Side::Left && self.session_slots() > 0 => {
                        Some(Action::Sessions(if event == "away" { "prev" } else { "next" }.into()))
                    }
                    None => None,
                }
            }
            _ => None,
        }
    }

    // ───────────────────────────── publishing ─────────────────────────────

    fn publish_state(&self) {
        let state = HubState {
            profile: self.profile_name().unwrap_or_default().to_owned(),
            page: self.page_ref().map(|p| p.name.clone()).unwrap_or_default(),
            in_folder: !self.folders.is_empty(),
            horizontal: self.config.orientation.rotation(self.sensor_horizontal) % 180 == 90,
            rotation: self.config.orientation.rotation(self.sensor_horizontal),
            active_app: self.active_app.clone(),
            ..Default::default()
        };
        let changed = {
            let mut s = self.snapshot.lock().unwrap();
            let changed = s.state != state;
            s.state = state.clone();
            changed
        };
        if changed {
            self.events.send(Event::State(state));
        }
    }

    fn publish_modules(&mut self) {
        let base_ports = self
            .modules
            .values()
            .find(|m| m.info.kind == ModuleKind::Base)
            .and_then(|m| m.info.usb_ports.clone());
        let modules: Vec<ModuleState> = self
            .modules
            .values()
            .map(|m| {
                let socket = base_ports.as_ref().and_then(|base| layout::socket(m.info.usb_ports.as_deref()?, base));
                let placement = socket.as_deref().and_then(|s| layout::place(m.info.kind, s));
                ModuleState {
                    kind: m.info.kind.name().to_owned(),
                    serial: m.info.serial.clone(),
                    socket,
                    placement,
                    firmware: m.firmware.clone(),
                    needs_replug: false,
                }
            })
            .collect();
        let mut moved = false;
        for ms in &modules {
            if let Some(m) = self.modules.get_mut(&ms.serial)
                && m.placement != ms.placement
            {
                m.placement = ms.placement;
                moved = true;
            }
        }
        self.snapshot.lock().unwrap().modules = modules.clone();
        self.events.send(Event::Modules(modules));
        if moved {
            self.refresh_all();
        }
        self.publish_claims();
    }
}

#[cfg(target_os = "linux")]
pub fn process_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: plain Win32 calls; the handle is closed before returning.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(process, &mut code) != 0;
        CloseHandle(process);
        ok && code == STILL_ACTIVE as u32
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn process_alive(_pid: u32) -> bool {
    true
}

/// Where `back` outside a folder goes from profile `current`: the previous profile, else the
/// focused app's, else the default; never the current one.
fn back_target(current: usize, previous: Option<usize>, app: usize, profiles: usize) -> Option<usize> {
    [previous, Some(app), Some(0)].into_iter().flatten().find(|&i| i != current && i < profiles)
}

impl Reporter {
    /// A new source; `seq` is `Controller::reports`.
    fn new(status: Status, pid: Option<u32>, file: Option<SessionFile>, seq: &mut u64) -> Self {
        let r = Self {
            status,
            pid,
            first: *seq,
            changed: *seq,
            changed_at: now_ms(),
            view: None,
            pressed: 0,
            waiting: false,
            file,
            discovered: false,
            remote: false,
            reported: Instant::now(),
            detail: Default::default(),
        };
        *seq += 1;
        r
    }

    /// Set the status; whether it changed. `seq` stamps the change (`Controller::reports`).
    fn set(&mut self, status: Status, seq: &mut u64) -> bool {
        if self.status == status {
            return false;
        }
        self.status = status;
        self.changed = *seq;
        self.changed_at = now_ms();
        *seq += 1;
        true
    }

    /// Ready, and you have looked at it since its turn ended: in the desktop app, or by
    /// bringing it up from its key.
    fn checked(&self) -> bool {
        if self.status != Status::Ready {
            return false;
        }
        let ended = self.view.and_then(|v| v.turn_ended).unwrap_or(self.changed_at);
        self.view.map_or(0, |v| v.focused).max(self.pressed) >= ended
    }

    /// Group in the `urgent` order: waiting for you, finished and not checked yet, working,
    /// finished and checked.
    fn urgency(&self) -> u8 {
        match self.status {
            Status::Attention => 0,
            Status::Ready if !self.checked() => 1,
            Status::Working => 2,
            Status::Ready => 3,
        }
    }

    /// Start order: when Claude Code started it, then when it first reported.
    fn started(&self) -> (u64, u64) {
        (self.file.as_ref().and_then(|f| f.started_at).unwrap_or(u64::MAX), self.first)
    }
}

/// Put sessions in slot order. `started` keeps keys in place as sessions come and go; the
/// others put recent changes, or sessions waiting for you, first.
fn sort_sessions(v: &mut [&Reporter], sort: SessionSort) {
    match sort {
        SessionSort::Started => v.sort_by_key(|r| r.started()),
        SessionSort::Changed => v.sort_by_key(|r| (Reverse(r.changed), r.started())),
        // `Status` is ordered by urgency.
        SessionSort::Urgent => v.sort_by_key(|r| (r.urgency(), Reverse(r.changed), r.started())),
    }
}

/// The first session on the page that holds `offset`, with `slots` keys a page; on the last
/// page when sessions have gone since.
fn page_start(offset: usize, total: usize, slots: usize) -> usize {
    if slots == 0 {
        return 0;
    }
    (offset / slots * slots).min(total.saturating_sub(1) / slots * slots)
}

/// What a session's status becomes when its file starts or stops waiting for the user (`was`,
/// `now`), `None` to keep it: attention while it waits, then what the file says, unless a hook
/// has changed the status in between.
fn file_wait(status: Status, was: bool, now: bool, file: Option<Status>) -> Option<Status> {
    match (was, now) {
        (false, true) if status != Status::Attention => Some(Status::Attention),
        (true, false) if status == Status::Attention => file.filter(|s| *s != Status::Attention),
        _ => None,
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn mtime(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The status a report leaves: one from background work (a subagent's tool call) says the
/// source is not done yet, so it turns `ready` into `working`, but never hides `attention`.
fn reported_status(old: Option<Status>, new: Status, background: bool) -> Status {
    match old {
        Some(Status::Attention) if background => Status::Attention,
        _ => new,
    }
}

/// Whether a `working` session has waited too long on background work: its process is idle
/// and nothing has reported since. Catches a wake-up that never came (agents stopped by hand,
/// a lost hook event).
fn waited_too_long(status: Status, idle: bool, silent: Duration) -> bool {
    status == Status::Working && idle && silent >= BACKGROUND_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_reports_never_hide_attention() {
        use Status::*;
        assert_eq!(reported_status(Some(Ready), Working, true), Working);
        assert_eq!(reported_status(None, Working, true), Working);
        assert_eq!(reported_status(Some(Attention), Working, true), Attention);
        assert_eq!(reported_status(Some(Attention), Working, false), Working);
        assert_eq!(reported_status(Some(Working), Ready, false), Ready);
    }

    #[test]
    fn stuck_background_work_times_out() {
        let long = BACKGROUND_TIMEOUT + Duration::from_secs(1);
        assert!(waited_too_long(Status::Working, true, long));
        assert!(!waited_too_long(Status::Working, true, Duration::from_secs(60)));
        // Busy in a long tool call of its own.
        assert!(!waited_too_long(Status::Working, false, long));
        assert!(!waited_too_long(Status::Attention, true, long));
        assert!(!waited_too_long(Status::Ready, true, long));
    }

    fn session(status: Status, started: u64, changed: u64) -> Reporter {
        let file = SessionFile { started_at: Some(started), ..Default::default() };
        let mut r = Reporter::new(status, None, Some(file), &mut 0);
        r.changed = changed;
        r
    }

    #[test]
    fn sorts_sessions() {
        use Status::*;
        let all = [session(Working, 1, 5), session(Attention, 2, 1), session(Ready, 3, 3), session(Attention, 4, 4)];
        let order = |sort| {
            let mut v: Vec<&Reporter> = all.iter().collect();
            sort_sessions(&mut v, sort);
            v.iter().map(|r| r.started().0).collect::<Vec<_>>()
        };
        assert_eq!(order(SessionSort::Started), [1, 2, 3, 4]);
        assert_eq!(order(SessionSort::Changed), [1, 4, 3, 2]);
        // Waiting for you first, the most recent first within a status.
        assert_eq!(order(SessionSort::Urgent), [4, 2, 3, 1]);
    }

    #[test]
    fn checked_sessions_go_last() {
        use Status::*;
        let at = |status, started, changed, view: Option<View>, pressed| {
            let mut r = session(status, started, changed);
            (r.changed_at, r.view, r.pressed) = (1000, view, pressed);
            r
        };
        let focused = |focused, turn_ended| Some(View { focused, turn_ended: Some(turn_ended) });
        let all = [
            at(Ready, 1, 9, focused(2000, 1500), 0), // looked at in the app after it finished
            at(Ready, 2, 8, focused(1200, 1500), 0), // finished after you last looked
            at(Working, 3, 7, None, 0),
            at(Ready, 4, 6, None, 1500),             // brought up from its key
            at(Ready, 5, 5, None, 0),                // terminal session, never brought up
            at(Attention, 6, 4, focused(9000, 1), 0), // waiting for you, whatever you saw
        ];
        let checked: Vec<bool> = all.iter().map(Reporter::checked).collect();
        assert_eq!(checked, [true, false, false, true, false, false]);
        let mut v: Vec<&Reporter> = all.iter().collect();
        sort_sessions(&mut v, SessionSort::Urgent);
        assert_eq!(v.iter().map(|r| r.started().0).collect::<Vec<_>>(), [6, 2, 5, 3, 1, 4]);
    }

    #[test]
    fn follows_waiting_session_files() {
        use Status::*;
        // A prompt no hook reported: attention.
        assert_eq!(file_wait(Ready, false, true, Some(Attention)), Some(Attention));
        // Still waiting, but a hook has since said otherwise (answered, file not rewritten yet).
        assert_eq!(file_wait(Working, true, true, Some(Attention)), None);
        assert_eq!(file_wait(Attention, false, true, Some(Attention)), None);
        // Answered, and no hook said so: what the file says now.
        assert_eq!(file_wait(Attention, true, false, Some(Working)), Some(Working));
        assert_eq!(file_wait(Attention, true, false, Some(Ready)), Some(Ready));
        // A hook already moved it on.
        assert_eq!(file_wait(Working, true, false, Some(Working)), None);
        // Attention from a hook, the file never waited: left alone.
        assert_eq!(file_wait(Attention, false, false, Some(Working)), None);
    }

    #[test]
    fn status_changes_are_stamped() {
        let mut r = session(Status::Working, 1, 0);
        let mut seq = 7;
        assert!(!r.set(Status::Working, &mut seq));
        assert_eq!((r.changed, seq), (0, 7));
        assert!(r.set(Status::Ready, &mut seq));
        assert_eq!((r.status, r.changed, seq), (Status::Ready, 7, 8));
    }

    #[test]
    fn session_pages() {
        // 13 keys a page.
        assert_eq!(page_start(0, 30, 13), 0);
        assert_eq!(page_start(13, 30, 13), 13);
        assert_eq!(page_start(26, 30, 13), 26);
        // Past the end (sessions closed): the last page.
        assert_eq!(page_start(39, 30, 13), 26);
        assert_eq!(page_start(13, 13, 13), 0);
        assert_eq!(page_start(13, 0, 13), 0);
        // Fewer keys on this page than where the offset came from: the page that holds it.
        assert_eq!(page_start(13, 30, 5), 10);
        assert_eq!(page_start(5, 30, 0), 0);
    }

    #[test]
    fn back_leaves_the_profile() {
        // Pinned Claude key pressed in Ghostty (1): back to Ghostty.
        assert_eq!(back_target(5, Some(1), 1, 6), Some(1));
        // Claude app focused (its profile 5 came from focus), Discord (2) before it.
        assert_eq!(back_target(5, Some(2), 5, 6), Some(2));
        // Nothing before: the default profile.
        assert_eq!(back_target(5, None, 5, 6), Some(0));
        // A previous profile that no longer exists is skipped.
        assert_eq!(back_target(5, Some(9), 5, 6), Some(0));
        assert_eq!(back_target(0, None, 0, 1), None);
    }
}

//! The controller owns all hub state (profile, page, folder stack, what every display shows)
//! and is the only place that decides what happens on input. Workers do I/O, the IPC server
//! reads the shared [`Snapshot`].

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use masterhub_core::config::{Action, Color, Config, Face, Leds, Page, Side, Widget};
use masterhub_core::ipc::{DisplayImage, Event, GUI_APP_ID, HubState, InputMsg, ModuleState, Status, TARGET_DIAL};
use masterhub_core::layout;
use masterhub_core::render::{DIAL_SIZE, KEY_SIZE, LiveData, Picture, Renderer, load_picture};
use masterhub_proto::{
    ImageTarget, InputReport, KEY_COUNT, KeyTracker, LedMode, LedTarget, ModuleKind, Rgb, Roll, Rotation,
    led_brightness, led_color, led_mode,
};

use crate::actions::Request;
use crate::claude::SessionFile;
use crate::ipc::Broadcaster;
use crate::hid::ModuleInfo;
use crate::stats::Stats;
use crate::worker::DeviceCmd;

pub enum Msg {
    Connected { info: ModuleInfo, firmware: Option<String>, commands: Sender<DeviceCmd> },
    Disconnected { serial: String },
    Input { serial: String, report: InputReport },
    ActiveApp(String),
    /// A validated config from the app (IPC) (already saved to disk) with its TOML text.
    ApplyConfig { config: Box<Config>, text: String },
    SetProfile(String),
    ShowPage(String),
    /// A tool's status for the Base LED; `None` forgets the source.
    SetStatus { source: String, status: Option<Status>, pid: Option<u32> },
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
    /// What Claude Code wrote about the session, re-read every tick (the name changes).
    file: Option<SessionFile>,
    /// Found in the session files rather than reported by a hook; its status follows the file
    /// until the first hook event.
    discovered: bool,
}

pub struct Controller {
    config: Config,
    config_path: PathBuf,
    config_mtime: Option<SystemTime>,
    renderer: Renderer,
    pictures: HashMap<(PathBuf, u32), Option<Rc<Picture>>>,
    modules: BTreeMap<String, Module>,
    profile: usize,
    page: usize,
    folders: Vec<String>,
    /// Last orientation reported by the Base's sensor, if any.
    sensor_horizontal: Option<bool>,
    active_app: Option<String>,
    anims: HashMap<(String, u8), Anim>,
    stats: Stats,
    live: HashMap<Widget, LiveData>,
    actions: Sender<Request>,
    snapshot: Arc<Mutex<Snapshot>>,
    events: Arc<Broadcaster>,
    /// Tool status per source, with the process that reported it.
    statuses: HashMap<String, Reporter>,
    /// Counts status reports, to order sessions that started at the same time.
    reports: u64,
    /// Advances every tick; animates working sessions.
    ticks: u32,
    /// Status on the Base LED, whether a flash is lit and when it toggles next.
    shown_status: Option<Status>,
    blink_on: bool,
    blink_due: Option<Instant>,
}

const TICK: Duration = Duration::from_secs(1);
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
        let c = Self {
            renderer: Renderer::new(config.font.as_deref()),
            config_mtime: mtime(&config_path),
            config,
            config_path,
            pictures: HashMap::new(),
            modules: BTreeMap::new(),
            profile: 0,
            page: 0,
            folders: Vec::new(),
            sensor_horizontal: None,
            active_app: None,
            anims: HashMap::new(),
            stats: Stats::new(),
            live: HashMap::new(),
            actions,
            snapshot,
            events,
            statuses: HashMap::new(),
            reports: 0,
            ticks: 0,
            shown_status: None,
            blink_on: true,
            blink_due: None,
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
            // The GUI's own window: GTK uses the app ID, Tauri the binary name (openmasterhub[.exe]).
            Msg::ActiveApp(class) if class == GUI_APP_ID || class.to_ascii_lowercase().starts_with("openmasterhub") => {}
            Msg::ActiveApp(class) => {
                let target = self.config.profile_for_app(&class);
                self.active_app = Some(class);
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
            Msg::SetStatus { source, status, pid } => {
                log::debug!("status {source}: {status:?}");
                match status {
                    Some(status) => {
                        let file = pid.filter(|_| source.starts_with("claude:")).and_then(SessionFile::of);
                        let first = self.statuses.get(&source).map(|r| r.first).unwrap_or(self.reports);
                        self.reports += 1;
                        self.statuses.insert(source, Reporter { status, pid, first, file, discovered: false });
                    }
                    None => {
                        self.statuses.remove(&source);
                    }
                }
                self.update_status();
                self.refresh_sessions();
            }
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

    fn switch_profile(&mut self, index: usize) {
        if index >= self.config.profiles.len() {
            return;
        }
        self.profile = index;
        self.page = 0;
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
            Action::Back(_) => {
                self.folders.pop();
            }
            Action::Profile(name) => {
                if let Some(i) = self.config.profiles.iter().position(|p| &p.name == name) {
                    self.switch_profile(i);
                }
                return true;
            }
            Action::Cmd(_) | Action::Keys(_) | Action::Volume(_) | Action::Mute(_) | Action::Session(_) => return false,
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
            let _ = m.tx.send(DeviceCmd::Backlight(self.config.backlight));
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
        let Some(colors) = colors else { return };
        let brightness = leds.brightness.unwrap_or(100);
        let mut reports = Vec::new();
        for i in 0..kind.led_count() {
            let Some(c) = colors.get(i as usize) else { continue };
            let t = LedTarget::Index(i);
            reports.extend(led_mode(kind, t, LedMode::Static));
            reports.extend(led_color(kind, t, c.0));
            reports.extend(led_brightness(kind, t, brightness));
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
        if attention != self.blink_due.is_some() {
            self.blink_on = true;
            self.blink_due = attention.then(|| Instant::now() + BLINK);
        }
        let top = if self.config.status.enabled { self.statuses.values().map(|r| r.status).max() } else { None };
        if top == self.shown_status {
            return;
        }
        log::info!("base status: {top:?}");
        self.shown_status = top;
        let Some(serial) = self.base_serial() else { return };
        if top.is_none() && self.effective_leds().base.is_none() {
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
        for (source, r) in &mut self.statuses {
            if let Some(pid) = r.pid.filter(|_| source.starts_with("claude:") && !r.discovered) {
                r.file = SessionFile::of(pid);
            }
        }
        let mut changed = self.statuses.len() != before;
        for (pid, file) in SessionFile::running() {
            let source = format!("claude:{}", file.session_id.as_deref().unwrap_or_default());
            match self.statuses.get_mut(&source) {
                Some(r) if !r.discovered => {}
                Some(r) => {
                    changed |= r.status != file.status();
                    r.status = file.status();
                    r.file = Some(file);
                }
                None => {
                    log::info!("found Claude Code session {source} (pid {pid})");
                    let r = Reporter { status: file.status(), pid: Some(pid), first: self.reports, file: Some(file), discovered: true };
                    self.reports += 1;
                    self.statuses.insert(source, r);
                    changed = true;
                }
            }
        }
        if changed {
            self.update_status();
        }
    }

    // ───────────────────────────── Claude Code sessions ─────────────────────────────

    /// Open Claude Code sessions in slot order: oldest first, so keys don't move around.
    fn sessions(&self) -> Vec<&Reporter> {
        let mut v: Vec<&Reporter> =
            self.statuses.iter().filter(|(source, _)| source.starts_with("claude:")).map(|(_, r)| r).collect();
        v.sort_by_key(|r| (r.file.as_ref().and_then(|f| f.started_at).unwrap_or(u64::MAX), r.first));
        v
    }

    /// What a session key shows; `None` for an empty slot.
    fn session_live(&self, slot: u8) -> Option<LiveData> {
        let r = *self.sessions().get(slot as usize)?;
        let c = &self.config.status;
        let (color, frame) = match r.status {
            Status::Attention => (c.attention, u32::from(!self.blink_on)),
            Status::Ready => (c.ready, 0),
            Status::Working => (c.working, self.ticks),
        };
        let name = r.file.as_ref().and_then(SessionFile::title).unwrap_or_else(|| "Claude".to_owned());
        Some(LiveData::Session { name, status: r.status, color: color.0, frame })
    }

    /// Redraw session keys after a status change or a flash, if any are on screen.
    fn refresh_sessions(&mut self) {
        let slots: Vec<Widget> =
            self.visible_widgets().into_iter().filter(|w| matches!(w, Widget::ClaudeSession { .. })).collect();
        if slots.is_empty() {
            return;
        }
        for w in slots {
            self.set_session_live(w);
        }
        self.refresh_all();
    }

    fn set_session_live(&mut self, w: Widget) {
        let Widget::ClaudeSession { slot } = w else { return };
        match self.session_live(slot) {
            Some(live) => self.live.insert(w, live),
            None => self.live.remove(&w),
        };
    }

    /// Bring up the session in `slot`, off the controller thread.
    fn bring_up(&self, slot: u8) {
        let Some(r) = self.sessions().get(slot as usize).copied() else { return };
        let (pid, file) = (r.pid, r.file.clone());
        std::thread::spawn(move || crate::claude::bring_up(pid, file.as_ref()));
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
            self.page_ref()?.key(target).map(|k| k.face.clone())
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
        let live = face.widget.as_ref().and_then(|w| self.live.get(w));
        let turn = self.turn_for(serial);
        let jpeg = match self.renderer.render(&face, size, frame, live, turn) {
            Ok(j) => Arc::new(j),
            Err(e) => {
                log::error!("rendering: {e:#}");
                return;
            }
        };
        let Some(m) = self.modules.get_mut(serial) else { return };
        if m.shown.get(&target).is_some_and(|old| *old == jpeg) {
            return;
        }
        m.shown.insert(target, jpeg.clone());
        let it = if target == TARGET_DIAL { ImageTarget::Dial } else { ImageTarget::Key(target) };
        let _ = m.tx.send(DeviceCmd::Image { target: it, jpeg: jpeg.clone() });
        self.snapshot.lock().unwrap().displays.insert((serial.to_owned(), target), jpeg.clone());
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
        for w in needed {
            self.set_session_live(w);
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
                if let Some(w) = self.face_for(t).and_then(|f| f.widget) {
                    needed.push(w);
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
                }
            }
            InputReport::Unknown { .. } => {}
        }

        for (control, index, event, value) in events {
            log::debug!("{control} {index} {event} {value:?}");
            let msg = InputMsg {
                serial: serial.to_owned(),
                control: control.into(),
                index,
                event: event.into(),
                value,
            };
            self.events.send(Event::Input(msg));
            if let Some(action) = self.binding(control, index, event) {
                if let Action::Session(slot) = action {
                    self.bring_up(slot);
                    continue;
                }
                let coalesce = (control == "fader").then(|| format!("{serial}/fader{index}"));
                if !self.navigate(&action) {
                    let _ = self.actions.send(Request { action, value, coalesce });
                }
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
                let k = self.page_ref()?.key(index)?;
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
                let r = p.rollers.iter().find(|r| r.side == side)?;
                if event == "away" { r.away.clone() } else { r.toward.clone() }
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

fn mtime(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

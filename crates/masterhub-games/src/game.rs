//! Game mode: while a game's window has focus, a second claim takes the keypad, the dial and
//! both rollers above daily mode. The knobs stay daily (the top knob already follows the
//! focused app, so it is the game's volume). Keys send the game's own binds through masterhubd's
//! virtual keyboard, or commands to the game's bridge mod (see `bridge`). The left roller flips
//! pages, the right one the dial's stat cards. An intro plays across all screens the first time
//! the game takes the hub after it starts.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use image::RgbImage;
use masterhub_core::config::Action;
use masterhub_core::ipc::{ClientEvent, Controls, Event, InputMsg, TARGET_DIAL};
use pulpit_render::{DialCard, DialFace, IntroStyle, KeyFace, Orb, PRESS_SECS, Renderer, Theme};

use crate::bridge::{Bridge, Stats};
use crate::hub::Hub;

const TICK: Duration = Duration::from_millis(25);
/// Above daily mode (0); the gallery (10) still wins.
const PRIORITY: i32 = 5;
const INTRO_SECS: f32 = 4.0;
/// The intro's last part blends into the real keys.
const INTRO_BLEND: f32 = 0.75;
/// How long the dial shows the page name or the volume after a change.
const SHOWN: Duration = Duration::from_millis(1500);
/// The eddies amount picker closes by itself after this long untouched.
const AMOUNT_OPEN: Duration = Duration::from_secs(8);
const AMOUNTS: [u32; 9] = [1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000, 1_000_000];
const AMOUNT_DEFAULT: usize = 2;
/// How long after a fight the combat page stays.
const COMBAT_LINGER: Duration = Duration::from_secs(5);

const RED: [u8; 3] = [0xFF, 0x3B, 0x3B];
const GREY: [u8; 3] = [0x60, 0x60, 0x68];

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// The game's own binds.
    Action,
    /// Cheats through the bridge.
    Cheat,
    /// On/off cheats (with an orb while on).
    Toggle,
}

/// A game's colours: (ground, accent) per key kind, and the dial ring at rest.
#[derive(Clone, Copy)]
pub struct Palette {
    pub action: ([u8; 3], [u8; 3]),
    pub cheat: ([u8; 3], [u8; 3]),
    pub toggle: ([u8; 3], [u8; 3]),
    pub ring: [u8; 3],
}

impl Palette {
    fn colors(&self, kind: Kind) -> ([u8; 3], [u8; 3]) {
        match kind {
            Kind::Action => self.action,
            Kind::Cheat => self.cheat,
            Kind::Toggle => self.toggle,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Press {
    /// A key combination (`o@800` holds it).
    Keys(&'static str),
    /// A shell command.
    Cmd(&'static str),
    /// A bridge command.
    Bridge(&'static str),
    /// Open the eddies amount picker on the dial (a second press gives).
    Amount,
    /// Nothing: the key only shows something.
    Show,
}

/// A readout a key shows instead of an icon (a display page): the value big, the label under it.
#[derive(Clone, Copy, PartialEq)]
pub enum Stat {
    Time,
    Session,
    Date,
    Cpu,
    GpuTemp,
    GpuLoad,
    Vram,
    /// The music player's track as the label; the icon shows play or pause.
    Track,
}

pub struct GKey {
    pub label: &'static str,
    pub icon: char,
    pub kind: Kind,
    pub press: Press,
    pub minor: Option<&'static str>,
    /// Pictures for looks that draw them, best first (`"cd-backpack|knapsack"`, see pulpit-render's art).
    pub art: Option<&'static str>,
    /// A readout instead of the icon.
    pub stat: Option<Stat>,
    /// This key's own (ground, accent) instead of its kind's colours.
    pub tint: Option<([u8; 3], [u8; 3])>,
    /// For toggles: whether it is on now.
    pub on: Option<fn(&Stats) -> bool>,
}

/// A key with pictures for its look (see [`GKey::art`]).
pub const fn ka(label: &'static str, icon: char, art: &'static str, kind: Kind, press: Press) -> GKey {
    GKey { label, icon, kind, press, minor: None, art: Some(art), stat: None, tint: None, on: None }
}

pub const fn k(label: &'static str, icon: char, kind: Kind, press: Press) -> GKey {
    GKey { label, icon, kind, press, minor: None, art: None, stat: None, tint: None, on: None }
}

pub struct Page {
    pub name: &'static str,
    pub keys: Vec<GKey>,
    /// Filled from the spawn panel's favourites instead of `keys`.
    pub favs: bool,
    /// Filled from the spawn panel's loadout sets; a press equips the set.
    pub loadouts: bool,
}

/// One item of a loadout set.
#[derive(Debug, Clone, serde::Deserialize)]
struct LoItem {
    id: String,
    #[serde(default)]
    qty: Option<u32>,
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    slot: Option<i64>,
    #[serde(default)]
    car: bool,
}

/// A bought perk saved in a set.
#[derive(Debug, Clone, serde::Deserialize)]
struct LoPerk {
    id: String,
    #[serde(default)]
    level: Option<u32>,
}

/// A loadout set from the spawn panel.
#[derive(Debug, Clone, serde::Deserialize)]
struct Loadout {
    name: String,
    #[serde(default)]
    items: Vec<LoItem>,
    /// The perk tree (optional).
    #[serde(default)]
    perks: Vec<LoPerk>,
    #[serde(default)]
    with_perks: Option<bool>,
    /// Body, Reflexes, Tech, Intelligence, Cool (3-20 each, 81 in total).
    #[serde(default)]
    attrs: Option<Vec<u32>>,
}

impl Loadout {
    /// The bridge's `apply` argument: "equip;id qty area slot|…" (vehicles go apart).
    fn apply_arg(&self, equip: bool) -> String {
        let parts: Vec<String> = self
            .items
            .iter()
            .filter(|i| !i.car)
            .map(|i| {
                let area = i.area.as_deref().filter(|a| !a.is_empty()).unwrap_or("-");
                let slot = i.slot.map_or("-".to_string(), |s| s.to_string());
                format!("{} {} {area} {slot}", i.id, i.qty.unwrap_or(1).max(1))
            })
            .collect();
        format!("{};{}", if equip { 1 } else { 0 }, parts.join("|"))
    }

    /// The bridge's `perks_apply` argument, when the set carries a tree and it is switched on.
    fn perks_arg(&self) -> Option<String> {
        (!self.perks.is_empty() && self.with_perks != Some(false)).then(|| {
            self.perks.iter().map(|p| format!("{} {}", p.id, p.level.unwrap_or(1).max(1))).collect::<Vec<_>>().join("|")
        })
    }
}

/// A favourite starred in the spawn panel (games/cyberpunk/tools/panel.py).
#[derive(Debug, Clone, serde::Deserialize)]
struct Fav {
    id: String,
    name: String,
    #[serde(default)]
    car: bool,
    #[serde(default)]
    qty: Option<u32>,
}

/// One game's mode. Each game is a file in `games/`, listed in `games::ALL` and switched in
/// the config's `[games]` table.
pub struct Game {
    pub claim_name: &'static str,
    /// The name on the dial.
    pub title: &'static str,
    /// Window class glob of the game.
    pub class: &'static str,
    /// Text in the game's command line (Wine renames the process, so not its name).
    pub process: &'static str,
    pub theme: Theme,
    pub palette: Palette,
    /// Played across all screens the first time the game takes the hub after it starts.
    pub intro: Option<IntroStyle>,
    /// The folder of the game's bridge mod (stats out, commands in); `None` = keys only.
    pub bridge_dir: Option<PathBuf>,
    pub pages: Vec<Page>,
    /// The page that takes over while the player is in combat.
    pub combat_page: Option<usize>,
    /// Spawn panel favourites (JSON list), shown on the `favs` page.
    pub favs_file: Option<PathBuf>,
    /// Spawn panel loadout sets, shown on the `loadouts` page.
    pub loadouts_file: Option<PathBuf>,
    /// Opens the spawn panel; run when the game starts.
    pub panel: Option<&'static str>,
    /// Live readouts from files the game writes (logs, add-on saves), for games without a bridge.
    pub feed: Option<Box<dyn Feed>>,
}

/// A game's live readouts read from its own files (no bridge mod): the fight flag for the
/// combat page and the dial's cards. Polled from the mode's thread a few times a second.
pub trait Feed: Send {
    fn update(&mut self);
    /// The player is in a fight (switches to the combat page).
    fn fighting(&self) -> bool;
    /// The dial cards the right roller flips through, most important first; the fight card
    /// alone while fighting. `ring` is the game's resting ring colour.
    fn cards(&self, ring: [u8; 3]) -> Vec<DialCard>;
    /// What the feed knows better than the page for this key (a bar slot's spell, its icon,
    /// its cooldown).
    fn key(&self, _key: &GKey) -> Option<KeyPatch> {
        None
    }
}

/// A feed's changes to a key's face.
#[derive(Debug, Clone, Default)]
pub struct KeyPatch {
    pub label: Option<String>,
    pub logo: Option<String>,
    pub minor: Option<String>,
    /// Cooldown left, 0..=1.
    pub cooldown: Option<f32>,
}

/// Window classes of the games switched on (set once at start).
static CLASSES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();

/// Whether `class` is the window of a game whose mode is switched on.
pub fn is_game(class: &str) -> bool {
    // A `class@title` pattern counts by its class here.
    CLASSES.get().is_some_and(|c| c.iter().any(|g| masterhub_core::config::glob_match(g.split('@').next().unwrap_or(g), class)))
}

/// Start the mode of every game in `games`, each on its own thread.
pub fn spawn_all(games: Vec<Game>) {
    let _ = CLASSES.set(games.iter().map(|g| g.class).collect());
    games.into_iter().for_each(spawn);
}

/// A picture of a game's fixed pages at rest (5×3 keys per page, 3× size), to judge a look
/// without the hub: `pulpit sheet NAME OUT.png`.
pub fn sheet(game: &Game, renderer: &Renderer) -> RgbImage {
    const S: u32 = 3;
    let cell = pulpit_render::KEY_SIZE * S + 12;
    let pages: Vec<&Page> = game.pages.iter().filter(|p| !p.favs && !p.loadouts).collect();
    let mut out = RgbImage::from_pixel(5 * cell + 12, pages.len() as u32 * (3 * cell + 24) + 12, image::Rgb([8, 8, 10]));
    for (pi, page) in pages.iter().enumerate() {
        for (i, key) in page.keys.iter().enumerate().take(15) {
            let (ground, accent) = key.tint.unwrap_or(game.palette.colors(key.kind));
            let mut face = KeyFace {
                label: key.label.into(),
                icon: Some(key.icon),
                logo: key.art.map(Into::into),
                minor: key.minor.map(Into::into),
                ground: Some(ground),
                accent: Some(accent),
                ..Default::default()
            };
            // Readouts with sample values.
            match key.stat {
                Some(Stat::Track) => face.label = "SAMPLE TRACK".into(),
                Some(stat) => {
                    let sample = match stat {
                        Stat::Time => "21:58",
                        Stat::Session => "1:47",
                        Stat::Date => "23",
                        Stat::Cpu => "18%",
                        Stat::GpuTemp => "64°",
                        Stat::GpuLoad => "90%",
                        _ => "16.4",
                    };
                    if stat == Stat::Date {
                        face.label = "MON NOV".into();
                    }
                    (face.icon, face.logo, face.minor) = (None, None, Some(sample.into()));
                }
                None => {}
            }
            let img = renderer.key_in(game.theme, &face, 0.0, 0.0);
            let big = image::imageops::resize(&img, img.width() * S, img.height() * S, image::imageops::FilterType::Nearest);
            let (x, y) = (12 + (i as u32 % 5) * cell, 12 + pi as u32 * (3 * cell + 24) + (i as u32 / 5) * cell);
            image::imageops::replace(&mut out, &big, x as i64, y as i64);
        }
    }
    out
}

/// Start a game mode on its own thread with its own connection to masterhubd.
fn spawn(game: Game) {
    std::thread::Builder::new()
        .name(game.claim_name.into())
        .spawn(move || match Renderer::new() {
            Ok(r) => Mode::new(game, r).run(),
            Err(e) => log::error!("game mode: {e:#}"),
        })
        .expect("spawn game mode");
}

/// The running game's process id: the first process whose command starts with a path
/// ending in `exe` (Wine and Proton keep the Windows path there).
fn game_pid(exe: &str) -> Option<u32> {
    std::fs::read_dir("/proc").ok()?.flatten().find_map(|e| {
        let pid: u32 = e.file_name().to_str()?.parse().ok()?;
        let cmd = std::fs::read(e.path().join("cmdline")).ok()?;
        let first = cmd.split(|&b| b == 0).next()?;
        String::from_utf8_lossy(first).ends_with(exe).then_some(pid)
    })
}

/// The default sink's volume (0..1) and mute, read once at the start of a dial turn.
fn read_volume() -> (f32, bool) {
    let out = std::process::Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output();
    let s = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let v = s.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    (v, s.contains("MUTED"))
}

/// 85749 → "85,749".
fn thousands(n: f64) -> String {
    let s = format!("{}", n.max(0.0).round() as u64);
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn blend(a: &RgbImage, b: &RgbImage, k: f32) -> RgbImage {
    RgbImage::from_fn(a.width(), a.height(), |x, y| {
        let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
        image::Rgb(std::array::from_fn(|i| (p[i] as f32 * (1.0 - k) + q[i] as f32 * k).round() as u8))
    })
}

struct Mode {
    game: Game,
    renderer: Renderer,
    hub: Hub,
    bridge: Option<Bridge>,
    start: Instant,
    owns: bool,
    page: usize,
    card: i32,
    press: [Option<Instant>; 15],
    sent: [Option<KeyFace>; 15],
    dial_sent: Option<DialFace>,
    intro: Option<Instant>,
    intro_pid: Option<u32>,
    /// The game process and when we first saw it (session time).
    session: Option<(u32, Instant)>,
    page_shown: Option<Instant>,
    volume: Option<(f32, bool, Instant)>,
    /// The amount picker: index into AMOUNTS, last touched.
    amount: Option<(usize, Instant)>,
    amount_idx: usize,
    stats: Option<Stats>,
    stats_at: Instant,
    /// In combat: the page to go back to, and when combat was last seen.
    combat: Option<(usize, Instant)>,
    favs: Vec<Fav>,
    favs_at: Instant,
    loadouts: Vec<Loadout>,
    /// The set last equipped from the hub, shown on its key for a moment.
    applied: Option<(String, Instant)>,
    /// Machine and music readouts, when the game has a display page.
    sys: Option<std::sync::Arc<std::sync::Mutex<crate::live::Sys>>>,
    /// Tells the readout thread whether anyone is looking.
    live: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Mode {
    fn new(game: Game, renderer: Renderer) -> Self {
        let bridge = game.bridge_dir.clone().map(Bridge::new);
        let live = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sys = game.pages.iter().any(|p| p.keys.iter().any(|k| k.stat.is_some())).then(|| crate::live::start(live.clone()));
        Self {
            game,
            renderer,
            hub: Hub::start(),
            bridge,
            start: Instant::now(),
            owns: false,
            page: 0,
            card: 0,
            press: [None; 15],
            sent: Default::default(),
            dial_sent: None,
            intro: None,
            intro_pid: None,
            session: None,
            page_shown: None,
            volume: None,
            amount: None,
            amount_idx: AMOUNT_DEFAULT,
            stats: None,
            stats_at: Instant::now() - Duration::from_secs(10),
            combat: None,
            favs: Vec::new(),
            favs_at: Instant::now() - Duration::from_secs(10),
            loadouts: Vec::new(),
            applied: None,
            sys,
            live,
        }
    }

    fn controls() -> Controls {
        Controls { keys: (0..15).collect(), dial: true, rollers: vec![0, 1], ..Default::default() }
    }

    fn run(mut self) {
        let mut next = Instant::now();
        loop {
            self.events();
            self.live.store(self.owns, std::sync::atomic::Ordering::Relaxed);
            if self.owns {
                self.draw();
            }
            next += TICK;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                next = now;
            }
        }
    }

    fn events(&mut self) {
        while let Ok(ev) = self.hub.events.try_recv() {
            match ev {
                ClientEvent::Online(true) => {
                    match self.hub.claim_apps(self.game.claim_name, Self::controls(), vec![self.game.class.into()], PRIORITY) {
                        Ok(id) => log::info!("game mode {}: claim #{id}", self.game.claim_name),
                        Err(e) => log::error!("game claim failed: {e:#}"),
                    }
                }
                ClientEvent::Online(false) => {
                    self.hub.claim = None;
                    self.owns = false;
                }
                ClientEvent::Event(Event::Claim(c)) if Some(c.claim) == self.hub.claim => {
                    let owns = !c.owns.keys.is_empty();
                    self.hub.key_turn = c.key_turn;
                    self.hub.dial_turn = c.dial_turn;
                    if owns && !self.owns {
                        self.took_over();
                    }
                    self.owns = owns;
                }
                ClientEvent::Event(Event::Input(i)) if i.claim.is_some() && i.claim == self.hub.claim => self.input(&i),
                _ => {}
            }
        }
    }

    /// The game got focus: redraw everything; the first time per game process, the intro.
    fn took_over(&mut self) {
        self.sent = Default::default();
        self.dial_sent = None;
        let pid = game_pid(self.game.process);
        if let Some(p) = pid {
            if self.session.is_none_or(|(s, _)| s != p) {
                self.session = Some((p, Instant::now()));
            }
            if self.intro_pid != Some(p) {
                self.intro_pid = Some(p);
                self.intro = Some(Instant::now());
                // A new game process: open the spawn panel on the second monitor.
                if let Some(cmd) = self.game.panel {
                    self.hub.run(Action::Cmd(cmd.into()), None);
                }
            }
        }
        log::info!("game mode active (pid {pid:?})");
    }

    fn bridge_up(&self) -> bool {
        self.stats.is_some()
    }

    fn send(&mut self, cmd: &str, arg: &str) {
        if let Some(b) = &mut self.bridge {
            b.send(cmd, arg);
        }
    }

    fn input(&mut self, i: &InputMsg) {
        // Any touch skips the intro.
        if self.intro.take().is_some() {
            self.sent = Default::default();
            self.dial_sent = None;
            if i.control == "key" {
                return;
            }
        }
        match (i.control.as_str(), i.event.as_str()) {
            ("key", "down") if i.index < 15 => {
                self.press[i.index as usize] = Some(Instant::now());
                if self.game.pages[self.page].loadouts {
                    if let Some(lo) = self.loadouts.get(i.index as usize).cloned()
                        && self.bridge_up()
                    {
                        if let Some(a) = lo.attrs.as_ref().filter(|a| a.len() == 5) {
                            self.send("set_attrs", &a.iter().map(u32::to_string).collect::<Vec<_>>().join(" "));
                        }
                        self.send("apply", &lo.apply_arg(true));
                        for car in lo.items.iter().filter(|i| i.car) {
                            self.send("vehicle", &car.id);
                        }
                        if let Some(perks) = lo.perks_arg() {
                            self.send("perks_apply", &perks);
                        }
                        // Unspent points back to the design's rest (80 perk, 81 attribute, 15 relic).
                        self.send("design_points", "");
                        self.applied = Some((lo.name.clone(), Instant::now()));
                    }
                    return;
                }
                if self.game.pages[self.page].favs {
                    if let Some(f) = self.favs.get(i.index as usize).cloned()
                        && self.bridge_up()
                    {
                        if f.car {
                            self.send("vehicle", &f.id);
                        } else {
                            self.send("give", &format!("{} {}", f.id, f.qty.unwrap_or(1).max(1)));
                        }
                    }
                    return;
                }
                let Some(key) = self.game.pages[self.page].keys.get(i.index as usize) else { return };
                let press = key.press;
                match press {
                    Press::Keys(k) => self.hub.run(Action::Keys(k.into()), None),
                    Press::Cmd(c) => self.hub.run(Action::Cmd(c.into()), None),
                    Press::Bridge(c) if self.bridge_up() => self.send(c, ""),
                    Press::Bridge(_) => {}
                    Press::Amount if self.bridge_up() => match self.amount {
                        Some((idx, _)) => self.give(idx),
                        None => self.amount = Some((self.amount_idx, Instant::now())),
                    },
                    Press::Amount | Press::Show => {}
                }
            }
            ("roller", ev) if i.index == 0 => {
                let n = self.game.pages.len();
                self.page = (self.page as i32 + if ev == "away" { 1 } else { -1 }).rem_euclid(n as i32) as usize;
                self.page_shown = Some(Instant::now());
                self.sent = Default::default();
            }
            ("roller", ev) => self.card += if ev == "away" { 1 } else { -1 },
            ("dial", d @ ("cw" | "ccw")) => {
                let up = d == "cw";
                if let Some((idx, _)) = self.amount {
                    let idx = if up { (idx + 1).min(AMOUNTS.len() - 1) } else { idx.saturating_sub(1) };
                    self.amount = Some((idx, Instant::now()));
                    return;
                }
                let (v, m) = match self.volume {
                    Some((v, m, at)) if at.elapsed() < SHOWN => (v, m),
                    _ => read_volume(),
                };
                let v = (v + if up { 0.02 } else { -0.02 }).clamp(0.0, 1.0);
                self.hub.run(Action::Volume(if up { "+2" } else { "-2" }.into()), None);
                self.volume = Some((v, m, Instant::now()));
            }
            ("dial", "down") => match self.amount {
                Some((idx, _)) => self.give(idx),
                None => {
                    if let Ok(a) = serde_json::from_value(serde_json::json!({ "mute": "toggle" })) {
                        self.hub.run(a, None);
                    }
                }
            },
            _ => {}
        }
    }

    /// Switch to the combat page when a fight starts; back to the page before it a few seconds
    /// after the fight ends (unless the page was flipped by hand meanwhile).
    fn follow_combat(&mut self) {
        let Some(cp) = self.game.combat_page else { return };
        let fighting = self.stats.as_ref().is_some_and(|s| s.ingame && s.combat) || self.game.feed.as_ref().is_some_and(|f| f.fighting());
        match (&mut self.combat, fighting) {
            (None, true) => {
                self.combat = Some((self.page, Instant::now()));
                self.set_page(cp);
            }
            (Some((_, seen)), true) => *seen = Instant::now(),
            (Some((back, seen)), false) if seen.elapsed() > COMBAT_LINGER => {
                let back = *back;
                self.combat = None;
                if self.page == cp {
                    self.set_page(back);
                }
            }
            _ => {}
        }
    }

    fn set_page(&mut self, page: usize) {
        if page != self.page {
            self.page = page;
            self.page_shown = Some(Instant::now());
            self.sent = Default::default();
        }
    }

    fn give(&mut self, idx: usize) {
        self.send("eddies", &AMOUNTS[idx].to_string());
        self.amount_idx = idx;
        self.amount = None;
    }

    fn fav_face(&self, index: usize) -> KeyFace {
        let Some(f) = self.favs.get(index) else {
            let mut e = KeyFace { ground: Some([0x0A, 0x0A, 0x0E]), accent: Some([0x30, 0x30, 0x38]), ..Default::default() };
            if index == 0 {
                e.label = "NO FAVS".into();
                e.minor = Some("STAR IN PANEL".into());
            }
            return e;
        };
        let (mut ground, mut accent) = self.game.palette.colors(if f.car { Kind::Action } else { Kind::Cheat });
        let mut minor = if f.car { "TO GARAGE".to_string() } else { format!("×{}", f.qty.unwrap_or(1).max(1)) };
        if !self.bridge_up() {
            ground = [0x10, 0x10, 0x13];
            accent = [0x40, 0x40, 0x48];
            minor = "CET OFFLINE".into();
        }
        KeyFace {
            label: f.name.to_uppercase(),
            icon: char::from_u32(if f.car { 0xf366 } else { 0xf6e2 }),
            minor: Some(minor),
            ground: Some(ground),
            accent: Some(accent),
            ..Default::default()
        }
    }

    fn loadout_face(&self, index: usize) -> KeyFace {
        let Some(lo) = self.loadouts.get(index) else {
            let mut e = KeyFace { ground: Some([0x0A, 0x0A, 0x0E]), accent: Some([0x30, 0x30, 0x38]), ..Default::default() };
            if index == 0 {
                e.label = "NO SETS".into();
                e.minor = Some("MAKE IN PANEL".into());
            }
            return e;
        };
        let (mut ground, mut accent) = self.game.palette.colors(Kind::Toggle);
        let weapons = lo.items.iter().filter(|i| i.area.as_deref() == Some("Weapon")).count();
        let cw = lo.items.iter().filter(|i| i.area.as_deref().is_some_and(|a| a.ends_with("CW"))).count();
        let mut minor = format!("{weapons}W · {cw}CW{}", if lo.perks_arg().is_some() { " · P" } else { "" });
        let mut orb = None;
        if self.applied.as_ref().is_some_and(|(n, at)| n == &lo.name && at.elapsed() < Duration::from_secs(3)) {
            minor = "EQUIPPED".into();
            orb = Some(Orb::Ready);
        }
        if !self.bridge_up() {
            ground = [0x10, 0x10, 0x13];
            accent = [0x40, 0x40, 0x48];
            minor = "CET OFFLINE".into();
        }
        KeyFace {
            label: lo.name.to_uppercase(),
            icon: char::from_u32(0xfe78),
            minor: Some(minor),
            orb,
            ground: Some(ground),
            accent: Some(accent),
            ..Default::default()
        }
    }

    fn face(&self, index: usize, t: f32) -> KeyFace {
        if self.game.pages[self.page].loadouts {
            return self.loadout_face(index);
        }
        if self.game.pages[self.page].favs {
            return self.fav_face(index);
        }
        let Some(key) = self.game.pages[self.page].keys.get(index) else { return KeyFace::default() };
        let (mut ground, mut accent) = key.tint.unwrap_or(self.game.palette.colors(key.kind));
        let needs_bridge = matches!(key.press, Press::Bridge(_) | Press::Amount);
        let mut f = KeyFace {
            label: key.label.into(),
            icon: Some(key.icon),
            logo: key.art.map(Into::into),
            minor: key.minor.map(Into::into),
            ..Default::default()
        };
        if needs_bridge && !self.bridge_up() {
            ground = [0x10, 0x10, 0x13];
            accent = [0x40, 0x40, 0x48];
            f.minor = Some("CET OFFLINE".into());
        } else if let (Some(on), Some(s)) = (key.on, &self.stats) {
            let lit = on(s);
            f.orb = lit.then_some(Orb::Recording);
            f.minor = Some(if lit { "ON" } else { "OFF" }.into());
        }
        if matches!(key.press, Press::Amount) {
            if let Some((idx, _)) = self.amount {
                f.orb = Some(Orb::Working);
                f.minor = Some(format!("GIVE {}", thousands(AMOUNTS[idx] as f64)));
            } else if self.bridge_up() {
                f.minor = Some(thousands(AMOUNTS[self.amount_idx] as f64));
            }
        }
        if let Some(stat) = key.stat {
            let sys = self.sys.as_ref().and_then(|s| s.lock().ok().map(|g| g.clone())).unwrap_or_default();
            let now = jiff::Zoned::now();
            let num = |v: Option<f32>, unit: &str| v.map_or("-".to_string(), |v| format!("{}{unit}", v.round()));
            let value = match stat {
                Stat::Time => now.strftime("%H:%M").to_string(),
                Stat::Session => {
                    let mins = self.session.map_or(0, |(_, at)| at.elapsed().as_secs() / 60);
                    format!("{}:{:02}", mins / 60, mins % 60)
                }
                Stat::Date => now.strftime("%d").to_string(),
                Stat::Cpu => num(sys.cpu, "%"),
                Stat::GpuTemp => num(sys.gpu_temp, "°"),
                Stat::GpuLoad => num(sys.gpu_load, "%"),
                Stat::Vram => sys.vram_gb.map_or("-".to_string(), |v| format!("{v:.1}")),
                Stat::Track => String::new(),
            };
            match stat {
                Stat::Track => {
                    let (playing, text) = sys.track.unwrap_or((false, "NO MUSIC".into()));
                    // The game's face has letters and digits only.
                    let plain: String = text.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { ' ' }).collect();
                    f.label = plain.split_whitespace().collect::<Vec<_>>().join(" ");
                    f.logo = Some(if playing { "dl-pause" } else { "dl-play" }.into());
                }
                _ => {
                    if stat == Stat::Date {
                        f.label = now.strftime("%a %b").to_string().to_uppercase();
                    }
                    (f.icon, f.logo, f.minor) = (None, None, Some(value));
                }
            }
        }
        if let Some(p) = self.game.feed.as_ref().and_then(|feed| feed.key(key)) {
            if let Some(l) = p.label {
                f.label = l;
            }
            if p.logo.is_some() {
                f.logo = p.logo;
            }
            if p.minor.is_some() {
                f.minor = p.minor;
            }
            f.ring = p.cooldown;
        }
        let _ = t;
        f.ground = Some(ground);
        f.accent = Some(accent);
        f
    }

    fn draw(&mut self) {
        let t = self.start.elapsed().as_secs_f32();
        if self.stats_at.elapsed() > Duration::from_millis(200) {
            self.stats = self.bridge.as_ref().and_then(Bridge::stats);
            if let Some(f) = &mut self.game.feed {
                f.update();
            }
            self.stats_at = Instant::now();
        }
        if self.favs_at.elapsed() > Duration::from_secs(2) {
            self.favs_at = Instant::now();
            if let Some(p) = &self.game.favs_file {
                let favs: Vec<Fav> = std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
                self.favs = favs.into_iter().take(15).collect();
            }
            if let Some(p) = &self.game.loadouts_file {
                let los: Vec<Loadout> = std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
                self.loadouts = los.into_iter().take(15).collect();
            }
        }
        self.follow_combat();
        if self.amount.is_some_and(|(_, at)| at.elapsed() > AMOUNT_OPEN) {
            self.amount = None;
        }

        if let Some(at) = self.intro {
            let it = at.elapsed().as_secs_f32();
            if it < INTRO_SECS {
                self.draw_intro(it, t);
                return;
            }
            self.intro = None;
            self.sent = Default::default();
            self.dial_sent = None;
        }

        for i in 0..15 {
            let p = self.press[i].map_or(0.0, |at| 1.0 - at.elapsed().as_secs_f32() / PRESS_SECS);
            if p <= 0.0 {
                self.press[i] = None;
            }
            let p = p.max(0.0);
            let face = self.face(i, t);
            let moving = p > 0.0 || face.orb.is_some() || face.ring.is_some_and(|r| r > 0.0) || face.label.len() > 8 || face.minor.as_ref().is_some_and(|m| m.len() > 11);
            if !moving && self.sent[i].as_ref() == Some(&face) {
                continue;
            }
            let img = self.renderer.key_in(self.game.theme, &face, t, p);
            self.sent[i] = (p == 0.0).then_some(face);
            if let Err(e) = self.hub.frame(i as u8, &img, self.hub.key_turn) {
                log::warn!("game frame {i}: {e:#}");
            }
        }

        let dial = self.dial_face();
        if self.dial_sent.as_ref() != Some(&dial) || dial.card.as_ref().is_some_and(|c| c.breathe) {
            let img = self.renderer.dial_in(self.game.theme, &dial, t);
            if let Err(e) = self.hub.frame(TARGET_DIAL, &img, self.hub.dial_turn) {
                log::warn!("game dial: {e:#}");
            }
            self.dial_sent = Some(dial);
        }
    }

    /// Intro frames at 20 fps (every other tick): 15 keys + dial is close to the keypad's limit.
    fn draw_intro(&mut self, it: f32, t: f32) {
        if ((it / TICK.as_secs_f32()) as u32) % 2 == 1 {
            return;
        }
        let (mut keys, mut dial) = self.renderer.intro(self.game.intro.unwrap_or_default(), it, INTRO_SECS);
        let p = it / INTRO_SECS;
        if p > INTRO_BLEND {
            let k = ((p - INTRO_BLEND) / (1.0 - INTRO_BLEND)).clamp(0.0, 1.0);
            for (i, img) in keys.iter_mut().enumerate() {
                let real = self.renderer.key_in(self.game.theme, &self.face(i, t), t, 0.0);
                *img = blend(img, &real, k);
            }
            let real = self.renderer.dial_in(self.game.theme, &self.dial_face(), t);
            dial = blend(&dial, &real, k);
        }
        for (i, img) in keys.iter().enumerate() {
            let _ = self.hub.frame(i as u8, img, self.hub.key_turn);
        }
        let _ = self.hub.frame(TARGET_DIAL, &dial, self.hub.dial_turn);
    }

    fn dial_face(&self) -> DialFace {
        let card = |eyebrow: &str, big: String| DialCard { eyebrow: eyebrow.into(), big, ..Default::default() };
        let face = |c: DialCard| DialFace { card: Some(c), ..Default::default() };
        if let Some((idx, _)) = self.amount {
            let mut c = card("+ EDDIES", thousands(AMOUNTS[idx] as f64));
            c.sub = Some("TURN · PRESS TO GIVE".into());
            c.ring = Some(self.game.palette.cheat.1);
            c.progress = Some((idx + 1) as f32 / AMOUNTS.len() as f32);
            return face(c);
        }
        if let Some((v, m, at)) = self.volume
            && at.elapsed() < SHOWN
        {
            return DialFace {
                card: None,
                value: v,
                big: if m { "MUTE".into() } else { format!("{}", (v * 100.0).round()) },
                name: "VOLUME".into(),
                status: None,
            };
        }
        if self.page_shown.is_some_and(|at| at.elapsed() < SHOWN) {
            return face(card("PAGE", self.game.pages[self.page].name.into()));
        }
        let (title, ring) = (self.game.title, self.game.palette.ring);
        if let Some(feed) = &self.game.feed {
            let mut cards = feed.cards(ring);
            let mins = self.session.map_or(0, |(_, at)| at.elapsed().as_secs() / 60);
            let mut c = card("SESSION", format!("{}:{:02}", mins / 60, mins % 60));
            c.sub = Some(jiff::Zoned::now().strftime("%H:%M").to_string());
            c.ring = Some(ring);
            c.seconds_dot = true;
            cards.push(c);
            if feed.fighting() {
                cards.truncate(1);
            }
            let i = self.card.rem_euclid(cards.len() as i32) as usize;
            return face(cards.swap_remove(i));
        }
        if self.bridge.is_none() {
            // Keys only: the session's length and the time of day.
            let mins = self.session.map_or(0, |(_, at)| at.elapsed().as_secs() / 60);
            let mut c = card(title, format!("{}:{:02}", mins / 60, mins % 60));
            c.sub = Some(format!("SESSION · {}", jiff::Zoned::now().strftime("%H:%M")));
            c.ring = Some(ring);
            c.seconds_dot = true;
            return face(c);
        }
        let Some(s) = &self.stats else {
            let mut c = card(title, "CET OFFLINE".into());
            c.sub = Some("GAME KEYS STILL WORK".into());
            c.ring = Some(GREY);
            return face(c);
        };
        if !s.ingame {
            let mut c = card(title, "MENU".into());
            c.ring = Some(ring);
            return face(c);
        }
        if s.combat {
            let hp = s.hp.unwrap_or(0.0);
            let mut c = card("COMBAT", format!("{}", hp.round()));
            c.sub = Some(format!(
                "RAM {}/{} · HEAL {}%",
                s.ram.unwrap_or(0.0).round(),
                s.ram_max.unwrap_or(0.0).round(),
                s.heal_charge.unwrap_or(0.0).round()
            ));
            let mut line = Vec::new();
            if let Some(v) = s.sandy.filter(|v| *v < 100.0) {
                line.push(format!("SANDY {}%", v.round()));
            }
            if let Some(v) = s.berserk.filter(|v| *v < 100.0) {
                line.push(format!("BERSERK {}%", v.round()));
            }
            if line.is_empty() {
                line.push("CYBERWARE READY".into());
            }
            if let Some(w) = s.wanted.filter(|w| *w > 0.0) {
                line.push(format!("NCPD {}", w.round()));
            }
            c.line = Some(line.join(" · "));
            c.ring = Some(if hp < 35.0 { RED } else { ring });
            c.progress = Some((hp / 100.0) as f32);
            c.breathe = hp < 35.0;
            return face(c);
        }
        if let Some(w) = s.wanted.filter(|w| *w > 0.0) {
            let mut c = card("NCPD", format!("{}", w.round()));
            c.sub = Some("STARS · WANTED KEY CLEARS".into());
            c.ring = Some(RED);
            c.progress = Some((w / 5.0) as f32);
            c.breathe = true;
            return face(c);
        }

        let mut cards = Vec::new();
        let mut main = card(
            &s.district.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| "NIGHT CITY".into()).to_uppercase(),
            s.eddies.map_or("—".into(), thousands),
        );
        main.sub = Some(format!("LVL {} · CRED {}", s.level.unwrap_or(0.0), s.cred.unwrap_or(0.0)));
        main.line = Some(format!(
            "HP {} · RAM {}/{}",
            s.hp.unwrap_or(0.0).round(),
            s.ram.unwrap_or(0.0).round(),
            s.ram_max.unwrap_or(0.0).round()
        ));
        main.ring = Some(if s.hp.unwrap_or(100.0) < 35.0 { RED } else { ring });
        main.progress = s.hp.map(|h| (h / 100.0) as f32);
        cards.push(main);
        if let Some(v) = s.speed {
            let mut c = card("DRIVING", format!("{}", v.round()));
            c.sub = Some("KM/H".into());
            c.ring = Some(ring);
            c.progress = Some((v / 200.0).min(1.0) as f32);
            cards.push(c);
        }
        if let Some(w) = &s.weapon {
            let mag = w.mag.unwrap_or(0.0);
            let big = match w.mag_left {
                Some(l) => format!("{mag}|{l}"),
                None => format!("{mag}"),
            };
            let mut c = card(&w.name.clone().unwrap_or_default().to_uppercase(), big);
            c.sub = Some(format!("RESERVE {}", w.reserve.unwrap_or(0.0)));
            c.ring = Some(self.game.palette.cheat.1);
            cards.push(c);
        }
        let mins = self.session.map_or(0, |(_, at)| at.elapsed().as_secs() / 60);
        let mut c = card("SESSION", format!("{}:{:02}", mins / 60, mins % 60));
        c.sub = s.time.as_ref().map(|t| format!("NIGHT CITY {t}"));
        let toggles: Vec<&str> = [(s.god, "GOD"), (s.stamina_inf, "STAMINA")].iter().filter(|x| x.0).map(|x| x.1).collect();
        c.line = (!toggles.is_empty()).then(|| toggles.join(" · ") + " ON");
        c.ring = Some(GREY);
        cards.push(c);

        let i = self.card.rem_euclid(cards.len() as i32) as usize;
        face(cards.swap_remove(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadout_apply_arg() {
        let lo: Loadout = serde_json::from_str(
            r#"{"name":"A","items":[{"id":"Items.X","qty":2,"area":"Weapon","slot":0},{"id":"Items.Y"},{"id":"Vehicle.v","car":true}]}"#,
        )
        .unwrap();
        assert_eq!(lo.apply_arg(true), "1;Items.X 2 Weapon 0|Items.Y 1 - -");
        assert_eq!(lo.perks_arg(), None);
        let lo: Loadout = serde_json::from_str(r#"{"name":"B","perks":[{"id":"NewPerks.A","level":3},{"id":"NewPerks.B"}]}"#).unwrap();
        assert_eq!(lo.perks_arg().as_deref(), Some("NewPerks.A 3|NewPerks.B 1"));
    }

    #[test]
    fn thousands_groups() {
        assert_eq!(thousands(85749.0), "85,749");
        assert_eq!(thousands(1_000_000.0), "1,000,000");
        assert_eq!(thousands(999.0), "999");
    }
}

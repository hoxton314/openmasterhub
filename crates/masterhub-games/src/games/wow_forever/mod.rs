//! WoW Forever (Battle.net under Lutris/Proton): an online game, so every hub key is one of
//! the game's own binds pressed once; nothing chained, nothing reads the game. Live readouts
//! come only from files the game writes itself: the combat and chat logs (see `logs`).
//! Binds: the client's defaults with the account's own changes (`bindings-cache.wtf`) on top,
//! read at start. Game-side notes and tools: games/wow_forever.

mod logs;
mod saved;
mod strip;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use pulpit_render::{DialCard, IntroStyle, Theme, WowFrame, WowPress, WowRing, WowStyle};

use crate::game::{Feed, GKey, Game, KeyPatch, Kind, Page, Palette, Press, Stat};

const REC_SAVE: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec save";
const REC_REPLAY: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec replay";
const REC_RECORD: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec record";

/// (ground, light) per kind of key: stand-in colours until the look round.
type Tint = ([u8; 3], [u8; 3]);
const WINDOW: Tint = ([0x2A, 0x22, 0x14], [0xF0, 0xC7, 0x5E]);
const SPELL: Tint = ([0x14, 0x22, 0x3A], [0x8F, 0xC3, 0xFF]);
const TRAVEL: Tint = ([0x17, 0x2E, 0x1C], [0x9B, 0xE0, 0x8A]);
const TOOL: Tint = ([0x22, 0x20, 0x26], [0xC9, 0xC2, 0xB4]);
/// The XP bar's purple, the dial's resting ring.
const XP: [u8; 3] = [0xA3, 0x6B, 0xFF];
const RED: [u8; 3] = [0xE0, 0x3A, 0x2E];
const GOLD: [u8; 3] = [0xF0, 0xC7, 0x5E];

fn install() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_")
}

/// The client's default keys for the actions the pages use (WoW binding names).
const DEFAULTS: &[(&str, &str)] = &[
    ("OPENALLBAGS", "B"),
    ("TOGGLECHARACTER0", "C"),
    ("TOGGLESPELLBOOK", "P"),
    ("TOGGLETALENTS", "N"),
    ("TOGGLEPROFESSIONBOOK", "K"),
    ("TOGGLEQUESTLOG", "L"),
    ("TOGGLEWORLDMAP", "M"),
    ("TOGGLESOCIAL", "O"),
    ("TOGGLEGUILDTAB", "J"),
    ("TOGGLEGROUPFINDER", "I"),
    ("TOGGLEACHIEVEMENT", "Y"),
    ("TOGGLECOLLECTIONS", "SHIFT-P"),
    ("TOGGLEENCOUNTERJOURNAL", "SHIFT-J"),
    ("TOGGLEGAMEMENU", "ESCAPE"),
    ("TOGGLEAUTORUN", "NUMLOCK"),
    ("SITORSTAND", "X"),
    ("TOGGLESHEATH", "Z"),
    ("TOGGLERUN", "NUMPADDIVIDE"),
    ("TOGGLEUI", "ALT-Z"),
    ("SCREENSHOT", "PRINTSCREEN"),
    ("TARGETNEARESTENEMY", "TAB"),
    ("REPLY", "R"),
    ("OPENCHAT", "ENTER"),
];

/// The picked look (round 2 board, 2026-11-24): action buttons, the cast press, the XP ring
/// (the zone card shows the minimap).
const THEME: Theme = Theme::Wow(WowStyle { frame: WowFrame::Button, press: WowPress::Cast, ring: WowRing::Xp });

/// Action → the hub's key combination ("ctrl+1").
struct Binds(HashMap<String, String>);

/// A combination as the action bar's hotkey text shows it: "ctrl+1" → "c1", "shift+p" → "sP".
fn hotkey(combo: &str) -> String {
    let mut out = String::new();
    for part in combo.split('+') {
        out.push_str(&match part {
            "ctrl" => "c".into(),
            "shift" => "s".into(),
            "alt" => "a".into(),
            "numlock" => "NL".into(),
            "sysrq" => "PS".into(),
            "esc" => "Esc".into(),
            "tab" => "Tab".into(),
            "enter" => "Ent".into(),
            "minus" => "-".into(),
            "equal" => "=".into(),
            "kpslash" => "N/".into(),
            k if k.starts_with("kp") => format!("N{}", &k[2..]),
            k => k.to_uppercase(),
        });
    }
    out
}

impl Binds {
    fn load() -> Self {
        let mut map: HashMap<String, String> = DEFAULTS.iter().filter_map(|(a, k)| Some((a.to_string(), combo(k)?))).collect();
        // Account binds (character binds, when used, sit in the realm folder; newest wins).
        let wtf = install().join("WTF/Account");
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = walk(&wtf, 3)
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|n| n == "bindings-cache.wtf"))
            .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
            .collect();
        files.sort();
        if let Some((_, p)) = files.last() {
            apply(&mut map, &std::fs::read_to_string(p).unwrap_or_default());
        }
        // What the game has bound right now, as our add-on saw it at the last /reload or
        // logout (bindings-cache.wtf is only written at logout): it wins.
        if let Some(snap) = saved::find(&install()).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| saved::parse(&t)) {
            for (prefix, slots) in &snap.bars {
                for (i, slot) in slots.iter().enumerate() {
                    let action = format!("{prefix}{}", i + 1);
                    match slot.bind.as_deref().and_then(combo) {
                        Some(c) => {
                            map.retain(|_, v| *v != c);
                            map.insert(action, c);
                        }
                        None => {
                            map.remove(&action);
                        }
                    }
                }
            }
        }
        Self(map)
    }

    fn key(&self, action: &str) -> Option<&'static str> {
        self.0.get(action).map(|k| &*Box::leak(k.clone().into_boxed_str()))
    }
}

fn walk(dir: &std::path::Path, depth: u32) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() && depth > 0 {
            out.extend(walk(&p, depth - 1));
        } else {
            out.push(p);
        }
    }
    out
}

/// `bind CTRL-1 MULTIACTIONBAR1BUTTON1` lines over the defaults. A key bound to a new action
/// leaves its old one; `bind KEY NONE` clears it.
fn apply(map: &mut HashMap<String, String>, text: &str) {
    for line in text.lines() {
        let mut w = line.split_whitespace();
        let (Some("bind"), Some(key), Some(action)) = (w.next(), w.next(), w.next()) else { continue };
        let Some(c) = combo(key) else { continue };
        map.retain(|_, v| *v != c);
        if action != "NONE" {
            map.insert(action.to_owned(), c);
        }
    }
}

/// A WoW key name ("CTRL-SHIFT-1", "NUMPADDIVIDE") as the hub's virtual keyboard calls it;
/// `None` for mouse buttons and wheels.
fn combo(key: &str) -> Option<String> {
    // "-" is itself a key: "CTRL--" is Ctrl and minus.
    let (mods, last) = match key.strip_suffix("--") {
        Some(m) => (m, "-"),
        None => match key.rsplit_once('-') {
            Some((m, k)) if !k.is_empty() => (m, k),
            _ => ("", key),
        },
    };
    let mut parts: Vec<String> = Vec::new();
    for m in mods.split('-').filter(|m| !m.is_empty()) {
        parts.push(
            match m {
                "CTRL" => "ctrl",
                "SHIFT" => "shift",
                "ALT" => "alt",
                "META" => "super",
                _ => return None,
            }
            .into(),
        );
    }
    let k = match last {
        k if k.starts_with("BUTTON") || k.starts_with("MOUSEWHEEL") => return None,
        "ESCAPE" => "esc".into(),
        "PRINTSCREEN" => "sysrq".into(),
        "-" => "minus".into(),
        "=" => "equal".into(),
        "[" => "leftbrace".into(),
        "]" => "rightbrace".into(),
        ";" => "semicolon".into(),
        "'" => "apostrophe".into(),
        "," => "comma".into(),
        "." => "dot".into(),
        "/" => "slash".into(),
        "\\" => "backslash".into(),
        "`" => "grave".into(),
        "NUMPADDIVIDE" => "kpslash".into(),
        "NUMPADMULTIPLY" => "kpasterisk".into(),
        "NUMPADMINUS" => "kpminus".into(),
        "NUMPADPLUS" => "kpplus".into(),
        "NUMPADDECIMAL" => "kpdot".into(),
        n if n.starts_with("NUMPAD") => format!("kp{}", &n[6..]),
        other => other.to_ascii_lowercase(),
    };
    parts.push(k);
    Some(parts.join("+"))
}

/// The logs' readouts and the add-on's save as the dial's cards and the bar keys' faces.
struct WowFeed {
    combat: logs::Tail,
    chat: logs::Tail,
    s: logs::Session,
    save: Option<(std::time::SystemTime, saved::Snapshot)>,
    /// The session's experience and copper totals when the save was read (what came after).
    at_load: (u64, u64),
    checked: Instant,
    /// Hub combination → bar slot from the save.
    slots: HashMap<String, saved::Slot>,
    zones: HashMap<String, u32>,
    asked_icons: std::collections::HashSet<u64>,
    /// Hub combination → the strip's cooldown index (bar 1 buttons 1-12 = 0-11, bar 2 1-6 = 12-17).
    slot_index: HashMap<String, usize>,
    /// The add-on's screen strip (live), and whether the mode wants it read.
    strip: std::sync::Arc<std::sync::Mutex<strip::Reading>>,
    want: std::sync::Arc<std::sync::atomic::AtomicBool>,
    track: Track,
}

/// What the strip's readings add up to over the session.
#[derive(Default)]
struct Track {
    last: Option<strip::Live>,
    /// (when, experience gained) and (when, copper gained), for the per-hour rates.
    xp: std::collections::VecDeque<(Instant, u64)>,
    money: std::collections::VecDeque<(Instant, i64)>,
    xp_total: u64,
    money_start: Option<u64>,
    level_up: Option<(u32, Instant)>,
    /// When the current fight began (combat flag).
    fight_since: Option<Instant>,
    /// Longest cooldown seen per strip slot in its current run (the sweep's full length).
    cd_full: [u8; 18],
    written: Option<Instant>,
}

impl WowFeed {
    fn new() -> Self {
        let want = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dir = install().join("Logs");
        let zones = pulpit_render_src()
            .and_then(|d| std::fs::read_to_string(d.join("maps/zones.json")).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self {
            combat: logs::Tail::new(dir.clone(), "WoWCombatLog"),
            chat: logs::Tail::new(dir, "WoWChatLog"),
            s: logs::Session::default(),
            save: None,
            at_load: (0, 0),
            checked: Instant::now() - std::time::Duration::from_secs(60),
            slots: HashMap::new(),
            zones,
            asked_icons: Default::default(),
            slot_index: HashMap::new(),
            strip: strip::start(want.clone()),
            want,
            track: Track::default(),
        }
    }

    /// Re-read the add-on's save when the game rewrote it.
    fn read_save(&mut self) {
        let Some(path) = saved::find(&install()) else { return };
        let Some(m) = saved::modified(&path) else { return };
        if self.save.as_ref().is_some_and(|(t, _)| *t == m) {
            return;
        }
        let Some(snap) = std::fs::read_to_string(&path).ok().and_then(|t| saved::parse(&t)) else { return };
        self.at_load = (self.s.xp_total, self.s.copper_total);
        self.slots = snap
            .bars
            .values()
            .flatten()
            .filter(|sl| !sl.empty)
            .filter_map(|sl| Some((combo(sl.bind.as_deref()?)?, sl.clone())))
            .collect();
        self.slot_index.clear();
        for (prefix, base) in [("MULTIACTIONBAR1BUTTON", 0), ("MULTIACTIONBAR2BUTTON", 12)] {
            for (i, sl) in snap.bars.get(prefix).into_iter().flatten().enumerate().take(if base == 0 { 12 } else { 6 }) {
                if let Some(c) = sl.bind.as_deref().and_then(combo) {
                    self.slot_index.insert(c, base + i);
                }
            }
        }
        // Icons not extracted yet: fetch them from the game files in the background.
        let dir = pulpit_render_src().map(|d| d.join("icons"));
        let missing: Vec<u64> = self
            .slots
            .values()
            .filter_map(saved::Slot::icon_id)
            .filter(|id| !self.asked_icons.contains(id) && dir.as_ref().is_some_and(|d| !d.join(format!("fdid_{id}.png")).exists()))
            .collect();
        if !missing.is_empty() {
            self.asked_icons.extend(missing.iter().copied());
            let script = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Repos/PULPIT/games/wow_forever/tools/extract.py");
            let mut cmd = std::process::Command::new("python3");
            cmd.arg("-I").arg(script);
            for id in &missing {
                cmd.arg("--fdid").arg(id.to_string());
            }
            match cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
                Ok(mut child) => {
                    std::thread::spawn(move || child.wait());
                }
                Err(e) => log::warn!("wow icons: {e}"),
            }
        }
        log::info!("wow: add-on save read ({} bar slots)", self.slots.len());
        self.save = Some((m, snap));
    }

    /// The strip's reading when it is current (the game in front, the add-on drawing).
    fn live(&self) -> Option<strip::Live> {
        let r = self.strip.lock().ok()?;
        r.at.filter(|at| at.elapsed() < std::time::Duration::from_secs(3)).and(r.live.clone())
    }

    /// Fold a new strip reading into the session's totals; write it for the helper panel.
    fn follow(&mut self, live: &strip::Live) {
        let now = Instant::now();
        let t = &mut self.track;
        if let Some(prev) = &t.last {
            let gained = if live.level > prev.level {
                t.level_up = Some((live.level, now));
                prev.xp_max.saturating_sub(prev.xp) as u64 + live.xp as u64
            } else if live.level == prev.level && live.xp > prev.xp {
                (live.xp - prev.xp) as u64
            } else {
                0
            };
            if gained > 0 {
                t.xp_total += gained;
                t.xp.push_back((now, gained));
            }
            let dm = live.money as i64 - prev.money as i64;
            if dm != 0 {
                t.money.push_back((now, dm));
            }
        }
        t.money_start.get_or_insert(live.money);
        t.fight_since = if live.combat { t.fight_since.or(Some(now)) } else { None };
        for (i, c) in live.cooldowns.iter().enumerate() {
            t.cd_full[i] = match c {
                Some(v) if *v > 0 => t.cd_full[i].max(*v),
                _ => 0,
            };
        }
        let window = std::time::Duration::from_secs(30 * 60);
        while t.xp.front().is_some_and(|(at, _)| now.duration_since(*at) > window) {
            t.xp.pop_front();
        }
        while t.money.front().is_some_and(|(at, _)| now.duration_since(*at) > window) {
            t.money.pop_front();
        }
        let changed = t.last.as_ref() != Some(live);
        t.last = Some(live.clone());
        if changed && t.written.is_none_or(|w| w.elapsed().as_millis() >= 500) {
            t.written = Some(now);
            write_live(live);
        }
    }

    fn snap(&self) -> Option<&saved::Snapshot> {
        self.save.as_ref().map(|(_, s)| s)
    }
}

/// The strip's reading for the helper panel (`~/.local/share/pulpit/wow-live.json`).
fn write_live(l: &strip::Live) {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return };
    let path = home.join(".local/share/pulpit/wow-live.json");
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
    let v = serde_json::json!({
        "at": unix, "level": l.level, "xp": l.xp, "xpmax": l.xp_max, "rested": l.rested, "money": l.money,
        "map": l.map, "x": l.pos.map(|p| p.0), "y": l.pos.map(|p| p.1), "combat": l.combat, "dead": l.dead,
        "resting": l.resting, "mounted": l.mounted, "class": l.class, "faction": l.faction,
    });
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

fn rate_per_hour(q: &std::collections::VecDeque<(Instant, f64)>) -> Option<f64> {
    let first = q.front()?.0;
    let secs = first.elapsed().as_secs_f64().max(300.0);
    Some(q.iter().map(|(_, v)| *v).sum::<f64>() * 3600.0 / secs)
}

fn pulpit_render_src() -> Option<PathBuf> {
    std::env::var_os("PULPIT_WOW_SRC")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share/pulpit/wow-src")))
}

fn short(n: f64) -> String {
    match n {
        n if n >= 1e6 => format!("{:.1}M", n / 1e6),
        n if n >= 1e4 => format!("{:.0}K", n / 1e3),
        n if n >= 1e3 => format!("{:.1}K", n / 1e3),
        n => format!("{}", n.round()),
    }
}

/// Copper as the game writes money: "12g 40s", "40s 3c".
fn coins(c: f64) -> String {
    let c = c.round() as u64;
    let (g, s, cu) = (c / 10_000, c / 100 % 100, c % 100);
    match (g, s) {
        (0, 0) => format!("{cu}c"),
        (0, _) => format!("{s}s {cu}c"),
        _ => format!("{g}g {s}s"),
    }
}

impl Feed for WowFeed {
    fn update(&mut self) {
        // Read the strip while the mode is drawing (this runs only then).
        self.want.store(true, std::sync::atomic::Ordering::Relaxed);
        let now = Instant::now();
        for l in self.combat.lines() {
            self.s.combat(&l, now);
        }
        for l in self.chat.lines() {
            self.s.chat(&l, now);
        }
        self.s.tick(now);
        if let Some(live) = self.live() {
            self.follow(&live);
        }
        if self.checked.elapsed().as_secs() >= 2 {
            self.checked = now;
            self.read_save();
        }
    }

    fn fighting(&self) -> bool {
        match self.live() {
            Some(l) => l.combat,
            None => self.s.fighting(Instant::now()),
        }
    }

    fn cards(&self, ring: [u8; 3]) -> Vec<DialCard> {
        let now = Instant::now();
        let snap = self.snap();
        let live = self.live();
        let t = &self.track;
        let card = |eyebrow: &str, big: String| DialCard { eyebrow: eyebrow.into(), big, ring: Some(ring), ..Default::default() };
        let mut cards = Vec::new();

        if self.fighting() {
            let secs = t.fight_since.map_or(0, |at| at.elapsed().as_secs());
            let mut c = card("In Combat", format!("{}:{:02}", secs / 60, secs % 60));
            c.sub = Some(match live.as_ref().and_then(|l| l.target) {
                Some(hp) => format!("TARGET {hp}%"),
                None => "FIGHT TIME".into(),
            });
            if let Some(f) = &self.s.last_fight {
                c.line = Some(format!("LAST {} DPS", short(f.dps() as f64)));
            }
            c.ring = Some(RED);
            c.breathe = true;
            c.progress = live.as_ref().and_then(|l| l.target).map(|hp| hp as f32 / 100.0);
            cards.push(c);
        }

        // The zone, on its own map: the strip's map id first, then the logs, then the save.
        let by_id = |id: u32| self.zones.iter().find(|(_, v)| **v == id).map(|(k, _)| k.clone());
        let zone = live
            .as_ref()
            .and_then(|l| by_id(l.map))
            .or_else(|| self.s.zone.clone())
            .or_else(|| snap.and_then(|s| s.zone.clone()))
            .unwrap_or_else(|| "Azeroth".into());
        let place = self.s.map.clone().filter(|m| *m != zone).or_else(|| snap.and_then(|s| s.subzone.clone()).filter(|z| !z.is_empty() && *z != zone));
        let mut z = card(&zone, place.unwrap_or_else(|| zone.clone()).to_uppercase());
        z.map = live.as_ref().map(|l| l.map).filter(|m| *m != 0).or_else(|| self.zones.get(&zone).copied()).map(|id| format!("zone_{id}"));
        let watched = snap.and_then(|s| s.quests.iter().find(|q| q.watched && !q.done).or(s.quests.iter().find(|q| !q.done)));
        z.sub = self.s.quest.clone().or_else(|| watched.and_then(|q| q.title.clone())).map(|q| q.to_uppercase());
        z.line = watched.and_then(|q| q.objectives.iter().find(|o| !o.done)).and_then(|o| o.text.clone()).map(|t| t.to_uppercase());
        if let Some(l) = &live
            && let Some((x, y)) = l.pos
        {
            z.line = Some(format!("{:.1}, {:.1}{}", x * 100.0, y * 100.0, z.line.map(|o| format!(" · {o}")).unwrap_or_default()));
        }
        z.ring = Some(GOLD);
        cards.push(z);

        // Experience: live from the strip; the save and the logs when the strip is not there.
        let mut x = card("Experience", short(t.xp_total as f64));
        x.ring = Some(XP);
        let rate = rate_per_hour(&t.xp.iter().map(|(a, v)| (*a, *v as f64)).collect()).map(|r| format!("{} XP/H", short(r)));
        match live.as_ref().filter(|l| l.xp_max > 0) {
            Some(l) => {
                let p = l.xp as f32 / l.xp_max as f32;
                x.big = format!("{}%", (p * 100.0).floor());
                x.progress = Some(p);
                x.sub = Some(format!("LEVEL {}{}", l.level, rate.map(|r| format!(" · {r}")).unwrap_or_default()));
                let left = l.xp_max.saturating_sub(l.xp);
                x.line = Some(if l.rested > 0 {
                    format!("{} TO GO · RESTED +{}%", short(left as f64), ((l.rested as f32 / l.xp_max as f32) * 100.0).round())
                } else {
                    format!("{} TO GO", short(left as f64))
                });
            }
            None => match snap.and_then(|s| Some((s.level?, s.xp?, s.xpmax.filter(|m| *m > 0.0)?))) {
                Some((level, xp, max)) => {
                    let p = (xp / max).min(1.0);
                    x.big = format!("{}%", (p * 100.0).floor());
                    x.progress = Some(p as f32);
                    x.sub = Some(format!("LEVEL {level} · AT THE LAST /RELOAD"));
                }
                None => x.sub = Some("THIS SESSION".into()),
            },
        }
        if let Some((lvl, at)) = t.level_up.filter(|(_, at)| at.elapsed().as_secs() < 30) {
            let _ = at;
            x.eyebrow = "Level Up".into();
            x.big = lvl.to_string();
            x.breathe = true;
            x.progress = Some(1.0);
        }
        cards.push(x);

        // Gold.
        let money = live.as_ref().map(|l| l.money).or_else(|| snap.and_then(|s| s.money).map(|m| m as u64));
        let mut g = card("Gold", money.map_or("—".into(), |m| coins(m as f64)));
        let made = match (money, t.money_start) {
            (Some(m), Some(s0)) => m as i64 - s0 as i64,
            _ => 0,
        };
        g.sub = Some(match rate_per_hour(&t.money.iter().map(|(a, v)| (*a, *v as f64)).collect()) {
            Some(r) if made != 0 => format!("{}{} THIS SESSION · {}/H", if made < 0 { "−" } else { "+" }, coins(made.unsigned_abs() as f64), coins(r.max(0.0))),
            _ => "THIS SESSION".into(),
        });
        g.line = self.s.loot.front().map(|(n, c, _)| if *c > 1 { format!("{} ×{c}", n.to_uppercase()) } else { n.to_uppercase() });
        g.ring = Some(GOLD);
        cards.push(g);

        if let Some(f) = &self.s.last_fight {
            let mut c = card("Last Fight", short(f.dps() as f64));
            c.sub = Some(format!("DPS · {}s", f.secs().round()));
            c.line = Some(format!("KILLS {} · DEATHS {} (LOG, MINUTES LATE)", self.s.kills, self.s.deaths));
            cards.push(c);
        }
        let _ = now;
        cards
    }

    fn key(&self, key: &GKey) -> Option<KeyPatch> {
        let Press::Keys(k) = key.press else { return None };
        let slot = self.slots.get(k)?;
        let mut p = KeyPatch { label: slot.name.clone().map(|n| n.to_uppercase()), ..Default::default() };
        if let Some(id) = slot.icon_id() {
            p.logo = Some(format!("wow:fdid_{id}"));
        }
        // The strip's cooldown for this button (seconds left), as a sweep and a number.
        if let (Some(&i), Some(l)) = (self.slot_index.get(k), self.live())
            && let Some(left) = l.cooldowns.get(i).copied().flatten().filter(|v| *v > 0)
        {
            let full = self.track.cd_full[i].max(left).max(1);
            p.cooldown = Some(left as f32 / full as f32);
            p.minor = Some(left.to_string());
        }
        Some(p)
    }
}

pub fn game() -> Game {
    use Press::*;
    let b = Binds::load();
    let key = |label: &'static str, tint: Tint, press| GKey { label, icon: ' ', kind: Kind::Action, press, minor: None, art: None, stat: None, tint: Some(tint), on: None };
    // A game action: its bind, or a dimmed key saying it has none.
    let act = |label: &'static str, action: &str, tint: Tint| match b.key(action) {
        Some(k) => key(label, tint, Keys(k)),
        None => GKey { minor: Some("NO BIND"), ..key(label, TOOL, Show) },
    };
    let bar = |n: u32, label: &'static str| {
        let action = format!("MULTIACTIONBAR1BUTTON{n}");
        act(label, &action, SPELL)
    };
    let travel = |n: u32, label: &'static str| act(label, &format!("MULTIACTIONBAR2BUTTON{n}"), TRAVEL);
    let stat = |label, stat, tint: Tint| GKey { stat: Some(stat), ..key(label, tint, Show) };
    let clip = || key("SAVE CLIP", TOOL, Cmd(REC_SAVE));

    let play = vec![
        act("BAGS", "OPENALLBAGS", WINDOW),
        act("CHARACTER", "TOGGLECHARACTER0", WINDOW),
        act("SPELLBOOK", "TOGGLESPELLBOOK", WINDOW),
        act("TALENTS", "TOGGLETALENTS", WINDOW),
        act("MAP", "TOGGLEWORLDMAP", WINDOW),
        bar(1, "BAR 1"),
        bar(2, "BAR 2"),
        bar(3, "BAR 3"),
        bar(4, "BAR 4"),
        bar(5, "BAR 5"),
        act("QUESTS", "TOGGLEQUESTLOG", WINDOW),
        act("AUTORUN", "TOGGLEAUTORUN", TRAVEL),
        travel(1, "MOUNT"),
        act("SCREENSHOT", "SCREENSHOT", TOOL),
        clip(),
    ];
    let names = ["BAR 1", "BAR 2", "BAR 3", "BAR 4", "BAR 5", "BAR 6", "BAR 7", "BAR 8", "BAR 9", "BAR 10", "BAR 11", "BAR 12"];
    let mut fight: Vec<GKey> = (1..=12).map(|n| bar(n, names[n as usize - 1])).collect();
    fight.extend([act("TARGET", "TARGETNEARESTENEMY", SPELL), act("SHEATHE", "TOGGLESHEATH", TRAVEL), clip()]);
    let travel_page = vec![
        travel(1, "MOUNT"),
        travel(2, "HEARTHSTONE"),
        travel(3, "TRAVEL 3"),
        travel(4, "TRAVEL 4"),
        travel(5, "TRAVEL 5"),
        act("AUTORUN", "TOGGLEAUTORUN", TRAVEL),
        act("SIT / STAND", "SITORSTAND", TRAVEL),
        act("SHEATHE", "TOGGLESHEATH", TRAVEL),
        act("WALK / RUN", "TOGGLERUN", TRAVEL),
        act("FOLLOW", "FOLLOWTARGET", TRAVEL),
        act("MAP", "TOGGLEWORLDMAP", WINDOW),
        act("QUESTS", "TOGGLEQUESTLOG", WINDOW),
        act("BAGS", "OPENALLBAGS", WINDOW),
        act("SCREENSHOT", "SCREENSHOT", TOOL),
        clip(),
    ];
    let menus = vec![
        act("BAGS", "OPENALLBAGS", WINDOW),
        act("CHARACTER", "TOGGLECHARACTER0", WINDOW),
        act("SPELLBOOK", "TOGGLESPELLBOOK", WINDOW),
        act("TALENTS", "TOGGLETALENTS", WINDOW),
        act("PROFESSIONS", "TOGGLEPROFESSIONBOOK", WINDOW),
        act("QUESTS", "TOGGLEQUESTLOG", WINDOW),
        act("MAP", "TOGGLEWORLDMAP", WINDOW),
        act("ACHIEVEMENTS", "TOGGLEACHIEVEMENT", WINDOW),
        act("COLLECTIONS", "TOGGLECOLLECTIONS", WINDOW),
        act("ADVENTURE GUIDE", "TOGGLEENCOUNTERJOURNAL", WINDOW),
        act("GROUP FINDER", "TOGGLEGROUPFINDER", WINDOW),
        act("GUILD", "TOGGLEGUILDTAB", WINDOW),
        act("FRIENDS", "TOGGLESOCIAL", WINDOW),
        act("REPLY", "REPLY", WINDOW),
        act("GAME MENU", "TOGGLEGAMEMENU", TOOL),
    ];
    let capture = vec![
        act("SCREENSHOT", "SCREENSHOT", TOOL),
        act("HIDE UI", "TOGGLEUI", TOOL),
        clip(),
        key("REPLAY", TOOL, Cmd(REC_REPLAY)),
        key("RECORD", TOOL, Cmd(REC_RECORD)),
        stat("TIME", Stat::Time, WINDOW),
        stat("SESSION", Stat::Session, WINDOW),
        stat("", Stat::Date, WINDOW),
        stat("GPU TEMP", Stat::GpuTemp, TOOL),
        stat("VRAM GB", Stat::Vram, TOOL),
        GKey { icon: '\u{fcc9}', ..key("PREVIOUS", TRAVEL, Cmd("playerctl -p playerctld previous")) },
        GKey { stat: Some(Stat::Track), icon: '\u{fa7e}', ..key("", TRAVEL, Cmd("playerctl -p playerctld play-pause")) },
        GKey { icon: '\u{fcca}', ..key("NEXT", TRAVEL, Cmd("playerctl -p playerctld next")) },
        stat("CPU", Stat::Cpu, TOOL),
        stat("GPU LOAD", Stat::GpuLoad, TOOL),
    ];
    // The game's own icons and the bar's hotkey text on every key that has them.
    let art = |label: &str| -> Option<&'static str> {
        Some(match label {
            "BAGS" => "wow:inv_misc_bag_08",
            "CHARACTER" => "wow:inv_chest_plate04",
            "SPELLBOOK" => "wow:inv_misc_book_09",
            "TALENTS" => "wow:inv_misc_book_11",
            "MAP" => "wow:inv_misc_map_01",
            "QUESTS" => "wow:inv_misc_note_01",
            "AUTORUN" | "WALK / RUN" => "wow:ability_rogue_sprint",
            "MOUNT" => "wow:ability_mount_charger",
            "HEARTHSTONE" => "wow:inv_misc_rune_01",
            "SCREENSHOT" => "wow:inv_misc_spyglass_02",
            "SAVE CLIP" => "wow:inv_misc_film_01",
            "REPLAY" => "wow:inv_misc_pocketwatch_01",
            "RECORD" => "wow:trade_engineering",
            "HIDE UI" => "wow:spell_nature_timestop",
            "TARGET" => "wow:spell_holy_fistofjustice",
            "SHEATHE" => "wow:inv_hammer_01",
            "SIT / STAND" => "wow:inv_misc_flower_02",
            "FOLLOW" | "GROUP FINDER" => "wow:inv_misc_groupneedmore",
            "PROFESSIONS" => "wow:trade_blacksmithing",
            "ACHIEVEMENTS" => "wow:inv_misc_tournaments_banner_human",
            "COLLECTIONS" => "wow:inv_misc_toy_07",
            "ADVENTURE GUIDE" => "wow:inv_misc_head_dragon_01",
            "GUILD" => "wow:ability_warrior_battleshout",
            "FRIENDS" => "wow:achievement_character_human_male",
            "REPLY" => "wow:inv_letter_15",
            "GAME MENU" => "wow:inv_misc_gear_01",
            _ => return None,
        })
    };
    let dress = |keys: Vec<GKey>| -> Vec<GKey> {
        keys.into_iter()
            .map(|mut k| {
                if k.art.is_none() {
                    // Bar slots show what the add-on saw on the bar; until then, an empty slot.
                    k.art = art(k.label).or((k.label.starts_with("BAR ") || k.label.starts_with("TRAVEL ")).then_some("wow:empty-slot"));
                }
                if let (Keys(c), None) = (k.press, k.minor) {
                    k.minor = Some(Box::leak(hotkey(c).into_boxed_str()));
                }
                k
            })
            .collect()
    };
    let page = |name, keys| Page { name, keys: dress(keys), favs: false, loadouts: false };
    Game {
        claim_name: "pulpit-wow-forever",
        title: "WOW FOREVER",
        // The Lutris/umu launcher names every window of the Battle.net prefix the same (Diablo IV,
        // WC3, the launcher itself), so the title picks WoW (engine `class@title` pattern).
        class: "steam_app_battlenet@World of Warcraft",
        process: "Wow.exe",
        theme: THEME,
        palette: Palette { action: WINDOW, cheat: TOOL, toggle: TRAVEL, ring: XP },
        intro: Some(IntroStyle::Hearth),
        bridge_dir: None,
        pages: vec![page("PLAY", play), page("FIGHT", fight), page("TRAVEL", travel_page), page("MENUS", menus), page("CAPTURE", capture)],
        combat_page: Some(1),
        favs_file: None,
        loadouts_file: None,
        // The levelling helper on the monitor without the game (games/wow_forever/tools/panel.*).
        panel: Some("~/Repos/PULPIT/scripts/pulpit-exec ~/Repos/PULPIT/games/wow_forever/tools/panel.sh"),
        feed: Some(Box::new(WowFeed::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names() {
        assert_eq!(combo("CTRL-1").as_deref(), Some("ctrl+1"));
        assert_eq!(combo("SHIFT-P").as_deref(), Some("shift+p"));
        assert_eq!(combo("CTRL--").as_deref(), Some("ctrl+minus"));
        assert_eq!(combo("-").as_deref(), Some("minus"));
        assert_eq!(combo("ALT-CTRL-NUMPAD5").as_deref(), Some("alt+ctrl+kp5"));
        assert_eq!(combo("PRINTSCREEN").as_deref(), Some("sysrq"));
        assert_eq!(combo("BUTTON4"), None);
        assert_eq!(combo("SHIFT-MOUSEWHEELUP"), None);
    }

    #[test]
    fn account_binds_over_defaults() {
        let mut m: HashMap<String, String> = DEFAULTS.iter().filter_map(|(a, k)| Some((a.to_string(), combo(k)?))).collect();
        apply(&mut m, "bind CTRL-1 MULTIACTIONBAR1BUTTON1\nbind B TOGGLEBACKPACK\nbind X NONE\n");
        assert_eq!(m.get("MULTIACTIONBAR1BUTTON1").map(String::as_str), Some("ctrl+1"));
        assert!(!m.contains_key("OPENALLBAGS"), "B moved to another action");
        assert!(!m.contains_key("SITORSTAND"), "X cleared");
    }

    #[test]
    fn money() {
        assert_eq!(coins(124_003.0), "12g 40s");
        assert_eq!(coins(4_003.0), "40s 3c");
        assert_eq!(coins(7.0), "7c");
    }

    #[test]
    fn hotkeys() {
        assert_eq!(hotkey("ctrl+1"), "c1");
        assert_eq!(hotkey("shift+p"), "sP");
        assert_eq!(hotkey("numlock"), "NL");
        assert_eq!(hotkey("ctrl+minus"), "c-");
    }
}

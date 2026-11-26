//! WoW Forever's own log files, read as the game writes them: the combat log (`/combatlog`,
//! `Logs/WoWCombatLog-*.txt`) and the chat log (`/chatlog`, `Logs/WoWChatLog.txt`). Nothing
//! here touches the game; these are the files Warcraft Logs reads too.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Without damage to or from the player for this long, the fight is over (the game's own
/// combat flag drops after about five seconds).
const FIGHT_GAP: Duration = Duration::from_secs(6);
/// How far back the per-hour rates look.
const RATE_WINDOW: Duration = Duration::from_secs(30 * 60);

/// The newest file whose name starts with `prefix` in `dir`.
fn newest(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Follows the newest log file with a given name prefix, handing out whole new lines. Starts
/// at the end of a file that already exists (old sessions are not replayed).
pub struct Tail {
    dir: PathBuf,
    prefix: &'static str,
    file: Option<(PathBuf, File, u64)>,
    partial: Vec<u8>,
    checked: Instant,
}

impl Tail {
    pub fn new(dir: PathBuf, prefix: &'static str) -> Self {
        Self { dir, prefix, file: None, partial: Vec::new(), checked: Instant::now() - Duration::from_secs(60) }
    }

    /// New complete lines since the last call.
    pub fn lines(&mut self) -> Vec<String> {
        if self.checked.elapsed() > Duration::from_secs(2) {
            self.checked = Instant::now();
            let found = newest(&self.dir, self.prefix);
            let same = matches!((&found, &self.file), (Some(f), Some((p, _, _))) if f == p);
            if let (Some(path), false) = (found, same) {
                let first = self.file.is_none();
                if let Ok(mut f) = File::open(&path) {
                    // A file found at start is old news; one that appears later is read whole.
                    let pos = if first { f.seek(SeekFrom::End(0)).unwrap_or(0) } else { 0 };
                    self.file = Some((path, f, pos));
                    self.partial.clear();
                }
            }
        }
        let Some((_, f, pos)) = &mut self.file else { return Vec::new() };
        let len = f.metadata().map_or(0, |m| m.len());
        if len < *pos {
            // Truncated (the chat log restarts): read from the top.
            *pos = 0;
            self.partial.clear();
        }
        if len == *pos || f.seek(SeekFrom::Start(*pos)).is_err() {
            return Vec::new();
        }
        let mut buf = Vec::new();
        let _ = f.take(4 << 20).read_to_end(&mut buf);
        *pos += buf.len() as u64;
        self.partial.extend_from_slice(&buf);
        let Some(end) = self.partial.iter().rposition(|&b| b == b'\n') else { return Vec::new() };
        let done: Vec<u8> = self.partial.drain(..=end).collect();
        String::from_utf8_lossy(&done).lines().map(|l| l.trim_end_matches('\r').to_owned()).filter(|l| !l.is_empty()).collect()
    }
}

/// The text after the timestamp: combat log lines are `M/D/YYYY HH:MM:SS.mmm-TZ  EVENT,...`,
/// chat log lines `M/D HH:MM:SS.mmm  text`; both put two spaces before the payload.
fn payload(line: &str) -> &str {
    line.split_once("  ").map_or(line, |(_, rest)| rest)
}

/// Splits a combat log event on commas, keeping quoted text and bracketed lists whole.
pub fn fields(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut quoted, mut start) = (0i32, false, 0);
    for (i, c) in s.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '[' | '(' if !quoted => depth += 1,
            ']' | ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                out.push(s[start..i].trim_matches('"'));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim_matches('"'));
    out
}

fn flags(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
}

/// COMBATLOG_OBJECT_AFFILIATION_MINE.
const MINE: u32 = 0x1;
const TYPE_PLAYER: u32 = 0x400;
const TYPE_PET: u32 = 0x1000;
const TYPE_GUARDIAN: u32 = 0x2000;

/// A cast the player made (for cooldowns on the bar keys).
#[derive(Debug, Clone, PartialEq)]
pub struct Cast {
    pub spell: u32,
    pub name: String,
    pub at: Instant,
}

/// One fight: from the first hit to [`FIGHT_GAP`] after the last.
#[derive(Debug, Clone, Default)]
pub struct Fight {
    pub start: Option<Instant>,
    pub last: Option<Instant>,
    pub damage: u64,
    pub healing: u64,
    pub taken: u64,
    pub kills: u32,
}

impl Fight {
    pub fn secs(&self) -> f32 {
        match (self.start, self.last) {
            (Some(s), Some(l)) => (l - s).as_secs_f32().max(1.0),
            _ => 1.0,
        }
    }
    pub fn dps(&self) -> f32 {
        self.damage as f32 / self.secs()
    }
}

/// What the logs tell about the session so far.
#[derive(Debug, Default)]
pub struct Session {
    pub me: Option<String>,
    pub name: Option<String>,
    pub advanced: bool,
    pub zone: Option<String>,
    pub map: Option<String>,
    pub fight: Fight,
    pub last_fight: Option<Fight>,
    pub deaths: u32,
    pub kills: u32,
    pub casts: VecDeque<Cast>,
    pub level: Option<u32>,
    pub level_up: Option<Instant>,
    /// (when, amount) of experience and copper earned, for the per-hour rates.
    pub xp: VecDeque<(Instant, u64)>,
    pub money: VecDeque<(Instant, u64)>,
    pub xp_total: u64,
    pub rested_total: u64,
    pub copper_total: u64,
    /// The last items received, newest first: (name, count, quality colour code if any).
    pub loot: VecDeque<(String, u32, Option<String>)>,
    pub quests_done: u32,
    pub quest: Option<String>,
    pub combat_lines: u64,
    pub chat_lines: u64,
}

impl Session {
    pub fn fighting(&self, now: Instant) -> bool {
        self.fight.last.is_some_and(|l| now.duration_since(l) < FIGHT_GAP)
    }

    /// Ends a fight that has gone quiet.
    pub fn tick(&mut self, now: Instant) {
        if self.fight.last.is_some() && !self.fighting(now) {
            self.last_fight = Some(std::mem::take(&mut self.fight));
        }
        let keep = |q: &mut VecDeque<(Instant, u64)>| {
            while q.front().is_some_and(|(t, _)| now.duration_since(*t) > RATE_WINDOW) {
                q.pop_front();
            }
        };
        keep(&mut self.xp);
        keep(&mut self.money);
    }

    fn hit(&mut self, now: Instant) {
        if self.fight.start.is_none() {
            self.fight.start = Some(now);
        }
        self.fight.last = Some(now);
    }

    /// One combat log line.
    pub fn combat(&mut self, line: &str, now: Instant) {
        self.combat_lines += 1;
        let f = fields(payload(line));
        let Some(&event) = f.first() else { return };
        match event {
            "COMBAT_LOG_VERSION" => self.advanced = f.iter().position(|x| *x == "ADVANCED_LOG_ENABLED").and_then(|i| f.get(i + 1)) == Some(&"1"),
            "ZONE_CHANGE" => {
                self.zone = f.get(2).map(|s| s.to_string());
                self.map = None;
            }
            "MAP_CHANGE" => self.map = f.get(2).map(|s| s.to_string()),
            _ if f.len() >= 9 => self.unit_event(event, &f, now),
            _ => {}
        }
    }

    fn unit_event(&mut self, event: &str, f: &[&str], now: Instant) {
        let (src, src_name, src_flags) = (f[1], f[2], flags(f[3]));
        let dst = f[5];
        let mine = |fl: u32| fl & MINE != 0;
        if self.me.is_none() && mine(src_flags) && src_flags & TYPE_PLAYER != 0 {
            self.me = Some(src.to_owned());
            self.name = Some(src_name.split('-').next().unwrap_or(src_name).to_owned());
        }
        let is_me = |g: &str| self.me.as_deref() == Some(g);
        let from_me = is_me(src) || (mine(src_flags) && src_flags & (TYPE_PET | TYPE_GUARDIAN) != 0);
        let to_me = is_me(dst);
        // Where the amount sits: after the 8 unit fields, the spell's 3 (not for swings and
        // environment), and the 17 advanced fields when the line has them. Forever writes them
        // even with ADVANCED_LOG_ENABLED,0 in the header, so look at the line: the advanced
        // block starts with a unit GUID ("Player-…") or sixteen zeros.
        let spell_fields = if event.starts_with("SWING") { 0 } else if event.starts_with("ENVIRONMENTAL") { 1 } else { 3 };
        let first = 9 + spell_fields;
        let advanced = self.advanced || f.get(first).is_some_and(|v| v.contains('-') || (v.len() == 16 && v.bytes().all(|b| b == b'0')));
        // The block's length differs between clients (Forever's has two power fields more than
        // retail's 17); it ends with position X, Y (decimals), map, facing and level.
        let amount_at = if advanced {
            (first + 2..f.len()).find(|&i| f[i].contains('.')).map_or(first + 17, |x| x + 5)
        } else {
            first
        };
        let amount = || f.get(amount_at).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        match event {
            e if e.ends_with("_DAMAGE") && !e.ends_with("_DURABILITY_DAMAGE") => {
                if from_me {
                    self.fight.damage += amount();
                    self.hit(now);
                } else if to_me {
                    self.fight.taken += amount();
                    self.hit(now);
                }
            }
            e if e.ends_with("_MISSED") && (from_me || to_me) => self.hit(now),
            e if e.ends_with("_HEAL") && !e.ends_with("_PERIODIC_HEAL") || e == "SPELL_PERIODIC_HEAL" => {
                if from_me {
                    // Amount, base amount, overhealing: count what landed.
                    let over = f.get(amount_at + 2).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                    self.fight.healing += amount().saturating_sub(over);
                }
            }
            "SPELL_CAST_SUCCESS" if is_me(src) => {
                let spell = f.get(9).and_then(|v| v.parse().ok()).unwrap_or(0);
                self.casts.push_front(Cast { spell, name: f.get(10).unwrap_or(&"").to_string(), at: now });
                self.casts.truncate(64);
            }
            "PARTY_KILL" if from_me => {
                self.fight.kills += 1;
                self.kills += 1;
            }
            "UNIT_DIED" if to_me => self.deaths += 1,
            _ => {}
        }
    }

    /// One chat log line (enUS client).
    pub fn chat(&mut self, line: &str, now: Instant) {
        self.chat_lines += 1;
        let text = strip_codes(payload(line));
        let t = text.as_str();
        if let Some(n) = xp_gained(t) {
            self.xp_total += n;
            self.xp.push_back((now, n));
            if let Some(r) = between(t, "(+", " exp Rested bonus").and_then(|v| v.replace(',', "").parse::<u64>().ok()) {
                self.rested_total += r;
            }
        } else if let Some(c) = money(t) {
            self.copper_total += c;
            self.money.push_back((now, c));
        } else if let Some(rest) = t.strip_prefix("You receive loot: ").or_else(|| t.strip_prefix("You receive item: ")).or_else(|| t.strip_prefix("You create: ")) {
            let (name, count) = item(rest);
            self.loot.push_front((name, count, None));
            self.loot.truncate(20);
        } else if let Some(n) = between(t, "you have reached level ", "!").and_then(|v| v.parse().ok()) {
            self.level = Some(n);
            self.level_up = Some(now);
        } else if let Some(q) = t.strip_prefix("Quest accepted: ") {
            self.quest = Some(q.trim().to_owned());
        } else if let Some(q) = t.strip_suffix(" completed.") {
            self.quests_done += 1;
            if self.quest.as_deref() == Some(q) {
                self.quest = None;
            }
        } else if let Some(z) = between(t, "General - ", "]") {
            // Joining a zone's General channel names the zone.
            if t.starts_with("Joined Channel:") || t.starts_with("Changed Channel:") {
                self.zone = Some(z.to_owned());
            }
        }
    }
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

/// Experience in a line: "X dies, you gain 45 experience." / "Experience gained: 450." /
/// "You gain 120 experience."
fn xp_gained(t: &str) -> Option<u64> {
    let n = between(t, "you gain ", " experience")
        .or_else(|| between(t, "You gain ", " experience"))
        .or_else(|| between(t, "Experience gained: ", "."))?;
    n.replace(',', "").trim().parse().ok()
}

/// Copper in a money line: "You loot 1 Silver, 12 Copper", "Your share of the loot is 3
/// Copper.", "Received 2 Gold, 5 Silver.".
fn money(t: &str) -> Option<u64> {
    let rest = ["You loot ", "Your share of the loot is ", "Received "].iter().find_map(|p| t.strip_prefix(p))?;
    let mut total = 0;
    let mut any = false;
    let words: Vec<&str> = rest.split([' ', ',', '.']).filter(|w| !w.is_empty()).collect();
    for w in words.windows(2) {
        let Ok(n) = w[0].parse::<u64>() else { continue };
        let mult = match w[1] {
            "Gold" => 10_000,
            "Silver" => 100,
            "Copper" => 1,
            _ => continue,
        };
        total += n * mult;
        any = true;
    }
    any.then_some(total)
}

/// "[Linen Cloth]x2." → ("Linen Cloth", 2).
fn item(s: &str) -> (String, u32) {
    let name = between(s, "[", "]").unwrap_or(s.trim_end_matches('.')).to_owned();
    let count = s.rsplit_once("]x").and_then(|(_, n)| n.trim_end_matches('.').parse().ok()).unwrap_or(1);
    (name, count)
}

/// Removes the client's colour and link codes: `|cff9d9d9d`, `|r`, `|Hitem:...|h`, `|h`, `|T...|t`.
pub fn strip_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if c != '|' {
            out.push(c);
            continue;
        }
        match it.peek().map(|(_, c)| *c) {
            Some('c') => {
                // |cAARRGGBB
                for _ in 0..9 {
                    it.next();
                }
            }
            Some('r') | Some('h') => {
                it.next();
            }
            Some('H') | Some('T') | Some('A') => {
                // Skip up to the closing |h / |t / |a.
                let close = match it.peek().map(|(_, c)| *c) {
                    Some('H') => "|h",
                    Some('T') => "|t",
                    _ => "|a",
                };
                match s[i + 1..].find(close) {
                    Some(j) => {
                        let end = i + 1 + j + 2;
                        while it.peek().is_some_and(|(k, _)| *k < end) {
                            it.next();
                        }
                    }
                    None => break,
                }
            }
            Some('|') => {
                it.next();
                out.push('|');
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "Player-1234-0ABCDEF0";

    fn cl(s: &mut Session, ev: &str, now: Instant) {
        s.combat(&format!("11/24/2026 20:00:00.000-1  {ev}"), now);
    }

    #[test]
    fn splits_fields() {
        let f = fields(r#"SPELL_DAMAGE,Player-1-A,"John-Concussion",0x511,0x0,Creature-0-1,"Kobold, Vermin",0x10a48,0x0,35395,"Crusader Strike",0x1"#);
        assert_eq!(f[2], "John-Concussion");
        assert_eq!(f[6], "Kobold, Vermin");
        assert_eq!(f[10], "Crusader Strike");
        assert_eq!(fields("COMBATANT_INFO,a,[(1,2),(3,4)],b").len(), 4);
    }

    #[test]
    fn fight_damage_and_casts() {
        let t0 = Instant::now();
        let mut s = Session::default();
        cl(&mut s, "COMBAT_LOG_VERSION,22,ADVANCED_LOG_ENABLED,0,BUILD_VERSION,1.60.1,PROJECT_ID,1", t0);
        cl(&mut s, "ZONE_CHANGE,0,\"Elwynn Forest\",0", t0);
        cl(&mut s, &format!("SPELL_CAST_SUCCESS,{ME},\"John-Concussion\",0x511,0x0,Creature-0-1,\"Kobold Vermin\",0x10a48,0x0,35395,\"Crusader Strike\",0x1"), t0);
        cl(&mut s, &format!("SPELL_DAMAGE,{ME},\"John-Concussion\",0x511,0x0,Creature-0-1,\"Kobold Vermin\",0x10a48,0x0,35395,\"Crusader Strike\",0x1,120,0,1,0,0,0,nil,nil,nil"), t0);
        cl(&mut s, &format!("SWING_DAMAGE,Creature-0-1,\"Kobold Vermin\",0x10a48,0x0,{ME},\"John-Concussion\",0x511,0x0,15,0,1,0,0,0,nil,nil,nil"), t0 + Duration::from_secs(2));
        cl(&mut s, &format!("PARTY_KILL,{ME},\"John-Concussion\",0x511,0x0,Creature-0-1,\"Kobold Vermin\",0x10a48,0x0"), t0 + Duration::from_secs(4));
        assert_eq!(s.zone.as_deref(), Some("Elwynn Forest"));
        assert_eq!(s.name.as_deref(), Some("John"));
        assert_eq!(s.fight.damage, 120);
        assert_eq!(s.fight.taken, 15);
        assert_eq!(s.casts[0].spell, 35395);
        assert!(s.fighting(t0 + Duration::from_secs(5)));
        s.tick(t0 + Duration::from_secs(10));
        assert!(!s.fighting(t0 + Duration::from_secs(10)));
        let last = s.last_fight.as_ref().unwrap();
        assert_eq!(last.kills, 1);
        assert!((last.dps() - 60.0).abs() < 0.1);
    }

    #[test]
    fn advanced_amount() {
        let mut s = Session { advanced: true, me: Some(ME.into()), ..Default::default() };
        let adv = "Player-1234-0ABCDEF0,0000000000000000,100,100,0,0,0,0,-1,0,0,0,-9000.0,100.0,0,1.5,12";
        cl(&mut s, &format!("SPELL_DAMAGE,{ME},\"John-Concussion\",0x511,0x0,Creature-0-1,\"Wolf\",0x10a48,0x0,20271,\"Judgment\",0x2,{adv},88,70,2,0,0,0,nil,nil,nil"), Instant::now());
        assert_eq!(s.fight.damage, 88);
    }

    #[test]
    fn forever_writes_advanced_fields_anyway() {
        // Real lines from WoW Forever 1.60.1 (header says ADVANCED_LOG_ENABLED,0).
        let me = "Player-6940-07E319E8";
        let mut s = Session { me: Some(me.into()), ..Default::default() };
        let now = Instant::now();
        s.combat("11/24/2026 04:29:37.4111  COMBAT_LOG_VERSION,22,ADVANCED_LOG_ENABLED,0,BUILD_VERSION,1.60.1,PROJECT_ID,18", now);
        s.combat(r#"11/24/2026 04:30:09.5811  SWING_DAMAGE,Player-6940-07E319E8,"Jovce-Gankerson-PvP-EU",0x518,0x80000000,Creature-0-7090-0-88982-1919-00000503A1,"Samuel Fipps",0xa28,0x80000000,Player-6940-07E319E8,0000000000000000,122,122,55,0,241,0,0,0,0,110,110,0,1986.38,1382.30,1420,4.1905,0,10,12,-1,1,0,0,0,nil,nil,nil"#, now);
        s.combat(r#"11/24/2026 04:30:09.5811  SWING_DAMAGE_LANDED,Creature-0-7090-0-88982-1919-00000503A1,"Samuel Fipps",0xa28,0x80000000,Player-6940-07E319E8,"Jovce-Gankerson-PvP-EU",0x518,0x80000000,Player-6940-07E319E8,0000000000000000,118,122,55,0,241,0,0,0,0,110,110,0,1986.38,1382.30,1420,4.1905,0,4,5,-1,1,0,0,0,nil"#, now);
        assert!(!s.advanced);
        assert_eq!(s.fight.damage, 10, "the swing's amount, not a GUID");
        assert_eq!(s.fight.taken, 0, "_LANDED repeats the swing; not counted again");
    }

    #[test]
    fn chat_lines() {
        let t0 = Instant::now();
        let mut s = Session::default();
        let c = |s: &mut Session, text: &str| s.chat(&format!("11/24 20:00:00.000  {text}"), t0);
        c(&mut s, "Kobold Vermin dies, you gain 45 experience (+22 exp Rested bonus).");
        c(&mut s, "Experience gained: 1,250.");
        c(&mut s, "You loot 1 Silver, 12 Copper");
        c(&mut s, "Your share of the loot is 3 Copper.");
        c(&mut s, "You receive loot: |cffffffff|Hitem:2589::::::::5:::::::::|h[Linen Cloth]|h|rx2.");
        c(&mut s, "Congratulations, you have reached level 6!");
        c(&mut s, "Quest accepted: Wolves Across the Border");
        c(&mut s, "Wolves Across the Border completed.");
        c(&mut s, "Changed Channel: [1. General - Westfall]");
        assert_eq!(s.xp_total, 1295);
        assert_eq!(s.rested_total, 22);
        assert_eq!(s.copper_total, 115);
        assert_eq!(s.loot[0], ("Linen Cloth".into(), 2, None));
        assert_eq!(s.level, Some(6));
        assert_eq!(s.quests_done, 1);
        assert_eq!(s.quest, None);
        assert_eq!(s.zone.as_deref(), Some("Westfall"));
    }

    #[test]
    fn strips_codes() {
        assert_eq!(strip_codes("|cff1eff00|Hitem:1:2|h[Sword]|h|r and |TInterface\\x:0|t ok || pipe"), "[Sword] and  ok | pipe");
    }
}

#[cfg(test)]
mod real {
    /// `WOW_COMBATLOG=path cargo test -p pulpit real_log -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_log() {
        let path = std::env::var("WOW_COMBATLOG").expect("WOW_COMBATLOG");
        let mut s = super::Session::default();
        let now = std::time::Instant::now();
        for l in std::fs::read_to_string(path).unwrap().lines() {
            s.combat(l, now);
        }
        println!("me {:?} name {:?} zone {:?} map {:?} damage {} taken {} healing {} kills {} casts {:?}", s.me, s.name, s.zone, s.map, s.fight.damage, s.fight.taken, s.fight.healing, s.kills, s.casts.iter().take(5).map(|c| &c.name).collect::<Vec<_>>());
    }
}

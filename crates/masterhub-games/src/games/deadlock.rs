//! Deadlock (Steam 1422450, Proton): keys only. An online game with anti-cheat, so every hub
//! key is one of the game's own binds pressed once, nothing chained or timed beyond a plain
//! hold, and nothing reads the game. The binds come from the game's own file
//! (`citadelkeys_personal.lst` in the Steam cloud folder) at start, so the panel follows the
//! user's rebinding (restart pulpit after one); an action whose bind is missing, unset or on the mouse falls back to
//! the game's default. Game-side notes: games/deadlock.

use std::collections::HashMap;

use masterhub_games_render::{DeadlockFrame, DeadlockPress, DeadlockRing, DeadlockStyle, IntroStyle, Theme};

use crate::ctx::{Clip, Ctx, steam_class};
use crate::game::{GKey, Game, Kind, Page, Palette, Press, Stat};

/// The look from the game's own material (see pulpit-render's face/deadlock.rs).
const THEME: Theme = Theme::Deadlock(DeadlockStyle { frame: DeadlockFrame::Card, press: DeadlockPress::Soul, ring: DeadlockRing::Souls });

/// (card field, light tone) pairs from the game's style sheets.
type Tint = ([u8; 3], [u8; 3]);
/// Screens, chat, menu: `colorCommonDarkGray` / `offWhite`.
const PLAIN: Tint = ([0x3A, 0x37, 0x31], [0xFF, 0xEF, 0xD7]);
/// Souls and buying: `deadlockGreenDarker` / `deadlockGreenLight`.
const SOULS: Tint = ([0x2B, 0x70, 0x47], [0xA8, 0xFF, 0xC9]);
/// Ability points: `enhancedColorDark` / `enhancedColorLight`.
const POINTS: Tint = ([0x0E, 0x6F, 0x86], [0x94, 0xF4, 0xFF]);
/// The three item kinds: `weaponBGColor3`, `spiritDarkColor`, `fortitudeDarkColor` and their lights.
const WEAPON: Tint = ([0x80, 0x4C, 0x00], [0xFF, 0xD1, 0x8D]);
const SPIRIT: Tint = ([0x61, 0x34, 0x84], [0xE7, 0xC8, 0xFF]);
const VITALITY: Tint = ([0x4C, 0x71, 0x13], [0xD3, 0xFC, 0x96]);
/// Replay controls: `buildsDarkColor` / `buildsLightColor`; the two teams.
const WATCH: Tint = ([0x36, 0x5B, 0x92], [0xBD, 0xD6, 0xFF]);
const AMBER: Tint = ([0x93, 0x76, 0x00], [0xFB, 0xDC, 0xA0]);
const SAPPHIRE: Tint = ([0x41, 0x56, 0xA0], [0xBD, 0xCB, 0xFF]);
/// Our own tools: `defaultBGColor1` / grey text.
const TOOL: Tint = ([0x26, 0x26, 0x26], [0xC9, 0xC2, 0xB4]);

/// The game's binds: action name → the hub's key combination ("alt+1").
struct Binds(HashMap<String, String>);

impl Binds {
    fn load() -> Self {
        // One Steam account folder per user of this machine: take the newest binds file.
        let newest = crate::steam::newest_userdata(std::path::Path::new("1422450/remote/cfg/citadelkeys_personal.lst"));
        let text = newest.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        Self(parse(&text))
    }

    /// The combination for `action`, or `default` when the file has none we can press.
    fn key(&self, action: &str, default: &'static str) -> &'static str {
        match self.0.get(action) {
            Some(k) => Box::leak(k.clone().into_boxed_str()),
            None => default,
        }
    }
}

/// `"Action" { "Key" "1" "Modifier" "ALT" }` blocks → action → combination. Mouse binds and
/// unset ones are left out.
fn parse(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut action = String::new();
    let (mut key, mut modifier) = (None, None);
    for line in text.lines() {
        let words: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        match (words.as_slice(), line.trim()) {
            ([name], _) => {
                action = (*name).to_owned();
                (key, modifier) = (None, None);
            }
            (["Key", v], _) => key = key_name(v),
            (["Modifier", v], _) => modifier = key_name(v),
            (_, "}") => {
                if let Some(k) = key.take() {
                    out.insert(std::mem::take(&mut action), modifier.take().map_or(k.clone(), |m| format!("{m}+{k}")));
                }
            }
            _ => {}
        }
    }
    out
}

/// A Deadlock key name as the hub's virtual keyboard calls it; `None` for the mouse and "NONE".
fn key_name(v: &str) -> Option<String> {
    Some(
        match v {
            "NONE" | "" => return None,
            m if m.starts_with("MOUSE") || m.starts_with("MWHEEL") => return None,
            "ESCAPE" => "esc",
            "UPARROW" => "up",
            "DOWNARROW" => "down",
            "LEFTARROW" => "left",
            "RIGHTARROW" => "right",
            "=" => "equal",
            "-" => "minus",
            "," => "comma",
            "." => "dot",
            "/" => "slash",
            ";" => "semicolon",
            "'" => "apostrophe",
            "[" => "leftbrace",
            "]" => "rightbrace",
            "\\" => "backslash",
            "`" => "grave",
            other => return Some(other.to_ascii_lowercase()),
        }
        .to_owned(),
    )
}

/// A Font Awesome stand-in for each game icon, drawn until `setup` has cut the real ones.
fn glyph(art: &str) -> char {
    let c = match art.strip_prefix("dl-").unwrap_or(art) {
        "cart" => 0xf07a,
        "quickbuy" => 0xf0e7,
        "team" => 0xf0c0,
        "graph" => 0xf201,
        "chat" => 0xf075,
        "globe" => 0xf0ac,
        "gear" => 0xf013,
        "keyboard" => 0xf11c,
        "book" => 0xf02d,
        "star" => 0xf005,
        "grid" => 0xf00a,
        "x" => 0xf00d,
        "check" => 0xf00c,
        "up" => 0xf077,
        "down" => 0xf078,
        "pause" => 0xf04c,
        "play" => 0xf04b,
        "eye" => 0xf06e,
        "eye-off" => 0xf070,
        "watch" => 0xf03d,
        "cursor" => 0xf245,
        "player" => 0xf007,
        "download" => 0xf019,
        "refresh" => 0xf021,
        "meter" => 0xf625,
        "ap" | "ult" => 0xf102,
        "gun" => 0xf05b,
        "spirit" => 0xe2ca,
        "survival" => 0xf004,
        "binoculars" => 0xf1e5,
        "faster" => 0xf04e,
        "slower" => 0xf04a,
        "left" => 0xf053,
        "right" => 0xf054,
        "list" => 0xf03a,
        "sound-off" => 0xf6a9,
        "mic-off" => 0xf131,
        "logo" | "soul" => 0xf3a5,
        _ => return ' ',
    };
    char::from_u32(c).unwrap_or(' ')
}

pub fn game(ctx: &Ctx) -> Game {
    use Press::*;
    // The look draws with the game's own icons and face, from the user's install (see setup).
    masterhub_games_render::set_mask_dir("dl-", ctx.data.join("masks"));
    if let Some(install) = &ctx.install {
        masterhub_games_render::set_deadlock_font(install.join("game/citadel/panorama/fonts/retaildemo-bold.otf"));
    }
    let b = Binds::load();
    let key = |label, art, tint: Tint, press| GKey { label, icon: glyph(art), kind: Kind::Action, press, minor: None, art: Some(art), stat: None, tint: Some(tint), on: None };
    let hold = |combo: &'static str, ms: u32| -> &'static str { Box::leak(format!("{combo}@{ms}").into_boxed_str()) };
    let stat = |label, stat, tint: Tint| GKey { stat: Some(stat), ..key(label, "", tint, Show) };
    // The display page: nothing to press for the game (Street Brawl needs no keys), so the keys
    // show the time, the machine and the music; the bottom row drives the player and clips.
    let live = vec![
        key("DEADLOCK", "dl-logo", PLAIN, Show),
        stat("TIME", Stat::Time, PLAIN),
        stat("SESSION", Stat::Session, SOULS),
        stat("", Stat::Date, PLAIN),
        stat("CPU", Stat::Cpu, TOOL),
        stat("GPU TEMP", Stat::GpuTemp, WEAPON),
        stat("GPU LOAD", Stat::GpuLoad, WEAPON),
        stat("VRAM GB", Stat::Vram, WEAPON),
        key("SAVE CLIP", "dl-download", TOOL, ctx.clip(Clip::Save)),
        key("GAME FPS", "dl-meter", TOOL, Keys("insert")),
        key("PREVIOUS", "dl-left", SPIRIT, Keys("previoussong")),
        GKey { stat: Some(Stat::Track), ..key("", "dl-play", SPIRIT, Keys("playpause")) },
        key("NEXT", "dl-right", SPIRIT, Keys("nextsong")),
        key("MUTE ALL", "dl-sound-off", SPIRIT, Keys("mute")),
        key("RECORD", "dl-watch", TOOL, ctx.clip(Clip::Record)),
    ];
    // Console commands the game runs for a key (games/deadlock/cfg/pulpit.cfg binds them to the
    // navigation keys): client-side settings, one command per key.
    let extra = vec![
        key("HIDE HUD", "dl-eye-off", WATCH, Keys("home")),
        key("PHOTO MODE", "dl-binoculars", WATCH, Keys("end")),
        key("GAME FPS", "dl-meter", TOOL, Keys("insert")),
        key("PING", "dl-graph", TOOL, Keys("delete")),
        key("REPLAY HUD", "dl-list", WATCH, Keys("scrolllock")),
        key("MUTE GAME", "dl-sound-off", SPIRIT, Keys("pageup")),
        key("VOICE", "dl-mic-off", SPIRIT, Keys("pagedown")),
        key("PING REPLY", "dl-chat", SOULS, Keys("pause")),
        key("TEAM CHAT", "dl-chat", PLAIN, Keys(b.key("ChatTeam", "enter"))),
        key("SCORES", "dl-team", PLAIN, Keys(hold(b.key("Scoreboard", "tab"), 1500))),
        key("MENU", "dl-logo", PLAIN, Keys(b.key("OpenMainMenu", "esc"))),
        key("CONSOLE", "dl-keyboard", TOOL, Keys(b.key("DevConsole", "f7"))),
        key("SAVE CLIP", "dl-download", TOOL, ctx.clip(Clip::Save)),
        key("REPLAY", "dl-refresh", TOOL, ctx.clip(Clip::Replay)),
        key("RECORD", "dl-watch", TOOL, ctx.clip(Clip::Record)),
    ];
    let tools = || -> [GKey; 4] {
        [
            key("SAVE CLIP", "dl-download", TOOL, ctx.clip(Clip::Save)),
            key("REPLAY", "dl-refresh", TOOL, ctx.clip(Clip::Replay)),
            key("RECORD", "dl-watch", TOOL, ctx.clip(Clip::Record)),
            key("GAME FPS", "dl-meter", TOOL, Keys("insert")),
        ]
    };
    // Colours as the game uses them: green = souls and buying, cyan = ability points, the three
    // item kinds in their own, blue = watching, grey = our tools.
    let mut game_page = vec![
        key("SHOP", "dl-cart", SOULS, Keys(b.key("OpenHeroSheet", "b"))),
        key("QUICK BUY", "dl-quickbuy", SOULS, Keys(b.key("PurchaseQuickbuy", "g"))),
        key("SCORES", "dl-team", PLAIN, Keys(hold(b.key("Scoreboard", "tab"), 1500))),
        key("TEAM CHAT", "dl-chat", PLAIN, Keys(b.key("ChatTeam", "enter"))),
        key("ALL CHAT", "dl-globe", PLAIN, Keys(b.key("Chat", "shift+enter"))),
        key("UPGRADE 1", "dl-ap", POINTS, Keys(b.key("AbilityUpgrade1", "alt+1"))),
        key("UPGRADE 2", "dl-ap", POINTS, Keys(b.key("AbilityUpgrade2", "alt+2"))),
        key("UPGRADE 3", "dl-ap", POINTS, Keys(b.key("AbilityUpgrade3", "alt+3"))),
        key("UPGRADE 4", "dl-ult", POINTS, Keys(b.key("AbilityUpgrade4", "alt+4"))),
        key("MENU", "dl-logo", PLAIN, Keys(b.key("OpenMainMenu", "esc"))),
        key("CONSOLE", "dl-keyboard", TOOL, Keys(b.key("DevConsole", "f7"))),
    ];
    game_page.extend(tools());
    let shop = vec![
        key("BUILD", "dl-book", PLAIN, Keys(b.key("ShopTabBuild", "1"))),
        key("PICKS", "dl-star", PLAIN, Keys(b.key("ShopTabRecommended", "2"))),
        key("ALL ITEMS", "dl-grid", PLAIN, Keys(b.key("ShopTabAllItems", "3"))),
        key("SEARCH", "dl-list", PLAIN, Keys(b.key("ShopFocusSearch", "backspace"))),
        key("CLOSE", "dl-x", PLAIN, Keys(b.key("OpenHeroSheet", "b"))),
        key("WEAPON", "dl-gun", WEAPON, Keys(b.key("ShopTabWeapon", "4"))),
        key("SPIRIT", "dl-spirit", SPIRIT, Keys(b.key("ShopTabSpirit", "5"))),
        key("VITALITY", "dl-survival", VITALITY, Keys(b.key("ShopTabVitality", "6"))),
        key("TIER UP", "dl-up", PLAIN, Keys(b.key("ShopTierUp", "up"))),
        key("TIER DOWN", "dl-down", PLAIN, Keys(b.key("ShopTierDown", "down"))),
        key("BUY FOUND", "dl-check", SOULS, Keys(b.key("ShopPurchaseSearchMatch", "enter"))),
        key("QUICK BUY", "dl-quickbuy", SOULS, Keys(b.key("PurchaseQuickbuy", "g"))),
        key("SCORES", "dl-team", PLAIN, Keys(hold(b.key("Scoreboard", "tab"), 1500))),
        key("SAVE CLIP", "dl-download", TOOL, ctx.clip(Clip::Save)),
        key("GAME FPS", "dl-meter", TOOL, Keys("insert")),
    ];
    let mut replay = vec![
        key("PAUSE", "dl-pause", WATCH, Keys(b.key("ReplayTogglePause", "p"))),
        key("SLOWER", "dl-slower", WATCH, Keys(b.key("ReplayDecreaseSpeed", "minus"))),
        key("FASTER", "dl-faster", WATCH, Keys(b.key("ReplayIncreaseSpeed", "equal"))),
        key("PREVIOUS", "dl-left", WATCH, Keys(b.key("ReplayPrevTarget", "alt+space"))),
        key("NEXT", "dl-right", WATCH, Keys(b.key("ReplayNextTarget", "space"))),
        key("CAMERA", "dl-binoculars", WATCH, Keys(b.key("ToggleReplayCamera", "c"))),
        key("CURSOR", "dl-cursor", WATCH, Keys(b.key("SpectatorToggleFreeCursor", "j"))),
        key("SEE BOTH", "dl-eye", PLAIN, Keys(b.key("SpectatorFOWTeamBoth", "f1"))),
        key("SEE TARGET", "dl-player", PLAIN, Keys(b.key("SpectatorFOWTeamTarget", "f2"))),
        key("AMBER", "dl-eye", AMBER, Keys(b.key("SpectatorFOWTeamAmber", "f3"))),
        key("SAPPHIRE", "dl-eye", SAPPHIRE, Keys(b.key("SpectatorFOWTeamSapphire", "f4"))),
    ];
    replay.extend(tools());
    let page = |name, keys| Page { name, keys, favs: false, loadouts: false };
    Game {
        claim_name: "pulpit-deadlock",
        title: "DEADLOCK",
        class: steam_class(1422450, "deadlock.exe"),
        process: "deadlock.exe",
        theme: THEME,
        palette: Palette { action: PLAIN, cheat: TOOL, toggle: SOULS, ring: [0x70, 0xF8, 0xC1] },
        intro: Some(IntroStyle::Emblem),
        bridge_dir: None,
        pages: vec![page("LIVE", live), page("EXTRA", extra), page("MATCH", game_page), page("SHOP", shop), page("REPLAY", replay)],
        combat_page: None,
        favs_file: None,
        loadouts_file: None,
        feed: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_binds() {
        let text = "\"KeyBindings\"\n{\n\t\"Name\"\t\t\"DEFAULT\"\n\t\"Keys\"\n\t{\n\t\t\"Ability1\"\n\t\t{\n\t\t\t\"Key\"\t\t\"1\"\n\t\t}\n\t\t\"AbilityUpgrade1\"\n\t\t{\n\t\t\t\"Key\"\t\t\"1\"\n\t\t\t\"Modifier\"\t\t\"ALT\"\n\t\t\t\"Key2\"\t\t\"NONE\"\n\t\t}\n\t\t\"Ping\"\n\t\t{\n\t\t\t\"Key\"\t\t\"MOUSE3\"\n\t\t}\n\t\t\"ReplayIncreaseSpeed\"\n\t\t{\n\t\t\t\"Key\"\t\t\"=\"\n\t\t}\n\t\t\"OpenMainMenu\"\n\t\t{\n\t\t\t\"Key\"\t\t\"ESCAPE\"\n\t\t}\n\t}\n}\n";
        let b = parse(text);
        assert_eq!(b.get("Ability1").map(String::as_str), Some("1"));
        assert_eq!(b.get("AbilityUpgrade1").map(String::as_str), Some("alt+1"));
        assert_eq!(b.get("ReplayIncreaseSpeed").map(String::as_str), Some("equal"));
        assert_eq!(b.get("OpenMainMenu").map(String::as_str), Some("esc"));
        assert!(!b.contains_key("Ping"));
        assert!(!b.contains_key("Keys") && !b.contains_key("KeyBindings"));
    }
}

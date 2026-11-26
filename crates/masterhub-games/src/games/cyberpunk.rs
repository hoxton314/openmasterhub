//! Cyberpunk 2077: the game's binds, cheats through the CET bridge mod, the spawn panel's
//! favourites and loadout sets. Game-side files: games/cyberpunk in the repo.

use std::path::PathBuf;

use pulpit_render::{CyberStyle, Frame, IconFx, IntroStyle, Numbers, PressFx, Ring, Theme};

use crate::game::{GKey, Game, Kind, Page, Palette, Press, k};

/// The assembled game look (picked on the proposals board).
const THEME: Theme = Theme::Cyber(CyberStyle {
    frame: Frame::Cut,
    icon: IconFx::Glow,
    press: PressFx::FlashGlitch,
    ring: Ring::Segmented,
    numbers: Numbers::Mono,
    scanlines: true,
});

const YELLOW: [u8; 3] = [0xFC, 0xEE, 0x0A];
const CYAN: [u8; 3] = [0x02, 0xD7, 0xF2];
const RED: [u8; 3] = [0xFF, 0x3B, 0x3B];

/// Cyberpunk 2077 (GOG via Heroic, umu GAMEID → steam_app_1091500). Binds are the game's
/// defaults; 3rd person and cinematic are CET binds of their mods (F13, F14).
pub fn game() -> Game {
    use Kind::*;
    use Press::*;
    let ico = |c: u32| char::from_u32(c).unwrap_or(' ');
    let game = vec![
        k("SAVE", ico(0xf55e), Action, Keys("f5")),
        k("MAP", ico(0xf8ed), Action, Keys("m")),
        k("JOURNAL", ico(0xf286), Action, Keys("j")),
        k("INVENTORY", ico(0xf1e7), Action, Keys("i")),
        k("CHARACTER", ico(0xfe78), Action, Keys("p")),
        k("PHONE", ico(0xf943), Action, Keys("t@500")),
        k("CAR", ico(0xf366), Action, Keys("v")),
        k("3RD PERSON", ico(0xfeab), Action, Keys("leftshift+v")),
        k("CINEMATIC", ico(0xf5e9), Action, Keys("leftctrl+leftshift+m")),
        k("TIME SKIP", ico(0xfd7f), Action, Keys("o@800")),
        k("SAVE CLIP", ico(0xfaf3), Action, Cmd("~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec save")),
        k("MANGOHUD", ico(0xfd25), Action, Keys("rightshift+f12@150")),
        k("WAYPOINT", ico(0xf8ea), Cheat, Bridge("waypoint")),
        k("HEAL", ico(0xf794), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf8c1), Cheat, Bridge("ammo")),
    ];
    let cheats = vec![
        k("+ EDDIES", ico(0xf48c), Cheat, Amount),
        k("+ LEVEL", ico(0xf890), Cheat, Bridge("level")),
        k("+ CRED", ico(0xfcc1), Cheat, Bridge("cred")),
        k("+ PERK", ico(0xfc5d), Cheat, Bridge("perk")),
        k("+ ATTRIBUTE", ico(0xf2dc), Cheat, Bridge("attr")),
        k("+ RELIC", ico(0xf6e2), Cheat, Bridge("relic")),
        k("HEAL", ico(0xf794), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf8c1), Cheat, Bridge("ammo")),
        k("RAM", ico(0xf915), Cheat, Bridge("ram")),
        k("WANTED", ico(0xfbf9), Cheat, Bridge("wanted")),
        GKey { on: Some(|s| s.god), ..k("GOD MODE", ico(0xfbc2), Toggle, Bridge("god")) },
        GKey { on: Some(|s| s.stamina_inf), ..k("STAMINA", ico(0xfb5a), Toggle, Bridge("stamina")) },
        k("WEATHER", ico(0xf46e), Cheat, Bridge("weather")),
        k("WAYPOINT", ico(0xf8ea), Cheat, Bridge("waypoint")),
        k("PANEL", ico(0xfb82), Action, Cmd("~/Repos/PULPIT/scripts/pulpit-exec ~/Repos/PULPIT/games/cyberpunk/tools/panel.sh --show")),
    ];
    let combat = vec![
        k("HACK 1", ico(0xf2dc), Action, Keys("1")),
        k("HACK 2", ico(0xf2dc), Action, Keys("2")),
        k("HACK 3", ico(0xf2dc), Action, Keys("3")),
        k("HACK 4", ico(0xf2dc), Action, Keys("4")),
        k("HACK 5", ico(0xf2dc), Action, Keys("5")),
        k("HEAL ITEM", ico(0xf794), Action, Keys("x")),
        k("CYBERWARE", ico(0xf26a), Action, Keys("e@300")),
        k("HEAL", ico(0xf794), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf8c1), Cheat, Bridge("ammo")),
        k("RAM", ico(0xf915), Cheat, Bridge("ram")),
        GKey { on: Some(|s| s.god), ..k("GOD MODE", ico(0xfbc2), Toggle, Bridge("god")) },
        GKey { on: Some(|s| s.stamina_inf), ..k("STAMINA", ico(0xfb5a), Toggle, Bridge("stamina")) },
        k("WANTED", ico(0xfbf9), Cheat, Bridge("wanted")),
        k("SAVE CLIP", ico(0xfaf3), Action, Cmd("~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec save")),
        k("WAYPOINT", ico(0xf8ea), Cheat, Bridge("waypoint")),
    ];
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    Game {
        claim_name: "pulpit-cyberpunk",
        title: "CYBERPUNK",
        class: "steam_app_1091500",
        process: "Cyberpunk2077.exe",
        theme: THEME,
        palette: Palette {
            action: ([0x05, 0x1A, 0x20], CYAN),
            cheat: ([0x22, 0x20, 0x03], YELLOW),
            toggle: ([0x2A, 0x08, 0x0C], RED),
            ring: CYAN,
        },
        intro: Some(IntroStyle::Glitch),
        bridge_dir: Some(home.join("Games/Heroic/Cyberpunk 2077/bin/x64/plugins/cyber_engine_tweaks/mods/pulpit_bridge")),
        pages: vec![
            Page { name: "GAME", keys: game, favs: false, loadouts: false },
            Page { name: "CHEATS", keys: cheats, favs: false, loadouts: false },
            Page { name: "COMBAT", keys: combat, favs: false, loadouts: false },
            Page { name: "FAVOURITES", keys: Vec::new(), favs: true, loadouts: false },
            Page { name: "LOADOUTS", keys: Vec::new(), favs: false, loadouts: true },
        ],
        combat_page: Some(2),
        favs_file: Some(home.join(".local/share/pulpit/cyberpunk-favs.json")),
        loadouts_file: Some(home.join(".local/share/pulpit/cyberpunk-loadouts.json")),
        panel: Some("~/Repos/PULPIT/scripts/pulpit-exec ~/Repos/PULPIT/games/cyberpunk/tools/panel.sh"),
        feed: None,
    }
}

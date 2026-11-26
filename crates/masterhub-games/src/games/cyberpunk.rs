//! Cyberpunk 2077: the game's binds, cheats through the CET bridge mod, the spawn panel's
//! favourites and loadout sets. Game-side files: games/cyberpunk in the repo.

use masterhub_games_render::{CyberStyle, Frame, IconFx, IntroStyle, Numbers, PressFx, Ring, Theme};

use crate::ctx::{Clip, Ctx, steam_class};
use crate::game::{GKey, Game, Kind, Page, Palette, Press, k};

/// The bridge mod's folder in the game's install (`masterhub-games setup cyberpunk` puts it there).
pub const BRIDGE: &str = "bin/x64/plugins/cyber_engine_tweaks/mods/pulpit_bridge";

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
pub fn game(ctx: &Ctx) -> Game {
    use Kind::*;
    use Press::*;
    let ico = |c: u32| char::from_u32(c).unwrap_or(' ');
    let game = vec![
        k("SAVE", ico(0xf0c7), Action, Keys("f5")),
        k("MAP", ico(0xf279), Action, Keys("m")),
        k("JOURNAL", ico(0xf02d), Action, Keys("j")),
        k("INVENTORY", ico(0xf0f2), Action, Keys("i")),
        k("CHARACTER", ico(0xf007), Action, Keys("p")),
        k("PHONE", ico(0xf3cf), Action, Keys("t@500")),
        k("CAR", ico(0xf1b9), Action, Keys("v")),
        k("3RD PERSON", ico(0xf729), Action, Keys("leftshift+v")),
        k("CINEMATIC", ico(0xf070), Action, Keys("leftctrl+leftshift+m")),
        k("TIME SKIP", ico(0xf04e), Action, Keys("o@800")),
        k("SAVE CLIP", ico(0xf111), Action, ctx.clip(Clip::Save)),
        k("MANGOHUD", ico(0xf625), Action, Keys("rightshift+f12@150")),
        k("WAYPOINT", ico(0xf3c5), Cheat, Bridge("waypoint")),
        k("HEAL", ico(0xf004), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf05b), Cheat, Bridge("ammo")),
    ];
    let cheats = vec![
        k("+ EDDIES", ico(0xf51e), Cheat, Amount),
        k("+ LEVEL", ico(0xf102), Cheat, Bridge("level")),
        k("+ CRED", ico(0xf005), Cheat, Bridge("cred")),
        k("+ PERK", ico(0xe2ca), Cheat, Bridge("perk")),
        k("+ ATTRIBUTE", ico(0xf5dc), Cheat, Bridge("attr")),
        k("+ RELIC", ico(0xf3a5), Cheat, Bridge("relic")),
        k("HEAL", ico(0xf004), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf05b), Cheat, Bridge("ammo")),
        k("RAM", ico(0xf538), Cheat, Bridge("ram")),
        k("WANTED", ico(0xe4f8), Cheat, Bridge("wanted")),
        GKey { on: Some(|s| s.god), ..k("GOD MODE", ico(0xf132), Toggle, Bridge("god")) },
        GKey { on: Some(|s| s.stamina_inf), ..k("STAMINA", ico(0xf70c), Toggle, Bridge("stamina")) },
        k("WEATHER", ico(0xf6c4), Cheat, Bridge("weather")),
        k("WAYPOINT", ico(0xf3c5), Cheat, Bridge("waypoint")),
        k("SAVE", ico(0xf0c7), Action, Keys("f5")),
    ];
    let combat = vec![
        k("HACK 1", ico(0xf5dc), Action, Keys("1")),
        k("HACK 2", ico(0xf5dc), Action, Keys("2")),
        k("HACK 3", ico(0xf5dc), Action, Keys("3")),
        k("HACK 4", ico(0xf5dc), Action, Keys("4")),
        k("HACK 5", ico(0xf5dc), Action, Keys("5")),
        k("HEAL ITEM", ico(0xf004), Action, Keys("x")),
        k("CYBERWARE", ico(0xf0e7), Action, Keys("e@300")),
        k("HEAL", ico(0xf004), Cheat, Bridge("heal")),
        k("AMMO", ico(0xf05b), Cheat, Bridge("ammo")),
        k("RAM", ico(0xf538), Cheat, Bridge("ram")),
        GKey { on: Some(|s| s.god), ..k("GOD MODE", ico(0xf132), Toggle, Bridge("god")) },
        GKey { on: Some(|s| s.stamina_inf), ..k("STAMINA", ico(0xf70c), Toggle, Bridge("stamina")) },
        k("WANTED", ico(0xe4f8), Cheat, Bridge("wanted")),
        k("SAVE CLIP", ico(0xf111), Action, ctx.clip(Clip::Save)),
        k("WAYPOINT", ico(0xf3c5), Cheat, Bridge("waypoint")),
    ];
    Game {
        claim_name: "pulpit-cyberpunk",
        title: "CYBERPUNK",
        // Steam and GOG under Proton/umu alike (umu's GAMEID is the Steam id).
        class: steam_class(1091500, "Cyberpunk2077.exe"),
        process: "Cyberpunk2077.exe",
        theme: THEME,
        palette: Palette {
            action: ([0x05, 0x1A, 0x20], CYAN),
            cheat: ([0x22, 0x20, 0x03], YELLOW),
            toggle: ([0x2A, 0x08, 0x0C], RED),
            ring: CYAN,
        },
        intro: Some(IntroStyle::Glitch),
        bridge_dir: ctx.install.as_ref().map(|p| p.join(BRIDGE)),
        pages: vec![
            Page { name: "GAME", keys: game, favs: false, loadouts: false },
            Page { name: "CHEATS", keys: cheats, favs: false, loadouts: false },
            Page { name: "COMBAT", keys: combat, favs: false, loadouts: false },
            Page { name: "FAVOURITES", keys: Vec::new(), favs: true, loadouts: false },
            Page { name: "LOADOUTS", keys: Vec::new(), favs: false, loadouts: true },
        ],
        combat_page: Some(2),
        favs_file: Some(ctx.data.join("favs.json")),
        loadouts_file: Some(ctx.data.join("loadouts.json")),
        feed: None,
    }
}

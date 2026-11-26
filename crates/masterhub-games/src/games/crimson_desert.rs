//! Crimson Desert (Steam 3321460, Proton): keys only, no mods. The binds are the game's
//! "Default" keyboard preset, read from the user's Input screen on 2026-11-23 (game 2.05).
//! On a controller the menu shortcuts (map, inventory, skills, journal, photo mode) are
//! unassigned, so these keys are the short way to them. Game-side files: games/crimson_desert.

use pulpit_render::{CrimsonFont, CrimsonFrame, CrimsonIcons, CrimsonPress, CrimsonRing, CrimsonStyle, IntroStyle, Theme};

use crate::game::{Game, Kind, Page, Palette, Press, ka};

/// The look after the game's menus: the parts the user picked on the proposals board on 2026-11-23
/// (docs/boards/crimson-look.html, https://claude.ai/artifact/5ZPcqPEPt5nmuwtd8bCEEQ).
const THEME: Theme = Theme::Crimson(CrimsonStyle {
    frame: CrimsonFrame::Plaque,
    font: CrimsonFont::Fell,
    icons: CrimsonIcons::Game,
    press: CrimsonPress::Ember,
    ring: CrimsonRing::Studs,
});

const GOLD: [u8; 3] = [0xC8, 0x96, 0x2E];
const CRIMSON: [u8; 3] = [0xC2, 0x38, 0x2C];
const BONE: [u8; 3] = [0xC9, 0xBF, 0xA8];

const MAP: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/Repos/PULPIT/games/crimson_desert/tools/map.sh";
const REC_SAVE: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec save";
const REC_REPLAY: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec replay";
const REC_RECORD: &str = "~/Repos/PULPIT/scripts/pulpit-exec ~/.local/bin/kreatura-rec record";

pub fn game() -> Game {
    use Kind::*;
    use Press::*;
    let ico = |c: u32| char::from_u32(c).unwrap_or(' ');
    // Gold = the game's menus, crimson = actions in the world, bone = our own tools.
    let keys = vec![
        ka("MAP", ico(0xf8ed), "cd-compass|treasure-map", Action, Keys("m")),
        ka("INVENTORY", ico(0xf1e7), "cd-backpack|knapsack", Action, Keys("i")),
        ka("JOURNAL", ico(0xfb8a), "cd-book|scroll-unfurled", Action, Keys("j")),
        ka("SKILLS", ico(0xfc0b), "cd-skill|star-swirl", Action, Keys("k")),
        ka("PHOTO", ico(0xf344), "cd-photo|photo-camera", Action, Keys("p")),
        ka("MOUNT", ico(0xf7b1), "cd-horseshoes|horse-head", Toggle, Keys("1")),
        ka("WEAPON", ico(0xfd08), "cd-sword|crossed-swords", Toggle, Keys("2")),
        ka("FOOD", ico(0xf59a), "cd-tab-pot|chicken-leg", Toggle, Keys("3")),
        ka("ELEMENT", ico(0xf689), "cd-flame|flame", Toggle, Keys("4")),
        ka("MENU", ico(0xf916), "cd-gear|hamburger-menu", Action, Keys("esc")),
        ka("WEB MAP", ico(0xf275), "cd-star|compass", Cheat, Cmd(MAP)),
        ka("SAVE CLIP", ico(0xfaf3), "film-strip", Cheat, Cmd(REC_SAVE)),
        ka("REPLAY", ico(0xf7c1), "backward-time", Cheat, Cmd(REC_REPLAY)),
        ka("RECORD", ico(0xfe96), "video-camera", Cheat, Cmd(REC_RECORD)),
        ka("FPS HUD", ico(0xfd25), "speedometer", Cheat, Keys("rightshift+f12@150")),
    ];
    Game {
        claim_name: "pulpit-crimson-desert",
        title: "CRIMSON DESERT",
        class: "steam_app_3321460",
        process: "CrimsonDesert.exe",
        theme: THEME,
        palette: Palette {
            action: ([0x2A, 0x20, 0x10], GOLD),
            cheat: ([0x22, 0x21, 0x1E], BONE),
            toggle: ([0x30, 0x10, 0x0E], CRIMSON),
            ring: GOLD,
        },
        intro: Some(IntroStyle::Ember),
        bridge_dir: None,
        pages: vec![Page { name: "GAME", keys, favs: false, loadouts: false }],
        combat_page: None,
        favs_file: None,
        loadouts_file: None,
        panel: None,
        feed: None,
    }
}

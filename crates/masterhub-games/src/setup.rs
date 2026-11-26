//! `masterhub-games setup <game> [--path DIR]`: find the game's install, put its game-side
//! files in place (CET mod, console cfg, add-on), extract what the look draws from the user's
//! own copy of the game into our data folder, and switch the game on in `games.toml`.
//!
//! Nothing from a game ships with OpenMasterHUB: icons and fonts are read from the install
//! on this machine, for this machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::{Config, GameConfig};
use crate::ctx::data_dir;
use crate::games::{self, Info};

const CET_INIT: &str = include_str!("../games/cyberpunk/pulpit_bridge/init.lua");
const CET_LOGIC: &str = include_str!("../games/cyberpunk/pulpit_bridge/logic.lua");
const DEADLOCK_CFG: &str = include_str!("../games/deadlock/cfg/pulpit.cfg");
const WOW_LUA: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT.lua");
const WOW_TOC: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT.toc");
const WOW_TOC_FOREVER: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT_Forever.toc");

/// Set up `name` and switch it on; `path` overrides the install found through Steam.
pub fn setup(cfg_path: &Path, name: &str, path: Option<PathBuf>, extract: bool) -> Result<()> {
    let info = games::by_name(name).with_context(|| format!("unknown game {name:?}"))?;
    let mut cfg = Config::load(cfg_path)?;
    let install = match path {
        Some(p) => Some(p.canonicalize().with_context(|| format!("{} does not exist", p.display()))?),
        None => games::install(info, &cfg),
    };
    let install = match (install, info.name) {
        (Some(p), _) => p,
        // Keys-only games work without knowing where they are.
        (None, "crimson_desert") => PathBuf::new(),
        (None, _) => bail!("{} was not found; pass its install folder with --path", info.title),
    };
    if !install.as_os_str().is_empty() {
        println!("{}: {}", info.title, install.display());
    }
    match info.name {
        "cyberpunk" => install_cet(&install)?,
        "deadlock" => {
            install_deadlock_cfg(&install)?;
            if extract {
                extract_deadlock(&install, &data_dir(info.name).join("masks"))?;
            }
        }
        "wow_forever" => {
            install_wow_addon(&install)?;
            if extract {
                println!("  game icons and fonts: extraction from Battle.net's CASC storage is not built in yet;");
                println!("  the keys use their stand-in look (labels, Cinzel and IM Fell) meanwhile");
            }
        }
        _ => {}
    }
    let entry = cfg.games.entry(info.name.to_owned()).or_insert_with(GameConfig::default);
    entry.enabled = true;
    if !install.as_os_str().is_empty() && Some(&install) != info.steam.and_then(crate::steam::find_app).as_ref() {
        entry.path = Some(install);
    }
    cfg.save(cfg_path)?;
    println!("{} is on ({}). Restart masterhub-games to pick it up.", info.title, cfg_path.display());
    Ok(())
}

/// Switch a game off (its files in the game folder stay).
pub fn disable(cfg_path: &Path, name: &str) -> Result<()> {
    let mut cfg = Config::load(cfg_path)?;
    games::check([&name.to_owned()])?;
    if let Some(g) = cfg.games.get_mut(name) {
        g.enabled = false;
    }
    cfg.save(cfg_path)
}

/// What this build has and what is set up, for `masterhub-games list`.
pub fn list(cfg_path: &Path) -> Result<()> {
    let cfg = Config::load(cfg_path)?;
    for info in games::ALL {
        let on = cfg.games.get(info.name).is_some_and(|g| g.enabled);
        let install = games::install(info, &cfg);
        println!(
            "{:<15} {:<4} {}{}",
            info.name,
            if on { "on" } else { "off" },
            install.as_ref().map_or("not found".into(), |p| p.display().to_string()),
            if assets_ready(info) { "" } else { "  (game assets not extracted)" }
        );
    }
    Ok(())
}

/// Whether the look's assets from the user's install are in place (or the game needs none).
fn assets_ready(info: &Info) -> bool {
    match info.name {
        "deadlock" => data_dir(info.name).join("masks/dl-logo.png").exists(),
        "wow_forever" => data_dir(info.name).join("src/fonts").is_dir(),
        _ => true,
    }
}

/// Copy the CET bridge into `<game>/bin/x64/plugins/cyber_engine_tweaks/mods/pulpit_bridge`.
fn install_cet(install: &Path) -> Result<()> {
    let cet = install.join("bin/x64/plugins/cyber_engine_tweaks");
    if !cet.is_dir() {
        println!("  Cyber Engine Tweaks is not installed: game keys work, the cheat keys need CET");
        println!("  (https://github.com/maximegmd/CyberEngineTweaks); run setup again after installing it");
        return Ok(());
    }
    let dir = install.join(crate::games::cyberpunk_bridge());
    std::fs::create_dir_all(dir.join("eval"))?;
    std::fs::write(dir.join("init.lua"), CET_INIT)?;
    std::fs::write(dir.join("logic.lua"), CET_LOGIC)?;
    println!("  bridge mod: {}", dir.display());
    Ok(())
}

/// Put `pulpit.cfg` (console commands on the navigation keys) into the game's cfg folder and
/// make `autoexec.cfg` run it.
fn install_deadlock_cfg(install: &Path) -> Result<()> {
    let cfg = install.join("game/citadel/cfg");
    if !cfg.is_dir() {
        bail!("no Deadlock cfg folder at {}", cfg.display());
    }
    std::fs::write(cfg.join("pulpit.cfg"), DEADLOCK_CFG)?;
    let auto = cfg.join("autoexec.cfg");
    let text = std::fs::read_to_string(&auto).unwrap_or_default();
    if !text.lines().any(|l| l.trim_start().starts_with("exec pulpit")) {
        let sep = if text.is_empty() || text.ends_with('\n') { "" } else { "\n" };
        std::fs::write(&auto, format!("{text}{sep}exec pulpit\n"))?;
    }
    println!("  console binds: {}", cfg.join("pulpit.cfg").display());
    Ok(())
}

/// Copy the add-on into `<install>/Interface/AddOns/PULPIT`.
fn install_wow_addon(install: &Path) -> Result<()> {
    let dir = install.join("Interface/AddOns/PULPIT");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("PULPIT.lua"), WOW_LUA)?;
    std::fs::write(dir.join("PULPIT.toc"), WOW_TOC)?;
    std::fs::write(dir.join("PULPIT_Forever.toc"), WOW_TOC_FOREVER)?;
    println!("  add-on: {} (switch it on in the game's AddOns list)", dir.display());
    Ok(())
}

/// The Deadlock icons the look uses: mask name → vector icon in `panorama/images`.
const DEADLOCK_ICONS: &[(&str, &str)] = &[
    ("cart", "icons/icon_cart"), ("quickbuy", "icons/icon_quickbuy_queue"), ("team", "icons/icon_team"),
    ("graph", "icons/icon_graph"), ("chat", "hud/ping/ping_icon_generic"), ("globe", "icons/icon_globe"),
    ("gear", "icons/icon_gear"), ("logo", "icons/deadlock_logo"), ("keyboard", "icons/keyboard"),
    ("book", "icons/icon_book"), ("star", "icons/icon_star"), ("grid", "icons/hero_grid"), ("x", "icons/icon_x"),
    ("check", "icons/icon_checkmark"), ("up", "icons/carat_up"), ("down", "icons/carat_down"),
    ("pause", "icons/icon_pause"), ("play", "icons/icon_play"), ("eye", "icons/icon_eye"), ("watch", "icons/icon_watch"),
    ("cursor", "icons/icon_cursor"), ("player", "icons/icon_player"), ("download", "icons/icon_download"),
    ("refresh", "icons/icon_refresh"), ("meter", "icons/icon_dps_meter"), ("ap", "hud/ap_icon"),
    ("soul", "hud/icons/icon_souls"), ("gun", "hud/icons/powerup_gun"), ("spirit", "hud/icons/powerup_spirit"),
    ("survival", "hud/icons/powerup_survival"), ("binoculars", "main_menu/icons/menu_watch"),
    ("faster", "profile/double_up"), ("slower", "profile/double_down"), ("left", "glyphs/arrow_left"),
    ("right", "glyphs/arrow_right"), ("logo-outer", "main_menu/deadlock_logo_outer"),
    ("logo-inner", "main_menu/deadlock_logo_inner"), ("list", "icons/icon_combat_log"), ("sound", "icons/icon_sound_on"),
    ("sound-off", "icons/icon_sound_off"), ("eye-off", "icons/icon_eye_disabled"), ("mic-off", "icons/icon_mic_muted"),
    ("ult", "hud/icons/icon_ultimate"),
];

/// Cut the icons out of the game's `pak01` into 128 px grey masks (the picture's alpha).
fn extract_deadlock(install: &Path, out: &Path) -> Result<()> {
    let pak = crate::vpk::Vpk::open(&install.join("game/citadel/pak01_dir.vpk"))?;
    std::fs::create_dir_all(out)?;
    let mut done = 0;
    for (name, src) in DEADLOCK_ICONS {
        let r = pak.read(&format!("panorama/images/{src}.vsvg_c")).and_then(|data| svg_mask(&data, 116, 128));
        match r {
            Ok(mask) => {
                mask.save(out.join(format!("dl-{name}.png")))?;
                done += 1;
            }
            Err(e) => println!("  {name}: {e:#}"),
        }
    }
    println!("  icons: {done} of {} from the game files into {}", DEADLOCK_ICONS.len(), out.display());
    Ok(())
}

/// The SVG inside a compiled `.vsvg_c`, rasterised to fit `inner` px, centred on a `size`
/// px square: its alpha as a grey mask.
fn svg_mask(data: &[u8], inner: u32, size: u32) -> Result<image::GrayImage> {
    let find = |pat: &[u8]| data.windows(pat.len()).position(|w| w == pat);
    let a = find(b"<svg").context("no SVG inside")?;
    let b = data.windows(6).rposition(|w| w == b"</svg>").context("no SVG end")? + 6;
    let tree = resvg::usvg::Tree::from_data(&data[a..b], &resvg::usvg::Options::default())?;
    let s = tree.size();
    let k = inner as f32 / s.width().max(s.height());
    let (w, h) = ((s.width() * k).round() as u32, (s.height() * k).round() as u32);
    let mut pix = resvg::tiny_skia::Pixmap::new(size, size).context("pixmap")?;
    let t = resvg::tiny_skia::Transform::from_row(k, 0.0, 0.0, k, (size - w) as f32 / 2.0, (size - h) as f32 / 2.0);
    resvg::render(&tree, t, &mut pix.as_mut());
    Ok(image::GrayImage::from_fn(size, size, |x, y| image::Luma([pix.pixel(x, y).map_or(0, |p| p.alpha())])))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rasterises_the_svg_inside_a_compiled_file() {
        let mut data = b"\x00\x01junk header".to_vec();
        data.extend(br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect x="0" y="0" width="10" height="5"/></svg>"#);
        data.extend(b"\x00trailer");
        let m = super::svg_mask(&data, 116, 128).unwrap();
        assert_eq!(m.dimensions(), (128, 128));
        assert_eq!(m.get_pixel(64, 30).0[0], 255);
        assert_eq!(m.get_pixel(64, 100).0[0], 0);
    }
}

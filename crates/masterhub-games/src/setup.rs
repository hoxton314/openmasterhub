//! `masterhub-games setup <game> [--path DIR]`: find the game's install, put its game-side
//! files in place (CET mod, console cfg, add-on), extract what the look draws from the user's
//! own copy of the game into our data folder, and switch the game on in `games.toml`.
//!
//! Nothing from a game ships with OpenMasterHUB: icons and fonts are read from the install
//! on this machine, for this machine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::config::{Config, GameConfig};
use crate::ctx::data_dir;
use crate::games::{self, Info};

const CET_INIT: &str = include_str!("../games/cyberpunk/pulpit_bridge/init.lua");
const CET_LOGIC: &str = include_str!("../games/cyberpunk/pulpit_bridge/logic.lua");
const DEADLOCK_CFG: &str = include_str!("../games/deadlock/cfg/pulpit.cfg");
const WOW_LUA: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT.lua");
const WOW_TOC: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT.toc");
const WOW_TOC_FOREVER: &str = include_str!("../games/wow_forever/addon/PULPIT/PULPIT_Forever.toc");

/// One thing `setup` does, shown to the user for approval before it runs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    /// Copy the Cyber Engine Tweaks bridge mod into this folder.
    CetBridge { path: PathBuf },
    /// Put `pulpit.cfg` into the game's cfg folder and an `exec pulpit` line into autoexec.cfg.
    DeadlockCfg { path: PathBuf },
    /// Cut the look's icons out of the game's archive into our data folder.
    DeadlockIcons { from: PathBuf, to: PathBuf },
    /// Copy the add-on into the game's AddOns folder.
    WowAddon { path: PathBuf },
    /// Switch the game on in games.toml.
    Enable,
}

impl Step {
    /// What it does, in one sentence.
    pub fn text(&self) -> String {
        match self {
            Step::CetBridge { path } => format!("Install the bridge mod into {}", path.display()),
            Step::DeadlockCfg { path } => format!("Add pulpit.cfg and an `exec pulpit` line to autoexec.cfg in {}", path.display()),
            Step::DeadlockIcons { from, to } => format!("Extract the game's icons from {} into {}", from.display(), to.display()),
            Step::WowAddon { path } => format!("Install the PULPIT add-on into {}", path.display()),
            Step::Enable => "Turn the game mode on".into(),
        }
    }
}

/// What setting up a game would do, from its install folder (given, configured or found).
#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub game: &'static str,
    pub title: &'static str,
    /// The install folder the steps use; `None` when it is unknown (and needed).
    pub path: Option<PathBuf>,
    /// The folder was found through Steam rather than given or configured.
    pub detected: bool,
    pub steps: Vec<Step>,
    /// Things the user should know (a missing mod loader, a part not built in yet).
    pub notes: Vec<String>,
    /// Why it can't be set up from this folder; the steps are empty then.
    pub error: Option<String>,
}

/// What `setup` would do for `info` with `path` (or the configured / Steam folder).
pub fn plan(info: &'static Info, cfg: &Config, path: Option<PathBuf>, extract: bool) -> Plan {
    let detected = path.is_none() && cfg.games.get(info.name).and_then(|g| g.path.as_ref()).is_none();
    let path = path.or_else(|| games::install(info, cfg));
    let mut p = Plan { game: info.name, title: info.title, path: path.clone(), detected, steps: Vec::new(), notes: Vec::new(), error: None };
    let need = |p: &mut Plan, what: &str| -> Option<PathBuf> {
        match &path {
            Some(dir) if dir.is_dir() => Some(dir.clone()),
            Some(dir) => {
                p.error = Some(format!("{} is not a folder", dir.display()));
                None
            }
            None => {
                let how = if info.steam.is_some() { "was not found through Steam" } else { "is not in its usual place" };
                p.error = Some(format!("{} {how}: choose its {what}", info.title));
                None
            }
        }
    };
    match info.name {
        "cyberpunk" => {
            let Some(dir) = need(&mut p, "install folder (the one with bin/x64)") else { return p };
            let cet = dir.join("bin/x64/plugins/cyber_engine_tweaks");
            if cet.is_dir() {
                p.steps.push(Step::CetBridge { path: dir.join(games::cyberpunk_bridge()) });
            } else if dir.join("bin/x64").is_dir() {
                p.notes.push("Cyber Engine Tweaks is not installed: the game keys work, the cheat keys and stats need it. Set up again after installing CET.".into());
            } else {
                p.error = Some(format!("{} has no bin/x64: not a Cyberpunk 2077 folder", dir.display()));
                return p;
            }
        }
        "deadlock" => {
            let Some(dir) = need(&mut p, "install folder (the one with game/citadel)") else { return p };
            let citadel = dir.join("game/citadel");
            if !citadel.join("cfg").is_dir() {
                p.error = Some(format!("{} has no game/citadel/cfg: not a Deadlock folder", dir.display()));
                return p;
            }
            p.steps.push(Step::DeadlockCfg { path: citadel.join("cfg") });
            if extract {
                p.steps.push(Step::DeadlockIcons { from: citadel.join("pak01_dir.vpk"), to: data_dir(info.name).join("masks") });
            }
        }
        "wow_forever" => {
            let Some(dir) = need(&mut p, "_forever_ folder (inside World of Warcraft)") else { return p };
            p.steps.push(Step::WowAddon { path: dir.join("Interface/AddOns/PULPIT") });
            if extract {
                p.notes.push("WoW's icons and fonts can't be extracted yet (Battle.net's CASC storage): the keys use a stand-in look.".into());
            }
        }
        // Keys only: nothing to put into the game.
        _ => p.path = path.filter(|d| d.is_dir()),
    }
    p.steps.push(Step::Enable);
    p
}

/// Run an approved plan and save the config.
pub fn apply(cfg_path: &Path, plan: &Plan) -> Result<()> {
    if let Some(e) = &plan.error {
        bail!("{e}");
    }
    let info = games::by_name(plan.game).context("unknown game")?;
    let mut cfg = Config::load(cfg_path)?;
    for step in &plan.steps {
        println!("{}", step.text());
        match step {
            Step::CetBridge { path } => install_cet(path)?,
            Step::DeadlockCfg { path } => install_deadlock_cfg(path)?,
            Step::DeadlockIcons { from, to } => extract_deadlock(from, to)?,
            Step::WowAddon { path } => install_wow_addon(path)?,
            Step::Enable => {
                let entry = cfg.games.entry(info.name.to_owned()).or_insert_with(GameConfig::default);
                entry.enabled = true;
                // A folder Steam finds by itself is not pinned in the config.
                if let Some(p) = plan.path.as_ref().filter(|_| !plan.detected) {
                    entry.path = Some(p.clone());
                }
            }
        }
    }
    cfg.save(cfg_path)
}

/// `setup GAME [--path DIR]` from the command line: plan, show and apply.
pub fn setup(cfg_path: &Path, name: &str, path: Option<PathBuf>, extract: bool) -> Result<()> {
    let info = games::by_name(name).with_context(|| format!("unknown game {name:?}"))?;
    let cfg = Config::load(cfg_path)?;
    let path = path.map(|p| p.canonicalize().with_context(|| format!("{} does not exist", p.display()))).transpose()?;
    let plan = plan(info, &cfg, path, extract);
    if let Some(p) = &plan.path {
        println!("{}: {}", info.title, p.display());
    }
    for n in &plan.notes {
        println!("note: {n}");
    }
    apply(cfg_path, &plan)?;
    println!("{} is on ({}).", info.title, cfg_path.display());
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

/// A game's state, for `list` and the app.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub game: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    pub enabled: bool,
    /// The configured or found install folder.
    pub path: Option<PathBuf>,
    /// The look's assets from the user's install are in place (or the game needs none).
    pub assets: bool,
}

pub fn status(cfg: &Config) -> Vec<Status> {
    games::ALL
        .iter()
        .map(|info| Status {
            game: info.name,
            title: info.title,
            about: info.about,
            enabled: cfg.games.get(info.name).is_some_and(|g| g.enabled),
            path: games::install(info, cfg),
            assets: assets_ready(info),
        })
        .collect()
}

/// What this build has and what is set up, for `masterhub-games list`.
pub fn list(cfg_path: &Path, json: bool) -> Result<()> {
    let st = status(&Config::load(cfg_path)?);
    if json {
        println!("{}", serde_json::to_string(&st)?);
        return Ok(());
    }
    for s in st {
        println!(
            "{:<15} {:<4} {}{}",
            s.game,
            if s.enabled { "on" } else { "off" },
            s.path.as_ref().map_or("not found".into(), |p| p.display().to_string()),
            if s.assets { "" } else { "  (game assets not extracted)" }
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

/// Copy the CET bridge into `dir` (`.../cyber_engine_tweaks/mods/pulpit_bridge`).
fn install_cet(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir.join("eval"))?;
    std::fs::write(dir.join("init.lua"), CET_INIT)?;
    std::fs::write(dir.join("logic.lua"), CET_LOGIC)?;
    Ok(())
}

/// Put `pulpit.cfg` (console commands on the navigation keys) into the game's cfg folder and
/// make `autoexec.cfg` run it.
fn install_deadlock_cfg(cfg: &Path) -> Result<()> {
    std::fs::write(cfg.join("pulpit.cfg"), DEADLOCK_CFG)?;
    let auto = cfg.join("autoexec.cfg");
    let text = std::fs::read_to_string(&auto).unwrap_or_default();
    if !text.lines().any(|l| l.trim_start().starts_with("exec pulpit")) {
        let sep = if text.is_empty() || text.ends_with('\n') { "" } else { "\n" };
        std::fs::write(&auto, format!("{text}{sep}exec pulpit\n"))?;
    }
    Ok(())
}

/// Copy the add-on into `dir` (`<install>/Interface/AddOns/PULPIT`).
fn install_wow_addon(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("PULPIT.lua"), WOW_LUA)?;
    std::fs::write(dir.join("PULPIT.toc"), WOW_TOC)?;
    std::fs::write(dir.join("PULPIT_Forever.toc"), WOW_TOC_FOREVER)?;
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
fn extract_deadlock(pak: &Path, out: &Path) -> Result<()> {
    let pak = crate::vpk::Vpk::open(pak)?;
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

//! The MCP tools: read the hub and its config, edit profiles, pages, controls and setups, and
//! switch setups and profiles. Edits load the service's config, change one piece, run the same
//! checks as the app (`actions::validate_config`) and save it with `set_config`, which checks
//! again before it writes the file. Nothing here can send hardware mode or run an action.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail, ensure};
use masterhub_core::config::{
    Config, ControlRef, DialConfig, FaderConfig, KeyConfig, KnobConfig, Page, PageView, PinnedKey, Profile, RollerConfig, Setup, Side,
    Source,
};
use masterhub_core::ipc::{HubState, ModuleState, Request};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::Hub;

/// What a tool does, for the agent's permission prompts (MCP tool annotations).
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Only reads.
    Read,
    /// Changes the config; repeating it changes nothing more.
    Edit,
    /// Changes the config and may remove something.
    Remove,
    /// Changes what the hub shows now, not the config.
    Live,
}

struct Tool {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    kind: Kind,
    /// Properties and required names of the input schema.
    params: fn() -> (Value, &'static [&'static str]),
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "get_hub",
        title: "Hub status",
        description: "Connected modules (kind, socket, placement, firmware, needs_replug), the live state (active setup, profile, page, focused app, why the profile is live, pin), the service's capabilities and version.",
        kind: Kind::Read,
        params: || (json!({}), &[]),
    },
    Tool {
        name: "list_profiles",
        title: "List profiles",
        description: "Every profile with its apps, base, colour, pages and folders (names and set keys), which other controls it sets, and the setups that list it.",
        kind: Kind::Read,
        params: || (json!({}), &[]),
    },
    Tool {
        name: "get_profile",
        title: "Get a profile",
        description: "One profile exactly as stored: pages, folders, keys, dial, knobs, faders, rollers, LEDs. Controls it does not set come from its base profile (see get_controls).",
        kind: Kind::Read,
        params: || (json!({ "name": name("Profile name") }), &["name"]),
    },
    Tool {
        name: "get_controls",
        title: "Resolved controls",
        description: "What every control of a profile does on one keypad page, after pinned keys and base profiles are applied, with where each comes from (own, base <name>, pinned, none).",
        kind: Kind::Read,
        params: || {
            (
                json!({
                    "profile": name("Profile name"),
                    "page": { "type": "string", "description": "Page (or folder) name; default the first page" },
                    "folder": { "type": "boolean", "description": "`page` names a folder" },
                    "setup": { "type": "string", "description": "Setup to resolve pinned-key exceptions for; default the active one" },
                }),
                &["profile"],
            )
        },
    },
    Tool {
        name: "list_setups",
        title: "List setups",
        description: "The setups (a config without [[setup]] has one implicit follow setup, \"Default\") and which one is active.",
        kind: Kind::Read,
        params: || (json!({}), &[]),
    },
    Tool {
        name: "get_config",
        title: "Get the config",
        description: "The whole config as TOML text.",
        kind: Kind::Read,
        params: || (json!({}), &[]),
    },
    Tool {
        name: "validate_config",
        title: "Validate a config",
        description: "Check a whole config (TOML text) the way saving does, without saving it. Returns {valid, error}.",
        kind: Kind::Read,
        params: || (json!({ "toml": { "type": "string", "description": "Config TOML" } }), &["toml"]),
    },
    Tool {
        name: "list_open_apps",
        title: "Open apps",
        description: "Window classes (.exe names on Windows) of the open windows, as a profile's `apps` patterns match them.",
        kind: Kind::Read,
        params: || (json!({}), &[]),
    },
    Tool {
        name: "add_profile",
        title: "Add a profile",
        description: "Create a profile with one empty page \"Main\", or a copy of `copy_from`. Optionally add it to setups (a follow or hybrid setup then picks it for its `apps`).",
        kind: Kind::Edit,
        params: || {
            (
                json!({
                    "name": name("New profile name"),
                    "apps": apps(),
                    "base": { "type": "string", "description": "Profile to inherit unset controls from" },
                    "color": color("Avatar colour in the app"),
                    "about": { "type": "string", "description": "Short description" },
                    "copy_from": { "type": "string", "description": "Copy pages, controls and LEDs of this profile" },
                    "setups": { "type": "array", "items": { "type": "string" }, "description": "Setups to append it to" },
                }),
                &["name"],
            )
        },
    },
    Tool {
        name: "update_profile",
        title: "Update a profile",
        description: "Change a profile's apps, base, colour, description or game flag. Fields left out stay; an empty string clears base, color or about.",
        kind: Kind::Edit,
        params: || {
            (
                json!({
                    "name": name("Profile name"),
                    "apps": apps(),
                    "base": { "type": "string", "description": "Profile to inherit unset controls from; \"\" for none" },
                    "color": { "type": "string", "description": "#rrggbb, or \"\" for none" },
                    "about": { "type": "string" },
                    "game": { "type": "boolean", "description": "List it under Games in the app" },
                }),
                &["name"],
            )
        },
    },
    Tool {
        name: "delete_profile",
        title: "Delete a profile",
        description: "Remove a profile, and its name from setup lists and pinned-key exceptions. Refused while another profile uses it as base, a setup as fallback, or an action switches to it.",
        kind: Kind::Remove,
        params: || (json!({ "name": name("Profile name") }), &["name"]),
    },
    Tool {
        name: "add_page",
        title: "Add a page or folder",
        description: "Add an empty keypad page (cycled with page next/prev) or folder (opened with a folder action, left with back) to a profile.",
        kind: Kind::Edit,
        params: || (json!({ "profile": name("Profile name"), "name": name("Page or folder name"), "folder": folder() }), &["profile", "name"]),
    },
    Tool {
        name: "delete_page",
        title: "Delete a page or folder",
        description: "Remove a keypad page or folder and its keys from a profile.",
        kind: Kind::Remove,
        params: || (json!({ "profile": name("Profile name"), "name": name("Page or folder name"), "folder": folder() }), &["profile", "name"]),
    },
    Tool {
        name: "set_key",
        title: "Set a key",
        description: "Set or clear a keypad key on a page or folder of a profile: its face (label, image, colours, widget) and its press/release actions. `key: null` clears it (a base profile's key then shows through).",
        kind: Kind::Edit,
        params: || {
            (
                json!({
                    "profile": name("Profile name"),
                    "page": { "type": "string", "description": "Page (or folder) name; default the first page" },
                    "folder": folder(),
                    "index": { "type": "integer", "minimum": 0, "maximum": 14, "description": "0..=14, row-major from the top-left (5 columns × 3 rows)" },
                    "key": nullable(object(face_and(&[("press", action()), ("release", action())]))),
                    "merge": merge(),
                }),
                &["profile", "index", "key"],
            )
        },
    },
    Tool {
        name: "set_dial",
        title: "Set the dial",
        description: "Set or clear a profile's dial: its screen face and the cw, ccw, press, release and long_press actions. `dial: null` clears it.",
        kind: Kind::Edit,
        params: || {
            let actions = ["cw", "ccw", "press", "release", "long_press"].map(|a| (a, action()));
            (json!({ "profile": name("Profile name"), "dial": nullable(object(face_and(&actions))), "merge": merge() }), &["profile", "dial"])
        },
    },
    Tool {
        name: "set_knob",
        title: "Set a knob",
        description: "Set or clear a knob of a profile: cw, ccw, press and release actions. `knob: null` clears it.",
        kind: Kind::Edit,
        params: || {
            let actions: Map<String, Value> = ["cw", "ccw", "press", "release"].map(|a| (a.to_owned(), action())).into_iter().collect();
            (
                json!({
                    "profile": name("Profile name"),
                    "index": { "type": "integer", "minimum": 0, "maximum": 2, "description": "0 bottom-left, 1 bottom-right, 2 top" },
                    "knob": nullable(object(actions)),
                    "merge": merge(),
                }),
                &["profile", "index", "knob"],
            )
        },
    },
    Tool {
        name: "set_fader",
        title: "Set a fader",
        description: "Set or clear a fader of a profile: its change action, run on every move with {value} (0..=100) substituted, e.g. {\"volume\": \"{value}\"}. `fader: null` clears it.",
        kind: Kind::Edit,
        params: || {
            (
                json!({
                    "profile": name("Profile name"),
                    "index": { "type": "integer", "minimum": 0, "maximum": 4, "description": "0 = top" },
                    "fader": nullable(object([("change".to_owned(), action())].into_iter().collect())),
                    "merge": merge(),
                }),
                &["profile", "index", "fader"],
            )
        },
    },
    Tool {
        name: "set_roller",
        title: "Set a roller",
        description: "Set or clear a roller of a profile: away (rolled towards the keys) and toward actions. `roller: null` clears it.",
        kind: Kind::Edit,
        params: || {
            (
                json!({
                    "profile": name("Profile name"),
                    "side": { "type": "string", "enum": ["left", "right"] },
                    "roller": nullable(object([("away".to_owned(), action()), ("toward".to_owned(), action())].into_iter().collect())),
                    "merge": merge(),
                }),
                &["profile", "side", "roller"],
            )
        },
    },
    Tool {
        name: "set_pinned_key",
        title: "Set a pinned key",
        description: "Set or clear a key shown on every page of every profile (except those in `except` or the profiles of `except_setups`). Pinned keys cannot use page or folder actions. `key: null` clears it.",
        kind: Kind::Edit,
        params: || {
            let mut key = face_and(&[("press", action()), ("release", action())]);
            key.insert("except".into(), json!({ "type": "array", "items": { "type": "string" }, "description": "Profiles that show their own key here" }));
            key.insert("except_setups".into(), json!({ "type": "array", "items": { "type": "string" }, "description": "Setups whose profiles show their own key here" }));
            (
                json!({
                    "index": { "type": "integer", "minimum": 0, "maximum": 14 },
                    "key": nullable(object(key)),
                    "merge": merge(),
                }),
                &["index", "key"],
            )
        },
    },
    Tool {
        name: "put_setup",
        title: "Add or change a setup",
        description: "Create a setup, or replace (merge: true: update) the one with the same name. In a config without [[setup]], the implicit \"Default\" setup is written out first so it is kept.",
        kind: Kind::Edit,
        params: || (json!({ "setup": setup_schema(), "merge": merge() }), &["setup"]),
    },
    Tool {
        name: "delete_setup",
        title: "Delete a setup",
        description: "Remove a setup (its profiles stay), and its name from pinned-key exceptions. Refused while an action switches to it.",
        kind: Kind::Remove,
        params: || (json!({ "name": name("Setup name") }), &["name"]),
    },
    Tool {
        name: "put_config",
        title: "Replace the config",
        description: "Replace the whole config with TOML text, after the same checks as saving in the app. The [app] table (app settings) is kept as it is. Prefer the structured tools.",
        kind: Kind::Remove,
        params: || (json!({ "toml": { "type": "string", "description": "Config TOML" } }), &["toml"]),
    },
    Tool {
        name: "switch_setup",
        title: "Switch setup",
        description: "Make a setup active, by name or \"next\"/\"prev\".",
        kind: Kind::Live,
        params: || (json!({ "name": name("Setup name, \"next\" or \"prev\"") }), &["name"]),
    },
    Tool {
        name: "switch_profile",
        title: "Switch profile",
        description: "Show a profile now. In follow setups it lasts until the focused app changes; in hybrid setups use pin_profile to keep it.",
        kind: Kind::Live,
        params: || (json!({ "name": name("Profile name") }), &["name"]),
    },
    Tool {
        name: "show_page",
        title: "Show a page",
        description: "Show a keypad page or open a folder of the live profile, by name.",
        kind: Kind::Live,
        params: || (json!({ "name": name("Page or folder name") }), &["name"]),
    },
    Tool {
        name: "pin_profile",
        title: "Pin a profile",
        description: "Hybrid setups: pin a profile (default the live one) so the focused app no longer changes it.",
        kind: Kind::Live,
        params: || (json!({ "profile": { "type": "string", "description": "Profile name; default the live profile" } }), &[]),
    },
    Tool {
        name: "release_pin",
        title: "Release the pin",
        description: "Hybrid setups: end the pin, so the profile follows the focused app again.",
        kind: Kind::Live,
        params: || (json!({}), &[]),
    },
];

pub fn exists(name: &str) -> bool {
    TOOLS.iter().any(|t| t.name == name)
}

/// `tools/list`.
pub fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|t| {
            let (properties, required) = (t.params)();
            json!({
                "name": t.name,
                "title": t.title,
                "description": t.description,
                "inputSchema": { "type": "object", "properties": properties, "required": required, "additionalProperties": false },
                "annotations": {
                    "title": t.title,
                    "readOnlyHint": t.kind == Kind::Read,
                    "destructiveHint": t.kind == Kind::Remove,
                    "idempotentHint": t.kind != Kind::Live,
                    "openWorldHint": false,
                },
            })
        })
        .collect()
}

/// `tools/call`.
pub fn call(hub: &dyn Hub, name: &str, args: &Value) -> Result<Value> {
    let a = Args(args);
    match name {
        "get_hub" => get_hub(hub),
        "list_profiles" => {
            let c = load(hub)?.config;
            let setups = c.effective_setups();
            Ok(Value::Array(c.profiles.iter().map(|p| profile_summary(p, &setups)).collect()))
        }
        "get_profile" => {
            let c = load(hub)?.config;
            let name = a.str("name")?;
            Ok(serde_json::to_value(find_profile(&c, &name)?)?)
        }
        "get_controls" => get_controls(hub, &a),
        "list_setups" => {
            let c = load(hub)?.config;
            let state: Option<HubState> = hub.call(Request::GetState).ok().and_then(|v| serde_json::from_value(v).ok());
            Ok(json!({
                "active": state.and_then(|s| s.setup),
                "implicit": c.setups.is_empty(),
                "setups": c.effective_setups(),
            }))
        }
        "get_config" => Ok(Value::String(load(hub)?.text)),
        "validate_config" => {
            let dir = load(hub)?.dir;
            Ok(match check_text(&a.str("toml")?, &dir) {
                Ok(_) => json!({ "valid": true }),
                Err(e) => json!({ "valid": false, "error": format!("{e:#}") }),
            })
        }
        "list_open_apps" => hub.call(Request::GetOpenApps),
        "add_profile" => add_profile(hub, &a),
        "update_profile" => update_profile(hub, &a),
        "delete_profile" => {
            let name = a.str("name")?;
            edit(hub, |c| {
                let before = c.profiles.len();
                c.profiles.retain(|p| p.name != name);
                ensure!(c.profiles.len() < before, "no profile {name:?}");
                for s in &mut c.setups {
                    s.profiles.retain(|p| *p != name);
                }
                for p in &mut c.pinned {
                    p.except.retain(|p| *p != name);
                }
                Ok(format!("deleted profile {name:?}"))
            })
        }
        "add_page" | "delete_page" => {
            let (profile, page, folder) = (a.str("profile")?, a.str("name")?, a.bool("folder")?.unwrap_or(false));
            let what = if folder { "folder" } else { "page" };
            let add = name == "add_page";
            edit(hub, |c| {
                let p = profile_mut(c, &profile)?;
                let list = if folder { &mut p.folders } else { &mut p.pages };
                let at = list.iter().position(|x| x.name == page);
                match (add, at) {
                    (true, Some(_)) => bail!("{profile:?} already has a {what} {page:?}"),
                    (true, None) => {
                        ensure!(!page.trim().is_empty(), "a {what} needs a name");
                        list.push(Page { name: page.clone(), keys: Vec::new() });
                        Ok(format!("added {what} {page:?} to {profile:?}"))
                    }
                    (false, Some(i)) => {
                        list.remove(i);
                        Ok(format!("deleted {what} {page:?} of {profile:?}"))
                    }
                    (false, None) => bail!("{profile:?} has no {what} {page:?}"),
                }
            })
        }
        "set_key" => set_key(hub, &a),
        "set_dial" => {
            let (profile, patch, merge) = (a.str("profile")?, a.value("dial")?, a.bool("merge")?.unwrap_or(false));
            edit(hub, |c| {
                let p = profile_mut(c, &profile)?;
                p.dial = match patch.is_null() {
                    true => None,
                    false => Some(upsert::<DialConfig>(p.dial.as_ref(), patch, merge, Map::new()).context("dial")?),
                };
                check_image(p.dial.as_ref().and_then(|d| d.face.image.as_deref()))?;
                Ok(format!("{} the dial of {profile:?}", if patch.is_null() { "cleared" } else { "set" }))
            })
        }
        "set_knob" => {
            let index = a.index("index", 2)?;
            set_indexed(hub, &a, "knob", index, |p| &mut p.knobs, |k: &KnobConfig| k.index == index)
        }
        "set_fader" => {
            let index = a.index("index", 4)?;
            set_indexed(hub, &a, "fader", index, |p| &mut p.faders, |f: &FaderConfig| f.index == index)
        }
        "set_roller" => {
            let side: Side = serde_json::from_value(a.value("side")?.clone()).context("side must be \"left\" or \"right\"")?;
            let (profile, patch, merge) = (a.str("profile")?, a.value("roller")?, a.bool("merge")?.unwrap_or(false));
            edit(hub, |c| {
                let p = profile_mut(c, &profile)?;
                let old = p.rollers.iter().position(|r| r.side == side).map(|i| p.rollers.remove(i));
                if !patch.is_null() {
                    let fixed = [("side".to_owned(), serde_json::to_value(side)?)].into_iter().collect();
                    p.rollers.push(upsert::<RollerConfig>(old.as_ref(), patch, merge, fixed).context("roller")?);
                }
                Ok(format!("{} the {side:?} roller of {profile:?}", if patch.is_null() { "cleared" } else { "set" }).to_lowercase())
            })
        }
        "set_pinned_key" => {
            let (index, patch, merge) = (a.index("index", 14)?, a.value("key")?, a.bool("merge")?.unwrap_or(false));
            edit(hub, |c| {
                let old = c.pinned.iter().position(|p| p.key.index == index).map(|i| c.pinned.remove(i));
                if !patch.is_null() {
                    let fixed = [("index".to_owned(), json!(index))].into_iter().collect();
                    let key = upsert::<PinnedKey>(old.as_ref(), patch, merge, fixed).context("key")?;
                    check_image(key.key.face.image.as_deref())?;
                    c.pinned.push(key);
                    c.pinned.sort_by_key(|p| p.key.index);
                }
                Ok(format!("{} pinned key {index}", if patch.is_null() { "cleared" } else { "set" }))
            })
        }
        "put_setup" => {
            let (patch, merge) = (a.value("setup")?, a.bool("merge")?.unwrap_or(false));
            let name = patch.get("name").and_then(Value::as_str).ok_or_else(|| anyhow!("setup needs a name"))?.to_owned();
            edit(hub, |c| {
                // Writing the first setup out would drop the implicit one; keep it.
                if c.setups.is_empty() {
                    c.setups = c.effective_setups();
                }
                let at = c.setups.iter().position(|s| s.name == name);
                let setup = upsert::<Setup>(at.map(|i| &c.setups[i]), patch, merge, Map::new()).context("setup")?;
                match at {
                    Some(i) => c.setups[i] = setup,
                    None => c.setups.push(setup),
                }
                Ok(format!("{} setup {name:?}", if at.is_some() { "updated" } else { "added" }))
            })
        }
        "delete_setup" => {
            let name = a.str("name")?;
            edit(hub, |c| {
                let before = c.setups.len();
                c.setups.retain(|s| s.name != name);
                ensure!(c.setups.len() < before, "no setup {name:?}");
                for p in &mut c.pinned {
                    p.except_setups.retain(|s| *s != name);
                }
                Ok(format!("deleted setup {name:?}"))
            })
        }
        "put_config" => {
            let loaded = load(hub)?;
            let mut config = check_text(&a.str("toml")?, &loaded.dir)?;
            config.app = loaded.config.app;
            save(hub, &config)?;
            Ok(json!("saved the config"))
        }
        "switch_setup" => live(hub, Request::SetSetup { name: a.str("name")? }),
        "switch_profile" => live(hub, Request::SetProfile { name: a.str("name")? }),
        "show_page" => live(hub, Request::ShowPage { name: a.str("name")? }),
        "pin_profile" => live(hub, Request::Pin { profile: a.opt_str("profile")? }),
        "release_pin" => live(hub, Request::ReleasePin),
        _ => bail!("unknown tool {name:?}"),
    }
}

// ───────────────────────────── arguments ─────────────────────────────

struct Args<'a>(&'a Value);

impl Args<'_> {
    fn value(&self, key: &str) -> Result<&Value> {
        self.0.get(key).ok_or_else(|| anyhow!("missing argument {key:?}"))
    }
    fn str(&self, key: &str) -> Result<String> {
        self.opt_str(key)?.ok_or_else(|| anyhow!("missing argument {key:?}"))
    }
    fn opt_str(&self, key: &str) -> Result<Option<String>> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => bail!("{key:?} must be a string"),
        }
    }
    fn bool(&self, key: &str) -> Result<Option<bool>> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => bail!("{key:?} must be true or false"),
        }
    }
    fn strings(&self, key: &str) -> Result<Option<Vec<String>>> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => Ok(Some(serde_json::from_value(v.clone()).with_context(|| format!("{key:?} must be a list of strings"))?)),
        }
    }
    /// An index 0..=`max`.
    fn index(&self, key: &str, max: u8) -> Result<u8> {
        let i = self.value(key)?.as_u64().ok_or_else(|| anyhow!("{key:?} must be a whole number"))?;
        ensure!(i <= u64::from(max), "{key:?} must be 0..={max}");
        Ok(i as u8)
    }
}

// ───────────────────────────── config ─────────────────────────────

struct Loaded {
    config: Config,
    text: String,
    /// The config's directory; relative image paths resolve against it.
    dir: PathBuf,
}

fn load(hub: &dyn Hub) -> Result<Loaded> {
    let text: String = serde_json::from_value(hub.call(Request::GetConfig)?)?;
    let path: String = serde_json::from_value(hub.call(Request::GetConfigPath)?)?;
    let dir = Path::new(&path).parent().unwrap_or(Path::new(".")).to_owned();
    let config = Config::parse_unchecked(&text, &dir).context("the current config does not parse")?;
    Ok(Loaded { config, text, dir })
}

/// Parse a config text and run every check saving runs.
fn check_text(text: &str, dir: &Path) -> Result<Config> {
    let config = Config::parse_unchecked(text, dir)?;
    crate::actions::validate_config(&config)?;
    Ok(config)
}

/// Check, send and wait until the service has applied it, so the next tool sees the change.
fn save(hub: &dyn Hub, config: &Config) -> Result<()> {
    crate::actions::validate_config(config).context("not saved")?;
    let toml = config.to_toml()?;
    hub.call(Request::SetConfig { toml: toml.clone() }).context("not saved")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if hub.call(Request::GetConfig)?.as_str() == Some(toml.as_str()) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    log::warn!("mcp: the service has not applied the saved config yet");
    Ok(())
}

/// Load, change and save; the change's message is the tool's result.
fn edit(hub: &dyn Hub, change: impl FnOnce(&mut Config) -> Result<String>) -> Result<Value> {
    let mut config = load(hub)?.config;
    let message = change(&mut config)?;
    save(hub, &config)?;
    Ok(Value::String(message))
}

fn names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<String> = names.map(|n| format!("{n:?}")).collect();
    if all.is_empty() { "none".into() } else { all.join(", ") }
}

fn find_profile<'a>(c: &'a Config, name: &str) -> Result<&'a Profile> {
    c.profile(name).ok_or_else(|| anyhow!("no profile {name:?}; there are {}", names(c.profiles.iter().map(|p| p.name.as_str()))))
}

fn profile_mut<'a>(c: &'a mut Config, name: &str) -> Result<&'a mut Profile> {
    find_profile(c, name)?;
    Ok(c.profiles.iter_mut().find(|p| p.name == name).expect("found above"))
}

/// The page (or folder) `name` of a profile; without a name the first page, created as "Main"
/// in a profile that has none.
fn page_mut<'a>(p: &'a mut Profile, name: Option<&str>, folder: bool) -> Result<&'a mut Page> {
    let profile = p.name.clone();
    let what = if folder { "folder" } else { "page" };
    let list = if folder { &mut p.folders } else { &mut p.pages };
    let Some(name) = name else {
        ensure!(!folder, "name the folder");
        if list.is_empty() {
            list.push(Page { name: "Main".into(), keys: Vec::new() });
        }
        return Ok(&mut list[0]);
    };
    let known = names(list.iter().map(|x| x.name.as_str()));
    list.iter_mut().find(|x| x.name == name).ok_or_else(|| anyhow!("{profile:?} has no {what} {name:?}; there are {known}"))
}

/// A control's new config: `patch` alone, or with `merge` laid over the old one (a `null` field
/// removes it). `fixed` (its index or side) always wins.
fn upsert<T: Serialize + DeserializeOwned>(old: Option<&T>, patch: &Value, merge: bool, fixed: Map<String, Value>) -> Result<T> {
    let Value::Object(patch) = patch else { bail!("must be an object or null") };
    let mut out = match (merge, old) {
        (true, Some(old)) => match serde_json::to_value(old)? {
            Value::Object(m) => m,
            _ => Map::new(),
        },
        _ => Map::new(),
    };
    for (k, v) in patch {
        if v.is_null() {
            out.remove(k);
        } else {
            out.insert(k.clone(), v.clone());
        }
    }
    out.extend(fixed);
    Ok(serde_json::from_value(Value::Object(out))?)
}

/// Images must be absolute or under `~/`: the service resolves other relative paths against
/// the config's folder, which the agent rarely means.
fn check_image(image: Option<&Path>) -> Result<()> {
    if let Some(img) = image
        && img.is_relative()
        && !img.starts_with("~")
    {
        bail!("image {} must be an absolute path (or start with ~/)", img.display());
    }
    Ok(())
}

fn live(hub: &dyn Hub, request: Request) -> Result<Value> {
    hub.call(request)?;
    Ok(json!("done"))
}

// ───────────────────────────── tools ─────────────────────────────

fn get_hub(hub: &dyn Hub) -> Result<Value> {
    let state = hub.call(Request::GetState)?;
    let modules: Vec<ModuleState> = serde_json::from_value(hub.call(Request::GetModules)?)?;
    // Serials identify the user's devices and no tool needs them.
    let modules: Vec<Value> = modules
        .into_iter()
        .map(|m| json!({ "kind": m.kind, "socket": m.socket, "placement": m.placement, "firmware": m.firmware, "needs_replug": m.needs_replug }))
        .collect();
    Ok(json!({
        "state": state,
        "modules": modules,
        "capabilities": hub.call(Request::GetCapabilities).unwrap_or(json!([])),
        "system": hub.call(Request::GetSystemInfo).ok(),
    }))
}

fn profile_summary(p: &Profile, setups: &[Setup]) -> Value {
    let pages = |list: &[Page]| -> Vec<Value> {
        list.iter().map(|pg| json!({ "name": pg.name, "keys": pg.keys.iter().filter(|k| k.is_set()).map(|k| k.index).collect::<Vec<_>>() })).collect()
    };
    json!({
        "name": p.name,
        "apps": p.apps,
        "base": p.base,
        "color": p.color,
        "about": p.about,
        "pages": pages(&p.pages),
        "folders": pages(&p.folders),
        "dial": p.dial.as_ref().is_some_and(DialConfig::is_set),
        "knobs": p.knobs.iter().map(|k| k.index).collect::<Vec<_>>(),
        "faders": p.faders.iter().map(|f| f.index).collect::<Vec<_>>(),
        "rollers": p.rollers.iter().map(|r| r.side).collect::<Vec<_>>(),
        "setups": setups
            .iter()
            .filter(|s| s.profiles.contains(&p.name) || s.fallback.as_ref() == Some(&p.name))
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
    })
}

fn source(s: &Source) -> String {
    match s {
        Source::Pinned(_) => "pinned".into(),
        Source::Own(_) => "own".into(),
        Source::Base { profile, .. } => format!("base {}", profile.name),
        Source::None => "none".into(),
    }
}

fn get_controls(hub: &dyn Hub, a: &Args) -> Result<Value> {
    let c = load(hub)?.config;
    let (name, page, folder) = (a.str("profile")?, a.opt_str("page")?, a.bool("folder")?.unwrap_or(false));
    let p = find_profile(&c, &name)?;
    let setup = match a.opt_str("setup")? {
        Some(s) => Some(s),
        None => hub.call(Request::GetState).ok().and_then(|v| serde_json::from_value::<HubState>(v).ok()).and_then(|s| s.setup),
    };
    let view = match (&page, folder) {
        (Some(f), true) => PageView::Folder(f),
        (None, true) => bail!("name the folder"),
        (Some(pg), false) => PageView::Page(
            // A base profile may have the page even if this one does not; then it is the next one.
            p.pages.iter().position(|x| x.name == *pg).ok_or_else(|| anyhow!("{name:?} has no page {pg:?}"))?,
        ),
        (None, false) => PageView::Page(0),
    };
    let s = setup.as_deref();
    let control = |ctl: ControlRef, config: Value| json!({ "from": source(&c.source_of(&name, view, s, ctl)), "config": config });
    let keys: Vec<Value> = (0..15u8)
        .map(|i| {
            let mut v = control(ControlRef::Key(i), val(c.key_at(&name, view, s, i)));
            v["index"] = json!(i);
            v
        })
        .collect();
    let indexed = |n: u8, f: &dyn Fn(u8) -> (ControlRef, Value)| -> Vec<Value> {
        (0..n)
            .map(|i| {
                let (ctl, config) = f(i);
                let mut v = control(ctl, config);
                v["index"] = json!(i);
                v
            })
            .collect()
    };
    let rollers = [Side::Left, Side::Right].map(|side| {
        let mut v = control(ControlRef::Roller(side), val(c.roller_of(&name, side)));
        v["side"] = json!(side);
        v
    });
    let page_name = page.clone().unwrap_or_else(|| p.pages.first().map(|x| x.name.clone()).unwrap_or_default());
    Ok(json!({
        "profile": name,
        "page": page_name,
        "setup": setup,
        "keys": keys,
        "dial": control(ControlRef::Dial, val(c.dial_of(&name))),
        "knobs": indexed(3, &|i| (ControlRef::Knob(i), val(c.knob_of(&name, i)))),
        "faders": indexed(5, &|i| (ControlRef::Fader(i), val(c.fader_of(&name, i)))),
        "rollers": rollers,
    }))
}

fn val<T: Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap_or_default()
}

fn add_profile(hub: &dyn Hub, a: &Args) -> Result<Value> {
    let name = a.str("name")?;
    let (apps, base, color, about, copy_from, setups) =
        (a.strings("apps")?, a.opt_str("base")?, a.opt_str("color")?, a.opt_str("about")?, a.opt_str("copy_from")?, a.strings("setups")?);
    edit(hub, |c| {
        ensure!(!name.trim().is_empty(), "a profile needs a name");
        ensure!(c.profile(&name).is_none(), "profile {name:?} already exists");
        let mut p = match &copy_from {
            Some(from) => Profile { name: name.clone(), ..find_profile(c, from)?.clone() },
            None => Profile { name: name.clone(), pages: vec![Page { name: "Main".into(), keys: Vec::new() }], ..Default::default() },
        };
        if let Some(apps) = apps {
            p.apps = apps;
        }
        if base.is_some() {
            p.base = base;
        }
        if let Some(color) = &color {
            p.color = Some(serde_json::from_value(json!(color)).context("color")?);
        }
        if about.is_some() {
            p.about = about;
        }
        c.profiles.push(p);
        if let Some(setups) = &setups {
            if c.setups.is_empty() && !setups.is_empty() {
                c.setups = c.effective_setups();
            }
            for s in setups {
                let setup = c.setups.iter_mut().find(|x| x.name == *s).ok_or_else(|| anyhow!("no setup {s:?}"))?;
                if !setup.profiles.contains(&name) {
                    setup.profiles.push(name.clone());
                }
            }
        }
        Ok(format!("added profile {name:?}"))
    })
}

fn update_profile(hub: &dyn Hub, a: &Args) -> Result<Value> {
    let name = a.str("name")?;
    let (apps, base, color, about, game) = (a.strings("apps")?, a.opt_str("base")?, a.opt_str("color")?, a.opt_str("about")?, a.bool("game")?);
    edit(hub, |c| {
        let p = profile_mut(c, &name)?;
        let clear = |s: String| (!s.is_empty()).then_some(s);
        if let Some(apps) = apps {
            p.apps = apps;
        }
        if let Some(base) = base {
            p.base = clear(base);
        }
        if let Some(color) = color {
            p.color = match clear(color) {
                Some(c) => Some(serde_json::from_value(json!(c)).context("color")?),
                None => None,
            };
        }
        if let Some(about) = about {
            p.about = clear(about);
        }
        if game.is_some() {
            p.game = game;
        }
        Ok(format!("updated profile {name:?}"))
    })
}

fn set_key(hub: &dyn Hub, a: &Args) -> Result<Value> {
    let (profile, page, folder) = (a.str("profile")?, a.opt_str("page")?, a.bool("folder")?.unwrap_or(false));
    let (index, patch, merge) = (a.index("index", 14)?, a.value("key")?, a.bool("merge")?.unwrap_or(false));
    edit(hub, |c| {
        let p = profile_mut(c, &profile)?;
        let pg = page_mut(p, page.as_deref(), folder)?;
        let old = pg.keys.iter().position(|k| k.index == index).map(|i| pg.keys.remove(i));
        if !patch.is_null() {
            let fixed = [("index".to_owned(), json!(index))].into_iter().collect();
            let key = upsert::<KeyConfig>(old.as_ref(), patch, merge, fixed).context("key")?;
            check_image(key.face.image.as_deref())?;
            pg.keys.push(key);
            pg.keys.sort_by_key(|k| k.index);
        }
        let page = pg.name.clone();
        Ok(format!("{} key {index} on {page:?} of {profile:?}", if patch.is_null() { "cleared" } else { "set" }))
    })
}

/// `set_knob` and `set_fader`: controls kept in a list by index.
fn set_indexed<T: Serialize + DeserializeOwned>(
    hub: &dyn Hub,
    a: &Args,
    what: &str,
    index: u8,
    list: impl FnOnce(&mut Profile) -> &mut Vec<T>,
    is: impl Fn(&T) -> bool,
) -> Result<Value> {
    let (profile, patch, merge) = (a.str("profile")?, a.value(what)?, a.bool("merge")?.unwrap_or(false));
    edit(hub, |c| {
        let list = list(profile_mut(c, &profile)?);
        let old = list.iter().position(&is).map(|i| list.remove(i));
        if !patch.is_null() {
            let fixed = [("index".to_owned(), json!(index))].into_iter().collect();
            list.push(upsert::<T>(old.as_ref(), patch, merge, fixed).context(what.to_owned())?);
        }
        Ok(format!("{} {what} {index} of {profile:?}", if patch.is_null() { "cleared" } else { "set" }))
    })
}

// ───────────────────────────── schemas ─────────────────────────────

fn name(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn folder() -> Value {
    json!({ "type": "boolean", "description": "A folder instead of a page" })
}

fn merge() -> Value {
    json!({ "type": "boolean", "description": "Lay the given fields over the current ones (null removes a field) instead of replacing it all" })
}

fn apps() -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": "Window classes that pick this profile in follow/hybrid setups (case-insensitive, * wildcards), e.g. [\"obs\", \"steam_app_*\"]; see list_open_apps" })
}

fn color(description: &str) -> Value {
    json!({ "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "description": format!("{description}, #rrggbb") })
}

fn nullable(schema: Value) -> Value {
    json!({ "anyOf": [schema, { "type": "null" }] })
}

fn object(properties: Map<String, Value>) -> Value {
    json!({ "type": "object", "properties": properties, "additionalProperties": false })
}

/// A key or dial face, plus `actions`.
fn face_and(actions: &[(&str, Value)]) -> Map<String, Value> {
    let mut m: Map<String, Value> = json!({
        "label": { "type": "string" },
        "image": { "type": "string", "description": "Absolute path (or ~/...) of a PNG, JPEG or animated GIF" },
        "background": color("Background"),
        "text_color": color("Label colour"),
        "widget": {
            "type": "object",
            "description": "Live data drawn on the face",
            "properties": {
                "kind": { "type": "string", "enum": ["clock", "cpu_usage", "cpu_temp", "ram_usage", "gpu_usage", "gpu_temp", "volume", "now_playing", "claude_session"] },
                "format": { "type": "string", "description": "clock: strftime format, default %H:%M" },
                "slot": { "type": "integer", "minimum": 0, "description": "claude_session: session slot, 0 = first" },
                "refresh": { "type": "integer", "minimum": 1, "description": "Seconds between updates" },
            },
            "required": ["kind"],
            "additionalProperties": false,
        },
        "fit": { "type": "string", "enum": ["fit", "fill", "center"] },
        "label_over_image": { "type": "boolean" },
    })
    .as_object()
    .cloned()
    .unwrap_or_default();
    for (k, v) in actions {
        m.insert((*k).to_owned(), v.clone());
    }
    m
}

/// What a control does: exactly one of these keys.
fn action() -> Value {
    let target = |value: Value| {
        json!({
            "anyOf": [
                value.clone(),
                {
                    "type": "object",
                    "description": "With a target; at most one of output, input, app, group",
                    "properties": {
                        "level": value.clone(), "state": value,
                        "output": { "type": "string", "description": "Output device name or \"default\"" },
                        "input": { "type": "string", "description": "Input device name or \"default\" (microphone)" },
                        "app": { "type": "string", "description": "App playing audio, e.g. \"discord\" (* wildcards)" },
                        "group": { "type": "string", "description": "An [[audio_group]] name" },
                    },
                    "additionalProperties": false,
                },
            ],
        })
    };
    json!({
        "type": "object",
        "description": "Exactly one key. keys: a combo; cmd: a shell command; page/folder/back: keypad navigation; profile/setup: switch (a name or next/prev/pick); pin: hybrid setups; volume/mute: audio; session/sessions: Claude Code session keys.",
        "minProperties": 1,
        "maxProperties": 1,
        "properties": {
            "keys": {
                "anyOf": [
                    { "type": "string", "description": "e.g. \"ctrl+shift+m\", \"playpause\", \"f13\"; \"o@700\" holds it 700 ms" },
                    { "type": "object", "properties": { "combo": { "type": "string" }, "hold": { "type": "integer", "minimum": 0, "maximum": 3000 } }, "required": ["combo"], "additionalProperties": false },
                ],
            },
            "cmd": { "type": "string", "description": "Shell command (sh -c; cmd /C on Windows), run when the control fires; {value} is a fader's position" },
            "page": { "type": "string", "description": "\"next\", \"prev\" or a page name of the same profile" },
            "folder": { "type": "string", "description": "A folder of the same profile" },
            "back": { "type": "boolean", "description": "true: leave the folder" },
            "profile": { "type": "string", "description": "Profile name, or next/prev/pick within the active setup" },
            "setup": { "type": "string", "description": "Setup name, or next/prev/pick" },
            "pin": { "type": "string", "enum": ["toggle", "release"] },
            "volume": target(json!({ "type": "string", "description": "\"+2\"/\"-2\" change by points, \"40\" sets, \"{value}\" follows a fader" })),
            "mute": target(json!({ "type": "string", "enum": ["toggle", "on", "off"] })),
            "session": { "type": "integer", "minimum": 0, "description": "Bring up the Claude Code session in this slot" },
            "sessions": { "type": "string", "enum": ["next", "prev"], "description": "Scroll the Claude Code session keys" },
        },
        "additionalProperties": false,
    })
}

fn setup_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "mode": { "type": "string", "enum": ["follow", "manual", "hybrid"], "description": "follow (default): the first profile whose apps match the focused window, else the fallback; manual: step through profiles; hybrid: follow until pinned" },
            "profiles": { "type": "array", "items": { "type": "string" }, "description": "Priority (follow, hybrid) or cycle (manual) order" },
            "fallback": { "type": "string", "description": "Follow and hybrid: used when no rule matches; not one of profiles" },
            "color": color("LED blink and UI colour"),
            "shortcut": { "type": "string", "description": "Global shortcut that switches to this setup, e.g. \"ctrl+alt+1\"" },
            "show_on_dial": { "type": "boolean" },
            "wrap": { "type": "boolean", "description": "Manual: after the last profile comes the first" },
            "next": { "type": "string", "description": "Manual: shortcut for the next profile" },
            "prev": { "type": "string", "description": "Manual: shortcut for the previous profile" },
            "pin": { "type": "string", "description": "Hybrid: shortcut that pins the live profile" },
            "release": { "type": "string", "enum": ["same_trigger", "timeout", "next_app"], "description": "Hybrid: when a pin ends" },
            "release_after": { "type": "integer", "minimum": 1, "description": "Hybrid, release = timeout: minutes" },
        },
        "required": ["name"],
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{FakeHub, ON};
    use super::*;
    use masterhub_core::config::Action;

    fn run(hub: &FakeHub, tool: &str, args: Value) -> Result<Value> {
        call(hub, tool, &args)
    }

    #[test]
    fn every_tool_is_dispatched() {
        let hub = FakeHub::new(ON);
        for t in TOOLS {
            let r = call(&hub, t.name, &json!({}));
            assert!(!r.is_err_and(|e| e.to_string().starts_with("unknown tool")), "{} has no handler", t.name);
        }
    }

    #[test]
    fn profiles_pages_and_keys() {
        let hub = FakeHub::new(ON);
        run(&hub, "add_profile", json!({ "name": "OBS", "apps": ["obs"], "base": "Desktop", "color": "#ff8800" })).unwrap();
        run(&hub, "add_page", json!({ "profile": "OBS", "name": "Scenes", "folder": true })).unwrap();
        run(&hub, "set_key", json!({ "profile": "OBS", "index": 0, "key": { "label": "Mute", "press": { "keys": "ctrl+shift+m" } } })).unwrap();
        run(&hub, "set_key", json!({ "profile": "OBS", "index": 1, "key": { "label": "Scenes", "press": { "folder": "Scenes" } } })).unwrap();
        run(&hub, "set_key", json!({ "profile": "OBS", "page": "Scenes", "folder": true, "index": 14, "key": { "label": "Back", "press": { "back": true } } }))
            .unwrap();
        run(&hub, "set_dial", json!({ "profile": "OBS", "dial": { "cw": { "volume": "+2" }, "ccw": { "volume": { "level": "-2", "app": "obs" } } } })).unwrap();
        run(&hub, "set_knob", json!({ "profile": "OBS", "index": 2, "knob": { "press": { "mute": "toggle" } } })).unwrap();
        run(&hub, "set_fader", json!({ "profile": "OBS", "index": 0, "fader": { "change": { "volume": "{value}" } } })).unwrap();
        run(&hub, "set_roller", json!({ "profile": "OBS", "side": "left", "roller": { "away": { "page": "next" } } })).unwrap();

        let c = hub.config();
        let p = c.profile("OBS").unwrap();
        assert_eq!((p.apps.as_slice(), p.base.as_deref()), (&["obs".to_owned()][..], Some("Desktop")));
        assert_eq!(p.pages[0].name, "Main");
        assert_eq!(p.pages[0].key(0).unwrap().press, Some(Action::Keys("ctrl+shift+m".into())));
        assert_eq!(p.folders[0].key(14).unwrap().press, Some(Action::Back(true)));
        assert!(p.dial.as_ref().unwrap().ccw.is_some());
        assert_eq!(p.knobs[0].index, 2);
        assert_eq!(p.rollers[0].side, Side::Left);
        // The gate survives every save.
        assert_eq!(c.app.mcp, Some(true));

        // Merge keeps the press; replace drops it; null clears.
        run(&hub, "set_key", json!({ "profile": "OBS", "index": 0, "key": { "label": "Mic" }, "merge": true })).unwrap();
        let k = hub.config().profile("OBS").unwrap().pages[0].key(0).cloned().unwrap();
        assert_eq!((k.face.label.as_deref(), k.press.is_some()), (Some("Mic"), true));
        run(&hub, "set_key", json!({ "profile": "OBS", "index": 0, "key": { "label": "Mic" } })).unwrap();
        assert!(hub.config().profile("OBS").unwrap().pages[0].key(0).unwrap().press.is_none());
        run(&hub, "set_key", json!({ "profile": "OBS", "index": 0, "key": null })).unwrap();
        assert!(hub.config().profile("OBS").unwrap().pages[0].key(0).is_none());

        let list = run(&hub, "list_profiles", json!({})).unwrap();
        assert_eq!(list[1]["name"], "OBS");
        assert_eq!(list[1]["folders"][0]["keys"], json!([14]));
        assert_eq!(list[1]["setups"], json!(["Default"]));
    }

    #[test]
    fn bad_edits_are_refused_and_nothing_is_saved() {
        let hub = FakeHub::new(ON);
        let before = hub.text.lock().unwrap().clone();
        let err = |tool: &str, args: Value| run(&hub, tool, args).unwrap_err().to_string();
        // A key name the service cannot send: caught by the same check as the app's.
        let e = run(&hub, "set_key", json!({ "profile": "Desktop", "index": 0, "key": { "press": { "keys": "ctrl+nosuchkey" } } })).unwrap_err();
        assert!(format!("{e:#}").contains("nosuchkey"), "{e:#}");
        assert!(err("set_key", json!({ "profile": "Desktop", "index": 0, "key": { "press": { "page": "Nope" } } })).contains("not saved"));
        assert!(err("set_key", json!({ "profile": "Desktop", "index": 15, "key": null })).contains("0..=14"));
        assert!(err("set_key", json!({ "profile": "Nope", "index": 0, "key": null })).contains("there are \"Desktop\""));
        let e = run(&hub, "set_key", json!({ "profile": "Desktop", "index": 0, "key": { "lable": "typo" } })).unwrap_err();
        assert!(format!("{e:#}").contains("lable"), "{e:#}");
        assert!(err("set_key", json!({ "profile": "Desktop", "index": 0, "key": { "image": "icons/x.png" } })).contains("absolute"));
        assert!(err("add_profile", json!({ "name": "Desktop" })).contains("already exists"));
        assert!(err("add_profile", json!({ "name": "X", "base": "Nope" })).contains("not saved"));
        assert!(err("set_knob", json!({ "profile": "Desktop", "index": 3, "knob": null })).contains("0..=2"));
        assert!(err("set_pinned_key", json!({ "index": 0, "key": { "press": { "page": "next" } } })).contains("not saved"));
        assert!(err("put_config", json!({ "toml": "[[profile]]\nname = 1\n" })).contains("invalid type"));
        assert_eq!(*hub.text.lock().unwrap(), before);
        // No save was even sent.
        assert!(!hub.sent.lock().unwrap().iter().any(|r| matches!(r, Request::SetConfig { .. })));
    }

    #[test]
    fn setups() {
        let hub = FakeHub::new(ON);
        run(&hub, "add_profile", json!({ "name": "Game" })).unwrap();
        run(&hub, "put_setup", json!({ "setup": { "name": "Play", "mode": "manual", "profiles": ["Desktop", "Game"], "wrap": true } })).unwrap();
        let c = hub.config();
        // The implicit setup was written out, so adding one kept it.
        assert_eq!(c.setups.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Default", "Play"]);
        run(&hub, "put_setup", json!({ "setup": { "name": "Play", "color": "#00ff00" }, "merge": true })).unwrap();
        let play = hub.config().setups.into_iter().find(|s| s.name == "Play").unwrap();
        assert_eq!((play.profiles.len(), play.color.is_some()), (2, true));
        assert!(run(&hub, "put_setup", json!({ "setup": { "name": "Bad", "fallback": "Nope" } })).is_err());
        run(&hub, "add_profile", json!({ "name": "Chat", "setups": ["Play"] })).unwrap();
        assert!(hub.config().setups[1].profiles.contains(&"Chat".to_owned()));
        // Deleting a profile takes it out of setup lists.
        run(&hub, "delete_profile", json!({ "name": "Chat" })).unwrap();
        assert!(!hub.config().setups[1].profiles.contains(&"Chat".to_owned()));
        run(&hub, "delete_setup", json!({ "name": "Play" })).unwrap();
        assert_eq!(hub.config().setups.len(), 1);
        let listed = run(&hub, "list_setups", json!({})).unwrap();
        assert_eq!((listed["implicit"].clone(), listed["setups"][0]["name"].clone()), (json!(false), json!("Default")));
    }

    #[test]
    fn put_config_keeps_the_app_settings() {
        let hub = FakeHub::new(ON);
        run(&hub, "put_config", json!({ "toml": "[app]\nmcp = false\ntheme = \"x\"\n\n[[profile]]\nname = \"Other\"\n" })).unwrap();
        let c = hub.config();
        assert_eq!((c.app.mcp, c.app.theme.as_deref(), c.profiles[0].name.as_str()), (Some(true), None, "Other"));
        assert_eq!(run(&hub, "validate_config", json!({ "toml": "backlight = 300\n" })).unwrap()["valid"], false);
        assert_eq!(run(&hub, "validate_config", json!({ "toml": "" })).unwrap()["valid"], true);
    }

    #[test]
    fn controls_resolve_bases_and_pins() {
        let hub = FakeHub::new(ON);
        run(&hub, "set_key", json!({ "profile": "Desktop", "index": 3, "key": { "label": "Copy", "press": { "keys": "ctrl+c" } } })).unwrap();
        run(&hub, "set_pinned_key", json!({ "index": 14, "key": { "label": "Home", "press": { "profile": "Desktop" } } })).unwrap();
        run(&hub, "add_profile", json!({ "name": "Kid", "base": "Desktop" })).unwrap();
        let v = run(&hub, "get_controls", json!({ "profile": "Kid" })).unwrap();
        assert_eq!(v["keys"][3]["from"], "base Desktop");
        assert_eq!(v["keys"][3]["config"]["label"], "Copy");
        assert_eq!(v["keys"][14]["from"], "pinned");
        assert_eq!(v["keys"][0]["from"], "none");
        assert_eq!(v["rollers"][1]["side"], "right");
        assert_eq!(run(&hub, "get_profile", json!({ "name": "Kid" })).unwrap()["base"], "Desktop");
    }

    #[test]
    fn live_tools_and_hub() {
        let hub = FakeHub::new(ON);
        run(&hub, "switch_setup", json!({ "name": "Play" })).unwrap();
        run(&hub, "switch_profile", json!({ "name": "Desktop" })).unwrap();
        run(&hub, "pin_profile", json!({})).unwrap();
        run(&hub, "release_pin", json!({})).unwrap();
        assert!(run(&hub, "switch_setup", json!({ "name": "Nope" })).unwrap_err().to_string().contains("no setup"));
        let sent = hub.sent.lock().unwrap().clone();
        assert!(sent.contains(&Request::SetSetup { name: "Play".into() }));
        assert!(sent.contains(&Request::Pin { profile: None }));
        assert!(sent.contains(&Request::ReleasePin));
        drop(sent);
        let h = run(&hub, "get_hub", json!({})).unwrap();
        assert_eq!(h["modules"][0]["kind"], "keypad");
        assert!(!h.to_string().contains("SERIAL"), "serials stay out of tool results");
    }
}

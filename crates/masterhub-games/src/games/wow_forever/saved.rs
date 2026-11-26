//! The PULPIT add-on's save (games/wow_forever/addon): `WTF/Account/<acc>/SavedVariables/
//! PULPIT.lua`, written by the game on /reload, logout and exit. The add-on keeps one JSON
//! string in it (`PulpitDB.json`): bars, character, quests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // kind, id, cd: for the helper panel
pub struct Slot {
    #[serde(default)]
    pub empty: bool,
    pub kind: Option<String>,
    pub id: Option<serde_json::Value>,
    /// The icon's FileDataID (a number), or a path on old clients.
    pub icon: Option<serde_json::Value>,
    pub name: Option<String>,
    /// Base cooldown in seconds (spells).
    pub cd: Option<f64>,
    /// The WoW key bound to this button ("CTRL-1").
    pub bind: Option<String>,
}

impl Slot {
    pub fn icon_id(&self) -> Option<u64> {
        self.icon.as_ref()?.as_u64()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Objective {
    pub text: Option<String>,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // id: for the helper panel
pub struct Quest {
    pub id: u64,
    pub title: Option<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub watched: bool,
    #[serde(default)]
    pub objectives: Vec<Objective>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)] // time, name, class: for the helper panel
pub struct Snapshot {
    pub time: Option<i64>,
    pub name: Option<String>,
    pub class: Option<String>,
    pub level: Option<u32>,
    pub xp: Option<f64>,
    pub xpmax: Option<f64>,
    pub rested: Option<f64>,
    pub money: Option<f64>,
    pub zone: Option<String>,
    pub subzone: Option<String>,
    #[serde(default)]
    pub bars: HashMap<String, Vec<Slot>>,
    #[serde(default)]
    pub quests: Vec<Quest>,
}

/// The newest PULPIT.lua under the account folders.
pub fn find(install: &Path) -> Option<PathBuf> {
    std::fs::read_dir(install.join("WTF/Account"))
        .ok()?
        .flatten()
        .map(|e| e.path().join("SavedVariables/PULPIT.lua"))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

pub fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// The snapshot in a saved-variables file's text.
pub fn parse(text: &str) -> Option<Snapshot> {
    let start = text.find("[\"json\"] = \"")? + "[\"json\"] = \"".len();
    let json = unescape(&text[start..])?;
    serde_json::from_str(&json).ok()
}

/// A Lua string body up to its closing quote, unescaped.
fn unescape(s: &str) -> Option<String> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => return String::from_utf8(out).ok(),
            b'\\' => {
                i += 1;
                let c = *b.get(i)?;
                match c {
                    b'n' | b'\n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'0'..=b'9' => {
                        let mut n = 0u32;
                        let mut k = 0;
                        while k < 3 && b.get(i).is_some_and(u8::is_ascii_digit) {
                            n = n * 10 + (b[i] - b'0') as u32;
                            i += 1;
                            k += 1;
                        }
                        out.push(n as u8);
                        continue;
                    }
                    other => out.push(other),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_save() {
        let text = r#"
PulpitDB = {
["made"] = {
["MULTIACTIONBAR1BUTTON10"] = "CTRL-0",
},
["json"] = "{\"level\":23,\"zone\":\"Elwynn Forest\",\"bars\":{\"MULTIACTIONBAR1BUTTON\":[{\"kind\":\"spell\",\"id\":35395,\"icon\":135891,\"name\":\"Crusader Strike\",\"cd\":6,\"bind\":\"CTRL-1\"},{\"empty\":true}]},\"quests\":[{\"id\":33,\"title\":\"Wolves \\\"Across\\\" the Border\",\"objectives\":[{\"text\":\"Tough Wolf Meat: 4/8\",\"done\":false}]}]}",
}
"#;
        let s = parse(text).expect("parsed");
        assert_eq!(s.level, Some(23));
        let bar = &s.bars["MULTIACTIONBAR1BUTTON"];
        assert_eq!(bar[0].icon_id(), Some(135891));
        assert!(bar[1].empty);
        assert_eq!(s.quests[0].title.as_deref(), Some("Wolves \"Across\" the Border"));
    }
}

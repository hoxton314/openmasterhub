//! Live surface: clients that take over controls (see `masterhub_core::ipc`). This module only
//! keeps the claims and decides who owns which control; the controller applies the result.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use masterhub_core::config::glob_match;
use masterhub_core::ipc::{Controls, LedZone, TARGET_DIAL};
use masterhub_proto::{LedMode, Rgb};

/// One control that can be owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Control {
    Key(u8),
    Dial,
    Knob(u8),
    Roller(u8),
    Fader(u8),
    Led(LedZone),
}

impl Control {
    /// The control behind an input event (`InputMsg::control` / `index`).
    pub fn of_input(control: &str, index: u8) -> Option<Self> {
        Some(match control {
            "key" => Self::Key(index),
            "dial" => Self::Dial,
            "knob" => Self::Knob(index),
            "roller" => Self::Roller(index),
            "fader" => Self::Fader(index),
            _ => return None,
        })
    }

    /// The control behind a display target (key 0..=14 or the dial).
    pub fn of_target(target: u8) -> Self {
        if target == TARGET_DIAL { Self::Dial } else { Self::Key(target) }
    }
}

/// Expand claimed controls into single controls.
pub fn expand(c: &Controls) -> BTreeSet<Control> {
    let mut v = BTreeSet::new();
    v.extend(c.keys.iter().map(|&k| Control::Key(k)));
    if c.dial {
        v.insert(Control::Dial);
    }
    v.extend(c.knobs.iter().map(|&k| Control::Knob(k)));
    v.extend(c.rollers.iter().map(|&k| Control::Roller(k)));
    v.extend(c.faders.iter().map(|&k| Control::Fader(k)));
    v.extend(c.leds.iter().map(|&z| Control::Led(z)));
    v
}

/// The inverse of [`expand`].
pub fn collect(set: &BTreeSet<Control>) -> Controls {
    let mut c = Controls::default();
    for ctl in set {
        match *ctl {
            Control::Key(k) => c.keys.push(k),
            Control::Dial => c.dial = true,
            Control::Knob(k) => c.knobs.push(k),
            Control::Roller(k) => c.rollers.push(k),
            Control::Fader(k) => c.faders.push(k),
            Control::Led(z) => c.leds.push(z),
        }
    }
    c
}

/// An LED as a client set it; `None` fields were never set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LedState {
    pub mode: Option<LedMode>,
    pub color: Option<Rgb>,
    pub brightness: Option<u8>,
}

pub fn parse_led_mode(s: &str) -> Option<LedMode> {
    Some(match s.to_ascii_lowercase().as_str() {
        "off" => LedMode::Off,
        "static" => LedMode::Static,
        "breathing" => LedMode::Breathing,
        "strobe" => LedMode::Strobe,
        "spring" => LedMode::Spring,
        "cycling" => LedMode::Cycling,
        "random" => LedMode::Random,
        "wave" => LedMode::Wave,
        "water" => LedMode::Water,
        "rainbow" => LedMode::Rainbow,
        _ => return None,
    })
}

pub struct Claim {
    pub name: String,
    pub controls: BTreeSet<Control>,
    pub apps: Vec<String>,
    pub priority: i32,
    /// Last frame per display target.
    pub frames: HashMap<u8, Arc<Vec<u8>>>,
    pub leds: HashMap<LedZone, LedState>,
}

impl Claim {
    /// A pattern is a class glob, or `class@title` (both globs) for apps that share a class,
    /// e.g. games started through one launcher (`steam_app_battlenet@World of Warcraft`).
    fn active(&self, app: Option<&str>, title: Option<&str>) -> bool {
        self.apps.is_empty()
            || app.is_some_and(|a| {
                self.apps.iter().any(|p| match p.split_once('@') {
                    Some((class, t)) => glob_match(class, a) && glob_match(t, title.unwrap_or_default()),
                    None => glob_match(p, a),
                })
            })
    }
}

#[derive(Default)]
pub struct Claims {
    /// By id; ids grow, so a higher id is a newer claim.
    claims: BTreeMap<u64, Claim>,
    owners: HashMap<Control, u64>,
    active: BTreeSet<u64>,
}

/// What changed in [`Claims::recompute`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// Controls whose owner changed (to another claim or back to the config).
    pub controls: BTreeSet<Control>,
    /// Claims whose `claim` event is out of date.
    pub claims: BTreeSet<u64>,
}

impl Claims {
    pub fn insert(&mut self, id: u64, claim: Claim) {
        self.claims.insert(id, claim);
    }

    pub fn remove(&mut self, id: u64) -> bool {
        self.claims.remove(&id).is_some()
    }

    pub fn get(&self, id: u64) -> Option<&Claim> {
        self.claims.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Claim> {
        self.claims.get_mut(&id)
    }

    pub fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.claims.keys().copied()
    }

    pub fn owner(&self, c: Control) -> Option<u64> {
        self.owners.get(&c).copied()
    }

    pub fn is_active(&self, id: u64) -> bool {
        self.active.contains(&id)
    }

    /// What claim `id` owns now.
    pub fn owned_by(&self, id: u64) -> BTreeSet<Control> {
        self.owners.iter().filter(|(_, o)| **o == id).map(|(c, _)| *c).collect()
    }

    /// Decide again who owns what for the focused `app` (window class) and its `title`.
    pub fn recompute(&mut self, app: Option<&str>, title: Option<&str>) -> Changes {
        let active: BTreeSet<u64> = self.claims.iter().filter(|(_, c)| c.active(app, title)).map(|(id, _)| *id).collect();
        let mut owners: HashMap<Control, u64> = HashMap::new();
        for &id in &active {
            let claim = &self.claims[&id];
            for &ctl in &claim.controls {
                let wins = match owners.get(&ctl) {
                    // Ids ascend, so on equal priority the newer claim wins.
                    Some(other) => claim.priority >= self.claims[other].priority,
                    None => true,
                };
                if wins {
                    owners.insert(ctl, id);
                }
            }
        }
        let mut changes = Changes::default();
        for ctl in owners.keys().chain(self.owners.keys()) {
            let (old, new) = (self.owners.get(ctl), owners.get(ctl));
            if old != new {
                changes.controls.insert(*ctl);
                changes.claims.extend(old.into_iter().chain(new).copied().filter(|id| self.claims.contains_key(id)));
            }
        }
        changes.claims.extend(active.symmetric_difference(&self.active).copied().filter(|id| self.claims.contains_key(id)));
        self.owners = owners;
        self.active = active;
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(keys: &[u8], apps: &[&str], priority: i32) -> Claim {
        Claim {
            name: "t".into(),
            controls: keys.iter().map(|&k| Control::Key(k)).collect(),
            apps: apps.iter().map(|s| s.to_string()).collect(),
            priority,
            frames: HashMap::new(),
            leds: HashMap::new(),
        }
    }

    #[test]
    fn focus_and_priority_decide_the_owner() {
        let mut c = Claims::default();
        c.insert(1, claim(&[0, 1], &[], 0));
        c.insert(2, claim(&[1, 2], &["cacaw"], 5));
        let ch = c.recompute(Some("firefox"), None);
        assert_eq!(c.owner(Control::Key(1)), Some(1));
        assert_eq!(c.owner(Control::Key(2)), None);
        assert!(ch.controls.contains(&Control::Key(0)));
        assert!(!c.is_active(2));

        let ch = c.recompute(Some("CACAW"), None);
        assert_eq!(c.owner(Control::Key(1)), Some(2));
        assert_eq!(c.owner(Control::Key(0)), Some(1));
        assert_eq!(ch.controls, [Control::Key(1), Control::Key(2)].into());
        assert_eq!(ch.claims, [1, 2].into());
    }

    #[test]
    fn newer_claim_wins_a_tie_and_release_gives_back() {
        let mut c = Claims::default();
        c.insert(1, claim(&[3], &[], 0));
        c.insert(2, claim(&[3], &[], 0));
        c.recompute(None, None);
        assert_eq!(c.owner(Control::Key(3)), Some(2));
        c.remove(2);
        let ch = c.recompute(None, None);
        assert_eq!(c.owner(Control::Key(3)), Some(1));
        assert_eq!(ch.claims, [1].into());
        c.remove(1);
        let ch = c.recompute(None, None);
        assert_eq!(c.owner(Control::Key(3)), None);
        assert!(ch.controls.contains(&Control::Key(3)));
        assert!(ch.claims.is_empty());
    }

    #[test]
    fn controls_round_trip() {
        let c = Controls { keys: vec![0, 4], dial: true, knobs: vec![1], leds: vec![LedZone::Base], ..Default::default() };
        assert_eq!(collect(&expand(&c)), c);
    }

    #[test]
    fn class_and_title() {
        let mut c = Claim { name: "wow".into(), controls: Default::default(), apps: vec!["steam_app_battlenet@World of Warcraft".into()], priority: 5, frames: Default::default(), leds: Default::default() };
        assert!(c.active(Some("steam_app_battlenet"), Some("World of Warcraft")));
        assert!(!c.active(Some("steam_app_battlenet"), Some("Battle.net")));
        assert!(!c.active(Some("kitty"), Some("World of Warcraft")));
        c.apps = vec!["steam_app_*".into()];
        assert!(c.active(Some("steam_app_battlenet"), None));
    }
}

//! Volume and mute of output and input devices and of apps, for `volume`/`mute` actions and the
//! volume widget. Linux: the PulseAudio protocol, served by PulseAudio or `pipewire-pulse`.
//! Windows: Core Audio (`IAudioEndpointVolume`, `ISimpleAudioVolume` per audio session).
//!
//! A fader that sets an app's level is remembered, so apps that start playing later (a game
//! launched after the fader moved) get the same level and mute state.

use std::collections::HashSet;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};
use masterhub_core::config::{AudioGroup, AudioTarget, Mute, VolumeChange, glob_match};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;
#[cfg(windows)]
pub use self::windows::com_init;

/// An output or input device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub input: bool,
    /// Name or description (`*` wildcards); `None` is the default device.
    pub name: Option<String>,
}

/// Apps matched by name, or by having been started by Steam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Apps {
    pub patterns: Vec<String>,
    pub steam: bool,
}

/// What a volume or mute action controls, with groups resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Device(Device),
    Apps(Apps),
}

impl Target {
    /// Resolve a configured target; `None` for an unknown group.
    pub fn resolve(target: &AudioTarget, groups: &[AudioGroup]) -> Option<Self> {
        let device = |input, name: Option<&String>| Self::Device(Device { input, name: name.cloned() });
        Some(match target {
            AudioTarget::Output => device(false, None),
            AudioTarget::OutputDevice(d) => device(false, Some(d)),
            AudioTarget::Input => device(true, None),
            AudioTarget::InputDevice(d) => device(true, Some(d)),
            AudioTarget::App(a) => Self::Apps(Apps { patterns: vec![a.clone()], steam: false }),
            AudioTarget::Group(name) => {
                let g = groups.iter().find(|g| &g.name == name)?;
                Self::Apps(Apps { patterns: g.apps.clone(), steam: g.steam })
            }
        })
    }
}

/// An app's audio stream (Linux) or session (Windows).
pub struct Stream {
    /// Stable for the stream's lifetime.
    pub key: String,
    /// Program and app names it may be matched by, e.g. `["Discord", "WEBRTC VoiceEngine"]`.
    pub names: Vec<String>,
    pub pid: Option<u32>,
    pub level: f32,
    pub muted: bool,
    handle: imp::StreamHandle,
}

impl Apps {
    fn matches(&self, s: &Stream) -> bool {
        let named = self.patterns.iter().any(|p| s.names.iter().any(|n| name_matches(p, n)));
        named || (self.steam && s.pid.is_some_and(started_by_steam))
    }
}

/// Case-insensitive glob match, ignoring `.exe`, and for reverse-DNS app ids
/// (`com.spotify.Client`, `org.mozilla.firefox`) also against their parts after the first.
fn name_matches(pattern: &str, name: &str) -> bool {
    let strip = |s: &str| {
        let lower = s.to_ascii_lowercase();
        lower.strip_suffix(".exe").map(str::to_owned).unwrap_or(lower)
    };
    let (p, n) = (strip(pattern), strip(name));
    glob_match(&p, &n) || (n.matches('.').count() >= 2 && n.split('.').skip(1).any(|part| glob_match(&p, part)))
}

/// Current level (0.0..=1.0) and mute state of the default output.
pub fn get() -> Option<(f32, bool)> {
    imp::device_get(&Device { input: false, name: None }).ok()
}

pub fn set_volume(target: &Target, change: VolumeChange) -> Result<()> {
    match target {
        Target::Device(d) => imp::device_set_volume(d, change),
        Target::Apps(apps) => {
            if let VolumeChange::To(v) = change {
                remember(apps, |r| r.level = Some(v as f32 / 100.0));
            }
            for s in imp::streams()?.iter().filter(|s| apps.matches(s)) {
                imp::stream_set_volume(s, change.apply(s.level))?;
            }
            Ok(())
        }
    }
}

pub fn set_mute(target: &Target, mute: Mute) -> Result<()> {
    match target {
        Target::Device(d) => imp::device_set_mute(d, mute),
        Target::Apps(apps) => {
            let streams: Vec<Stream> = imp::streams()?.into_iter().filter(|s| apps.matches(s)).collect();
            let on = match mute {
                // Unmute when everything is muted, else mute everything.
                Mute::Toggle if streams.is_empty() => !remembered_mute(apps),
                Mute::Toggle => !streams.iter().all(|s| s.muted),
                Mute::On => true,
                Mute::Off => false,
            };
            remember(apps, |r| r.muted = Some(on));
            for s in &streams {
                imp::stream_set_mute(s, on)?;
            }
            Ok(())
        }
    }
}

/// Apps playing audio now and the devices, as suggestions for targets. Parts the sound server
/// cannot list stay empty.
pub fn info() -> masterhub_core::ipc::AudioInfo {
    #[cfg(windows)]
    com_init();
    let mut apps: Vec<String> = imp::streams().unwrap_or_default().into_iter().filter_map(|s| s.names.into_iter().next()).collect();
    apps.sort_by_key(|n| n.to_lowercase());
    apps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    let devices = |input| {
        imp::devices(input).unwrap_or_else(|e| {
            log::debug!("audio devices: {e:#}");
            Vec::new()
        })
    };
    masterhub_core::ipc::AudioInfo { apps, outputs: devices(false), inputs: devices(true) }
}

struct Remembered {
    apps: Apps,
    level: Option<f32>,
    muted: Option<bool>,
}

static REMEMBERED: Mutex<Vec<Remembered>> = Mutex::new(Vec::new());

fn remember(apps: &Apps, update: impl FnOnce(&mut Remembered)) {
    let mut list = REMEMBERED.lock().unwrap_or_else(|e| e.into_inner());
    let i = match list.iter().position(|r| &r.apps == apps) {
        Some(i) => i,
        None => {
            list.push(Remembered { apps: apps.clone(), level: None, muted: None });
            list.len() - 1
        }
    };
    update(&mut list[i]);
}

fn remembered_mute(apps: &Apps) -> bool {
    let list = REMEMBERED.lock().unwrap_or_else(|e| e.into_inner());
    list.iter().find(|r| &r.apps == apps).and_then(|r| r.muted).unwrap_or(false)
}

/// Watch for new app streams and give them the remembered level and mute state of the faders
/// and keys that match them.
pub fn spawn_watcher() {
    let spawned = thread::Builder::new().name("audio".into()).spawn(|| {
        #[cfg(windows)]
        com_init();
        let mut seen: HashSet<String> = HashSet::new();
        let mut failing = false;
        loop {
            thread::sleep(Duration::from_millis(500));
            if REMEMBERED.lock().map(|l| l.is_empty()).unwrap_or(true) {
                continue;
            }
            let streams = match imp::streams() {
                Ok(s) => s,
                Err(e) => {
                    if !std::mem::replace(&mut failing, true) {
                        log::warn!("audio streams: {e:#}");
                    }
                    continue;
                }
            };
            failing = false;
            for s in streams.iter().filter(|s| !seen.contains(&s.key)) {
                if let Err(e) = apply_remembered(s) {
                    log::debug!("restoring {:?}: {e:#}", s.names);
                }
            }
            seen = streams.into_iter().map(|s| s.key).collect();
        }
    });
    if let Err(e) = spawned {
        log::error!("spawning audio watcher: {e}");
    }
}

fn apply_remembered(s: &Stream) -> Result<()> {
    let (mut level, mut muted) = (None, None);
    for r in REMEMBERED.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|r| r.apps.matches(s)) {
        level = r.level.or(level);
        muted = r.muted.or(muted);
    }
    if let Some(l) = level {
        log::debug!("{:?} started; volume {:.0}%", s.names, l * 100.0);
        imp::stream_set_volume(s, l)?;
    }
    if let Some(m) = muted {
        imp::stream_set_mute(s, m)?;
    }
    Ok(())
}

/// Whether a process was started by Steam: a game, Proton, or a launcher Steam ran. Not the
/// Steam client itself, whose helpers (`steamwebhelper`) play sounds too.
fn started_by_steam(pid: u32) -> bool {
    let is_steam = |name: &str, exact: bool| {
        let name = name.to_ascii_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        if exact { name == "steam" } else { name.starts_with("steam") }
    };
    let Some((name, mut parent)) = imp::process(pid) else { return false };
    if is_steam(&name, false) {
        return false;
    }
    #[cfg(target_os = "linux")]
    if linux::has_steam_game_id(pid) {
        return true;
    }
    for _ in 0..32 {
        let Some(p) = parent.filter(|p| *p > 1) else { return false };
        let Some((name, next)) = imp::process(p) else { return false };
        if is_steam(&name, true) {
            return true;
        }
        parent = next.filter(|n| *n != p);
    }
    false
}

/// The first device whose name or description matches `pattern`.
fn pick<'a, T>(items: &'a [T], pattern: &str, names: impl Fn(&T) -> [&str; 2]) -> Result<&'a T> {
    match items.iter().find(|d| names(d).iter().any(|n| glob_match(pattern, n))) {
        Some(d) => Ok(d),
        None => bail!("no audio device matches {pattern:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_app_names() {
        assert!(name_matches("discord", "Discord"));
        assert!(name_matches("discord", "Discord.exe"));
        assert!(name_matches("Discord.exe", "discord"));
        assert!(name_matches("spotify", "com.spotify.Client"));
        assert!(name_matches("com.spotify.client", "com.spotify.Client"));
        assert!(name_matches("discord", "com.discordapp.Discord"));
        assert!(name_matches("firefox", "org.mozilla.firefox"));
        assert!(!name_matches("org", "org.mozilla.firefox"));
        assert!(name_matches("*chrom*", "Google Chrome"));
        assert!(!name_matches("spotify", "spotifyd"));
        assert!(!name_matches("client", "my.client"));
    }

    #[test]
    fn resolves_groups() {
        let groups = vec![AudioGroup { name: "Games".into(), apps: vec!["cs2".into()], steam: true }];
        let t = Target::resolve(&AudioTarget::Group("Games".into()), &groups).unwrap();
        assert_eq!(t, Target::Apps(Apps { patterns: vec!["cs2".into()], steam: true }));
        assert_eq!(Target::resolve(&AudioTarget::Group("nope".into()), &groups), None);
        assert_eq!(Target::resolve(&AudioTarget::Input, &[]), Some(Target::Device(Device { input: true, name: None })));
    }
}

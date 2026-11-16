//! The PulseAudio native protocol over its Unix socket, served by PulseAudio or by
//! `pipewire-pulse`, so the same code covers PipeWire and PulseAudio distros. X11 or Wayland
//! makes no difference here.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use anyhow::{Context, Result};
use masterhub_core::config::{Mute, VolumeChange};
use pulseaudio::protocol::{
    self, ChannelVolume, ClientInfoList, Command, CommandReply, GetSinkInfo, GetSourceInfo, Props, SetDeviceMuteParams,
    SetDeviceVolumeParams, SetStreamMuteParams, SetStreamVolumeParams, SinkInfo, SinkInfoList, SinkInputInfoList,
    SourceInfo, SourceInfoList, Volume,
};

use super::{Device, Stream, pick};

/// A sink input's index and channel count.
pub struct StreamHandle {
    index: u32,
    channels: u8,
}

struct Conn {
    sock: BufReader<UnixStream>,
    version: u16,
    seq: u32,
}

impl Conn {
    fn open() -> Result<Self> {
        let path = pulseaudio::socket_path_from_env().context("no PulseAudio or pipewire-pulse socket")?;
        let sock = UnixStream::connect(&path).with_context(|| format!("connecting to {}", path.display()))?;
        sock.set_read_timeout(Some(Duration::from_secs(2)))?;
        sock.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut c = Self { sock: BufReader::new(sock), version: protocol::MAX_VERSION, seq: 0 };
        let cookie = pulseaudio::cookie_path_from_env().and_then(|p| std::fs::read(p).ok()).unwrap_or_default();
        let auth = protocol::AuthParams { version: protocol::MAX_VERSION, supports_shm: false, supports_memfd: false, cookie };
        let reply: protocol::AuthReply = c.request(Command::Auth(auth)).context("PulseAudio auth")?;
        c.version = c.version.min(reply.version);
        let mut props = Props::new();
        props.set(protocol::Prop::ApplicationName, c"masterhubd");
        let _: protocol::SetClientNameReply = c.request(Command::SetClientName(props))?;
        Ok(c)
    }

    fn send(&mut self, cmd: &Command) -> Result<()> {
        self.seq = self.seq.wrapping_add(1);
        protocol::write_command_message(self.sock.get_mut(), self.seq, cmd, self.version)?;
        Ok(())
    }

    fn request<T: CommandReply>(&mut self, cmd: Command) -> Result<T> {
        self.send(&cmd)?;
        Ok(protocol::read_reply_message::<T>(&mut self.sock, self.version)?.1)
    }

    fn ack(&mut self, cmd: Command) -> Result<()> {
        self.send(&cmd)?;
        protocol::read_ack_message(&mut self.sock)?;
        Ok(())
    }
}

thread_local! {
    static CONN: RefCell<Option<Conn>> = const { RefCell::new(None) };
}

/// Run `f` on this thread's connection, reconnecting once if it fails (the sound server may
/// have restarted).
fn with<R>(mut f: impl FnMut(&mut Conn) -> Result<R>) -> Result<R> {
    CONN.with_borrow_mut(|slot| {
        if let Some(c) = slot.as_mut()
            && let Ok(r) = f(c)
        {
            return Ok(r);
        }
        *slot = None;
        let c = slot.insert(Conn::open()?);
        f(c)
    })
}

fn level(v: &ChannelVolume) -> f32 {
    v.channels().iter().map(|c| c.as_u32()).max().unwrap_or(0) as f32 / Volume::NORM.as_u32() as f32
}

fn volume(level: f32, channels: usize) -> ChannelVolume {
    let raw = (level.clamp(0.0, 1.0) * Volume::NORM.as_u32() as f32).round() as u32;
    let mut v = ChannelVolume::empty();
    for _ in 0..channels.max(1) {
        v.push(Volume::from_u32_clamped(raw));
    }
    v
}

fn text(s: &CStr) -> String {
    s.to_string_lossy().into_owned()
}

fn prop(props: &Props, key: &str) -> Option<String> {
    props
        .iter()
        .find(|(k, _)| k.to_bytes() == key.as_bytes())
        .map(|(_, v)| String::from_utf8_lossy(v.strip_suffix(&[0]).unwrap_or(v)).into_owned())
        .filter(|s| !s.is_empty())
}

/// The name, volume, mute state and channel count of a device.
struct DeviceInfo {
    name: CString,
    cvolume: ChannelVolume,
    muted: bool,
}

impl From<SinkInfo> for DeviceInfo {
    fn from(s: SinkInfo) -> Self {
        Self { name: s.name, cvolume: s.cvolume, muted: s.muted }
    }
}

impl From<SourceInfo> for DeviceInfo {
    fn from(s: SourceInfo) -> Self {
        Self { name: s.name, cvolume: s.cvolume, muted: s.muted }
    }
}

fn device(d: &Device) -> Result<DeviceInfo> {
    with(|c| {
        Ok(match (&d.name, d.input) {
            (None, false) => {
                c.request::<SinkInfo>(Command::GetSinkInfo(GetSinkInfo { index: None, name: Some(c"@DEFAULT_SINK@".into()) }))?.into()
            }
            (None, true) => c
                .request::<SourceInfo>(Command::GetSourceInfo(GetSourceInfo { index: None, name: Some(c"@DEFAULT_SOURCE@".into()) }))?
                .into(),
            (Some(pattern), false) => {
                let sinks: SinkInfoList = c.request(Command::GetSinkInfoList)?;
                pick(&sinks, pattern, |s| [s.name.to_str().unwrap_or(""), s.description.as_deref().and_then(|d| d.to_str().ok()).unwrap_or("")])?
                    .clone()
                    .into()
            }
            (Some(pattern), true) => {
                let sources: Vec<SourceInfo> = inputs(c)?;
                pick(&sources, pattern, |s| [s.name.to_str().unwrap_or(""), s.description.as_deref().and_then(|d| d.to_str().ok()).unwrap_or("")])?
                    .clone()
                    .into()
            }
        })
    })
}

/// Sources without the monitors of the outputs.
fn inputs(c: &mut Conn) -> Result<Vec<SourceInfo>> {
    let mut sources: SourceInfoList = c.request(Command::GetSourceInfoList)?;
    sources.retain(|s| s.monitor_of_sink_index.is_none());
    Ok(sources)
}

pub fn device_get(d: &Device) -> Result<(f32, bool)> {
    let info = device(d)?;
    Ok((level(&info.cvolume).min(1.0), info.muted))
}

pub fn device_set_volume(d: &Device, change: VolumeChange) -> Result<()> {
    let info = device(d)?;
    let params = SetDeviceVolumeParams {
        device_index: None,
        device_name: Some(info.name),
        volume: volume(change.apply(level(&info.cvolume)), info.cvolume.channels().len()),
    };
    with(|c| c.ack(if d.input { Command::SetSourceVolume(params.clone()) } else { Command::SetSinkVolume(params.clone()) }))
}

pub fn device_set_mute(d: &Device, mute: Mute) -> Result<()> {
    let info = device(d)?;
    let mute = match mute {
        Mute::Toggle => !info.muted,
        Mute::On => true,
        Mute::Off => false,
    };
    let params = SetDeviceMuteParams { device_index: None, device_name: Some(info.name), mute };
    with(|c| c.ack(if d.input { Command::SetSourceMute(params.clone()) } else { Command::SetSinkMute(params.clone()) }))
}

pub fn devices(input: bool) -> Result<Vec<String>> {
    with(|c| {
        let descriptions: Vec<Option<CString>> = if input {
            inputs(c)?.into_iter().map(|s| s.description.or(Some(s.name))).collect()
        } else {
            let sinks: SinkInfoList = c.request(Command::GetSinkInfoList)?;
            sinks.into_iter().map(|s| s.description.or(Some(s.name))).collect()
        };
        Ok(descriptions.into_iter().flatten().map(|d| text(&d)).collect())
    })
}

pub fn streams() -> Result<Vec<Stream>> {
    with(|c| {
        let inputs: SinkInputInfoList = c.request(Command::GetSinkInputInfoList)?;
        let clients: ClientInfoList = c.request(Command::GetClientInfoList)?;
        Ok(inputs
            .into_iter()
            .filter(|s| s.has_volume && s.volume_writable)
            .map(|s| {
                let client = s.client_index.and_then(|i| clients.iter().find(|c| c.index == i));
                // The app reports its own pid, which is wrong inside a pid namespace (Steam
                // Runtime, Flatpak); PipeWire's comes from the socket but belongs to PipeWire
                // itself for native PipeWire clients. Take the one running the app's binary.
                let binary = prop(&s.props, "application.process.binary");
                let pids: Vec<u32> = [prop(&s.props, "application.process.id"), client.and_then(|c| prop(&c.props, "pipewire.sec.pid"))]
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p.parse().ok())
                    .collect();
                let runs_binary = |pid: &&u32| {
                    // `comm` is cut to 15 bytes.
                    let comm = process(**pid).map(|(name, _)| name).unwrap_or_default();
                    binary.as_deref().is_some_and(|b| !comm.is_empty() && b.starts_with(&comm))
                };
                let pid = pids.iter().find(runs_binary).or(pids.first()).copied();
                let names = ["application.process.binary", "application.name", "pipewire.access.portal.app_id", "application.id"]
                    .iter()
                    .filter_map(|k| prop(&s.props, k).or_else(|| client.and_then(|c| prop(&c.props, k))))
                    .collect();
                Stream {
                    key: s.index.to_string(),
                    names,
                    pid,
                    level: level(&s.cvolume).min(1.0),
                    muted: s.muted,
                    handle: StreamHandle { index: s.index, channels: s.cvolume.channels().len() as u8 },
                }
            })
            .collect())
    })
}

pub fn stream_set_volume(s: &Stream, level: f32) -> Result<()> {
    let params = SetStreamVolumeParams { index: s.handle.index, volume: volume(level, s.handle.channels as usize) };
    with(|c| c.ack(Command::SetSinkInputVolume(params)))
}

pub fn stream_set_mute(s: &Stream, mute: bool) -> Result<()> {
    with(|c| c.ack(Command::SetSinkInputMute(SetStreamMuteParams { index: s.handle.index, mute })))
}

/// A process's name (`comm`) and parent.
pub fn process(pid: u32) -> Option<(String, Option<u32>)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // "pid (comm) state ppid ...", where comm may itself contain parentheses.
    let (head, rest) = stat.rsplit_once(')')?;
    let name = head.split_once('(')?.1.to_owned();
    Some((name, rest.split_whitespace().nth(1).and_then(|p| p.parse().ok())))
}

/// Steam sets `SteamGameId` for everything it launches, also inside the Steam Runtime
/// container, where the process tree no longer leads back to Steam.
pub fn has_steam_game_id(pid: u32) -> bool {
    let Ok(env) = std::fs::read(format!("/proc/{pid}/environ")) else { return false };
    env.split(|b| *b == 0).any(|var| {
        var.strip_prefix(b"SteamGameId=").or_else(|| var.strip_prefix(b"SteamAppId=")).is_some_and(|id| !id.is_empty() && id != b"0")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a running sound server: `cargo test -p masterhubd -- --ignored live_`.
    #[test]
    #[ignore]
    fn live_lists_and_reads() {
        let out = device_get(&Device { input: false, name: None }).unwrap();
        let mic = device_get(&Device { input: true, name: None }).unwrap();
        eprintln!("output {out:?}, input {mic:?}");
        eprintln!("info {:?}", super::super::info());
        for s in streams().unwrap() {
            eprintln!("stream {} {:?} pid {:?} level {} muted {}", s.key, s.names, s.pid, s.level, s.muted);
        }
    }

    /// Sets the first stream to 50 % and mutes it, then restores it.
    #[test]
    #[ignore]
    fn live_sets_a_stream() {
        let s = streams().unwrap().into_iter().next().expect("an app playing audio");
        let apps = super::super::Apps { patterns: vec![s.names[0].clone()], steam: false };
        let t = super::super::Target::Apps(apps);
        super::super::set_volume(&t, VolumeChange::To(50)).unwrap();
        super::super::set_mute(&t, Mute::Toggle).unwrap();
        let now = streams().unwrap().into_iter().find(|x| x.key == s.key).unwrap();
        eprintln!("now {} muted {}", now.level, now.muted);
        let shown = std::process::Command::new("pactl").args(["list", "sink-inputs"]).output().unwrap();
        let shown = String::from_utf8_lossy(&shown.stdout);
        eprintln!("{}", shown.lines().filter(|l| l.contains("Volume:") || l.contains("Mute:")).collect::<Vec<_>>().join("\n"));
        stream_set_volume(&now, s.level).unwrap();
        stream_set_mute(&now, s.muted).unwrap();
        assert!((now.level - 0.5).abs() < 0.01);
        assert_ne!(now.muted, s.muted);
    }
}

//! Live data for widgets. Each value is computed on demand, at most once per tick, and only
//! for widgets currently on screen.

use std::collections::HashMap;
#[cfg(not(windows))]
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;

use image::RgbaImage;
use masterhub_core::config::Widget;
use masterhub_core::render::LiveData;
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

use crate::audio;

#[cfg(windows)]
mod windows;

pub struct Stats {
    cpu_prev: Option<(u64, u64)>,
    nvml: Option<Nvml>,
    #[cfg(not(windows))]
    cpu_temp_path: Option<PathBuf>,
    #[cfg(not(windows))]
    amd_gpu: Option<PathBuf>,
    art_cache: HashMap<String, Option<RgbaImage>>,
    #[cfg(not(windows))]
    session: Option<zbus::blocking::Connection>,
    #[cfg(windows)]
    media: self::windows::Media,
    #[cfg(windows)]
    sensors: self::windows::Sensors,
}

impl Stats {
    pub fn new() -> Self {
        let nvml = Nvml::init().ok();
        if nvml.is_some() {
            log::info!("NVML available for GPU widgets");
        }
        Self {
            cpu_prev: None,
            nvml,
            #[cfg(not(windows))]
            cpu_temp_path: find_cpu_temp(),
            #[cfg(not(windows))]
            amd_gpu: find_amd_gpu(),
            art_cache: HashMap::new(),
            #[cfg(not(windows))]
            session: zbus::blocking::Connection::session().ok(),
            #[cfg(windows)]
            media: self::windows::Media::new(),
            #[cfg(windows)]
            sensors: self::windows::Sensors::new(),
        }
    }

    /// Sample all widgets in `needed` once. Missing data (e.g. no GPU) gives `None`.
    pub fn sample(&mut self, needed: &[Widget]) -> HashMap<Widget, LiveData> {
        let mut out = HashMap::new();
        for w in needed {
            if out.contains_key(w) {
                continue;
            }
            if let Some(v) = self.one(w) {
                out.insert(w.clone(), v);
            }
        }
        out
    }

    fn one(&mut self, w: &Widget) -> Option<LiveData> {
        Some(match w {
            Widget::Clock { format } => {
                let now = jiff::Zoned::now();
                let fmt = format.as_deref().unwrap_or("%H:%M");
                LiveData::Text { text: now.strftime(fmt).to_string(), caption: Some(now.strftime("%a %d").to_string()) }
            }
            Widget::CpuUsage => {
                let f = self.cpu_usage()?;
                gauge(f, format!("{:.0}%", f * 100.0), "CPU")
            }
            Widget::RamUsage => {
                let f = ram_usage()?;
                gauge(f, format!("{:.0}%", f * 100.0), "RAM")
            }
            Widget::CpuTemp => {
                let t = self.cpu_temp()?;
                gauge(t / 100.0, format!("{t:.0}°"), "CPU")
            }
            Widget::GpuUsage => {
                let f = self.gpu_usage()?;
                gauge(f, format!("{:.0}%", f * 100.0), "GPU")
            }
            Widget::GpuTemp => {
                let t = self.gpu_temp()?;
                gauge(t / 100.0, format!("{t:.0}°"), "GPU")
            }
            Widget::Volume => {
                let (v, muted) = audio::get()?;
                let text = if muted { "Muted".to_owned() } else { format!("{:.0}%", v * 100.0) };
                gauge(if muted { 0.0 } else { v }, text, "Volume")
            }
            Widget::NowPlaying => self.now_playing()?,
        })
    }

    fn cpu_usage(&mut self) -> Option<f32> {
        let (idle, total) = cpu_times()?;
        let prev = self.cpu_prev.replace((idle, total));
        let (pi, pt) = prev?;
        let dt = total.saturating_sub(pt);
        (dt > 0).then(|| 1.0 - idle.saturating_sub(pi) as f32 / dt as f32)
    }

    fn gpu_usage(&mut self) -> Option<f32> {
        if let Some(nvml) = &self.nvml {
            return Some(nvml.device_by_index(0).ok()?.utilization_rates().ok()?.gpu as f32 / 100.0);
        }
        #[cfg(windows)]
        {
            self.sensors.read(self::windows::Sensor::GpuLoad)
        }
        #[cfg(not(windows))]
        {
            let s = std::fs::read_to_string(self.amd_gpu.as_ref()?.join("gpu_busy_percent")).ok()?;
            Some(s.trim().parse::<f32>().ok()? / 100.0)
        }
    }

    fn gpu_temp(&mut self) -> Option<f32> {
        if let Some(nvml) = &self.nvml {
            return Some(nvml.device_by_index(0).ok()?.temperature(TemperatureSensor::Gpu).ok()? as f32);
        }
        #[cfg(windows)]
        {
            self.sensors.read(self::windows::Sensor::GpuTemp)
        }
        #[cfg(not(windows))]
        {
            let hwmon = std::fs::read_dir(self.amd_gpu.as_ref()?.join("hwmon")).ok()?.flatten().next()?.path();
            read_millideg(&hwmon.join("temp1_input"))
        }
    }

    /// Linux: hwmon. Windows: LibreHardwareMonitor, while it runs.
    fn cpu_temp(&mut self) -> Option<f32> {
        #[cfg(windows)]
        {
            self.sensors.read(self::windows::Sensor::CpuTemp)
        }
        #[cfg(not(windows))]
        {
            read_millideg(self.cpu_temp_path.as_deref()?)
        }
    }

    /// The playing (or most recent) media session, with its cover art.
    #[cfg(windows)]
    fn now_playing(&mut self) -> Option<LiveData> {
        let p = self.media.playing()?;
        if p.title.is_empty() && p.artist.is_empty() {
            return None;
        }
        // Players often set the thumbnail after the title, so its presence is part of the key.
        let key = format!("smtc:{}\n{}\n{}", p.title, p.artist, p.thumbnail.is_some());
        let thumbnail = p.thumbnail;
        let art = self.art_cached(&key, || image::load_from_memory(&self::windows::read_thumbnail(thumbnail.as_ref()?)?).ok());
        Some(LiveData::Media { title: p.title, artist: p.artist, art })
    }

    #[cfg(not(windows))]
    fn now_playing(&mut self) -> Option<LiveData> {
        let conn = self.session.as_ref()?;
        let dbus = zbus::blocking::fdo::DBusProxy::new(conn).ok()?;
        let players: Vec<String> = dbus
            .list_names()
            .ok()?
            .into_iter()
            .map(|n| n.to_string())
            .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
            .collect();
        let mut best: Option<(bool, HashMap<String, zbus::zvariant::OwnedValue>)> = None;
        for name in players {
            let Ok(proxy) = zbus::blocking::Proxy::new(conn, name.as_str(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
            else {
                continue;
            };
            let playing = proxy.get_property::<String>("PlaybackStatus").is_ok_and(|s| s == "Playing");
            let Ok(meta) = proxy.get_property::<HashMap<String, zbus::zvariant::OwnedValue>>("Metadata") else {
                continue;
            };
            if best.as_ref().is_none_or(|(p, _)| playing && !p) {
                best = Some((playing, meta));
            }
        }
        let (_, meta) = best?;
        let title: String = meta.get("xesam:title").and_then(|v| String::try_from(v.clone()).ok()).unwrap_or_default();
        let artist = meta
            .get("xesam:artist")
            .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
            .map(|a| a.join(", "))
            .unwrap_or_default();
        let art_url = meta.get("mpris:artUrl").and_then(|v| String::try_from(v.clone()).ok());
        let art = art_url.and_then(|u| self.art(&u));
        if title.is_empty() && artist.is_empty() {
            return None;
        }
        Some(LiveData::Media { title, artist, art })
    }

    /// Cover art from `file://` or `http(s)://` (fetched once with curl and cached).
    #[cfg(not(windows))]
    fn art(&mut self, url: &str) -> Option<RgbaImage> {
        self.art_cached(url, || {
            if let Some(path) = url.strip_prefix("file://") {
                image::open(percent_decode(path)).ok()
            } else if url.starts_with("http://") || url.starts_with("https://") {
                Command::new("curl")
                    .args(["-sfL", "--max-time", "3", url])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| image::load_from_memory(&o.stdout).ok())
            } else {
                None
            }
        })
    }

    /// Cover art by `key`, loaded once, scaled to the dial and cached.
    fn art_cached(&mut self, key: &str, load: impl FnOnce() -> Option<image::DynamicImage>) -> Option<RgbaImage> {
        if let Some(cached) = self.art_cache.get(key) {
            return cached.clone();
        }
        let img = load().map(|i| i.resize(240, 240, image::imageops::FilterType::Triangle).to_rgba8());
        if self.art_cache.len() > 32 {
            self.art_cache.clear();
        }
        self.art_cache.insert(key.to_owned(), img.clone());
        img
    }
}

fn gauge(fraction: f32, text: String, caption: &str) -> LiveData {
    LiveData::Gauge { fraction: fraction.clamp(0.0, 1.0), text, caption: caption.to_owned() }
}

/// Cumulative (idle, total) CPU time since boot.
#[cfg(not(windows))]
fn cpu_times() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let nums: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
    let idle = nums.get(3)? + nums.get(4).unwrap_or(&0);
    Some((idle, nums.iter().sum()))
}

#[cfg(windows)]
fn cpu_times() -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: valid out-pointers. Kernel time includes idle time.
    (unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0).then(|| (ft(idle), ft(kernel) + ft(user)))
}

#[cfg(windows)]
fn ram_usage() -> Option<f32> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: the struct is plain data; dwLength must be set before the call.
    let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    (unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then(|| 1.0 - m.ullAvailPhys as f32 / m.ullTotalPhys as f32)
}

#[cfg(not(windows))]
fn ram_usage() -> Option<f32> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let field = |name: &str| -> Option<f32> {
        info.lines().find(|l| l.starts_with(name))?.split_whitespace().nth(1)?.parse().ok()
    };
    let (total, avail) = (field("MemTotal:")?, field("MemAvailable:")?);
    Some(1.0 - avail / total)
}

#[cfg(not(windows))]
fn read_millideg(path: &Path) -> Option<f32> {
    Some(std::fs::read_to_string(path).ok()?.trim().parse::<f32>().ok()? / 1000.0)
}

/// CPU package temperature: k10temp (AMD) Tctl or coretemp (Intel) package.
#[cfg(not(windows))]
fn find_cpu_temp() -> Option<PathBuf> {
    for dir in std::fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let name = std::fs::read_to_string(dir.path().join("name")).unwrap_or_default();
        if matches!(name.trim(), "k10temp" | "coretemp" | "zenpower") {
            return Some(dir.path().join("temp1_input"));
        }
    }
    None
}

#[cfg(not(windows))]
fn find_amd_gpu() -> Option<PathBuf> {
    std::fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|e| e.path().join("device"))
        .find(|d| d.join("gpu_busy_percent").exists())
}

#[cfg(not(windows))]
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn decodes_file_urls() {
        assert_eq!(percent_decode("/tmp/My%20Song.jpg"), "/tmp/My Song.jpg");
    }

    #[test]
    fn cpu_usage_needs_two_samples() {
        let mut s = Stats::new();
        assert!(s.cpu_usage().is_none());
        std::thread::sleep(std::time::Duration::from_millis(50));
        let v = s.cpu_usage().unwrap();
        assert!((0.0..=1.0).contains(&v));
    }
}

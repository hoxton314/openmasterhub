//! Readouts for a game mode's display page: the machine (GPU, CPU) and the music player, polled
//! on their own thread so a slow tool never stalls the keys. Nothing here looks at the game.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Sys {
    /// °C.
    pub gpu_temp: Option<f32>,
    /// Percent.
    pub gpu_load: Option<f32>,
    pub vram_gb: Option<f32>,
    /// Percent of all cores.
    pub cpu: Option<f32>,
    /// What the music player has loaded: (playing, "artist - title").
    pub track: Option<(bool, String)>,
}

fn out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

/// (busy, total) jiffies from the first line of /proc/stat.
fn cpu_times() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let v: Vec<u64> = s.lines().next()?.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    let total: u64 = v.iter().sum();
    // idle + iowait
    Some((total - v.get(3)? - v.get(4).copied().unwrap_or(0), total))
}

/// Start polling every two seconds while `active` is set (the page is on the hub).
pub fn start(active: Arc<AtomicBool>) -> Arc<Mutex<Sys>> {
    let sys = Arc::new(Mutex::new(Sys::default()));
    let shared = sys.clone();
    let spawned = std::thread::Builder::new().name("pulpit-live".into()).spawn(move || {
        let mut last = cpu_times();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if !active.load(Ordering::Relaxed) {
                continue;
            }
            let mut s = Sys::default();
            if let Some(line) = out("nvidia-smi", &["--query-gpu=temperature.gpu,utilization.gpu,memory.used", "--format=csv,noheader,nounits"]) {
                let v: Vec<f32> = line.lines().next().unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect();
                if let [t, l, m] = v[..] {
                    (s.gpu_temp, s.gpu_load, s.vram_gb) = (Some(t), Some(l), Some(m / 1024.0));
                }
            }
            let now = cpu_times();
            if let (Some((b0, t0)), Some((b1, t1))) = (last, now)
                && t1 > t0
            {
                s.cpu = Some((b1 - b0) as f32 * 100.0 / (t1 - t0) as f32);
            }
            last = now;
            if let Some(line) = out("playerctl", &["-p", "playerctld", "metadata", "--format", "{{status}}|{{artist}}|{{title}}"]) {
                let mut p = line.splitn(3, '|');
                let (status, artist, title) = (p.next().unwrap_or(""), p.next().unwrap_or(""), p.next().unwrap_or(""));
                let text = if artist.is_empty() { title.to_owned() } else { format!("{artist} - {title}") };
                if !text.is_empty() {
                    s.track = Some((status == "Playing", text));
                }
            }
            if let Ok(mut g) = shared.lock() {
                *g = s;
            }
        }
    });
    if let Err(e) = spawned {
        log::warn!("live readouts: {e}");
    }
    sys
}

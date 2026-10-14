//! PipeWire through `wpctl`, on the default sink.

use std::process::Command;

use anyhow::{Context, Result, ensure};
use masterhub_core::config::{Mute, VolumeChange};

const SINK: &str = "@DEFAULT_AUDIO_SINK@";

/// `wpctl get-volume @DEFAULT_AUDIO_SINK@` → "Volume: 0.45 [MUTED]"
pub fn get() -> Option<(f32, bool)> {
    let out = Command::new("wpctl").args(["get-volume", SINK]).output().ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let v: f32 = s.split_whitespace().nth(1)?.parse().ok()?;
    Some((v.min(1.0), s.contains("MUTED")))
}

pub fn set_volume(change: VolumeChange) -> Result<()> {
    let arg = match change {
        VolumeChange::By(d) if d >= 0 => format!("{d}%+"),
        VolumeChange::By(d) => format!("{}%-", -d),
        VolumeChange::To(v) => format!("{v}%"),
    };
    wpctl(&["set-volume", "-l", "1.0", SINK, &arg])
}

pub fn set_mute(mute: Mute) -> Result<()> {
    let arg = match mute {
        Mute::Toggle => "toggle",
        Mute::On => "1",
        Mute::Off => "0",
    };
    wpctl(&["set-mute", SINK, arg])
}

fn wpctl(args: &[&str]) -> Result<()> {
    let status = Command::new("wpctl").args(args).status().context("running wpctl")?;
    ensure!(status.success(), "wpctl {} failed", args.join(" "));
    Ok(())
}

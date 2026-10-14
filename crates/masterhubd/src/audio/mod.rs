//! Volume and mute of the default output device, for `volume`/`mute` actions and the volume
//! widget. Linux: PipeWire through `wpctl`. Windows: Core Audio (`IAudioEndpointVolume`).

use anyhow::Result;
use masterhub_core::config::{Mute, VolumeChange};

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

/// Current level (0.0..=1.0) and mute state.
pub fn get() -> Option<(f32, bool)> {
    imp::get()
}

pub fn set_volume(change: VolumeChange) -> Result<()> {
    imp::set_volume(change)
}

pub fn set_mute(mute: Mute) -> Result<()> {
    imp::set_mute(mute)
}

//! Core Audio: `IAudioEndpointVolume` on the default render endpoint.

use anyhow::{Context, Result};
use masterhub_core::config::{Mute, VolumeChange};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};

/// Join the multithreaded COM apartment, once per thread. Needed before any COM or WinRT call.
pub fn com_init() {
    thread_local! {
        // SAFETY: plain COM call; an already initialised thread just returns an error code.
        static COM: () = { let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }; };
    }
    COM.with(|_| {});
}

fn endpoint() -> Result<IAudioEndpointVolume> {
    com_init();
    // SAFETY: COM calls on an initialised thread; the interfaces are released on drop.
    unsafe {
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).context("no default output device")?;
        Ok(device.Activate(CLSCTX_ALL, None)?)
    }
}

pub fn get() -> Option<(f32, bool)> {
    let ep = endpoint().ok()?;
    // SAFETY: COM calls on a valid interface.
    unsafe { Some((ep.GetMasterVolumeLevelScalar().ok()?, ep.GetMute().ok()?.as_bool())) }
}

pub fn set_volume(change: VolumeChange) -> Result<()> {
    let ep = endpoint()?;
    // SAFETY: COM calls on a valid interface; a null event context is allowed.
    unsafe {
        let level = change.apply(ep.GetMasterVolumeLevelScalar()?);
        ep.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
    }
    Ok(())
}

pub fn set_mute(mute: Mute) -> Result<()> {
    let ep = endpoint()?;
    // SAFETY: COM calls on a valid interface; a null event context is allowed.
    unsafe {
        let on = match mute {
            Mute::Toggle => !ep.GetMute()?.as_bool(),
            Mute::On => true,
            Mute::Off => false,
        };
        ep.SetMute(on, std::ptr::null())?;
    }
    Ok(())
}

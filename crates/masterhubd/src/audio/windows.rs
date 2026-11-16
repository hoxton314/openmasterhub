//! Core Audio: `IAudioEndpointVolume` on output and input endpoints, `ISimpleAudioVolume` on
//! the audio sessions of apps, on every active output (apps may play to any of them).

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use masterhub_core::config::{Mute, VolumeChange};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    AudioSessionStateExpired, DEVICE_STATE_ACTIVE, EDataFlow, IAudioSessionControl2, IAudioSessionManager2, IMMDevice,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, eCapture, eConsole, eRender,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToBSTR;
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ};
use windows::core::Interface;

use super::{Device, Stream, pick};

/// Join the multithreaded COM apartment, once per thread. Needed before any COM or WinRT call.
pub fn com_init() {
    thread_local! {
        // SAFETY: plain COM call; an already initialised thread just returns an error code.
        static COM: () = { let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }; };
    }
    COM.with(|_| {});
}

/// The session's volume control.
pub struct StreamHandle(ISimpleAudioVolume);

fn enumerator() -> Result<IMMDeviceEnumerator> {
    com_init();
    // SAFETY: COM call on an initialised thread.
    Ok(unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? })
}

fn flow(input: bool) -> EDataFlow {
    if input { eCapture } else { eRender }
}

/// Active endpoints with their friendly names, e.g. "Speakers (Realtek(R) Audio)".
fn endpoints(input: bool) -> Result<Vec<(IMMDevice, String)>> {
    let devices = enumerator()?;
    // SAFETY: COM calls on valid interfaces; the property value is converted before it is dropped.
    unsafe {
        let list = devices.EnumAudioEndpoints(flow(input), DEVICE_STATE_ACTIVE)?;
        let mut out = Vec::new();
        for i in 0..list.GetCount()? {
            let device = list.Item(i)?;
            let name = device
                .OpenPropertyStore(STGM_READ)
                .and_then(|p| p.GetValue(&PKEY_Device_FriendlyName))
                .and_then(|v| PropVariantToBSTR(&v))
                .map(|b| b.to_string())
                .unwrap_or_default();
            out.push((device, name));
        }
        Ok(out)
    }
}

fn endpoint(d: &Device) -> Result<IAudioEndpointVolume> {
    let device = match &d.name {
        // SAFETY: COM call on a valid interface.
        None => unsafe {
            enumerator()?
                .GetDefaultAudioEndpoint(flow(d.input), eConsole)
                .with_context(|| format!("no default {} device", if d.input { "input" } else { "output" }))?
        },
        Some(pattern) => {
            let all = endpoints(d.input)?;
            pick(&all, pattern, |(_, name)| [name.as_str(), ""])?.0.clone()
        }
    };
    // SAFETY: COM call on a valid interface.
    Ok(unsafe { device.Activate(CLSCTX_ALL, None)? })
}

pub fn device_get(d: &Device) -> Result<(f32, bool)> {
    let ep = endpoint(d)?;
    // SAFETY: COM calls on a valid interface.
    unsafe { Ok((ep.GetMasterVolumeLevelScalar()?, ep.GetMute()?.as_bool())) }
}

pub fn device_set_volume(d: &Device, change: VolumeChange) -> Result<()> {
    let ep = endpoint(d)?;
    // SAFETY: COM calls on a valid interface; a null event context is allowed.
    unsafe {
        let level = change.apply(ep.GetMasterVolumeLevelScalar()?);
        ep.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
    }
    Ok(())
}

pub fn device_set_mute(d: &Device, mute: Mute) -> Result<()> {
    let ep = endpoint(d)?;
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

pub fn devices(input: bool) -> Result<Vec<String>> {
    Ok(endpoints(input)?.into_iter().map(|(_, name)| name).filter(|n| !n.is_empty()).collect())
}

/// The audio sessions of apps on every active output, without the system sounds.
pub fn streams() -> Result<Vec<Stream>> {
    let mut out = Vec::new();
    for (device, _) in endpoints(false)? {
        // SAFETY: COM calls on valid interfaces; the session id string is copied, then freed.
        unsafe {
            let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
            let sessions = manager.GetSessionEnumerator()?;
            for i in 0..sessions.GetCount()? {
                let Ok(control) = sessions.GetSession(i) else { continue };
                let Ok(control) = control.cast::<IAudioSessionControl2>() else { continue };
                // S_OK for the system sounds, S_FALSE for an app.
                if control.IsSystemSoundsSession() == S_OK {
                    continue;
                }
                if control.GetState().is_ok_and(|s| s == AudioSessionStateExpired) {
                    continue;
                }
                let Ok(pid) = control.GetProcessId() else { continue };
                let Some(exe) = crate::focus::exe_name(pid) else { continue };
                let key = match control.GetSessionInstanceIdentifier() {
                    Ok(id) => {
                        let s = id.to_string().unwrap_or_default();
                        CoTaskMemFree(Some(id.0 as *const _));
                        s
                    }
                    Err(_) => format!("{pid}/{i}"),
                };
                let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue };
                out.push(Stream {
                    key,
                    names: vec![exe],
                    pid: Some(pid),
                    level: volume.GetMasterVolume().unwrap_or(1.0),
                    muted: volume.GetMute().map(|m| m.as_bool()).unwrap_or(false),
                    handle: StreamHandle(volume),
                });
            }
        }
    }
    Ok(out)
}

pub fn stream_set_volume(s: &Stream, level: f32) -> Result<()> {
    // SAFETY: COM call on a valid interface; a null event context is allowed.
    unsafe { s.handle.0.SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null())? };
    Ok(())
}

pub fn stream_set_mute(s: &Stream, mute: bool) -> Result<()> {
    // SAFETY: COM call on a valid interface; a null event context is allowed.
    unsafe { s.handle.0.SetMute(mute, std::ptr::null())? };
    Ok(())
}

/// When the process table was taken, and pid → (parent, exe name).
type ProcessTable = (Option<Instant>, HashMap<u32, (u32, String)>);

/// A process's exe name and parent, from a process snapshot at most a second old.
pub fn process(pid: u32) -> Option<(String, Option<u32>)> {
    thread_local! {
        static TABLE: RefCell<ProcessTable> = RefCell::new((None, HashMap::new()));
    }
    TABLE.with_borrow_mut(|(taken, table)| {
        if taken.is_none_or(|t| t.elapsed() > Duration::from_secs(1)) {
            *table = crate::claude::process_table();
            *taken = Some(Instant::now());
        }
        table.get(&pid).map(|(parent, name)| (name.clone(), Some(*parent)))
    })
}

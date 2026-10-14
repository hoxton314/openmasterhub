//! Windows widget sources: now playing from the system media sessions (SMTC), and CPU/GPU
//! sensors from LibreHardwareMonitor (or OpenHardwareMonitor) through WMI while it runs.
//! Windows has no unprivileged temperature source of its own.

use std::time::{Duration, Instant};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager, GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::{Buffer, DataReader, IRandomAccessStreamReference, InputStreamOptions};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE};
use windows::Win32::System::Variant::{VARIANT, VT_BSTR, VT_R4, VT_R8, VariantClear};
use windows::Win32::System::Wmi::{
    IEnumWbemClassObject, IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
    WBEM_INFINITE, WbemLocator,
};
use windows::core::{BSTR, PCWSTR, w};

use crate::audio::com_init;

/// How often to look for the hardware monitor again when it is not running.
const RETRY: Duration = Duration::from_secs(30);
const NAMESPACES: [&str; 2] = ["root\\LibreHardwareMonitor", "root\\OpenHardwareMonitor"];

pub struct Media {
    manager: Option<SessionManager>,
}

/// The playing (or most recent) media session.
pub struct Playing {
    pub title: String,
    pub artist: String,
    pub thumbnail: Option<IRandomAccessStreamReference>,
}

impl Media {
    pub fn new() -> Self {
        Self { manager: None }
    }

    pub fn playing(&mut self) -> Option<Playing> {
        com_init();
        if self.manager.is_none() {
            self.manager = SessionManager::RequestAsync().ok()?.join().ok();
        }
        let manager = self.manager.as_ref()?;
        let playing = |s: &windows::Media::Control::GlobalSystemMediaTransportControlsSession| {
            s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).is_ok_and(|st| st == PlaybackStatus::Playing)
        };
        let session = manager
            .GetSessions()
            .ok()
            .and_then(|all| all.into_iter().find(playing))
            .or_else(|| manager.GetCurrentSession().ok())?;
        let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
        let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
        Some(Playing { title, artist, thumbnail: props.Thumbnail().ok() })
    }
}

/// The encoded bytes of a media thumbnail.
pub fn read_thumbnail(r: &IRandomAccessStreamReference) -> Option<Vec<u8>> {
    let stream = r.OpenReadAsync().ok()?.join().ok()?;
    let size = u32::try_from(stream.Size().ok()?).ok()?.min(16 << 20);
    let buffer = stream.ReadAsync(&Buffer::Create(size).ok()?, size, InputStreamOptions::None).ok()?.join().ok()?;
    let reader = DataReader::FromBuffer(&buffer).ok()?;
    let mut bytes = vec![0u8; reader.UnconsumedBufferLength().ok()? as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}

/// LibreHardwareMonitor's WMI sensors.
pub struct Sensors {
    services: Option<IWbemServices>,
    last_try: Option<Instant>,
}

pub enum Sensor {
    CpuTemp,
    GpuLoad,
    GpuTemp,
}

impl Sensors {
    pub fn new() -> Self {
        Self { services: None, last_try: None }
    }

    /// Current value: °C for temperatures, 0.0..=1.0 for load.
    pub fn read(&mut self, sensor: Sensor) -> Option<f32> {
        let (kind, parents, names): (&str, &[&str], &[&str]) = match sensor {
            Sensor::CpuTemp => (
                "Temperature",
                &["/intelcpu/", "/amdcpu/"],
                &["CPU Package", "Core (Tctl/Tdie)", "Core (Tctl)", "Core (Tdie)", "Package", "Core Average", "Core Max"],
            ),
            Sensor::GpuLoad => ("Load", &["/gpu-nvidia/", "/gpu-amd/", "/gpu-intel/", "/nvidiagpu/", "/atigpu/"], &["GPU Core"]),
            Sensor::GpuTemp => ("Temperature", &["/gpu-nvidia/", "/gpu-amd/", "/gpu-intel/", "/nvidiagpu/", "/atigpu/"], &["GPU Core"]),
        };
        let rows = self.query(&format!("SELECT Name, Parent, Value FROM Sensor WHERE SensorType = '{kind}'"))?;
        let rows: Vec<_> = rows.into_iter().filter(|(_, parent, _)| parents.iter().any(|p| parent.starts_with(p))).collect();
        let value = names
            .iter()
            .find_map(|n| rows.iter().find(|(name, _, _)| name == n))
            .or(rows.first())
            .map(|(_, _, v)| *v)?;
        Some(if matches!(sensor, Sensor::GpuLoad) { value / 100.0 } else { value })
    }

    /// (Name, Parent, Value) rows; reconnects at most every `RETRY` while the monitor is absent.
    fn query(&mut self, wql: &str) -> Option<Vec<(String, String, f32)>> {
        if self.services.is_none() {
            if self.last_try.is_some_and(|t| t.elapsed() < RETRY) {
                return None;
            }
            self.last_try = Some(Instant::now());
            self.services = connect();
            if self.services.is_some() {
                log::info!("reading sensors from the hardware monitor's WMI namespace");
            }
        }
        // SAFETY: COM calls on interfaces owned by this struct, on an initialised thread.
        let rows = unsafe { rows(self.services.as_ref()?, wql) };
        if rows.is_none() {
            // The monitor was closed; look for it again later.
            self.services = None;
        }
        rows
    }
}

fn connect() -> Option<IWbemServices> {
    com_init();
    // SAFETY: COM calls on an initialised thread; the interfaces are released on drop.
    unsafe {
        let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).ok()?;
        let empty = BSTR::new();
        let services = NAMESPACES
            .iter()
            .find_map(|ns| locator.ConnectServer(&BSTR::from(*ns), &empty, &empty, &empty, 0, &empty, None).ok())?;
        // RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE
        CoSetProxyBlanket(&services, 10, 0, PCWSTR::null(), RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE, None, EOAC_NONE).ok()?;
        // A namespace without sensors (the monitor exited) counts as absent.
        rows(&services, "SELECT Name FROM Sensor").filter(|r| !r.is_empty())?;
        Some(services)
    }
}

/// # Safety
/// `services` must be a valid interface on a COM-initialised thread.
unsafe fn rows(services: &IWbemServices, wql: &str) -> Option<Vec<(String, String, f32)>> {
    let flags = WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0);
    unsafe {
        let objects: IEnumWbemClassObject = services.ExecQuery(&BSTR::from("WQL"), &BSTR::from(wql), flags, None).ok()?;
        let mut out = Vec::new();
        loop {
            let mut row: [Option<IWbemClassObject>; 1] = [None];
            let mut returned = 0u32;
            objects.Next(WBEM_INFINITE, &mut row, &mut returned).ok().ok()?;
            let Some(obj) = row[0].take().filter(|_| returned == 1) else { break };
            let text = |name| match prop(&obj, name) {
                Some(Prop::Text(s)) => s,
                _ => String::new(),
            };
            let value = match prop(&obj, w!("Value")) {
                Some(Prop::Number(v)) => v,
                _ => f32::NAN,
            };
            out.push((text(w!("Name")), text(w!("Parent")), value));
        }
        Some(out)
    }
}

enum Prop {
    Text(String),
    Number(f32),
}

/// # Safety
/// `obj` must be a valid interface on a COM-initialised thread.
unsafe fn prop(obj: &IWbemClassObject, name: PCWSTR) -> Option<Prop> {
    unsafe {
        let mut v = VARIANT::default();
        obj.Get(name, 0, &mut v, None, None).ok()?;
        let inner = &v.Anonymous.Anonymous;
        let out = match inner.vt {
            VT_R4 => Some(Prop::Number(inner.Anonymous.fltVal)),
            VT_R8 => Some(Prop::Number(inner.Anonymous.dblVal as f32)),
            VT_BSTR => Some(Prop::Text(inner.Anonymous.bstrVal.to_string())),
            _ => None,
        };
        let _ = VariantClear(&mut v);
        out
    }
}

//! Windows HID access through hidapi's native backend. The USB port path of each module comes
//! from the Configuration Manager: the HID device's parent USB device has a
//! `DEVPKEY_Device_LocationPaths` entry like `PCIROOT(0)#PCI(0801)#USBROOT(0)#USB(1)#USB(2)#USB(2)`,
//! whose `USB(n)` segments are the same port numbers Linux shows in sysfs.

use std::ffi::CString;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use hidapi::HidApi;
use masterhub_proto::{FEATURE_COMMAND, FEATURE_LEN, FeatureCommand, ModuleKind, VENDOR_ID};

use super::ModuleInfo;

fn api() -> Result<&'static Mutex<HidApi>> {
    static API: OnceLock<Option<Mutex<HidApi>>> = OnceLock::new();
    API.get_or_init(|| match HidApi::new() {
        Ok(a) => Some(Mutex::new(a)),
        Err(e) => {
            log::error!("hidapi: {e}");
            None
        }
    })
    .as_ref()
    .ok_or_else(|| anyhow!("hidapi unavailable"))
}

/// List all MasterHUB modules.
pub fn enumerate() -> Vec<ModuleInfo> {
    let Ok(api) = api() else { return Vec::new() };
    let mut api = api.lock().unwrap();
    if let Err(e) = api.reset_devices().and_then(|_| api.add_devices(VENDOR_ID, 0)) {
        log::warn!("enumerating HID devices: {e}");
        return Vec::new();
    }
    let mut found: Vec<ModuleInfo> = Vec::new();
    for d in api.device_list() {
        let Some(kind) = ModuleKind::from_pid(d.product_id()) else { continue };
        let path = d.path().to_string_lossy().into_owned();
        if found.iter().any(|m| m.path == path) {
            continue; // one entry per top-level collection
        }
        found.push(ModuleInfo {
            kind,
            serial: d.serial_number().unwrap_or_default().to_owned(),
            usb_ports: location::usb_ports(&path),
            path,
        });
    }
    found.sort_by(|a, b| a.serial.cmp(&b.serial));
    found
}

pub struct HidDevice {
    dev: hidapi::HidDevice,
    pub info: ModuleInfo,
}

impl HidDevice {
    pub fn open(info: &ModuleInfo) -> Result<Self> {
        let path = CString::new(info.path.clone())?;
        let dev = api()?.lock().unwrap().open_path(&path).with_context(|| format!("opening {}", info.path))?;
        Ok(Self { dev, info: info.clone() })
    }

    pub fn write_output(&mut self, report: &[u8]) -> Result<()> {
        let n = self.dev.write(report).context("writing output report")?;
        if n < report.len() {
            bail!("short write: {n} of {} bytes", report.len());
        }
        Ok(())
    }

    pub fn get_feature(&self, report_id: u8) -> Result<Vec<u8>> {
        let mut buf = [0u8; FEATURE_LEN];
        buf[0] = report_id;
        let n = self.dev.get_feature_report(&mut buf).with_context(|| format!("GET feature {report_id:#04x}"))?;
        Ok(buf[..n].to_vec())
    }

    /// Send a feature SET command and read report `0x03` back. Returns the read-back, which is
    /// empty when a display module ignored the command.
    pub fn command(&self, cmd: &FeatureCommand) -> Result<Vec<u8>> {
        self.dev.send_feature_report(&cmd.0).with_context(|| format!("SET feature {:02x?}", &cmd.0[..4]))?;
        // A module that ignores the command answers the GET with a zero-length report, which
        // Windows reports as an error rather than 0 bytes.
        Ok(self.get_feature(FEATURE_COMMAND).unwrap_or_default())
    }

    /// Read one input report, waiting at most `timeout`. `Ok(None)` on timeout.
    pub fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        let n = self.dev.read_timeout(buf, timeout.as_millis() as i32).context("reading input report")?;
        Ok((n > 0).then_some(n))
    }
}

mod location {
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Device_Interface_PropertyW, CM_Get_DevNode_PropertyW, CM_Get_Parent, CM_LOCATE_DEVNODE_NORMAL,
        CM_Locate_DevNodeW, CR_SUCCESS,
    };
    use windows_sys::Win32::Devices::Properties::{DEVPKEY_Device_InstanceId, DEVPKEY_Device_LocationPaths, DEVPROPTYPE};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Read a string or string-list property into a list of strings.
    fn strings(buf: &[u16]) -> Vec<String> {
        buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
    }

    /// USB port numbers of the device behind a HID interface path.
    pub fn usb_ports(interface_path: &str) -> Option<Vec<u8>> {
        // Interface path → device instance ID → devnode.
        let path = wide(interface_path);
        let mut ty: DEVPROPTYPE = 0;
        let mut buf = vec![0u16; 512];
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: all pointers are valid for the sizes passed.
        let cr = unsafe {
            CM_Get_Device_Interface_PropertyW(path.as_ptr(), &DEVPKEY_Device_InstanceId, &mut ty, buf.as_mut_ptr().cast(), &mut size, 0)
        };
        if cr != CR_SUCCESS {
            return None;
        }
        let instance = wide(strings(&buf).first()?);
        let mut devinst = 0u32;
        // SAFETY: as above.
        if unsafe { CM_Locate_DevNodeW(&mut devinst, instance.as_ptr(), CM_LOCATE_DEVNODE_NORMAL) } != CR_SUCCESS {
            return None;
        }
        // The HID devnode has no location; walk up to the USB device.
        for _ in 0..4 {
            let mut size = (buf.len() * 2) as u32;
            // SAFETY: as above.
            let cr = unsafe {
                CM_Get_DevNode_PropertyW(devinst, &DEVPKEY_Device_LocationPaths, &mut ty, buf.as_mut_ptr().cast(), &mut size, 0)
            };
            if cr == CR_SUCCESS
                && let Some(ports) = strings(&buf[..size as usize / 2]).iter().find_map(|p| super::super::parse_location_path(p))
            {
                return Some(ports);
            }
            let mut parent = 0u32;
            // SAFETY: as above.
            if unsafe { CM_Get_Parent(&mut parent, devinst, 0) } != CR_SUCCESS {
                return None;
            }
            devinst = parent;
        }
        None
    }
}

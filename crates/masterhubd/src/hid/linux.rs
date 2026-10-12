//! Linux hidraw access: discovery via sysfs, reports via read/write/ioctl.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use masterhub_core::layout::ports_from_sysfs;
use masterhub_proto::{FEATURE_COMMAND, FEATURE_LEN, FeatureCommand, ModuleKind, VENDOR_ID};

use super::ModuleInfo;

/// List all MasterHUB modules currently exposed as hidraw devices.
pub fn enumerate() -> Vec<ModuleInfo> {
    let Ok(entries) = std::fs::read_dir("/sys/class/hidraw") else {
        return Vec::new();
    };
    let mut found: Vec<ModuleInfo> = entries
        .flatten()
        .filter_map(|e| {
            let uevent = std::fs::read_to_string(e.path().join("device/uevent")).ok()?;
            let (vid, pid) = parse_hid_id(&uevent)?;
            if vid != VENDOR_ID {
                return None;
            }
            let serial = uevent.lines().find_map(|l| l.strip_prefix("HID_UNIQ="))?.to_owned();
            Some(ModuleInfo {
                kind: ModuleKind::from_pid(pid)?,
                serial,
                path: Path::new("/dev").join(e.file_name()).display().to_string(),
                usb_ports: usb_name(&e.path()).and_then(|n| ports_from_sysfs(&n)),
            })
        })
        .collect();
    found.sort_by(|a, b| a.serial.cmp(&b.serial));
    found
}

/// Walk up from the hidraw node to the USB device directory, e.g. `.../3-1.2.2/3-1.2.2:1.0/...`.
fn usb_name(hidraw: &Path) -> Option<String> {
    let real = std::fs::canonicalize(hidraw.join("device")).ok()?;
    real.ancestors()
        .filter_map(|p| p.file_name()?.to_str())
        .find(|n| is_usb_device_name(n))
        .map(str::to_owned)
}

/// `3-1.2.2` yes, `3-1.2.2:1.0` (an interface) or `usb3` no.
fn is_usb_device_name(n: &str) -> bool {
    n.split_once('-').is_some_and(|(bus, ports)| {
        !bus.is_empty()
            && bus.bytes().all(|b| b.is_ascii_digit())
            && !ports.is_empty()
            && ports.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    })
}

/// `HID_ID=0003:00002516:000001FE` -> (vid, pid)
fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let id = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut parts = id.split(':').skip(1);
    let vid = u32::from_str_radix(parts.next()?, 16).ok()?;
    let pid = u32::from_str_radix(parts.next()?, 16).ok()?;
    Some((vid as u16, pid as u16))
}

// _IOC(_IOC_READ|_IOC_WRITE, 'H', nr, len)
const fn hid_ioc(nr: u8, len: usize) -> libc::c_ulong {
    ((3 << 30) | ((len as libc::c_ulong) << 16) | ((b'H' as libc::c_ulong) << 8) | nr as libc::c_ulong) as libc::c_ulong
}

/// An open module.
pub struct HidDevice {
    file: File,
    pub info: ModuleInfo,
}

impl HidDevice {
    pub fn open(info: &ModuleInfo) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&info.path)
            .with_context(|| format!("opening {}", info.path))?;
        Ok(Self { file, info: info.clone() })
    }

    pub fn write_output(&mut self, report: &[u8]) -> Result<()> {
        let n = self.file.write(report).context("writing output report")?;
        if n != report.len() {
            bail!("short write: {n} of {} bytes", report.len());
        }
        Ok(())
    }

    pub fn get_feature(&self, report_id: u8) -> Result<Vec<u8>> {
        let mut buf = [0u8; FEATURE_LEN];
        buf[0] = report_id;
        // SAFETY: buf is a valid, writable buffer of the length encoded in the request.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), hid_ioc(0x07, buf.len()), buf.as_mut_ptr()) };
        if n < 0 {
            return Err(io::Error::last_os_error()).with_context(|| format!("GET feature {report_id:#04x}"));
        }
        Ok(buf[..n as usize].to_vec())
    }

    /// Send a feature SET command and read report `0x03` back. Returns the read-back, which is
    /// empty when a display module ignored the command.
    pub fn command(&self, cmd: &FeatureCommand) -> Result<Vec<u8>> {
        let mut buf = cmd.0;
        // SAFETY: as in get_feature.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), hid_ioc(0x06, buf.len()), buf.as_mut_ptr()) };
        if n < 0 {
            return Err(io::Error::last_os_error()).with_context(|| format!("SET feature {:02x?}", &cmd.0[..4]));
        }
        self.get_feature(FEATURE_COMMAND)
    }

    /// Read one input report, waiting at most `timeout`. `Ok(None)` on timeout.
    pub fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        // SAFETY: pfd is a valid pollfd for the duration of the call.
        let r = unsafe { libc::poll(&mut pfd, 1, timeout.as_millis() as libc::c_int) };
        if r < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(e).context("poll");
        }
        if r == 0 {
            return Ok(None);
        }
        if pfd.revents & (libc::POLLERR | libc::POLLHUP) != 0 {
            bail!("device disconnected");
        }
        let n = self.file.read(buf).context("reading input report")?;
        Ok(Some(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hid_id() {
        let u = "DRIVER=hid-generic\nHID_ID=0003:00002516:000001FE\nHID_NAME=COOLER MASTER\n";
        assert_eq!(parse_hid_id(u), Some((0x2516, 0x01FE)));
    }

    #[test]
    fn usb_names() {
        assert!(is_usb_device_name("3-1.2.2"));
        assert!(is_usb_device_name("1-4"));
        assert!(!is_usb_device_name("3-1.2.2:1.0"));
        assert!(!is_usb_device_name("usb3"));
        assert!(!is_usb_device_name("0003:2516:01FE.000E"));
    }

    #[test]
    fn ioctl_numbers_match_kernel() {
        // HIDIOCGFEATURE(32) / HIDIOCSFEATURE(32) as computed by <linux/hidraw.h>
        assert_eq!(hid_ioc(0x07, 32), 0xC020_4807);
        assert_eq!(hid_ioc(0x06, 32), 0xC020_4806);
    }
}

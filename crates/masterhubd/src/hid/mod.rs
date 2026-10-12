//! HID access to the modules, one backend per OS behind the same API:
//! `enumerate()` and `HidDevice::{open, write_output, get_feature, command, read_timeout}`.

use masterhub_proto::ModuleKind;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{HidDevice, enumerate};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{HidDevice, enumerate};

/// A connected MasterHUB module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub kind: ModuleKind,
    pub serial: String,
    /// OS device path (`/dev/hidrawN`, `\\?\HID#VID_2516…`).
    pub path: String,
    /// USB port numbers from the root hub down (`3-1.2.2` → `[1, 2, 2]`), used to work out
    /// which Base socket the module is in.
    pub usb_ports: Option<Vec<u8>>,
}

/// Windows device location path → USB port numbers:
/// `PCIROOT(0)#PCI(0801)#PCI(0004)#USBROOT(0)#USB(1)#USB(2)#USB(2)` → `[1, 2, 2]`. Stops at a
/// composite-device interface (`USBMI(n)`).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_location_path(path: &str) -> Option<Vec<u8>> {
    let ports: Vec<u8> = path
        .split('#')
        .take_while(|seg| !seg.starts_with("USBMI("))
        .filter_map(|seg| seg.strip_prefix("USB(")?.strip_suffix(')')?.parse().ok())
        .collect();
    (!ports.is_empty() && path.contains("USBROOT(")).then_some(ports)
}

#[cfg(test)]
mod tests {
    use super::parse_location_path;

    #[test]
    fn location_paths() {
        assert_eq!(parse_location_path("PCIROOT(0)#PCI(0801)#PCI(0004)#USBROOT(0)#USB(1)#USB(2)#USB(2)"), Some(vec![1, 2, 2]));
        assert_eq!(parse_location_path("PCIROOT(0)#PCI(1400)#USBROOT(0)#USB(1)#USB(1)#USB(4)#USB(4)"), Some(vec![1, 1, 4, 4]));
        assert_eq!(parse_location_path("PCIROOT(0)#PCI(1400)#USBROOT(0)#USB(3)#USBMI(0)"), Some(vec![3]));
        assert_eq!(parse_location_path("ACPI(_SB_)#ACPI(PCI0)"), None);
    }
}

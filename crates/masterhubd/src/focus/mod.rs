//! Follows the focused application and reports it to the controller (`Msg::ActiveApp`) for
//! per-app profiles. Linux: the Hyprland window class. Windows: the focused program's exe
//! name (`obs64.exe`). Also brings a process's window to the front (`raise`).

#[cfg(target_os = "linux")]
mod hyprland;
#[cfg(target_os = "linux")]
pub use hyprland::{raise, spawn};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{raise, spawn};

/// Focus the window of a process, or of its nearest ancestor that has one.
#[cfg(not(any(target_os = "linux", windows)))]
pub fn raise(_pid: u32, _title: Option<&str>) -> anyhow::Result<()> {
    anyhow::bail!("not supported on this platform")
}

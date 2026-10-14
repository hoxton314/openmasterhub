//! Follows the focused application and reports it to the controller (`Msg::ActiveApp`) for
//! per-app profiles. Linux: the Hyprland window class. Windows: the focused program's exe
//! name (`obs64.exe`).

#[cfg(target_os = "linux")]
mod hyprland;
#[cfg(target_os = "linux")]
pub use hyprland::spawn;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::spawn;

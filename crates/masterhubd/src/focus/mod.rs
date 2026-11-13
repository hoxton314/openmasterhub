//! Follows the focused application and reports it to the controller (`Msg::ActiveApp`) for
//! per-app profiles. Linux: the Hyprland window class, or the window class reported by a KWin
//! script on KDE Plasma. Windows: the focused program's exe name (`obs64.exe`).

#[cfg(target_os = "linux")]
mod hyprland;
#[cfg(target_os = "linux")]
mod kde;

/// Start every Linux focus source. Each one stays idle when its desktop is not running.
#[cfg(target_os = "linux")]
pub fn spawn(controller: std::sync::mpsc::Sender<crate::controller::Msg>) {
    kde::spawn(controller.clone());
    hyprland::spawn(controller);
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::spawn;

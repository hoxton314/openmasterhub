//! Follows the focused application and reports it to the controller (`Msg::ActiveApp`) for
//! per-app profiles. Linux: the window class from Hyprland, or from KWin on KDE Plasma. Windows:
//! the focused program's exe name (`obs64.exe`). Also brings a process's window to the front
//! (`raise`) and lists the apps with open windows (`open_apps`).

#[cfg(target_os = "linux")]
mod hyprland;
#[cfg(target_os = "linux")]
mod kde;

/// Start every Linux focus source. Each one stays idle until its desktop is running.
#[cfg(target_os = "linux")]
pub fn spawn(controller: std::sync::mpsc::Sender<crate::controller::Msg>) {
    kde::spawn(controller.clone());
    hyprland::spawn(controller);
}

/// Focus the window of a process, or of its nearest ancestor that has one.
#[cfg(target_os = "linux")]
pub fn raise(pid: u32, title: Option<&str>) -> anyhow::Result<()> {
    if hyprland::running() { hyprland::raise(pid, title) } else { kde::raise(pid, title) }
}

/// Window classes of the open windows, from whichever desktop is running.
#[cfg(target_os = "linux")]
fn window_classes() -> anyhow::Result<Vec<String>> {
    if hyprland::running() { hyprland::window_classes() } else { kde::window_classes() }
}

#[cfg(windows)]
use self::windows::window_classes;

#[cfg(not(any(target_os = "linux", windows)))]
fn window_classes() -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}

/// Apps with an open window (window classes, `.exe` names on Windows), sorted, without the
/// OpenMasterHUB app itself: suggestions for a profile's `apps`.
pub fn open_apps() -> anyhow::Result<Vec<String>> {
    Ok(clean(window_classes()?))
}

fn clean(mut classes: Vec<String>) -> Vec<String> {
    classes.retain(|c| {
        !c.is_empty() && c != masterhub_core::ipc::GUI_APP_ID && !c.to_ascii_lowercase().starts_with("openmasterhub")
    });
    classes.sort();
    classes.dedup();
    classes
}

#[cfg(target_os = "linux")]
fn parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // "pid (comm) state ppid ...", where comm may itself contain parentheses.
    stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{raise, spawn};

/// Focus the window of a process, or of its nearest ancestor that has one.
#[cfg(not(any(target_os = "linux", windows)))]
pub fn raise(_pid: u32, _title: Option<&str>) -> anyhow::Result<()> {
    anyhow::bail!("not supported on this platform")
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleans_the_app_list() {
        let v = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let got = super::clean(v(&["kitty", "", "firefox", "openmasterhub", "OpenMasterHUB.exe", "kitty", masterhub_core::ipc::GUI_APP_ID]));
        assert_eq!(got, v(&["firefox", "kitty"]));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_parent_process() {
        assert_eq!(super::parent_pid(std::process::id()), Some(std::os::unix::process::parent_id()));
    }
}

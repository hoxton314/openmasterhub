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

/// How the focused window is found (`get_system_info`), if any backend applies.
#[cfg(target_os = "linux")]
pub fn backend() -> Option<&'static str> {
    if hyprland::running() {
        return Some("hyprland");
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    desktop.split(':').any(|d| d.eq_ignore_ascii_case("kde")).then_some("kwin")
}

#[cfg(windows)]
pub fn backend() -> Option<&'static str> {
    Some("win32")
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn backend() -> Option<&'static str> {
    None
}

/// Name of a running program, for the claim list.
#[cfg(target_os = "linux")]
pub fn program_name(pid: u32) -> Option<String> {
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    Some(comm.trim().to_owned()).filter(|c| !c.is_empty())
}

#[cfg(windows)]
pub fn program_name(pid: u32) -> Option<String> {
    exe_name(pid)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn program_name(_pid: u32) -> Option<String> {
    None
}

/// Class and title of the open windows, from whichever desktop is running.
#[cfg(target_os = "linux")]
fn window_list() -> anyhow::Result<Vec<(String, String)>> {
    if hyprland::running() { hyprland::window_list() } else { kde::window_list() }
}

#[cfg(windows)]
use self::windows::window_list;

#[cfg(not(any(target_os = "linux", windows)))]
fn window_list() -> anyhow::Result<Vec<(String, String)>> {
    Ok(Vec::new())
}

/// Apps with an open window (window classes, `.exe` names on Windows), sorted, without the
/// OpenMasterHUB app itself: suggestions for a profile's `apps`.
pub fn open_apps() -> anyhow::Result<Vec<String>> {
    Ok(clean(window_list()?.into_iter().map(|(class, _)| class).collect()))
}

/// Open windows with their titles (`get_open_windows`), without the OpenMasterHUB app itself.
/// Front to back where the desktop says so, else in its own order.
pub fn open_windows() -> anyhow::Result<Vec<masterhub_core::ipc::WindowInfo>> {
    let mut windows = window_list()?;
    windows.retain(|(class, _)| !is_gui(class) && !class.is_empty());
    Ok(windows.into_iter().map(|(class, title)| masterhub_core::ipc::WindowInfo { class, title }).collect())
}

fn is_gui(class: &str) -> bool {
    class == masterhub_core::ipc::GUI_APP_ID || class.to_ascii_lowercase().starts_with("openmasterhub")
}

fn clean(mut classes: Vec<String>) -> Vec<String> {
    classes.retain(|c| !c.is_empty() && !is_gui(c));
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
pub use self::windows::{exe_name, raise, spawn};

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

//! Commands about the machine and the service: whether `masterhubd` runs and what it logged,
//! autostart, the checks of the first-run flow, the vendor app, command test runs and window
//! titles. Platform code is behind `cfg`, as in the service.

use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use masterhub_core::ipc::{Request, SystemInfo, WindowInfo};
use serde::Serialize;
use tauri::State;

use crate::{AppState, err};

/// The service has been reachable since the app started (so a missing one has crashed).
static SEEN_ONLINE: AtomicBool = AtomicBool::new(false);
/// When the app last started the service, for the `starting` state.
static LAUNCHED_AT: Mutex<Option<Instant>> = Mutex::new(None);
/// The service the app started itself (no systemd), to learn how it ended.
#[cfg(not(target_os = "linux"))]
pub static CHILD: Mutex<Option<std::process::Child>> = Mutex::new(None);

pub fn note_online(online: bool) {
    if online {
        SEEN_ONLINE.store(true, Ordering::Relaxed);
    }
}

pub fn note_launched() {
    *LAUNCHED_AT.lock().unwrap() = Some(Instant::now());
}

/// A `Command` that opens no console window on Windows.
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Standard output of a successful command, `None` when it fails or cannot start.
fn output(cmd: &mut Command) -> Option<String> {
    let out = cmd.stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

// ───────────────────────────── service status and log ─────────────────────────────

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    /// It ended with an error: `reason` reads after "It ran into", `code` is the exit code.
    Crashed {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
    },
}

/// Whether masterhubd is starting, stopped or crashed (and with which exit code). The UI asks
/// while the service is unreachable; one that has just become reachable is still "starting"
/// until the UI hears of it.
#[tauri::command]
pub fn service_status(state: State<AppState>) -> ServiceStatus {
    if state.client.is_online() {
        return ServiceStatus::Starting;
    }
    if LAUNCHED_AT.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(6)) {
        return ServiceStatus::Starting;
    }
    let status = platform_status();
    if let ServiceStatus::Crashed { code, .. } = &status {
        let lines = read_log(40);
        return ServiceStatus::Crashed { reason: Some(crash_reason(&lines, *code)), code: *code };
    }
    status
}

#[cfg(target_os = "linux")]
fn platform_status() -> ServiceStatus {
    let Some(text) = output(command("systemctl").args(["--user", "show", "masterhubd", "--property=ActiveState,SubState,Result,ExecMainStatus"])) else {
        return ServiceStatus::Stopped;
    };
    parse_systemd(&text)
}

/// The state from `systemctl show` output.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_systemd(text: &str) -> ServiceStatus {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).unwrap_or("").trim();
    let code = get("ExecMainStatus").parse::<i32>().ok().filter(|c| *c != 0);
    let failed = get("Result") != "success" && !get("Result").is_empty();
    match (get("ActiveState"), get("SubState")) {
        ("failed", _) => ServiceStatus::Crashed { reason: None, code },
        ("activating", "auto-restart") => ServiceStatus::Crashed { reason: None, code },
        ("activating", _) | ("reloading", _) => ServiceStatus::Starting,
        // Running, but not answering yet.
        ("active", _) => ServiceStatus::Starting,
        (_, _) if failed => ServiceStatus::Crashed { reason: None, code },
        _ => ServiceStatus::Stopped,
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_status() -> ServiceStatus {
    if let Some(child) = CHILD.lock().unwrap().as_mut()
        && let Ok(Some(exit)) = child.try_wait()
    {
        return match exit.code() {
            Some(0) => ServiceStatus::Stopped,
            code => ServiceStatus::Crashed { reason: None, code },
        };
    }
    if process_ids("masterhubd").is_empty() {
        // Running before, gone now, and nobody asked it to stop.
        if SEEN_ONLINE.load(Ordering::Relaxed) { ServiceStatus::Crashed { reason: None, code: None } } else { ServiceStatus::Stopped }
    } else {
        ServiceStatus::Starting
    }
}

/// What went wrong, from the last error lines of the log, for "It ran into …".
fn crash_reason(lines: &[String], code: Option<i32>) -> String {
    let errors: Vec<&String> = lines.iter().filter(|l| l.contains("ERROR") || l.contains("panicked")).collect();
    let text = errors.last().map(|l| l.to_ascii_lowercase()).unwrap_or_default();
    let what = ["audio", "usb", "hid", "config"]
        .iter()
        .find(|k| text.contains(**k))
        .map(|k| match *k {
            "usb" | "hid" => "a USB error".to_owned(),
            k => format!("an {k} error"),
        })
        .unwrap_or_else(|| if text.contains("panicked") { "an internal error".into() } else { "an error".into() });
    match code {
        Some(c) => format!("{what} (exit code {c})"),
        None => what,
    }
}

/// The last `n` lines of the service log, as `HH:MM:SS LEVEL message`.
#[tauri::command]
pub fn service_log(lines: Option<usize>) -> Vec<String> {
    read_log(lines.unwrap_or(40).clamp(1, 500))
}

fn read_log(n: usize) -> Vec<String> {
    raw_log(n).iter().map(|l| format_log_line(l)).filter(|l| !l.trim().is_empty()).collect()
}

/// systemd's journal for the unit.
#[cfg(target_os = "linux")]
fn raw_log(n: usize) -> Vec<String> {
    let n = n.to_string();
    output(command("journalctl").args(["--user", "-u", "masterhubd", "-n", &n, "--no-pager", "-o", "cat"]))
        .map(|t| t.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// `%LOCALAPPDATA%\openmasterhub\masterhubd.log` (and the one it replaced when it grew).
#[cfg(not(target_os = "linux"))]
fn raw_log(n: usize) -> Vec<String> {
    let Some(dir) = log_dir_path() else { return Vec::new() };
    let mut lines: Vec<String> = Vec::new();
    for name in ["masterhubd.old.log", "masterhubd.log"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
            lines.extend(text.lines().map(str::to_owned));
        }
    }
    let skip = lines.len().saturating_sub(n);
    lines.split_off(skip)
}

/// `[2026-11-27T15:31:02Z INFO  masterhubd] text` (env_logger) as `15:31:02 INFO  text`, in local
/// time; other lines (systemd's, panics) stay as they are.
fn format_log_line(line: &str) -> String {
    let parse = || {
        let rest = line.strip_prefix('[')?;
        let (head, text) = rest.split_once("] ")?;
        let mut parts = head.split_whitespace();
        let (ts, level) = (parts.next()?, parts.next()?);
        let time = match ts.parse::<jiff::Timestamp>() {
            Ok(t) => t.to_zoned(jiff::tz::TimeZone::system()).strftime("%H:%M:%S").to_string(),
            Err(_) => ts.to_owned(),
        };
        Some(format!("{time} {level:<5} {text}"))
    };
    parse().unwrap_or_else(|| line.to_owned())
}

/// Where the service writes its log file; Linux logs to the journal, which has no folder.
#[tauri::command]
pub fn service_log_dir() -> String {
    #[cfg(target_os = "linux")]
    {
        String::new()
    }
    #[cfg(not(target_os = "linux"))]
    {
        log_dir_path().map(|p| p.display().to_string()).unwrap_or_default()
    }
}

#[cfg(not(target_os = "linux"))]
fn log_dir_path() -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("openmasterhub"))
}

// ───────────────────────────── autostart ─────────────────────────────

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const RUN_VALUE: &str = "OpenMasterHUB service";

/// Whether the service starts with the session (systemd unit enabled; the Run key on Windows).
#[tauri::command]
pub fn service_autostart() -> bool {
    #[cfg(target_os = "linux")]
    {
        output(command("systemctl").args(["--user", "is-enabled", "masterhubd"])).is_some_and(|s| s.trim().starts_with("enabled"))
    }
    #[cfg(windows)]
    {
        output(command("reg").args(["query", RUN_KEY, "/v", RUN_VALUE])).is_some()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        false
    }
}

#[tauri::command]
pub fn set_service_autostart(on: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let verb = if on { "enable" } else { "disable" };
        let ok = command("systemctl").args(["--user", verb, "masterhubd"]).status().is_ok_and(|s| s.success());
        if ok { Ok(()) } else { Err(format!("systemctl --user {verb} masterhubd failed")) }
    }
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(err)?;
        let service = exe.parent().map(|d| d.join("masterhubd.exe")).filter(|p| p.exists()).ok_or("masterhubd is not installed next to the app")?;
        let ok = if on {
            command("reg").args(["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/f", "/d", &format!("\"{}\"", service.display())]).status()
        } else {
            command("reg").args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]).status()
        };
        // Deleting a value that is not there fails, which is what was asked for.
        if ok.is_ok_and(|s| s.success()) || !on { Ok(()) } else { Err("could not write the Run key".into()) }
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = on;
        Err("autostart is not supported on this platform".into())
    }
}

// ───────────────────────────── processes ─────────────────────────────

/// Ids of the processes named `name` (an `.exe` name on Windows; the program name elsewhere).
fn process_ids(name: &str) -> Vec<u32> {
    #[cfg(windows)]
    {
        let exe = if name.to_ascii_lowercase().ends_with(".exe") { name.to_owned() } else { format!("{name}.exe") };
        output(command("tasklist").args(["/FI", &format!("IMAGENAME eq {exe}"), "/FO", "CSV", "/NH"])).map(|t| parse_tasklist(&t)).unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
        // `comm` holds the first 15 characters of the program name.
        let name: String = name.chars().take(15).collect();
        dir.flatten()
            .filter_map(|e| {
                let pid: u32 = e.file_name().to_str()?.parse().ok()?;
                let comm = std::fs::read_to_string(e.path().join("comm")).ok()?;
                comm.trim().eq_ignore_ascii_case(&name).then_some(pid)
            })
            .collect()
    }
}

/// Process ids in `tasklist /FO CSV /NH` output: `"MasterHUB.exe","8812","Console","1","1 K"`.
#[cfg_attr(not(windows), allow(dead_code))]
fn parse_tasklist(text: &str) -> Vec<u32> {
    text.lines()
        .filter_map(|l| {
            let mut fields = l.trim().trim_matches('"').split("\",\"");
            fields.next()?;
            fields.next()?.parse().ok()
        })
        .collect()
}

#[derive(Serialize)]
pub struct VendorApp {
    name: String,
    pid: u32,
}

const VENDOR_APP: &str = "MasterHUB";

fn vendor() -> Option<VendorApp> {
    let pid = *process_ids(VENDOR_APP).first()?;
    Some(VendorApp { name: if cfg!(windows) { "MasterHUB.exe" } else { VENDOR_APP }.to_owned(), pid })
}

/// The vendor MasterHUB app, when it runs next to masterhubd: both fight over the modules.
#[tauri::command]
pub fn vendor_app() -> Option<VendorApp> {
    vendor()
}

/// End the vendor app (all of its processes) without asking it.
#[tauri::command]
pub fn close_vendor_app() -> Result<(), String> {
    #[cfg(windows)]
    {
        let ok = command("taskkill").args(["/IM", "MasterHUB.exe", "/F", "/T"]).status().is_ok_and(|s| s.success());
        if ok { Ok(()) } else { Err("taskkill could not end MasterHUB.exe".into()) }
    }
    #[cfg(not(windows))]
    {
        let pids = process_ids(VENDOR_APP);
        if pids.is_empty() {
            return Ok(());
        }
        let mut cmd = command("kill");
        cmd.args(pids.iter().map(u32::to_string));
        if cmd.status().is_ok_and(|s| s.success()) { Ok(()) } else { Err("could not end the vendor app".into()) }
    }
}

// ───────────────────────────── system checks ─────────────────────────────

#[derive(Serialize)]
pub struct Check {
    id: &'static str,
    title: &'static str,
    detail: String,
    /// `success`, `warning`, `danger` or `off`.
    status: &'static str,
    label: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
}

fn check(id: &'static str, title: &'static str, status: &'static str, label: &'static str, detail: impl Into<String>) -> Check {
    Check { id, title, detail: detail.into(), status, label, help: None }
}

/// What the first-run flow and the settings check: the service, USB access, focus tracking,
/// temperatures (Windows) and the vendor app.
#[tauri::command]
pub fn system_checks(state: State<AppState>) -> Vec<Check> {
    let online = state.client.is_online();
    let mut out = vec![if online {
        check("service", "masterhubd service", "success", "Running", if service_autostart() { "started with your session" } else { "running; not set to start with your session" })
    } else {
        check("service", "masterhubd service", "danger", "Stopped", "the app edits your config, but only the service talks to the hub")
    }];

    #[cfg(target_os = "linux")]
    {
        const RULE: &str = "70-masterhub.rules";
        let dirs = ["/usr/lib/udev/rules.d", "/lib/udev/rules.d", "/etc/udev/rules.d", "/usr/local/lib/udev/rules.d"];
        let found = dirs.iter().any(|d| std::path::Path::new(d).join(RULE).exists());
        let opened = online && state.client.modules().is_ok_and(|m| !m.is_empty());
        out.push(if found || opened {
            check("udev", "USB access", "success", "Allowed", if found { format!("udev rule {RULE}") } else { "the service opens the modules".to_owned() })
        } else {
            Check {
                help: Some(format!("Copy udev/{RULE} to /etc/udev/rules.d/, then replug the hub")),
                ..check("udev", "USB access", "warning", "Missing", format!("udev rule {RULE} not found; the modules may not be readable without root"))
            }
        });
    }

    out.push(match state.client.system_info() {
        Ok(SystemInfo { focus_backend: Some(b), .. }) => check("focus", "Focus tracking", "success", "Works", format!("following the focused window through {}", backend_name(&b))),
        Ok(_) => Check {
            help: Some("masterhubd ctl profile NAME switches by hand".into()),
            ..check("focus", "Focus tracking", "warning", "Limited", "Per-app profiles need Hyprland, KDE Plasma or Windows. Elsewhere, switch profiles by hand or from the tray.")
        },
        Err(_) => check("focus", "Focus tracking", "off", "Unknown", "needs the service"),
    });

    #[cfg(windows)]
    out.push(if process_ids("LibreHardwareMonitor").is_empty() {
        check("lhm", "Temperatures", "off", "Optional", "Need LibreHardwareMonitor running. SmartScreen may warn once; choose More info › Run anyway.")
    } else {
        check("lhm", "Temperatures", "success", "Running", "LibreHardwareMonitor is running")
    });

    out.push(match vendor() {
        Some(v) => Check {
            help: Some("Close it from the hardware screen".into()),
            ..check("vendor", "Vendor MasterHUB app", "warning", "Running", format!("{} (PID {}) fights over the modules; close it", v.name, v.pid))
        },
        None => check("vendor", "Vendor MasterHUB app", "success", "Not running", "nothing else is driving the modules"),
    });
    out
}

fn backend_name(b: &str) -> &str {
    match b {
        "hyprland" => "Hyprland",
        "kwin" => "KWin",
        "win32" => "Windows",
        other => other,
    }
}

/// The service's `get_system_info` (platform, focus backend, version).
#[tauri::command]
pub fn system_info(state: State<AppState>) -> Result<SystemInfo, String> {
    state.client.system_info().map_err(err)
}

/// The program an AI agent starts for the MCP server (`masterhubd mcp`, docs/MCP.md): the
/// service next to the app (Windows installer, dev builds), else `masterhubd` from `PATH`.
#[tauri::command]
pub fn mcp_program() -> String {
    let name = if cfg!(windows) { "masterhubd.exe" } else { "masterhubd" };
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(name)))
        .filter(|p| p.is_file())
        .map_or_else(|| "masterhubd".to_owned(), |p| p.display().to_string())
}

// ───────────────────────────── command test ─────────────────────────────

#[derive(Serialize)]
pub struct CommandResult {
    code: Option<i32>,
    ms: u128,
    output: String,
}

const TEST_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_OUTPUT_MAX: usize = 4096;

/// Run a `cmd` action once, as the service would (`sh -c`, `cmd /C`), with `{value}` replaced.
#[tauri::command]
pub async fn run_command_test(cmd: String, value: Option<u8>) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_test(&cmd, value)).await.map_err(err)?
}

fn shell(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = Command::new("cmd");
        c.arg("/C").raw_arg(cmd).creation_flags(CREATE_NO_WINDOW);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    }
}

fn run_test(cmd: &str, value: Option<u8>) -> Result<CommandResult, String> {
    use std::io::Read;
    let cmd = match value {
        Some(v) => cmd.replace("{value}", &v.to_string()),
        None => cmd.to_owned(),
    };
    let start = Instant::now();
    let mut child = shell(&cmd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(err)?;
    // Readers end with the pipes; a grandchild that keeps them open must not hold us up.
    let collected = std::sync::Arc::new(Mutex::new(Vec::<u8>::new()));
    let readers: Vec<_> = [child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>)]
        .into_iter()
        .flatten()
        .map(|mut pipe| {
            let sink = collected.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                while let Ok(n) = pipe.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let mut sink = sink.lock().unwrap();
                    if sink.len() < TEST_OUTPUT_MAX {
                        sink.extend_from_slice(&buf[..n]);
                    }
                }
            })
        })
        .collect();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().map_err(err)? {
            Some(s) => break Some(s),
            None if start.elapsed() > TEST_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(15)),
        }
    };
    let ms = start.elapsed().as_millis();
    let deadline = Instant::now() + Duration::from_millis(150);
    while readers.iter().any(|r| !r.is_finished()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut output = String::from_utf8_lossy(&collected.lock().unwrap()).into_owned();
    if timed_out {
        output = format!("stopped after {} s{}{}", TEST_TIMEOUT.as_secs(), if output.is_empty() { "" } else { ": " }, output);
    }
    Ok(CommandResult { code: status.and_then(|s| s.code()), ms, output })
}

// ───────────────────────────── windows ─────────────────────────────

/// Open windows with their titles for the rule editor; the classes alone when the service is
/// older than the request.
#[tauri::command]
pub fn open_window_titles(state: State<AppState>) -> Vec<WindowInfo> {
    match state.client.open_windows() {
        Ok(w) => w,
        Err(e) => {
            log::debug!("open windows: {e:#}");
            state.client.open_apps().unwrap_or_default().into_iter().map(|class| WindowInfo { class, title: String::new() }).collect()
        }
    }
}

/// Tell a module to flash so the user can find it.
#[tauri::command]
pub fn identify(state: State<AppState>, serial: String) -> Result<(), String> {
    state.client.identify(&serial).map_err(err)
}

/// Run a request without caring about its answer (tray, shortcuts), logging a failure.
pub fn fire(state: &AppState, request: Request, what: &str) {
    if let Err(e) = state.client.call(request) {
        log::warn!("{what}: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systemd_states() {
        let show = |active: &str, sub: &str, result: &str, code: &str| format!("ActiveState={active}\nSubState={sub}\nResult={result}\nExecMainStatus={code}\n");
        assert_eq!(parse_systemd(&show("inactive", "dead", "success", "0")), ServiceStatus::Stopped);
        assert_eq!(parse_systemd(&show("active", "running", "success", "0")), ServiceStatus::Starting);
        assert_eq!(parse_systemd(&show("activating", "start", "success", "0")), ServiceStatus::Starting);
        assert_eq!(parse_systemd(&show("failed", "failed", "exit-code", "101")), ServiceStatus::Crashed { reason: None, code: Some(101) });
        assert_eq!(parse_systemd(&show("activating", "auto-restart", "exit-code", "101")), ServiceStatus::Crashed { reason: None, code: Some(101) });
        assert_eq!(parse_systemd(""), ServiceStatus::Stopped);
    }

    #[test]
    fn log_lines() {
        let l = format_log_line("[2026-11-27T15:31:07Z ERROR masterhubd::audio] PipeWire connection lost");
        assert!(l.ends_with("ERROR PipeWire connection lost"), "{l}");
        assert_eq!(l.split(' ').next().unwrap().len(), 8);
        assert_eq!(format_log_line("masterhubd.service: Main process exited"), "masterhubd.service: Main process exited");
    }

    #[test]
    fn reasons() {
        let lines = vec!["15:31:07 ERROR audio: PipeWire connection lost".to_owned()];
        assert_eq!(crash_reason(&lines, Some(101)), "an audio error (exit code 101)");
        assert_eq!(crash_reason(&[], None), "an error");
        assert_eq!(crash_reason(&["thread 'x' panicked at src/a.rs".to_owned()], Some(101)), "an internal error (exit code 101)");
    }

    #[test]
    fn tasklist_output() {
        assert_eq!(parse_tasklist("\"MasterHUB.exe\",\"8812\",\"Console\",\"1\",\"123,456 K\"\r\n\"MasterHUB.exe\",\"9001\",\"Console\",\"1\",\"9 K\"\r\n"), vec![8812, 9001]);
        assert!(parse_tasklist("INFO: No tasks are running which match the specified criteria.").is_empty());
    }

    #[test]
    fn test_runs() {
        #[cfg(not(windows))]
        {
            let r = run_test("echo hi {value}", Some(7)).unwrap();
            assert_eq!(r.code, Some(0));
            assert_eq!(r.output.trim(), "hi 7");
            let r = run_test("echo oops >&2; exit 3", None).unwrap();
            assert_eq!(r.code, Some(3));
            assert!(r.output.contains("oops"));
        }
    }
}

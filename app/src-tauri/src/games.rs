//! The Game modes section: asks `masterhub-games` (next to the app, or on PATH) what each game
//! would need, runs the setup the user approved and keeps the game-mode service running.

use std::path::PathBuf;
use std::process::Command;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// `masterhub-games` next to the app (Windows sidecar, dev builds), else from PATH (Linux package).
fn binary() -> PathBuf {
    let name = if cfg!(windows) { "masterhub-games.exe" } else { "masterhub-games" };
    std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(name))).filter(|p| p.exists()).unwrap_or_else(|| name.into())
}

fn command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(binary());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Run with `args`; stdout on success, else stderr (or stdout) as the error.
fn run(args: &[&str]) -> Result<String, String> {
    let out = command().args(args).output().map_err(|e| format!("masterhub-games: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let msg = stderr.lines().rev().find(|l| !l.trim().is_empty()).map(str::to_owned).unwrap_or(stdout);
    Err(msg.trim_start_matches("Error: ").to_owned())
}

fn json(args: &[&str]) -> Result<serde_json::Value, String> {
    serde_json::from_str(&run(args)?).map_err(err)
}

/// The games this build knows and their state; empty when masterhub-games is not installed.
#[tauri::command]
pub async fn games_list() -> Result<serde_json::Value, String> {
    json(&["list", "--json"])
}

/// What setting up `game` would do, from `path` or the folder found through Steam.
#[tauri::command]
pub async fn games_plan(game: String, path: Option<String>) -> Result<serde_json::Value, String> {
    let mut args = vec!["plan", game.as_str()];
    if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
        args.extend(["--path", p]);
    }
    json(&args)
}

/// Run the approved setup, then make sure the game-mode service runs (it reloads by itself).
#[tauri::command]
pub async fn games_setup(game: String, path: Option<String>) -> Result<String, String> {
    let mut args = vec!["setup", game.as_str()];
    if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
        args.extend(["--path", p]);
    }
    let out = run(&args)?;
    ensure_running();
    Ok(out)
}

#[tauri::command]
pub async fn games_disable(game: String) -> Result<(), String> {
    run(&["disable", &game]).map(|_| ())
}

/// Start the game-mode service if it isn't running: the systemd user unit on Linux (enabled,
/// so it also starts at login), else the binary, detached. A second copy exits by itself.
fn ensure_running() {
    #[cfg(target_os = "linux")]
    if Command::new("systemctl").args(["--user", "enable", "--now", "masterhub-games"]).status().is_ok_and(|s| s.success()) {
        return;
    }
    let mut cmd = command();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    let spawned = cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    if let Err(e) = spawned {
        log::warn!("starting masterhub-games: {e}");
    }
}

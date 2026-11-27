//! Files the app reads and writes itself: the config folder, TOML import/export, backups of
//! the config, and showing folders in the system's file manager.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use masterhub_core::config::{self, Config, Profile};
use masterhub_core::ipc::Client;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{AppState, err};

/// The folder of the config the service uses, else of the default config path.
pub fn config_dir(client: &Client) -> PathBuf {
    config_path(client).parent().map(Path::to_owned).unwrap_or_else(|| PathBuf::from("."))
}

/// The config the service uses, else the default path.
fn config_path(client: &Client) -> PathBuf {
    client.config_path().map(PathBuf::from).unwrap_or_else(|_| config::default_path())
}

/// `<config dir>/backups`.
fn backups_dir(client: &Client) -> PathBuf {
    config_dir(client).join("backups")
}

/// Show a folder in the file manager. Only folders are opened, never files, so a path from
/// the UI cannot start a program.
pub fn open_folder(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        bail!("{} is not a folder", dir.display());
    }
    #[cfg(target_os = "linux")]
    let mut cmd = std::process::Command::new("xdg-open");
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    cmd.arg(dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    // Reap the opener in the background; explorer.exe exits with 1 even when it worked.
    let mut child = cmd.spawn().with_context(|| format!("opening {}", dir.display()))?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Show a folder in the file manager; for a file, the folder it is in.
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(path);
    let dir = if p.is_dir() { p } else { p.parent().map(Path::to_owned).unwrap_or(p) };
    open_folder(&dir).map_err(err)
}

/// Pick a TOML file and return its profiles (`None`: cancelled). The file need not be a whole
/// config: references to other profiles are checked when the result is saved.
#[tauri::command]
pub async fn import_toml(app: AppHandle) -> Result<Option<Vec<Profile>>, String> {
    let Some(path) = app.dialog().file().add_filter("TOML", &["toml"]).blocking_pick_file() else { return Ok(None) };
    let path = path.into_path().map_err(err)?;
    import(&path).map(Some).map_err(err)
}

fn import(path: &Path) -> Result<Vec<Profile>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let cfg = Config::parse_unchecked(&text, dir).with_context(|| format!("in {}", path.display()))?;
    if cfg.profiles.is_empty() {
        bail!("{} has no [[profile]]", path.display());
    }
    Ok(cfg.profiles)
}

/// Save one profile as a TOML file chosen in a save dialog; the path written (`None`: cancelled).
#[tauri::command]
pub async fn export_toml(app: AppHandle, profile: Profile) -> Result<Option<String>, String> {
    let name: String = profile.name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' }).collect();
    let dialog = app.dialog().file().add_filter("TOML", &["toml"]).set_file_name(format!("{}.toml", name.to_lowercase()));
    let Some(path) = dialog.blocking_save_file() else { return Ok(None) };
    let mut path = path.into_path().map_err(err)?;
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("toml")) {
        path.set_extension("toml");
    }
    export(&path, profile).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

fn export(path: &Path, profile: Profile) -> Result<()> {
    let cfg = Config { profiles: vec![profile], ..Default::default() };
    std::fs::write(path, cfg.to_toml()?).with_context(|| format!("writing {}", path.display()))
}

/// `<config dir>/templates`.
fn templates_dir(client: &Client) -> PathBuf {
    config_dir(client).join("templates")
}

/// Save a user template (the UI's `Template` JSON) as `<config dir>/templates/<id>.json`;
/// returns its path.
#[tauri::command]
pub fn template_save(state: State<AppState>, template: serde_json::Value) -> Result<String, String> {
    save_template(&templates_dir(&state.client), &template).map(|p| p.display().to_string()).map_err(err)
}

fn save_template(dir: &Path, template: &serde_json::Value) -> Result<PathBuf> {
    let id = template.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let file: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    if file.trim_matches('-').is_empty() {
        bail!("template has no id");
    }
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{file}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(template)?).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// The user's templates in `<config dir>/templates/` with their text, as for themes.
#[tauri::command]
pub fn templates_list(state: State<AppState>) -> Vec<crate::themes::ThemeFile> {
    crate::themes::list(&templates_dir(&state.client))
}

/// A saved copy of the config.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Backup {
    pub path: String,
    /// File name, e.g. `config-20261126-142233.toml` (UTC).
    pub name: String,
    /// Unix seconds.
    pub modified: u64,
}

/// Copy the current config to `<config dir>/backups/config-<UTC time>.toml`; returns its path.
#[tauri::command]
pub fn backup_config(state: State<AppState>) -> Result<String, String> {
    backup(&state.client).map(|p| p.display().to_string()).map_err(err)
}

fn backup(client: &Client) -> Result<PathBuf> {
    let text = match client.config() {
        Ok(text) => text,
        Err(_) => {
            let path = config_path(client);
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
        }
    };
    write_backup(&backups_dir(client), &text, unix_now())
}

fn write_backup(dir: &Path, text: &str, now: u64) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let stamp = utc_stamp(now);
    let mut path = dir.join(format!("config-{stamp}.toml"));
    for n in 2.. {
        if !path.exists() {
            break;
        }
        path = dir.join(format!("config-{stamp}-{n}.toml"));
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Backups of the config, newest first.
#[tauri::command]
pub fn backups_list(state: State<AppState>) -> Vec<Backup> {
    list_backups(&backups_dir(&state.client))
}

fn list_backups(dir: &Path) -> Vec<Backup> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<Backup> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if !name.ends_with(".toml") {
                return None;
            }
            let modified = e.metadata().ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
            Some(Backup { path: e.path().display().to_string(), name, modified })
        })
        .collect();
    v.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| b.name.cmp(&a.name)));
    v
}

/// Replace the config with a TOML file (e.g. a backup), keeping its text as written. The
/// current config is backed up first; returns that backup's path.
#[tauri::command]
pub fn restore_config(state: State<AppState>, path: String) -> Result<String, String> {
    restore(&state.client, Path::new(&path)).map_err(err)
}

fn restore(client: &Client, path: &Path) -> Result<String> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    // Relative paths in the file are meant relative to the config folder it is restored to.
    Config::parse(&text, &config_dir(client)).with_context(|| format!("in {}", path.display()))?;
    let saved = backup(client)?;
    client.set_config(&text)?;
    Ok(saved.display().to_string())
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `YYYYMMDD-HHMMSS` in UTC.
fn utc_stamp(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_782_400), "20000229-000000");
        assert_eq!(utc_stamp(1_795_703_553), "20261126-143233");
    }

    #[test]
    fn backups_import_export_templates() {
        let d = std::env::temp_dir().join(format!("omh-files-{}", std::process::id()));
        let a = write_backup(&d, "backlight = 10\n", 0).unwrap();
        let b = write_backup(&d, "backlight = 20\n", 0).unwrap();
        assert_ne!(a, b);
        assert!(b.to_string_lossy().ends_with("config-19700101-000000-2.toml"));
        assert_eq!(list_backups(&d).len(), 2);
        // A profile that refers to another profile exports and imports on its own.
        let cfg = Config::parse("[[profile]]\nname='A'\n[[profile]]\nname='B'\n[profile.dial]\npress={profile='A'}\n", &d).unwrap();
        export(&d.join("b.toml"), cfg.profiles[1].clone()).unwrap();
        assert_eq!(import(&d.join("b.toml")).unwrap(), [cfg.profiles[1].clone()]);
        std::fs::write(d.join("empty.toml"), "backlight = 1\n").unwrap();
        assert!(import(&d.join("empty.toml")).is_err());
        let t = save_template(&d.join("templates"), &serde_json::json!({"id": "my/../t", "kind": "profile"})).unwrap();
        assert!(t.ends_with("templates/my----t.json"), "{}", t.display());
        assert!(save_template(&d, &serde_json::json!({"kind": "profile"})).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }
}

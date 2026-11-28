//! User themes: JSON files in `<config dir>/themes/` (Linux and Windows alike). The UI parses
//! and validates them; this side only lists them and opens the folder.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::{AppState, err};
use crate::files::{config_dir, open_folder};

/// Larger files are skipped: a theme is a few KB of tokens.
const MAX_THEME: u64 = 256 * 1024;

/// One theme file and its text (`UserThemeFile` in the UI).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeFile {
    pub file: String,
    pub json: String,
}

/// `<config dir>/themes`.
pub fn dir(config_dir: &Path) -> PathBuf {
    config_dir.join("themes")
}

/// The `*.json` files in `dir` by name, without JSON schemas (`*.schema.json`). A missing
/// folder has none; unreadable or oversized files are left out and logged.
pub fn list(dir: &Path) -> Vec<ThemeFile> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<ThemeFile> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().into_string().ok()?;
            let lower = file.to_ascii_lowercase();
            if !lower.ends_with(".json") || lower.ends_with(".schema.json") {
                return None;
            }
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            if meta.len() > MAX_THEME {
                log::warn!("theme {file}: larger than {MAX_THEME} bytes, skipped");
                return None;
            }
            match std::fs::read_to_string(e.path()) {
                Ok(json) => Some(ThemeFile { file, json }),
                Err(e) => {
                    log::warn!("theme {file}: {e}");
                    None
                }
            }
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));
    files
}

/// The user's theme files with their text.
#[tauri::command]
pub fn themes_list(state: State<AppState>) -> Vec<ThemeFile> {
    list(&dir(&config_dir(&state.client)))
}

/// The user themes folder, created if missing.
#[tauri::command]
pub fn themes_dir(state: State<AppState>) -> Result<String, String> {
    let d = dir(&config_dir(&state.client));
    std::fs::create_dir_all(&d).map_err(err)?;
    Ok(d.display().to_string())
}

/// Show the user themes folder in the file manager, creating it if missing.
#[tauri::command]
pub fn open_themes_dir(state: State<AppState>) -> Result<(), String> {
    let d = dir(&config_dir(&state.client));
    std::fs::create_dir_all(&d).map_err(err)?;
    open_folder(&d).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_json_files_by_name() {
        let d = std::env::temp_dir().join(format!("omh-themes-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("orchid.json"), r#"{"id":"orchid"}"#).unwrap();
        std::fs::write(d.join("Amber.JSON"), "{}").unwrap();
        std::fs::write(d.join("theme.schema.json"), "{}").unwrap();
        std::fs::write(d.join("notes.txt"), "x").unwrap();
        std::fs::write(d.join("huge.json"), vec![b' '; MAX_THEME as usize + 1]).unwrap();
        std::fs::create_dir_all(d.join("dir.json")).unwrap();
        let files = list(&d);
        std::fs::remove_dir_all(&d).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.file.as_str()).collect();
        assert_eq!(names, ["Amber.JSON", "orchid.json"]);
        assert_eq!(files[1].json, r#"{"id":"orchid"}"#);
        assert!(list(&d).is_empty());
    }
}

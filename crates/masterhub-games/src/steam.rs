//! Finding Steam games: the Steam folders of this machine, their libraries
//! (`steamapps/libraryfolders.vdf`) and a game's install folder from its app manifest.

use std::path::{Path, PathBuf};

/// Steam's own folders that exist here (Linux native and Flatpak; Windows default and registry).
pub fn roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(not(windows))]
    if let Some(home) = dirs::home_dir() {
        v.push(home.join(".local/share/Steam"));
        v.push(home.join(".steam/steam"));
        v.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    #[cfg(windows)]
    {
        if let Some(p) = registry_steam_path() {
            v.push(p);
        }
        v.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    }
    let mut seen = Vec::new();
    v.into_iter()
        .filter(|p| p.join("steamapps").is_dir())
        .filter(|p| {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            let new = !seen.contains(&c);
            seen.push(c);
            new
        })
        .collect()
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    let out = std::process::Command::new("reg").args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("SteamPath"))?;
    Some(PathBuf::from(line.split("REG_SZ").nth(1)?.trim()))
}

/// Every library folder of every Steam root.
pub fn libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    for root in roots() {
        libs.push(root.clone());
        if let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libs.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
        }
    }
    libs.dedup();
    libs
}

/// The install folder of Steam app `app`, if one of the libraries has it.
pub fn find_app(app: u32) -> Option<PathBuf> {
    libraries().into_iter().find_map(|lib| {
        let manifest = std::fs::read_to_string(lib.join(format!("steamapps/appmanifest_{app}.acf"))).ok()?;
        let dir = vdf_values(&manifest, "installdir").into_iter().next()?;
        let path = lib.join("steamapps/common").join(dir);
        path.is_dir().then_some(path)
    })
}

/// The newest file at `rel` under any Steam user's folder (`userdata/<id>/<rel>`).
pub fn newest_userdata(rel: &Path) -> Option<PathBuf> {
    roots()
        .into_iter()
        .flat_map(|r| std::fs::read_dir(r.join("userdata")).into_iter().flatten().flatten())
        .map(|e| e.path().join(rel))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Values of `"key" "value"` lines in a Valve KeyValues text (`\\` unescaped).
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let words: Vec<&str> = l.split('"').skip(1).step_by(2).collect();
            match words.as_slice() {
                [k, v] if k.eq_ignore_ascii_case(key) => Some(v.replace("\\\\", "\\")),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_library_and_manifest_values() {
        let libs = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"1422450\"\t\t\"123\"\n\t\t}\n\t}\n}\n";
        assert_eq!(vdf_values(libs, "path"), ["D:\\SteamLibrary"]);
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1422450\"\n\t\"installdir\"\t\t\"Deadlock\"\n}\n";
        assert_eq!(vdf_values(acf, "installdir"), ["Deadlock"]);
    }
}

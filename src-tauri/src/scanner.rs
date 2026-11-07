use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredGame {
    pub name: String,
    pub executable: String,
    pub path: String,
    pub source: String,
}

pub fn scan_pc_games() -> Vec<DiscoveredGame> {
    let mut games = Vec::new();
    scan_steam_games(&mut games);
    scan_epic_games(&mut games);
    scan_standalone_amd_games(&mut games);
    scan_gog_games(&mut games);
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games.dedup_by(|a, b| a.executable.eq_ignore_ascii_case(&b.executable));
    info!("PC Game Scan complete: found {} games.", games.len());
    games
}

fn scan_standalone_amd_games(games: &mut Vec<DiscoveredGame>) {
    let local_appdata = match dirs::data_local_dir() {
        Some(d) => d,
        None => return,
    };

    let gmdb_path = local_appdata.join("AMD").join("CN").join("gmdb.blb");
    if !gmdb_path.exists() {
        return;
    }

    if let Ok(content) = fs::read_to_string(&gmdb_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(games_arr) = val.get("games").and_then(|g| g.as_array()) {
                for item in games_arr {
                    let exe_path_str = item.get("exe_path").and_then(|p| p.as_str()).unwrap_or("").trim();
                    if exe_path_str.is_empty() {
                        continue;
                    }

                    let p = Path::new(exe_path_str);
                    if !p.exists() {
                        continue;
                    }

                    let file_name = match p.file_name() {
                        Some(f) => f.to_string_lossy().to_string(),
                        None => continue,
                    };

                    let file_name_lower = file_name.to_lowercase();
                    // Skip non-gaming utilities
                    if file_name_lower == "vlc.exe"
                        || file_name_lower == "wmplayer.exe"
                        || file_name_lower == "resolve.exe"
                        || file_name_lower == "python.exe"
                        || file_name_lower == "pythonw.exe"
                        || file_name_lower == "lm studio.exe"
                        || file_name_lower == "ollama app.exe"
                        || file_name_lower == "ollama.exe"
                        || file_name_lower == "discord.exe"
                        || file_name_lower == "obs64.exe"
                        || file_name_lower == "chrome.exe"
                        || file_name_lower == "firefox.exe"
                        || file_name_lower == "msedge.exe"
                        || file_name_lower == "spotify.exe"
                    {
                        continue;
                    }

                    // Skip steamapps common paths because scan_steam_games already captures them
                    if exe_path_str.to_lowercase().contains("steamapps") {
                        continue;
                    }

                    // Skip epic games manifests already captured
                    if exe_path_str.to_lowercase().contains("epicgames") {
                        continue;
                    }

                    let title = item.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .trim();

                    let game_name = if !title.is_empty() {
                        title.to_string()
                    } else {
                        p.file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| file_name.clone())
                    };

                    games.push(DiscoveredGame {
                        name: game_name,
                        executable: file_name,
                        path: exe_path_str.to_string(),
                        source: "Standalone".to_string(),
                    });
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn scan_gog_games(games: &mut Vec<DiscoveredGame>) {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subpath in &["SOFTWARE\\GOG.com\\Games", "SOFTWARE\\WOW6432Node\\GOG.com\\Games"] {
        if let Ok(key) = hklm.open_subkey(subpath) {
            for subkey_name in key.enum_keys().flatten() {
                if let Ok(game_key) = key.open_subkey(&subkey_name) {
                    let game_name: String = game_key.get_value("gameName").unwrap_or_default();
                    let exe_name: String = game_key.get_value("exe").unwrap_or_default();
                    let path_str: String = game_key.get_value("path").unwrap_or_default();

                    if !game_name.is_empty() && !exe_name.is_empty() {
                        games.push(DiscoveredGame {
                            name: game_name,
                            executable: exe_name,
                            path: path_str,
                            source: "GOG Galaxy".to_string(),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn scan_gog_games(_games: &mut Vec<DiscoveredGame>) {}

fn scan_epic_games(games: &mut Vec<DiscoveredGame>) {
    let manifest_dir = PathBuf::from(
        std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string()),
    )
    .join("Epic")
    .join("EpicGamesLauncher")
    .join("Data")
    .join("Manifests");

    if !manifest_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(manifest_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "item") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let is_incomplete = val
                            .get("bIsIncompleteInstall")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if is_incomplete {
                            continue;
                        }

                        let display_name = val
                            .get("DisplayName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim();

                        if display_name.is_empty() {
                            continue;
                        }

                        // Pick main process name, fallback to launch executable
                        let main_proc = val
                            .get("MainWindowProcessName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim();

                        let launch_exe = val
                            .get("LaunchExecutable")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim();

                        let exe_name = if !main_proc.is_empty() {
                            main_proc
                        } else if !launch_exe.is_empty() {
                            launch_exe
                        } else {
                            continue;
                        };

                        let install_loc = val
                            .get("InstallLocation")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        games.push(DiscoveredGame {
                            name: display_name.to_string(),
                            executable: exe_name.to_string(),
                            path: install_loc,
                            source: "Epic Games".to_string(),
                        });
                    }
                }
            }
        }
    }
}

fn scan_steam_games(games: &mut Vec<DiscoveredGame>) {
    let steam_paths = get_steam_install_paths();

    for steam_dir in steam_paths {
        let vdf_path = steam_dir.join("steamapps").join("libraryfolders.vdf");
        if !vdf_path.exists() {
            continue;
        }

        let libraries = get_steam_library_folders_public(&vdf_path);
        for lib in libraries {
            let steamapps_dir = lib.join("steamapps");
            let common_dir = steamapps_dir.join("common");

            if let Ok(entries) = fs::read_dir(&steamapps_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Some((game_name, install_dir)) = parse_acf(&content) {
                                let game_folder = common_dir.join(&install_dir);
                                if game_folder.exists() {
                                    find_game_executables(&game_name, &game_folder, games);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_steam_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(steam_key) = hkcu.open_subkey("Software\\Valve\\Steam") {
            if let Ok(steam_path) = steam_key.get_value::<String, _>("SteamPath") {
                let p = PathBuf::from(steam_path);
                if p.exists() {
                    paths.push(p);
                }
            }
        }

        // Common default Windows locations
        for drive in ["C", "D", "E", "F", "G", "Z"] {
            let candidate = PathBuf::from(format!("{}:\\Program Files (x86)\\Steam", drive));
            if candidate.exists() && !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let candidates = [
                home.join(".local/share/Steam"),
                home.join(".steam/steam"),
                home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            ];
            for c in candidates {
                if c.exists() && !paths.contains(&c) {
                    paths.push(c);
                }
            }
        }
    }

    paths
}

pub fn get_steam_library_folders_public(vdf_path: &Path) -> Vec<PathBuf> {
    let mut libs = Vec::new();
    if let Ok(content) = fs::read_to_string(vdf_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"path\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    let path_str = parts[3].replace("\\\\", "\\");
                    let p = PathBuf::from(path_str);
                    if p.exists() && !libs.contains(&p) {
                        libs.push(p);
                    }
                }
            }
        }
    }
    libs
}

fn parse_acf(content: &str) -> Option<(String, String)> {
    let mut name = None;
    let mut installdir = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"name\"") {
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 4 {
                name = Some(parts[3].to_string());
            }
        } else if trimmed.starts_with("\"installdir\"") {
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 4 {
                installdir = Some(parts[3].to_string());
            }
        }
    }

    match (name, installdir) {
        (Some(n), Some(dir)) => {
            // Filter out tools, runtimes, and soundtracks
            if n.contains("Soundtrack") || n.contains("Steamworks") || n.contains("Redistributable") {
                None
            } else {
                Some((n, dir))
            }
        }
        _ => None,
    }
}

fn find_game_executables(game_name: &str, folder: &Path, out: &mut Vec<DiscoveredGame>) {
    let mut found_exes = Vec::new();
    collect_exes(folder, 0, &mut found_exes);

    // Filter out common crash handlers, redistributables, and helpers
    found_exes.retain(|p| {
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        !name.contains("crashreport")
            && !name.contains("crashpad")
            && !name.contains("unitycrashhandler")
            && !name.contains("vc_redist")
            && !name.contains("dxsetup")
            && !name.contains("unins")
            && !name.contains("setup")
            && !name.contains("vcredist")
    });

    if found_exes.is_empty() {
        return;
    }

    // Prefer the exe that matches the game name or installdir, or the first primary exe
    let selected = if let Some(best) = found_exes.iter().find(|p| {
        let exe_name = p.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
        game_name.to_lowercase().contains(&exe_name) || exe_name.contains(&game_name.to_lowercase())
    }) {
        best.clone()
    } else {
        found_exes[0].clone()
    };

    let exe_filename = selected.file_name().unwrap_or_default().to_string_lossy().to_string();

    out.push(DiscoveredGame {
        name: game_name.to_string(),
        executable: exe_filename,
        path: selected.to_string_lossy().to_string(),
        source: "Steam".to_string(),
    });
}

fn collect_exes(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 2 || out.len() >= 5 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        let mut subdirs = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                #[cfg(target_os = "windows")]
                let is_exe = p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe"));

                #[cfg(not(target_os = "windows"))]
                let is_exe = true;

                if is_exe {
                    out.push(p);
                }
            } else if p.is_dir() && depth < 2 {
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if !name.contains("content")
                    && !name.contains("data")
                    && !name.contains("texture")
                    && !name.contains("sound")
                    && !name.contains("music")
                    && !name.contains("redist")
                    && !name.contains("directx")
                    && !name.contains("shader")
                    && !name.contains("plugin")
                    && !name.contains("saved")
                    && !name.contains("cache")
                    && !name.contains("support")
                    && !name.contains("mono")
                {
                    subdirs.push(p);
                }
            }
        }

        // If we found executables at root level, stop immediately
        if depth == 0 && !out.is_empty() {
            return;
        }

        for sub in subdirs {
            collect_exes(&sub, depth + 1, out);
            if out.len() >= 5 {
                return;
            }
        }
    }
}

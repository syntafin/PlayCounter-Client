use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeImportCandidate {
    pub game_title: String,
    pub executable: String,
    pub source: String,
    pub duration_seconds: i64,
    pub formatted_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    pub game_title: String,
    pub executable: String,
    pub source: String,
    pub duration_seconds: i64,
}

pub fn scan_available_imports() -> Vec<PlaytimeImportCandidate> {
    let mut candidates = Vec::new();

    // 1. Scan AMD Software: Adrenalin Edition (gmdb.blb)
    scan_amd_adrenalin_playtime(&mut candidates);

    // 2. Scan Steam localconfig.vdf
    scan_steam_playtime(&mut candidates);

    // Sort by duration descending
    candidates.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
    candidates
}

/// AMD Software: Adrenalin stores exact playtime in %LOCALAPPDATA%\AMD\CN\gmdb.blb
fn scan_amd_adrenalin_playtime(candidates: &mut Vec<PlaytimeImportCandidate>) {
    if let Some(local_appdata) = dirs::data_local_dir() {
        let gmdb_path = local_appdata.join("AMD").join("CN").join("gmdb.blb");
        if !gmdb_path.exists() {
            return;
        }

        if let Ok(content) = fs::read_to_string(&gmdb_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(games_arr) = json.get("games").and_then(|g| g.as_array()) {
                    for item in games_arr {
                        let mins = item.get("total_played_mins").and_then(|m| m.as_i64()).unwrap_or(0);
                        if mins <= 0 {
                            continue;
                        }

                        let title = item.get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Unknown Game")
                            .to_string();

                        let exe_path_str = item.get("exe_path")
                            .and_then(|p| p.as_str())
                            .unwrap_or("");

                        let exe_name = if !exe_path_str.is_empty() {
                            Path::new(exe_path_str)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| format!("{}.exe", title.to_lowercase().replace(' ', "")))
                        } else {
                            format!("{}.exe", title.to_lowercase().replace(' ', ""))
                        };

                        // Skip non-gaming utilities like VLC or AI packages if unwanted, or let the user choose in preview
                        if exe_name.eq_ignore_ascii_case("vlc.exe") {
                            continue;
                        }

                        let duration_seconds = mins * 60;
                        let hours = mins / 60;
                        let remaining_mins = mins % 60;
                        let formatted_time = if hours > 0 {
                            format!("{}h {}m", hours, remaining_mins)
                        } else {
                            format!("{}m", remaining_mins)
                        };

                        candidates.push(PlaytimeImportCandidate {
                            game_title: title,
                            executable: exe_name,
                            source: "AMD Adrenalin".to_string(),
                            duration_seconds,
                            formatted_time,
                        });
                    }
                }
            }
        }
    }
}

/// Steam stores playtime in userdata/<SteamID>/config/localconfig.vdf
fn scan_steam_playtime(candidates: &mut Vec<PlaytimeImportCandidate>) {
    let steam_paths = get_steam_paths();

    for steam_dir in steam_paths {
        let userdata_dir = steam_dir.join("userdata");
        if !userdata_dir.exists() {
            continue;
        }

        // Build a mapping of AppID -> (Game Name, Executable) from appmanifest files
        let app_map = build_steam_app_map(&steam_dir);

        if let Ok(user_entries) = fs::read_dir(userdata_dir) {
            for user_entry in user_entries.flatten() {
                let localconfig_path = user_entry.path().join("config").join("localconfig.vdf");
                if !localconfig_path.exists() {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&localconfig_path) {
                    parse_localconfig_playtime(&content, &app_map, candidates);
                }
            }
        }
    }
}

fn parse_localconfig_playtime(
    content: &str,
    app_map: &HashMap<String, (String, String)>,
    candidates: &mut Vec<PlaytimeImportCandidate>,
) {
    let mut current_appid: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for AppID header, e.g. "730"
        if trimmed.starts_with('"') && !trimmed.contains("Playtime") && !trimmed.contains("LastPlayed") {
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 2 {
                let potential_id = parts[1];
                if potential_id.chars().all(|c| c.is_ascii_digit()) {
                    current_appid = Some(potential_id.to_string());
                }
            }
        } else if trimmed.starts_with("\"Playtime\"") {
            if let Some(ref appid) = current_appid {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    if let Ok(mins) = parts[3].parse::<i64>() {
                        if mins > 0 {
                            let (name, exe) = if let Some(info) = app_map.get(appid) {
                                info.clone()
                            } else {
                                (format!("Steam App {}", appid), format!("steam_{}.exe", appid))
                            };

                            let duration_seconds = mins * 60;
                            let hours = mins / 60;
                            let remaining_mins = mins % 60;
                            let formatted_time = if hours > 0 {
                                format!("{}h {}m", hours, remaining_mins)
                            } else {
                                format!("{}m", remaining_mins)
                            };

                            candidates.push(PlaytimeImportCandidate {
                                game_title: name,
                                executable: exe,
                                source: "Steam".to_string(),
                                duration_seconds,
                                formatted_time,
                            });
                        }
                    }
                }
            }
        }
    }
}

fn build_steam_app_map(steam_dir: &Path) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let vdf_path = steam_dir.join("steamapps").join("libraryfolders.vdf");
    if !vdf_path.exists() {
        return map;
    }

    let libraries = crate::scanner::get_steam_library_folders_public(&vdf_path);
    for lib in libraries {
        let steamapps = lib.join("steamapps");
        let common = steamapps.join("common");

        if let Ok(entries) = fs::read_dir(&steamapps) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("appmanifest_") && file_name.ends_with(".acf") {
                    let appid = file_name
                        .trim_start_matches("appmanifest_")
                        .trim_end_matches(".acf")
                        .to_string();

                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Some((name, installdir)) = parse_acf_name_and_dir(&content) {
                            let game_folder = common.join(&installdir);
                            let exe = find_primary_exe(&game_folder).unwrap_or_else(|| {
                                format!("{}.exe", name.to_lowercase().replace(' ', ""))
                            });
                            map.insert(appid, (name, exe));
                        }
                    }
                }
            }
        }
    }
    map
}

fn parse_acf_name_and_dir(content: &str) -> Option<(String, String)> {
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
        (Some(n), Some(dir)) => Some((n, dir)),
        _ => None,
    }
}

fn find_primary_exe(dir: &Path) -> Option<String> {
    if !dir.exists() {
        return None;
    }
    let mut exes = Vec::new();
    collect_exes_shallow(dir, 0, &mut exes);
    exes.retain(|name| {
        let l = name.to_lowercase();
        !l.contains("crash") && !l.contains("redist") && !l.contains("unins") && !l.contains("setup")
    });
    exes.first().cloned()
}

fn collect_exes_shallow(dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth > 2 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_exes_shallow(&p, depth + 1, out);
            } else if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("exe") {
                        if let Some(file_name) = p.file_name() {
                            out.push(file_name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
}

fn get_steam_paths() -> Vec<PathBuf> {
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

        for drive in ["C", "D", "E", "F", "G", "Z"] {
            let candidate = PathBuf::from(format!("{}:\\Program Files (x86)\\Steam", drive));
            if candidate.exists() && !paths.contains(&candidate) {
                paths.push(candidate);
            }
        }
    }

    paths
}

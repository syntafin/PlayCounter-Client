use crate::config::AppConfig;
use crate::database::Database;
use crate::models::{GameSession, ImportSummary, PlayCounterBackup};
use crate::scanner::{scan_pc_games, DiscoveredGame};
use crate::sync::{sync_pending_events, SyncClient};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::info;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub sync_client: Arc<Mutex<Option<SyncClient>>>,
    pub active_game_info: Arc<Mutex<Option<ActiveGameInfo>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveGameInfo {
    pub game: String,
    pub started_at_rfc: String,
    pub session_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub game: String,
    pub total_seconds: i64,
    pub formatted_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerStatus {
    pub active_game: Option<String>,
    pub active_duration_seconds: u64,
    pub api_enabled: bool,
    pub pending_events_count: usize,
    pub watched_games_count: usize,
}

#[tauri::command]
pub fn get_historical_stats(state: State<'_, AppState>) -> Result<Vec<GameStats>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw_stats = db.get_total_playtime_by_game().map_err(|e| e.to_string())?;

    let stats = raw_stats
        .into_iter()
        .map(|(game, total_seconds)| {
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let formatted_time = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };
            GameStats {
                game,
                total_seconds,
                formatted_time,
            }
        })
        .collect();

    Ok(stats)
}

#[tauri::command]
pub fn get_recent_sessions(limit: usize, state: State<'_, AppState>) -> Result<Vec<GameSession>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_recent_sessions(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tracker_status(state: State<'_, AppState>) -> Result<TrackerStatus, String> {
    let (active_game, active_duration_seconds) = {
        let active = state.active_game_info.lock().map_err(|e| e.to_string())?;
        if let Some(info) = &*active {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&info.started_at_rfc) {
                let duration = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds().max(0) as u64;
                (Some(info.game.clone()), duration)
            } else {
                (Some(info.game.clone()), 0)
            }
        } else {
            (None, 0)
        }
    };

    let (pending_events_count, api_enabled, watched_games_count) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let count = db.get_unsynced_events().map(|v| v.len()).unwrap_or(0);
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (count, cfg.api_enabled, cfg.watched_games.len())
    };

    Ok(TrackerStatus {
        active_game,
        active_duration_seconds,
        api_enabled,
        pending_events_count,
        watched_games_count,
    })
}

#[tauri::command]
pub async fn trigger_sync(state: State<'_, AppState>) -> Result<usize, String> {
    let client_opt = {
        let client_lock = state.sync_client.lock().map_err(|e| e.to_string())?;
        client_lock.clone()
    };

    if let Some(client) = client_opt {
        info!("Manual sync triggered from UI.");
        sync_pending_events(&state.db, &client)
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("ERR_API_DISABLED".to_string())
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn save_config(new_config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    info!("Saving updated configuration from UI...");
    new_config.save("config.json").map_err(|e| e.to_string())?;

    let new_client = if new_config.api_enabled {
        if let Some(token) = &new_config.api_token {
            SyncClient::new(new_config.api_base_url.clone(), token.clone()).ok()
        } else {
            None
        }
    } else {
        None
    };

    {
        let mut client_lock = state.sync_client.lock().map_err(|e| e.to_string())?;
        *client_lock = new_client;
    }

    {
        let mut cfg_lock = state.config.lock().map_err(|e| e.to_string())?;
        *cfg_lock = new_config;
    }

    Ok(())
}

#[tauri::command]
pub async fn login_with_token(
    token: String,
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let clean_token = token.trim().to_string();
    if clean_token.is_empty() {
        return Err("ERR_EMPTY_TOKEN".to_string());
    }

    let mut cfg = {
        let lock = state.config.lock().map_err(|e| e.to_string())?;
        lock.clone()
    };

    if let Some(endpoint_url) = url {
        let clean_url = endpoint_url.trim().to_string();
        if !clean_url.is_empty() {
            cfg.api_base_url = clean_url;
        }
    }

    let client = SyncClient::new(cfg.api_base_url.clone(), clean_token.clone())
        .map_err(|e| e.to_string())?;
    let games = client.fetch_remote_games().await.map_err(|e| e.to_string())?;

    cfg.api_token = Some(clean_token);
    cfg.api_enabled = true;
    if !games.is_empty() {
        cfg.watched_games = games.clone();
    }

    cfg.save("config.json").map_err(|e| e.to_string())?;

    {
        let mut client_lock = state.sync_client.lock().map_err(|e| e.to_string())?;
        *client_lock = Some(client);
    }
    {
        let mut cfg_lock = state.config.lock().map_err(|e| e.to_string())?;
        *cfg_lock = cfg;
    }

    Ok(games.len())
}

#[tauri::command]
pub fn logout_api(state: State<'_, AppState>) -> Result<(), String> {
    let mut cfg = {
        let lock = state.config.lock().map_err(|e| e.to_string())?;
        lock.clone()
    };

    cfg.api_enabled = false;
    cfg.api_token = None;
    let _ = cfg.save("config.json");

    {
        let mut client_lock = state.sync_client.lock().map_err(|e| e.to_string())?;
        *client_lock = None;
    }
    {
        let mut cfg_lock = state.config.lock().map_err(|e| e.to_string())?;
        *cfg_lock = cfg;
    }

    Ok(())
}

#[tauri::command]
pub async fn test_api_connection(url: String, token: String) -> Result<usize, String> {
    let client = SyncClient::new(url, token).map_err(|e| e.to_string())?;
    let games = client.fetch_remote_games().await.map_err(|e| e.to_string())?;
    Ok(games.len())
}

#[tauri::command]
pub async fn scan_installed_games() -> Result<Vec<DiscoveredGame>, String> {
    tokio::task::spawn_blocking(move || scan_pc_games())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_custom_game(game: String, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let clean = game.trim().to_string();
    if clean.is_empty() {
        return Err("ERR_EMPTY_NAME".to_string());
    }

    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    if !cfg.watched_games.iter().any(|g| g.eq_ignore_ascii_case(&clean)) {
        cfg.watched_games.push(clean);
        let _ = cfg.save("config.json");
    }
    Ok(cfg.watched_games.clone())
}

#[tauri::command]
pub fn add_multiple_games(games: Vec<String>, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    for game in games {
        let clean = game.trim().to_string();
        if !clean.is_empty() && !cfg.watched_games.iter().any(|g| g.eq_ignore_ascii_case(&clean)) {
            cfg.watched_games.push(clean);
        }
    }
    let _ = cfg.save("config.json");
    Ok(cfg.watched_games.clone())
}

#[tauri::command]
pub fn remove_custom_game(game: String, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.watched_games.retain(|g| !g.eq_ignore_ascii_case(&game));
    let _ = cfg.save("config.json");
    Ok(cfg.watched_games.clone())
}

#[tauri::command]
pub fn remove_multiple_games(games: Vec<String>, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.watched_games.retain(|g| !games.iter().any(|to_rem| to_rem.eq_ignore_ascii_case(g)));
    let _ = cfg.save("config.json");
    Ok(cfg.watched_games.clone())
}

#[tauri::command]
pub async fn scan_playtime_imports() -> Result<Vec<crate::importer::PlaytimeImportCandidate>, String> {
    tokio::task::spawn_blocking(move || crate::importer::scan_available_imports())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apply_playtime_imports(
    items: Vec<crate::importer::ImportEntry>,
    add_to_watched: bool,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut imported_count = 0;

    let mut cfg = if add_to_watched {
        Some(state.config.lock().map_err(|e| e.to_string())?)
    } else {
        None
    };

    for item in &items {
        let game_key = if !item.executable.trim().is_empty() {
            item.executable.trim().to_string()
        } else {
            item.game_title.trim().to_string()
        };

        if game_key.is_empty() {
            continue;
        }

        db.save_imported_playtime(&game_key, &item.source, item.duration_seconds)
            .map_err(|e| e.to_string())?;
        imported_count += 1;

        if let Some(ref mut c) = cfg {
            if !c.watched_games.iter().any(|g| g.eq_ignore_ascii_case(&game_key)) {
                c.watched_games.push(game_key.clone());
            }
        }
    }

    if let Some(c) = cfg {
        let _ = c.save("config.json");
    }

    Ok(imported_count)
}

#[tauri::command]
pub fn pick_game_file() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("Executables (*.exe)", &["exe"])
        .set_title("Spiel-Ausführungsdatei (.exe) auswählen")
        .pick_file();

    Ok(file.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn pick_game_folder() -> Result<Option<String>, String> {
    let folder = rfd::FileDialog::new()
        .set_title("Spiel-Ordner auswählen")
        .pick_folder();

    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn export_backup(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("PlayCounter Backup (*.json)", &["json"])
        .set_file_name(&format!("playcounter-backup-{}.json", chrono::Utc::now().format("%Y-%m-%d")))
        .set_title("PlayCounter Datensicherung speichern")
        .save_file();

    let path = match file {
        Some(p) => p,
        None => return Ok(None),
    };

    let (sessions, playtime_events, imported_playtime) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = db.get_all_sessions().map_err(|e| e.to_string())?;
        let p = db.get_all_playtime_events().map_err(|e| e.to_string())?;
        let i = db.get_all_imported_playtime().map_err(|e| e.to_string())?;
        (s, p, i)
    };

    let watched_games = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.watched_games.clone()
    };

    let backup = PlayCounterBackup {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        watched_games,
        sessions,
        playtime_events,
        imported_playtime,
    };

    let json_data = serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())?;
    std::fs::write(&path, json_data).map_err(|e| format!("Fehler beim Schreiben der Datei: {}", e))?;

    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn import_backup(state: State<'_, AppState>) -> Result<Option<ImportSummary>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("PlayCounter Backup (*.json)", &["json"])
        .set_title("PlayCounter Datensicherung auswählen")
        .pick_file();

    let path = match file {
        Some(p) => p,
        None => return Ok(None),
    };

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Fehler beim Lesen der Sicherungsdatei: {}", e))?;

    let backup: PlayCounterBackup = serde_json::from_str(&content)
        .map_err(|e| format!("Ungültiges Datensicherungs-Format: {}", e))?;

    let mut sessions_restored = 0;
    let mut events_restored = 0;
    let mut imported_playtime_restored = 0;

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for s in &backup.sessions {
            if db.restore_session(s).unwrap_or(false) {
                sessions_restored += 1;
            }
        }
        for e in &backup.playtime_events {
            if db.restore_playtime_event(e).unwrap_or(false) {
                events_restored += 1;
            }
        }
        for ip in &backup.imported_playtime {
            if db.restore_imported_playtime(ip).is_ok() {
                imported_playtime_restored += 1;
            }
        }
    }

    let mut games_restored = 0;
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        for g in backup.watched_games {
            if !cfg.watched_games.iter().any(|existing| existing.eq_ignore_ascii_case(&g)) {
                cfg.watched_games.push(g);
                games_restored += 1;
            }
        }
        if games_restored > 0 {
            let _ = cfg.save("config.json");
        }
    }

    info!(
        "Backup imported from {:?}: {} sessions, {} events, {} imported playtime, {} games",
        path, sessions_restored, events_restored, imported_playtime_restored, games_restored
    );

    Ok(Some(ImportSummary {
        sessions_restored,
        events_restored,
        imported_playtime_restored,
        games_restored,
    }))
}

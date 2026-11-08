// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod database;
mod detector;
mod importer;
mod models;
mod scanner;
mod sync;
mod tracker;

use commands::*;
use config::AppConfig;
use database::Database;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sync::{sync_pending_events, SyncClient};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tracker::GameTracker;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    // 1. Initialize logging
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer().with_target(false).compact())
        .try_init();

    info!("Starting PlayCounter...");

    // 2. Load Configuration
    let config_path = "config.json";
    let config = AppConfig::load_or_create(config_path);

    // 3. Open SQLite Database
    let raw_db = match Database::open(&config.db_path) {
        Ok(db) => db,
        Err(err) => {
            error!("Fatal: Could not open database: {}", err);
            panic!("Database error: {}", err);
        }
    };

    let db = Arc::new(Mutex::new(raw_db));
    let config_shared = Arc::new(Mutex::new(config.clone()));
    let active_game_info = Arc::new(Mutex::new(None));

    // 4. Setup API Sync Client
    let initial_client = if config.api_enabled {
        if let Some(token) = &config.api_token {
            SyncClient::new(config.api_base_url.clone(), token.clone()).ok()
        } else {
            None
        }
    } else {
        None
    };

    let sync_client_shared = Arc::new(Mutex::new(initial_client));

    let app_state = AppState {
        db: Arc::clone(&db),
        config: Arc::clone(&config_shared),
        sync_client: Arc::clone(&sync_client_shared),
        active_game_info: Arc::clone(&active_game_info),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_historical_stats,
            get_recent_sessions,
            get_game_sessions,
            get_tracker_status,
            trigger_sync,
            get_config,
            save_config,
            test_api_connection,
            add_custom_game,
            add_multiple_games,
            remove_custom_game,
            remove_multiple_games,
            scan_installed_games,
            login_with_token,
            logout_api,
            scan_playtime_imports,
            apply_playtime_imports,
            pick_game_file,
            pick_game_folder,
            export_backup,
            import_backup
        ])
        .setup(move |app| {
            let handle = app.handle();

            // Setup System Tray Menu
            let show_i = MenuItem::with_id(handle, "show", "Dashboard öffnen", true, None::<&str>)?;
            let sync_i = MenuItem::with_id(handle, "sync", "Jetzt synchronisieren", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(handle, "quit", "Beenden", true, None::<&str>)?;
            let tray_menu = Menu::with_items(handle, &[&show_i, &sync_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("PlayCounter")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "sync" => {
                        let app_state: tauri::State<AppState> = app.state();
                        let client_opt = {
                            let lock = app_state.sync_client.lock().unwrap();
                            lock.clone()
                        };
                        if let Some(client) = client_opt {
                            let db = Arc::clone(&app_state.db);
                            tauri::async_runtime::spawn(async move {
                                let _ = sync_pending_events(&db, &client).await;
                            });
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(handle)?;

            // Background Tracking Task
            let tracker_db = Arc::clone(&db);
            let tracker_cfg_shared = Arc::clone(&config_shared);
            let tracker_active_info = Arc::clone(&active_game_info);
            let app_handle_for_tick = handle.clone();

            tauri::async_runtime::spawn(async move {
                let init_cfg = {
                    let lock = tracker_cfg_shared.lock().unwrap();
                    lock.clone()
                };

                let mut tracker = GameTracker::new(init_cfg.clone(), tracker_db);
                let scan_interval = Duration::from_secs(init_cfg.scan_interval_secs);
                let mut ticker = tokio::time::interval(scan_interval);

                loop {
                    ticker.tick().await;

                    // Sync watched games list from shared config
                    {
                        if let Ok(lock) = tracker_cfg_shared.lock() {
                            tracker.update_watched_games(lock.watched_games.clone());
                        }
                    }

                    tracker.tick();

                    // Update shared active game state
                    let current_active = tracker.get_primary_active_game().map(|(game, started_at, session_id)| {
                        ActiveGameInfo {
                            game,
                            started_at_rfc: started_at.to_rfc3339(),
                            session_id,
                        }
                    });

                    {
                        let mut lock = tracker_active_info.lock().unwrap();
                        *lock = current_active.clone();
                    }

                    // Emit event to Vue frontend
                    let _ = app_handle_for_tick.emit("tracker-tick", &current_active);
                }
            });

            // Dynamic Background Sync Task
            let sync_db = Arc::clone(&db);
            let sync_cfg_shared = Arc::clone(&config_shared);
            let sync_client_bg = Arc::clone(&sync_client_shared);

            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(30));
                loop {
                    ticker.tick().await;

                    let client_opt = {
                        if let Ok(lock) = sync_client_bg.lock() {
                            lock.clone()
                        } else {
                            None
                        }
                    };

                    if let Some(client) = client_opt {
                        // Attempt sync
                        if let Err(err) = sync_pending_events(&sync_db, &client).await {
                            warn!("Background sync error: {}", err);
                        }

                        // Also refresh remote games periodically
                        if let Ok(remote_games) = client.fetch_remote_games().await {
                            if !remote_games.is_empty() {
                                if let Ok(mut cfg_lock) = sync_cfg_shared.lock() {
                                    if cfg_lock.watched_games != remote_games {
                                        cfg_lock.watched_games = remote_games;
                                        let _ = cfg_lock.save("config.json");
                                    }
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close button (X) hides window to tray instead of quitting the background tracker
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

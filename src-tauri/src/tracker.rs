use crate::config::AppConfig;
use crate::database::Database;
use crate::detector::ProcessDetector;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

struct ActiveGame {
    session_id: i64,
    started_at: DateTime<Utc>,
    last_heartbeat: DateTime<Utc>,
}

pub struct GameTracker {
    config: AppConfig,
    db: Arc<Mutex<Database>>,
    detector: ProcessDetector,
    active_games: HashMap<String, ActiveGame>,
}

impl GameTracker {
    pub fn new(config: AppConfig, db: Arc<Mutex<Database>>) -> Self {
        let detector = ProcessDetector::new();
        Self {
            config,
            db,
            detector,
            active_games: HashMap::new(),
        }
    }

    /// Dynamically updates the watched games list (e.g. from remote API)
    pub fn update_watched_games(&mut self, games: Vec<String>) {
        if !games.is_empty() {
            info!("Updated watched games list ({} games registered)", games.len());
            self.config.watched_games = games;
        }
    }

    /// Single tracking tick (called periodically in loop)
    pub fn tick(&mut self) {
        let now = Utc::now();
        let running_now = self.detector.scan_running_games(&self.config.watched_games);

        // 1. Check for newly started games and existing running games
        for game in &running_now {
            if let Some(active) = self.active_games.get_mut(game) {
                // Game is still running - check if heartbeat interval elapsed
                let elapsed = (now - active.last_heartbeat).num_seconds();
                if elapsed >= self.config.heartbeat_interval_secs as i64 {
                    info!(
                        "💓 HEARTBEAT for '{}' (session #{}, running for {} min)",
                        game,
                        active.session_id,
                        (now - active.started_at).num_minutes()
                    );
                    let db_lock = self.db.lock().unwrap();
                    if let Err(err) = db_lock.record_heartbeat(active.session_id, game, now) {
                        warn!("Failed to record heartbeat for '{}': {}", game, err);
                    }
                    active.last_heartbeat = now;
                }
            } else {
                // Game just started!
                info!("🎮 START detected for '{}'!", game);
                let db_lock = self.db.lock().unwrap();
                match db_lock.start_session(game, now) {
                    Ok(session_id) => {
                        self.active_games.insert(
                            game.clone(),
                            ActiveGame {
                                session_id,
                                started_at: now,
                                last_heartbeat: now,
                            },
                        );
                    }
                    Err(err) => {
                        warn!("Failed to record session start for '{}': {}", game, err);
                    }
                }
            }
        }

        // 2. Check for games that have stopped
        let mut stopped_games = Vec::new();
        for (game, active) in &self.active_games {
            if !running_now.contains(game) {
                stopped_games.push((game.clone(), active.session_id, active.started_at));
            }
        }

        for (game, session_id, started_at) in stopped_games {
            let duration = (now - started_at).num_seconds().max(0);
            let mins = duration / 60;
            let secs = duration % 60;
            info!(
                "🛑 END detected for '{}' (Session #{}, Duration: {}m {}s)",
                game, session_id, mins, secs
            );

            let db_lock = self.db.lock().unwrap();
            if let Err(err) = db_lock.end_session(session_id, &game, now) {
                warn!("Failed to record session end for '{}': {}", game, err);
            }

            self.active_games.remove(&game);
        }
    }

    /// Access database reference for stats or queries
    #[allow(dead_code)]
    pub fn database(&self) -> &Arc<Mutex<Database>> {
        &self.db
    }

    /// Get current list of active games
    #[allow(dead_code)]
    pub fn active_games_count(&self) -> usize {
        self.active_games.len()
    }

    /// Returns the first currently active game and its start time (if any)
    pub fn get_primary_active_game(&self) -> Option<(String, DateTime<Utc>, i64)> {
        self.active_games
            .iter()
            .next()
            .map(|(name, active)| (name.clone(), active.started_at, active.session_id))
    }
}

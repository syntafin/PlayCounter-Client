use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameEventType {
    Start,
    Heartbeat,
    End,
}

impl GameEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameEventType::Start => "start",
            GameEventType::Heartbeat => "heartbeat",
            GameEventType::End => "end",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "start" => Some(GameEventType::Start),
            "heartbeat" => Some(GameEventType::Heartbeat),
            "end" => Some(GameEventType::End),
            _ => None,
        }
    }
}

/// Backward-compatible with the PowerShell tracker's `playtime` table
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeEvent {
    pub id: Option<i64>,
    pub game: String,
    pub event: String,
    pub timestamp: String,
    pub is_synced: i64,
}

/// Enhanced session representation for statistics, dashboards, and crash recovery
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: Option<i64>,
    pub game: String,
    pub start_time: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: i64,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedPlaytimeRecord {
    pub game: String,
    pub source: String,
    pub duration_seconds: i64,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayCounterBackup {
    pub version: u32,
    pub exported_at: String,
    pub client_version: String,
    pub watched_games: Vec<String>,
    pub sessions: Vec<GameSession>,
    pub playtime_events: Vec<PlaytimeEvent>,
    pub imported_playtime: Vec<ImportedPlaytimeRecord>,
}

#[allow(dead_code)]
pub type PlayTrackerBackup = PlayCounterBackup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub sessions_restored: usize,
    pub events_restored: usize,
    pub imported_playtime_restored: usize,
    pub games_restored: usize,
}

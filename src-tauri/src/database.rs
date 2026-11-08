use crate::models::{GameEventType, GameSession, ImportedPlaytimeRecord, PlaytimeEvent};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use std::path::Path;
use tracing::info;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_tables()?;
        db.recover_dangling_sessions()?;
        Ok(db)
    }

    /// In-memory database for testing
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        // 1. Playtime table (100% backward compatible with existing PowerShell tracker)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS playtime (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game TEXT NOT NULL,
                event TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                is_synced INTEGER DEFAULT 0
            );",
            [],
        )?;

        // 2. Sessions table (for aggregated local playtime, statistics, and crash recovery)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game TEXT NOT NULL,
                start_time TEXT NOT NULL,
                last_heartbeat TEXT NOT NULL,
                end_time TEXT,
                duration_seconds INTEGER DEFAULT 0,
                is_synced INTEGER DEFAULT 0
            );",
            [],
        )?;

        // 3. Imported playtime table (preserves historical imports separate from live sessions)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS imported_playtime (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game TEXT NOT NULL,
                source TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                imported_at TEXT NOT NULL,
                UNIQUE(game, source)
            );",
            [],
        )?;

        // Create indexes for fast querying
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_playtime_synced ON playtime(is_synced);",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_game ON sessions(game);",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_imported_game ON imported_playtime(game);",
            [],
        )?;

        Ok(())
    }

    /// Recovers any dangling sessions left open if the daemon or computer was shut down unexpectedly.
    fn recover_dangling_sessions(&self) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "SELECT id, game, start_time, last_heartbeat FROM sessions WHERE end_time IS NULL;",
        )?;

        let dangling: Vec<(i64, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .filter_map(Result::ok)
            .collect();

        for (id, game, start_str, last_hb_str) in dangling {
            let start = DateTime::parse_from_rfc3339(&start_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let last_hb = DateTime::parse_from_rfc3339(&last_hb_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(start);

            let duration = (last_hb - start).num_seconds().max(0);

            info!(
                "Recovering unfinished session #{} for '{}': closed at last heartbeat with duration {}s",
                id, game, duration
            );

            self.conn.execute(
                "UPDATE sessions SET end_time = ?1, duration_seconds = ?2 WHERE id = ?3;",
                params![last_hb.to_rfc3339(), duration, id],
            )?;

            // Also record the synthetic "end" event into playtime
            let ts = last_hb.format("%Y-%m-%d %H:%M:%SZ").to_string();
            let _ = self.record_raw_playtime_event(&game, "end", &ts);
        }

        Ok(())
    }

    /// Records a raw event into the `playtime` table (for API sync)
    pub fn record_raw_playtime_event(&self, game: &str, event: &str, timestamp: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO playtime (game, event, timestamp, is_synced) VALUES (?1, ?2, ?3, 0);",
            params![game, event, timestamp],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Starts a new game session and logs the "start" event
    pub fn start_session(&self, game: &str, now: DateTime<Utc>) -> Result<i64> {
        let now_rfc = now.to_rfc3339();
        let now_formatted = now.format("%Y-%m-%d %H:%M:%SZ").to_string();

        // 1. Insert into sessions
        self.conn.execute(
            "INSERT INTO sessions (game, start_time, last_heartbeat, duration_seconds, is_synced)
             VALUES (?1, ?2, ?2, 0, 0);",
            params![game, now_rfc],
        )?;
        let session_id = self.conn.last_insert_rowid();

        // 2. Insert into playtime event log
        self.record_raw_playtime_event(game, GameEventType::Start.as_str(), &now_formatted)?;

        Ok(session_id)
    }

    /// Updates the heartbeat of an active session and logs the "heartbeat" event
    pub fn record_heartbeat(&self, session_id: i64, game: &str, now: DateTime<Utc>) -> Result<()> {
        let now_rfc = now.to_rfc3339();
        let now_formatted = now.format("%Y-%m-%d %H:%M:%SZ").to_string();

        // Fetch session start time to update duration
        let start_time_str: Option<String> = self
            .conn
            .query_row(
                "SELECT start_time FROM sessions WHERE id = ?1;",
                params![session_id],
                |row| row.get(0),
            )
            .ok();

        let duration = if let Some(start_str) = start_time_str {
            if let Ok(start) = DateTime::parse_from_rfc3339(&start_str) {
                (now - start.with_timezone(&Utc)).num_seconds().max(0)
            } else {
                0
            }
        } else {
            0
        };

        self.conn.execute(
            "UPDATE sessions SET last_heartbeat = ?1, duration_seconds = ?2 WHERE id = ?3;",
            params![now_rfc, duration, session_id],
        )?;

        self.record_raw_playtime_event(game, GameEventType::Heartbeat.as_str(), &now_formatted)?;

        Ok(())
    }

    /// Ends an active session and logs the "end" event
    pub fn end_session(&self, session_id: i64, game: &str, now: DateTime<Utc>) -> Result<i64> {
        let now_rfc = now.to_rfc3339();
        let now_formatted = now.format("%Y-%m-%d %H:%M:%SZ").to_string();

        let start_time_str: Option<String> = self
            .conn
            .query_row(
                "SELECT start_time FROM sessions WHERE id = ?1;",
                params![session_id],
                |row| row.get(0),
            )
            .ok();

        let duration = if let Some(start_str) = start_time_str {
            if let Ok(start) = DateTime::parse_from_rfc3339(&start_str) {
                (now - start.with_timezone(&Utc)).num_seconds().max(0)
            } else {
                0
            }
        } else {
            0
        };

        self.conn.execute(
            "UPDATE sessions SET end_time = ?1, duration_seconds = ?2 WHERE id = ?3;",
            params![now_rfc, duration, session_id],
        )?;

        self.record_raw_playtime_event(game, GameEventType::End.as_str(), &now_formatted)?;

        Ok(duration)
    }

    /// Fetches all unsynced playtime events for API synchronization
    #[allow(dead_code)]
    pub fn get_unsynced_events(&self) -> Result<Vec<PlaytimeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, game, event, timestamp, is_synced 
             FROM playtime 
             WHERE is_synced = 0 
             ORDER BY timestamp ASC;",
        )?;

        let events = stmt
            .query_map([], |row| {
                Ok(PlaytimeEvent {
                    id: Some(row.get(0)?),
                    game: row.get(1)?,
                    event: row.get(2)?,
                    timestamp: row.get(3)?,
                    is_synced: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(events)
    }

    /// Marks events as synced
    #[allow(dead_code)]
    pub fn mark_events_synced(&self, ids: &[i64]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        let mut stmt = self.conn.prepare("UPDATE playtime SET is_synced = 1 WHERE id = ?1;")?;
        for id in ids {
            count += stmt.execute(params![id])?;
        }
        Ok(count)
    }

    /// Aggregates total playtime by game in seconds (combines live sessions and imported historical playtime)
    pub fn get_total_playtime_by_game(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT game, SUM(duration_seconds) as total_duration 
             FROM (
                 SELECT game, duration_seconds FROM sessions
                 UNION ALL
                 SELECT game, duration_seconds FROM imported_playtime
             )
             GROUP BY game 
             ORDER BY total_duration DESC;",
        )?;

        let stats = stmt
            .query_map([], |row| {
                let game: String = row.get(0)?;
                let total: i64 = row.get(1).unwrap_or(0);
                Ok((game, total))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(stats)
    }

    /// Saves or updates imported playtime for a game from an external source (e.g. Steam or AMD Adrenalin)
    pub fn save_imported_playtime(&self, game: &str, source: &str, duration_seconds: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO imported_playtime (game, source, duration_seconds, imported_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(game, source) DO UPDATE SET
                duration_seconds = excluded.duration_seconds,
                imported_at = excluded.imported_at;",
            params![game, source, duration_seconds, now],
        )?;
        Ok(())
    }

    /// Returns the last N completed sessions
    #[allow(dead_code)]
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<GameSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, game, start_time, last_heartbeat, end_time, duration_seconds, is_synced 
             FROM sessions 
             ORDER BY id DESC 
             LIMIT ?1;",
        )?;

        let sessions = stmt
            .query_map(params![limit as i64], |row| {
                let start_str: String = row.get(2)?;
                let hb_str: String = row.get(3)?;
                let end_str: Option<String> = row.get(4)?;

                let start_time = DateTime::parse_from_rfc3339(&start_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let last_heartbeat = DateTime::parse_from_rfc3339(&hb_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or(start_time);
                let end_time = end_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                });

                Ok(GameSession {
                    id: Some(row.get(0)?),
                    game: row.get(1)?,
                    start_time,
                    last_heartbeat,
                    end_time,
                    duration_seconds: row.get(5)?,
                    is_synced: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(sessions)
    }

    pub fn get_sessions_by_game(&self, game_key: &str, limit: usize) -> Result<Vec<GameSession>> {
        let clean = game_key.trim();
        let without_exe = clean.strip_suffix(".exe").unwrap_or(clean);
        let with_exe = format!("{}.exe", without_exe);

        let mut stmt = self.conn.prepare(
            "SELECT id, game, start_time, last_heartbeat, end_time, duration_seconds, is_synced 
             FROM sessions 
             WHERE LOWER(game) = LOWER(?1) OR LOWER(game) = LOWER(?2) OR LOWER(game) = LOWER(?3)
             ORDER BY id DESC 
             LIMIT ?4;",
        )?;

        let sessions = stmt
            .query_map(params![clean, without_exe, with_exe, limit as i64], |row| {
                let start_str: String = row.get(2)?;
                let hb_str: String = row.get(3)?;
                let end_str: Option<String> = row.get(4)?;

                let start_time = DateTime::parse_from_rfc3339(&start_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let last_heartbeat = DateTime::parse_from_rfc3339(&hb_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or(start_time);
                let end_time = end_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                });

                Ok(GameSession {
                    id: Some(row.get(0)?),
                    game: row.get(1)?,
                    start_time,
                    last_heartbeat,
                    end_time,
                    duration_seconds: row.get(5)?,
                    is_synced: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(sessions)
    }

    pub fn get_all_sessions(&self) -> Result<Vec<GameSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, game, start_time, last_heartbeat, end_time, duration_seconds, is_synced 
             FROM sessions 
             ORDER BY id ASC;",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                let start_str: String = row.get(2)?;
                let hb_str: String = row.get(3)?;
                let end_str: Option<String> = row.get(4)?;

                let start_time = DateTime::parse_from_rfc3339(&start_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let last_heartbeat = DateTime::parse_from_rfc3339(&hb_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or(start_time);
                let end_time = end_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                });

                Ok(GameSession {
                    id: Some(row.get(0)?),
                    game: row.get(1)?,
                    start_time,
                    last_heartbeat,
                    end_time,
                    duration_seconds: row.get(5)?,
                    is_synced: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(sessions)
    }

    pub fn get_all_playtime_events(&self) -> Result<Vec<PlaytimeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, game, event, timestamp, is_synced FROM playtime ORDER BY id ASC;",
        )?;

        let events = stmt
            .query_map([], |row| {
                Ok(PlaytimeEvent {
                    id: Some(row.get(0)?),
                    game: row.get(1)?,
                    event: row.get(2)?,
                    timestamp: row.get(3)?,
                    is_synced: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(events)
    }

    pub fn get_all_imported_playtime(&self) -> Result<Vec<ImportedPlaytimeRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT game, source, duration_seconds, imported_at FROM imported_playtime ORDER BY id ASC;",
        )?;

        let list = stmt
            .query_map([], |row| {
                Ok(ImportedPlaytimeRecord {
                    game: row.get(0)?,
                    source: row.get(1)?,
                    duration_seconds: row.get(2)?,
                    imported_at: row.get(3)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(list)
    }

    pub fn restore_session(&self, s: &GameSession) -> Result<bool> {
        let start_str = s.start_time.to_rfc3339();
        let hb_str = s.last_heartbeat.to_rfc3339();
        let end_str = s.end_time.map(|t| t.to_rfc3339());

        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE game = ?1 AND start_time = ?2);",
            params![s.game, start_str],
            |row| row.get(0),
        )?;

        if exists {
            return Ok(false);
        }

        self.conn.execute(
            "INSERT INTO sessions (game, start_time, last_heartbeat, end_time, duration_seconds, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![s.game, start_str, hb_str, end_str, s.duration_seconds, s.is_synced],
        )?;

        Ok(true)
    }

    pub fn restore_playtime_event(&self, e: &PlaytimeEvent) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM playtime WHERE game = ?1 AND event = ?2 AND timestamp = ?3);",
            params![e.game, e.event, e.timestamp],
            |row| row.get(0),
        )?;

        if exists {
            return Ok(false);
        }

        self.conn.execute(
            "INSERT INTO playtime (game, event, timestamp, is_synced)
             VALUES (?1, ?2, ?3, ?4);",
            params![e.game, e.event, e.timestamp, e.is_synced],
        )?;

        Ok(true)
    }

    pub fn restore_imported_playtime(&self, rec: &ImportedPlaytimeRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO imported_playtime (game, source, duration_seconds, imported_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(game, source) DO UPDATE SET
                duration_seconds = MAX(duration_seconds, excluded.duration_seconds),
                imported_at = excluded.imported_at;",
            params![rec.game, rec.source, rec.duration_seconds, rec.imported_at],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_session_lifecycle_and_playtime_events() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        // 1. Start session
        let session_id = db.start_session("cyberpunk2077.exe", now).unwrap();
        assert_eq!(session_id, 1);

        // 2. Record heartbeat after 15 minutes
        let hb_time = now + Duration::minutes(15);
        db.record_heartbeat(session_id, "cyberpunk2077.exe", hb_time).unwrap();

        // 3. End session after 30 minutes
        let end_time = now + Duration::minutes(30);
        let duration = db.end_session(session_id, "cyberpunk2077.exe", end_time).unwrap();
        assert_eq!(duration, 1800); // 30 minutes = 1800s

        // 4. Verify unsynced events in playtime table
        let unsynced = db.get_unsynced_events().unwrap();
        assert_eq!(unsynced.len(), 3);
        assert_eq!(unsynced[0].event, "start");
        assert_eq!(unsynced[1].event, "heartbeat");
        assert_eq!(unsynced[2].event, "end");

        // 5. Verify mark synced
        let ids: Vec<i64> = unsynced.iter().map(|e| e.id.unwrap()).collect();
        let marked = db.mark_events_synced(&ids).unwrap();
        assert_eq!(marked, 3);
        assert_eq!(db.get_unsynced_events().unwrap().len(), 0);
    }

    #[test]
    fn test_crash_recovery() {
        let db = Database::open_in_memory().unwrap();
        let now = Utc::now();

        // Start session without closing it
        let session_id = db.start_session("factorio.exe", now).unwrap();
        let hb_time = now + Duration::minutes(20);
        db.record_heartbeat(session_id, "factorio.exe", hb_time).unwrap();

        // Trigger crash recovery
        db.recover_dangling_sessions().unwrap();

        // Verify session was automatically closed with heartbeat duration
        let sessions = db.get_recent_sessions(1).unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].end_time.is_some());
        assert_eq!(sessions[0].duration_seconds, 1200); // 20 minutes
    }
}

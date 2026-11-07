use crate::database::Database;
use crate::models::PlaytimeEvent;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("API error (Status {status}): {body}")]
    Api { status: u16, body: String },
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Missing or invalid token")]
    MissingToken,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub games: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EventPayload<'a> {
    pub game: &'a str,
    pub event: &'a str,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct SyncClient {
    base_url: String,
    client: Client,
}

impl SyncClient {
    pub fn new(base_url: String, token: String) -> Result<Self, SyncError> {
        let clean_token = token.trim();
        if clean_token.is_empty() {
            return Err(SyncError::MissingToken);
        }

        let mut headers = HeaderMap::new();
        let auth_val = format!("Bearer {}", clean_token);
        let mut header_value = HeaderValue::from_str(&auth_val)
            .map_err(|_| SyncError::MissingToken)?;
        header_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, header_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    /// Fetches the remote games configuration list from GET /config
    #[allow(dead_code)]
    pub async fn fetch_remote_games(&self) -> Result<Vec<String>, SyncError> {
        let url = format!("{}/config", self.base_url);
        debug!("Fetching remote config from {}", url);

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let body_text = response.text().await?;
        if let Ok(cfg) = serde_json::from_str::<ConfigResponse>(&body_text) {
            Ok(cfg.games)
        } else if let Ok(games) = serde_json::from_str::<Vec<String>>(&body_text) {
            Ok(games)
        } else {
            Err(SyncError::Api {
                status: status.as_u16(),
                body: format!("Invalid JSON response from {}: {}", url, body_text),
            })
        }
    }

    /// Sends a single event (start, heartbeat, end) to POST /event
    pub async fn send_event(&self, game: &str, event: &str, raw_timestamp: &str) -> Result<(), SyncError> {
        let url = format!("{}/event", self.base_url);

        // Normalize timestamp: remove 'T' and 'Z' to match the API expectation (e.g., "YYYY-MM-DD HH:mm:ss")
        let clean_ts = raw_timestamp
            .replace('T', " ")
            .replace('Z', "")
            .trim()
            .to_string();

        let payload = EventPayload {
            game,
            event,
            timestamp: clean_ts,
        };

        let response = self.client.post(&url).json(&payload).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SyncError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(())
    }
}

/// Synchronizes all pending local events to the remote API in chronological order
pub async fn sync_pending_events(
    db: &Arc<Mutex<Database>>,
    client: &SyncClient,
) -> Result<usize, SyncError> {
    let pending_events: Vec<PlaytimeEvent> = {
        let db_lock = db.lock().unwrap();
        db_lock.get_unsynced_events()?
    };

    if pending_events.is_empty() {
        return Ok(0);
    }

    info!("🔄 Found {} pending event(s) to synchronize...", pending_events.len());
    let mut synced_count = 0;

    for event in pending_events {
        let event_id = match event.id {
            Some(id) => id,
            None => continue,
        };

        match client.send_event(&event.game, &event.event, &event.timestamp).await {
            Ok(_) => {
                {
                    let db_lock = db.lock().unwrap();
                    let _ = db_lock.mark_events_synced(&[event_id])?;
                }
                synced_count += 1;
                debug!("✅ Synced event #{} ({} - {})", event_id, event.game, event.event);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(err) => {
                error!("❌ Sync failed for event #{} ({} - {}): {}", event_id, event.game, event.event, err);
                // Stop immediately to preserve event sequence
                return Err(err);
            }
        }
    }

    info!("🎉 Successfully synchronized {} event(s).", synced_count);
    Ok(synced_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_client_missing_token() {
        let result = SyncClient::new("https://api.syntafin.de".to_string(), "   ".to_string());
        assert!(matches!(result, Err(SyncError::MissingToken)));
    }

    #[test]
    fn test_config_response_deserialization() {
        let json_obj = r#"{"games":["cyberpunk2077.exe","factorio.exe"]}"#;
        let parsed: ConfigResponse = serde_json::from_str(json_obj).unwrap();
        assert_eq!(parsed.games.len(), 2);
        assert_eq!(parsed.games[0], "cyberpunk2077.exe");

        let json_arr = r#"["cyberpunk2077.exe","factorio.exe"]"#;
        let parsed_arr: Vec<String> = serde_json::from_str(json_arr).unwrap();
        assert_eq!(parsed_arr.len(), 2);
    }

    #[test]
    fn test_timestamp_cleaning() {
        let raw = "2026-11-07T19:05:00Z";
        let cleaned = raw.replace('T', " ").replace('Z', "").trim().to_string();
        assert_eq!(cleaned, "2026-11-07 19:05:00");
    }
}


use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Path to the SQLite database
    pub db_path: PathBuf,
    /// Process scan interval in seconds (default: 15)
    pub scan_interval_secs: u64,
    /// Heartbeat interval in seconds (default: 900 = 15 min)
    pub heartbeat_interval_secs: u64,
    /// API sync interval in seconds (default: 30)
    pub sync_interval_secs: u64,
    /// Remote config refresh interval in seconds (default: 3600 = 1 hour)
    pub config_refresh_interval_secs: u64,
    /// List of executable names or game process names to monitor
    pub watched_games: Vec<String>,
    /// Whether API synchronization is enabled
    pub api_enabled: bool,
    /// Base URL for the remote sync API
    pub api_base_url: String,
    /// Auth token / secret for API communication
    pub api_token: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("gaming.db"),
            scan_interval_secs: 15,
            heartbeat_interval_secs: 900,
            sync_interval_secs: 30,
            config_refresh_interval_secs: 3600,
            watched_games: vec![],
            api_enabled: false,
            api_base_url: "https://api.syntafin.de/v1/game".to_string(),
            api_token: None,
        }
    }
}

impl AppConfig {
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let mut config = if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                    Ok(cfg) => {
                        info!("Loaded configuration from {}", path.display());
                        cfg
                    }
                    Err(err) => {
                        warn!(
                            "Failed to parse config file: {}. Falling back to default.",
                            err
                        );
                        AppConfig::default()
                    }
                },
                Err(err) => {
                    warn!(
                        "Failed to read config file: {}. Falling back to default.",
                        err
                    );
                    AppConfig::default()
                }
            }
        } else {
            let default_config = AppConfig::default();
            if let Ok(json) = serde_json::to_string_pretty(&default_config) {
                let _ = fs::write(path, json);
                info!("Created default configuration at {}", path.display());
            }
            default_config
        };

        // Automatic fallback resolution for token and api_enabled
        config.resolve_token_and_flags();
        config
    }

    /// Automatically discovers token from secret.conf or api_enabled.txt for seamless migration
    pub fn resolve_token_and_flags(&mut self) {
        // 1. Resolve token if not explicitly configured
        if self.api_token.as_ref().map_or(true, |t| t.trim().is_empty()) {
            let possible_secret_paths = [
                PathBuf::from("secret.conf"),
                PathBuf::from("../PlayTracker/secret.conf"),
                PathBuf::from("../../PlayTracker/secret.conf"),
            ];

            for secret_path in &possible_secret_paths {
                if secret_path.exists() {
                    if let Ok(token) = fs::read_to_string(secret_path) {
                        let trimmed = token.trim().to_string();
                        if !trimmed.is_empty() {
                            info!("Loaded API token from {}", secret_path.display());
                            self.api_token = Some(trimmed);
                            break;
                        }
                    }
                }
            }
        }

        // 2. Check for api_enabled.txt flag
        let possible_flag_paths = [
            PathBuf::from("api_enabled.txt"),
            PathBuf::from("../PlayTracker/api_enabled.txt"),
        ];

        for flag_path in &possible_flag_paths {
            if flag_path.exists() {
                info!("Found {} -> API synchronization enabled.", flag_path.display());
                self.api_enabled = true;
                break;
            }
        }
    }

    #[allow(dead_code)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }
}

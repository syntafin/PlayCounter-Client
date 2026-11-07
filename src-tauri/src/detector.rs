use std::collections::HashSet;
use sysinfo::{ProcessesToUpdate, System};
use tracing::debug;

pub struct ProcessDetector {
    system: System,
}

impl ProcessDetector {
    pub fn new() -> Self {
        let mut system = System::new().expect("Failed to initialize sysinfo system");
        system.refresh_processes(ProcessesToUpdate::All, true);
        Self { system }
    }

    /// Scans the currently running system processes and returns the set of matching games.
    /// Handles cross-platform differences (case-insensitivity, .exe suffixes on Windows).
    pub fn scan_running_games(&mut self, watched_games: &[String]) -> HashSet<String> {
        // Refresh only process list for high performance & minimal CPU
        self.system.refresh_processes(ProcessesToUpdate::All, true);

        let mut running_matches = HashSet::new();

        // Collect all running process names (normalized to lowercase)
        let running_processes: HashSet<String> = self
            .system
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();

        for watched in watched_games {
            let watched_norm = watched.to_lowercase();
            let watched_file_name = std::path::Path::new(&watched_norm)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| watched_norm.clone());
            let watched_without_exe = watched_file_name.trim_end_matches(".exe");

            // Check direct match or match with/without .exe or full path
            let is_running = running_processes.iter().any(|proc_name| {
                let proc_norm = proc_name.to_lowercase();
                let proc_without_exe = proc_norm.trim_end_matches(".exe");

                proc_norm == watched_norm 
                    || proc_norm == watched_file_name 
                    || proc_without_exe == watched_without_exe
            });

            if is_running {
                debug!("Detected running game: {}", watched);
                running_matches.insert(watched.clone());
            }
        }

        running_matches
    }
}

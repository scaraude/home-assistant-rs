//! Service that watches log files and emits new entries via the event bus.

use crate::events::SystemEvent;
use crate::logs::{LogEntry, LogFile};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Tracks state for a single log file
struct LogFileState {
    /// Last known file position
    position: u64,
    /// Last timestamp we saw (to avoid duplicate entries)
    last_timestamp: Option<DateTime<Utc>>,
}

/// Service that watches log files for new entries and broadcasts them
pub struct LogWatcherService {
    event_tx: broadcast::Sender<SystemEvent>,
    log_dir: PathBuf,
    file_states: HashMap<LogFile, LogFileState>,
    poll_interval: Duration,
}

impl LogWatcherService {
    pub fn new(event_tx: broadcast::Sender<SystemEvent>) -> Self {
        let log_dir = Self::get_log_dir();
        Self {
            event_tx,
            log_dir,
            file_states: HashMap::new(),
            poll_interval: Duration::from_secs(60), // Match monitor.sh interval
        }
    }

    fn get_log_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
        PathBuf::from(".")
    }

    /// Run the watcher loop
    pub async fn run(mut self) {
        info!("LogWatcherService started");

        // Initialize file states
        for log_file in LogFile::ALL {
            self.file_states.insert(
                log_file,
                LogFileState {
                    position: 0,
                    last_timestamp: None,
                },
            );
        }

        // Seek to end of all files on startup (don't replay history)
        self.seek_to_end_all();

        let mut ticker = interval(self.poll_interval);

        loop {
            ticker.tick().await;
            self.check_all_files();
        }
    }

    /// Seek to end of all log files
    fn seek_to_end_all(&mut self) {
        for log_file in LogFile::ALL {
            let path = self.log_dir.join(log_file.filename());
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Some(state) = self.file_states.get_mut(&log_file) {
                    state.position = metadata.len();
                    debug!(
                        file = %log_file.filename(),
                        position = state.position,
                        "Initialized log file position"
                    );
                }
            }
        }
    }

    /// Check all log files for new entries
    fn check_all_files(&mut self) {
        for log_file in LogFile::ALL {
            if let Err(e) = self.check_file(log_file) {
                warn!(
                    file = %log_file.filename(),
                    error = %e,
                    "Error checking log file"
                );
            }
        }
    }

    /// Check a single log file for new entries
    fn check_file(&mut self, log_file: LogFile) -> Result<(), String> {
        let path = self.log_dir.join(log_file.filename());

        if !path.exists() {
            return Ok(());
        }

        let state = self
            .file_states
            .get_mut(&log_file)
            .ok_or("Missing file state")?;

        // Check if file has grown
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        let current_size = metadata.len();

        if current_size <= state.position {
            // File hasn't grown (or was truncated/rotated)
            if current_size < state.position {
                // File was truncated or rotated, reset position
                state.position = 0;
                state.last_timestamp = None;
                debug!(
                    file = %log_file.filename(),
                    "Log file was rotated, resetting position"
                );
            }
            return Ok(());
        }

        // Read new entries
        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(state.position))
            .map_err(|e| e.to_string())?;

        let reader = BufReader::new(file);
        let mut new_entries = Vec::new();
        let mut new_position = state.position;

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            new_position += line.len() as u64 + 1; // +1 for newline

            // Skip header and empty lines
            if line.starts_with("Timestamp") || line.trim().is_empty() {
                continue;
            }

            // Parse the entry based on file type
            if let Some(entry) = Self::parse_line(log_file, &line) {
                // Check if this entry is newer than the last one we saw
                let entry_ts = Self::get_entry_timestamp(&entry);
                if let Some(last_ts) = state.last_timestamp {
                    if entry_ts <= last_ts {
                        continue;
                    }
                }
                state.last_timestamp = Some(entry_ts);
                new_entries.push(entry);
            }
        }

        // Update position
        state.position = new_position;

        // Broadcast new entries if any
        if !new_entries.is_empty() {
            debug!(
                file = %log_file.filename(),
                count = new_entries.len(),
                "Broadcasting new log entries"
            );

            let event = SystemEvent::LogEntries {
                log_file,
                entries: new_entries,
                timestamp: Utc::now(),
            };

            if let Err(e) = self.event_tx.send(event) {
                error!(
                    file = %log_file.filename(),
                    error = %e,
                    "Failed to broadcast log entries"
                );
            }
        }

        Ok(())
    }

    /// Parse a log line into a LogEntry
    fn parse_line(log_file: LogFile, line: &str) -> Option<LogEntry> {
        let parts: Vec<&str> = line.split(',').collect();

        match log_file {
            LogFile::SystemMonitor => {
                if parts.len() >= 6 {
                    Some(LogEntry::SystemMonitor(crate::logs::SystemMonitorEntry {
                        timestamp: parts[0].to_string(),
                        cpu_usage: parts[1].parse().unwrap_or(0.0),
                        ram_used: parts[2].parse().unwrap_or(0.0),
                        ram_total: parts[3].parse().unwrap_or(0.0),
                        ram_usage: parts[4].parse().unwrap_or(0.0),
                        cpu_temp: parts[5].parse().unwrap_or(0.0),
                    }))
                } else {
                    None
                }
            }
            LogFile::ProcessMonitor => {
                if parts.len() >= 6 {
                    Some(LogEntry::ProcessMonitor(crate::logs::ProcessMonitorEntry {
                        timestamp: parts[0].to_string(),
                        process: parts[1].to_string(),
                        pid: parts[2].to_string(),
                        cpu: parts[3].parse().unwrap_or(0.0),
                        ram: parts[4].parse().unwrap_or(0.0),
                        status: parts[5].to_string(),
                    }))
                } else {
                    None
                }
            }
            LogFile::TopCpuConsumers | LogFile::TopRamConsumers => {
                if parts.len() >= 6 {
                    Some(LogEntry::TopConsumer(crate::logs::TopConsumerEntry {
                        timestamp: parts[0].to_string(),
                        rank: parts[1].parse().unwrap_or(0),
                        process: parts[2].to_string(),
                        pid: parts[3].to_string(),
                        cpu: parts[4].parse().unwrap_or(0.0),
                        ram: parts[5].parse().unwrap_or(0.0),
                    }))
                } else {
                    None
                }
            }
        }
    }

    /// Extract timestamp from a log entry
    fn get_entry_timestamp(entry: &LogEntry) -> DateTime<Utc> {
        let ts_str = match entry {
            LogEntry::SystemMonitor(e) => &e.timestamp,
            LogEntry::ProcessMonitor(e) => &e.timestamp,
            LogEntry::TopConsumer(e) => &e.timestamp,
        };

        NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S")
            .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
            .unwrap_or_else(|_| Utc::now())
    }
}

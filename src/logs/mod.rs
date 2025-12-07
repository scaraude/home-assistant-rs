use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Allowed log files (whitelist for security)
const ALLOWED_LOG_FILES: &[&str] = &[
    "system_monitor.log",
    "process_monitor.log",
    "top_cpu_consumers.log",
    "top_ram_consumers.log",
];

/// Log file metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct LogFileInfo {
    pub name: String,
    pub display_name: String,
}

/// System monitor log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMonitorEntry {
    pub timestamp: String,
    pub cpu_usage: f32,
    pub ram_used: f32,
    pub ram_total: f32,
    pub ram_usage: f32,
    pub cpu_temp: f32,
}

/// Process monitor log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessMonitorEntry {
    pub timestamp: String,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
    pub status: String,
}

/// Top consumer log entry (CPU or RAM)
#[derive(Debug, Serialize, Deserialize)]
pub struct TopConsumerEntry {
    pub timestamp: String,
    pub rank: i32,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
}

/// Generic log entry for JSON response
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogEntry {
    SystemMonitor(SystemMonitorEntry),
    ProcessMonitor(ProcessMonitorEntry),
    TopConsumer(TopConsumerEntry),
}

/// Get list of available log files
pub fn list_log_files() -> Vec<LogFileInfo> {
    ALLOWED_LOG_FILES
        .iter()
        .map(|name| LogFileInfo {
            name: name.to_string(),
            display_name: format_display_name(name),
        })
        .collect()
}

/// Format log file name for display
fn format_display_name(filename: &str) -> String {
    filename
        .replace(".log", "")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validate log file name against whitelist
pub fn is_valid_log_file(filename: &str) -> bool {
    // Check against whitelist
    if !ALLOWED_LOG_FILES.contains(&filename) {
        return false;
    }

    // Prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return false;
    }

    true
}

/// Get the log directory path
fn get_log_dir() -> PathBuf {
    // Try HOME environment variable first
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home);
    }

    // Fallback to current directory
    PathBuf::from(".")
}

/// Read log file and return last N lines as parsed entries
pub fn read_log_file(filename: &str, max_lines: usize) -> Result<Vec<LogEntry>, String> {
    // Validate filename
    if !is_valid_log_file(filename) {
        return Err(format!("Invalid log file name: {}", filename));
    }

    // Build full path
    let log_dir = get_log_dir();
    let file_path = log_dir.join(filename);

    // Check if file exists
    if !file_path.exists() {
        return Err(format!("Log file not found: {}", filename));
    }

    // Read file and collect lines
    let file = File::open(&file_path).map_err(|e| format!("Failed to open log file: {}", e))?;
    let reader = BufReader::new(file);

    // Collect all lines (skip header if present)
    let mut lines: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.starts_with("Timestamp") && !line.trim().is_empty())
        .collect();

    // Take last N lines
    let start_index = if lines.len() > max_lines {
        lines.len() - max_lines
    } else {
        0
    };
    lines = lines[start_index..].to_vec();

    // Parse based on file type
    let entries = match filename {
        "system_monitor.log" => parse_system_monitor_log(&lines)?,
        "process_monitor.log" => parse_process_monitor_log(&lines)?,
        "top_cpu_consumers.log" | "top_ram_consumers.log" => parse_top_consumers_log(&lines)?,
        _ => return Err(format!("Unknown log file type: {}", filename)),
    };

    Ok(entries)
}

/// Parse system monitor log lines
fn parse_system_monitor_log(lines: &[String]) -> Result<Vec<LogEntry>, String> {
    let mut entries = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            entries.push(LogEntry::SystemMonitor(SystemMonitorEntry {
                timestamp: parts[0].to_string(),
                cpu_usage: parts[1].parse().unwrap_or(0.0),
                ram_used: parts[2].parse().unwrap_or(0.0),
                ram_total: parts[3].parse().unwrap_or(0.0),
                ram_usage: parts[4].parse().unwrap_or(0.0),
                cpu_temp: parts[5].parse().unwrap_or(0.0),
            }));
        }
    }

    Ok(entries)
}

/// Parse process monitor log lines
fn parse_process_monitor_log(lines: &[String]) -> Result<Vec<LogEntry>, String> {
    let mut entries = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            entries.push(LogEntry::ProcessMonitor(ProcessMonitorEntry {
                timestamp: parts[0].to_string(),
                process: parts[1].to_string(),
                pid: parts[2].to_string(),
                cpu: parts[3].parse().unwrap_or(0.0),
                ram: parts[4].parse().unwrap_or(0.0),
                status: parts[5].to_string(),
            }));
        }
    }

    Ok(entries)
}

/// Parse top consumers log lines (CPU or RAM)
fn parse_top_consumers_log(lines: &[String]) -> Result<Vec<LogEntry>, String> {
    let mut entries = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            entries.push(LogEntry::TopConsumer(TopConsumerEntry {
                timestamp: parts[0].to_string(),
                rank: parts[1].parse().unwrap_or(0),
                process: parts[2].to_string(),
                pid: parts[3].to_string(),
                cpu: parts[4].parse().unwrap_or(0.0),
                ram: parts[5].parse().unwrap_or(0.0),
            }));
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_log_file() {
        assert!(is_valid_log_file("system_monitor.log"));
        assert!(is_valid_log_file("process_monitor.log"));
        assert!(!is_valid_log_file("../etc/passwd"));
        assert!(!is_valid_log_file("system_monitor.log/../passwd"));
        assert!(!is_valid_log_file("invalid.log"));
    }

    #[test]
    fn test_format_display_name() {
        assert_eq!(format_display_name("system_monitor.log"), "System Monitor");
        assert_eq!(
            format_display_name("process_monitor.log"),
            "Process Monitor"
        );
        assert_eq!(
            format_display_name("top_cpu_consumers.log"),
            "Top Cpu Consumers"
        );
    }
}

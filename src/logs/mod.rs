use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tracing::debug;

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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMonitorEntry {
    pub timestamp: String,
    pub cpu_usage: f32,
    pub ram_used: f32,
    pub ram_total: f32,
    pub ram_usage: f32,
    pub cpu_temp: f32,
}

/// Process monitor log entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessMonitorEntry {
    pub timestamp: String,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
    pub status: String,
}

/// Top consumer log entry (CPU or RAM)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopConsumerEntry {
    pub timestamp: String,
    pub rank: i32,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
}

/// Generic log entry for JSON response
#[derive(Debug, Serialize, Deserialize, Clone)]
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

/// Read log file with offset support for delta updates
/// Returns (entries, total_line_count)
pub fn read_log_file_with_offset(
    filename: &str,
    since_line: Option<usize>,
    max_lines: usize,
) -> Result<(Vec<LogEntry>, usize), String> {
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
    let all_lines: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.starts_with("Timestamp") && !line.trim().is_empty())
        .collect();

    let total_lines = all_lines.len();

    // Apply offset and limit
    let lines = if let Some(offset) = since_line {
        // Delta mode: return lines after offset
        if offset < all_lines.len() {
            all_lines[offset..].to_vec()
        } else {
            // Offset beyond file size, return empty
            Vec::new()
        }
    } else {
        // Full mode: take last N lines
        let start_index = if all_lines.len() > max_lines {
            all_lines.len() - max_lines
        } else {
            0
        };
        all_lines[start_index..].to_vec()
    };

    // Parse based on file type
    let entries = match filename {
        "system_monitor.log" => parse_system_monitor_log(&lines)?,
        "process_monitor.log" => parse_process_monitor_log(&lines)?,
        "top_cpu_consumers.log" | "top_ram_consumers.log" => parse_top_consumers_log(&lines)?,
        _ => return Err(format!("Unknown log file type: {}", filename)),
    };

    Ok((entries, total_lines))
}

/// Read process history for a specific process/PID combination
/// Returns only entries matching the specified process and PID
pub fn read_process_history(
    process_name: &str,
    pid: &str,
    max_lines: usize,
) -> Result<Vec<LogEntry>, String> {
    let filename = "process_monitor.log";

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

    // Read only last N lines efficiently using a circular buffer
    let file = File::open(&file_path).map_err(|e| format!("Failed to open log file: {}", e))?;
    let reader = BufReader::new(file);

    // Use a circular buffer to keep only the last max_lines
    let mut buffer: Vec<String> = Vec::with_capacity(max_lines);
    let mut count = 0;

    for line in reader.lines().filter_map(|l| l.ok()) {
        // Skip header and empty lines
        if line.starts_with("Timestamp") || line.trim().is_empty() {
            continue;
        }

        if buffer.len() < max_lines {
            buffer.push(line);
        } else {
            // Circular buffer: replace oldest entry
            buffer[count % max_lines] = line;
        }
        count += 1;
    }

    // If we wrapped around, reorder the buffer
    let lines = if count > max_lines {
        let start_idx = count % max_lines;
        let mut reordered = Vec::with_capacity(max_lines);
        reordered.extend_from_slice(&buffer[start_idx..]);
        reordered.extend_from_slice(&buffer[..start_idx]);
        reordered
    } else {
        buffer
    };

    // Parse and filter for specific process/PID
    let all_entries = parse_process_monitor_log(&lines)?;

    debug!(
        total_lines_read = count,
        parsed_entries = all_entries.len(),
        process = %process_name,
        pid = %pid,
        "read_process_history: filtering entries"
    );

    let filtered_entries: Vec<LogEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            if let LogEntry::ProcessMonitor(pm) = entry {
                // Only filter by process name, not PID
                // PIDs change when processes restart, but we want full history
                pm.process == process_name
            } else {
                false
            }
        })
        .collect();

    debug!(
        filtered_entries = filtered_entries.len(),
        "read_process_history: completed filtering (by process name only)"
    );

    Ok(filtered_entries)
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

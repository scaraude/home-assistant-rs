use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::debug;

/// Enumerates supported log files, preventing stringly-typed lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFile {
    SystemMonitor,
    ProcessMonitor,
    TopCpuConsumers,
    TopRamConsumers,
}

impl LogFile {
    pub const ALL: [LogFile; 4] = [
        LogFile::SystemMonitor,
        LogFile::ProcessMonitor,
        LogFile::TopCpuConsumers,
        LogFile::TopRamConsumers,
    ];

    pub fn filename(self) -> &'static str {
        match self {
            LogFile::SystemMonitor => "system_monitor.log",
            LogFile::ProcessMonitor => "process_monitor.log",
            LogFile::TopCpuConsumers => "top_cpu_consumers.log",
            LogFile::TopRamConsumers => "top_ram_consumers.log",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            LogFile::SystemMonitor => "System Monitor",
            LogFile::ProcessMonitor => "Process Monitor",
            LogFile::TopCpuConsumers => "Top CPU Consumers",
            LogFile::TopRamConsumers => "Top RAM Consumers",
        }
    }
}

impl FromStr for LogFile {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LogFile::ALL
            .iter()
            .copied()
            .find(|lf| lf.filename() == s)
            .ok_or(())
    }
}

/// Log file metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct LogFileInfo {
    pub name: String,
    pub display_name: String,
}

/// System monitor log entry
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SystemMonitorEntry {
    pub timestamp: String,
    pub cpu_usage: f32,
    pub ram_used: f32,
    pub ram_total: f32,
    pub ram_usage: f32,
    pub cpu_temp: f32,
}

/// Process monitor log entry
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProcessMonitorEntry {
    pub timestamp: String,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
    pub status: String,
}

/// Top consumer log entry (CPU or RAM)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TopConsumerEntry {
    pub timestamp: String,
    pub rank: i32,
    pub process: String,
    pub pid: String,
    pub cpu: f32,
    pub ram: f32,
}

/// Generic log entry for JSON response
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum LogEntry {
    SystemMonitor(SystemMonitorEntry),
    ProcessMonitor(ProcessMonitorEntry),
    TopConsumer(TopConsumerEntry),
}

/// Get list of available log files
pub fn list_log_files() -> Vec<LogFileInfo> {
    LogFile::ALL
        .iter()
        .map(|log_file| LogFileInfo {
            name: log_file.filename().to_string(),
            display_name: log_file.display_name().to_string(),
        })
        .collect()
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

/// Read log file entries since a specific timestamp (efficient reverse-read from end of file)
/// Returns entries from `since_timestamp` up to now, reading backwards from file end.
pub fn read_log_file_since(
    filename: &str,
    since_timestamp: DateTime<Utc>,
) -> Result<Vec<LogEntry>, String> {
    let log_file =
        LogFile::from_str(filename).map_err(|_| format!("Invalid log file name: {}", filename))?;

    let log_dir = get_log_dir();
    let file_path = log_dir.join(log_file.filename());

    if !file_path.exists() {
        return Err(format!("Log file not found: {}", filename));
    }

    // Read lines from end of file until we hit the cutoff timestamp
    let lines = read_lines_from_end_until(&file_path, since_timestamp)?;

    // Parse based on file type
    let entries = match log_file {
        LogFile::SystemMonitor => parse_system_monitor_log(&lines)?,
        LogFile::ProcessMonitor => parse_process_monitor_log(&lines)?,
        LogFile::TopCpuConsumers | LogFile::TopRamConsumers => parse_top_consumers_log(&lines)?,
    };

    Ok(entries)
}

/// Efficiently read lines from end of file until timestamp cutoff
/// Uses reverse reading with chunked I/O to avoid loading entire file
fn read_lines_from_end_until(
    file_path: &PathBuf,
    cutoff: DateTime<Utc>,
) -> Result<Vec<String>, String> {
    let mut file = File::open(file_path).map_err(|e| format!("Failed to open log file: {}", e))?;
    let file_size = file
        .metadata()
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();

    if file_size == 0 {
        return Ok(Vec::new());
    }

    const CHUNK_SIZE: u64 = 64 * 1024; // 64KB chunks
    let mut result_lines: Vec<String> = Vec::new();
    let mut position = file_size;
    let mut leftover = String::new();

    while position > 0 {
        // Calculate chunk to read
        let chunk_start = position.saturating_sub(CHUNK_SIZE);
        let chunk_len = (position - chunk_start) as usize;

        // Seek and read chunk
        file.seek(SeekFrom::Start(chunk_start))
            .map_err(|e| format!("Failed to seek: {}", e))?;

        let mut buffer = vec![0u8; chunk_len];
        file.read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read: {}", e))?;

        // Convert to string and prepend leftover from previous chunk
        let chunk_str = String::from_utf8_lossy(&buffer);
        let combined = format!("{}{}", chunk_str, leftover);

        // Split into lines (in reverse since we're reading backwards)
        let mut lines: Vec<&str> = combined.lines().collect();

        // If we're not at the start of file, the first line might be partial
        if chunk_start > 0 && !lines.is_empty() {
            leftover = lines.remove(0).to_string();
        } else {
            leftover.clear();
        }

        // Process lines in reverse order (newest first in our result)
        for line in lines.into_iter().rev() {
            // Skip header and empty lines
            if line.starts_with("Timestamp") || line.trim().is_empty() {
                continue;
            }

            // Parse timestamp from line to check cutoff
            if let Some(ts) = parse_timestamp_from_line(line) {
                if ts < cutoff {
                    // We've gone past our cutoff, reverse result and return
                    result_lines.reverse();
                    return Ok(result_lines);
                }
            }

            result_lines.push(line.to_string());
        }

        position = chunk_start;
    }

    // Handle any remaining leftover at the start of file
    if !leftover.is_empty() && !leftover.starts_with("Timestamp") && !leftover.trim().is_empty() {
        if let Some(ts) = parse_timestamp_from_line(&leftover) {
            if ts >= cutoff {
                result_lines.push(leftover);
            }
        }
    }

    // Reverse to get chronological order
    result_lines.reverse();
    Ok(result_lines)
}

/// Parse timestamp from the start of a CSV log line
/// Format: "YYYY-MM-DD HH:MM:SS,..."
fn parse_timestamp_from_line(line: &str) -> Option<DateTime<Utc>> {
    let timestamp_str = line.split(',').next()?;
    let naive = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").ok()?;
    Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

/// Read log file with offset support for delta updates (legacy API for compatibility)
/// Returns (entries, total_line_count)
pub fn read_log_file_with_offset(
    filename: &str,
    since_line: Option<usize>,
    max_lines: usize,
) -> Result<(Vec<LogEntry>, usize), String> {
    let log_file =
        LogFile::from_str(filename).map_err(|_| format!("Invalid log file name: {}", filename))?;

    // Build full path
    let log_dir = get_log_dir();
    let file_path = log_dir.join(log_file.filename());

    // Check if file exists
    if !file_path.exists() {
        return Err(format!("Log file not found: {}", filename));
    }

    // For delta mode with since_line, use the legacy full-read approach
    // This is rarely used now that frontend uses timestamp-based fetching
    if since_line.is_some() {
        let file = File::open(&file_path).map_err(|e| format!("Failed to open log file: {}", e))?;
        let reader = BufReader::new(file);

        let all_lines: Vec<String> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.starts_with("Timestamp") && !line.trim().is_empty())
            .collect();

        let total_lines = all_lines.len();
        let offset = since_line.unwrap();
        let lines = if offset < all_lines.len() {
            all_lines[offset..].to_vec()
        } else {
            Vec::new()
        };

        let entries = match log_file {
            LogFile::SystemMonitor => parse_system_monitor_log(&lines)?,
            LogFile::ProcessMonitor => parse_process_monitor_log(&lines)?,
            LogFile::TopCpuConsumers | LogFile::TopRamConsumers => parse_top_consumers_log(&lines)?,
        };

        return Ok((entries, total_lines));
    }

    // For initial load, use efficient reverse-read
    // Calculate cutoff based on max_lines (assuming 1 line per minute)
    let hours = (max_lines as f64 / 60.0).ceil() as i64;
    let cutoff = Utc::now() - chrono::Duration::hours(hours);

    let entries = read_log_file_since(filename, cutoff)?;
    let total_lines = entries.len();

    Ok((entries, total_lines))
}

/// Read process history for a specific process/PID combination
/// Returns only entries matching the specified process and PID
pub fn read_process_history(
    process_name: &str,
    pid: &str,
    max_lines: usize,
) -> Result<Vec<LogEntry>, String> {
    let filename = LogFile::ProcessMonitor.filename();

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
    fn test_format_display_name() {
        assert_eq!(LogFile::SystemMonitor.display_name(), "System Monitor");
        assert_eq!(LogFile::ProcessMonitor.display_name(), "Process Monitor");
        assert_eq!(LogFile::TopCpuConsumers.display_name(), "Top CPU Consumers");
    }
}

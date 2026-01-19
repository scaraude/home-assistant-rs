use crate::http::query::QueryParams;
use crate::http::responses::*;
use crate::logs::{self, LogFile};
use chrono::{Duration, Utc};
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;
use std::str::FromStr;
use tracing::{debug, error, info, warn};

pub fn serve_logs_list() -> Response<Full<Bytes>> {
    debug!("Getting list of available log files");

    let log_files = logs::list_log_files();

    let json = match serialize_to_json(&log_files, "log files list") {
        Ok(json) => json,
        Err(response) => return *response,
    };

    info!(
        log_file_count = log_files.len(),
        response_size = json.len(),
        "Successfully serialized log files list to JSON"
    );
    json_response(json)
}

pub fn serve_log_view(query: Option<&str>) -> Response<Full<Bytes>> {
    let params = QueryParams::new(query);
    let filename = params.get("file");
    let max_lines = params.get_usize("lines", 1000);
    let since_line = params.get_optional_usize("since_line");

    debug!(
        filename = ?filename,
        max_lines = max_lines,
        since_line = ?since_line,
        "Parsed query parameters for log view"
    );

    // Require filename parameter
    let Some(file) = filename else {
        warn!("Missing required 'file' parameter");
        return bad_request_response("Missing 'file' parameter");
    };

    info!(
        filename = %file,
        max_lines = max_lines,
        since_line = ?since_line,
        "Reading log file (delta support enabled)"
    );

    // Read and parse log file with delta support
    match logs::read_log_file_with_offset(file, since_line, max_lines) {
        Ok((entries, total_lines)) => {
            debug!(
                entry_count = entries.len(),
                total_lines = total_lines,
                filename = %file,
                "Retrieved log entries"
            );

            let json = match serialize_to_json(&entries, "log view entries") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                entry_count = entries.len(),
                total_lines = total_lines,
                response_size = json.len(),
                filename = %file,
                "Successfully serialized log entries to JSON"
            );

            json_response_with_total_lines(json, total_lines)
        }
        Err(e) => {
            error!(error = %e, filename = %file, "Failed to read log file");
            bad_request_response(e.to_string().as_str())
        }
    }
}

/// Serve log file entries since a specific timestamp (efficient reverse-read)
/// Query params: file (required), since (ISO timestamp or hours like "24h", "1w")
pub fn serve_log_since(query: Option<&str>) -> Response<Full<Bytes>> {
    let params = QueryParams::new(query);
    let filename = params.get("file");
    let since_param = params.get("since");

    debug!(
        filename = ?filename,
        since = ?since_param,
        "Parsed query parameters for log since"
    );

    let Some(file) = filename else {
        warn!("Missing required 'file' parameter");
        return bad_request_response("Missing 'file' parameter");
    };

    // Validate file name
    if LogFile::from_str(file).is_err() {
        warn!(file = %file, "Invalid log file name");
        return bad_request_response("Invalid log file name");
    }

    // Parse the 'since' parameter: either ISO timestamp or duration like "24h", "1w", "1m"
    let since_timestamp = match since_param {
        Some(s) => parse_since_param(s),
        None => {
            // Default to last 24 hours
            Utc::now() - Duration::hours(24)
        }
    };

    info!(
        filename = %file,
        since = %since_timestamp,
        "Reading log file since timestamp (efficient reverse-read)"
    );

    match logs::read_log_file_since(file, since_timestamp) {
        Ok(entries) => {
            debug!(
                entry_count = entries.len(),
                filename = %file,
                "Retrieved log entries"
            );

            let json = match serialize_to_json(&entries, "log entries since") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                entry_count = entries.len(),
                response_size = json.len(),
                filename = %file,
                "Successfully serialized log entries to JSON"
            );
            json_response(json)
        }
        Err(e) => {
            error!(error = %e, filename = %file, "Failed to read log file");
            bad_request_response(e.to_string().as_str())
        }
    }
}

/// Parse 'since' parameter - supports durations like "24h", "1w", "1m", "1y" or ISO timestamps
fn parse_since_param(s: &str) -> chrono::DateTime<Utc> {
    let now = Utc::now();

    // Try duration format first: 24h, 1w, 1m, 1y
    if let Some(hours) = s.strip_suffix('h')
        && let Ok(h) = hours.parse::<i64>()
    {
        return now - Duration::hours(h);
    }
    if let Some(days) = s.strip_suffix('d')
        && let Ok(d) = days.parse::<i64>()
    {
        return now - Duration::days(d);
    }
    if let Some(weeks) = s.strip_suffix('w')
        && let Ok(w) = weeks.parse::<i64>()
    {
        return now - Duration::weeks(w);
    }
    if let Some(months) = s.strip_suffix('m')
        && let Ok(m) = months.parse::<i64>()
    {
        return now - Duration::days(m * 30);
    }
    if let Some(years) = s.strip_suffix('y')
        && let Ok(y) = years.parse::<i64>()
    {
        return now - Duration::days(y * 365);
    }

    // Try parsing as Unix timestamp (seconds)
    if let Ok(ts) = s.parse::<i64>()
        && let Some(dt) = chrono::DateTime::from_timestamp(ts, 0)
    {
        return dt;
    }

    // Try parsing as ISO timestamp
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }

    // Default to 24 hours ago
    now - Duration::hours(24)
}

pub fn serve_process_history(query: Option<&str>) -> Response<Full<Bytes>> {
    let params = QueryParams::new(query);
    let process_name = params.get("process");
    let pid = params.get("pid");
    let max_lines = params.get_usize("lines", 10000);

    debug!(
        process = ?process_name,
        pid = ?pid,
        max_lines = max_lines,
        "Parsed query parameters for process history"
    );

    // Require process and pid parameters
    let (Some(process), Some(process_pid)) = (process_name, pid) else {
        warn!("Missing required 'process' or 'pid' parameter");
        return bad_request_response("Missing required 'process' and 'pid' parameters");
    };

    info!(
        process = %process,
        pid = %process_pid,
        max_lines = max_lines,
        "Reading process history"
    );

    // Read process history from log file
    match logs::read_process_history(process, process_pid, max_lines) {
        Ok(entries) => {
            debug!(
                entry_count = entries.len(),
                process = %process,
                pid = %process_pid,
                "Retrieved process history"
            );

            let json = match serialize_to_json(&entries, "process history") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                entry_count = entries.len(),
                response_size = json.len(),
                process = %process,
                pid = %process_pid,
                "Successfully serialized process history to JSON"
            );

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, process = %process, pid = %process_pid, "Failed to read process history");
            bad_request_response(e.to_string().as_str())
        }
    }
}

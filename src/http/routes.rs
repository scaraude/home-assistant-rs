use crate::db::Database;
use crate::http::query::{ExecutionLogsQuery, QueryParams};
use crate::http::responses::*;
use crate::logs::{self, LogFile};
use crate::models::{
    AutomationAction, AutomationCondition, AutomationRule, CreateAutomationRuleRequest,
    DeviceCapability, NetworkEdge, NetworkTopology, SensorType, SwitchCommand,
    SwitchCommandMessage, TimeWindowRequest, UpdateAutomationRuleRequest,
};
use crate::mqtt::MqttClient;
use crate::mqtt::topic::ZigbeeTopic;
use crate::state::{DeviceStateStore, SwitchStateStore};
use chrono::{Duration, Utc};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

// ============================================================================
// API Endpoint Handlers
// ============================================================================

pub fn health_check() -> Response<Full<Bytes>> {
    success_response()
}

pub fn serve_sensors(db: &Database) -> Response<Full<Bytes>> {
    debug!("Querying database for all sensors");

    match db.get_all_sensors() {
        Ok(sensors) => {
            debug!(
                sensor_count = sensors.len(),
                "Retrieved sensors from database"
            );

            match serde_json::to_string(&sensors) {
                Ok(json) => {
                    info!(
                        sensor_count = sensors.len(),
                        response_size = json.len(),
                        "Successfully serialized sensors to JSON"
                    );
                    json_response(json)
                }
                Err(e) => {
                    error!(error = %e, "Failed to serialize sensors to JSON");
                    internal_error_response("Failed to serialize response")
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching sensors");
            internal_error_response("Database error")
        }
    }
}

pub fn serve_readings(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    debug!(query = ?query, "Parsing query parameters");

    // Parse query parameters for device_id, hours, and since
    let params = QueryParams::new(query);
    let device_id = params.get("device_id");
    let since_param = params.get_optional_i64("since");

    // Determine timestamp: use 'since' if provided, otherwise calculate from 'hours'
    let since = since_param.unwrap_or_else(|| {
        let hours = params.get_i64("hours", 24);
        chrono::Utc::now().timestamp() - (hours * 3600)
    });

    info!(
        device_id = ?device_id,
        since_timestamp = since,
        "Querying readings (delta support enabled)"
    );

    // Use SQL-filtered queries - no more Rust-side filtering!
    let result = if let Some(sid) = device_id {
        debug!(device_id = %sid, since = since, "Querying readings for specific sensor");
        // Get readings for specific sensor with time filter in SQL
        db.get_readings_for_sensor_since(sid, since)
    } else {
        debug!(since = since, "Querying readings for all sensors");
        // Get all readings since timestamp
        db.get_readings_since(since)
    };

    match result {
        Ok(readings) => {
            debug!(
                reading_count = readings.len(),
                "Retrieved readings from database"
            );

            // Find latest timestamp for delta tracking
            let latest_timestamp = readings
                .iter()
                .map(|r| r.timestamp().timestamp())
                .max()
                .unwrap_or(since);

            match serde_json::to_string(&readings) {
                Ok(json) => {
                    info!(
                        reading_count = readings.len(),
                        response_size = json.len(),
                        device_id = ?device_id,
                        latest_ts = latest_timestamp,
                        "Successfully serialized readings to JSON"
                    );

                    json_response_with_timestamp(json, latest_timestamp)
                }
                Err(e) => {
                    error!(error = %e, "Failed to serialize readings to JSON");
                    internal_error_response("Failed to serialize response")
                }
            }
        }
        Err(e) => {
            error!(error = %e, device_id = ?device_id, since = since, "Database error while fetching readings");
            internal_error_response("Database error")
        }
    }
}

pub fn serve_logs_list() -> Response<Full<Bytes>> {
    debug!("Getting list of available log files");

    let log_files = logs::list_log_files();

    match serde_json::to_string(&log_files) {
        Ok(json) => {
            info!(
                log_file_count = log_files.len(),
                response_size = json.len(),
                "Successfully serialized log files list to JSON"
            );
            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Failed to serialize log files to JSON");
            internal_error_response("Failed to serialize response")
        }
    }
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

            match serde_json::to_string(&entries) {
                Ok(json) => {
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
                    error!(error = %e, filename = %file, "Failed to serialize log entries to JSON");
                    internal_error_response(e.to_string().as_str())
                }
            }
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

            match serde_json::to_string(&entries) {
                Ok(json) => {
                    info!(
                        entry_count = entries.len(),
                        response_size = json.len(),
                        filename = %file,
                        "Successfully serialized log entries to JSON"
                    );
                    json_response(json)
                }
                Err(e) => {
                    error!(error = %e, filename = %file, "Failed to serialize log entries to JSON");
                    internal_error_response(e.to_string().as_str())
                }
            }
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
    if let Some(hours) = s.strip_suffix('h') {
        if let Ok(h) = hours.parse::<i64>() {
            return now - Duration::hours(h);
        }
    }
    if let Some(days) = s.strip_suffix('d') {
        if let Ok(d) = days.parse::<i64>() {
            return now - Duration::days(d);
        }
    }
    if let Some(weeks) = s.strip_suffix('w') {
        if let Ok(w) = weeks.parse::<i64>() {
            return now - Duration::weeks(w);
        }
    }
    if let Some(months) = s.strip_suffix('m') {
        if let Ok(m) = months.parse::<i64>() {
            return now - Duration::days(m * 30);
        }
    }
    if let Some(years) = s.strip_suffix('y') {
        if let Ok(y) = years.parse::<i64>() {
            return now - Duration::days(y * 365);
        }
    }

    // Try parsing as Unix timestamp (seconds)
    if let Ok(ts) = s.parse::<i64>() {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
            return dt;
        }
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

            match serde_json::to_string(&entries) {
                Ok(json) => {
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
                    error!(error = %e, process = %process, pid = %process_pid, "Failed to serialize process history to JSON");
                    internal_error_response(e.to_string().as_str())
                }
            }
        }
        Err(e) => {
            error!(error = %e, process = %process, pid = %process_pid, "Failed to read process history");
            bad_request_response(e.to_string().as_str())
        }
    }
}

pub fn serve_switches_list(
    db: &Arc<Database>,
    switch_state: &Arc<SwitchStateStore>,
    _device_state: &DeviceStateStore,
) -> Response<Full<Bytes>> {
    debug!("Getting list of available switches from database");

    // Get all devices from database and filter for commanders (switches)
    let devices = match db.get_all_devices() {
        Ok(devices) => devices,
        Err(e) => {
            error!(error = %e, "Failed to query devices from database");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Filter for commander devices and build switch list
    let switches: Vec<_> = devices
        .into_iter()
        .filter(|device| {
            matches!(
                device.capability,
                crate::models::DeviceCapability::Commander { .. }
            )
        })
        .map(|device| {
            let current_state = switch_state.get_state(&device.id).unwrap_or_default();

            // Fetch device state from database instead of in-memory store
            let device_db_state = db.get_device_state(&device.id).ok().flatten();
            let link_quality = device_db_state.as_ref().and_then(|s| s.link_quality);
            let battery_level = device_db_state.as_ref().and_then(|s| s.battery_level);
            let last_seen = device_db_state.as_ref().map(|s| s.last_seen.timestamp());

            info!(
                device_id = %device.id,
                mqtt_topic = %device.mqtt_topic,
                name = %device.name,
                state = %current_state,
                link_quality = ?link_quality,
                battery_level = ?battery_level,
                "Retrieved switch state from database"
            );

            serde_json::json!({
                "id": device.id,
                "mqtt_topic": device.mqtt_topic,
                "name": device.name,
                "state": current_state.to_bool(),
                "link_quality": link_quality,
                "battery_level": battery_level,
                "last_seen": last_seen,
            })
        })
        .collect();

    match serde_json::to_string(&switches) {
        Ok(json) => {
            info!(
                switch_count = switches.len(),
                response_size = json.len(),
                "Successfully serialized switches list to JSON"
            );
            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Failed to serialize switches to JSON");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_device_state(db: &Arc<Database>, device_id: &str) -> Response<Full<Bytes>> {
    debug!(device_id = %device_id, "Getting device state from database");

    match db.get_device_state(device_id) {
        Ok(Some(state)) => {
            let json_data = serde_json::json!({
                "device_id": state.device_id,
                "battery_level": state.battery_level,
                "link_quality": state.link_quality,
                "turbo_mode": state.turbo_mode,
                "last_seen": state.last_seen.timestamp(),
            });

            match serde_json::to_string(&json_data) {
                Ok(json) => {
                    info!(
                        device_id = %device_id,
                        battery_level = ?state.battery_level,
                        link_quality = ?state.link_quality,
                        turbo_mode = ?state.turbo_mode,
                        "Successfully retrieved device state"
                    );
                    json_response(json)
                }
                Err(e) => {
                    error!(error = %e, device_id = %device_id, "Failed to serialize device state");
                    internal_error_response(e.to_string().as_str())
                }
            }
        }
        Ok(None) => {
            debug!(device_id = %device_id, "No device state found");
            not_found_response("Device state not found")
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Database error while fetching device state");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_device_states(db: &Arc<Database>) -> Response<Full<Bytes>> {
    debug!("Getting all device states from database");

    match db.get_all_device_states() {
        Ok(states) => match serde_json::to_string(&states) {
            Ok(json) => {
                info!(
                    device_state_count = states.len(),
                    response_size = json.len(),
                    "Successfully retrieved device states"
                );
                json_response(json)
            }
            Err(e) => {
                error!(error = %e, "Failed to serialize device states");
                internal_error_response(e.to_string().as_str())
            }
        },
        Err(e) => {
            error!(error = %e, "Database error while fetching device states");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub async fn execute_command(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
    mqtt: &Arc<Mutex<MqttClient>>,
) -> Response<Full<Bytes>> {
    debug!("Parsing command request body");

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Parse the command
    let command: SwitchCommand = match serde_json::from_slice::<SwitchCommand>(&body_bytes) {
        Ok(cmd) => {
            debug!(device_id = %cmd.device_id, state = %cmd.state, "Parsed command");
            cmd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse command JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Look up the MQTT topic for this device ID
    let mqtt_topic = match db.get_mqtt_topic_for_device(&command.device_id) {
        Ok(Some(topic)) => {
            debug!(
                device_id = %command.device_id,
                mqtt_topic = %topic,
                "Found MQTT topic for device"
            );
            topic
        }
        Ok(None) => {
            warn!(device_id = %command.device_id, "Device not found in database");
            return not_found_response("Device not found");
        }
        Err(e) => {
            error!(error = %e, device_id = %command.device_id, "Failed to query device from database");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Build the MQTT payload
    let payload = match serde_json::to_string(&SwitchCommandMessage {
        state: command.state,
    }) {
        Ok(json) => json,
        Err(e) => {
            error!(
                error = %e,
                device_id = %command.device_id,
                "Failed to serialize MQTT command payload"
            );
            return internal_error_response("Failed to serialize command payload");
        }
    };

    info!(
        device_id = %command.device_id,
        mqtt_topic = %mqtt_topic,
        state = %command.state,
        payload = %payload,
        "Executing switch command"
    );

    // Publish to MQTT using the actual MQTT topic
    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_command(&mqtt_topic, &payload).await {
        Ok(_) => {
            info!(
                device_id = %command.device_id,
                mqtt_topic = %mqtt_topic,
                state = %command.state,
                "Command executed successfully"
            );
            json_response(r#"{"status":"ok"}"#.into())
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %command.device_id,
                mqtt_topic = %mqtt_topic,
                "Failed to publish command to MQTT"
            );
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct PermitJoinRequest {
    value: bool,
    time: Option<u64>,
}

#[derive(serde::Serialize)]
struct PermitJoinPayload {
    value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<u64>,
}

pub async fn permit_join(
    req: Request<hyper::body::Incoming>,
    mqtt: &Arc<Mutex<MqttClient>>,
) -> Response<Full<Bytes>> {
    debug!("Parsing permit join request body");

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    let request: PermitJoinRequest = match serde_json::from_slice::<PermitJoinRequest>(&body_bytes)
    {
        Ok(req) => {
            debug!(
                value = req.value,
                time = req.time,
                "Parsed permit join request"
            );
            req
        }
        Err(e) => {
            error!(error = %e, "Failed to parse permit join JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    let payload = PermitJoinPayload {
        value: request.value,
        time: if request.value {
            Some(request.time.unwrap_or(180))
        } else {
            None
        },
    };

    let payload_json = match serde_json::to_string(&payload) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "Failed to serialize permit join payload");
            return internal_error_response("Failed to serialize payload");
        }
    };

    info!(
        value = request.value,
        time = payload.time,
        payload = %payload_json,
        "Publishing Zigbee2MQTT permit_join request"
    );

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard
        .publish_bridge_request("permit_join", &payload_json)
        .await
    {
        Ok(_) => json_response(r#"{"status":"ok"}"#.into()),
        Err(e) => {
            error!(error = %e, "Failed to publish permit_join to MQTT");
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceUpdate {
    name: String,
}

pub async fn update_device(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
    path: &str,
) -> Response<Full<Bytes>> {
    debug!("Parsing device update request");

    // Extract device ID from path (e.g., "/api/devices/0x123abc")
    let device_id = path.strip_prefix("/api/devices/").unwrap_or("");

    if device_id.is_empty() {
        warn!("Missing device ID in path");
        return bad_request_response("Missing device ID");
    }

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Parse the update payload
    let update: DeviceUpdate = match serde_json::from_slice::<DeviceUpdate>(&body_bytes) {
        Ok(upd) => {
            debug!(device_id = %device_id, name = %upd.name, "Parsed device update");
            upd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse update JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Validate name is not empty
    if update.name.trim().is_empty() {
        warn!(device_id = %device_id, "Device name cannot be empty");
        return bad_request_response("Device name cannot be empty");
    }

    info!(
        device_id = %device_id,
        new_name = %update.name,
        "Updating device name"
    );

    // Update the device name in the database
    match db.update_device_name(device_id, &update.name) {
        Ok(_) => {
            info!(
                device_id = %device_id,
                new_name = %update.name,
                "Device name updated successfully"
            );
            json_response(r#"{"status":"ok"}"#.into())
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %device_id,
                "Failed to update device name in database"
            );
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceOptionUpdate {
    turbo_mode: Option<bool>,
}

pub async fn set_device_option(
    req: Request<hyper::body::Incoming>,
    mqtt: &Arc<Mutex<MqttClient>>,
    db: &Arc<Database>,
    path: &str,
) -> Response<Full<Bytes>> {
    debug!("Parsing device option update request");

    let device_path = path.strip_prefix("/api/devices/").unwrap_or("");
    let device_id = device_path.strip_suffix("/set").unwrap_or("");

    if device_id.is_empty() {
        warn!("Missing device ID in path");
        return bad_request_response("Missing device ID");
    }

    let device = match db._get_device_by_id(device_id) {
        Ok(Some(device)) => device,
        Ok(None) => {
            warn!(device_id = %device_id, "Device not found for option update");
            return not_found_response("Device not found");
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to fetch device for option update");
            return internal_error_response("Failed to fetch device");
        }
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    let update: DeviceOptionUpdate = match serde_json::from_slice::<DeviceOptionUpdate>(&body_bytes)
    {
        Ok(upd) => upd,
        Err(e) => {
            error!(error = %e, "Failed to parse device option update JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    let turbo_mode = match update.turbo_mode {
        Some(value) => value,
        None => {
            warn!(device_id = %device_id, "Missing turbo_mode in update payload");
            return bad_request_response("Missing turbo_mode value");
        }
    };

    let payload = serde_json::json!({ "turbo_mode": turbo_mode });
    let payload_json = match serde_json::to_string(&payload) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "Failed to serialize turbo_mode payload");
            return internal_error_response("Failed to serialize payload");
        }
    };

    info!(
        device_id = %device_id,
        mqtt_topic = %device.mqtt_topic,
        turbo_mode,
        payload = %payload_json,
        "Publishing turbo_mode update"
    );

    let mqtt_target = ZigbeeTopic::parse(&device.mqtt_topic)
        .and_then(|topic| topic.device_id())
        .unwrap_or(device.mqtt_topic.as_str());

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_command(mqtt_target, &payload_json).await {
        Ok(_) => json_response(r#"{"status":"ok"}"#.into()),
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to publish turbo_mode update");
            internal_error_response(e.to_string().as_str())
        }
    }
}

// ============================================================================
// Automation Rules Endpoints
// ============================================================================

pub fn serve_automation_rules(db: &Database) -> Response<Full<Bytes>> {
    match db.get_all_automation_rules() {
        Ok(rules) => {
            let json = serde_json::to_string(&rules).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize automation rules");
                "[]".to_string()
            });

            info!(rule_count = rules.len(), "Served automation rules list");

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Failed to get automation rules");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_automation_rule(db: &Database, rule_id: &str) -> Response<Full<Bytes>> {
    match db.get_automation_rule(rule_id) {
        Ok(Some(rule)) => {
            let json = serde_json::to_string(&rule).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize automation rule");
                "{}".to_string()
            });

            info!(rule_id = %rule_id, "Served automation rule");

            json_response(json)
        }
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found");
            not_found_response("Rule not found")
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to get automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

/// Validate that automation conditions use fields compatible with their device types
fn validate_condition_fields(
    db: &Database,
    conditions: &[AutomationCondition],
) -> Result<(), String> {
    for condition in conditions {
        // Get device from database
        let device = db
            .get_device(&condition.device_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Device '{}' not found", condition.device_id))?;

        // Extract capability subtype
        let subtype = match &device.capability {
            DeviceCapability::Sensor { sensor_type } => match sensor_type {
                SensorType::TempHumidity => "temp_humidity",
                SensorType::Presence => "presence",
            },
            _ => return Err(format!("Device '{}' is not a sensor", condition.device_id)),
        };

        // Convert SensorField to string for comparison
        let field_str = match condition.field {
            crate::models::SensorField::Temperature => "temperature",
            crate::models::SensorField::Humidity => "humidity",
            crate::models::SensorField::Battery => "battery",
            crate::models::SensorField::LinkQuality => "link_quality",
            crate::models::SensorField::Presence => "presence",
            crate::models::SensorField::Illumination => "illumination",
        };

        // Validate field compatibility
        let valid = match (subtype, field_str) {
            ("temp_humidity", "temperature" | "humidity" | "battery" | "link_quality") => true,
            ("presence", "presence" | "illumination" | "battery" | "link_quality") => true,
            _ => false,
        };

        if !valid {
            let valid_fields = match subtype {
                "temp_humidity" => "temperature, humidity, battery, link_quality",
                "presence" => "presence, illumination, battery, link_quality",
                _ => "unknown",
            };

            return Err(format!(
                "Field '{}' is not valid for {} sensor. Valid fields: {}",
                field_str, subtype, valid_fields
            ));
        }
    }

    Ok(())
}

pub async fn create_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
) -> Response<Full<Bytes>> {
    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read body");
        }
    };

    // Parse the request
    let request: CreateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule request");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Validate request
    if request.name.trim().is_empty() {
        return bad_request_response("Rule name cannot be empty");
    }

    if request.conditions.is_empty() {
        return bad_request_response("At least one condition is required");
    }

    if request.actions.is_empty() {
        return bad_request_response("At least one action is required");
    }

    // Build the automation rule
    let conditions: Vec<AutomationCondition> = request
        .conditions
        .into_iter()
        .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
        .collect();

    // Validate that condition fields are compatible with device types
    if let Err(err) = validate_condition_fields(db, &conditions) {
        warn!(error = %err, "Automation rule validation failed");
        return bad_request_response(&err);
    }

    let actions: Vec<AutomationAction> = request
        .actions
        .into_iter()
        .map(|a| AutomationAction::new(a.device_id, a.action))
        .collect();

    let mut rule = AutomationRule::new(
        request.name,
        request.description,
        request.condition_operator,
        conditions,
        actions,
    );

    if let Some(enabled) = request.enabled {
        rule.enabled = enabled;
    }

    if let Some(time_window) = request.time_window {
        apply_time_window_update(&mut rule, time_window);
    }

    // Insert into database
    match db.insert_automation_rule(&rule) {
        Ok(_) => {
            info!(rule_id = %rule.id, rule_name = %rule.name, "Created automation rule");

            let json = serde_json::to_string(&rule).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize created rule");
                "{}".to_string()
            });
            json_response_with_status(json, StatusCode::CREATED)
        }
        Err(e) => {
            error!(error = %e, "Failed to insert automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub async fn update_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
    rule_id: &str,
) -> Response<Full<Bytes>> {
    // Get existing rule
    let mut rule = match db.get_automation_rule(rule_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found for update");
            return not_found_response("Rule not found");
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to fetch rule for update");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read body");
        }
    };

    // Parse the update request
    let update: UpdateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule update");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Apply updates
    if let Some(name) = update.name {
        if name.trim().is_empty() {
            return bad_request_response("Rule name cannot be empty");
        }
        rule.name = name;
    }

    if let Some(description) = update.description {
        rule.description = Some(description);
    }

    if let Some(enabled) = update.enabled {
        rule.enabled = enabled;
    }

    if let Some(condition_operator) = update.condition_operator {
        rule.condition_operator = condition_operator;
    }

    if let Some(conditions) = update.conditions {
        if conditions.is_empty() {
            return bad_request_response("At least one condition is required");
        }
        let new_conditions: Vec<AutomationCondition> = conditions
            .into_iter()
            .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
            .collect();

        // Validate that condition fields are compatible with device types
        if let Err(err) = validate_condition_fields(db, &new_conditions) {
            warn!(error = %err, "Automation rule update validation failed");
            return bad_request_response(&err);
        }

        rule.conditions = new_conditions;
    }

    if let Some(actions) = update.actions {
        if actions.is_empty() {
            return bad_request_response("At least one action is required");
        }
        rule.actions = actions
            .into_iter()
            .map(|a| AutomationAction::new(a.device_id, a.action))
            .collect();
    }

    if let Some(time_window) = update.time_window {
        apply_time_window_update(&mut rule, time_window);
    }

    // Update timestamp
    rule.updated_at = chrono::Utc::now();

    // Save to database
    match db.update_automation_rule(&rule) {
        Ok(_) => {
            info!(rule_id = %rule.id, rule_name = %rule.name, "Updated automation rule");

            let json = serde_json::to_string(&rule).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize updated rule");
                "{}".to_string()
            });

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to update automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

fn apply_time_window_update(rule: &mut AutomationRule, update: TimeWindowRequest) {
    if let Some(enabled) = update.enabled {
        rule.time_window.enabled = enabled;
    }

    if let Some(start_time) = update.start_time {
        rule.time_window.start_time = Some(start_time);
    }

    if let Some(end_time) = update.end_time {
        rule.time_window.end_time = Some(end_time);
    }

    if let Some(active_days) = update.active_days {
        rule.time_window.active_days = Some(active_days);
    }
}

pub fn delete_automation_rule(db: &Database, rule_id: &str) -> Response<Full<Bytes>> {
    match db.delete_automation_rule(rule_id) {
        Ok(_) => {
            info!(rule_id = %rule_id, "Deleted automation rule");
            no_content_response()
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to delete automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_execution_logs(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    let limit = ExecutionLogsQuery::new(query).limit();

    match db.get_execution_logs(limit) {
        Ok(logs) => {
            let json = serde_json::to_string(&logs).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize execution logs");
                "[]".to_string()
            });

            info!(log_count = logs.len(), limit = %limit, "Served automation execution logs");

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Failed to get execution logs");
            internal_error_response(e.to_string().as_str())
        }
    }
}

// ============================================================================
// Network Topology Handlers
// ============================================================================

pub fn serve_network_topology(
    db: &Database,
    device_state: &DeviceStateStore,
) -> Response<Full<Bytes>> {
    debug!("Fetching network topology");

    match db.get_all_devices() {
        Ok(devices) => {
            // Compute edges from parent_device_id relationships
            let mut edges = Vec::new();
            let mut device_state_map = HashMap::new();

            // Build a map of device states for quick link quality lookup
            for device in &devices {
                if let Some(state) = device_state.get_state(&device.id) {
                    device_state_map.insert(device.id.clone(), state);
                }
            }

            // Create edges from parent-child relationships
            for device in &devices {
                if let Some(parent_id) = &device.parent_device_id {
                    let link_quality = device_state_map
                        .get(&device.id)
                        .and_then(|state| state.link_quality);

                    edges.push(NetworkEdge {
                        source_id: parent_id.clone(),
                        target_id: device.id.clone(),
                        link_quality,
                    });
                }
            }

            let topology = NetworkTopology { devices, edges };

            match serde_json::to_string(&topology) {
                Ok(json) => {
                    info!(
                        device_count = topology.devices.len(),
                        edge_count = topology.edges.len(),
                        "Served network topology"
                    );
                    json_response(json)
                }
                Err(e) => {
                    error!(error = %e, "Failed to serialize network topology");
                    internal_error_response("Failed to serialize topology")
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch devices for topology");
            internal_error_response("Database error")
        }
    }
}

pub async fn refresh_network_map(mqtt: &Arc<Mutex<MqttClient>>) -> Response<Full<Bytes>> {
    debug!("Refreshing network map");

    // Publish networkmap request to zigbee2mqtt
    let topic = "zigbee2mqtt/bridge/request/networkmap";
    let payload = r#"{"type":"raw","routes":true}"#;

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_raw(topic, payload).await {
        Ok(_) => {
            info!("Successfully published networkmap request");
            json_response(r#"{"status":"ok","message":"Network map refresh requested"}"#.into())
        }
        Err(e) => {
            error!(error = %e, "Failed to publish networkmap request");
            internal_error_response("Failed to request network map")
        }
    }
}

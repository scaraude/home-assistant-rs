use crate::cache::ResponseCache;
use crate::db::Database;
use crate::device_state::DeviceStateStore;
use crate::logs;
use crate::models::{
    AutomationAction, AutomationCondition, AutomationRule, CreateAutomationRuleRequest,
    SwitchCommand, UpdateAutomationRuleRequest,
};
use crate::mqtt::MqttClient;
use crate::switch_state::SwitchStateStore;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct HttpServer {
    db: Arc<Database>,
    cache: Arc<ResponseCache>,
    mqtt: Arc<Mutex<MqttClient>>,
    switch_state: Arc<SwitchStateStore>,
    device_state: DeviceStateStore,
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(
        db: Arc<Database>,
        cache: Arc<ResponseCache>,
        mqtt: Arc<Mutex<MqttClient>>,
        switch_state: Arc<SwitchStateStore>,
        device_state: DeviceStateStore,
        addr: SocketAddr,
    ) -> Self {
        Self {
            db,
            cache,
            mqtt,
            switch_state,
            device_state,
            addr,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        info!(addr = %self.addr, "Binding HTTP server to address");

        let listener = match TcpListener::bind(self.addr).await {
            Ok(l) => {
                info!(addr = %self.addr, "HTTP server listening on address");
                l
            }
            Err(e) => {
                error!(error = %e, addr = %self.addr, "Failed to bind HTTP server");
                return Err(Box::new(e));
            }
        };

        let mut connection_count = 0u64;

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    connection_count += 1;
                    debug!(
                        client_addr = %client_addr,
                        connection_count = connection_count,
                        "Accepted new HTTP connection"
                    );

                    let io = TokioIo::new(stream);
                    let db = self.db.clone();
                    let cache = self.cache.clone();
                    let mqtt = self.mqtt.clone();
                    let switch_state = self.switch_state.clone();
                    let device_state = self.device_state.clone();
                    let conn_id = connection_count;

                    tokio::spawn(async move {
                        debug!(connection_id = conn_id, client_addr = %client_addr, "Starting HTTP connection handler");

                        // Clone once per connection, reused across all requests via the service closure
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(
                                io,
                                service_fn(|req| {
                                    handle_request(
                                        req,
                                        db.clone(),
                                        cache.clone(),
                                        mqtt.clone(),
                                        switch_state.clone(),
                                        device_state.clone(),
                                    )
                                }),
                            )
                            .await
                        {
                            error!(
                                error = ?err,
                                connection_id = conn_id,
                                client_addr = %client_addr,
                                "Error serving HTTP connection"
                            );
                        } else {
                            debug!(connection_id = conn_id, client_addr = %client_addr, "HTTP connection closed gracefully");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Failed to accept incoming HTTP connection");
                }
            }
        }
    }
}

// ============================================================================
// Query Parameter Parser
// ============================================================================
//
// Simple zero-dependency parser to eliminate duplicate query string parsing
// logic across endpoints. Keeps binary size minimal while reducing duplication.

struct QueryParams<'a> {
    query: Option<&'a str>,
}

impl<'a> QueryParams<'a> {
    fn new(query: Option<&'a str>) -> Self {
        Self { query }
    }

    fn get(&self, key: &str) -> Option<&'a str> {
        self.query.and_then(|q| {
            for param in q.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    if k == key {
                        return Some(v);
                    }
                }
            }
            None
        })
    }

    fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    fn get_usize(&self, key: &str, default: usize) -> usize {
        self.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    fn get_optional_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    fn get_optional_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(|v| v.parse().ok())
    }
}

// ============================================================================
// HTTP Response Helper Functions
// ============================================================================
//
// These helpers eliminate the need for `.unwrap()` calls when building
// HTTP responses. They use `.expect()` with clear error messages for
// cases that should never fail (e.g., building a response with valid headers).
//
// This approach is safer than `.unwrap()` because:
// 1. It documents invariants with descriptive messages
// 2. Makes failures observable in production logs
// 3. Centralizes response building logic

/// Create a successful JSON response (200 OK)
fn json_response(json: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response - this should never happen with valid headers")
}

// /// Create a JSON response with custom status code
// fn json_response_with_status(json: String, status: StatusCode) -> Response<Full<Bytes>> {
//     Response::builder()
//         .status(status)
//         .header("Content-Type", "application/json")
//         .body(Full::new(Bytes::from(json)))
//         .expect("Failed to build JSON response - this should never happen with valid headers")
// }

/// Create a JSON response with ETag header
fn json_response_with_etag(json: String, etag: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build cached JSON response - this should never happen")
}

/// Create a JSON response with ETag and custom timestamp header
fn json_response_with_cache_headers(
    json: String,
    etag: String,
    latest_timestamp: i64,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .header("X-Latest-Timestamp", latest_timestamp.to_string())
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build cached JSON response with timestamp header")
}

// /// Create a JSON response with ETag and total lines header (for log pagination)
// fn json_response_with_lines_header(
//     json: String,
//     etag: String,
//     total_lines: usize,
// ) -> Response<Full<Bytes>> {
//     Response::builder()
//         .status(StatusCode::OK)
//         .header("Content-Type", "application/json")
//         .header("ETag", etag)
//         .header("Cache-Control", "private, must-revalidate")
//         .header("X-Total-Lines", total_lines.to_string())
//         .body(Full::new(Bytes::from(json)))
//         .expect("Failed to build cached JSON response with total lines header")
// }

/// Create a 304 Not Modified response with ETag
fn not_modified_response(etag: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header("ETag", etag)
        .body(Full::new(Bytes::new()))
        .expect("Failed to build 304 Not Modified response")
}

/// Create an error response with JSON error message
fn error_response(message: &str, status: StatusCode) -> Response<Full<Bytes>> {
    let json = format!(r#"{{"error":"{}"}}"#, message);
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build error response")
}

/// Create a simple success response
fn success_response() -> Response<Full<Bytes>> {
    json_response(r#"{"status":"ok"}"#.to_string())
}

// /// Create an empty response with custom status code
// fn empty_response(status: StatusCode) -> Response<Full<Bytes>> {
//     Response::builder()
//         .status(status)
//         .body(Full::new(Bytes::new()))
//         .expect("Failed to build empty response")
// }

/// Create a 404 Not Found response
fn not_found_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::NOT_FOUND)
}

/// Create a 500 Internal Server Error response
fn internal_error_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::INTERNAL_SERVER_ERROR)
}

/// Create a 400 Bad Request response
fn bad_request_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::BAD_REQUEST)
}

// ============================================================================
// End of Helper Functions
// ============================================================================

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    db: Arc<Database>,
    cache: Arc<ResponseCache>,
    mqtt: Arc<Mutex<MqttClient>>,
    switch_state: Arc<SwitchStateStore>,
    device_state: DeviceStateStore,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let query = req.uri().query().map(|s| s.to_string());

    let start = std::time::Instant::now();

    info!(
        method = %method,
        path = %path,
        query = ?query,
        "Incoming HTTP request"
    );

    // Check if client sent If-None-Match header (conditional request)
    let client_etag = req
        .headers()
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"').to_string());

    // Try to get cached response for GET requests on cacheable endpoints
    if method == hyper::Method::GET && is_cacheable_path(&path, query.as_deref()) {
        if let Some(cached) = cache.get(&path, query.as_deref()) {
            // Check if client's ETag matches cached ETag
            if let Some(ref client_etag_value) = client_etag {
                if client_etag_value == &cached.etag {
                    debug!(
                        path = %path,
                        client_etag = %client_etag_value,
                        cached_etag = %cached.etag,
                        "Cache hit - returning 304 Not Modified"
                    );
                    return Ok(not_modified_response(cached.etag.clone()));
                }
            }

            // Cache hit, return cached response with ETag
            debug!(path = %path, etag = %cached.etag, "Cache hit - returning cached response");
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header("ETag", cached.etag)
                .header("Cache-Control", "private, must-revalidate")
                .body(Full::new(cached.body))
                .expect("Failed to build cached response"));
        }
    }

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/") => {
            debug!("Serving index page");
            serve_static_file("static/index.html", "text/html; charset=utf-8")
        }
        ("GET", "/health") => {
            debug!("Health check request");
            health_check()
        }
        ("GET", "/api/sensors") => {
            debug!("Serving sensors list");
            serve_sensors(&db, &cache, &path, query.as_deref())
        }
        ("GET", "/api/readings") => {
            debug!(query = ?query, "Serving readings");
            serve_readings(&db, &cache, &path, query.as_deref())
        }
        ("GET", "/api/devices/switches") => {
            debug!("Serving switches list");
            serve_switches_list(&db, &switch_state, &device_state)
        }
        ("POST", "/api/commands/execute") => {
            debug!("Executing command");
            execute_command(req, &db, &mqtt).await
        }
        ("PATCH", path) if path.starts_with("/api/devices/") => {
            debug!(path = %path, "Updating device");
            update_device(req, &db, path).await
        }
        ("GET", "/api/automation/rules") => {
            debug!("Serving automation rules list");
            serve_automation_rules(&db)
        }
        ("GET", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Serving automation rule by ID");
            let rule_id = &path["/api/automation/rules/".len()..];
            serve_automation_rule(&db, rule_id)
        }
        ("POST", "/api/automation/rules") => {
            debug!("Creating automation rule");
            create_automation_rule(req, &db).await
        }
        ("PUT", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Updating automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            update_automation_rule(req, &db, rule_id).await
        }
        ("DELETE", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Deleting automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            delete_automation_rule(&db, rule_id)
        }
        ("GET", "/api/automation/logs") => {
            debug!("Serving automation execution logs");
            serve_execution_logs(&db, query.as_deref())
        }
        ("GET", "/api/logs/list") => {
            debug!("Serving logs list");
            serve_logs_list()
        }
        ("GET", "/api/logs/view") => {
            debug!(query = ?query, "Serving log file view");
            serve_log_view(&cache, &path, query.as_deref())
        }
        ("GET", "/api/logs/process") => {
            debug!(query = ?query, "Serving process history");
            serve_process_history(&cache, &path, query.as_deref())
        }
        (_, path_str) if path_str.starts_with("/assets/") || path_str.ends_with(".svg") => {
            debug!(path = %path_str, "Serving static asset");
            serve_static_asset(path_str)
        }
        _ => {
            warn!(path = %path, method = %method, "Request to unknown path");
            not_found()
        }
    };

    let elapsed = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = elapsed.as_millis(),
        "HTTP request completed"
    );

    Ok(response)
}

/// Determine if a path should be cached
/// Delta requests (with since/since_line params) are not cached to avoid pollution
fn is_cacheable_path(path: &str, query: Option<&str>) -> bool {
    // Check if path is a cacheable endpoint
    if !matches!(path, "/api/readings" | "/api/logs/view" | "/api/sensors") {
        return false;
    }

    // Don't cache delta requests - they change constantly and pollute the cache
    if let Some(q) = query {
        if q.contains("since=") || q.contains("since_line=") {
            return false;
        }
    }

    true
}

/// Create a cached JSON response
fn json_response_with_cache(
    json: String,
    cache: &ResponseCache,
    path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
    let bytes = Bytes::from(json.clone());
    let etag = cache.put(path, query, bytes.clone());

    debug!(path = %path, etag = %etag, "Created cached response");

    json_response_with_etag(json, etag)
}

fn health_check() -> Response<Full<Bytes>> {
    success_response()
}

fn serve_static_file(file_path: &str, content_type: &str) -> Response<Full<Bytes>> {
    match std::fs::read(file_path) {
        Ok(contents) => {
            debug!(file_path = %file_path, size = contents.len(), "Serving static file");
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", content_type)
                .header("Cache-Control", "public, max-age=3600")
                .body(Full::new(Bytes::from(contents)))
                .expect("Failed to build static file response")
        }
        Err(e) => {
            error!(error = %e, file_path = %file_path, "Failed to read static file");
            not_found_response("File not found")
        }
    }
}

fn serve_static_asset(path: &str) -> Response<Full<Bytes>> {
    // Remove leading slash and construct file path
    let file_path = format!("static{}", path);

    // Determine content type based on extension
    let content_type = if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };

    serve_static_file(&file_path, content_type)
}

fn serve_sensors(
    db: &Database,
    cache: &ResponseCache,
    path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
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
                    json_response_with_cache(json, cache, path, query)
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

fn serve_readings(
    db: &Database,
    cache: &ResponseCache,
    path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
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
                .map(|r| r.timestamp.timestamp())
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

                    // Create cached response with custom header for delta tracking
                    let bytes = Bytes::from(json.clone());
                    let etag = cache.put(path, query, bytes.clone());

                    json_response_with_cache_headers(json, etag, latest_timestamp)
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

fn serve_logs_list() -> Response<Full<Bytes>> {
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

fn serve_log_view(cache: &ResponseCache, path: &str, query: Option<&str>) -> Response<Full<Bytes>> {
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

                    // Create cached response with custom header for delta tracking
                    let bytes = Bytes::from(json);
                    let etag = cache.put(path, query, bytes.clone());

                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .header("ETag", etag)
                        .header("Cache-Control", "private, must-revalidate")
                        .header("X-Total-Lines", total_lines.to_string())
                        .body(Full::new(bytes))
                        .unwrap()
                }
                Err(e) => {
                    error!(error = %e, filename = %file, "Failed to serialize log entries to JSON");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("[]")))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            error!(error = %e, filename = %file, "Failed to read log file");
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

fn serve_process_history(
    cache: &ResponseCache,
    path: &str,
    query: Option<&str>,
) -> Response<Full<Bytes>> {
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

                    // Create cached response
                    let bytes = Bytes::from(json);
                    let etag = cache.put(path, query, bytes.clone());

                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .header("ETag", etag)
                        .header("Cache-Control", "private, must-revalidate")
                        .body(Full::new(bytes))
                        .unwrap()
                }
                Err(e) => {
                    error!(error = %e, process = %process, pid = %process_pid, "Failed to serialize process history to JSON");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("[]")))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            error!(error = %e, process = %process, pid = %process_pid, "Failed to read process history");
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

fn serve_switches_list(
    db: &Arc<Database>,
    switch_state: &Arc<SwitchStateStore>,
    device_state: &DeviceStateStore,
) -> Response<Full<Bytes>> {
    debug!("Getting list of available switches from database");

    // Get all devices from database and filter for commanders (switches)
    let devices = match db.get_all_devices() {
        Ok(devices) => devices,
        Err(e) => {
            error!(error = %e, "Failed to query devices from database");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("[]")))
                .unwrap();
        }
    };

    // Filter for commander devices and build switch list
    let switches: Vec<_> = devices
        .into_iter()
        .filter(|device| device.device_type == crate::models::DeviceType::Commander)
        .map(|device| {
            let current_state = switch_state.get_state(&device.id).unwrap_or(false);
            let link_quality = device_state
                .get_state(&device.id)
                .and_then(|state| state.link_quality);

            info!(
                device_id = %device.id,
                mqtt_topic = %device.mqtt_topic,
                name = %device.name,
                state = %current_state,
                link_quality = ?link_quality,
                "Retrieved switch state from store"
            );

            serde_json::json!({
                "id": device.id,
                "mqtt_topic": device.mqtt_topic,
                "name": device.name,
                "state": current_state,
                "link_quality": link_quality,
                "last_updated": chrono::Utc::now().timestamp()
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
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, "Failed to serialize switches to JSON");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("[]")))
                .unwrap()
        }
    }
}

async fn execute_command(
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Failed to read request body"}"#,
                )))
                .unwrap();
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(r#"{"error":"Invalid JSON format"}"#)))
                .unwrap();
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
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(r#"{"error":"Device not found"}"#)))
                .unwrap();
        }
        Err(e) => {
            error!(error = %e, device_id = %command.device_id, "Failed to query device from database");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(r#"{"error":"Database error"}"#)))
                .unwrap();
        }
    };

    // Build the MQTT payload
    let state_str = if command.state { "ON" } else { "OFF" };
    let payload = format!(r#"{{"state":"{}"}}"#, state_str);

    info!(
        device_id = %command.device_id,
        mqtt_topic = %mqtt_topic,
        state = %state_str,
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
                state = %state_str,
                "Command executed successfully"
            );
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
                .unwrap()
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %command.device_id,
                mqtt_topic = %mqtt_topic,
                "Failed to publish command to MQTT"
            );
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Failed to send command"}"#,
                )))
                .unwrap()
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceUpdate {
    name: String,
}

async fn update_device(
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Failed to read request body"}"#,
                )))
                .unwrap();
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(r#"{"error":"Invalid JSON format"}"#)))
                .unwrap();
        }
    };

    // Validate name is not empty
    if update.name.trim().is_empty() {
        warn!(device_id = %device_id, "Device name cannot be empty");
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(
                r#"{"error":"Device name cannot be empty"}"#,
            )))
            .unwrap();
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
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
                .unwrap()
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %device_id,
                "Failed to update device name in database"
            );
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Failed to update device name"}"#,
                )))
                .unwrap()
        }
    }
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}
fn serve_automation_rules(db: &Database) -> Response<Full<Bytes>> {
    match db.get_all_automation_rules() {
        Ok(rules) => {
            let json = serde_json::to_string(&rules).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize automation rules");
                "[]".to_string()
            });

            info!(rule_count = rules.len(), "Served automation rules list");

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, "Failed to get automation rules");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

/// GET /api/automation/rules/:id - Get a single automation rule
fn serve_automation_rule(db: &Database, rule_id: &str) -> Response<Full<Bytes>> {
    match db.get_automation_rule(rule_id) {
        Ok(Some(rule)) => {
            let json = serde_json::to_string(&rule).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize automation rule");
                "{}".to_string()
            });

            info!(rule_id = %rule_id, "Served automation rule");

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found");
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(r#"{"error":"Rule not found"}"#)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to get automation rule");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

/// POST /api/automation/rules - Create a new automation rule
async fn create_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
) -> Response<Full<Bytes>> {
    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(r#"{"error":"Failed to read body"}"#)))
                .unwrap();
        }
    };

    // Parse the request
    let request: CreateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule request");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"Invalid JSON: {}"}}"#,
                    e
                ))))
                .unwrap();
        }
    };

    // Validate request
    if request.name.trim().is_empty() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(
                r#"{"error":"Rule name cannot be empty"}"#,
            )))
            .unwrap();
    }

    if request.conditions.is_empty() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(
                r#"{"error":"At least one condition is required"}"#,
            )))
            .unwrap();
    }

    if request.actions.is_empty() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(
                r#"{"error":"At least one action is required"}"#,
            )))
            .unwrap();
    }

    // Build the automation rule
    let conditions: Vec<AutomationCondition> = request
        .conditions
        .into_iter()
        .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
        .collect();

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

    // Insert into database
    match db.insert_automation_rule(&rule) {
        Ok(_) => {
            info!(rule_id = %rule.id, rule_name = %rule.name, "Created automation rule");

            let json = serde_json::to_string(&rule).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize created rule");
                "{}".to_string()
            });

            Response::builder()
                .status(StatusCode::CREATED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, "Failed to insert automation rule");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

/// PUT /api/automation/rules/:id - Update an automation rule
async fn update_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
    rule_id: &str,
) -> Response<Full<Bytes>> {
    // Get existing rule
    let mut rule = match db.get_automation_rule(rule_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found for update");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(r#"{"error":"Rule not found"}"#)))
                .unwrap();
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to fetch rule for update");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap();
        }
    };

    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(r#"{"error":"Failed to read body"}"#)))
                .unwrap();
        }
    };

    // Parse the update request
    let update: UpdateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule update");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"Invalid JSON: {}"}}"#,
                    e
                ))))
                .unwrap();
        }
    };

    // Apply updates
    if let Some(name) = update.name {
        if name.trim().is_empty() {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(
                    r#"{"error":"Rule name cannot be empty"}"#,
                )))
                .unwrap();
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
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(
                    r#"{"error":"At least one condition is required"}"#,
                )))
                .unwrap();
        }
        rule.conditions = conditions
            .into_iter()
            .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
            .collect();
    }

    if let Some(actions) = update.actions {
        if actions.is_empty() {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(
                    r#"{"error":"At least one action is required"}"#,
                )))
                .unwrap();
        }
        rule.actions = actions
            .into_iter()
            .map(|a| AutomationAction::new(a.device_id, a.action))
            .collect();
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

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to update automation rule");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

/// DELETE /api/automation/rules/:id - Delete an automation rule
fn delete_automation_rule(db: &Database, rule_id: &str) -> Response<Full<Bytes>> {
    match db.delete_automation_rule(rule_id) {
        Ok(_) => {
            info!(rule_id = %rule_id, "Deleted automation rule");
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to delete automation rule");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

/// GET /api/automation/logs - Get automation execution logs
fn serve_execution_logs(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    // Parse limit from query string (default: 100)
    let limit = query
        .and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("limit="))
                .and_then(|p| p.strip_prefix("limit="))
                .and_then(|v| v.parse::<i64>().ok())
        })
        .unwrap_or(100);

    match db.get_execution_logs(limit) {
        Ok(logs) => {
            let json = serde_json::to_string(&logs).unwrap_or_else(|e| {
                error!(error = %e, "Failed to serialize execution logs");
                "[]".to_string()
            });

            info!(log_count = logs.len(), limit = %limit, "Served automation execution logs");

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, "Failed to get execution logs");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(r#"{{"error":"{}"}}"#, e))))
                .unwrap()
        }
    }
}

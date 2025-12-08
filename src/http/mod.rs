use crate::cache::ResponseCache;
use crate::db::Database;
use crate::logs;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

pub struct HttpServer {
    db: Arc<Database>,
    cache: Arc<ResponseCache>,
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(db: Arc<Database>, cache: Arc<ResponseCache>, addr: SocketAddr) -> Self {
        Self { db, cache, addr }
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
                    let conn_id = connection_count;

                    tokio::spawn(async move {
                        debug!(connection_id = conn_id, client_addr = %client_addr, "Starting HTTP connection handler");

                        // Clone once per connection, reused across all requests via the service closure
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(
                                io,
                                service_fn(|req| handle_request(req, db.clone(), cache.clone())),
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

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    db: Arc<Database>,
    cache: Arc<ResponseCache>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path();
    let method = req.method();
    let query = req.uri().query();

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
    if method == hyper::Method::GET && is_cacheable_path(path, query) {
        if let Some(cached) = cache.get(path, query) {
            // Check if client's ETag matches cached ETag
            if let Some(ref client_etag_value) = client_etag {
                if client_etag_value == &cached.etag {
                    debug!(
                        path = %path,
                        client_etag = %client_etag_value,
                        cached_etag = %cached.etag,
                        "Cache hit - returning 304 Not Modified"
                    );
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("ETag", &cached.etag)
                        .body(Full::new(Bytes::new()))
                        .unwrap());
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
                .unwrap());
        }
    }

    let response = match path {
        "/" => {
            debug!("Serving index page");
            serve_static_file("static/index.html", "text/html; charset=utf-8")
        }
        "/health" => {
            debug!("Health check request");
            health_check()
        }
        "/api/sensors" => {
            debug!("Serving sensors list");
            serve_sensors(&db, &cache, path, query)
        }
        "/api/readings" => {
            debug!(query = ?query, "Serving readings");
            serve_readings(&db, &cache, path, query)
        }
        "/api/logs/list" => {
            debug!("Serving logs list");
            serve_logs_list()
        }
        "/api/logs/view" => {
            debug!(query = ?query, "Serving log file view");
            serve_log_view(&cache, path, query)
        }
        _ if path.starts_with("/assets/") || path.ends_with(".svg") => {
            debug!(path = %path, "Serving static asset");
            serve_static_asset(path)
        }
        _ => {
            warn!(path = %path, "Request to unknown path");
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
    let bytes = Bytes::from(json);
    let etag = cache.put(path, query, bytes.clone());

    debug!(path = %path, etag = %etag, "Created cached response");

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .body(Full::new(bytes))
        .unwrap()
}

fn health_check() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
        .unwrap()
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
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, file_path = %file_path, "Failed to read static file");
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("File not found")))
                .unwrap()
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
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("[]")))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching sensors");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Database error")))
                .unwrap()
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

    // Parse query parameters for sensor_id, hours, and since
    let mut sensor_id: Option<&str> = None;
    let mut hours: Option<i64> = None;
    let mut since_param: Option<i64> = None;

    if let Some(q) = query {
        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "sensor_id" => {
                        sensor_id = Some(value);
                        debug!(sensor_id = %value, "Parsed sensor_id parameter");
                    }
                    "hours" => {
                        hours = Some(value.parse().unwrap_or_else(|e| {
                            warn!(
                                value = %value,
                                error = %e,
                                "Failed to parse hours parameter, using default (24)"
                            );
                            24
                        }));
                        debug!(hours = ?hours, "Parsed hours parameter");
                    }
                    "since" => {
                        since_param = value.parse().ok();
                        if since_param.is_none() {
                            warn!(value = %value, "Failed to parse since parameter (must be Unix timestamp)");
                        } else {
                            debug!(since = ?since_param, "Parsed since parameter");
                        }
                    }
                    _ => {
                        debug!(key = %key, value = %value, "Ignoring unknown query parameter");
                    }
                }
            }
        }
    }

    // Determine timestamp: use 'since' if provided, otherwise calculate from 'hours'
    let since = if let Some(ts) = since_param {
        ts
    } else {
        let h = hours.unwrap_or(24);
        chrono::Utc::now().timestamp() - (h * 3600)
    };

    info!(
        sensor_id = ?sensor_id,
        since_timestamp = since,
        "Querying readings (delta support enabled)"
    );

    // Use SQL-filtered queries - no more Rust-side filtering!
    let result = if let Some(sid) = sensor_id {
        debug!(sensor_id = %sid, since = since, "Querying readings for specific sensor");
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
                        sensor_id = ?sensor_id,
                        latest_ts = latest_timestamp,
                        "Successfully serialized readings to JSON"
                    );

                    // Create cached response with custom header for delta tracking
                    let bytes = Bytes::from(json);
                    let etag = cache.put(path, query, bytes.clone());

                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .header("ETag", etag)
                        .header("Cache-Control", "private, must-revalidate")
                        .header("X-Latest-Timestamp", latest_timestamp.to_string())
                        .body(Full::new(bytes))
                        .unwrap()
                }
                Err(e) => {
                    error!(error = %e, "Failed to serialize readings to JSON");
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from("[]")))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            error!(error = %e, sensor_id = ?sensor_id, hours = hours, "Database error while fetching readings");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Database error")))
                .unwrap()
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
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            error!(error = %e, "Failed to serialize log files to JSON");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("[]")))
                .unwrap()
        }
    }
}

fn serve_log_view(cache: &ResponseCache, path: &str, query: Option<&str>) -> Response<Full<Bytes>> {
    debug!(query = ?query, "Parsing query parameters for log view");

    // Parse query parameters for file, lines, and since_line
    let mut filename: Option<&str> = None;
    let mut max_lines: usize = 1000; // Default to last 1000 lines
    let mut since_line: Option<usize> = None;

    if let Some(q) = query {
        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "file" => {
                        filename = Some(value);
                        debug!(filename = %value, "Parsed file parameter");
                    }
                    "lines" => {
                        max_lines = value.parse().unwrap_or_else(|e| {
                            warn!(
                                value = %value,
                                error = %e,
                                "Failed to parse lines parameter, using default (1000)"
                            );
                            1000
                        });
                        debug!(max_lines = max_lines, "Parsed lines parameter");
                    }
                    "since_line" => {
                        since_line = value.parse().ok();
                        if since_line.is_none() {
                            warn!(value = %value, "Failed to parse since_line parameter (must be integer)");
                        } else {
                            debug!(since_line = ?since_line, "Parsed since_line parameter (delta mode)");
                        }
                    }
                    _ => {
                        debug!(key = %key, value = %value, "Ignoring unknown query parameter");
                    }
                }
            }
        }
    }

    // Require filename parameter
    let Some(file) = filename else {
        warn!("Missing required 'file' parameter");
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::from(
                r#"{"error":"Missing 'file' parameter"}"#,
            )))
            .unwrap();
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

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

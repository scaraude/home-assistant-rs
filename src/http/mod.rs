mod query;
mod responses;
mod routes;
mod static_files;

use crate::cache::ResponseCache;
use crate::db::Database;
use crate::mqtt::MqttClient;
use crate::state::{DeviceStateStore, SwitchStateStore};
use http_body_util::Full;
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
                    return Ok(responses::not_modified_response(cached.etag.clone()));
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
            static_files::serve_static_file("static/index.html", "text/html; charset=utf-8")
        }
        ("GET", "/health") => {
            debug!("Health check request");
            routes::health_check()
        }
        ("GET", "/api/sensors") => {
            debug!("Serving sensors list");
            routes::serve_sensors(&db, &cache, &path, query.as_deref())
        }
        ("GET", "/api/readings") => {
            debug!(query = ?query, "Serving readings");
            routes::serve_readings(&db, &cache, &path, query.as_deref())
        }
        ("GET", "/api/devices/switches") => {
            debug!("Serving switches list");
            routes::serve_switches_list(&db, &switch_state, &device_state)
        }
        ("GET", path) if path.starts_with("/api/devices/") && path.ends_with("/state") => {
            debug!(path = %path, "Serving device state");
            let device_id = &path["/api/devices/".len()..path.len() - "/state".len()];
            routes::serve_device_state(&db, device_id)
        }
        ("POST", "/api/commands/execute") => {
            debug!("Executing command");
            routes::execute_command(req, &db, &mqtt).await
        }
        ("PATCH", path) if path.starts_with("/api/devices/") => {
            debug!(path = %path, "Updating device");
            routes::update_device(req, &db, path).await
        }
        ("GET", "/api/automation/rules") => {
            debug!("Serving automation rules list");
            routes::serve_automation_rules(&db)
        }
        ("GET", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Serving automation rule by ID");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::serve_automation_rule(&db, rule_id)
        }
        ("POST", "/api/automation/rules") => {
            debug!("Creating automation rule");
            routes::create_automation_rule(req, &db).await
        }
        ("PUT", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Updating automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::update_automation_rule(req, &db, rule_id).await
        }
        ("DELETE", path) if path.starts_with("/api/automation/rules/") => {
            debug!(path = %path, "Deleting automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::delete_automation_rule(&db, rule_id)
        }
        ("GET", "/api/automation/logs") => {
            debug!("Serving automation execution logs");
            routes::serve_execution_logs(&db, query.as_deref())
        }
        ("GET", "/api/logs/list") => {
            debug!("Serving logs list");
            routes::serve_logs_list()
        }
        ("GET", "/api/logs/view") => {
            debug!(query = ?query, "Serving log file view");
            routes::serve_log_view(&cache, &path, query.as_deref())
        }
        ("GET", "/api/logs/process") => {
            debug!(query = ?query, "Serving process history");
            routes::serve_process_history(&cache, &path, query.as_deref())
        }
        (_, path_str) if path_str.starts_with("/assets/") || path_str.ends_with(".svg") => {
            debug!(path = %path_str, "Serving static asset");
            static_files::serve_static_asset(path_str)
        }
        _ => {
            warn!(path = %path, method = %method, "Request to unknown path");
            responses::not_found_response("Endpoint not found")
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

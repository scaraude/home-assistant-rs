mod query;
mod responses;
mod routes;
mod static_files;
pub mod websocket;

use crate::db::Database;
use crate::http::query::DeviceStatePath;
use crate::mqtt::MqttClient;
use crate::services::WebSocketBroadcaster;
use crate::state::{DeviceStateStore, SwitchStateStore};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct HttpServer {
    db: Arc<Database>,
    mqtt: Arc<Mutex<MqttClient>>,
    switch_state: Arc<SwitchStateStore>,
    device_state: DeviceStateStore,
    addr: SocketAddr,
    ws_broadcaster: Arc<WebSocketBroadcaster>,
}

impl HttpServer {
    pub fn new(
        db: Arc<Database>,
        mqtt: Arc<Mutex<MqttClient>>,
        switch_state: Arc<SwitchStateStore>,
        device_state: DeviceStateStore,
        addr: SocketAddr,
        ws_broadcaster: Arc<WebSocketBroadcaster>,
    ) -> Self {
        Self {
            db,
            mqtt,
            switch_state,
            device_state,
            addr,
            ws_broadcaster,
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
                    let mqtt = self.mqtt.clone();
                    let switch_state = self.switch_state.clone();
                    let device_state = self.device_state.clone();
                    let ws_broadcaster = self.ws_broadcaster.clone();
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
                                        mqtt.clone(),
                                        switch_state.clone(),
                                        device_state.clone(),
                                        ws_broadcaster.clone(),
                                    )
                                }),
                            )
                            .with_upgrades()
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
    mqtt: Arc<Mutex<MqttClient>>,
    switch_state: Arc<SwitchStateStore>,
    device_state: DeviceStateStore,
    ws_broadcaster: Arc<WebSocketBroadcaster>,
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

    let response_result = match (method.as_str(), path.as_str()) {
        ("GET", "/") => Ok({
            debug!("Serving index page");
            static_files::serve_static_file("static/index.html", "text/html; charset=utf-8")
        }),
        ("GET", "/health") => Ok({
            debug!("Health check request");
            routes::health_check()
        }),
        ("GET", "/api/sensors") => Ok({
            debug!("Serving sensors list");
            routes::serve_sensors(&db)
        }),
        ("GET", "/api/readings") => Ok({
            debug!(query = ?query, "Serving readings");
            routes::serve_readings(&db, query.as_deref())
        }),
        ("GET", "/api/devices/switches") => Ok({
            debug!("Serving switches list");
            routes::serve_switches_list(&db, &switch_state, &device_state)
        }),
        ("GET", path) if DeviceStatePath::parse(path).is_some() => Ok({
            let device_path = DeviceStatePath::parse(path).expect("path validated");
            debug!(path = %path, "Serving device state");
            routes::serve_device_state(&db, device_path.device_id)
        }),
        ("POST", "/api/commands/execute") => Ok({
            debug!("Executing command");
            routes::execute_command(req, &db, &mqtt).await
        }),
        ("POST", "/api/zigbee/permit_join") => Ok({
            debug!("Publishing Zigbee permit join request");
            routes::permit_join(req, &mqtt).await
        }),
        ("PATCH", path) if path.starts_with("/api/devices/") => Ok({
            debug!(path = %path, "Updating device");
            routes::update_device(req, &db, path).await
        }),
        ("GET", "/api/automation/rules") => Ok({
            debug!("Serving automation rules list");
            routes::serve_automation_rules(&db)
        }),
        ("GET", path) if path.starts_with("/api/automation/rules/") => Ok({
            debug!(path = %path, "Serving automation rule by ID");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::serve_automation_rule(&db, rule_id)
        }),
        ("POST", "/api/automation/rules") => Ok({
            debug!("Creating automation rule");
            routes::create_automation_rule(req, &db).await
        }),
        ("PUT", path) if path.starts_with("/api/automation/rules/") => Ok({
            debug!(path = %path, "Updating automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::update_automation_rule(req, &db, rule_id).await
        }),
        ("DELETE", path) if path.starts_with("/api/automation/rules/") => Ok({
            debug!(path = %path, "Deleting automation rule");
            let rule_id = &path["/api/automation/rules/".len()..];
            routes::delete_automation_rule(&db, rule_id)
        }),
        ("GET", "/api/automation/logs") => Ok({
            debug!("Serving automation execution logs");
            routes::serve_execution_logs(&db, query.as_deref())
        }),
        ("GET", "/api/logs/list") => Ok({
            debug!("Serving logs list");
            routes::serve_logs_list()
        }),
        ("GET", "/api/logs/view") => Ok({
            debug!(query = ?query, "Serving log file view");
            routes::serve_log_view(query.as_deref())
        }),
        ("GET", "/api/logs/since") => Ok({
            debug!(query = ?query, "Serving log file since timestamp");
            routes::serve_log_since(query.as_deref())
        }),
        ("GET", "/api/logs/process") => Ok({
            debug!(query = ?query, "Serving process history");
            routes::serve_process_history(query.as_deref())
        }),
        ("GET", "/ws") => websocket::handle_websocket_upgrade(req, ws_broadcaster).await,
        (_, path_str) if path_str.starts_with("/assets/") || path_str.ends_with(".svg") => Ok({
            debug!(path = %path_str, "Serving static asset");
            static_files::serve_static_asset(path_str)
        }),
        _ => Ok({
            warn!(path = %path, method = %method, "Request to unknown path");
            responses::not_found_response("Endpoint not found")
        }),
    };

    let response = response_result?;
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

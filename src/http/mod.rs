use crate::db::Database;
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
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(db: Arc<Database>, addr: SocketAddr) -> Self {
        Self { db, addr }
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
                    let conn_id = connection_count;

                    tokio::spawn(async move {
                        debug!(connection_id = conn_id, client_addr = %client_addr, "Starting HTTP connection handler");

                        // Clone once per connection, reused across all requests via the service closure
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service_fn(|req| handle_request(req, db.clone())))
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

    let response = match path {
        "/" => {
            debug!("Serving index page");
            serve_index()
        }
        "/api/sensors" => {
            debug!("Serving sensors list");
            serve_sensors(&db)
        }
        "/api/readings" => {
            debug!(query = ?query, "Serving readings");
            serve_readings(&db, query)
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

fn serve_index() -> Response<Full<Bytes>> {
    let html = include_str!("../../static/index.html");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

fn serve_sensors(db: &Database) -> Response<Full<Bytes>> {
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
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(json)))
                        .unwrap()
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

fn serve_readings(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    debug!(query = ?query, "Parsing query parameters");

    // Parse query parameters for sensor_id and hours
    let mut sensor_id: Option<&str> = None;
    let mut hours: i64 = 24; // Default to last 24 hours

    if let Some(q) = query {
        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "sensor_id" => {
                        sensor_id = Some(value);
                        debug!(sensor_id = %value, "Parsed sensor_id parameter");
                    }
                    "hours" => {
                        hours = value.parse().unwrap_or_else(|e| {
                            warn!(
                                value = %value,
                                error = %e,
                                "Failed to parse hours parameter, using default (24)"
                            );
                            24
                        });
                        debug!(hours = hours, "Parsed hours parameter");
                    }
                    _ => {
                        debug!(key = %key, value = %value, "Ignoring unknown query parameter");
                    }
                }
            }
        }
    }

    // Calculate timestamp for the time range
    let since = chrono::Utc::now().timestamp() - (hours * 3600);

    info!(
        sensor_id = ?sensor_id,
        hours = hours,
        since_timestamp = since,
        "Querying readings"
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

            match serde_json::to_string(&readings) {
                Ok(json) => {
                    info!(
                        reading_count = readings.len(),
                        response_size = json.len(),
                        sensor_id = ?sensor_id,
                        hours = hours,
                        "Successfully serialized readings to JSON"
                    );
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(json)))
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

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

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

pub struct HttpServer {
    db: Arc<Database>,
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(db: Arc<Database>, addr: SocketAddr) -> Self {
        Self { db, addr }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.addr).await?;
        println!("HTTP server listening on http://{}", self.addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let db = self.db.clone();

            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| handle_request(req, db.clone())))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    db: Arc<Database>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path();

    match path {
        "/" => Ok(serve_index()),
        "/api/sensors" => Ok(serve_sensors(&db)),
        "/api/readings" => Ok(serve_readings(&db, req.uri().query())),
        _ => Ok(not_found()),
    }
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
    match db.get_all_sensors() {
        Ok(sensors) => {
            let json = serde_json::to_string(&sensors).unwrap_or_else(|_| "[]".to_string());
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Database error")))
                .unwrap()
        }
    }
}

fn serve_readings(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    // Parse query parameters for sensor_id and hours
    let mut sensor_id: Option<&str> = None;
    let mut hours: i64 = 24; // Default to last 24 hours

    if let Some(q) = query {
        for param in q.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "sensor_id" => sensor_id = Some(value),
                    "hours" => hours = value.parse().unwrap_or(24),
                    _ => {}
                }
            }
        }
    }

    // Calculate timestamp for the time range
    let since = chrono::Utc::now().timestamp() - (hours * 3600);

    let result = if let Some(sid) = sensor_id {
        // Get readings for specific sensor
        db.get_recent_readings(sid, 1000).map(|mut readings| {
            readings.retain(|r| r.timestamp.timestamp() > since);
            readings
        })
    } else {
        // Get all readings since timestamp
        db.get_readings_since(since)
    };

    match result {
        Ok(readings) => {
            let json = serde_json::to_string(&readings).unwrap_or_else(|_| "[]".to_string());
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap()
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
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

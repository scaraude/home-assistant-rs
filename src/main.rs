mod cache;
mod db;
mod http;
mod logs;
mod models;
mod mqtt;

use cache::ResponseCache;
use db::Database;
use http::HttpServer;
use mqtt::MqttListener;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with environment filter (use RUST_LOG env var to control)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🏠 Home Assistant RS - Starting...");

    // Configuration from environment variables with defaults
    let mqtt_broker = std::env::var("MQTT_BROKER").unwrap_or_else(|_| "localhost".to_string());
    let mqtt_port = std::env::var("MQTT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1883);
    let http_addr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "home_assistant.db".to_string());

    debug!(
        mqtt_broker = %mqtt_broker,
        mqtt_port = %mqtt_port,
        http_addr = %http_addr,
        db_path = %db_path,
        "Configuration loaded"
    );

    // Initialize database
    info!(db_path = %db_path, "Opening database");
    let db = Arc::new(Database::new(db_path)?);
    info!("Database initialized successfully");

    // Start MQTT listener
    info!(
        broker = %mqtt_broker,
        port = mqtt_port,
        "Connecting to MQTT broker"
    );
    let (mqtt_listener, mut readings_rx) =
        MqttListener::new(&mqtt_broker, mqtt_port, "home-assistant-rs");

    mqtt_listener.subscribe().await?;
    info!("Successfully subscribed to zigbee2mqtt topics");

    // Spawn database writer task
    let db_clone = db.clone();
    tokio::spawn(async move {
        info!("Database writer task started");
        let mut reading_count = 0u64;

        while let Some(reading) = readings_rx.recv().await {
            reading_count += 1;

            info!(
                sensor_id = %reading.sensor_id,
                temperature = %reading.temperature,
                humidity = ?reading.humidity,
                battery = ?reading.battery,
                count = reading_count,
                "Received sensor reading"
            );

            if let Err(e) = db_clone.insert_reading(&reading) {
                error!(
                    error = %e,
                    sensor_id = %reading.sensor_id,
                    reading_count = reading_count,
                    "Failed to insert reading into database"
                );
            } else {
                debug!(
                    sensor_id = %reading.sensor_id,
                    reading_count = reading_count,
                    "Successfully inserted reading"
                );
            }
        }

        warn!("Database writer task channel closed - no more readings will be processed");
    });

    // Initialize response cache (60 second TTL matches monitor.sh interval)
    let cache = Arc::new(ResponseCache::new(60));
    info!("Response cache initialized with 60s TTL");

    // Start HTTP server
    info!(addr = %http_addr, "Starting HTTP server");
    let server = HttpServer::new(db, cache, http_addr);
    server.run().await?;

    Ok(())
}

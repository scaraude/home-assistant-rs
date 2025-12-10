mod cache;
mod db;
mod device_state;
mod http;
mod logs;
mod models;
mod mqtt;
mod switch_state;

use cache::ResponseCache;
use db::Database;
use device_state::DeviceStateStore;
use http::HttpServer;
use mqtt::MqttListener;
use std::sync::Arc;
use switch_state::SwitchStateStore;
use tokio::sync::Mutex;
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

    // Initialize device state store
    let device_state = DeviceStateStore::new();
    info!("Device state store initialized");

    // Initialize switch state store (before MQTT listener so we can pass it)
    let switch_state = Arc::new(SwitchStateStore::new());
    info!("Switch state store initialized");

    // Start MQTT listener
    info!(
        broker = %mqtt_broker,
        port = mqtt_port,
        "Connecting to MQTT broker"
    );
    let (mqtt_listener, mut readings_rx) = MqttListener::new(
        &mqtt_broker,
        mqtt_port,
        "home-assistant-rs",
        db.clone(),
        device_state.clone(),
        (*switch_state).clone(),
    );

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
                device_id = %reading.device_id,
                temperature = %reading.temperature,
                humidity = ?reading.humidity,
                count = reading_count,
                "Received sensor reading"
            );

            if let Err(e) = db_clone.insert_reading(&reading) {
                error!(
                    error = %e,
                    device_id = %reading.device_id,
                    reading_count = reading_count,
                    "Failed to insert reading into database"
                );
            } else {
                debug!(
                    device_id = %reading.device_id,
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

    // Spawn cache cleanup task (runs every 5 minutes to prevent memory accumulation)
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        info!("Cache cleanup task started (runs every 5 minutes)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes

        loop {
            interval.tick().await;
            let removed = cache_clone.cleanup_expired();
            if removed > 0 {
                info!(
                    removed_entries = removed,
                    "Cache cleanup: removed expired entries"
                );
            } else {
                debug!("Cache cleanup: no expired entries to remove");
            }
        }
    });

    // Wrap MQTT listener in Arc<Mutex> for sharing with HTTP server
    let mqtt_listener = Arc::new(Mutex::new(mqtt_listener));

    // Start HTTP server
    info!(addr = %http_addr, "Starting HTTP server");
    let server = HttpServer::new(db, cache, mqtt_listener, switch_state, http_addr);
    server.run().await?;

    Ok(())
}

mod automation;
mod cache;
mod db;
mod events;
mod http;
mod logs;
mod models;
mod mqtt;
mod services;
mod state;

use cache::ResponseCache;
use db::Database;
use events::bus::EventBus;
use http::HttpServer;
use mqtt::MqttClient;
use services::{AutomationService, DbWriterService, StateManagerService};
use state::{DeviceStateStore, SwitchStateStore};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

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
    let switch_state = SwitchStateStore::new();
    info!("Switch state store initialized");

    let event_bus = EventBus::new(1000);
    info!("Event bus initialized with capacity 1000");

    // Start MQTT client
    info!(
        broker = %mqtt_broker,
        port = mqtt_port,
        "Connecting to MQTT broker"
    );
    let mqtt_client = MqttClient::new(
        &mqtt_broker,
        mqtt_port,
        "home-assistant-rs",
        db.clone(),
        event_bus.clone(),
    );

    mqtt_client.subscribe_to_zigbee2mqtt().await?;
    info!("Successfully subscribed to zigbee2mqtt topics");

    // Spawn background services that consume the event bus.
    let db_writer = DbWriterService::new(db.clone(), event_bus.subscribe());
    tokio::spawn(async move {
        db_writer.run().await;
    });

    let state_manager =
        StateManagerService::new(device_state.clone(), switch_state.clone(), event_bus.subscribe());
    tokio::spawn(async move {
        state_manager.run().await;
    });

    let automation_service = AutomationService::new(
        db.clone(),
        mqtt_client.clone(),
        device_state.clone(),
        switch_state.clone(),
        event_bus.clone(),
        event_bus.subscribe(),
    );
    tokio::spawn(async move {
        automation_service.run().await;
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

    // Wrap MQTT client in Arc<Mutex> for sharing with HTTP server
    let mqtt_client = Arc::new(Mutex::new(mqtt_client));
    let switch_state = Arc::new(switch_state);

    // Start HTTP server
    info!(addr = %http_addr, "Starting HTTP server");
    let server = HttpServer::new(
        db,
        cache,
        mqtt_client,
        switch_state,
        device_state,
        http_addr,
    );
    server.run().await?;

    Ok(())
}

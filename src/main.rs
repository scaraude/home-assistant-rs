mod db;
mod events;
mod http;
mod logs;
mod models;
mod mqtt;
mod services;
mod state;
mod system;

use db::Database;
use events::bus::EventBus;
use http::HttpServer;
use mqtt::MqttClient;
use services::{
    AutomationService, DbWriterService, LogWatcherService, NotifierService, RetentionService,
    StateManagerService, WebSocketBroadcaster,
};
use state::{DeviceStateStore, SwitchStateStore};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Event bus capacity sized for short bursts of MQTT + service events.
const EVENT_BUS_CAPACITY: usize = 1000;
/// MQTT client queue size for outgoing commands and inbound processing.
const MQTT_CLIENT_QUEUE_CAPACITY: usize = 20;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with environment filter (use RUST_LOG env var to control)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🏠 Home Automation RS - Starting...");

    // Configuration from environment variables with defaults
    let mqtt_broker = std::env::var("MQTT_BROKER").unwrap_or_else(|_| "localhost".to_string());
    let mqtt_port = std::env::var("MQTT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1883);
    let http_addr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "home_automation.db".to_string());

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

    let event_bus = EventBus::new(EVENT_BUS_CAPACITY);
    info!(
        event_bus_capacity = EVENT_BUS_CAPACITY,
        "Event bus initialized"
    );

    let ws_broadcaster = Arc::new(WebSocketBroadcaster::new(event_bus.subscribe()));
    let ws_broadcaster_task = ws_broadcaster.clone();
    tokio::spawn(async move {
        ws_broadcaster_task.run().await;
    });
    info!("WebSocket broadcaster task spawned");

    // Start MQTT client
    info!(
        broker = %mqtt_broker,
        port = mqtt_port,
        "Connecting to MQTT broker"
    );
    let mqtt_client = MqttClient::new(
        &mqtt_broker,
        mqtt_port,
        "home-automation-rs",
        Some(MQTT_CLIENT_QUEUE_CAPACITY),
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

    let state_manager = StateManagerService::new(
        device_state.clone(),
        switch_state.clone(),
        event_bus.subscribe(),
    );
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

    // Spawn log watcher service to monitor log files and emit events
    let log_watcher = LogWatcherService::new(event_bus.sender());
    tokio::spawn(async move {
        log_watcher.run().await;
    });
    info!("LogWatcherService task spawned");

    // Spawn notifier service: pushes device availability alerts to a Discord
    // webhook, with a SQLite-backed queue for offline periods.
    let notifier = NotifierService::new(db.clone(), event_bus.subscribe());
    tokio::spawn(async move {
        notifier.run().await;
    });
    info!("NotifierService task spawned");

    // Spawn retention service to downsample historical readings (keeps the DB
    // small enough to fit the SQLite page cache on limited RAM).
    let retention = RetentionService::new(db.clone());
    tokio::spawn(async move {
        retention.run().await;
    });
    info!("RetentionService task spawned");

    // Wrap MQTT client in Arc<Mutex> for sharing with HTTP server
    let mqtt_client = Arc::new(Mutex::new(mqtt_client));
    let switch_state = Arc::new(switch_state);

    // Start HTTP server
    info!(addr = %http_addr, "Starting HTTP server");
    let server = HttpServer::new(
        db,
        mqtt_client,
        switch_state,
        device_state,
        http_addr,
        ws_broadcaster,
        event_bus,
    );
    server.run().await?;

    Ok(())
}

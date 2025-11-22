mod db;
mod http;
mod models;
mod mqtt;

use db::Database;
use http::HttpServer;
use mqtt::MqttListener;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("🏠 Home Assistant RS - Starting...");

    // Initialize database
    println!("📊 Opening database: {}", db_path);
    let db = Arc::new(Database::new(db_path)?);

    // Start MQTT listener
    println!(
        "📡 Connecting to MQTT broker at {}:{}",
        mqtt_broker, mqtt_port
    );
    let (mqtt_listener, mut readings_rx) =
        MqttListener::new(&mqtt_broker, mqtt_port, "home-assistant-rs");

    mqtt_listener.subscribe().await?;
    println!("✅ Subscribed to zigbee2mqtt topics");

    // Spawn database writer task
    let db_clone = db.clone();
    tokio::spawn(async move {
        while let Some(reading) = readings_rx.recv().await {
            println!(
                "🌡️  {} = {:.1}°C (humidity: {:?}, battery: {:?})",
                reading.sensor_id, reading.temperature, reading.humidity, reading.battery
            );

            if let Err(e) = db_clone.insert_reading(&reading) {
                eprintln!("❌ Failed to insert reading: {}", e);
            }
        }
    });

    // Start HTTP server
    println!("🌐 Starting HTTP server");
    let server = HttpServer::new(db, http_addr);
    server.run().await?;

    Ok(())
}

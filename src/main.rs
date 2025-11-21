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
    // Configuration - TODO: Move to config file or environment variables
    let mqtt_broker = "localhost";
    let mqtt_port = 1883;
    let http_addr = "0.0.0.0:8080".parse()?;
    let db_path = "home_assistant.db";

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
        MqttListener::new(mqtt_broker, mqtt_port, "home-assistant-rs");

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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Temperature reading from a sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureReading {
    /// Unique sensor identifier (e.g., "0x00158d0001a2b3c4")
    pub sensor_id: String,

    /// Temperature in Celsius
    pub temperature: f32,

    /// Optional humidity percentage
    pub humidity: Option<f32>,

    /// Optional battery level percentage
    pub battery: Option<u8>,

    /// Timestamp when the reading was received
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

/// Zigbee2MQTT message format
/// This matches the JSON structure published by zigbee2mqtt
#[derive(Debug, Deserialize)]
pub struct Zigbee2MqttMessage {
    /// Temperature in Celsius
    pub temperature: Option<f32>,

    /// Humidity percentage
    pub humidity: Option<f32>,

    /// Battery level percentage
    pub battery: Option<u8>,

    /// Link quality indicator (not currently used but present in messages)
    #[allow(dead_code)]
    pub linkquality: Option<u8>,
}

impl TemperatureReading {
    /// Create a new temperature reading from a zigbee2mqtt message
    pub fn from_mqtt(sensor_id: String, msg: Zigbee2MqttMessage) -> Option<Self> {
        // Only create a reading if we have temperature data
        msg.temperature.map(|temperature| Self {
            sensor_id,
            temperature,
            humidity: msg.humidity,
            battery: msg.battery,
            timestamp: Utc::now(),
        })
    }
}

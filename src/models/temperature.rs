use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Temperature reading from a sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureReading {
    /// Device ID (foreign key to devices table)
    pub device_id: String,

    /// Temperature in Celsius
    pub temperature: f32,

    /// Optional humidity percentage
    pub humidity: Option<f32>,

    /// Optional battery level percentage
    pub battery: Option<u8>,

    /// Optional link quality indicator
    pub link_quality: Option<u8>,

    /// Timestamp when the reading was received
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

/// Zigbee2MQTT message format
/// This matches the JSON structure published by zigbee2mqtt
#[derive(Debug, Deserialize, Clone)]
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
    /// Note: device_id should be the UUID from the devices table, not the MQTT topic
    pub fn from_mqtt(device_id: String, msg: Zigbee2MqttMessage) -> Option<Self> {
        debug!(
            device_id = %device_id,
            temperature = ?msg.temperature,
            humidity = ?msg.humidity,
            "Attempting to create TemperatureReading from MQTT message"
        );

        // Only create a reading if we have temperature data
        match msg.temperature {
            Some(temperature) => {
                debug!(
                    device_id = %device_id,
                    temperature = %temperature,
                    humidity = ?msg.humidity,
                    battery = ?msg.battery,
                    linkquality = ?msg.linkquality,
                    "Successfully created TemperatureReading"
                );
                Some(Self {
                    device_id,
                    temperature,
                    humidity: msg.humidity,
                    battery: msg.battery,
                    link_quality: msg.linkquality,
                    timestamp: Utc::now(),
                })
            }
            None => {
                debug!(
                    device_id = %device_id,
                    humidity = ?msg.humidity,
                    battery = ?msg.battery,
                    linkquality = ?msg.linkquality,
                    "MQTT message does not contain temperature data - skipping reading creation"
                );
                None
            }
        }
    }
}

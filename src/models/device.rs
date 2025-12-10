use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of device in the home automation system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    #[serde(rename = "temphumiditysensor")]
    TempHumiditySensor,
    Commander,
}

impl DeviceType {
    /// Convert to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            DeviceType::TempHumiditySensor => "temphumiditysensor",
            DeviceType::Commander => "commander",
        }
    }

    /// Parse from database string representation
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "temphumiditysensor" => Some(DeviceType::TempHumiditySensor),
            "commander" => Some(DeviceType::Commander),
            _ => None,
        }
    }
}

/// Power source for a device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PowerSource {
    Battery,
    Plugged,
}

impl PowerSource {
    /// Convert to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            PowerSource::Battery => "battery",
            PowerSource::Plugged => "plugged",
        }
    }

    /// Parse from database string representation
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "battery" => Some(PowerSource::Battery),
            "plugged" => Some(PowerSource::Plugged),
            _ => None,
        }
    }
}

/// Static device metadata stored in database
/// This represents the fixed properties of a device that rarely change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique identifier (UUID stored as string for SQLite compatibility)
    pub id: String,

    /// MQTT topic for this device (e.g., "0x00158d0001a2b3c4")
    pub mqtt_topic: String,

    /// Human-readable name for the device
    pub name: String,

    /// Type of device (sensor, commander, etc.)
    pub device_type: DeviceType,

    /// How the device is powered
    pub power_source: PowerSource,

    /// When the device was added to the system
    #[serde(with = "chrono::serde::ts_seconds")]
    pub added_at: DateTime<Utc>,
}

impl Device {
    /// Create a new device with a generated UUID
    pub fn new(
        mqtt_topic: String,
        name: String,
        device_type: DeviceType,
        power_source: PowerSource,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            mqtt_topic,
            name,
            device_type,
            power_source,
            added_at: Utc::now(),
        }
    }

    /// Check if this device is battery-powered
    pub fn is_battery_powered(&self) -> bool {
        self.power_source == PowerSource::Battery
    }
}

/// Dynamic device state (ephemeral, not persisted to DB)
/// This represents frequently changing data that's tracked in memory
/// and reconstructed from MQTT messages on restart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    /// Reference to the device ID
    pub device_id: String,

    /// Link quality indicator (0-255)
    pub link_quality: Option<u8>,

    /// Battery level percentage (0-100), only meaningful for battery-powered devices
    pub battery_level: Option<u8>,

    /// Last time we received a message from this device
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_seen: DateTime<Utc>,
}

impl DeviceState {
    /// Create a new device state
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            link_quality: None,
            battery_level: None,
            last_seen: Utc::now(),
        }
    }

    /// Update link quality from MQTT message
    pub fn update_link_quality(&mut self, link_quality: u8) {
        self.link_quality = Some(link_quality);
        self.last_seen = Utc::now();
    }

    /// Update battery level from MQTT message
    pub fn update_battery(&mut self, battery: u8) {
        self.battery_level = Some(battery);
        self.last_seen = Utc::now();
    }

    /// Mark the device as seen (updates last_seen timestamp)
    pub fn mark_seen(&mut self) {
        self.last_seen = Utc::now();
    }
}

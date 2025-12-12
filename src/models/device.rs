use crate::impl_db_enum;
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

// Implement database string conversion using the DbEnum trait
impl_db_enum!(DeviceType {
    TempHumiditySensor => "temphumiditysensor",
    Commander => "commander"
});

/// Power source for a device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PowerSource {
    Battery,
    Plugged,
}

// Implement database string conversion using the DbEnum trait
impl_db_enum!(PowerSource {
    Battery => "battery",
    Plugged => "plugged"
});

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

/// Minimal device info for API responses (ID + name only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID
    pub device_id: String,

    /// Human-readable name
    pub name: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::db_enum::DbEnum;

    #[test]
    fn test_device_type_to_db_string() {
        assert_eq!(
            DeviceType::TempHumiditySensor.to_db_string(),
            "temphumiditysensor"
        );
        assert_eq!(DeviceType::Commander.to_db_string(), "commander");
    }

    #[test]
    fn test_device_type_from_db_string() {
        assert_eq!(
            DeviceType::from_db_string("temphumiditysensor"),
            Some(DeviceType::TempHumiditySensor)
        );
        assert_eq!(
            DeviceType::from_db_string("commander"),
            Some(DeviceType::Commander)
        );
        assert_eq!(DeviceType::from_db_string("invalid"), None);
    }

    #[test]
    fn test_device_type_roundtrip() {
        let types = vec![DeviceType::TempHumiditySensor, DeviceType::Commander];
        for device_type in types {
            let db_string = device_type.to_db_string();
            let parsed = DeviceType::from_db_string(db_string);
            assert_eq!(Some(device_type), parsed);
        }
    }

    #[test]
    fn test_power_source_to_db_string() {
        assert_eq!(PowerSource::Battery.to_db_string(), "battery");
        assert_eq!(PowerSource::Plugged.to_db_string(), "plugged");
    }

    #[test]
    fn test_power_source_from_db_string() {
        assert_eq!(
            PowerSource::from_db_string("battery"),
            Some(PowerSource::Battery)
        );
        assert_eq!(
            PowerSource::from_db_string("plugged"),
            Some(PowerSource::Plugged)
        );
        assert_eq!(PowerSource::from_db_string("invalid"), None);
    }

    #[test]
    fn test_power_source_roundtrip() {
        let sources = vec![PowerSource::Battery, PowerSource::Plugged];
        for source in sources {
            let db_string = source.to_db_string();
            let parsed = PowerSource::from_db_string(db_string);
            assert_eq!(Some(source), parsed);
        }
    }
}

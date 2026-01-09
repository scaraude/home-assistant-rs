use crate::impl_db_enum;
use crate::models::db_enum::DbEnum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device capability - what can this device do?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DeviceCapability {
    Sensor { sensor_type: SensorType },
    Commander { commander_type: CommanderType },
}

/// Type of sensor capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SensorType {
    #[serde(rename = "temp_humidity")]
    TempHumidity,
    Presence,
}

impl_db_enum!(SensorType {
    TempHumidity => "temp_humidity",
    Presence => "presence"
});

/// Type of commander capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommanderType {
    Switch,
}

impl_db_enum!(CommanderType {
    Switch => "switch"
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

    /// IEEE address for this device (usually matches mqtt_topic)
    pub ieee_addr: String,

    /// Human-readable name for the device
    pub name: String,

    /// What can this device do (sensor or commander)
    pub capability: DeviceCapability,

    /// How the device is powered
    pub power_source: PowerSource,

    /// When the device was added to the system
    #[serde(with = "chrono::serde::ts_seconds")]
    pub added_at: DateTime<Utc>,

    /// Whether this device acts as a router in the Zigbee mesh
    pub is_bridge: bool,

    /// Parent device in the mesh network (None for coordinator)
    pub parent_device_id: Option<String>,
}

impl Device {
    /// Create a new device with a generated UUID
    pub fn new(
        mqtt_topic: String,
        name: String,
        capability: DeviceCapability,
        power_source: PowerSource,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ieee_addr: mqtt_topic.clone(),
            mqtt_topic,
            name,
            capability,
            power_source,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
        }
    }

    /// Check if this device is battery-powered
    pub fn _is_battery_powered(&self) -> bool {
        self.power_source == PowerSource::Battery
    }

    /// Get the capability type and subtype for database storage
    pub fn capability_to_db(&self) -> (String, String) {
        match &self.capability {
            DeviceCapability::Sensor { sensor_type } => {
                ("sensor".to_string(), sensor_type.to_db_string().to_string())
            }
            DeviceCapability::Commander { commander_type } => (
                "commander".to_string(),
                commander_type.to_db_string().to_string(),
            ),
        }
    }

    /// Create capability from database strings
    pub fn capability_from_db(cap_type: &str, cap_subtype: &str) -> Option<DeviceCapability> {
        match cap_type {
            "sensor" => SensorType::from_db_string(cap_subtype)
                .map(|sensor_type| DeviceCapability::Sensor { sensor_type }),
            "commander" => CommanderType::from_db_string(cap_subtype)
                .map(|commander_type| DeviceCapability::Commander { commander_type }),
            _ => None,
        }
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
}

/// Network topology response containing devices and their connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    /// All devices in the network
    pub devices: Vec<Device>,
    /// Network edges computed from parent_device_id relationships
    pub edges: Vec<NetworkEdge>,
}

/// A connection between two devices in the Zigbee mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEdge {
    /// Source device ID (parent)
    pub source_id: String,
    /// Target device ID (child)
    pub target_id: String,
    /// Link quality indicator (0-255), if available
    pub link_quality: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::db_enum::DbEnum;

    #[test]
    fn test_sensor_type_to_db_string() {
        assert_eq!(SensorType::TempHumidity.to_db_string(), "temp_humidity");
        assert_eq!(SensorType::Presence.to_db_string(), "presence");
    }

    #[test]
    fn test_sensor_type_from_db_string() {
        assert_eq!(
            SensorType::from_db_string("temp_humidity"),
            Some(SensorType::TempHumidity)
        );
        assert_eq!(
            SensorType::from_db_string("presence"),
            Some(SensorType::Presence)
        );
        assert_eq!(SensorType::from_db_string("invalid"), None);
    }

    #[test]
    fn test_commander_type_to_db_string() {
        assert_eq!(CommanderType::Switch.to_db_string(), "switch");
    }

    #[test]
    fn test_commander_type_from_db_string() {
        assert_eq!(
            CommanderType::from_db_string("switch"),
            Some(CommanderType::Switch)
        );
        assert_eq!(CommanderType::from_db_string("invalid"), None);
    }

    #[test]
    fn test_capability_to_db() {
        let device = Device::new(
            "0x123".to_string(),
            "Test Sensor".to_string(),
            DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            PowerSource::Battery,
        );
        let (cap_type, cap_subtype) = device.capability_to_db();
        assert_eq!(cap_type, "sensor");
        assert_eq!(cap_subtype, "temp_humidity");

        let device = Device::new(
            "0x456".to_string(),
            "Test Switch".to_string(),
            DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            },
            PowerSource::Plugged,
        );
        let (cap_type, cap_subtype) = device.capability_to_db();
        assert_eq!(cap_type, "commander");
        assert_eq!(cap_subtype, "switch");
    }

    #[test]
    fn test_capability_from_db() {
        let cap = Device::capability_from_db("sensor", "temp_humidity");
        assert_eq!(
            cap,
            Some(DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity
            })
        );

        let cap = Device::capability_from_db("commander", "switch");
        assert_eq!(
            cap,
            Some(DeviceCapability::Commander {
                commander_type: CommanderType::Switch
            })
        );

        let cap = Device::capability_from_db("invalid", "invalid");
        assert_eq!(cap, None);
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

use crate::impl_db_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Device capability - what can this device do?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DeviceCapability {
    Sensor { sensor_type: SensorType },
    Commander { commander_type: CommanderType },
    Router { turbo_mode: bool },
    Coordinator,
}

/// Type of sensor capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SensorType {
    #[serde(rename = "temp_humidity")]
    TempHumidity,
    Presence,
    #[serde(rename = "energy_meter")]
    EnergyMeter,
}

impl_db_enum!(SensorType {
    TempHumidity => "temp_humidity",
    Presence => "presence",
    EnergyMeter => "energy_meter"
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

    /// What can this device do (supports multiple capabilities)
    pub capabilities: Vec<DeviceCapability>,

    /// Available fields reported by the device (from Zigbee2MQTT payloads)
    pub available_fields: Vec<String>,

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
        capabilities: Vec<DeviceCapability>,
        available_fields: Vec<String>,
        power_source: PowerSource,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ieee_addr: mqtt_topic.clone(),
            mqtt_topic,
            name,
            capabilities,
            available_fields,
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

    /// Check if this device has a specific capability
    #[cfg(test)]
    pub fn has_capability(&self, capability: &DeviceCapability) -> bool {
        self.capabilities.iter().any(|cap| cap == capability)
    }

    /// Check if this device has a specific sensor capability
    #[cfg(test)]
    pub fn has_sensor_type(&self, sensor_type: &SensorType) -> bool {
        self.capabilities.iter().any(|cap| {
            matches!(cap, DeviceCapability::Sensor { sensor_type: cap_type } if cap_type == sensor_type)
        })
    }

    /// Check if this device has a specific commander capability
    #[cfg(test)]
    pub fn has_commander_type(&self, commander_type: &CommanderType) -> bool {
        self.capabilities.iter().any(|cap| {
            matches!(cap, DeviceCapability::Commander { commander_type: cap_type } if cap_type == commander_type)
        })
    }

    /// Check if the device supports turbo mode based on available fields
    pub fn supports_turbo_mode(&self) -> bool {
        self.available_fields
            .iter()
            .any(|field| field == "turbo_mode")
    }

    /// Serialize capabilities for database storage
    pub fn capabilities_to_db(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.capabilities)
    }

    /// Serialize available fields for database storage
    pub fn available_fields_to_db(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.available_fields)
    }

    /// Deserialize capabilities from database string
    pub fn capabilities_from_db(value: &str) -> Option<Vec<DeviceCapability>> {
        if value.trim().is_empty() {
            return Some(Vec::new());
        }
        serde_json::from_str(value).ok()
    }

    /// Deserialize available fields from database string
    pub fn available_fields_from_db(value: &str) -> Option<Vec<String>> {
        if value.trim().is_empty() {
            return Some(Vec::new());
        }
        serde_json::from_str(value).ok()
    }
}

/// Minimal device info for API responses (ID + name + capability)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID
    pub device_id: String,

    /// Human-readable name
    pub name: String,

    /// Device capabilities
    pub capabilities: Vec<DeviceCapability>,

    /// Available fields reported by the device
    pub available_fields: Vec<String>,
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

    /// Turbo mode flag, if supported by the device
    pub turbo_mode: Option<bool>,
}

impl DeviceState {
    /// Create a new device state
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            link_quality: None,
            battery_level: None,
            last_seen: Utc::now(),
            turbo_mode: None,
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

    // Legacy capability serialization is intentionally not supported.

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

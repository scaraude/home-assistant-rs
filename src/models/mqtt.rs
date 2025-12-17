use chrono::{DateTime, Utc};
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::{CommanderType, SensorType};

/// Unified MQTT message format for all device types
/// Zigbee2MQTT sends different fields depending on device type,
/// so we make everything optional and extract what we need
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceMqttMessage {
    // ==================== Common fields (all devices) ====================
    /// Link quality indicator (0-255)
    pub linkquality: Option<u8>,

    /// Battery level percentage (0-100) - only for battery-powered devices
    pub battery: Option<u8>,

    // ==================== Sensor fields ====================
    /// Temperature in Celsius
    pub temperature: Option<f32>,

    /// Humidity percentage
    pub humidity: Option<f32>,

    /// Presence/occupancy detection
    pub occupancy: Option<bool>,

    // ==================== Commander fields ====================
    /// Switch state parsed from MQTT payload
    pub state: Option<SwitchState>,

    // ==================== Catch-all ====================
    /// Other fields we don't use yet
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl DeviceMqttMessage {
    /// Extract common device state fields (battery, link quality)
    pub fn extract_device_state(&self) -> DeviceStateFields {
        DeviceStateFields {
            battery_level: self.battery,
            link_quality: self.linkquality,
        }
    }

    /// Check if this message contains sensor data for the given sensor type
    pub fn has_sensor_data(&self, sensor_type: &SensorType) -> bool {
        match sensor_type {
            SensorType::TempHumidity => self.temperature.is_some(),
            SensorType::Presence => self.occupancy.is_some(),
        }
    }

    /// Check if this message contains commander data for the given commander type
    pub fn has_commander_data(&self, commander_type: &CommanderType) -> bool {
        match commander_type {
            CommanderType::Switch => self.state.is_some(),
        }
    }
}

/// Strongly typed container for device state metrics extracted from MQTT payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceStateFields {
    pub battery_level: Option<u8>,
    pub link_quality: Option<u8>,
}

/// Switch state enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwitchState {
    On,
    Off,
}

impl SwitchState {
    /// Parse from MQTT string value (case-insensitive)
    pub fn from_mqtt_string(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ON" => Some(SwitchState::On),
            "OFF" => Some(SwitchState::Off),
            _ => None,
        }
    }

    pub fn to_mqtt_string(&self) -> &'static str {
        match self {
            SwitchState::On => "ON",
            SwitchState::Off => "OFF",
        }
    }

    /// Convert to boolean (ON = true, OFF = false)
    pub fn to_bool(&self) -> bool {
        matches!(self, SwitchState::On)
    }

    pub fn to_integer(&self) -> i32 {
        match self {
            SwitchState::On => 1,
            SwitchState::Off => 0,
        }
    }

    /// Toggle the switch state.
    pub fn toggled(self) -> Self {
        match self {
            SwitchState::On => SwitchState::Off,
            SwitchState::Off => SwitchState::On,
        }
    }
}

impl Serialize for SwitchState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_mqtt_string())
    }
}

impl<'de> Deserialize<'de> for SwitchState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SwitchStateVisitor;

        impl<'de> de::Visitor<'de> for SwitchStateVisitor {
            type Value = SwitchState;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string (ON/OFF) or boolean")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SwitchState::from_mqtt_string(value)
                    .ok_or_else(|| de::Error::custom(format!("invalid switch state: {}", value)))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(if value {
                    SwitchState::On
                } else {
                    SwitchState::Off
                })
            }
        }

        deserializer.deserialize_any(SwitchStateVisitor)
    }
}

impl Default for SwitchState {
    fn default() -> Self {
        SwitchState::Off
    }
}

impl fmt::Display for SwitchState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwitchState::On => write!(f, "ON"),
            SwitchState::Off => write!(f, "OFF"),
        }
    }
}

/// Sensor reading - time-series data stored in database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SensorReading {
    #[serde(rename = "temp_humidity")]
    TempHumidity {
        device_id: String,
        temperature: f32,
        humidity: f32,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    Presence {
        device_id: String,
        occupied: bool,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
}

impl SensorReading {
    /// Create a sensor reading from MQTT message
    pub fn from_mqtt(
        device_id: String,
        sensor_type: &SensorType,
        msg: &DeviceMqttMessage,
    ) -> Option<Self> {
        match sensor_type {
            SensorType::TempHumidity => msg.temperature.map(|temp| SensorReading::TempHumidity {
                device_id,
                temperature: temp,
                humidity: msg.humidity.unwrap_or(0.0),
                timestamp: Utc::now(),
            }),
            SensorType::Presence => msg.occupancy.map(|occ| SensorReading::Presence {
                device_id,
                occupied: occ,
                timestamp: Utc::now(),
            }),
        }
    }

    /// Get the device ID for this reading
    pub fn device_id(&self) -> &str {
        match self {
            SensorReading::TempHumidity { device_id, .. } => device_id,
            SensorReading::Presence { device_id, .. } => device_id,
        }
    }

    /// Get the timestamp for this reading
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SensorReading::TempHumidity { timestamp, .. } => *timestamp,
            SensorReading::Presence { timestamp, .. } => *timestamp,
        }
    }
}

/// Command to send to a switch device
#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchCommand {
    /// Device ID to control
    pub device_id: String,

    /// Desired state (true = ON, false = OFF)
    pub state: SwitchState,
}

/// Payload sent to Zigbee2MQTT when toggling a switch.
#[derive(Debug, Serialize)]
pub struct SwitchCommandMessage {
    pub state: SwitchState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_switch_state_from_mqtt_string() {
        assert_eq!(SwitchState::from_mqtt_string("ON"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("on"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("OFF"), Some(SwitchState::Off));
        assert_eq!(SwitchState::from_mqtt_string("off"), Some(SwitchState::Off));
        assert_eq!(SwitchState::from_mqtt_string("invalid"), None);
    }

    #[test]
    fn test_switch_state_deserializes_from_bool() {
        #[derive(Deserialize)]
        struct Wrapper {
            state: SwitchState,
        }

        let on: Wrapper = serde_json::from_str(r#"{"state": true}"#).unwrap();
        assert_eq!(on.state, SwitchState::On);

        let off: Wrapper = serde_json::from_str(r#"{"state": false}"#).unwrap();
        assert_eq!(off.state, SwitchState::Off);
    }

    #[test]
    fn test_switch_state_bool_conversion() {
        assert_eq!(SwitchState::On.to_bool(), true);
        assert_eq!(SwitchState::Off.to_bool(), false);
    }

    #[test]
    fn test_device_mqtt_message_has_sensor_data() {
        let msg = DeviceMqttMessage {
            linkquality: Some(100),
            battery: Some(80),
            temperature: Some(22.5),
            humidity: Some(50.0),
            occupancy: None,
            state: None,
            other: Default::default(),
        };

        assert!(msg.has_sensor_data(&SensorType::TempHumidity));
        assert!(!msg.has_sensor_data(&SensorType::Presence));
    }

    #[test]
    fn test_extract_device_state_returns_struct_with_fields() {
        let msg = DeviceMqttMessage {
            linkquality: Some(10),
            battery: Some(90),
            temperature: None,
            humidity: None,
            occupancy: None,
            state: None,
            other: Default::default(),
        };

        let state = msg.extract_device_state();
        assert_eq!(state.battery_level, Some(90));
        assert_eq!(state.link_quality, Some(10));
    }

    #[test]
    fn test_device_mqtt_message_has_commander_data() {
        let msg = DeviceMqttMessage {
            linkquality: Some(100),
            battery: None,
            temperature: None,
            humidity: None,
            occupancy: None,
            state: Some(SwitchState::On),
            other: Default::default(),
        };

        assert!(msg.has_commander_data(&CommanderType::Switch));
    }

    #[test]
    fn test_sensor_reading_from_mqtt() {
        let msg = DeviceMqttMessage {
            linkquality: Some(100),
            battery: Some(80),
            temperature: Some(22.5),
            humidity: Some(50.0),
            occupancy: None,
            state: None,
            other: Default::default(),
        };

        let reading =
            SensorReading::from_mqtt("device1".to_string(), &SensorType::TempHumidity, &msg);
        assert!(reading.is_some());

        if let Some(SensorReading::TempHumidity {
            temperature,
            humidity,
            ..
        }) = reading
        {
            assert_eq!(temperature, 22.5);
            assert_eq!(humidity, 50.0);
        } else {
            panic!("Expected TempHumidity reading");
        }
    }
}

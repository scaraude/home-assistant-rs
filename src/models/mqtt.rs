use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    /// Switch state (ON/OFF as string)
    pub state: Option<String>,

    // ==================== Catch-all ====================
    /// Other fields we don't use yet
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl DeviceMqttMessage {
    /// Extract common device state fields (battery, link quality)
    pub fn extract_device_state(&self) -> (Option<u8>, Option<u8>) {
        (self.linkquality, self.battery)
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

/// Switch state enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
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

    /// Convert to boolean (ON = true, OFF = false)
    pub fn to_bool(&self) -> bool {
        matches!(self, SwitchState::On)
    }

    /// Convert from boolean (true = ON, false = OFF)
    pub fn from_bool(value: bool) -> Self {
        if value {
            SwitchState::On
        } else {
            SwitchState::Off
        }
    }
}

/// Sensor reading - time-series data stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Commander state - current state stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CommanderState {
    Switch {
        device_id: String,
        state: SwitchState,
        #[serde(with = "chrono::serde::ts_seconds")]
        last_updated: DateTime<Utc>,
    },
}

impl CommanderState {
    /// Create a commander state from MQTT message
    pub fn from_mqtt(
        device_id: String,
        commander_type: &CommanderType,
        msg: &DeviceMqttMessage,
    ) -> Option<Self> {
        match commander_type {
            CommanderType::Switch => msg
                .state
                .as_ref()
                .and_then(|s| SwitchState::from_mqtt_string(s))
                .map(|state| CommanderState::Switch {
                    device_id,
                    state,
                    last_updated: Utc::now(),
                }),
        }
    }

    /// Update state from MQTT message
    pub fn update_from_mqtt(&mut self, msg: &DeviceMqttMessage) {
        match self {
            CommanderState::Switch {
                state,
                last_updated,
                ..
            } => {
                if let Some(state_str) = &msg.state {
                    if let Some(new_state) = SwitchState::from_mqtt_string(state_str) {
                        *state = new_state;
                        *last_updated = Utc::now();
                    }
                }
            }
        }
    }

    /// Get the device ID for this state
    pub fn device_id(&self) -> &str {
        match self {
            CommanderState::Switch { device_id, .. } => device_id,
        }
    }
}

/// Command to send to a switch device
#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchCommand {
    /// Device ID to control
    pub device_id: String,

    /// Desired state (true = ON, false = OFF)
    pub state: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_state_from_mqtt_string() {
        assert_eq!(SwitchState::from_mqtt_string("ON"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("on"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("OFF"), Some(SwitchState::Off));
        assert_eq!(SwitchState::from_mqtt_string("off"), Some(SwitchState::Off));
        assert_eq!(SwitchState::from_mqtt_string("invalid"), None);
    }

    #[test]
    fn test_switch_state_bool_conversion() {
        assert_eq!(SwitchState::On.to_bool(), true);
        assert_eq!(SwitchState::Off.to_bool(), false);
        assert_eq!(SwitchState::from_bool(true), SwitchState::On);
        assert_eq!(SwitchState::from_bool(false), SwitchState::Off);
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
    fn test_device_mqtt_message_has_commander_data() {
        let msg = DeviceMqttMessage {
            linkquality: Some(100),
            battery: None,
            temperature: None,
            humidity: None,
            occupancy: None,
            state: Some("ON".to_string()),
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

    #[test]
    fn test_commander_state_from_mqtt() {
        let msg = DeviceMqttMessage {
            linkquality: Some(100),
            battery: None,
            temperature: None,
            humidity: None,
            occupancy: None,
            state: Some("ON".to_string()),
            other: Default::default(),
        };

        let state = CommanderState::from_mqtt("device1".to_string(), &CommanderType::Switch, &msg);
        assert!(state.is_some());

        if let Some(CommanderState::Switch { state, .. }) = state {
            assert_eq!(state, SwitchState::On);
        } else {
            panic!("Expected Switch state");
        }
    }
}

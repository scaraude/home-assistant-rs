use chrono::{DateTime, Utc};
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::{CommanderType, DeviceCapability, SensorType};

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

    /// Illumination level (only for presence sensors)
    pub illumination: Option<String>,

    // ==================== Energy meter fields ====================
    /// Power consumption in Watts
    pub power: Option<f32>,

    /// Voltage in Volts
    pub voltage: Option<f32>,

    /// Current in Amperes
    pub current: Option<f32>,

    /// AC frequency in Hertz
    pub ac_frequency: Option<f32>,

    /// Power factor (0-1)
    pub power_factor: Option<f32>,

    /// Total consumed energy in kWh
    pub energy: Option<f32>,

    /// Total produced energy in kWh
    pub produced_energy: Option<f32>,

    // ==================== Commander fields ====================
    /// Switch state parsed from MQTT payload
    pub state: Option<SwitchState>,

    /// Turbo mode flag (if supported by device)
    pub turbo_mode: Option<bool>,

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
            turbo_mode: self.turbo_mode,
        }
    }

    /// Check if this message contains sensor data for the given sensor type
    pub fn has_sensor_data(&self, sensor_type: &SensorType) -> bool {
        match sensor_type {
            SensorType::TempHumidity => self.temperature.is_some(),
            SensorType::Presence => self.occupancy.is_some(),
            SensorType::EnergyMeter => self.power.is_some(),
        }
    }

    /// Check if this message contains commander data for the given commander type
    pub fn has_commander_data(&self, commander_type: &CommanderType) -> bool {
        match commander_type {
            CommanderType::Switch => self.state.is_some(),
        }
    }

    /// Heuristic for switch devices that publish config-only payloads before state changes.
    pub fn has_switch_config_hint(&self) -> bool {
        self.other.keys().any(|key| {
            matches!(
                key.as_str(),
                "inching_control"
                    | "inching_time"
                    | "inching_mode"
                    | "inching_control_set"
                    | "delayed_power_on_state"
                    | "delayed_power_on_time"
                    | "detach_relay_mode"
                    | "external_trigger_mode"
            )
        })
    }

    /// Detect all capabilities advertised by this message
    pub fn detect_capabilities(&self) -> Vec<DeviceCapability> {
        let mut capabilities = Vec::new();

        for sensor_type in [
            SensorType::TempHumidity,
            SensorType::Presence,
            SensorType::EnergyMeter,
        ] {
            if self.has_sensor_data(&sensor_type) {
                capabilities.push(DeviceCapability::Sensor { sensor_type });
            }
        }

        if self.has_commander_data(&CommanderType::Switch) || self.has_switch_config_hint() {
            capabilities.push(DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            });
        }

        capabilities
    }

    /// Collect available fields present in this payload
    pub fn available_fields(&self) -> Vec<String> {
        let mut fields = Vec::new();

        if self.linkquality.is_some() {
            fields.push("linkquality".to_string());
        }
        if self.battery.is_some() {
            fields.push("battery".to_string());
        }
        if self.temperature.is_some() {
            fields.push("temperature".to_string());
        }
        if self.humidity.is_some() {
            fields.push("humidity".to_string());
        }
        if self.occupancy.is_some() {
            fields.push("occupancy".to_string());
        }
        if self.illumination.is_some() {
            fields.push("illumination".to_string());
        }
        if self.power.is_some() {
            fields.push("power".to_string());
        }
        if self.voltage.is_some() {
            fields.push("voltage".to_string());
        }
        if self.current.is_some() {
            fields.push("current".to_string());
        }
        if self.ac_frequency.is_some() {
            fields.push("ac_frequency".to_string());
        }
        if self.power_factor.is_some() {
            fields.push("power_factor".to_string());
        }
        if self.energy.is_some() {
            fields.push("energy".to_string());
        }
        if self.produced_energy.is_some() {
            fields.push("produced_energy".to_string());
        }
        if self.state.is_some() {
            fields.push("state".to_string());
        }
        if self.turbo_mode.is_some() {
            fields.push("turbo_mode".to_string());
        }

        for (key, value) in &self.other {
            if !value.is_null() {
                fields.push(key.clone());
            }
        }

        fields.sort_unstable();
        fields.dedup();
        fields
    }
}

/// Strongly typed container for device state metrics extracted from MQTT payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceStateFields {
    pub battery_level: Option<u8>,
    pub link_quality: Option<u8>,
    pub turbo_mode: Option<bool>,
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
        illumination: Option<String>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "energy_meter")]
    EnergyMeter {
        device_id: String,
        power: f32,           // Primary metric (Watts)
        energy: f32,          // kWh consumed
        produced_energy: f32, // kWh produced
        voltage: f32,         // Volts
        current: f32,         // Amperes
        ac_frequency: f32,    // Hertz
        power_factor: f32,    // 0-1
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
                illumination: msg.illumination.clone(),
                timestamp: Utc::now(),
            }),
            SensorType::EnergyMeter => msg.power.map(|pwr| SensorReading::EnergyMeter {
                device_id,
                power: pwr,
                energy: msg.energy.unwrap_or(0.0),
                produced_energy: msg.produced_energy.unwrap_or(0.0),
                voltage: msg.voltage.unwrap_or(0.0),
                current: msg.current.unwrap_or(0.0),
                ac_frequency: msg.ac_frequency.unwrap_or(0.0),
                power_factor: msg.power_factor.unwrap_or(0.0),
                timestamp: Utc::now(),
            }),
        }
    }

    /// Get the device ID for this reading
    pub fn device_id(&self) -> &str {
        match self {
            SensorReading::TempHumidity { device_id, .. } => device_id,
            SensorReading::Presence { device_id, .. } => device_id,
            SensorReading::EnergyMeter { device_id, .. } => device_id,
        }
    }

    /// Get the timestamp for this reading
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SensorReading::TempHumidity { timestamp, .. } => *timestamp,
            SensorReading::Presence { timestamp, .. } => *timestamp,
            SensorReading::EnergyMeter { timestamp, .. } => *timestamp,
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

    fn create_test_mqtt_message(
        linkality: Option<u8>,
        battery: Option<u8>,
        temperature: Option<f32>,
        humidity: Option<f32>,
        occupancy: Option<bool>,
        illumination: Option<String>,
        state: Option<SwitchState>,
        turbo_mode: Option<bool>,
    ) -> DeviceMqttMessage {
        DeviceMqttMessage {
            linkquality: linkality,
            battery,
            temperature,
            humidity,
            occupancy,
            illumination,
            power: None,
            voltage: None,
            current: None,
            ac_frequency: None,
            power_factor: None,
            energy: None,
            produced_energy: None,
            state,
            turbo_mode,
            other: serde_json::Map::new(),
        }
    }

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
        let temp_msg =
            create_test_mqtt_message(None, None, Some(22.5), Some(55.0), None, None, None, None);
        assert!(temp_msg.has_sensor_data(&SensorType::TempHumidity));
        assert!(!temp_msg.has_sensor_data(&SensorType::Presence));

        let presence_msg = create_test_mqtt_message(
            None,
            None,
            None,
            None,
            Some(true),
            Some("bright".to_string()),
            None,
            None,
        );
        assert!(!presence_msg.has_sensor_data(&SensorType::TempHumidity));
        assert!(presence_msg.has_sensor_data(&SensorType::Presence));
    }

    #[test]
    fn test_extract_device_state_returns_struct_with_fields() {
        let msg = create_test_mqtt_message(Some(10), Some(90), None, None, None, None, None, None);

        let state = msg.extract_device_state();
        assert_eq!(state.battery_level, Some(90));
        assert_eq!(state.link_quality, Some(10));
    }

    #[test]
    fn test_device_mqtt_message_has_commander_data() {
        let msg = create_test_mqtt_message(
            None,
            None,
            None,
            None,
            None,
            None,
            Some(SwitchState::On),
            None,
        );

        assert!(msg.has_commander_data(&CommanderType::Switch));
    }

    #[test]
    fn test_sensor_reading_from_mqtt() {
        let msg =
            create_test_mqtt_message(None, None, Some(22.5), Some(50.0), None, None, None, None);

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

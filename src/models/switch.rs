use serde::{Deserialize, Serialize};

/// Zigbee2MQTT message format for switch devices
/// Extends the temperature message to include state field
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SwitchMqttMessage {
    /// Switch state (ON/OFF)
    pub state: Option<String>,

    /// Link quality indicator
    pub linkquality: Option<u8>,

    /// Other fields that might be present but we don't use
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

/// Command to send to a switch device
#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchCommand {
    /// Device ID to control
    pub device_id: String,

    /// Desired state (true = ON, false = OFF)
    pub state: bool,
}

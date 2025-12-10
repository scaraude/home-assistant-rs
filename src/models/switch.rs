use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Switch device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchDevice {
    /// Device IEEE address (e.g., "0x7cc6b6fffec90892")
    pub id: String,

    /// Friendly name for display
    pub name: String,

    /// Current state (ON/OFF)
    pub state: bool,

    /// Optional link quality indicator
    pub link_quality: Option<u8>,

    /// Last time state was updated
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_updated: DateTime<Utc>,
}

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

impl SwitchDevice {
    /// Create a new switch device from MQTT message
    pub fn from_mqtt(device_id: String, msg: SwitchMqttMessage) -> Option<Self> {
        debug!(
            device_id = %device_id,
            state = ?msg.state,
            linkquality = ?msg.linkquality,
            "Attempting to create SwitchDevice from MQTT message"
        );

        // Only create if we have state data
        if let Some(state_str) = msg.state {
            let state = state_str.to_uppercase() == "ON";
            debug!(
                device_id = %device_id,
                state = %state,
                linkquality = ?msg.linkquality,
                "Successfully created SwitchDevice"
            );
            Some(Self {
                id: device_id.clone(),
                name: device_id, // Default to ID, can be customized later
                state,
                link_quality: msg.linkquality,
                last_updated: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Update device state from MQTT message
    pub fn update_from_mqtt(&mut self, msg: SwitchMqttMessage) {
        if let Some(state_str) = msg.state {
            self.state = state_str.to_uppercase() == "ON";
            self.last_updated = Utc::now();
        }
        if let Some(lq) = msg.linkquality {
            self.link_quality = Some(lq);
        }
        debug!(
            device_id = %self.id,
            state = %self.state,
            linkquality = ?self.link_quality,
            "Updated SwitchDevice from MQTT"
        );
    }
}

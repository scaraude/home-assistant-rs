use crate::db::Database;
use crate::models::{Device, DeviceType, PowerSource, SwitchMqttMessage, TempSensorMqttMessage};
use tracing::{debug, error, info};

/// MQTT message types for device discovery
pub enum MqttMessage {
    TempHumiditySensor(TempSensorMqttMessage),
    Switch(SwitchMqttMessage),
}

/// Helper function to get or create a device from MQTT topic
/// Returns the device UUID if successful
pub async fn get_or_create_device(
    db: &Database,
    mqtt_topic: &str,
    msg: MqttMessage,
) -> Option<String> {
    // Try to get existing device
    match db.get_device_by_mqtt_topic(mqtt_topic) {
        Ok(Some(device)) => {
            debug!(
                device_id = %device.id,
                mqtt_topic = %mqtt_topic,
                "Found existing device"
            );
            return Some(device.id);
        }
        Ok(None) => {
            // Device doesn't exist, create it
            info!(
                mqtt_topic = %mqtt_topic,
                "New device detected, creating entry"
            );

            // Determine device type based on available fields
            let device_type = match msg {
                MqttMessage::TempHumiditySensor(_) => DeviceType::TempHumiditySensor,
                MqttMessage::Switch(_) => DeviceType::Commander,
            };

            // Determine power source (if battery field exists, assume battery powered)
            let power_source: PowerSource = match msg {
                MqttMessage::TempHumiditySensor(_) => PowerSource::Battery,
                MqttMessage::Switch(switch_msg) => {
                    if switch_msg.other.get("battery").is_some() {
                        PowerSource::Battery
                    } else {
                        PowerSource::Plugged
                    }
                }
            };

            let device = Device::new(
                mqtt_topic.to_string(),
                mqtt_topic.to_string(), // Use MQTT topic as default name
                device_type,
                power_source,
            );

            match db.insert_device(&device) {
                Ok(_) => {
                    info!(
                        device_id = %device.id,
                        mqtt_topic = %mqtt_topic,
                        device_type = ?device.device_type,
                        "Successfully created new device"
                    );
                    Some(device.id)
                }
                Err(e) => {
                    error!(
                        error = %e,
                        mqtt_topic = %mqtt_topic,
                        "Failed to insert new device"
                    );
                    None
                }
            }
        }
        Err(e) => {
            error!(
                error = %e,
                mqtt_topic = %mqtt_topic,
                "Failed to query device"
            );
            None
        }
    }
}

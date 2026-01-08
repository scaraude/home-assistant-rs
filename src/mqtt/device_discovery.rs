use crate::db::Database;
use crate::models::{
    CommanderType, Device, DeviceCapability, DeviceMqttMessage, PowerSource, SensorType,
};
use tracing::{debug, error, info};

/// Helper function to get or create a device from unified MQTT message
/// Returns the device UUID if successful
pub async fn get_or_create_device_unified(
    db: &Database,
    mqtt_topic: &str,
    msg: &DeviceMqttMessage,
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

            // Determine device capability based on available fields
            let capability = if msg.has_sensor_data(&SensorType::TempHumidity) {
                DeviceCapability::Sensor {
                    sensor_type: SensorType::TempHumidity,
                }
            } else if msg.has_sensor_data(&SensorType::Presence) {
                DeviceCapability::Sensor {
                    sensor_type: SensorType::Presence,
                }
            } else if msg.has_commander_data(&CommanderType::Switch) || msg.has_switch_config_hint()
            {
                DeviceCapability::Commander {
                    commander_type: CommanderType::Switch,
                }
            } else {
                error!(
                    mqtt_topic = %mqtt_topic,
                    "Cannot determine device type from MQTT message"
                );
                return None;
            };

            // Determine power source (if battery field exists, assume battery powered)
            let power_source = if msg.battery.is_some() {
                PowerSource::Battery
            } else {
                PowerSource::Plugged
            };

            let device = Device::new(
                mqtt_topic.to_string(),
                mqtt_topic.to_string(), // Use MQTT topic as default name
                capability,
                power_source,
            );

            match db.insert_device(&device) {
                Ok(_) => {
                    info!(
                        device_id = %device.id,
                        mqtt_topic = %mqtt_topic,
                        capability = ?device.capability,
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

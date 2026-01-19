use crate::db::Database;
use crate::models::{Device, DeviceMqttMessage, PowerSource};
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
            let detected_capabilities = msg.detect_capabilities();
            let detected_fields = msg.available_fields();
            if !detected_capabilities.is_empty() || !detected_fields.is_empty() {
                let mut merged_capabilities = device.capabilities.clone();
                for capability in detected_capabilities {
                    if !merged_capabilities.contains(&capability) {
                        merged_capabilities.push(capability);
                    }
                }

                let mut merged_fields = device.available_fields.clone();
                for field in detected_fields {
                    if !merged_fields.contains(&field) {
                        merged_fields.push(field);
                    }
                }
                merged_fields.sort_unstable();
                merged_fields.dedup();

                if merged_capabilities != device.capabilities
                    || merged_fields != device.available_fields
                {
                    if let Err(e) =
                        db.update_device_metadata(&device.id, &merged_capabilities, &merged_fields)
                    {
                        error!(
                            error = %e,
                            device_id = %device.id,
                            "Failed to update device metadata"
                        );
                    }
                }
            }
            return Some(device.id);
        }
        Ok(None) => {
            // Device doesn't exist, create it
            info!(
                mqtt_topic = %mqtt_topic,
                "New device detected, creating entry"
            );

            // Determine device capabilities based on available fields
            let capabilities = msg.detect_capabilities();
            if capabilities.is_empty() {
                error!(
                    mqtt_topic = %mqtt_topic,
                    "Cannot determine device capabilities from MQTT message"
                );
                return None;
            }

            // Determine power source (if battery field exists, assume battery powered)
            let power_source = if msg.battery.is_some() {
                PowerSource::Battery
            } else {
                PowerSource::Plugged
            };

            let device = Device::new(
                mqtt_topic.to_string(),
                mqtt_topic.to_string(), // Use MQTT topic as default name
                capabilities,
                msg.available_fields(),
                power_source,
            );

            match db.insert_device(&device) {
                Ok(_) => {
                    info!(
                        device_id = %device.id,
                        mqtt_topic = %mqtt_topic,
                        capabilities = ?device.capabilities,
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

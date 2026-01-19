use crate::db::Database;
use crate::events::{SystemEvent, bus::EventBus};
use crate::models::{
    CommanderType, DeviceCapability, DeviceMqttMessage, SensorReading, SensorType,
};
use crate::mqtt::device_discovery::get_or_create_device_unified;
use chrono::Utc;
use tracing::{debug, error, info};

/// Handle a unified MQTT device message
/// Routes to appropriate handler based on device capability
pub async fn handle_device_message(
    db: &Database,
    event_bus: &EventBus,
    mqtt_topic: &str,
    msg: DeviceMqttMessage,
    no_temperature: &mut u64,
    publish_failures: &mut u64,
) {
    debug!(
        mqtt_topic = %mqtt_topic,
        has_temperature = msg.temperature.is_some(),
        has_state = msg.state.is_some(),
        has_occupancy = msg.occupancy.is_some(),
        linkquality = ?msg.linkquality,
        battery = ?msg.battery,
        turbo_mode = ?msg.turbo_mode,
        "Received unified MQTT message"
    );

    // Get or create device
    let device_id = match get_or_create_device_unified(db, mqtt_topic, &msg).await {
        Some(id) => id,
        None => {
            error!(
                mqtt_topic = %mqtt_topic,
                "Failed to get or create device, skipping message"
            );
            return;
        }
    };

    let device_state_fields = msg.extract_device_state();
    // Publish device state event so downstream services can update stores/DB
    publish_device_state_event(
        event_bus,
        &device_id,
        device_state_fields.battery_level,
        device_state_fields.link_quality,
        device_state_fields.turbo_mode,
        publish_failures,
    );

    // Get device to determine capability
    let device = match db.get_device_by_mqtt_topic(mqtt_topic) {
        Ok(Some(d)) => d,
        Ok(None) => {
            error!(
                mqtt_topic = %mqtt_topic,
                "Device not found after creation - this should not happen"
            );
            return;
        }
        Err(e) => {
            error!(
                error = %e,
                mqtt_topic = %mqtt_topic,
                "Failed to query device from database"
            );
            return;
        }
    };

    // Route based on device capability
    match &device.capability {
        DeviceCapability::Sensor { sensor_type } => {
            handle_sensor_message(
                sensor_type,
                &device_id,
                &msg,
                event_bus,
                no_temperature,
                publish_failures,
            )
            .await;
        }
        DeviceCapability::Commander { commander_type } => {
            handle_commander_message(
                commander_type,
                &device_id,
                &msg,
                event_bus,
                publish_failures,
            )
            .await;
        }
        _ => {
            debug!(
                device_id = %device_id,
                capability = ?device.capability,
                "Device capability not handled, skipping"
            );
        }
    }
}

/// Handle sensor-specific MQTT messages
async fn handle_sensor_message(
    sensor_type: &SensorType,
    device_id: &str,
    msg: &DeviceMqttMessage,
    event_bus: &EventBus,
    no_temperature: &mut u64,
    publish_failures: &mut u64,
) {
    // Create sensor reading based on type
    let sensor_reading = SensorReading::from_mqtt(device_id.to_string(), sensor_type, msg);

    match sensor_reading {
        Some(reading) => {
            match &reading {
                SensorReading::TempHumidity {
                    temperature,
                    humidity,
                    ..
                } => {
                    info!(
                        device_id = %device_id,
                        temperature = %temperature,
                        humidity = %humidity,
                        "Created temperature/humidity reading from MQTT message"
                    );
                }
                SensorReading::Presence { occupied, .. } => {
                    info!(
                        device_id = %device_id,
                        occupied = %occupied,
                        "Created presence reading from MQTT message"
                    );
                }
                SensorReading::EnergyMeter { power, energy, .. } => {
                    info!(
                        device_id = %device_id,
                        power = %power,
                        energy = %energy,
                        "Created energy meter reading from MQTT message"
                    );
                }
            }

            let event = SystemEvent::SensorReading {
                device_id: device_id.to_string(),
                timestamp: reading.timestamp(),
                reading,
            };
            if let Err(e) = event_bus.publish(event) {
                *publish_failures += 1;
                error!(
                    error = %e,
                    device_id = %device_id,
                    publish_failures = *publish_failures,
                    "Failed to publish sensor reading event"
                );
            } else {
                debug!(device_id = %device_id, "Published sensor reading event");
            }
        }
        None => {
            *no_temperature += 1;
            debug!(
                device_id = %device_id,
                sensor_type = ?sensor_type,
                no_data_count = *no_temperature,
                "Message does not contain expected sensor data"
            );
        }
    }
}

/// Handle commander-specific MQTT messages
async fn handle_commander_message(
    commander_type: &CommanderType,
    device_id: &str,
    msg: &DeviceMqttMessage,
    event_bus: &EventBus,
    publish_failures: &mut u64,
) {
    match commander_type {
        CommanderType::Switch => {
            if let Some(state) = msg.state {
                info!(
                    device_id = %device_id,
                    state = %state,
                    linkquality = ?msg.linkquality,
                    "Received switch state update"
                );

                let event = SystemEvent::SwitchState {
                    device_id: device_id.to_string(),
                    state,
                    timestamp: Utc::now(),
                };

                if let Err(e) = event_bus.publish(event) {
                    *publish_failures += 1;
                    error!(
                        error = %e,
                        device_id = %device_id,
                        publish_failures = *publish_failures,
                        "Failed to publish switch state event"
                    );
                } else {
                    debug!(
                        device_id = %device_id,
                        state = %state,
                        "Published switch state event"
                    );
                }
            }
        }
    }
}

fn publish_device_state_event(
    event_bus: &EventBus,
    device_id: &str,
    battery: Option<u8>,
    link_quality: Option<u8>,
    turbo_mode: Option<bool>,
    publish_failures: &mut u64,
) {
    let event = SystemEvent::DeviceState {
        device_id: device_id.to_string(),
        battery,
        link_quality,
        turbo_mode,
        timestamp: Utc::now(),
    };

    if let Err(e) = event_bus.publish(event) {
        *publish_failures += 1;
        error!(
            error = %e,
            device_id = %device_id,
            publish_failures = *publish_failures,
            "Failed to publish device state event"
        );
    } else {
        debug!(
            device_id = %device_id,
            battery = ?battery,
            link_quality = ?link_quality,
            turbo_mode = ?turbo_mode,
            "Published device state event"
        );
    }
}

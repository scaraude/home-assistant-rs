use crate::db::Database;
use crate::models::{
    CommanderType, DeviceCapability, DeviceMqttMessage, SensorReading, SensorType,
};
use crate::mqtt::device_discovery::get_or_create_device_unified;
use crate::state::{DeviceStateStore, SwitchStateStore};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Handle a unified MQTT device message
/// Routes to appropriate handler based on device capability
pub async fn handle_device_message(
    db: &Database,
    device_state: &DeviceStateStore,
    switch_state: &SwitchStateStore,
    tx: &mpsc::Sender<SensorReading>,
    mqtt_topic: &str,
    msg: DeviceMqttMessage,
    no_temperature: &mut u64,
    send_failures: &mut u64,
) {
    debug!(
        mqtt_topic = %mqtt_topic,
        has_temperature = msg.temperature.is_some(),
        has_state = msg.state.is_some(),
        has_occupancy = msg.occupancy.is_some(),
        linkquality = ?msg.linkquality,
        battery = ?msg.battery,
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

    // Update common device state (link quality, battery, last seen)
    // Update in-memory state
    if let Some(lq) = msg.linkquality {
        device_state.update_link_quality(device_id.clone(), lq);
        // Persist to database
        if let Err(e) = db.update_device_link_quality(&device_id, lq) {
            error!(error = %e, device_id = %device_id, "Failed to persist link quality to database");
        }
    }
    if let Some(battery) = msg.battery {
        device_state.update_battery(device_id.clone(), battery);
        // Persist to database
        if let Err(e) = db.update_device_battery(&device_id, battery) {
            error!(error = %e, device_id = %device_id, "Failed to persist battery level to database");
        }
    }
    device_state.mark_seen(device_id.clone());
    // Persist last_seen to database
    if let Err(e) = db.update_device_last_seen(&device_id) {
        error!(error = %e, device_id = %device_id, "Failed to persist last_seen to database");
    }

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
                tx,
                no_temperature,
                send_failures,
            )
            .await;
        }
        DeviceCapability::Commander { commander_type } => {
            handle_commander_message(commander_type, &device_id, &msg, switch_state).await;
        }
    }
}

/// Handle sensor-specific MQTT messages
async fn handle_sensor_message(
    sensor_type: &SensorType,
    device_id: &str,
    msg: &DeviceMqttMessage,
    tx: &mpsc::Sender<SensorReading>,
    no_temperature: &mut u64,
    send_failures: &mut u64,
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
            }

            // Send to channel
            match tx.send(reading).await {
                Ok(_) => {
                    debug!(device_id = %device_id, "Sent reading to channel");
                }
                Err(e) => {
                    *send_failures += 1;
                    error!(
                        error = %e,
                        device_id = %device_id,
                        send_failures = *send_failures,
                        "Failed to send reading to channel - receiver dropped?"
                    );
                }
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
    switch_state: &SwitchStateStore,
) {
    match commander_type {
        CommanderType::Switch => {
            if let Some(state_str) = &msg.state {
                let state = state_str.to_uppercase() == "ON";
                info!(
                    device_id = %device_id,
                    state = %state,
                    linkquality = ?msg.linkquality,
                    "Received switch state update"
                );

                // Update switch state store
                switch_state.set_state(device_id.to_string(), state);
                debug!(
                    device_id = %device_id,
                    state = %state,
                    "Updated switch state in store"
                );
            }
        }
    }
}

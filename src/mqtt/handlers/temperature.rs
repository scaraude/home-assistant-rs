use crate::db::Database;
use crate::models::{TempSensorMqttMessage, TemperatureReading};
use crate::mqtt::device_discovery::{MqttMessage, get_or_create_device};
use crate::state::DeviceStateStore;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub async fn handle_temperature_message(
    db: &Database,
    device_state: &DeviceStateStore,
    tx: &mpsc::Sender<TemperatureReading>,
    mqtt_topic: &str,
    msg: TempSensorMqttMessage,
    no_temperature: &mut u64,
    send_failures: &mut u64,
) {
    debug!(
        mqtt_topic = %mqtt_topic,
        message = ?msg,
        "Successfully parsed MQTT message"
    );

    // Get or create device
    if let Some(device_id) =
        get_or_create_device(db, mqtt_topic, MqttMessage::TempHumiditySensor(msg.clone())).await
    {
        // Update device state with link quality and battery
        if let Some(lq) = msg.linkquality {
            device_state.update_link_quality(device_id.clone(), lq);
        }
        if let Some(battery) = msg.battery {
            device_state.update_battery(device_id.clone(), battery);
        }
        device_state.mark_seen(device_id.clone());

        // Create temperature reading if the message contains temperature data
        match TemperatureReading::from_mqtt(device_id.clone(), msg) {
            Some(reading) => {
                info!(
                    device_id = %reading.device_id,
                    temperature = %reading.temperature,
                    humidity = ?reading.humidity,
                    "Created temperature reading from MQTT message"
                );

                // Send to channel (drop if full to avoid blocking)
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
                    no_temp_count = *no_temperature,
                    "Message does not contain temperature data"
                );
            }
        }
    } else {
        error!(
            mqtt_topic = %mqtt_topic,
            "Failed to get or create device, skipping message"
        );
    }
}

pub fn handle_temperature_parse_error(
    mqtt_topic: &str,
    topic: &str,
    payload: &[u8],
    error: &serde_json::Error,
    parse_errors: &mut u64,
) {
    *parse_errors += 1;
    warn!(
        error = %error,
        mqtt_topic = %mqtt_topic,
        topic = %topic,
        payload_preview = ?String::from_utf8_lossy(&payload[..payload.len().min(100)]),
        parse_error_count = *parse_errors,
        "Failed to parse MQTT message as Zigbee2MqttMessage"
    );
}

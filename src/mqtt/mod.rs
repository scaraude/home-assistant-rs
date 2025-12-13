use crate::db::Database;
use crate::device_state::DeviceStateStore;
use crate::models::{
    Device, DeviceType, PowerSource, SwitchMqttMessage, TempSensorMqttMessage, TemperatureReading,
};
use crate::switch_state::SwitchStateStore;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Unified MQTT client for both publishing commands and listening to messages.
/// Handles all MQTT operations including subscribing to topics and publishing commands.
#[derive(Clone)]
pub struct MqttClient {
    client: AsyncClient,
}

enum MqttMessage {
    TempHumiditySensor(TempSensorMqttMessage),
    Switch(SwitchMqttMessage),
}

impl MqttClient {
    /// Create a new MQTT client and start listening to messages
    pub fn new(
        broker_host: &str,
        broker_port: u16,
        client_id: &str,
        db: Arc<Database>,
        device_state: DeviceStateStore,
        switch_state: SwitchStateStore,
    ) -> (Self, mpsc::Receiver<TemperatureReading>) {
        info!(
            broker = %broker_host,
            port = broker_port,
            client_id = %client_id,
            "Creating MQTT client"
        );

        let mut mqtt_options = MqttOptions::new(client_id, broker_host, broker_port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));
        mqtt_options.set_max_packet_size(1024 * 1024, 1024 * 1024); // 1MB max packet size

        debug!(
            keep_alive_secs = 60,
            max_packet_size = "1MB",
            "MQTT options configured"
        );

        let (async_client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
        let (tx, rx) = mpsc::channel(100);

        info!(
            queue_size = 10,
            channel_capacity = 100,
            "MQTT client and channels created"
        );

        // Spawn the event loop handler
        tokio::spawn(async move {
            info!("MQTT event loop started");
            let mut message_count = 0u64;
            let mut error_count = 0u64;
            let mut skipped_bridge = 0u64;
            let mut parse_errors = 0u64;
            let mut no_temperature = 0u64;
            let mut send_failures = 0u64;

            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(p))) => {
                        message_count += 1;
                        debug!(
                            topic = %p.topic,
                            payload_size = p.payload.len(),
                            qos = ?p.qos,
                            message_count = message_count,
                            "Received MQTT message"
                        );

                        // Extract MQTT identifier from topic: zigbee2mqtt/<mqtt_id>
                        if let Some(mqtt_topic) = p.topic.strip_prefix("zigbee2mqtt/") {
                            // Skip bridge topics
                            if mqtt_topic.starts_with("bridge/") {
                                skipped_bridge += 1;
                                debug!(
                                    topic = %p.topic,
                                    skipped_count = skipped_bridge,
                                    "Skipping bridge topic"
                                );
                                continue;
                            }

                            // Skip /set topics (command topics, not device state)
                            if mqtt_topic.ends_with("/set") {
                                debug!(
                                    topic = %p.topic,
                                    "Skipping /set command topic"
                                );
                                continue;
                            }

                            // Try parsing as switch message first
                            if let Ok(switch_msg) =
                                serde_json::from_slice::<SwitchMqttMessage>(&p.payload)
                            {
                                if let Some(device_id) = get_or_create_device(
                                    &db,
                                    mqtt_topic,
                                    MqttMessage::Switch(switch_msg.clone()),
                                )
                                .await
                                {
                                    if let Some(state_str) = &switch_msg.state {
                                        let state = state_str.to_uppercase() == "ON";
                                        info!(
                                            device_id = %device_id,
                                            state = %state,
                                            linkquality = ?switch_msg.linkquality,
                                            "Received switch state update"
                                        );

                                        // Update device state with link quality
                                        if let Some(lq) = switch_msg.linkquality {
                                            device_state.update_link_quality(device_id.clone(), lq);
                                        }
                                        device_state.mark_seen(device_id.clone());

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

                            // Also try parsing as temperature message
                            match serde_json::from_slice::<TempSensorMqttMessage>(&p.payload) {
                                Ok(msg) => {
                                    debug!(
                                        mqtt_topic = %mqtt_topic,
                                        message = ?msg,
                                        "Successfully parsed MQTT message"
                                    );

                                    // Get or create device
                                    if let Some(device_id) = get_or_create_device(
                                        &db,
                                        mqtt_topic,
                                        MqttMessage::TempHumiditySensor(msg.clone()),
                                    )
                                    .await
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
                                        match TemperatureReading::from_mqtt(device_id.clone(), msg)
                                        {
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
                                                        send_failures += 1;
                                                        error!(
                                                            error = %e,
                                                            device_id = %device_id,
                                                            send_failures = send_failures,
                                                            "Failed to send reading to channel - receiver dropped?"
                                                        );
                                                    }
                                                }
                                            }
                                            None => {
                                                no_temperature += 1;
                                                debug!(
                                                    device_id = %device_id,
                                                    no_temp_count = no_temperature,
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
                                Err(e) => {
                                    parse_errors += 1;
                                    warn!(
                                        error = %e,
                                        mqtt_topic = %mqtt_topic,
                                        topic = %p.topic,
                                        payload_preview = ?String::from_utf8_lossy(&p.payload[..p.payload.len().min(100)]),
                                        parse_error_count = parse_errors,
                                        "Failed to parse MQTT message as Zigbee2MqttMessage"
                                    );
                                }
                            }
                        } else {
                            debug!(
                                topic = %p.topic,
                                "Ignoring message - not from zigbee2mqtt namespace"
                            );
                        }
                    }
                    Ok(Event::Incoming(packet)) => {
                        debug!(packet = ?packet, "Received non-publish MQTT packet");
                    }
                    Ok(Event::Outgoing(packet)) => {
                        debug!(packet = ?packet, "Sent MQTT packet");
                    }
                    Err(e) => {
                        error_count += 1;
                        error!(
                            error = %e,
                            error_count = error_count,
                            "MQTT connection error - will retry in 5 seconds"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }

                // Periodic statistics logging
                if message_count % 100 == 0 && message_count > 0 {
                    info!(
                        total_messages = message_count,
                        errors = error_count,
                        skipped_bridge = skipped_bridge,
                        parse_errors = parse_errors,
                        no_temperature = no_temperature,
                        send_failures = send_failures,
                        "MQTT event loop statistics"
                    );
                }
            }
        });

        (
            Self {
                client: async_client,
            },
            rx,
        )
    }

    /// Subscribe to zigbee2mqtt topics
    pub async fn subscribe_to_zigbee2mqtt(&self) -> Result<(), rumqttc::ClientError> {
        info!(
            topic = "zigbee2mqtt/#",
            qos = "AtMostOnce",
            "Subscribing to MQTT topics"
        );

        match self
            .client
            .subscribe("zigbee2mqtt/#", QoS::AtMostOnce)
            .await
        {
            Ok(_) => {
                info!("Successfully subscribed to zigbee2mqtt/# topics");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, topic = "zigbee2mqtt/#", "Failed to subscribe to MQTT topics");
                Err(e)
            }
        }
    }

    /// Publish a command to a device
    pub async fn publish_command(
        &self,
        device_id: &str,
        payload: &str,
    ) -> Result<(), rumqttc::ClientError> {
        let topic = format!("zigbee2mqtt/{}/set", device_id);

        info!(
            topic = %topic,
            payload = %payload,
            "Publishing MQTT command"
        );

        match self
            .client
            .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
        {
            Ok(_) => {
                info!(topic = %topic, "Successfully published command");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, topic = %topic, "Failed to publish command");
                Err(e)
            }
        }
    }
}

/// Helper function to get or create a device from MQTT topic
/// Returns the device UUID if successful
async fn get_or_create_device(db: &Database, mqtt_topic: &str, msg: MqttMessage) -> Option<String> {
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

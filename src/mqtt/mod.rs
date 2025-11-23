use crate::models::{TemperatureReading, Zigbee2MqttMessage};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct MqttListener {
    client: AsyncClient,
}

impl MqttListener {
    /// Create a new MQTT listener
    pub fn new(
        broker_host: &str,
        broker_port: u16,
        client_id: &str,
    ) -> (Self, mpsc::Receiver<TemperatureReading>) {
        info!(
            broker = %broker_host,
            port = broker_port,
            client_id = %client_id,
            "Creating MQTT listener"
        );

        let mut mqtt_options = MqttOptions::new(client_id, broker_host, broker_port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));

        debug!(keep_alive_secs = 60, "MQTT keep-alive configured");

        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
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

                        // Extract sensor ID from topic: zigbee2mqtt/<sensor_id>
                        if let Some(sensor_id) = p.topic.strip_prefix("zigbee2mqtt/") {
                            // Skip bridge topics
                            if sensor_id.starts_with("bridge/") {
                                skipped_bridge += 1;
                                debug!(
                                    topic = %p.topic,
                                    skipped_count = skipped_bridge,
                                    "Skipping bridge topic"
                                );
                                continue;
                            }

                            // Parse the message
                            match serde_json::from_slice::<Zigbee2MqttMessage>(&p.payload) {
                                Ok(msg) => {
                                    debug!(
                                        sensor_id = %sensor_id,
                                        message = ?msg,
                                        "Successfully parsed MQTT message"
                                    );

                                    // Create temperature reading if the message contains temperature data
                                    match TemperatureReading::from_mqtt(sensor_id.to_string(), msg)
                                    {
                                        Some(reading) => {
                                            info!(
                                                sensor_id = %reading.sensor_id,
                                                temperature = %reading.temperature,
                                                humidity = ?reading.humidity,
                                                battery = ?reading.battery,
                                                "Created temperature reading from MQTT message"
                                            );

                                            // Send to channel (drop if full to avoid blocking)
                                            match tx.send(reading).await {
                                                Ok(_) => {
                                                    debug!(sensor_id = %sensor_id, "Sent reading to channel");
                                                }
                                                Err(e) => {
                                                    send_failures += 1;
                                                    error!(
                                                        error = %e,
                                                        sensor_id = %sensor_id,
                                                        send_failures = send_failures,
                                                        "Failed to send reading to channel - receiver dropped?"
                                                    );
                                                }
                                            }
                                        }
                                        None => {
                                            no_temperature += 1;
                                            debug!(
                                                sensor_id = %sensor_id,
                                                no_temp_count = no_temperature,
                                                "Message does not contain temperature data"
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    parse_errors += 1;
                                    warn!(
                                        error = %e,
                                        sensor_id = %sensor_id,
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

        (Self { client }, rx)
    }

    /// Subscribe to zigbee2mqtt topics
    pub async fn subscribe(&self) -> Result<(), rumqttc::ClientError> {
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
}

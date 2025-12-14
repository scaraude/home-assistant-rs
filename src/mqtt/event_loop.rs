use crate::db::Database;
use crate::events::bus::EventBus;
use crate::models::DeviceMqttMessage;
use crate::mqtt::handlers::handle_device_message;
use rumqttc::{Event, EventLoop, Packet};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Spawn the MQTT event loop handler
pub(super) fn spawn_event_loop(
    mut eventloop: EventLoop,
    db: Arc<Database>,
    event_bus: EventBus,
) {
    tokio::spawn(async move {
        info!("MQTT event loop started");
        let mut message_count = 0u64;
        let mut error_count = 0u64;
        let mut skipped_bridge = 0u64;
        let mut parse_errors = 0u64;
        let mut no_temperature = 0u64;
        let mut publish_failures = 0u64;

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

                        // Parse as unified device message
                        match serde_json::from_slice::<DeviceMqttMessage>(&p.payload) {
                            Ok(msg) => {
                                handle_device_message(
                                    &db,
                                    &event_bus,
                                    mqtt_topic,
                                    msg,
                                    &mut no_temperature,
                                    &mut publish_failures,
                                )
                                .await;
                            }
                            Err(e) => {
                                parse_errors += 1;
                                warn!(
                                    error = %e,
                                    mqtt_topic = %mqtt_topic,
                                    topic = %p.topic,
                                    payload_preview = ?String::from_utf8_lossy(&p.payload[..p.payload.len().min(100)]),
                                    parse_error_count = parse_errors,
                                    "Failed to parse MQTT message"
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
                    publish_failures = publish_failures,
                    "MQTT event loop statistics"
                );
            }
        }
    });
}

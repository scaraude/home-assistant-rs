use crate::db::Database;
use crate::events::bus::EventBus;
use crate::models::DeviceMqttMessage;
use crate::mqtt::dedup_filter::MqttDedupFilter;
use crate::mqtt::handlers::{handle_bridge_event, handle_bridge_response, handle_device_message};
use crate::mqtt::topic::ZigbeeTopic;
use rumqttc::{AsyncClient, Event, EventLoop, Packet, QoS};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Spawn the MQTT event loop handler
pub(super) fn spawn_event_loop(
    mut eventloop: EventLoop,
    client: AsyncClient,
    db: Arc<Database>,
    event_bus: EventBus,
    subscriptions: Arc<Mutex<Vec<(String, QoS)>>>,
) {
    tokio::spawn(async move {
        info!("MQTT event loop started");
        let mut message_count = 0u64;
        let mut error_count = 0u64;
        let mut skipped_bridge = 0u64;
        let mut parse_errors = 0u64;
        let mut no_temperature = 0u64;
        let mut publish_failures = 0u64;
        let mut dedup_filter = MqttDedupFilter::new();

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                    info!(
                        clean_session = !connack.session_present,
                        "MQTT connection established"
                    );

                    if !connack.session_present {
                        info!("MQTT session missing on broker - re-subscribing to topics");
                        let topics = {
                            let guard = subscriptions.lock().await;
                            guard.clone()
                        };

                        for (topic, qos) in topics {
                            match client.subscribe(topic.clone(), qos).await {
                                Ok(_) => {
                                    info!(topic = %topic, qos = ?qos, "Re-subscribed to MQTT topic")
                                }
                                Err(e) => {
                                    error!(error = %e, topic = %topic, "Failed to re-subscribe to MQTT topic")
                                }
                            }
                        }
                    }

                    continue;
                }
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    message_count += 1;

                    // Filter duplicate messages at the interface layer
                    if !dedup_filter.should_process(&p.topic, &p.payload) {
                        continue;
                    }

                    debug!(
                        topic = %p.topic,
                        payload_size = p.payload.len(),
                        qos = ?p.qos,
                        message_count = message_count,
                        "Received MQTT message"
                    );

                    if let Some(topic) = ZigbeeTopic::parse(&p.topic) {
                        // Handle bridge response topics (e.g., networkmap)
                        if topic.is_bridge_response() {
                            debug!(topic = %p.topic, "Handling bridge response");
                            if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                                handle_bridge_response(&payload, &db, &event_bus.sender()).await;
                            } else {
                                warn!(topic = %p.topic, "Invalid UTF-8 in bridge response payload");
                            }
                            continue;
                        }

                        // Handle bridge event topics (e.g., device_joined)
                        if topic.is_bridge_event() {
                            debug!(topic = %p.topic, "Handling bridge event");
                            if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                                handle_bridge_event(&payload, &db, &event_bus.sender()).await;
                            } else {
                                warn!(topic = %p.topic, "Invalid UTF-8 in bridge event payload");
                            }
                            continue;
                        }

                        // Skip other bridge topics
                        if topic.is_bridge() {
                            skipped_bridge += 1;
                            debug!(
                                topic = %p.topic,
                                skipped_count = skipped_bridge,
                                "Skipping bridge topic"
                            );
                            continue;
                        }

                        if topic.is_command_channel() {
                            debug!(
                                topic = %p.topic,
                                "Skipping /set command topic"
                            );
                            continue;
                        }

                        if topic.is_availability() {
                            debug!(
                                topic = %p.topic,
                                "Skipping /availability topic"
                            );
                            continue;
                        }

                        let Some(mqtt_topic) = topic.device_id() else {
                            debug!(topic = %p.topic, "Ignoring topic without device id");
                            continue;
                        };

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
            if message_count.is_multiple_of(100) && message_count > 0 {
                info!(
                    total_messages = message_count,
                    errors = error_count,
                    skipped_bridge = skipped_bridge,
                    parse_errors = parse_errors,
                    no_temperature = no_temperature,
                    publish_failures = publish_failures,
                    duplicates_filtered = dedup_filter.filtered_count(),
                    "MQTT event loop statistics"
                );
            }
        }
    });
}

use crate::db::Database;
use crate::models::{SwitchMqttMessage, TempSensorMqttMessage, TemperatureReading};
use crate::mqtt::handlers::{
    handle_switch_message, handle_temperature_message, handle_temperature_parse_error,
};
use crate::state::{DeviceStateStore, SwitchStateStore};
use rumqttc::{Event, EventLoop, Packet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Spawn the MQTT event loop handler
pub(super) fn spawn_event_loop(
    mut eventloop: EventLoop,
    db: Arc<Database>,
    device_state: DeviceStateStore,
    switch_state: SwitchStateStore,
    tx: mpsc::Sender<TemperatureReading>,
) {
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
                            handle_switch_message(
                                &db,
                                &device_state,
                                &switch_state,
                                mqtt_topic,
                                switch_msg,
                            )
                            .await;
                        }

                        // Also try parsing as temperature message
                        match serde_json::from_slice::<TempSensorMqttMessage>(&p.payload) {
                            Ok(msg) => {
                                handle_temperature_message(
                                    &db,
                                    &device_state,
                                    &tx,
                                    mqtt_topic,
                                    msg,
                                    &mut no_temperature,
                                    &mut send_failures,
                                )
                                .await;
                            }
                            Err(e) => {
                                handle_temperature_parse_error(
                                    mqtt_topic,
                                    &p.topic,
                                    &p.payload,
                                    &e,
                                    &mut parse_errors,
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
}

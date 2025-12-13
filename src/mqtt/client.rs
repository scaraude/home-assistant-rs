use super::event_loop::spawn_event_loop;
use crate::db::Database;
use crate::models::SensorReading;
use crate::state::{DeviceStateStore, SwitchStateStore};
use rumqttc::{AsyncClient, ClientError, MqttOptions, QoS};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Unified MQTT client for both publishing commands and listening to messages.
/// Handles all MQTT operations including subscribing to topics and publishing commands.
#[derive(Clone)]
pub struct MqttClient {
    client: AsyncClient,
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
    ) -> (Self, mpsc::Receiver<SensorReading>) {
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

        let (async_client, eventloop) = AsyncClient::new(mqtt_options, 10);
        let (tx, rx) = mpsc::channel(100);

        info!(
            queue_size = 10,
            channel_capacity = 100,
            "MQTT client and channels created"
        );

        // Spawn the event loop handler
        spawn_event_loop(eventloop, db, device_state, switch_state, tx);

        (
            Self {
                client: async_client,
            },
            rx,
        )
    }

    /// Subscribe to zigbee2mqtt topics
    pub async fn subscribe_to_zigbee2mqtt(&self) -> Result<(), ClientError> {
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
    pub async fn publish_command(&self, device_id: &str, payload: &str) -> Result<(), ClientError> {
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

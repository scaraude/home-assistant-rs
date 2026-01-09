use super::event_loop::spawn_event_loop;
use crate::db::Database;
use crate::events::bus::EventBus;
use crate::mqtt::topic::{ZIGBEE_NAMESPACE, ZigbeeTopic};
use rumqttc::{AsyncClient, ClientError, MqttOptions, QoS};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Unified MQTT client for both publishing commands and listening to messages.
/// Handles all MQTT operations including subscribing to topics and publishing commands.
#[derive(Clone)]
pub struct MqttClient {
    client: AsyncClient,
    event_bus: EventBus,
    subscriptions: Arc<Mutex<Vec<(String, QoS)>>>,
}

impl MqttClient {
    /// Create a new MQTT client and start listening to messages
    pub fn new(
        broker_host: &str,
        broker_port: u16,
        client_id: &str,
        capacity: Option<usize>,
        db: Arc<Database>,
        event_bus: EventBus,
    ) -> Self {
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

        let cap = if capacity.is_some() {
            capacity.unwrap()
        } else {
            50
        };
        let (async_client, eventloop) = AsyncClient::new(mqtt_options, cap);
        info!(queue_size = cap, "MQTT client created");
        let subscriptions = Arc::new(Mutex::new(Vec::new()));

        // Spawn the event loop handler
        spawn_event_loop(
            eventloop,
            async_client.clone(),
            db,
            event_bus.clone(),
            subscriptions.clone(),
        );

        Self {
            client: async_client,
            event_bus,
            subscriptions,
        }
    }

    /// Subscribe to zigbee2mqtt topics
    pub async fn subscribe_to_zigbee2mqtt(&self) -> Result<(), ClientError> {
        let subscription = format!("{ZIGBEE_NAMESPACE}#");
        info!(
            topic = %subscription,
            qos = "AtMostOnce",
            subscribers = self.event_bus.receiver_count(),
            "Subscribing to MQTT topics"
        );

        let qos = QoS::AtMostOnce;

        match self.client.subscribe(&subscription, qos).await {
            Ok(_) => {
                info!("Successfully subscribed to Zigbee namespace topics");
                self.remember_subscription(subscription.clone(), qos).await;
                Ok(())
            }
            Err(e) => {
                error!(error = %e, topic = %subscription, "Failed to subscribe to MQTT topics");
                Err(e)
            }
        }
    }

    /// Publish a command to a device
    pub async fn publish_command(&self, device_id: &str, payload: &str) -> Result<(), ClientError> {
        let topic = ZigbeeTopic::command_topic(device_id);

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

    /// Publish a bridge request (e.g. permit_join) to Zigbee2MQTT.
    pub async fn publish_bridge_request(
        &self,
        request: &str,
        payload: &str,
    ) -> Result<(), ClientError> {
        let topic = ZigbeeTopic::bridge_request_topic(request);

        info!(
            topic = %topic,
            payload = %payload,
            "Publishing MQTT bridge request"
        );

        match self
            .client
            .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
        {
            Ok(_) => {
                info!(topic = %topic, "Successfully published bridge request");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, topic = %topic, "Failed to publish bridge request");
                Err(e)
            }
        }
    }

    /// Publish to a raw MQTT topic (for custom requests)
    pub async fn publish_raw(&self, topic: &str, payload: &str) -> Result<(), ClientError> {
        info!(
            topic = %topic,
            payload = %payload,
            "Publishing raw MQTT message"
        );

        match self
            .client
            .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
        {
            Ok(_) => {
                info!(topic = %topic, "Successfully published raw message");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, topic = %topic, "Failed to publish raw message");
                Err(e)
            }
        }
    }

    async fn remember_subscription(&self, topic: String, qos: QoS) {
        let mut guard = self.subscriptions.lock().await;
        if guard
            .iter()
            .any(|(existing_topic, existing_qos)| existing_topic == &topic && *existing_qos == qos)
        {
            return;
        }

        guard.push((topic, qos));
    }
}

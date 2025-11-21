use crate::models::{TemperatureReading, Zigbee2MqttMessage};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;

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
        let mut mqtt_options = MqttOptions::new(client_id, broker_host, broker_port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));

        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
        let (tx, rx) = mpsc::channel(100);

        // Spawn the event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(p))) => {
                        // Extract sensor ID from topic: zigbee2mqtt/<sensor_id>
                        if let Some(sensor_id) = p.topic.strip_prefix("zigbee2mqtt/") {
                            // Skip bridge topics
                            if sensor_id.starts_with("bridge/") {
                                continue;
                            }

                            // Parse the message
                            if let Ok(msg) =
                                serde_json::from_slice::<Zigbee2MqttMessage>(&p.payload)
                            {
                                // Create temperature reading if the message contains temperature data
                                if let Some(reading) =
                                    TemperatureReading::from_mqtt(sensor_id.to_string(), msg)
                                {
                                    // Send to channel (drop if full to avoid blocking)
                                    let _ = tx.send(reading).await;
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("MQTT error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        (Self { client }, rx)
    }

    /// Subscribe to zigbee2mqtt topics
    pub async fn subscribe(&self) -> Result<(), rumqttc::ClientError> {
        // Subscribe to all zigbee2mqtt device messages
        self.client
            .subscribe("zigbee2mqtt/#", QoS::AtMostOnce)
            .await?;
        Ok(())
    }
}

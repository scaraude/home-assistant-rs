use tokio::sync::broadcast;

use super::SystemEvent;

/// Thin wrapper over Tokio's broadcast channel tailored for `SystemEvent`s.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    /// Create a new event bus with the provided ring-buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event to all current subscribers.
    pub fn publish(
        &self,
        event: SystemEvent,
    ) -> Result<usize, broadcast::error::SendError<SystemEvent>> {
        self.sender.send(event)
    }

    /// Subscribe to the bus and receive future events.
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }

    /// Number of active receivers currently attached to the bus.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get a clone of the sender for services that need to emit events.
    pub fn sender(&self) -> broadcast::Sender<SystemEvent> {
        self.sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::SystemEvent;
    use crate::models::SensorReading;
    use chrono::Utc;

    fn sample_event() -> SystemEvent {
        SystemEvent::SensorReading {
            device_id: "demo".into(),
            reading: SensorReading::TempHumidity {
                device_id: "demo".into(),
                temperature: 20.0,
                humidity: 40.0,
                timestamp: Utc::now(),
            },
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn publish_and_receive_event() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();

        let event = sample_event();
        bus.publish(event.clone()).expect("publish");

        let received = rx.recv().await.expect("receive");
        assert_eq!(received.event_type(), event.event_type());
    }

    #[tokio::test]
    async fn reports_lagged_receivers() {
        let bus = EventBus::new(1);
        let mut rx = bus.subscribe();

        // Fill the buffer without receiving to trigger lag.
        bus.publish(sample_event()).expect("publish");
        bus.publish(sample_event()).expect("publish again");

        match rx.recv().await {
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                assert_eq!(skipped, 1);
            }
            other => panic!("expected lagged error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn receiver_count_tracks_subscribers() {
        let bus = EventBus::new(4);
        assert_eq!(bus.receiver_count(), 0);
        let _rx1 = bus.subscribe();
        let _rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);
    }
}

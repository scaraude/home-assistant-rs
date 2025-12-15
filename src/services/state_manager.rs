use crate::events::SystemEvent;
use crate::state::{DeviceStateStore, SwitchStateStore};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Service that keeps the in-memory state authoritative by consuming bus events.
pub struct StateManagerService {
    device_state: DeviceStateStore,
    switch_state: SwitchStateStore,
    event_rx: broadcast::Receiver<SystemEvent>,
}

impl StateManagerService {
    pub fn new(
        device_state: DeviceStateStore,
        switch_state: SwitchStateStore,
        event_rx: broadcast::Receiver<SystemEvent>,
    ) -> Self {
        Self {
            device_state,
            switch_state,
            event_rx,
        }
    }

    /// Run the event loop until the sender side drops.
    pub async fn run(mut self) {
        info!("StateManagerService started");

        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    self.handle_event(event);
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "StateManagerService lagged behind the event bus"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Event bus closed; StateManagerService exiting");
                    break;
                }
            }
        }
    }

    fn handle_event(&self, event: SystemEvent) {
        match event {
            SystemEvent::DeviceState {
                device_id,
                battery,
                link_quality,
                timestamp,
            } => {
                let log_timestamp = timestamp.clone();
                let id_for_log = device_id.clone();
                self.device_state.update_state(device_id, move |state| {
                    if let Some(level) = battery {
                        state.battery_level = Some(level);
                    }
                    if let Some(quality) = link_quality {
                        state.link_quality = Some(quality);
                    }
                    state.last_seen = timestamp;
                });

                debug!(
                    device_id = %id_for_log,
                    battery = ?battery,
                    link_quality = ?link_quality,
                    timestamp = %log_timestamp,
                    "Device state updated from event"
                );
            }
            SystemEvent::SwitchState {
                device_id,
                state,
                timestamp,
            } => {
                let id_for_log = device_id.clone();
                self.switch_state.set_state(device_id, state);
                debug!(
                    device_id = %id_for_log,
                    state = state.to_bool(),
                    timestamp = %timestamp,
                    "Switch state updated from event"
                );
            }
            _ => {} // Ignore other event types.
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::SwitchState;

    use super::*;
    use chrono::{TimeZone, Utc};

    #[tokio::test]
    async fn updates_device_state_from_events() {
        let device_store = DeviceStateStore::new();
        let switch_store = SwitchStateStore::new();
        let (tx, rx) = broadcast::channel(4);

        let service = StateManagerService::new(device_store.clone(), switch_store.clone(), rx);
        let handle = tokio::spawn(service.run());

        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        tx.send(SystemEvent::DeviceState {
            device_id: "dev-1".into(),
            battery: Some(90),
            link_quality: Some(200),
            timestamp: ts,
        })
        .unwrap();
        drop(tx);

        handle.await.unwrap();

        let state = device_store
            .get_state("dev-1")
            .expect("device state updated");
        assert_eq!(state.battery_level, Some(90));
        assert_eq!(state.link_quality, Some(200));
        assert_eq!(state.last_seen, ts);
    }

    #[tokio::test]
    async fn updates_switch_state_from_events() {
        let device_store = DeviceStateStore::new();
        let switch_store = SwitchStateStore::new();
        let (tx, rx) = broadcast::channel(4);

        let service = StateManagerService::new(device_store.clone(), switch_store.clone(), rx);
        let handle = tokio::spawn(service.run());

        tx.send(SystemEvent::SwitchState {
            device_id: "switch-1".into(),
            state: SwitchState::On,
            timestamp: Utc::now(),
        })
        .unwrap();
        drop(tx);

        handle.await.unwrap();
        assert_eq!(
            switch_store.get_state("switch-1"),
            Some(SwitchState::On),
            "switch state updated"
        );
    }
}

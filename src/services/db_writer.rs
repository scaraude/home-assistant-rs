use std::sync::Arc;

use crate::{
    db::Database,
    events::SystemEvent,
    models::{AutomationExecutionLog, DeviceState},
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Service responsible for persisting incoming events to SQLite.
pub struct DbWriterService {
    db: Arc<Database>,
    event_rx: broadcast::Receiver<SystemEvent>,
}

impl DbWriterService {
    pub fn new(db: Arc<Database>, event_rx: broadcast::Receiver<SystemEvent>) -> Self {
        Self { db, event_rx }
    }

    /// Run the service loop until the event bus closes.
    pub async fn run(mut self) {
        info!("DbWriterService started");
        let mut processed_events: u64 = 0;

        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    processed_events += 1;
                    self.handle_event(event);

                    if processed_events % 100 == 0 {
                        info!(processed_events, "DbWriterService processed events batch");
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "DbWriterService lagged behind the event bus"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Event bus closed; DbWriterService exiting");
                    break;
                }
            }
        }
    }

    fn handle_event(&self, event: SystemEvent) {
        match event {
            SystemEvent::SensorReading { reading, .. } => {
                if let Err(e) = self.db.insert_reading(&reading) {
                    error!(
                        error = %e,
                        device_id = %reading.device_id(),
                        "Failed to persist sensor reading"
                    );
                } else {
                    debug!(
                        device_id = %reading.device_id(),
                        "Persisted sensor reading to database"
                    );
                }
            }
            SystemEvent::DeviceState {
                device_id,
                battery,
                link_quality,
                timestamp,
            } => {
                let state = DeviceState {
                    device_id: device_id.clone(),
                    battery_level: battery,
                    link_quality,
                    last_seen: timestamp,
                };

                if let Err(e) = self.db.upsert_device_state(&state) {
                    error!(
                        error = %e,
                        device_id = %device_id,
                        "Failed to persist device state"
                    );
                } else {
                    debug!(device_id = %device_id, "Persisted device state to database");
                }
            }
            SystemEvent::SwitchState {
                device_id,
                state,
                timestamp,
            } => {
                if let Err(e) = self.db.insert_switch_state(&device_id, state, timestamp) {
                    error!(
                        error = %e,
                        device_id = %device_id,
                        "Failed to persist switch state"
                    );
                } else {
                    debug!(device_id = %device_id, "Persisted switch state to database");
                }
            }
            SystemEvent::AutomationExecuted {
                rule_id,
                success,
                error,
                timestamp,
            } => {
                let rule_name = self
                    .db
                    .get_automation_rule(&rule_id)
                    .ok()
                    .flatten()
                    .map(|rule| rule.name)
                    .unwrap_or_else(|| rule_id.clone());

                let log = AutomationExecutionLog::new(
                    rule_id.clone(),
                    rule_name.clone(),
                    success,
                    error,
                    timestamp,
                );

                if let Err(e) = self.db.insert_execution_log(&log) {
                    error!(
                        error = %e,
                        rule_id = %rule_id,
                        rule_name = %rule_name,
                        "Failed to insert automation execution log"
                    );
                } else if success {
                    if let Err(e) = self.db.record_rule_trigger(&rule_id) {
                        error!(error = %e, rule_id = %rule_id, "Failed to record rule trigger");
                    }
                }
            }
            _ => {
                // We only persist a subset of events.
            }
        }
    }
}

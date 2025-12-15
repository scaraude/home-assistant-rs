//! Event definitions for the application-wide event bus.

pub mod bus;

use crate::models::{AutomationAction, DeviceCapability, SensorReading, SwitchState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core system events that flow through the broadcast event bus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SystemEvent {
    SensorReading {
        device_id: String,
        reading: SensorReading,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    SwitchState {
        device_id: String,
        state: SwitchState,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    DeviceState {
        device_id: String,
        battery: Option<u8>,
        link_quality: Option<u8>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    DeviceDiscovered {
        device_id: String,
        mqtt_topic: String,
        capability: DeviceCapability,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    AutomationTriggered {
        rule_id: String,
        actions: Vec<AutomationAction>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    AutomationExecuted {
        rule_id: String,
        success: bool,
        error: Option<String>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
}

impl SystemEvent {
    /// Human-friendly identifier for the event variant.
    pub fn event_type(&self) -> &'static str {
        match self {
            SystemEvent::SensorReading { .. } => "sensor_reading",
            SystemEvent::SwitchState { .. } => "switch_state",
            SystemEvent::DeviceState { .. } => "device_state",
            SystemEvent::DeviceDiscovered { .. } => "device_discovered",
            SystemEvent::AutomationTriggered { .. } => "automation_triggered",
            SystemEvent::AutomationExecuted { .. } => "automation_executed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_helper_matches_variants() {
        let event = SystemEvent::SwitchState {
            device_id: "switch-1".into(),
            state: SwitchState::On,
            timestamp: Utc::now(),
        };
        assert_eq!("switch_state", event.event_type());
    }
}

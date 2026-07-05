//! Event definitions for the application-wide event bus.

pub mod bus;

use crate::logs::{LogEntry, LogFile};
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
        turbo_mode: Option<bool>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    DeviceDiscovered {
        device_id: String,
        mqtt_topic: String,
        capabilities: Vec<DeviceCapability>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    /// Zigbee availability transition reported by Zigbee2MQTT.
    DeviceAvailability {
        device_id: String,
        device_name: String,
        online: bool,
        /// Previous known state (None when the device had no recorded state).
        was_online: Option<bool>,
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
    AutomationRuleDeleted {
        rule_id: String,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    /// New log entries from monitor.sh log files
    LogEntries {
        log_file: LogFile,
        entries: Vec<LogEntry>,
        #[serde(with = "chrono::serde::ts_seconds")]
        timestamp: DateTime<Utc>,
    },
    /// Network topology has been updated
    NetworkTopologyUpdated,
    /// Device pairing event (device_joined, device_interview)
    DevicePairing {
        ieee_addr: String,
        friendly_name: String,
    },
    /// Device position updated on floor map
    DevicePositionUpdated {
        device_id: String,
        x: f64,
        y: f64,
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
            SystemEvent::DeviceAvailability { .. } => "device_availability",
            SystemEvent::AutomationTriggered { .. } => "automation_triggered",
            SystemEvent::AutomationExecuted { .. } => "automation_executed",
            SystemEvent::AutomationRuleDeleted { .. } => "automation_rule_deleted",
            SystemEvent::LogEntries { .. } => "log_entries",
            SystemEvent::NetworkTopologyUpdated => "network_topology_updated",
            SystemEvent::DevicePairing { .. } => "device_pairing",
            SystemEvent::DevicePositionUpdated { .. } => "device_position_updated",
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

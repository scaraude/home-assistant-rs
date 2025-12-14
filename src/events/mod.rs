//! Event definitions for the application-wide event bus.

pub mod bus;

use crate::models::{AutomationAction, DeviceCapability, SensorReading};
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
        state: bool,
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

    /// Timestamp associated with the event.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SystemEvent::SensorReading { timestamp, .. }
            | SystemEvent::SwitchState { timestamp, .. }
            | SystemEvent::DeviceState { timestamp, .. }
            | SystemEvent::DeviceDiscovered { timestamp, .. }
            | SystemEvent::AutomationTriggered { timestamp, .. }
            | SystemEvent::AutomationExecuted { timestamp, .. } => *timestamp,
        }
    }

    /// Device identifier when the event references a device.
    pub fn device_id(&self) -> Option<&str> {
        match self {
            SystemEvent::SensorReading { device_id, .. }
            | SystemEvent::SwitchState { device_id, .. }
            | SystemEvent::DeviceState { device_id, .. }
            | SystemEvent::DeviceDiscovered { device_id, .. } => Some(device_id.as_str()),
            SystemEvent::AutomationTriggered { .. } | SystemEvent::AutomationExecuted { .. } => {
                None
            }
        }
    }

    /// Convenience constructor for automation execution failures.
    pub fn automation_failure(rule_id: impl Into<String>, error: impl Into<String>) -> Self {
        SystemEvent::AutomationExecuted {
            rule_id: rule_id.into(),
            success: false,
            error: Some(error.into()),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn demo_reading() -> SensorReading {
        SensorReading::TempHumidity {
            device_id: "device-123".into(),
            temperature: 21.5,
            humidity: 40.0,
            timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn event_type_helper_matches_variants() {
        let event = SystemEvent::SwitchState {
            device_id: "switch-1".into(),
            state: true,
            timestamp: Utc::now(),
        };
        assert_eq!("switch_state", event.event_type());
    }

    #[test]
    fn serialization_round_trip() {
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let event = SystemEvent::SensorReading {
            device_id: "sensor-1".into(),
            reading: demo_reading(),
            timestamp: ts,
        };

        let json = serde_json::to_string(&event).expect("serialize");
        let back: SystemEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, back);
        assert_eq!(back.event_type(), "sensor_reading");
        assert_eq!(back.timestamp(), ts);
        assert_eq!(back.device_id(), Some("sensor-1"));
    }
}

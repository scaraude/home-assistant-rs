use crate::db::Database;
use crate::events::{SystemEvent, bus::EventBus};
use chrono::Utc;
use tracing::{debug, error, info, warn};

/// Handle a Zigbee2MQTT availability message ("zigbee2mqtt/<device>/availability").
///
/// The payload is either `{"state":"online"}` (legacy_availability_payload: false)
/// or the plain string `online` / `offline`. State is persisted and a
/// `DeviceAvailability` event is published only on actual transitions, so
/// retained messages replayed at (re)connection stay silent.
pub async fn handle_availability_message(
    db: &Database,
    event_bus: &EventBus,
    mqtt_topic: &str,
    payload: &[u8],
) {
    let Some(online) = parse_availability_payload(payload) else {
        warn!(
            mqtt_topic = %mqtt_topic,
            payload_preview = ?String::from_utf8_lossy(&payload[..payload.len().min(100)]),
            "Unrecognized availability payload"
        );
        return;
    };

    // Only track devices we already know; availability alone should not
    // create device records.
    let device = match db.get_device_by_mqtt_topic(mqtt_topic) {
        Ok(Some(device)) => device,
        Ok(None) => {
            debug!(mqtt_topic = %mqtt_topic, "Availability for unknown device, ignoring");
            return;
        }
        Err(e) => {
            error!(error = %e, mqtt_topic = %mqtt_topic, "Failed to resolve device for availability");
            return;
        }
    };

    let was_online = match db.get_device_availability(&device.id) {
        Ok(previous) => previous,
        Err(e) => {
            error!(error = %e, device_id = %device.id, "Failed to read previous availability");
            return;
        }
    };

    if was_online == Some(online) {
        debug!(device_id = %device.id, online = online, "Availability unchanged");
        return;
    }

    let now = Utc::now();
    if let Err(e) = db.set_device_availability(&device.id, online, now.timestamp()) {
        error!(error = %e, device_id = %device.id, "Failed to persist availability");
        return;
    }

    info!(
        device_id = %device.id,
        name = %device.name,
        online = online,
        was_online = ?was_online,
        "Device availability changed"
    );

    let event = SystemEvent::DeviceAvailability {
        device_id: device.id.clone(),
        device_name: device.name.clone(),
        online,
        was_online,
        timestamp: now,
    };
    if let Err(e) = event_bus.publish(event) {
        error!(error = %e, device_id = %device.id, "Failed to publish availability event");
    }
}

fn parse_availability_payload(payload: &[u8]) -> Option<bool> {
    #[derive(serde::Deserialize)]
    struct AvailabilityPayload {
        state: String,
    }

    let state = if let Ok(parsed) = serde_json::from_slice::<AvailabilityPayload>(payload) {
        parsed.state
    } else {
        String::from_utf8_lossy(payload).trim().to_string()
    };

    match state.as_str() {
        "online" => Some(true),
        "offline" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_availability_payload;

    #[test]
    fn parses_json_payloads() {
        assert_eq!(
            parse_availability_payload(br#"{"state":"online"}"#),
            Some(true)
        );
        assert_eq!(
            parse_availability_payload(br#"{"state":"offline"}"#),
            Some(false)
        );
    }

    #[test]
    fn parses_legacy_string_payloads() {
        assert_eq!(parse_availability_payload(b"online"), Some(true));
        assert_eq!(parse_availability_payload(b"offline"), Some(false));
    }

    #[test]
    fn rejects_unknown_payloads() {
        assert_eq!(parse_availability_payload(b"whatever"), None);
        assert_eq!(parse_availability_payload(br#"{"state":"weird"}"#), None);
    }
}

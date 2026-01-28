//! MQTT message deduplication filter
//!
//! Zigbee devices sometimes send duplicate messages within the same second
//! (e.g., same temperature with slightly different humidity).
//! This filter treats small numeric deltas within the window as duplicates.
//! This filter catches these duplicates at the interface layer before processing.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::time::Instant;

/// Time window in seconds to consider messages as duplicates
const DEDUP_WINDOW_SECS: u64 = 1;
const MAX_RECENT_PER_TOPIC: usize = 5;
const ABS_EPSILON: f64 = 0.5;
const REL_EPSILON: f64 = 0.01;

/// A message signature for deduplication
#[derive(Debug, Clone)]
struct MessageSignature {
    payload_hash: u64,
    received_at: Instant,
    numeric_fields: HashMap<String, f64>,
}

/// Filter that tracks recent MQTT messages and identifies duplicates
#[derive(Debug, Default)]
pub struct MqttDedupFilter {
    /// Recent message signatures per topic
    recent_messages: HashMap<String, VecDeque<MessageSignature>>,
    /// Count of filtered duplicates
    filtered_count: u64,
}

impl MqttDedupFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an MQTT message should be processed or filtered
    ///
    /// Returns `true` if the message should be processed (not a duplicate),
    /// `false` if it should be filtered out.
    ///
    /// A message is a duplicate if the topic recently saw the same payload
    /// or a near-identical set of numeric fields within DEDUP_WINDOW_SECS.
    pub fn should_process(&mut self, topic: &str, payload: &[u8]) -> bool {
        let now = Instant::now();
        let payload_hash = Self::hash_payload(payload);
        let numeric_fields = Self::extract_numeric_fields(payload);

        let recent = self
            .recent_messages
            .entry(topic.to_string())
            .or_insert_with(VecDeque::new);

        // Drop entries outside the window.
        while let Some(front) = recent.front() {
            if now.duration_since(front.received_at).as_secs() < DEDUP_WINDOW_SECS {
                break;
            }
            recent.pop_front();
        }

        let is_duplicate = recent.iter().any(|prior| {
            prior.payload_hash == payload_hash
                || Self::is_similar_numeric_fields(prior, &numeric_fields)
        });

        if is_duplicate {
            let elapsed_ms = recent
                .back()
                .map(|last| now.duration_since(last.received_at).as_millis())
                .unwrap_or(0);
            self.filtered_count += 1;
            tracing::debug!(
                topic = %topic,
                elapsed_ms,
                filtered_total = self.filtered_count,
                "Filtered duplicate MQTT message"
            );
            return false;
        }

        if recent.len() == MAX_RECENT_PER_TOPIC {
            recent.pop_front();
        }

        recent.push_back(MessageSignature {
            payload_hash,
            received_at: now,
            numeric_fields,
        });

        true
    }

    fn hash_payload(payload: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.hash(&mut hasher);
        hasher.finish()
    }

    fn extract_numeric_fields(payload: &[u8]) -> HashMap<String, f64> {
        let payload_str = match std::str::from_utf8(payload) {
            Ok(s) => s,
            Err(_) => return HashMap::new(),
        };

        let value: serde_json::Value = match serde_json::from_str(payload_str) {
            Ok(v) => v,
            Err(_) => return HashMap::new(),
        };

        let obj = match value.as_object() {
            Some(map) => map,
            None => return HashMap::new(),
        };

        let mut fields = HashMap::new();
        for (key, value) in obj {
            if Self::is_ignored_key(key) {
                continue;
            }
            if let Some(num) = value.as_f64() {
                fields.insert(key.clone(), num);
            }
        }

        fields
    }

    fn is_similar_numeric_fields(
        last: &MessageSignature,
        numeric_fields: &HashMap<String, f64>,
    ) -> bool {
        let mut compared = false;

        for (key, current) in numeric_fields {
            let Some(prev) = last.numeric_fields.get(key) else {
                continue;
            };
            let max_mag = prev.abs().max(current.abs());
            let tolerance = ABS_EPSILON.max(max_mag * REL_EPSILON);
            if (prev - current).abs() > tolerance {
                return false;
            }
            compared = true;
        }

        compared
    }

    fn is_ignored_key(key: &str) -> bool {
        matches!(
            key,
            "linkquality"
                | "battery"
                | "voltage"
                | "last_seen"
                | "update"
                | "rssi"
                | "lqi"
                | "state"
                | "action"
        )
    }

    pub fn filtered_count(&self) -> u64 {
        self.filtered_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_message_passes() {
        let mut filter = MqttDedupFilter::new();
        assert!(filter.should_process("device/sensor1", b"temp:20.5"));
        assert_eq!(filter.filtered_count(), 0);
    }

    #[test]
    fn test_duplicate_filtered() {
        let mut filter = MqttDedupFilter::new();

        assert!(filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(!filter.should_process("device/sensor1", b"temp:20.5"));
        assert_eq!(filter.filtered_count(), 1);
    }

    #[test]
    fn test_multiple_duplicates_counted() {
        let mut filter = MqttDedupFilter::new();

        assert!(filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(!filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(!filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(!filter.should_process("device/sensor1", b"temp:20.5"));
        assert_eq!(filter.filtered_count(), 3);
    }

    #[test]
    fn test_different_payload_passes() {
        let mut filter = MqttDedupFilter::new();

        assert!(filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(filter.should_process("device/sensor1", b"temp:21.0"));
        assert_eq!(filter.filtered_count(), 0);
    }

    #[test]
    fn test_different_topics_independent() {
        let mut filter = MqttDedupFilter::new();

        assert!(filter.should_process("device/sensor1", b"temp:20.5"));
        assert!(filter.should_process("device/sensor2", b"temp:20.5"));
        assert!(!filter.should_process("device/sensor1", b"temp:20.5"));
        assert_eq!(filter.filtered_count(), 1);
    }

    #[test]
    fn test_realistic_zigbee_json_payloads() {
        let mut filter = MqttDedupFilter::new();

        let payload1 = br#"{"temperature":15.66,"humidity":59.19,"linkquality":120}"#;
        let payload2 = br#"{"temperature":15.66,"humidity":58.94,"linkquality":120}"#;
        let payload3 = br#"{"temperature":15.66,"humidity":57.4,"linkquality":120}"#;
        let payload4 = br#"{"temperature":15.66,"humidity":59.19,"linkquality":80}"#;

        // First message passes
        assert!(filter.should_process("zigbee2mqtt/sensor_living_room", payload1));

        // Exact duplicate filtered
        assert!(!filter.should_process("zigbee2mqtt/sensor_living_room", payload1));

        // Slight humidity change filtered
        assert!(!filter.should_process("zigbee2mqtt/sensor_living_room", payload2));

        // Linkquality-only change filtered
        assert!(!filter.should_process("zigbee2mqtt/sensor_living_room", payload4));

        // Larger humidity change passes
        assert!(filter.should_process("zigbee2mqtt/sensor_living_room", payload3));

        assert_eq!(filter.filtered_count(), 3);
    }

    #[test]
    fn test_energy_meter_similarity() {
        let mut filter = MqttDedupFilter::new();

        let payload1 = br#"{"power":3848,"energy":334.89,"linkquality":120}"#;
        let payload2 = br#"{"power":3848,"energy":334.89,"linkquality":80}"#;
        let payload3 = br#"{"power":3797,"energy":334.89,"linkquality":120}"#;

        assert!(filter.should_process("zigbee2mqtt/plug_kitchen", payload1));
        assert!(!filter.should_process("zigbee2mqtt/plug_kitchen", payload2));
        assert!(filter.should_process("zigbee2mqtt/plug_kitchen", payload3));
        assert_eq!(filter.filtered_count(), 1);
    }

    #[test]
    fn test_empty_payload() {
        let mut filter = MqttDedupFilter::new();

        assert!(filter.should_process("device/sensor1", b""));
        assert!(!filter.should_process("device/sensor1", b""));
        assert_eq!(filter.filtered_count(), 1);
    }

    #[test]
    fn test_binary_payload() {
        let mut filter = MqttDedupFilter::new();

        let binary_payload = &[0x00, 0x01, 0x02, 0xFF, 0xFE];
        assert!(filter.should_process("device/binary", binary_payload));
        assert!(!filter.should_process("device/binary", binary_payload));
        assert_eq!(filter.filtered_count(), 1);
    }

    #[test]
    fn test_many_devices_tracked() {
        let mut filter = MqttDedupFilter::new();

        // Simulate 10 different sensors
        for i in 0..10 {
            let topic = format!("zigbee2mqtt/sensor_{}", i);
            let payload = format!(r#"{{"temperature":{}.5}}"#, 15 + i);
            assert!(filter.should_process(&topic, payload.as_bytes()));
        }

        // All duplicates should be filtered
        for i in 0..10 {
            let topic = format!("zigbee2mqtt/sensor_{}", i);
            let payload = format!(r#"{{"temperature":{}.5}}"#, 15 + i);
            assert!(!filter.should_process(&topic, payload.as_bytes()));
        }

        assert_eq!(filter.filtered_count(), 10);
    }

    #[test]
    fn test_payload_update_resets_tracking() {
        let mut filter = MqttDedupFilter::new();

        // Initial reading
        assert!(filter.should_process("device/sensor1", b"temp:20.0"));

        // Duplicate filtered
        assert!(!filter.should_process("device/sensor1", b"temp:20.0"));

        // New value passes and resets tracking
        assert!(filter.should_process("device/sensor1", b"temp:21.0"));

        // Old value is filtered within the window because it was recently seen
        assert!(!filter.should_process("device/sensor1", b"temp:20.0"));

        assert_eq!(filter.filtered_count(), 2);
    }

    #[test]
    fn test_hash_consistency() {
        // Same payload should always produce same hash
        let hash1 = MqttDedupFilter::hash_payload(b"test payload");
        let hash2 = MqttDedupFilter::hash_payload(b"test payload");
        assert_eq!(hash1, hash2);

        // Different payloads should produce different hashes
        let hash3 = MqttDedupFilter::hash_payload(b"different payload");
        assert_ne!(hash1, hash3);
    }
}

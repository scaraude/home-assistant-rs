//! MQTT message deduplication filter
//!
//! Zigbee devices sometimes send duplicate messages within the same second
//! (e.g., same temperature with slightly different humidity).
//! This filter catches these duplicates at the interface layer before processing.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

/// Time window in seconds to consider messages as duplicates
const DEDUP_WINDOW_SECS: u64 = 2;

/// A message signature for deduplication
#[derive(Debug, Clone)]
struct MessageSignature {
    payload_hash: u64,
    received_at: Instant,
}

/// Filter that tracks recent MQTT messages and identifies duplicates
#[derive(Debug, Default)]
pub struct MqttDedupFilter {
    /// Last message signature per topic
    last_messages: HashMap<String, MessageSignature>,
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
    /// A message is a duplicate if same topic + same payload within DEDUP_WINDOW_SECS.
    pub fn should_process(&mut self, topic: &str, payload: &[u8]) -> bool {
        let now = Instant::now();
        let payload_hash = Self::hash_payload(payload);

        if let Some(last) = self.last_messages.get(topic) {
            let elapsed = now.duration_since(last.received_at);

            if last.payload_hash == payload_hash && elapsed.as_secs() < DEDUP_WINDOW_SECS {
                self.filtered_count += 1;
                tracing::debug!(
                    topic = %topic,
                    elapsed_ms = elapsed.as_millis(),
                    filtered_total = self.filtered_count,
                    "Filtered duplicate MQTT message"
                );
                return false;
            }
        }

        self.last_messages.insert(
            topic.to_string(),
            MessageSignature {
                payload_hash,
                received_at: now,
            },
        );

        true
    }

    fn hash_payload(payload: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.hash(&mut hasher);
        hasher.finish()
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

        // First message passes
        assert!(filter.should_process("zigbee2mqtt/sensor_living_room", payload1));

        // Exact duplicate filtered
        assert!(!filter.should_process("zigbee2mqtt/sensor_living_room", payload1));

        // Different payload (humidity changed) passes
        assert!(filter.should_process("zigbee2mqtt/sensor_living_room", payload2));

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

        // Old value now passes (tracking was reset to new value)
        assert!(filter.should_process("device/sensor1", b"temp:20.0"));

        assert_eq!(filter.filtered_count(), 1);
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

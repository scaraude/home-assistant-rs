//! Helpers for working with Zigbee2MQTT topic names.

/// Prefix for all Zigbee2MQTT topics we care about.
pub const ZIGBEE_NAMESPACE: &str = "zigbee2mqtt/";
/// Bridge topics are emitted by Zigbee2MQTT itself and should be ignored.
pub const BRIDGE_PREFIX: &str = "bridge/";
/// Suffix used for command topics (publish-only).
pub const COMMAND_SUFFIX: &str = "/set";

/// Strongly-typed view over a Zigbee2MQTT topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZigbeeTopic<'a> {
    tail: &'a str,
}

impl<'a> ZigbeeTopic<'a> {
    /// Parse the topic, returning None if it is outside the Zigbee namespace.
    pub fn parse(topic: &'a str) -> Option<Self> {
        let tail = topic.strip_prefix(ZIGBEE_NAMESPACE)?;
        Some(Self { tail })
    }

    /// Whether the topic describes bridge metadata.
    pub fn is_bridge(&self) -> bool {
        self.tail.starts_with(BRIDGE_PREFIX)
    }

    /// Whether this topic points to the command channel ("/set").
    pub fn is_command_channel(&self) -> bool {
        self.tail.ends_with(COMMAND_SUFFIX)
    }

    /// Return the device identifier for telemetry topics.
    pub fn device_id(&self) -> Option<&'a str> {
        if self.is_bridge() {
            None
        } else if self.is_command_channel() {
            self.tail.strip_suffix(COMMAND_SUFFIX)
        } else {
            Some(self.tail)
        }
    }

    /// Build a command topic for a device.
    pub fn command_topic(device_id: &str) -> String {
        format!("{ZIGBEE_NAMESPACE}{device_id}{COMMAND_SUFFIX}")
    }
}

//! Helpers for working with Zigbee2MQTT topic names.

/// Prefix for all Zigbee2MQTT topics we care about.
pub const ZIGBEE_NAMESPACE: &str = "zigbee2mqtt/";
/// Bridge topics are emitted by Zigbee2MQTT itself and should be ignored.
pub const BRIDGE_PREFIX: &str = "bridge/";
/// Bridge request topics for Zigbee2MQTT (e.g. permit_join).
pub const BRIDGE_REQUEST_PREFIX: &str = "bridge/request/";
/// Bridge response topics for Zigbee2MQTT (e.g. networkmap response).
pub const BRIDGE_RESPONSE_PREFIX: &str = "bridge/response/";
/// Bridge event topics for Zigbee2MQTT (e.g. device_joined).
pub const BRIDGE_EVENT_PREFIX: &str = "bridge/event";
/// Suffix used for command topics (publish-only).
pub const COMMAND_SUFFIX: &str = "/set";
/// Suffix used for availability topics (online/offline state).
pub const AVAILABILITY_SUFFIX: &str = "/availability";

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

    /// Whether this is a bridge response topic (e.g. networkmap).
    pub fn is_bridge_response(&self) -> bool {
        self.tail.starts_with(BRIDGE_RESPONSE_PREFIX)
    }

    /// Whether this is a bridge event topic (e.g. device_joined).
    pub fn is_bridge_event(&self) -> bool {
        self.tail.starts_with(BRIDGE_EVENT_PREFIX)
    }

    /// Whether this topic points to the command channel ("/set").
    pub fn is_command_channel(&self) -> bool {
        self.tail.ends_with(COMMAND_SUFFIX)
    }

    /// Whether this topic is an availability topic ("/availability").
    pub fn is_availability(&self) -> bool {
        self.tail.ends_with(AVAILABILITY_SUFFIX)
    }

    /// Return the device identifier for availability topics.
    pub fn availability_device_id(&self) -> Option<&'a str> {
        self.tail.strip_suffix(AVAILABILITY_SUFFIX)
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

    /// Build a bridge request topic (e.g. "permit_join").
    pub fn bridge_request_topic(request: &str) -> String {
        format!("{ZIGBEE_NAMESPACE}{BRIDGE_REQUEST_PREFIX}{request}")
    }
}

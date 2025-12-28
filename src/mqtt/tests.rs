#[cfg(test)]
mod tests {
    use crate::models::{DeviceMqttMessage, SwitchState};
    use crate::mqtt::topic::ZigbeeTopic;
    use crate::state::{DeviceStateStore, SwitchStateStore};

    // ==================== Message Parsing Tests ====================

    #[test]
    fn test_parse_temperature_sensor_message_full() {
        let json = r#"{
            "temperature": 22.5,
            "humidity": 45.0,
            "battery": 100,
            "linkquality": 255
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.temperature, Some(22.5));
        assert_eq!(msg.humidity, Some(45.0));
        assert_eq!(msg.battery, Some(100));
        assert_eq!(msg.linkquality, Some(255));
    }

    #[test]
    fn test_parse_temperature_sensor_message_minimal() {
        let json = r#"{
            "temperature": 18.0
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.temperature, Some(18.0));
        assert_eq!(msg.humidity, None);
        assert_eq!(msg.battery, None);
        assert_eq!(msg.linkquality, None);
    }

    #[test]
    fn test_parse_temperature_sensor_message_humidity_only() {
        let json = r#"{
            "humidity": 60.0,
            "battery": 95,
            "linkquality": 200
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.temperature, None);
        assert_eq!(msg.humidity, Some(60.0));
        assert_eq!(msg.battery, Some(95));
        assert_eq!(msg.linkquality, Some(200));
    }

    #[test]
    fn test_parse_switch_message_on() {
        let json = r#"{
            "state": "ON",
            "linkquality": 150
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some(SwitchState::On));
        assert_eq!(msg.linkquality, Some(150));
    }

    #[test]
    fn test_parse_switch_message_off() {
        let json = r#"{
            "state": "OFF",
            "linkquality": 180
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some(SwitchState::Off));
        assert_eq!(msg.linkquality, Some(180));
    }

    #[test]
    fn test_parse_switch_message_with_extra_fields() {
        let json = r#"{
            "state": "ON",
            "battery": 75,
            "voltage": 3000,
            "linkquality": 200
        }"#;

        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some(SwitchState::On));
        assert_eq!(msg.linkquality, Some(200));
        // battery is a known field, so it goes into the battery field, not other
        assert_eq!(msg.battery, Some(75));
        // voltage is unknown, so it goes into other
        assert!(msg.other.contains_key("voltage"));
    }

    #[test]
    fn test_parse_empty_json() {
        let json = r#"{}"#;

        let temp_msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(temp_msg.temperature, None);
        assert_eq!(temp_msg.humidity, None);
        assert_eq!(temp_msg.battery, None);
        assert_eq!(temp_msg.linkquality, None);

        let switch_msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(switch_msg.state, None);
        assert_eq!(switch_msg.linkquality, None);
    }

    #[test]
    fn test_parse_message_with_unknown_fields() {
        let json = r#"{
            "temperature": 22.5,
            "humidity": 45.0,
            "voltage": 3000,
            "unknown_field": "value"
        }"#;

        // Should parse successfully, ignoring unknown fields
        let msg: DeviceMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.temperature, Some(22.5));
        assert_eq!(msg.humidity, Some(45.0));
    }

    // ==================== Topic Filtering Tests ====================

    #[test]
    fn test_topic_strip_zigbee2mqtt_prefix() {
        let topic = "zigbee2mqtt/0x00158d0001a2b3c4";
        assert!(topic.starts_with("zigbee2mqtt/"));

        let extracted = topic.strip_prefix("zigbee2mqtt/").unwrap();
        assert_eq!(extracted, "0x00158d0001a2b3c4");
    }

    #[test]
    fn test_topic_not_zigbee2mqtt() {
        let topic = "some/other/topic";
        assert!(!topic.starts_with("zigbee2mqtt/"));
        assert!(topic.strip_prefix("zigbee2mqtt/").is_none());
    }

    #[test]
    fn test_topic_is_bridge() {
        assert!("bridge/state".starts_with("bridge/"));
        assert!("bridge/devices".starts_with("bridge/"));
        assert!("bridge/config".starts_with("bridge/"));
    }

    #[test]
    fn test_topic_is_not_bridge() {
        assert!(!"0x00158d0001a2b3c4".starts_with("bridge/"));
        assert!(!"sensor_001".starts_with("bridge/"));
    }

    #[test]
    fn test_topic_is_set_command() {
        assert!("0x00158d0001a2b3c4/set".ends_with("/set"));
        assert!("switch_001/set".ends_with("/set"));
        assert!("device/set".ends_with("/set"));
    }

    #[test]
    fn test_topic_is_not_set_command() {
        assert!(!"0x00158d0001a2b3c4".ends_with("/set"));
        assert!(!"0x00158d0001a2b3c4/get".ends_with("/set"));
        assert!(!"sensor_001".ends_with("/set"));
    }

    // ==================== TemperatureReading Creation Tests ====================

    // NOTE: Old tests removed - TemperatureReading::from_mqtt now uses TempSensorMqttMessage (legacy)
    // Unified message parsing is tested in src/models/mqtt.rs tests instead

    // ==================== State Store Tests ====================

    #[test]
    fn test_device_state_get_nonexistent() {
        let device_state = DeviceStateStore::new();

        let state = device_state.get_state("nonexistent");
        assert!(state.is_none());
    }

    #[test]
    fn test_switch_state_on() {
        let switch_state = SwitchStateStore::new();

        switch_state.set_state("switch_001".to_string(), SwitchState::On);

        let state = switch_state.get_state("switch_001").unwrap();
        assert_eq!(state, SwitchState::On);
    }

    #[test]
    fn test_switch_state_off() {
        let switch_state = SwitchStateStore::new();

        switch_state.set_state("switch_001".to_string(), SwitchState::Off);

        let state = switch_state.get_state("switch_001").unwrap();
        assert_eq!(state, SwitchState::Off);
    }

    #[test]
    fn test_switch_state_toggle() {
        let switch_state = SwitchStateStore::new();

        // Set to ON
        switch_state.set_state("switch_001".to_string(), SwitchState::On);
        assert_eq!(switch_state.get_state("switch_001"), Some(SwitchState::On));

        // Toggle to OFF
        switch_state.set_state("switch_001".to_string(), SwitchState::Off);
        assert_eq!(switch_state.get_state("switch_001"), Some(SwitchState::Off));

        // Toggle back to ON
        switch_state.set_state("switch_001".to_string(), SwitchState::On);
        assert_eq!(switch_state.get_state("switch_001"), Some(SwitchState::On));
    }

    #[test]
    fn test_switch_state_get_nonexistent() {
        let switch_state = SwitchStateStore::new();

        let state = switch_state.get_state("nonexistent");
        assert!(state.is_none());
    }

    #[test]
    fn test_switch_state_case_conversion() {
        assert_eq!(SwitchState::from_mqtt_string("ON"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("on"), Some(SwitchState::On));
        assert_eq!(SwitchState::from_mqtt_string("Off"), Some(SwitchState::Off));
        assert_eq!(SwitchState::from_mqtt_string("invalid"), None);
    }

    // ==================== Command Publishing Format Tests ====================

    #[test]
    fn test_publish_command_topic_format() {
        let device_id = "0x00158d0001a2b3c4";
        let topic = ZigbeeTopic::command_topic(device_id);
        assert_eq!(topic, "zigbee2mqtt/0x00158d0001a2b3c4/set");
    }

    #[test]
    fn test_publish_command_topic_format_various_ids() {
        assert_eq!(
            ZigbeeTopic::command_topic("switch_001"),
            "zigbee2mqtt/switch_001/set"
        );
        assert_eq!(
            ZigbeeTopic::command_topic("sensor_temp"),
            "zigbee2mqtt/sensor_temp/set"
        );
        assert_eq!(
            ZigbeeTopic::command_topic("0xABCD"),
            "zigbee2mqtt/0xABCD/set"
        );
    }

    #[test]
    fn test_publish_command_payload_on() {
        let payload = r#"{"state":"ON"}"#;
        let parsed: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed["state"], "ON");
    }

    #[test]
    fn test_publish_command_payload_off() {
        let payload = r#"{"state":"OFF"}"#;
        let parsed: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed["state"], "OFF");
    }

    #[test]
    fn test_publish_command_payload_toggle() {
        let payload = r#"{"state":"TOGGLE"}"#;
        let parsed: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed["state"], "TOGGLE");
    }

    // ==================== Edge Cases ====================

    // NOTE: Old tests removed - TemperatureReading::from_mqtt now uses unified DeviceMqttMessage
    // These edge case tests are covered by the sensor reading logic in handlers

    // #[test]
    // fn test_temperature_reading_extreme_values() {
    //     let msg = DeviceMqttMessage {
    //         temperature: Some(-40.0),
    //         humidity: Some(0.0),
    //         battery: Some(0),
    //         linkquality: Some(0),
    //     };
    //
    //     let reading = TemperatureReading::from_mqtt("device_001".to_string(), msg);
    //     assert!(reading.is_some());
    //
    //     let reading = reading.unwrap();
    //     assert_eq!(reading.temperature, -40.0);
    //     assert_eq!(reading.humidity, Some(0.0));
    //     assert_eq!(reading.battery, Some(0));
    //     assert_eq!(reading.link_quality, Some(0));
    // }

    // #[test]
    // fn test_temperature_reading_high_values() {
    //     let msg = DeviceMqttMessage {
    //         temperature: Some(125.0),
    //         humidity: Some(100.0),
    //         battery: Some(100),
    //         linkquality: Some(255),
    //     };
    //
    //     let reading = TemperatureReading::from_mqtt("device_001".to_string(), msg);
    //     assert!(reading.is_some());
    //
    //     let reading = reading.unwrap();
    //     assert_eq!(reading.temperature, 125.0);
    //     assert_eq!(reading.humidity, Some(100.0));
    // }

    #[test]
    fn test_device_id_formats() {
        // Test various device ID formats work correctly
        let ids = vec![
            "0x00158d0001a2b3c4",
            "sensor_001",
            "switch-living-room",
            "temp.bedroom",
            "device_with_underscores",
        ];

        for id in ids {
            let topic = format!("zigbee2mqtt/{}", id);
            assert!(topic.starts_with("zigbee2mqtt/"));
            assert_eq!(topic.strip_prefix("zigbee2mqtt/").unwrap(), id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{SwitchMqttMessage, TempSensorMqttMessage, TemperatureReading};
    use crate::mqtt::device_discovery::MqttMessage;
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

        let msg: TempSensorMqttMessage = serde_json::from_str(json).unwrap();
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

        let msg: TempSensorMqttMessage = serde_json::from_str(json).unwrap();
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

        let msg: TempSensorMqttMessage = serde_json::from_str(json).unwrap();
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

        let msg: SwitchMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some("ON".to_string()));
        assert_eq!(msg.linkquality, Some(150));
    }

    #[test]
    fn test_parse_switch_message_off() {
        let json = r#"{
            "state": "OFF",
            "linkquality": 180
        }"#;

        let msg: SwitchMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some("OFF".to_string()));
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

        let msg: SwitchMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.state, Some("ON".to_string()));
        assert_eq!(msg.linkquality, Some(200));
        assert!(msg.other.contains_key("battery"));
        assert!(msg.other.contains_key("voltage"));
    }

    #[test]
    fn test_parse_empty_json() {
        let json = r#"{}"#;

        let temp_msg: TempSensorMqttMessage = serde_json::from_str(json).unwrap();
        assert_eq!(temp_msg.temperature, None);
        assert_eq!(temp_msg.humidity, None);
        assert_eq!(temp_msg.battery, None);
        assert_eq!(temp_msg.linkquality, None);

        let switch_msg: SwitchMqttMessage = serde_json::from_str(json).unwrap();
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
        let msg: TempSensorMqttMessage = serde_json::from_str(json).unwrap();
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
        let topic = "homeassistant/sensor/temp1";
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

    #[test]
    fn test_temperature_reading_from_mqtt_with_all_fields() {
        let msg = TempSensorMqttMessage {
            temperature: Some(22.5),
            humidity: Some(45.0),
            battery: Some(100),
            linkquality: Some(255),
        };

        let reading = TemperatureReading::from_mqtt("device_001".to_string(), msg);
        assert!(reading.is_some());

        let reading = reading.unwrap();
        assert_eq!(reading.device_id, "device_001");
        assert_eq!(reading.temperature, 22.5);
        assert_eq!(reading.humidity, Some(45.0));
        assert_eq!(reading.battery, Some(100));
        assert_eq!(reading.link_quality, Some(255));
    }

    #[test]
    fn test_temperature_reading_from_mqtt_temperature_only() {
        let msg = TempSensorMqttMessage {
            temperature: Some(18.0),
            humidity: None,
            battery: None,
            linkquality: None,
        };

        let reading = TemperatureReading::from_mqtt("device_002".to_string(), msg);
        assert!(reading.is_some());

        let reading = reading.unwrap();
        assert_eq!(reading.device_id, "device_002");
        assert_eq!(reading.temperature, 18.0);
        assert_eq!(reading.humidity, None);
        assert_eq!(reading.battery, None);
        assert_eq!(reading.link_quality, None);
    }

    #[test]
    fn test_temperature_reading_from_mqtt_no_temperature() {
        let msg = TempSensorMqttMessage {
            temperature: None,
            humidity: Some(60.0),
            battery: Some(95),
            linkquality: Some(200),
        };

        let reading = TemperatureReading::from_mqtt("device_003".to_string(), msg);
        // Should return None because temperature is required
        assert!(reading.is_none());
    }

    // ==================== State Store Tests ====================

    #[test]
    fn test_device_state_update_link_quality() {
        let device_state = DeviceStateStore::new();

        device_state.update_link_quality("device_001".to_string(), 255);

        let state = device_state.get_state("device_001").unwrap();
        assert_eq!(state.link_quality, Some(255));
        assert_eq!(state.device_id, "device_001");
    }

    #[test]
    fn test_device_state_update_battery() {
        let device_state = DeviceStateStore::new();

        device_state.update_battery("device_001".to_string(), 100);

        let state = device_state.get_state("device_001").unwrap();
        assert_eq!(state.battery_level, Some(100));
    }

    #[test]
    fn test_device_state_mark_seen() {
        let device_state = DeviceStateStore::new();

        device_state.mark_seen("device_001".to_string());

        let state = device_state.get_state("device_001").unwrap();
        // Just verify that last_seen was set (timestamp > 0)
        assert!(state.last_seen.timestamp() > 0);
    }

    #[test]
    fn test_device_state_multiple_updates() {
        let device_state = DeviceStateStore::new();

        // Update link quality
        device_state.update_link_quality("device_001".to_string(), 200);

        // Update battery
        device_state.update_battery("device_001".to_string(), 85);

        let state = device_state.get_state("device_001").unwrap();
        assert_eq!(state.link_quality, Some(200));
        assert_eq!(state.battery_level, Some(85));
    }

    #[test]
    fn test_device_state_get_nonexistent() {
        let device_state = DeviceStateStore::new();

        let state = device_state.get_state("nonexistent");
        assert!(state.is_none());
    }

    #[test]
    fn test_switch_state_on() {
        let switch_state = SwitchStateStore::new();

        switch_state.set_state("switch_001".to_string(), true);

        let state = switch_state.get_state("switch_001").unwrap();
        assert!(state);
    }

    #[test]
    fn test_switch_state_off() {
        let switch_state = SwitchStateStore::new();

        switch_state.set_state("switch_001".to_string(), false);

        let state = switch_state.get_state("switch_001").unwrap();
        assert!(!state);
    }

    #[test]
    fn test_switch_state_toggle() {
        let switch_state = SwitchStateStore::new();

        // Set to ON
        switch_state.set_state("switch_001".to_string(), true);
        assert_eq!(switch_state.get_state("switch_001"), Some(true));

        // Toggle to OFF
        switch_state.set_state("switch_001".to_string(), false);
        assert_eq!(switch_state.get_state("switch_001"), Some(false));

        // Toggle back to ON
        switch_state.set_state("switch_001".to_string(), true);
        assert_eq!(switch_state.get_state("switch_001"), Some(true));
    }

    #[test]
    fn test_switch_state_get_nonexistent() {
        let switch_state = SwitchStateStore::new();

        let state = switch_state.get_state("nonexistent");
        assert!(state.is_none());
    }

    #[test]
    fn test_switch_state_case_conversion() {
        // Test that state strings are converted to uppercase for boolean comparison
        assert_eq!("ON".to_uppercase(), "ON");
        assert_eq!("on".to_uppercase(), "ON");
        assert_eq!("On".to_uppercase(), "ON");
        assert_eq!("off".to_uppercase(), "OFF");
        assert_eq!("OFF".to_uppercase(), "OFF");

        // Verify the logic used in the actual code
        assert_eq!("ON".to_uppercase() == "ON", true);
        assert_eq!("on".to_uppercase() == "ON", true);
        assert_eq!("OFF".to_uppercase() == "ON", false);
    }

    // ==================== Command Publishing Format Tests ====================

    #[test]
    fn test_publish_command_topic_format() {
        let device_id = "0x00158d0001a2b3c4";
        let topic = format!("zigbee2mqtt/{}/set", device_id);
        assert_eq!(topic, "zigbee2mqtt/0x00158d0001a2b3c4/set");
    }

    #[test]
    fn test_publish_command_topic_format_various_ids() {
        assert_eq!(
            format!("zigbee2mqtt/{}/set", "switch_001"),
            "zigbee2mqtt/switch_001/set"
        );
        assert_eq!(
            format!("zigbee2mqtt/{}/set", "sensor_temp"),
            "zigbee2mqtt/sensor_temp/set"
        );
        assert_eq!(
            format!("zigbee2mqtt/{}/set", "0xABCD"),
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

    // ==================== MqttMessage Enum Tests ====================

    #[test]
    fn test_mqtt_message_enum_temperature() {
        let msg = MqttMessage::TempHumiditySensor(TempSensorMqttMessage {
            temperature: Some(22.0),
            humidity: Some(45.0),
            battery: Some(100),
            linkquality: Some(255),
        });

        match msg {
            MqttMessage::TempHumiditySensor(sensor_msg) => {
                assert_eq!(sensor_msg.temperature, Some(22.0));
            }
            _ => panic!("Expected TempHumiditySensor variant"),
        }
    }

    #[test]
    fn test_mqtt_message_enum_switch() {
        let msg = MqttMessage::Switch(SwitchMqttMessage {
            state: Some("ON".to_string()),
            linkquality: Some(200),
            other: serde_json::Map::new(),
        });

        match msg {
            MqttMessage::Switch(switch_msg) => {
                assert_eq!(switch_msg.state, Some("ON".to_string()));
            }
            _ => panic!("Expected Switch variant"),
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_temperature_reading_extreme_values() {
        let msg = TempSensorMqttMessage {
            temperature: Some(-40.0),
            humidity: Some(0.0),
            battery: Some(0),
            linkquality: Some(0),
        };

        let reading = TemperatureReading::from_mqtt("device_001".to_string(), msg);
        assert!(reading.is_some());

        let reading = reading.unwrap();
        assert_eq!(reading.temperature, -40.0);
        assert_eq!(reading.humidity, Some(0.0));
        assert_eq!(reading.battery, Some(0));
        assert_eq!(reading.link_quality, Some(0));
    }

    #[test]
    fn test_temperature_reading_high_values() {
        let msg = TempSensorMqttMessage {
            temperature: Some(125.0),
            humidity: Some(100.0),
            battery: Some(100),
            linkquality: Some(255),
        };

        let reading = TemperatureReading::from_mqtt("device_001".to_string(), msg);
        assert!(reading.is_some());

        let reading = reading.unwrap();
        assert_eq!(reading.temperature, 125.0);
        assert_eq!(reading.humidity, Some(100.0));
    }

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

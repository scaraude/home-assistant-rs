#[cfg(test)]
mod tests {
    use super::super::connection::{MutexExt, Transaction};
    use super::super::queries::utils::timestamp_to_datetime;
    use super::super::*;
    use crate::models::{
        AutomationAction, AutomationCondition, AutomationExecutionLog, AutomationRule,
        CommanderType, ComparisonOperator, Device, DeviceCapability, LogicalOperator, PowerSource,
        SensorField, SensorReading, SensorType, SwitchAction, TimeWindow,
    };
    use chrono::Utc;
    use tempfile::TempDir;

    /// Helper to create a test database in a temporary directory
    fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();
        (db, temp_dir)
    }

    /// Helper to create and insert a test device
    fn insert_test_device(db: &Database, device_id: &str, sensor_type: SensorType) {
        let device = Device {
            id: device_id.to_string(),
            name: format!("Test Device {}", device_id),
            mqtt_topic: format!("zigbee2mqtt/{}", device_id),
            ieee_addr: format!("0x{}", device_id),
            capabilities: vec![DeviceCapability::Sensor { sensor_type }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device).unwrap();
    }

    // ==================== Temperature Reading Tests ====================

    #[test]
    fn test_insert_and_get_temperature_reading() {
        let (db, _temp_dir) = create_test_db();

        let timestamp = Utc::now();
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };

        // Insert reading
        db.insert_reading(&reading).unwrap();

        // Retrieve readings
        let readings = db.get_readings_since(timestamp.timestamp() - 1).unwrap();
        assert_eq!(readings.len(), 1);

        // Pattern match to verify the reading
        match &readings[0] {
            SensorReading::TempHumidity {
                device_id,
                temperature,
                humidity,
                ..
            } => {
                assert_eq!(device_id, "sensor1");
                assert_eq!(*temperature, 22.5);
                assert_eq!(*humidity, 45.0);
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_get_readings_for_specific_sensor() {
        let (db, _temp_dir) = create_test_db();

        let timestamp = Utc::now();
        let reading1 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };

        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor2".to_string(),
            temperature: 18.0,
            humidity: 60.0,
            timestamp,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();

        // Get readings for sensor1 only
        let readings = db
            .get_readings_for_sensor_since("sensor1", timestamp.timestamp() - 1)
            .unwrap();
        assert_eq!(readings.len(), 1);

        match &readings[0] {
            SensorReading::TempHumidity {
                device_id,
                temperature,
                ..
            } => {
                assert_eq!(device_id, "sensor1");
                assert_eq!(*temperature, 22.5);
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_get_latest_reading_for_sensor() {
        let (db, _temp_dir) = create_test_db();

        let now = Utc::now();
        let reading1 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 20.0,
            humidity: 40.0,
            timestamp: now - chrono::Duration::seconds(10),
        };

        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp: now,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();

        // Should get the latest reading (reading2)
        let latest = db.get_latest_reading_for_sensor("sensor1").unwrap();
        assert!(latest.is_some());

        match latest.unwrap() {
            SensorReading::TempHumidity {
                temperature,
                humidity,
                ..
            } => {
                assert_eq!(temperature, 22.5);
                assert_eq!(humidity, 45.0);
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_get_latest_reading_for_nonexistent_sensor() {
        let (db, _temp_dir) = create_test_db();

        let latest = db.get_latest_reading_for_sensor("nonexistent").unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn test_get_all_sensor_devices() {
        let (db, _temp_dir) = create_test_db();

        // Insert devices first (required for INNER JOIN)
        let device1 = Device {
            id: "sensor1".to_string(),
            mqtt_topic: "zigbee2mqtt/sensor1".to_string(),
            ieee_addr: "0x0000000000000001".to_string(),
            name: "sensor1".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        let device2 = Device {
            id: "sensor2".to_string(),
            mqtt_topic: "zigbee2mqtt/sensor2".to_string(),
            ieee_addr: "0x0000000000000002".to_string(),
            name: "sensor2".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device1).unwrap();
        db.insert_device(&device2).unwrap();

        // Insert readings from multiple sensors
        let timestamp = Utc::now();
        let reading1 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 50.0,
            timestamp,
        };

        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor2".to_string(),
            temperature: 18.0,
            humidity: 60.0,
            timestamp,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();

        let sensors = db.get_all_sensor_devices().unwrap();
        assert_eq!(sensors.len(), 2);
    }

    #[test]
    fn test_temperature_reading_with_required_fields() {
        let (db, _temp_dir) = create_test_db();

        let timestamp = Utc::now();
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 50.0, // humidity is now required
            timestamp,
        };

        db.insert_reading(&reading).unwrap();

        let readings = db.get_readings_since(timestamp.timestamp() - 1).unwrap();
        assert_eq!(readings.len(), 1);

        match &readings[0] {
            SensorReading::TempHumidity { humidity, .. } => {
                assert_eq!(*humidity, 50.0); // Verify humidity is stored
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    // ==================== Energy Reading Tests ====================

    #[test]
    fn test_insert_energy_reading() {
        let (db, _temp_dir) = create_test_db();

        // Create device first
        let device = Device {
            id: "energy_meter1".to_string(),
            mqtt_topic: "zigbee2mqtt/energy_meter1".to_string(),
            ieee_addr: "zigbee2mqtt/energy_meter1".to_string(),
            name: "Test Energy Meter".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::EnergyMeter,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device).unwrap();

        let timestamp = Utc::now();
        let reading = SensorReading::EnergyMeter {
            device_id: "energy_meter1".to_string(),
            power: 1500.0,
            energy: 10.5,
            produced_energy: 0.0,
            voltage: 230.0,
            current: 6.5,
            ac_frequency: 50.0,
            power_factor: 0.98,
            timestamp,
        };

        // Insert reading
        db.insert_reading(&reading).unwrap();

        // Retrieve readings
        let readings = db.get_readings_since(timestamp.timestamp() - 1).unwrap();
        assert_eq!(readings.len(), 1);

        // Pattern match to verify the reading
        match &readings[0] {
            SensorReading::EnergyMeter {
                device_id,
                power,
                energy,
                voltage,
                current,
                ..
            } => {
                assert_eq!(device_id, "energy_meter1");
                assert_eq!(*power, 1500.0);
                assert_eq!(*energy, 10.5);
                assert_eq!(*voltage, 230.0);
                assert_eq!(*current, 6.5);
            }
            _ => panic!("Expected EnergyMeter reading"),
        }
    }

    #[test]
    fn test_get_energy_readings_for_sensor() {
        let (db, _temp_dir) = create_test_db();

        // Create devices first
        let device1 = Device {
            id: "energy_meter1".to_string(),
            mqtt_topic: "zigbee2mqtt/energy_meter1".to_string(),
            ieee_addr: "zigbee2mqtt/energy_meter1".to_string(),
            name: "Test Energy Meter 1".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::EnergyMeter,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device1).unwrap();

        let device2 = Device {
            id: "energy_meter2".to_string(),
            mqtt_topic: "zigbee2mqtt/energy_meter2".to_string(),
            ieee_addr: "zigbee2mqtt/energy_meter2".to_string(),
            name: "Test Energy Meter 2".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::EnergyMeter,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device2).unwrap();

        let timestamp = Utc::now();
        let reading1 = SensorReading::EnergyMeter {
            device_id: "energy_meter1".to_string(),
            power: 2000.0,
            energy: 15.0,
            produced_energy: 0.0,
            voltage: 235.0,
            current: 8.5,
            ac_frequency: 50.0,
            power_factor: 0.99,
            timestamp,
        };

        let reading2 = SensorReading::EnergyMeter {
            device_id: "energy_meter2".to_string(),
            power: 500.0,
            energy: 5.0,
            produced_energy: 0.0,
            voltage: 230.0,
            current: 2.2,
            ac_frequency: 50.0,
            power_factor: 0.95,
            timestamp,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();

        // Get readings for energy_meter1 only
        let readings = db
            .get_readings_for_sensor_since("energy_meter1", timestamp.timestamp() - 1)
            .unwrap();
        assert_eq!(readings.len(), 1);

        match &readings[0] {
            SensorReading::EnergyMeter {
                device_id, power, ..
            } => {
                assert_eq!(device_id, "energy_meter1");
                assert_eq!(*power, 2000.0);
            }
            _ => panic!("Expected EnergyMeter reading"),
        }
    }

    #[test]
    fn test_get_latest_energy_reading() {
        let (db, _temp_dir) = create_test_db();

        // Create device first
        let device = Device {
            id: "energy_meter1".to_string(),
            mqtt_topic: "zigbee2mqtt/energy_meter1".to_string(),
            ieee_addr: "zigbee2mqtt/energy_meter1".to_string(),
            name: "Test Energy Meter".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::EnergyMeter,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device).unwrap();

        let now = Utc::now();
        let old_reading = SensorReading::EnergyMeter {
            device_id: "energy_meter1".to_string(),
            power: 1000.0,
            energy: 10.0,
            produced_energy: 0.0,
            voltage: 230.0,
            current: 4.3,
            ac_frequency: 50.0,
            power_factor: 0.97,
            timestamp: now - chrono::Duration::seconds(60),
        };

        let new_reading = SensorReading::EnergyMeter {
            device_id: "energy_meter1".to_string(),
            power: 1500.0,
            energy: 10.5,
            produced_energy: 0.0,
            voltage: 232.0,
            current: 6.5,
            ac_frequency: 50.0,
            power_factor: 0.98,
            timestamp: now,
        };

        db.insert_reading(&old_reading).unwrap();
        db.insert_reading(&new_reading).unwrap();

        // Get latest reading
        let latest = db.get_latest_reading_for_sensor("energy_meter1").unwrap();
        assert!(latest.is_some());

        match latest.unwrap() {
            SensorReading::EnergyMeter { power, .. } => {
                assert_eq!(power, 1500.0); // Should be the newer reading
            }
            _ => panic!("Expected EnergyMeter reading"),
        }
    }

    #[test]
    fn test_energy_readings_foreign_key_cascade() {
        let (db, _temp_dir) = create_test_db();

        // Create device first
        let device = Device {
            id: "energy_meter1".to_string(),
            mqtt_topic: "zigbee2mqtt/energy_meter1".to_string(),
            ieee_addr: "zigbee2mqtt/energy_meter1".to_string(),
            name: "Test Energy Meter".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::EnergyMeter,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        // Insert reading
        let timestamp = Utc::now();
        let reading = SensorReading::EnergyMeter {
            device_id: "energy_meter1".to_string(),
            power: 1500.0,
            energy: 10.5,
            produced_energy: 0.0,
            voltage: 230.0,
            current: 6.5,
            ac_frequency: 50.0,
            power_factor: 0.98,
            timestamp,
        };

        db.insert_reading(&reading).unwrap();

        // Verify reading exists
        let readings = db.get_readings_since(timestamp.timestamp() - 1).unwrap();
        assert_eq!(readings.len(), 1);

        // Delete device
        db._delete_device("energy_meter1").unwrap();

        // Verify reading was cascade deleted
        let readings = db.get_readings_since(timestamp.timestamp() - 1).unwrap();
        assert_eq!(readings.len(), 0);
    }

    // ==================== Device Tests ====================

    #[test]
    fn test_insert_and_get_device_by_id() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Living Room Sensor".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        let retrieved = db._get_device_by_id("device1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "device1");
        assert_eq!(retrieved.name, "Living Room Sensor");
        assert!(retrieved.has_sensor_type(&SensorType::TempHumidity));
        assert_eq!(retrieved.power_source, PowerSource::Battery);
    }

    #[test]
    fn test_get_device_by_mqtt_topic() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Living Room Sensor".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        let retrieved = db.get_device_by_mqtt_topic("zigbee2mqtt/device1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "device1");
    }

    #[test]
    fn test_get_all_devices() {
        let (db, _temp_dir) = create_test_db();

        let device1 = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Device 1".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        let device2 = Device {
            id: "device2".to_string(),
            mqtt_topic: "zigbee2mqtt/device2".to_string(),
            ieee_addr: "zigbee2mqtt/device2".to_string(),
            name: "Device 2".to_string(),
            capabilities: vec![DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device1).unwrap();
        db.insert_device(&device2).unwrap();

        let devices = db.get_all_devices().unwrap();
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn test_update_device_name() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Old Name".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();
        db.update_device_name("device1", "New Name").unwrap();

        let updated = db._get_device_by_id("device1").unwrap().unwrap();
        assert_eq!(updated.name, "New Name");
    }

    #[test]
    fn test_get_mqtt_topic_for_device() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Device 1".to_string(),
            capabilities: vec![DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        let mqtt_topic = db.get_mqtt_topic_for_device("device1").unwrap();
        assert!(mqtt_topic.is_some());
        assert_eq!(mqtt_topic.unwrap(), "zigbee2mqtt/device1");
    }

    #[test]
    fn test_get_mqtt_topic_for_nonexistent_device() {
        let (db, _temp_dir) = create_test_db();

        let mqtt_topic = db.get_mqtt_topic_for_device("nonexistent").unwrap();
        assert!(mqtt_topic.is_none());
    }

    #[test]
    fn test_get_all_sensor_devices_with_device_names() {
        let (db, _temp_dir) = create_test_db();

        // Insert device with custom name
        let device1 = Device {
            id: "sensor1".to_string(),
            mqtt_topic: "zigbee2mqtt/sensor1".to_string(),
            ieee_addr: "0x0000000000000001".to_string(),
            name: "Living Room Sensor".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device1).unwrap();

        // Insert device with default name (device_id)
        let device2 = Device {
            id: "sensor2".to_string(),
            mqtt_topic: "zigbee2mqtt/sensor2".to_string(),
            ieee_addr: "0x0000000000000002".to_string(),
            name: "sensor2".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };
        db.insert_device(&device2).unwrap();

        // Insert a reading for device1
        let timestamp = Utc::now();
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 50.0,
            timestamp,
        };
        db.insert_reading(&reading).unwrap();

        // Insert a reading for device2
        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor2".to_string(),
            temperature: 18.0,
            humidity: 60.0,
            timestamp,
        };
        db.insert_reading(&reading2).unwrap();

        let sensors = db.get_all_sensor_devices().unwrap();
        assert_eq!(sensors.len(), 2);

        // Check that sensor1 has the custom name
        let sensor1 = sensors.iter().find(|s| s.device_id == "sensor1").unwrap();
        assert_eq!(sensor1.name, "Living Room Sensor");
        assert!(sensor1
            .capabilities
            .iter()
            .any(|cap| matches!(cap, DeviceCapability::Sensor { sensor_type } if *sensor_type == SensorType::TempHumidity)));

        // Check that sensor2 has default name (device_id)
        let sensor2 = sensors.iter().find(|s| s.device_id == "sensor2").unwrap();
        assert_eq!(sensor2.name, "sensor2");
        assert!(sensor2
            .capabilities
            .iter()
            .any(|cap| matches!(cap, DeviceCapability::Sensor { sensor_type } if *sensor_type == SensorType::TempHumidity)));
    }

    // ==================== Automation Rule Tests ====================

    #[test]
    fn test_insert_and_get_automation_rule() {
        let (db, _temp_dir) = create_test_db();

        let rule = AutomationRule {
            id: "rule1".to_string(),
            name: "Test Rule".to_string(),
            description: Some("Test description".to_string()),
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![AutomationCondition {
                id: "cond1".to_string(),
                device_id: "sensor1".to_string(),
                field: SensorField::Temperature,
                operator: ComparisonOperator::GreaterThan,
                value: 25.0,
            }],
            actions: vec![AutomationAction {
                id: "action1".to_string(),
                device_id: "switch1".to_string(),
                action: SwitchAction::On,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule).unwrap();

        let retrieved = db.get_automation_rule("rule1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Rule");
        assert_eq!(retrieved.conditions.len(), 1);
        assert_eq!(retrieved.actions.len(), 1);
        assert_eq!(retrieved.conditions[0].field, SensorField::Temperature);
        assert_eq!(retrieved.actions[0].action, SwitchAction::On);
    }

    #[test]
    fn test_get_all_automation_rules() {
        let (db, _temp_dir) = create_test_db();

        let rule1 = AutomationRule {
            id: "rule1".to_string(),
            name: "Rule 1".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        let rule2 = AutomationRule {
            id: "rule2".to_string(),
            name: "Rule 2".to_string(),
            description: None,
            enabled: false,
            condition_operator: LogicalOperator::Or,
            conditions: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule1).unwrap();
        db.insert_automation_rule(&rule2).unwrap();

        let rules = db.get_all_automation_rules().unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_update_automation_rule() {
        let (db, _temp_dir) = create_test_db();

        let mut rule = AutomationRule {
            id: "rule1".to_string(),
            name: "Original Name".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule).unwrap();

        // Update the rule
        rule.name = "Updated Name".to_string();
        rule.enabled = false;
        rule.conditions.push(AutomationCondition {
            id: "cond1".to_string(),
            device_id: "sensor1".to_string(),
            field: SensorField::Humidity,
            operator: ComparisonOperator::LessThan,
            value: 30.0,
        });

        db.update_automation_rule(&rule).unwrap();

        let updated = db.get_automation_rule("rule1").unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert!(!updated.enabled);
        assert_eq!(updated.conditions.len(), 1);
    }

    #[test]
    fn test_delete_automation_rule() {
        let (db, _temp_dir) = create_test_db();

        let rule = AutomationRule {
            id: "rule1".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![AutomationCondition {
                id: "cond1".to_string(),
                device_id: "sensor1".to_string(),
                field: SensorField::Temperature,
                operator: ComparisonOperator::GreaterThan,
                value: 25.0,
            }],
            actions: vec![AutomationAction {
                id: "action1".to_string(),
                device_id: "switch1".to_string(),
                action: SwitchAction::On,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule).unwrap();
        db.delete_automation_rule("rule1").unwrap();

        let retrieved = db.get_automation_rule("rule1").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_record_rule_trigger() {
        let (db, _temp_dir) = create_test_db();

        let rule = AutomationRule {
            id: "rule1".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule).unwrap();

        // Record trigger
        db.record_rule_trigger("rule1").unwrap();

        let updated = db.get_automation_rule("rule1").unwrap().unwrap();
        assert!(updated.last_triggered_at.is_some());
        assert_eq!(updated.trigger_count, 1);

        // Record another trigger
        db.record_rule_trigger("rule1").unwrap();

        let updated = db.get_automation_rule("rule1").unwrap().unwrap();
        assert_eq!(updated.trigger_count, 2);
    }

    #[test]
    fn test_insert_and_get_execution_logs() {
        let (db, _temp_dir) = create_test_db();

        let now = Utc::now();
        let log1 = AutomationExecutionLog {
            id: "log1".to_string(),
            rule_id: "rule1".to_string(),
            rule_name: "Test Rule".to_string(),
            success: true,
            error_message: None,
            executed_at: now - chrono::Duration::seconds(5),
        };

        let log2 = AutomationExecutionLog {
            id: "log2".to_string(),
            rule_id: "rule1".to_string(),
            rule_name: "Test Rule".to_string(),
            success: false,
            error_message: Some("Test error".to_string()),
            executed_at: now,
        };

        db.insert_execution_log(&log1).unwrap();
        db.insert_execution_log(&log2).unwrap();

        let logs = db.get_execution_logs(10).unwrap();
        assert_eq!(logs.len(), 2);

        // Verify order (most recent first)
        assert_eq!(logs[0].id, "log2");
        assert_eq!(logs[1].id, "log1");

        // Verify error message
        assert!(!logs[0].success);
        assert_eq!(logs[0].error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_execution_logs_limit() {
        let (db, _temp_dir) = create_test_db();

        // Insert 5 logs
        for i in 0..5 {
            let log = AutomationExecutionLog {
                id: format!("log{}", i),
                rule_id: "rule1".to_string(),
                rule_name: "Test Rule".to_string(),
                success: true,
                error_message: None,
                executed_at: Utc::now(),
            };
            db.insert_execution_log(&log).unwrap();
        }

        // Request only 3
        let logs = db.get_execution_logs(3).unwrap();
        assert_eq!(logs.len(), 3);
    }

    // ==================== Transaction Tests ====================

    #[test]
    fn test_transaction_commit() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        let conn = db.conn.lock_or_recover();
        let tx = Transaction::begin(&conn).unwrap();

        conn.execute(
            "INSERT INTO devices (id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at)
             VALUES ('test', 'topic', 'topic', 'name', '[{\"type\":\"sensor\",\"sensor_type\":\"temp_humidity\"}]', '[]', 'battery', 0)",
            [],
        )
        .unwrap();

        tx.commit().unwrap();
        drop(conn);

        // Verify data was committed
        let device = db._get_device_by_id("test").unwrap();
        assert!(device.is_some());
    }

    #[test]
    fn test_transaction_rollback_on_drop() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();

        {
            let conn = db.conn.lock_or_recover();
            let _tx = Transaction::begin(&conn).unwrap();

            conn.execute(
                "INSERT INTO devices (id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at)
                 VALUES ('test', 'topic', 'topic', 'name', '[{\"type\":\"sensor\",\"sensor_type\":\"temp_humidity\"}]', '[]', 'battery', 0)",
                [],
            )
            .unwrap();

            // Transaction is dropped without commit
        }

        // Verify data was rolled back
        let device = db._get_device_by_id("test").unwrap();
        assert!(device.is_none());
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_insert_duplicate_device_id() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Device 1".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        // Try to insert with same ID but different topic
        let duplicate = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/different".to_string(),
            ieee_addr: "zigbee2mqtt/different".to_string(),
            name: "Different".to_string(),
            capabilities: vec![DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        let result = db.insert_device(&duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_duplicate_mqtt_topic() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Device 1".to_string(),
            capabilities: vec![DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        db.insert_device(&device).unwrap();

        // Try to insert different ID with same topic
        let duplicate = Device {
            id: "device2".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            ieee_addr: "zigbee2mqtt/device1".to_string(),
            name: "Different".to_string(),
            capabilities: vec![DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            }],
            available_fields: Vec::new(),
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
            is_bridge: false,
            parent_device_id: None,
            color: None,
        };

        let result = db.insert_device(&duplicate);
        assert!(result.is_err());
    }

    // ==================== Database Initialization Tests ====================

    #[test]
    fn test_database_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir
            .path()
            .join("nested")
            .join("directory")
            .join("test.db");

        let _db = Database::new(&db_path).unwrap();

        // Verify directory was created
        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn test_database_schema_initialization() {
        let (db, _temp_dir) = create_test_db();

        // Verify tables exist by querying them (will fail if tables don't exist)
        let conn = db.conn.lock_or_recover();

        // Check temperature_readings table
        conn.query_row("SELECT COUNT(*) FROM temperature_readings", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

        // Check devices table
        conn.query_row("SELECT COUNT(*) FROM devices", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

        // Check automation_rules table
        conn.query_row("SELECT COUNT(*) FROM automation_rules", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

        // Check automation_conditions table
        conn.query_row("SELECT COUNT(*) FROM automation_conditions", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

        // Check automation_actions table
        conn.query_row("SELECT COUNT(*) FROM automation_actions", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();

        // Check automation_execution_log table
        conn.query_row("SELECT COUNT(*) FROM automation_execution_log", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    }

    #[test]
    fn test_indexes_exist() {
        let (db, _temp_dir) = create_test_db();
        let conn = db.conn.lock_or_recover();

        // Check idx_sensor_time index
        let index_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_sensor_time'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_exists, 1);

        // Check idx_mqtt_topic index
        let index_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_mqtt_topic'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_exists, 1);

        // Check idx_ieee_addr index
        let index_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_ieee_addr'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_exists, 1);
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_timestamp_to_datetime_valid() {
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC
        let dt = timestamp_to_datetime(ts, "test");
        assert_eq!(dt.timestamp(), ts);
    }

    #[test]
    fn test_timestamp_to_datetime_invalid() {
        // Test with invalid timestamp (way out of range)
        let invalid_ts = i64::MAX;
        let dt = timestamp_to_datetime(invalid_ts, "test");
        // Should return Unix epoch as fallback
        assert_eq!(dt, chrono::DateTime::UNIX_EPOCH);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_readings_query() {
        let (db, _temp_dir) = create_test_db();

        let readings = db.get_readings_since(Utc::now().timestamp()).unwrap();
        assert_eq!(readings.len(), 0);
    }

    #[test]
    fn test_empty_devices_query() {
        let (db, _temp_dir) = create_test_db();

        let devices = db.get_all_devices().unwrap();
        assert_eq!(devices.len(), 0);
    }

    #[test]
    fn test_multiple_conditions_and_actions() {
        let (db, _temp_dir) = create_test_db();

        let rule = AutomationRule {
            id: "rule1".to_string(),
            name: "Complex Rule".to_string(),
            description: None,
            enabled: true,
            condition_operator: LogicalOperator::And,
            conditions: vec![
                AutomationCondition {
                    id: "cond1".to_string(),
                    device_id: "sensor1".to_string(),
                    field: SensorField::Temperature,
                    operator: ComparisonOperator::GreaterThan,
                    value: 25.0,
                },
                AutomationCondition {
                    id: "cond2".to_string(),
                    device_id: "sensor2".to_string(),
                    field: SensorField::Humidity,
                    operator: ComparisonOperator::LessThan,
                    value: 40.0,
                },
            ],
            actions: vec![
                AutomationAction {
                    id: "action1".to_string(),
                    device_id: "switch1".to_string(),
                    action: SwitchAction::On,
                },
                AutomationAction {
                    id: "action2".to_string(),
                    device_id: "switch2".to_string(),
                    action: SwitchAction::Off,
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            time_window: TimeWindow::default(),
        };

        db.insert_automation_rule(&rule).unwrap();

        let retrieved = db.get_automation_rule("rule1").unwrap().unwrap();
        assert_eq!(retrieved.conditions.len(), 2);
        assert_eq!(retrieved.actions.len(), 2);
    }

    // ==================== Batch Query Tests (CRIT-2) ====================

    #[test]
    fn test_batch_query_empty_device_list() {
        let (db, _temp_dir) = create_test_db();

        let device_ids = vec![];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_batch_query_single_temperature_device() {
        let (db, _temp_dir) = create_test_db();

        // Register device first
        insert_test_device(&db, "sensor1", SensorType::TempHumidity);

        // Insert a temperature reading
        let timestamp = Utc::now();
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };
        db.insert_reading(&reading).unwrap();

        // Query using batch method
        let device_ids = vec!["sensor1".to_string()];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("sensor1"));

        match results.get("sensor1").unwrap() {
            SensorReading::TempHumidity {
                device_id,
                temperature,
                humidity,
                ..
            } => {
                assert_eq!(device_id, "sensor1");
                assert_eq!(*temperature, 22.5);
                assert_eq!(*humidity, 45.0);
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_batch_query_multiple_temperature_devices() {
        let (db, _temp_dir) = create_test_db();

        // Register devices first
        insert_test_device(&db, "sensor1", SensorType::TempHumidity);
        insert_test_device(&db, "sensor2", SensorType::TempHumidity);
        insert_test_device(&db, "sensor3", SensorType::TempHumidity);

        let timestamp = Utc::now();

        // Insert readings for multiple devices
        let reading1 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };

        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor2".to_string(),
            temperature: 18.0,
            humidity: 60.0,
            timestamp,
        };

        let reading3 = SensorReading::TempHumidity {
            device_id: "sensor3".to_string(),
            temperature: 25.0,
            humidity: 35.0,
            timestamp,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();
        db.insert_reading(&reading3).unwrap();

        // Query all three devices in a single batch
        let device_ids = vec![
            "sensor1".to_string(),
            "sensor2".to_string(),
            "sensor3".to_string(),
        ];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.contains_key("sensor1"));
        assert!(results.contains_key("sensor2"));
        assert!(results.contains_key("sensor3"));

        // Verify sensor1
        match results.get("sensor1").unwrap() {
            SensorReading::TempHumidity { temperature, .. } => {
                assert_eq!(*temperature, 22.5);
            }
            _ => panic!("Expected TempHumidity reading"),
        }

        // Verify sensor2
        match results.get("sensor2").unwrap() {
            SensorReading::TempHumidity { temperature, .. } => {
                assert_eq!(*temperature, 18.0);
            }
            _ => panic!("Expected TempHumidity reading"),
        }

        // Verify sensor3
        match results.get("sensor3").unwrap() {
            SensorReading::TempHumidity { temperature, .. } => {
                assert_eq!(*temperature, 25.0);
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_batch_query_mixed_sensor_types() {
        let (db, _temp_dir) = create_test_db();

        // Register devices first
        insert_test_device(&db, "temp_sensor", SensorType::TempHumidity);
        insert_test_device(&db, "presence_sensor", SensorType::Presence);
        insert_test_device(&db, "energy_sensor", SensorType::EnergyMeter);

        let timestamp = Utc::now();

        // Insert temperature reading
        let temp_reading = SensorReading::TempHumidity {
            device_id: "temp_sensor".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };

        // Insert presence reading
        let presence_reading = SensorReading::Presence {
            device_id: "presence_sensor".to_string(),
            occupied: true,
            illumination: Some("bright".to_string()),
            timestamp,
        };

        // Insert energy reading
        let energy_reading = SensorReading::EnergyMeter {
            device_id: "energy_sensor".to_string(),
            power: 150.0,
            energy: 1000.0,
            produced_energy: 0.0,
            voltage: 230.0,
            current: 0.65,
            ac_frequency: 50.0,
            power_factor: 0.95,
            timestamp,
        };

        db.insert_reading(&temp_reading).unwrap();
        db.insert_reading(&presence_reading).unwrap();
        db.insert_reading(&energy_reading).unwrap();

        // Query all three different sensor types in a single batch
        let device_ids = vec![
            "temp_sensor".to_string(),
            "presence_sensor".to_string(),
            "energy_sensor".to_string(),
        ];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        assert_eq!(results.len(), 3);

        // Verify temperature reading
        match results.get("temp_sensor").unwrap() {
            SensorReading::TempHumidity { temperature, .. } => {
                assert_eq!(*temperature, 22.5);
            }
            _ => panic!("Expected TempHumidity reading"),
        }

        // Verify presence reading
        match results.get("presence_sensor").unwrap() {
            SensorReading::Presence { occupied, .. } => {
                assert!(*occupied);
            }
            _ => panic!("Expected Presence reading"),
        }

        // Verify energy reading
        match results.get("energy_sensor").unwrap() {
            SensorReading::EnergyMeter { power, .. } => {
                assert_eq!(*power, 150.0);
            }
            _ => panic!("Expected EnergyMeter reading"),
        }
    }

    #[test]
    fn test_batch_query_returns_latest_reading_only() {
        let (db, _temp_dir) = create_test_db();

        // Register device first
        insert_test_device(&db, "sensor1", SensorType::TempHumidity);

        let timestamp1 = Utc::now();
        let timestamp2 = timestamp1 + chrono::Duration::seconds(10);
        let timestamp3 = timestamp2 + chrono::Duration::seconds(10);

        // Insert multiple readings for the same device at different times
        let reading1 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 20.0,
            humidity: 50.0,
            timestamp: timestamp1,
        };

        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.0,
            humidity: 48.0,
            timestamp: timestamp2,
        };

        let reading3 = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 24.0,
            humidity: 45.0,
            timestamp: timestamp3,
        };

        db.insert_reading(&reading1).unwrap();
        db.insert_reading(&reading2).unwrap();
        db.insert_reading(&reading3).unwrap();

        // Query should return only the latest reading
        let device_ids = vec!["sensor1".to_string()];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        assert_eq!(results.len(), 1);

        match results.get("sensor1").unwrap() {
            SensorReading::TempHumidity {
                temperature,
                timestamp,
                ..
            } => {
                // Should be the most recent reading
                assert_eq!(*temperature, 24.0);
                // SQLite stores timestamps as seconds, so we compare timestamps truncated to seconds
                assert_eq!(timestamp.timestamp(), timestamp3.timestamp());
            }
            _ => panic!("Expected TempHumidity reading"),
        }
    }

    #[test]
    fn test_batch_query_partial_results() {
        let (db, _temp_dir) = create_test_db();

        // Register only one device
        insert_test_device(&db, "sensor1", SensorType::TempHumidity);

        let timestamp = Utc::now();

        // Insert reading for only one device
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 45.0,
            timestamp,
        };
        db.insert_reading(&reading).unwrap();

        // Query for multiple devices, but only one exists
        let device_ids = vec![
            "sensor1".to_string(),
            "sensor2".to_string(),
            "sensor3".to_string(),
        ];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        // Should only return results for devices that have readings
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("sensor1"));
        assert!(!results.contains_key("sensor2"));
        assert!(!results.contains_key("sensor3"));
    }

    #[test]
    fn test_batch_query_no_results() {
        let (db, _temp_dir) = create_test_db();

        // Query for devices that don't exist
        let device_ids = vec!["nonexistent1".to_string(), "nonexistent2".to_string()];
        let results = db.get_latest_readings_batch(&device_ids).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_batch_query_performance_benefit() {
        let (db, _temp_dir) = create_test_db();

        // Register devices first
        for i in 1..=5 {
            insert_test_device(&db, &format!("sensor{}", i), SensorType::TempHumidity);
        }

        let timestamp = Utc::now();

        // Insert readings for 5 devices
        for i in 1..=5 {
            let reading = SensorReading::TempHumidity {
                device_id: format!("sensor{}", i),
                temperature: 20.0 + i as f32,
                humidity: 40.0 + i as f32,
                timestamp,
            };
            db.insert_reading(&reading).unwrap();
        }

        let device_ids: Vec<String> = (1..=5).map(|i| format!("sensor{}", i)).collect();

        // Time the batch query
        let start = std::time::Instant::now();
        let batch_results = db.get_latest_readings_batch(&device_ids).unwrap();
        let batch_duration = start.elapsed();

        // Time individual queries
        let start = std::time::Instant::now();
        let mut individual_results = std::collections::HashMap::new();
        for device_id in &device_ids {
            if let Ok(Some(reading)) = db.get_latest_reading_for_sensor(device_id) {
                individual_results.insert(device_id.clone(), reading);
            }
        }
        let individual_duration = start.elapsed();

        // Verify same results
        assert_eq!(batch_results.len(), 5);
        assert_eq!(individual_results.len(), 5);

        // Batch query should be faster (this is informational, not a hard assertion)
        println!(
            "Batch query: {:?}, Individual queries: {:?}, Speedup: {:.2}x",
            batch_duration,
            individual_duration,
            individual_duration.as_micros() as f64 / batch_duration.as_micros() as f64
        );

        // Note: We don't assert on performance as it can vary based on system load,
        // but this test documents the expected performance benefit
    }
}

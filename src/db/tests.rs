#[cfg(test)]
mod tests {
    use super::super::queries::utils::timestamp_to_datetime;
    use super::super::*;
    use crate::models::{
        AutomationAction, AutomationCondition, AutomationExecutionLog, AutomationRule,
        CommanderType, ComparisonOperator, Device, DeviceCapability, LogicalOperator, PowerSource,
        SensorField, SensorReading, SensorType, SwitchAction,
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
    fn test_get_all_sensors() {
        let (db, _temp_dir) = create_test_db();

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

        let sensors = db.get_all_sensors().unwrap();
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

    // ==================== Device Tests ====================

    #[test]
    fn test_insert_and_get_device_by_id() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            name: "Living Room Sensor".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
        };

        db.insert_device(&device).unwrap();

        let retrieved = db._get_device_by_id("device1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "device1");
        assert_eq!(retrieved.name, "Living Room Sensor");
        assert!(matches!(
            retrieved.capability,
            DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity
            }
        ));
        assert_eq!(retrieved.power_source, PowerSource::Battery);
    }

    #[test]
    fn test_get_device_by_mqtt_topic() {
        let (db, _temp_dir) = create_test_db();

        let device = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            name: "Living Room Sensor".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
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
            name: "Device 1".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
        };

        let device2 = Device {
            id: "device2".to_string(),
            mqtt_topic: "zigbee2mqtt/device2".to_string(),
            name: "Device 2".to_string(),
            capability: DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            },
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
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
            name: "Old Name".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
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
            name: "Device 1".to_string(),
            capability: DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            },
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
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
    fn test_get_all_sensors_with_device_names() {
        let (db, _temp_dir) = create_test_db();

        // Insert a device
        let device = Device {
            id: "sensor1".to_string(),
            mqtt_topic: "zigbee2mqtt/sensor1".to_string(),
            name: "Living Room Sensor".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
        };
        db.insert_device(&device).unwrap();

        // Insert a reading for this device
        let timestamp = Utc::now();
        let reading = SensorReading::TempHumidity {
            device_id: "sensor1".to_string(),
            temperature: 22.5,
            humidity: 50.0,
            timestamp,
        };
        db.insert_reading(&reading).unwrap();

        // Insert a reading for a device without a name
        let reading2 = SensorReading::TempHumidity {
            device_id: "sensor2".to_string(),
            temperature: 18.0,
            humidity: 60.0,
            timestamp,
        };
        db.insert_reading(&reading2).unwrap();

        let sensors = db.get_all_sensors().unwrap();
        assert_eq!(sensors.len(), 2);

        // Check that sensor1 has the custom name
        let sensor1 = sensors.iter().find(|s| s.device_id == "sensor1").unwrap();
        assert_eq!(sensor1.name, "Living Room Sensor");

        // Check that sensor2 falls back to device_id
        let sensor2 = sensors.iter().find(|s| s.device_id == "sensor2").unwrap();
        assert_eq!(sensor2.name, "sensor2");
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
            "INSERT INTO devices (id, mqtt_topic, name, capability_type, capability_subtype, power_source, added_at)
             VALUES ('test', 'topic', 'name', 'sensor', 'temp_humidity', 'battery', 0)",
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
                "INSERT INTO devices (id, mqtt_topic, name, capability_type, capability_subtype, power_source, added_at)
                 VALUES ('test', 'topic', 'name', 'sensor', 'temp_humidity', 'battery', 0)",
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
            name: "Device 1".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
        };

        db.insert_device(&device).unwrap();

        // Try to insert with same ID but different topic
        let duplicate = Device {
            id: "device1".to_string(),
            mqtt_topic: "zigbee2mqtt/different".to_string(),
            name: "Different".to_string(),
            capability: DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            },
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
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
            name: "Device 1".to_string(),
            capability: DeviceCapability::Sensor {
                sensor_type: SensorType::TempHumidity,
            },
            power_source: PowerSource::Battery,
            added_at: Utc::now(),
        };

        db.insert_device(&device).unwrap();

        // Try to insert different ID with same topic
        let duplicate = Device {
            id: "device2".to_string(),
            mqtt_topic: "zigbee2mqtt/device1".to_string(),
            name: "Different".to_string(),
            capability: DeviceCapability::Commander {
                commander_type: CommanderType::Switch,
            },
            power_source: PowerSource::Plugged,
            added_at: Utc::now(),
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
        };

        db.insert_automation_rule(&rule).unwrap();

        let retrieved = db.get_automation_rule("rule1").unwrap().unwrap();
        assert_eq!(retrieved.conditions.len(), 2);
        assert_eq!(retrieved.actions.len(), 2);
    }
}

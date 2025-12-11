use crate::models::{
    AutomationAction, AutomationCondition, AutomationExecutionLog, AutomationRule,
    ComparisonOperator, Device, DeviceInfo, DeviceType, LogicalOperator, PowerSource, SensorField,
    SwitchAction, TemperatureReading,
};
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create or open the database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        info!(path = %path_str, "Initializing database connection");

        // Ensure the parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            let parent_str = parent.display().to_string();
            debug!(parent_dir = %parent_str, "Checking parent directory");

            match fs::create_dir_all(parent) {
                Ok(_) => debug!(parent_dir = %parent_str, "Parent directory exists or created"),
                Err(e) => {
                    error!(error = %e, parent_dir = %parent_str, "Failed to create parent directory");
                    return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e)));
                }
            }
        }

        debug!(path = %path_str, "Opening database connection");
        let conn = match Connection::open(&path) {
            Ok(c) => {
                info!(path = %path_str, "Database connection opened successfully");
                c
            }
            Err(e) => {
                error!(error = %e, path = %path_str, "Failed to open database connection");
                return Err(e);
            }
        };

        let db = Self {
            conn: Mutex::new(conn),
        };

        info!("Initializing database schema");
        db.init_schema()?;

        info!(path = %path_str, "Database initialization complete");
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        debug!("Acquiring database lock for schema initialization");
        let conn = self.conn.lock().unwrap();

        debug!("Creating temperature_readings table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS temperature_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL,
                battery INTEGER,
                link_quality INTEGER,
                timestamp INTEGER NOT NULL
            )",
            [],
        ) {
            Ok(_) => debug!("Temperature readings table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create temperature_readings table");
                return Err(e);
            }
        }

        // Index for efficient queries by sensor and time
        debug!("Creating index idx_sensor_time if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sensor_time
             ON temperature_readings(device_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_sensor_time created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_sensor_time");
                return Err(e);
            }
        }

        // Migration: Add link_quality column if it doesn't exist
        debug!("Checking if link_quality column exists");
        let column_exists: Result<i32, _> = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('temperature_readings') WHERE name='link_quality'",
            [],
            |row| row.get(0),
        );

        match column_exists {
            Ok(0) => {
                info!("link_quality column does not exist, adding it");
                match conn.execute(
                    "ALTER TABLE temperature_readings ADD COLUMN link_quality INTEGER",
                    [],
                ) {
                    Ok(_) => info!("Successfully added link_quality column"),
                    Err(e) => {
                        error!(error = %e, "Failed to add link_quality column");
                        return Err(e);
                    }
                }
            }
            Ok(_) => debug!("link_quality column already exists"),
            Err(e) => {
                error!(error = %e, "Failed to check if link_quality column exists");
                return Err(e);
            }
        }

        // Create devices table
        debug!("Creating devices table if not exists");
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                mqtt_topic TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                device_type TEXT NOT NULL,
                power_source TEXT NOT NULL,
                added_at INTEGER NOT NULL
            )",
            [],
        ) {
            Ok(_) => debug!("Devices table created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create devices table");
                return Err(e);
            }
        }

        // Create index on mqtt_topic for fast lookups
        debug!("Creating index idx_mqtt_topic if not exists");
        match conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_mqtt_topic ON devices(mqtt_topic)",
            [],
        ) {
            Ok(_) => debug!("Index idx_mqtt_topic created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_mqtt_topic");
                return Err(e);
            }
        }

        // Create automation_rules table
        debug!("Creating automation_rules table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                enabled INTEGER DEFAULT 1,
                condition_operator TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_triggered_at INTEGER,
                trigger_count INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Create automation_conditions table
        debug!("Creating automation_conditions table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_conditions (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                field TEXT NOT NULL,
                operator TEXT NOT NULL,
                value REAL NOT NULL,
                FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_condition_rule
             ON automation_conditions(rule_id)",
            [],
        )?;

        // Create automation_actions table
        debug!("Creating automation_actions table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_actions (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                action TEXT NOT NULL,
                FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_action_rule
             ON automation_actions(rule_id)",
            [],
        )?;

        // Create automation_execution_log table
        debug!("Creating automation_execution_log table if not exists");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS automation_execution_log (
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL,
                rule_name TEXT NOT NULL,
                success INTEGER NOT NULL,
                error_message TEXT,
                executed_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_time
             ON automation_execution_log(executed_at DESC)",
            [],
        )?;

        info!("Database schema initialization complete");
        Ok(())
    }

    /// Insert a temperature reading
    pub fn insert_reading(&self, reading: &TemperatureReading) -> Result<()> {
        debug!(
            device_id = %reading.device_id,
            temperature = %reading.temperature,
            humidity = ?reading.humidity,
            battery = ?reading.battery,
            link_quality = ?reading.link_quality,
            timestamp = %reading.timestamp,
            "Inserting temperature reading"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock().unwrap().execute(
            "INSERT INTO temperature_readings
             (device_id, temperature, humidity, battery, link_quality, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                reading.device_id,
                reading.temperature,
                reading.humidity,
                reading.battery,
                reading.link_quality,
                reading.timestamp.timestamp()
            ],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                debug!(
                    device_id = %reading.device_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully inserted reading"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %reading.device_id,
                    "Database insert failed"
                );
                Err(e)
            }
        }
    }

    /// Get all sensors with their device info (ID + name)
    pub fn get_all_sensors(&self) -> Result<Vec<DeviceInfo>> {
        debug!("Querying all distinct sensors with device info");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT tr.device_id, COALESCE(d.name, tr.device_id) as name
             FROM temperature_readings tr
             LEFT JOIN devices d ON tr.device_id = d.id
             ORDER BY name",
        )?;

        let sensors = stmt
            .query_map([], |row| {
                Ok(DeviceInfo {
                    device_id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            sensor_count = sensors.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all sensors with device info"
        );
        debug!(sensors = ?sensors, "Sensor device info");

        Ok(sensors)
    }

    /// Get readings for all sensors within a time range
    pub fn get_readings_since(&self, since_timestamp: i64) -> Result<Vec<TemperatureReading>> {
        debug!(
            since_timestamp = since_timestamp,
            "Querying readings since timestamp"
        );
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, battery, link_quality, timestamp
             FROM temperature_readings
             WHERE timestamp > ?1
             ORDER BY device_id, timestamp DESC",
        )?;

        let readings = stmt
            .query_map(params![since_timestamp], |row| {
                Ok(TemperatureReading {
                    device_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    link_quality: row.get(4)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            reading_count = readings.len(),
            since_timestamp = since_timestamp,
            duration_ms = elapsed.as_millis(),
            "Retrieved readings since timestamp for all sensors"
        );

        if readings.is_empty() {
            warn!(
                since_timestamp = since_timestamp,
                "No readings found since timestamp"
            );
        }

        Ok(readings)
    }

    /// Get readings for a specific device within a time range (SQL-filtered)
    pub fn get_readings_for_sensor_since(
        &self,
        device_id: &str,
        since_timestamp: i64,
    ) -> Result<Vec<TemperatureReading>> {
        debug!(
            device_id = %device_id,
            since_timestamp = since_timestamp,
            "Querying readings for specific device since timestamp"
        );
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, battery, link_quality, timestamp
             FROM temperature_readings
             WHERE device_id = ?1 AND timestamp > ?2
             ORDER BY timestamp DESC",
        )?;

        let readings = stmt
            .query_map(params![device_id, since_timestamp], |row| {
                Ok(TemperatureReading {
                    device_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    link_quality: row.get(4)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            device_id = %device_id,
            reading_count = readings.len(),
            since_timestamp = since_timestamp,
            duration_ms = elapsed.as_millis(),
            "Retrieved readings for specific device"
        );

        if readings.is_empty() {
            warn!(
                device_id = %device_id,
                since_timestamp = since_timestamp,
                "No readings found for device since timestamp"
            );
        }

        Ok(readings)
    }

    /// Get the latest reading for a specific device (optimized for automation conditions)
    pub fn get_latest_reading_for_sensor(
        &self,
        device_id: &str,
    ) -> Result<Option<TemperatureReading>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, battery, link_quality, timestamp
             FROM temperature_readings
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut readings = stmt
            .query_map(params![device_id], |row| {
                Ok(TemperatureReading {
                    device_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    link_quality: row.get(4)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(readings.pop())
    }

    /// Insert a new device into the database
    pub fn insert_device(&self, device: &Device) -> Result<()> {
        debug!(
            device_id = %device.id,
            mqtt_topic = %device.mqtt_topic,
            name = %device.name,
            device_type = %device.device_type.to_db_string(),
            power_source = %device.power_source.to_db_string(),
            "Inserting device"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock().unwrap().execute(
            "INSERT INTO devices (id, mqtt_topic, name, device_type, power_source, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                device.id,
                device.mqtt_topic,
                device.name,
                device.device_type.to_db_string(),
                device.power_source.to_db_string(),
                device.added_at.timestamp()
            ],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    device_id = %device.id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully inserted device"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %device.id,
                    "Database insert device failed"
                );
                Err(e)
            }
        }
    }

    /// Get a device by its ID
    pub fn get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        debug!(device_id = %device_id, "Querying device by ID");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, name, device_type, power_source, added_at
             FROM devices
             WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![device_id], |row| {
            let device_type_str: String = row.get(3)?;
            let power_source_str: String = row.get(4)?;

            Ok(Device {
                id: row.get(0)?,
                mqtt_topic: row.get(1)?,
                name: row.get(2)?,
                device_type: DeviceType::from_db_string(&device_type_str).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        3,
                        "device_type".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                power_source: PowerSource::from_db_string(&power_source_str).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "power_source".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                added_at: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
            })
        });

        let elapsed = start.elapsed();
        match result {
            Ok(device) => {
                debug!(
                    device_id = %device_id,
                    duration_us = elapsed.as_micros(),
                    "Found device"
                );
                Ok(Some(device))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(device_id = %device_id, "Device not found");
                Ok(None)
            }
            Err(e) => {
                error!(error = %e, device_id = %device_id, "Failed to query device");
                Err(e)
            }
        }
    }

    /// Get a device by its MQTT topic
    pub fn get_device_by_mqtt_topic(&self, mqtt_topic: &str) -> Result<Option<Device>> {
        debug!(mqtt_topic = %mqtt_topic, "Querying device by MQTT topic");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, name, device_type, power_source, added_at
             FROM devices
             WHERE mqtt_topic = ?1",
        )?;

        let result = stmt.query_row(params![mqtt_topic], |row| {
            let device_type_str: String = row.get(3)?;
            let power_source_str: String = row.get(4)?;

            Ok(Device {
                id: row.get(0)?,
                mqtt_topic: row.get(1)?,
                name: row.get(2)?,
                device_type: DeviceType::from_db_string(&device_type_str).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        3,
                        "device_type".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                power_source: PowerSource::from_db_string(&power_source_str).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "power_source".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?,
                added_at: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
            })
        });

        let elapsed = start.elapsed();
        match result {
            Ok(device) => {
                debug!(
                    mqtt_topic = %mqtt_topic,
                    device_id = %device.id,
                    duration_us = elapsed.as_micros(),
                    "Found device"
                );
                Ok(Some(device))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(mqtt_topic = %mqtt_topic, "Device not found");
                Ok(None)
            }
            Err(e) => {
                error!(error = %e, mqtt_topic = %mqtt_topic, "Failed to query device");
                Err(e)
            }
        }
    }

    /// Get all devices
    pub fn get_all_devices(&self) -> Result<Vec<Device>> {
        debug!("Querying all devices");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, name, device_type, power_source, added_at
             FROM devices
             ORDER BY name",
        )?;

        let devices = stmt
            .query_map([], |row| {
                let device_type_str: String = row.get(3)?;
                let power_source_str: String = row.get(4)?;

                Ok(Device {
                    id: row.get(0)?,
                    mqtt_topic: row.get(1)?,
                    name: row.get(2)?,
                    device_type: DeviceType::from_db_string(&device_type_str).ok_or_else(|| {
                        rusqlite::Error::InvalidColumnType(
                            3,
                            "device_type".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    power_source: PowerSource::from_db_string(&power_source_str).ok_or_else(
                        || {
                            rusqlite::Error::InvalidColumnType(
                                4,
                                "power_source".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        },
                    )?,
                    added_at: chrono::DateTime::from_timestamp(row.get(5)?, 0).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            device_count = devices.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all devices"
        );

        Ok(devices)
    }

    /// Update device name
    pub fn update_device_name(&self, device_id: &str, new_name: &str) -> Result<()> {
        debug!(
            device_id = %device_id,
            new_name = %new_name,
            "Updating device name"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock().unwrap().execute(
            "UPDATE devices SET name = ?1 WHERE id = ?2",
            params![new_name, device_id],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    device_id = %device_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully updated device name"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %device_id,
                    "Failed to update device name"
                );
                Err(e)
            }
        }
    }

    /// Delete a device
    pub fn delete_device(&self, device_id: &str) -> Result<()> {
        debug!(device_id = %device_id, "Deleting device");

        let start = std::time::Instant::now();
        let result = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM devices WHERE id = ?1", params![device_id]);

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    device_id = %device_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully deleted device"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %device_id,
                    "Failed to delete device"
                );
                Err(e)
            }
        }
    }

    /// Get MQTT topic for a device ID (useful for sending commands)
    pub fn get_mqtt_topic_for_device(&self, device_id: &str) -> Result<Option<String>> {
        debug!(device_id = %device_id, "Looking up MQTT topic for device ID");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT mqtt_topic FROM devices WHERE id = ?1",
            params![device_id],
            |row| row.get(0),
        );

        let elapsed = start.elapsed();
        match result {
            Ok(mqtt_topic) => {
                debug!(
                    device_id = %device_id,
                    mqtt_topic = %mqtt_topic,
                    duration_us = elapsed.as_micros(),
                    "Found MQTT topic for device"
                );
                Ok(Some(mqtt_topic))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(device_id = %device_id, "No device found for ID");
                Ok(None)
            }
            Err(e) => {
                error!(error = %e, device_id = %device_id, "Failed to query MQTT topic");
                Err(e)
            }
        }
    }

    // ==================== Automation Rules Methods ====================

    /// Insert a new automation rule with its conditions and actions
    pub fn insert_automation_rule(&self, rule: &AutomationRule) -> Result<()> {
        debug!(
            rule_id = %rule.id,
            name = %rule.name,
            "Inserting automation rule"
        );

        let conn = self.conn.lock().unwrap();
        let start = std::time::Instant::now();

        // Begin transaction
        conn.execute("BEGIN TRANSACTION", [])?;

        // Insert the rule
        let rule_result = conn.execute(
            "INSERT INTO automation_rules (id, name, description, enabled, condition_operator, created_at, updated_at, last_triggered_at, trigger_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                rule.id,
                rule.name,
                rule.description,
                if rule.enabled { 1 } else { 0 },
                rule.condition_operator.to_db_string(),
                rule.created_at.timestamp(),
                rule.updated_at.timestamp(),
                rule.last_triggered_at.map(|dt| dt.timestamp()),
                rule.trigger_count
            ],
        );

        if let Err(e) = rule_result {
            conn.execute("ROLLBACK", [])?;
            error!(error = %e, rule_id = %rule.id, "Failed to insert rule");
            return Err(e);
        }

        // Insert conditions
        for condition in &rule.conditions {
            let cond_result = conn.execute(
                "INSERT INTO automation_conditions (id, rule_id, device_id, field, operator, value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    condition.id,
                    rule.id,
                    condition.device_id,
                    condition.field.to_db_string(),
                    condition.operator.to_db_string(),
                    condition.value
                ],
            );

            if let Err(e) = cond_result {
                conn.execute("ROLLBACK", [])?;
                error!(error = %e, rule_id = %rule.id, "Failed to insert condition");
                return Err(e);
            }
        }

        // Insert actions
        for action in &rule.actions {
            let action_result = conn.execute(
                "INSERT INTO automation_actions (id, rule_id, device_id, action)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    action.id,
                    rule.id,
                    action.device_id,
                    action.action.to_db_string()
                ],
            );

            if let Err(e) = action_result {
                conn.execute("ROLLBACK", [])?;
                error!(error = %e, rule_id = %rule.id, "Failed to insert action");
                return Err(e);
            }
        }

        // Commit transaction
        conn.execute("COMMIT", [])?;

        let elapsed = start.elapsed();
        info!(
            rule_id = %rule.id,
            duration_ms = elapsed.as_millis(),
            "Successfully inserted automation rule"
        );

        Ok(())
    }

    /// Get all automation rules with their conditions and actions
    pub fn get_all_automation_rules(&self) -> Result<Vec<AutomationRule>> {
        debug!("Querying all automation rules");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();

        // Get all rules
        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, condition_operator, created_at, updated_at, last_triggered_at, trigger_count
             FROM automation_rules
             ORDER BY name",
        )?;

        let rule_rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let condition_operator_str: String = row.get(4)?;

            Ok((
                id,
                row.get::<_, String>(1)?,         // name
                row.get::<_, Option<String>>(2)?, // description
                row.get::<_, i32>(3)? == 1,       // enabled
                condition_operator_str,
                row.get::<_, i64>(5)?,         // created_at
                row.get::<_, i64>(6)?,         // updated_at
                row.get::<_, Option<i64>>(7)?, // last_triggered_at
                row.get::<_, i64>(8)?,         // trigger_count
            ))
        })?;

        let mut rules = Vec::new();

        for rule_row_result in rule_rows {
            let (
                id,
                name,
                description,
                enabled,
                condition_operator_str,
                created_at,
                updated_at,
                last_triggered_at,
                trigger_count,
            ) = rule_row_result?;

            let condition_operator = LogicalOperator::from_db_string(&condition_operator_str)
                .ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "condition_operator".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;

            // Get conditions for this rule
            let mut cond_stmt = conn.prepare(
                "SELECT id, device_id, field, operator, value
                 FROM automation_conditions
                 WHERE rule_id = ?1",
            )?;

            let conditions: Vec<AutomationCondition> = cond_stmt
                .query_map(params![id], |row| {
                    let field_str: String = row.get(2)?;
                    let operator_str: String = row.get(3)?;

                    Ok(AutomationCondition {
                        id: row.get(0)?,
                        device_id: row.get(1)?,
                        field: SensorField::from_db_string(&field_str).ok_or_else(|| {
                            rusqlite::Error::InvalidColumnType(
                                2,
                                "field".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                        operator: ComparisonOperator::from_db_string(&operator_str).ok_or_else(
                            || {
                                rusqlite::Error::InvalidColumnType(
                                    3,
                                    "operator".to_string(),
                                    rusqlite::types::Type::Text,
                                )
                            },
                        )?,
                        value: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>>>()?;

            // Get actions for this rule
            let mut action_stmt = conn.prepare(
                "SELECT id, device_id, action
                 FROM automation_actions
                 WHERE rule_id = ?1",
            )?;

            let actions: Vec<AutomationAction> = action_stmt
                .query_map(params![id], |row| {
                    let action_str: String = row.get(2)?;

                    Ok(AutomationAction {
                        id: row.get(0)?,
                        device_id: row.get(1)?,
                        action: SwitchAction::from_db_string(&action_str).ok_or_else(|| {
                            rusqlite::Error::InvalidColumnType(
                                2,
                                "action".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                    })
                })?
                .collect::<Result<Vec<_>>>()?;

            rules.push(AutomationRule {
                id,
                name,
                description,
                enabled,
                condition_operator,
                conditions,
                actions,
                created_at: chrono::DateTime::from_timestamp(created_at, 0).unwrap_or_default(),
                updated_at: chrono::DateTime::from_timestamp(updated_at, 0).unwrap_or_default(),
                last_triggered_at: last_triggered_at
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
                trigger_count,
            });
        }

        let elapsed = start.elapsed();
        info!(
            rule_count = rules.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all automation rules"
        );

        Ok(rules)
    }

    /// Get a single automation rule by ID
    pub fn get_automation_rule(&self, rule_id: &str) -> Result<Option<AutomationRule>> {
        debug!(rule_id = %rule_id, "Querying automation rule by ID");

        let all_rules = self.get_all_automation_rules()?;
        Ok(all_rules.into_iter().find(|r| r.id == rule_id))
    }

    /// Update an automation rule
    pub fn update_automation_rule(&self, rule: &AutomationRule) -> Result<()> {
        debug!(rule_id = %rule.id, "Updating automation rule");

        let conn = self.conn.lock().unwrap();
        let start = std::time::Instant::now();

        conn.execute("BEGIN TRANSACTION", [])?;

        // Update the rule metadata
        let result = conn.execute(
            "UPDATE automation_rules
             SET name = ?1, description = ?2, enabled = ?3, condition_operator = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                rule.name,
                rule.description,
                if rule.enabled { 1 } else { 0 },
                rule.condition_operator.to_db_string(),
                rule.updated_at.timestamp(),
                rule.id
            ],
        );

        if let Err(e) = result {
            conn.execute("ROLLBACK", [])?;
            return Err(e);
        }

        // Delete existing conditions and actions
        conn.execute(
            "DELETE FROM automation_conditions WHERE rule_id = ?1",
            params![rule.id],
        )?;
        conn.execute(
            "DELETE FROM automation_actions WHERE rule_id = ?1",
            params![rule.id],
        )?;

        // Insert new conditions
        for condition in &rule.conditions {
            conn.execute(
                "INSERT INTO automation_conditions (id, rule_id, device_id, field, operator, value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    condition.id,
                    rule.id,
                    condition.device_id,
                    condition.field.to_db_string(),
                    condition.operator.to_db_string(),
                    condition.value
                ],
            )?;
        }

        // Insert new actions
        for action in &rule.actions {
            conn.execute(
                "INSERT INTO automation_actions (id, rule_id, device_id, action)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    action.id,
                    rule.id,
                    action.device_id,
                    action.action.to_db_string()
                ],
            )?;
        }

        conn.execute("COMMIT", [])?;

        let elapsed = start.elapsed();
        info!(
            rule_id = %rule.id,
            duration_ms = elapsed.as_millis(),
            "Successfully updated automation rule"
        );

        Ok(())
    }

    /// Delete an automation rule (cascades to conditions and actions)
    pub fn delete_automation_rule(&self, rule_id: &str) -> Result<()> {
        debug!(rule_id = %rule_id, "Deleting automation rule");

        let start = std::time::Instant::now();
        let result = self.conn.lock().unwrap().execute(
            "DELETE FROM automation_rules WHERE id = ?1",
            params![rule_id],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    rule_id = %rule_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully deleted automation rule"
                );
                Ok(())
            }
            Err(e) => {
                error!(error = %e, rule_id = %rule_id, "Failed to delete automation rule");
                Err(e)
            }
        }
    }

    /// Update rule's last triggered timestamp and increment trigger count
    pub fn record_rule_trigger(&self, rule_id: &str) -> Result<()> {
        debug!(rule_id = %rule_id, "Recording rule trigger");

        let now = chrono::Utc::now().timestamp();
        let result = self.conn.lock().unwrap().execute(
            "UPDATE automation_rules
             SET last_triggered_at = ?1, trigger_count = trigger_count + 1
             WHERE id = ?2",
            params![now, rule_id],
        );

        match result {
            Ok(_) => {
                debug!(rule_id = %rule_id, "Recorded rule trigger");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, rule_id = %rule_id, "Failed to record rule trigger");
                Err(e)
            }
        }
    }

    /// Insert an execution log entry
    pub fn insert_execution_log(&self, log: &AutomationExecutionLog) -> Result<()> {
        debug!(
            log_id = %log.id,
            rule_id = %log.rule_id,
            success = %log.success,
            "Inserting execution log"
        );

        let result = self.conn.lock().unwrap().execute(
            "INSERT INTO automation_execution_log (id, rule_id, rule_name, success, error_message, executed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                log.id,
                log.rule_id,
                log.rule_name,
                if log.success { 1 } else { 0 },
                log.error_message,
                log.executed_at.timestamp()
            ],
        );

        match result {
            Ok(_) => {
                debug!(log_id = %log.id, "Inserted execution log");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, log_id = %log.id, "Failed to insert execution log");
                Err(e)
            }
        }
    }

    /// Get recent execution logs (limited to last N entries)
    pub fn get_execution_logs(&self, limit: i64) -> Result<Vec<AutomationExecutionLog>> {
        debug!(limit = %limit, "Querying execution logs");

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, rule_id, rule_name, success, error_message, executed_at
             FROM automation_execution_log
             ORDER BY executed_at DESC
             LIMIT ?1",
        )?;

        let logs = stmt
            .query_map(params![limit], |row| {
                Ok(AutomationExecutionLog {
                    id: row.get(0)?,
                    rule_id: row.get(1)?,
                    rule_name: row.get(2)?,
                    success: row.get::<_, i32>(3)? == 1,
                    error_message: row.get(4)?,
                    executed_at: chrono::DateTime::from_timestamp(row.get(5)?, 0)
                        .unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        info!(log_count = logs.len(), "Retrieved execution logs");
        Ok(logs)
    }
}

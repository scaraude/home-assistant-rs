use crate::models::TemperatureReading;
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
                sensor_id TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL,
                battery INTEGER,
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
             ON temperature_readings(sensor_id, timestamp DESC)",
            [],
        ) {
            Ok(_) => debug!("Index idx_sensor_time created/verified"),
            Err(e) => {
                error!(error = %e, "Failed to create index idx_sensor_time");
                return Err(e);
            }
        }

        info!("Database schema initialization complete");
        Ok(())
    }

    /// Insert a temperature reading
    pub fn insert_reading(&self, reading: &TemperatureReading) -> Result<()> {
        debug!(
            sensor_id = %reading.sensor_id,
            temperature = %reading.temperature,
            humidity = ?reading.humidity,
            battery = ?reading.battery,
            timestamp = %reading.timestamp,
            "Inserting temperature reading"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock().unwrap().execute(
            "INSERT INTO temperature_readings
             (sensor_id, temperature, humidity, battery, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                reading.sensor_id,
                reading.temperature,
                reading.humidity,
                reading.battery,
                reading.timestamp.timestamp()
            ],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                debug!(
                    sensor_id = %reading.sensor_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully inserted reading"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    sensor_id = %reading.sensor_id,
                    "Database insert failed"
                );
                Err(e)
            }
        }
    }

    /// Get all sensors with their latest reading
    pub fn get_all_sensors(&self) -> Result<Vec<String>> {
        debug!("Querying all distinct sensor IDs");
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT sensor_id
             FROM temperature_readings
             ORDER BY sensor_id",
        )?;

        let sensors = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            sensor_count = sensors.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all sensors"
        );
        debug!(sensors = ?sensors, "Sensor IDs");

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
            "SELECT sensor_id, temperature, humidity, battery, timestamp
             FROM temperature_readings
             WHERE timestamp > ?1
             ORDER BY sensor_id, timestamp DESC",
        )?;

        let readings = stmt
            .query_map(params![since_timestamp], |row| {
                Ok(TemperatureReading {
                    sensor_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap_or_default(),
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

    /// Get readings for a specific sensor within a time range (SQL-filtered)
    pub fn get_readings_for_sensor_since(
        &self,
        sensor_id: &str,
        since_timestamp: i64,
    ) -> Result<Vec<TemperatureReading>> {
        debug!(
            sensor_id = %sensor_id,
            since_timestamp = since_timestamp,
            "Querying readings for specific sensor since timestamp"
        );
        let start = std::time::Instant::now();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT sensor_id, temperature, humidity, battery, timestamp
             FROM temperature_readings
             WHERE sensor_id = ?1 AND timestamp > ?2
             ORDER BY timestamp DESC",
        )?;

        let readings = stmt
            .query_map(params![sensor_id, since_timestamp], |row| {
                Ok(TemperatureReading {
                    sensor_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(4)?, 0).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            sensor_id = %sensor_id,
            reading_count = readings.len(),
            since_timestamp = since_timestamp,
            duration_ms = elapsed.as_millis(),
            "Retrieved readings for specific sensor"
        );

        if readings.is_empty() {
            warn!(
                sensor_id = %sensor_id,
                since_timestamp = since_timestamp,
                "No readings found for sensor since timestamp"
            );
        }

        Ok(readings)
    }
}

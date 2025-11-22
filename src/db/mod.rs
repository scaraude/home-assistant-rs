use crate::models::TemperatureReading;
use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create or open the database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS temperature_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sensor_id TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL,
                battery INTEGER,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Index for efficient queries by sensor and time
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sensor_time
             ON temperature_readings(sensor_id, timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    /// Insert a temperature reading
    pub fn insert_reading(&self, reading: &TemperatureReading) -> Result<()> {
        self.conn.lock().unwrap().execute(
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
        )?;
        Ok(())
    }

    /// Get recent readings for a specific sensor
    pub fn get_recent_readings(
        &self,
        sensor_id: &str,
        limit: usize,
    ) -> Result<Vec<TemperatureReading>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT sensor_id, temperature, humidity, battery, timestamp
             FROM temperature_readings
             WHERE sensor_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let readings = stmt
            .query_map(params![sensor_id, limit], |row| {
                Ok(TemperatureReading {
                    sensor_id: row.get(0)?,
                    temperature: row.get(1)?,
                    humidity: row.get(2)?,
                    battery: row.get(3)?,
                    timestamp: chrono::DateTime::from_timestamp(row.get(4)?, 0)
                        .ok_or_else(|| rusqlite::Error::InvalidQuery)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(readings)
    }

    /// Get all sensors with their latest reading
    pub fn get_all_sensors(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT sensor_id
             FROM temperature_readings
             ORDER BY sensor_id",
        )?;

        let sensors = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;

        Ok(sensors)
    }

    /// Get readings for all sensors within a time range
    pub fn get_readings_since(&self, since_timestamp: i64) -> Result<Vec<TemperatureReading>> {
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

        Ok(readings)
    }

    /// Get readings for a specific sensor within a time range (SQL-filtered)
    pub fn get_readings_for_sensor_since(
        &self,
        sensor_id: &str,
        since_timestamp: i64,
    ) -> Result<Vec<TemperatureReading>> {
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

        Ok(readings)
    }
}

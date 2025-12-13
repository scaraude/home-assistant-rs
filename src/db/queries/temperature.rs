use crate::models::{DeviceInfo, TemperatureReading};
use rusqlite::{Result, params};
use tracing::{debug, info, warn};

use super::super::connection::{Database, MutexExt};
use super::utils::timestamp_to_datetime;

impl Database {
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
        let result = self.conn.lock_or_recover().execute(
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
                tracing::error!(
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

        let conn = self.conn.lock_or_recover();
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

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, battery, link_quality, timestamp
             FROM temperature_readings
             WHERE timestamp > ?1
             ORDER BY device_id, timestamp DESC",
        )?;

        let readings = stmt
            .query_map(params![since_timestamp], Self::map_temperature_reading_row)?
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

        let conn = self.conn.lock_or_recover();
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
                    timestamp: timestamp_to_datetime(row.get(5)?, "temperature_reading.timestamp"),
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
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, battery, link_quality, timestamp
             FROM temperature_readings
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut readings = stmt
            .query_map(params![device_id], Self::map_temperature_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(readings.pop())
    }

    /// Helper to map a database row to a TemperatureReading struct
    ///
    /// Eliminates duplication across get_readings_since, get_readings_for_sensor_since, and get_latest_reading_for_sensor
    fn map_temperature_reading_row(row: &rusqlite::Row) -> Result<TemperatureReading> {
        Ok(TemperatureReading {
            device_id: row.get(0)?,
            temperature: row.get(1)?,
            humidity: row.get(2)?,
            battery: row.get(3)?,
            link_quality: row.get(4)?,
            timestamp: timestamp_to_datetime(row.get(5)?, "temperature_reading.timestamp"),
        })
    }
}

use crate::models::{DeviceInfo, SensorReading};
use rusqlite::{Result, params};
use tracing::{debug, info, warn};

use super::super::connection::{Database, MutexExt};
use super::utils::{invalid_column_error, timestamp_to_datetime};

impl Database {
    /// Insert a sensor reading
    pub fn insert_reading(&self, reading: &SensorReading) -> Result<()> {
        match reading {
            SensorReading::TempHumidity {
                device_id,
                temperature,
                humidity,
                timestamp,
            } => {
                debug!(
                    device_id = %device_id,
                    temperature = %temperature,
                    humidity = %humidity,
                    timestamp = %timestamp,
                    "Inserting temperature/humidity reading"
                );

                let start = std::time::Instant::now();
                let result = self.conn.lock_or_recover().execute(
                    "INSERT INTO temperature_readings
                     (device_id, temperature, humidity, timestamp)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![device_id, temperature, humidity, timestamp.timestamp()],
                );

                match result {
                    Ok(rows) => {
                        let elapsed = start.elapsed();
                        debug!(
                            device_id = %device_id,
                            rows_affected = rows,
                            duration_us = elapsed.as_micros(),
                            "Successfully inserted reading"
                        );
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            device_id = %device_id,
                            "Database insert failed"
                        );
                        Err(e)
                    }
                }
            }
            SensorReading::Presence {
                device_id,
                occupied,
                illumination,
                timestamp,
            } => {
                debug!(
                    device_id = %device_id,
                    occupied = %occupied,
                    illumination = ?illumination,
                    timestamp = %timestamp,
                    "Inserting presence reading"
                );

                let start = std::time::Instant::now();
                let result = self.conn.lock_or_recover().execute(
                    "INSERT INTO presence_readings
                     (device_id, occupied, illumination, timestamp)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        device_id,
                        *occupied as i32,
                        illumination,
                        timestamp.timestamp()
                    ],
                );

                match result {
                    Ok(rows) => {
                        let elapsed = start.elapsed();
                        debug!(
                            device_id = %device_id,
                            rows_affected = rows,
                            duration_us = elapsed.as_micros(),
                            "Successfully inserted presence reading"
                        );
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            device_id = %device_id,
                            "Database insert failed for presence reading"
                        );
                        Err(e)
                    }
                }
            }
            SensorReading::EnergyMeter {
                device_id,
                power,
                energy,
                produced_energy,
                voltage,
                current,
                ac_frequency,
                power_factor,
                timestamp,
            } => {
                debug!(
                    device_id = %device_id,
                    power = %power,
                    energy = %energy,
                    timestamp = %timestamp,
                    "Inserting energy reading"
                );

                let start = std::time::Instant::now();
                let result = self.conn.lock_or_recover().execute(
                    "INSERT INTO energy_readings
                     (device_id, power, energy, produced_energy, voltage,
                      current, ac_frequency, power_factor, timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        device_id,
                        power,
                        energy,
                        produced_energy,
                        voltage,
                        current,
                        ac_frequency,
                        power_factor,
                        timestamp.timestamp()
                    ],
                );

                match result {
                    Ok(rows) => {
                        let elapsed = start.elapsed();
                        debug!(
                            device_id = %device_id,
                            rows_affected = rows,
                            duration_us = elapsed.as_micros(),
                            "Successfully inserted energy reading"
                        );
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            device_id = %device_id,
                            "Database insert failed for energy reading"
                        );
                        Err(e)
                    }
                }
            }
        }
    }

    /// Get all sensors with their device info (ID + name + capability metadata)
    pub fn get_all_sensor_devices(&self) -> Result<Vec<DeviceInfo>> {
        debug!("Querying all distinct sensors with device info");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT
                 sensors.device_id,
                 COALESCE(d.name, sensors.device_id) as name,
                 d.capabilities,
                 d.available_fields
             FROM (
                 SELECT DISTINCT device_id FROM temperature_readings
                 UNION
                 SELECT DISTINCT device_id FROM presence_readings
                 UNION
                 SELECT DISTINCT device_id FROM energy_readings
             ) sensors
             INNER JOIN devices d ON sensors.device_id = d.id
             ORDER BY name",
        )?;

        let sensors = stmt
            .query_map([], |row| {
                let capabilities_raw: String = row.get(2)?;
                let available_fields_raw: String = row.get(3)?;
                let capabilities = crate::models::Device::capabilities_from_db(&capabilities_raw)
                    .ok_or_else(|| invalid_column_error(2, "capabilities"))?;
                let available_fields =
                    crate::models::Device::available_fields_from_db(&available_fields_raw)
                        .ok_or_else(|| invalid_column_error(3, "available_fields"))?;
                Ok(DeviceInfo {
                    device_id: row.get(0)?,
                    name: row.get(1)?,
                    capabilities,
                    available_fields,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let elapsed = start.elapsed();
        info!(
            sensor_count = sensors.len(),
            duration_ms = elapsed.as_millis(),
            "Retrieved all sensors with device info and capabilities"
        );
        debug!(sensors = ?sensors, "Sensor device info");

        Ok(sensors)
    }

    /// Get readings for all sensors within a time range
    pub fn get_readings_since(&self, since_timestamp: i64) -> Result<Vec<SensorReading>> {
        debug!(
            since_timestamp = since_timestamp,
            "Querying readings since timestamp"
        );
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();

        // Get temperature readings
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, timestamp
             FROM temperature_readings
             WHERE timestamp > ?1
             ORDER BY device_id, timestamp DESC",
        )?;

        let mut readings = stmt
            .query_map(params![since_timestamp], Self::map_sensor_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        // Get presence readings
        let mut stmt = conn.prepare(
            "SELECT device_id, occupied, illumination, timestamp
             FROM presence_readings
             WHERE timestamp > ?1
             ORDER BY device_id, timestamp DESC",
        )?;

        let mut presence_readings = stmt
            .query_map(params![since_timestamp], Self::map_presence_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        readings.append(&mut presence_readings);

        // Get energy readings
        let mut stmt = conn.prepare(
            "SELECT device_id, power, energy, produced_energy, voltage,
                    current, ac_frequency, power_factor, timestamp
             FROM energy_readings
             WHERE timestamp > ?1
             ORDER BY device_id, timestamp DESC",
        )?;

        let mut energy_readings = stmt
            .query_map(params![since_timestamp], Self::map_energy_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        readings.append(&mut energy_readings);

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
    ) -> Result<Vec<SensorReading>> {
        debug!(
            device_id = %device_id,
            since_timestamp = since_timestamp,
            "Querying readings for specific device since timestamp"
        );
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();

        // Try temperature readings first
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, timestamp
             FROM temperature_readings
             WHERE device_id = ?1 AND timestamp > ?2
             ORDER BY timestamp DESC",
        )?;

        let mut readings = stmt
            .query_map(
                params![device_id, since_timestamp],
                Self::map_sensor_reading_row,
            )?
            .collect::<Result<Vec<_>>>()?;

        // Try presence readings
        let mut stmt = conn.prepare(
            "SELECT device_id, occupied, illumination, timestamp
             FROM presence_readings
             WHERE device_id = ?1 AND timestamp > ?2
             ORDER BY timestamp DESC",
        )?;

        let mut presence_readings = stmt
            .query_map(
                params![device_id, since_timestamp],
                Self::map_presence_reading_row,
            )?
            .collect::<Result<Vec<_>>>()?;

        readings.append(&mut presence_readings);

        // Try energy readings
        let mut stmt = conn.prepare(
            "SELECT device_id, power, energy, produced_energy, voltage,
                    current, ac_frequency, power_factor, timestamp
             FROM energy_readings
             WHERE device_id = ?1 AND timestamp > ?2
             ORDER BY timestamp DESC",
        )?;

        let mut energy_readings = stmt
            .query_map(
                params![device_id, since_timestamp],
                Self::map_energy_reading_row,
            )?
            .collect::<Result<Vec<_>>>()?;

        readings.append(&mut energy_readings);

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
    #[cfg(test)]
    pub fn get_latest_reading_for_sensor(&self, device_id: &str) -> Result<Option<SensorReading>> {
        let conn = self.conn.lock_or_recover();

        // Try temperature readings first
        let mut stmt = conn.prepare(
            "SELECT device_id, temperature, humidity, timestamp
             FROM temperature_readings
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut readings = stmt
            .query_map(params![device_id], Self::map_sensor_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        if let Some(reading) = readings.pop() {
            return Ok(Some(reading));
        }

        // Try presence readings
        let mut stmt = conn.prepare(
            "SELECT device_id, occupied, illumination, timestamp
             FROM presence_readings
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut presence_readings = stmt
            .query_map(params![device_id], Self::map_presence_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        if let Some(reading) = presence_readings.pop() {
            return Ok(Some(reading));
        }

        // Try energy readings
        let mut stmt = conn.prepare(
            "SELECT device_id, power, energy, produced_energy, voltage,
                    current, ac_frequency, power_factor, timestamp
             FROM energy_readings
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut energy_readings = stmt
            .query_map(params![device_id], Self::map_energy_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        Ok(energy_readings.pop())
    }

    /// Get the latest readings for multiple devices in a single database transaction
    ///
    /// This method eliminates the N+1 query problem when evaluating automation rules
    /// with multiple conditions. Instead of querying each device separately, it fetches
    /// all readings in a single batch operation.
    ///
    /// Returns a HashMap mapping device_id to its latest SensorReading.
    /// Devices with no readings are not included in the result.
    pub fn get_latest_readings_batch(
        &self,
        device_ids: &[String],
    ) -> Result<std::collections::HashMap<String, SensorReading>> {
        use std::collections::HashMap;

        if device_ids.is_empty() {
            return Ok(HashMap::new());
        }

        debug!(
            device_count = device_ids.len(),
            device_ids = ?device_ids,
            "Batch querying latest readings for multiple devices"
        );
        let start = std::time::Instant::now();

        let mut result = HashMap::new();
        let conn = self.conn.lock_or_recover();

        // Build placeholders for SQL IN clause
        let placeholders = device_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Build params vector that can be reused
        let params_refs: Vec<&dyn rusqlite::ToSql> = device_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        // Query temperature readings
        let query = format!(
            "SELECT device_id, temperature, humidity, timestamp
             FROM temperature_readings
             WHERE device_id IN ({})
             AND (device_id, timestamp) IN (
                 SELECT device_id, MAX(timestamp)
                 FROM temperature_readings
                 WHERE device_id IN ({})
                 GROUP BY device_id
             )",
            placeholders, placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        // Duplicate params for both IN clauses
        let doubled_params: Vec<&dyn rusqlite::ToSql> = params_refs
            .iter()
            .chain(params_refs.iter())
            .copied()
            .collect();

        let temp_readings = stmt
            .query_map(doubled_params.as_slice(), Self::map_sensor_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        for reading in temp_readings {
            result.insert(reading.device_id().to_string(), reading);
        }

        // Query presence readings
        let query = format!(
            "SELECT device_id, occupied, illumination, timestamp
             FROM presence_readings
             WHERE device_id IN ({})
             AND (device_id, timestamp) IN (
                 SELECT device_id, MAX(timestamp)
                 FROM presence_readings
                 WHERE device_id IN ({})
                 GROUP BY device_id
             )",
            placeholders, placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        let doubled_params: Vec<&dyn rusqlite::ToSql> = params_refs
            .iter()
            .chain(params_refs.iter())
            .copied()
            .collect();

        let presence_readings = stmt
            .query_map(doubled_params.as_slice(), Self::map_presence_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        for reading in presence_readings {
            result.insert(reading.device_id().to_string(), reading);
        }

        // Query energy readings
        let query = format!(
            "SELECT device_id, power, energy, produced_energy, voltage,
                    current, ac_frequency, power_factor, timestamp
             FROM energy_readings
             WHERE device_id IN ({})
             AND (device_id, timestamp) IN (
                 SELECT device_id, MAX(timestamp)
                 FROM energy_readings
                 WHERE device_id IN ({})
                 GROUP BY device_id
             )",
            placeholders, placeholders
        );

        let mut stmt = conn.prepare(&query)?;
        let doubled_params: Vec<&dyn rusqlite::ToSql> = params_refs
            .iter()
            .chain(params_refs.iter())
            .copied()
            .collect();

        let energy_readings = stmt
            .query_map(doubled_params.as_slice(), Self::map_energy_reading_row)?
            .collect::<Result<Vec<_>>>()?;

        for reading in energy_readings {
            result.insert(reading.device_id().to_string(), reading);
        }

        let elapsed = start.elapsed();
        info!(
            device_count = device_ids.len(),
            readings_found = result.len(),
            duration_ms = elapsed.as_millis(),
            "Batch query completed for latest readings"
        );

        Ok(result)
    }

    /// Helper to map a database row to a SensorReading enum
    ///
    /// Eliminates duplication across get_readings_since, get_readings_for_sensor_since, and get_latest_reading_for_sensor
    fn map_sensor_reading_row(row: &rusqlite::Row) -> Result<SensorReading> {
        Ok(SensorReading::TempHumidity {
            device_id: row.get(0)?,
            temperature: row.get(1)?,
            humidity: row.get(2)?,
            timestamp: timestamp_to_datetime(row.get(3)?, "temperature_reading.timestamp"),
        })
    }

    /// Helper to map a database row to a Presence SensorReading
    fn map_presence_reading_row(row: &rusqlite::Row) -> Result<SensorReading> {
        Ok(SensorReading::Presence {
            device_id: row.get(0)?,
            occupied: row.get::<_, i32>(1)? != 0,
            illumination: row.get(2)?,
            timestamp: timestamp_to_datetime(row.get(3)?, "presence_reading.timestamp"),
        })
    }

    /// Helper to map a database row to an Energy SensorReading
    fn map_energy_reading_row(row: &rusqlite::Row) -> Result<SensorReading> {
        Ok(SensorReading::EnergyMeter {
            device_id: row.get(0)?,
            power: row.get(1)?,
            energy: row.get(2)?,
            produced_energy: row.get(3)?,
            voltage: row.get(4)?,
            current: row.get(5)?,
            ac_frequency: row.get(6)?,
            power_factor: row.get(7)?,
            timestamp: timestamp_to_datetime(row.get(8)?, "energy_reading.timestamp"),
        })
    }
}

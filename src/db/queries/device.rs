use crate::models::db_enum::DbEnum;
use crate::models::{Device, PowerSource};
use rusqlite::{Result, params};
use tracing::{debug, error, info};

use super::super::connection::{Database, MutexExt};
use super::utils::{invalid_column_error, timestamp_to_datetime};

impl Database {
    /// Insert a new device into the database
    pub fn insert_device(&self, device: &Device) -> Result<()> {
        let (capability_type, capability_subtype) = device.capability_to_db();

        debug!(
            device_id = %device.id,
            mqtt_topic = %device.mqtt_topic,
            name = %device.name,
            capability_type = %capability_type,
            capability_subtype = %capability_subtype,
            power_source = %device.power_source.to_db_string(),
            "Inserting device"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock_or_recover().execute(
            "INSERT INTO devices (id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                device.id,
                device.mqtt_topic,
                device.ieee_addr,
                device.name,
                capability_type,
                capability_subtype,
                device.power_source.to_db_string(),
                device.added_at.timestamp(),
                device.is_bridge as i32,
                device.parent_device_id
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
    pub fn _get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        debug!(device_id = %device_id, "Querying device by ID");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id
             FROM devices
             WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![device_id], Self::map_device_row);

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

    /// Get a device by its ID
    pub fn get_device(&self, device_id: &str) -> Result<Option<Device>> {
        debug!(device_id = %device_id, "Querying device by ID");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id
             FROM devices
             WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![device_id], Self::map_device_row);

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

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id
             FROM devices
             WHERE mqtt_topic = ?1",
        )?;

        let result = stmt.query_row(params![mqtt_topic], Self::map_device_row);

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

    /// Get a device by its IEEE address
    pub fn get_device_by_ieee_addr(&self, ieee_addr: &str) -> Result<Option<Device>> {
        debug!(ieee_addr = %ieee_addr, "Querying device by IEEE address");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id
             FROM devices
             WHERE ieee_addr = ?1",
        )?;

        let result = stmt.query_row(params![ieee_addr], Self::map_device_row);

        let elapsed = start.elapsed();
        match result {
            Ok(device) => {
                debug!(
                    ieee_addr = %ieee_addr,
                    device_id = %device.id,
                    duration_us = elapsed.as_micros(),
                    "Found device"
                );
                Ok(Some(device))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(ieee_addr = %ieee_addr, "Device not found");
                Ok(None)
            }
            Err(e) => {
                error!(error = %e, ieee_addr = %ieee_addr, "Failed to query device");
                Err(e)
            }
        }
    }

    /// Get all devices
    pub fn get_all_devices(&self) -> Result<Vec<Device>> {
        debug!("Querying all devices");
        let start = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, mqtt_topic, ieee_addr, name, capability_type, capability_subtype, power_source, added_at, is_bridge, parent_device_id
             FROM devices
             ORDER BY name",
        )?;

        let devices = stmt
            .query_map([], Self::map_device_row)?
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
        let result = self.conn.lock_or_recover().execute(
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
    pub fn _delete_device(&self, device_id: &str) -> Result<()> {
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

        let conn = self.conn.lock_or_recover();
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

    /// Helper to map a database row to a Device struct
    ///
    /// Eliminates duplication across get_device_by_id, get_device_by_mqtt_topic, and get_all_devices
    fn map_device_row(row: &rusqlite::Row) -> Result<Device> {
        let capability_type_str: String = row.get(4)?;
        let capability_subtype_str: String = row.get(5)?;
        let power_source_str: String = row.get(6)?;
        let is_bridge_int: i32 = row.get(8).unwrap_or(0);

        Ok(Device {
            id: row.get(0)?,
            mqtt_topic: row.get(1)?,
            ieee_addr: row.get(2)?,
            name: row.get(3)?,
            capability: Device::capability_from_db(&capability_type_str, &capability_subtype_str)
                .ok_or_else(|| invalid_column_error(4, "capability"))?,
            power_source: PowerSource::from_db_string(&power_source_str)
                .ok_or_else(|| invalid_column_error(6, "power_source"))?,
            added_at: timestamp_to_datetime(row.get(7)?, "device.added_at"),
            is_bridge: is_bridge_int != 0,
            parent_device_id: row.get(9)?,
        })
    }

    /// Update device network topology fields
    pub fn update_device_topology(
        &self,
        device_id: &str,
        is_bridge: bool,
        parent_device_id: Option<String>,
    ) -> Result<()> {
        debug!(
            device_id = %device_id,
            is_bridge = %is_bridge,
            parent_device_id = ?parent_device_id,
            "Updating device network topology"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock_or_recover().execute(
            "UPDATE devices SET is_bridge = ?1, parent_device_id = ?2 WHERE id = ?3",
            params![is_bridge as i32, parent_device_id, device_id],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    device_id = %device_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully updated device topology"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %device_id,
                    "Failed to update device topology"
                );
                Err(e)
            }
        }
    }
}

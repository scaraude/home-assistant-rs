use crate::models::db_enum::DbEnum;
use crate::models::{Device, DeviceCapability, PowerSource};
use rusqlite::{Result, params};
use tracing::{debug, error, info};

use super::super::connection::{Database, MutexExt};
use super::utils::{invalid_column_error, timestamp_to_datetime};

impl Database {
    /// Insert a new device into the database
    pub fn insert_device(&self, device: &Device) -> Result<()> {
        let capabilities_json = match device.capabilities_to_db() {
            Ok(json) => json,
            Err(e) => {
                error!(error = %e, device_id = %device.id, "Failed to serialize device capabilities");
                return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e)));
            }
        };
        let available_fields_json = match device.available_fields_to_db() {
            Ok(json) => json,
            Err(e) => {
                error!(error = %e, device_id = %device.id, "Failed to serialize device available fields");
                return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e)));
            }
        };

        debug!(
            device_id = %device.id,
            mqtt_topic = %device.mqtt_topic,
            name = %device.name,
            capabilities = ?device.capabilities,
            power_source = %device.power_source.to_db_string(),
            "Inserting device"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock_or_recover().execute(
            "INSERT INTO devices (id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                device.id,
                device.mqtt_topic,
                device.ieee_addr,
                device.name,
                capabilities_json,
                available_fields_json,
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
            "SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id
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
            "SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id
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
            "SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id
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
            "SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id
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
            "SELECT id, mqtt_topic, ieee_addr, name, capabilities, available_fields, power_source, added_at, is_bridge, parent_device_id
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

    /// Update device capabilities and available fields
    pub fn update_device_metadata(
        &self,
        device_id: &str,
        capabilities: &[DeviceCapability],
        available_fields: &[String],
    ) -> Result<()> {
        let capabilities_json = serde_json::to_string(capabilities).map_err(|e| {
            error!(
                error = %e,
                device_id = %device_id,
                "Failed to serialize device capabilities"
            );
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        let available_fields_json = serde_json::to_string(available_fields).map_err(|e| {
            error!(
                error = %e,
                device_id = %device_id,
                "Failed to serialize device available fields"
            );
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        debug!(
            device_id = %device_id,
            capabilities = ?capabilities,
            field_count = available_fields.len(),
            "Updating device capabilities and available fields"
        );

        let start = std::time::Instant::now();
        let result = self.conn.lock_or_recover().execute(
            "UPDATE devices SET capabilities = ?1, available_fields = ?2 WHERE id = ?3",
            params![capabilities_json, available_fields_json, device_id],
        );

        match result {
            Ok(rows) => {
                let elapsed = start.elapsed();
                info!(
                    device_id = %device_id,
                    rows_affected = rows,
                    duration_us = elapsed.as_micros(),
                    "Successfully updated device metadata"
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %device_id,
                    "Failed to update device metadata"
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
        let capabilities_raw: String = row.get(4)?;
        let available_fields_raw: String = row.get(5)?;
        let power_source_str: String = row.get(6)?;
        let is_bridge_int: i32 = row.get(8).unwrap_or(0);
        let capabilities = Device::capabilities_from_db(&capabilities_raw)
            .ok_or_else(|| invalid_column_error(4, "capabilities"))?;
        let available_fields = Device::available_fields_from_db(&available_fields_raw)
            .ok_or_else(|| invalid_column_error(5, "available_fields"))?;
        Ok(Device {
            id: row.get(0)?,
            mqtt_topic: row.get(1)?,
            ieee_addr: row.get(2)?,
            name: row.get(3)?,
            capabilities,
            available_fields,
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

        let (capabilities_json, available_fields_json) = match self.get_device(device_id)? {
            Some(device) => {
                let mut capabilities = device.capabilities.clone();
                let has_router = capabilities
                    .iter()
                    .any(|cap| matches!(cap, DeviceCapability::Router { .. }));
                if is_bridge && !has_router {
                    capabilities.push(DeviceCapability::Router {
                        turbo_mode: device.supports_turbo_mode(),
                    });
                } else if !is_bridge && has_router {
                    capabilities.retain(|cap| !matches!(cap, DeviceCapability::Router { .. }));
                }

                let caps_json = serde_json::to_string(&capabilities).map_err(|e| {
                    error!(
                        error = %e,
                        device_id = %device_id,
                        "Failed to serialize updated device capabilities"
                    );
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?;
                let fields_json = serde_json::to_string(&device.available_fields).map_err(|e| {
                    error!(
                        error = %e,
                        device_id = %device_id,
                        "Failed to serialize device available fields"
                    );
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?;
                (Some(caps_json), Some(fields_json))
            }
            None => (None, None),
        };

        let start = std::time::Instant::now();
        let result = if let (Some(caps_json), Some(fields_json)) =
            (capabilities_json, available_fields_json)
        {
            self.conn.lock_or_recover().execute(
                "UPDATE devices SET is_bridge = ?1, parent_device_id = ?2, capabilities = ?3, available_fields = ?4 WHERE id = ?5",
                params![is_bridge as i32, parent_device_id, caps_json, fields_json, device_id],
            )
        } else {
            self.conn.lock_or_recover().execute(
                "UPDATE devices SET is_bridge = ?1, parent_device_id = ?2 WHERE id = ?3",
                params![is_bridge as i32, parent_device_id, device_id],
            )
        };

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

use crate::models::DeviceState;
use rusqlite::{Result, params};
use tracing::debug;

use super::super::connection::{Database, MutexExt};

impl Database {
    /// Upsert device state (insert or update)
    pub fn upsert_device_state(&self, state: &DeviceState) -> Result<()> {
        debug!(
            device_id = %state.device_id,
            battery_level = ?state.battery_level,
            link_quality = ?state.link_quality,
            last_seen = %state.last_seen,
            "Upserting device state"
        );

        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, battery_level, link_quality, last_seen)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(device_id) DO UPDATE SET
                battery_level = excluded.battery_level,
                link_quality = excluded.link_quality,
                last_seen = excluded.last_seen",
            params![
                state.device_id,
                state.battery_level,
                state.link_quality,
                state.last_seen.timestamp()
            ],
        )?;

        debug!(device_id = %state.device_id, "Device state upserted successfully");
        Ok(())
    }

    /// Get device state from database
    pub fn get_device_state(&self, device_id: &str) -> Result<Option<DeviceState>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, battery_level, link_quality, last_seen
             FROM device_state
             WHERE device_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![device_id], |row| {
            Ok(DeviceState {
                device_id: row.get(0)?,
                battery_level: row.get(1)?,
                link_quality: row.get(2)?,
                last_seen: chrono::DateTime::from_timestamp(row.get(3)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        Ok(rows.next().transpose()?)
    }

    /// Get all device states
    pub fn get_all_device_states(&self) -> Result<Vec<DeviceState>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, battery_level, link_quality, last_seen
             FROM device_state
             ORDER BY last_seen DESC",
        )?;

        let states = stmt
            .query_map([], |row| {
                Ok(DeviceState {
                    device_id: row.get(0)?,
                    battery_level: row.get(1)?,
                    link_quality: row.get(2)?,
                    last_seen: chrono::DateTime::from_timestamp(row.get(3)?, 0)
                        .unwrap_or_else(chrono::Utc::now),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        debug!(count = states.len(), "Retrieved all device states");
        Ok(states)
    }

    /// Update battery level for a device
    pub fn update_device_battery(&self, device_id: &str, battery_level: u8) -> Result<()> {
        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, battery_level, last_seen)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(device_id) DO UPDATE SET
                battery_level = excluded.battery_level,
                last_seen = excluded.last_seen",
            params![device_id, battery_level, chrono::Utc::now().timestamp()],
        )?;

        debug!(device_id = %device_id, battery_level = %battery_level, "Updated device battery level");
        Ok(())
    }

    /// Update link quality for a device
    pub fn update_device_link_quality(&self, device_id: &str, link_quality: u8) -> Result<()> {
        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, link_quality, last_seen)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(device_id) DO UPDATE SET
                link_quality = excluded.link_quality,
                last_seen = excluded.last_seen",
            params![device_id, link_quality, chrono::Utc::now().timestamp()],
        )?;

        debug!(device_id = %device_id, link_quality = %link_quality, "Updated device link quality");
        Ok(())
    }

    /// Update last_seen timestamp for a device
    pub fn update_device_last_seen(&self, device_id: &str) -> Result<()> {
        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, last_seen)
             VALUES (?1, ?2)
             ON CONFLICT(device_id) DO UPDATE SET
                last_seen = excluded.last_seen",
            params![device_id, chrono::Utc::now().timestamp()],
        )?;

        debug!(device_id = %device_id, "Updated device last_seen timestamp");
        Ok(())
    }
}

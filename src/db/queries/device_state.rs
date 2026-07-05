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
            turbo_mode = ?state.turbo_mode,
            last_seen = %state.last_seen,
            "Upserting device state"
        );

        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, battery_level, link_quality, turbo_mode, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(device_id) DO UPDATE SET
                battery_level = excluded.battery_level,
                link_quality = excluded.link_quality,
                turbo_mode = excluded.turbo_mode,
                last_seen = excluded.last_seen",
            params![
                state.device_id,
                state.battery_level,
                state.link_quality,
                state.turbo_mode,
                state.last_seen.timestamp()
            ],
        )?;

        debug!(device_id = %state.device_id, "Device state upserted successfully");
        Ok(())
    }

    /// Get the last known availability of a device (true = online).
    /// Returns None when no availability was ever recorded.
    pub fn get_device_availability(&self, device_id: &str) -> Result<Option<bool>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT availability FROM device_state WHERE device_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![device_id], |row| {
            row.get::<_, Option<String>>(0)
        })?;

        Ok(rows
            .next()
            .transpose()?
            .flatten()
            .map(|state| state == "online"))
    }

    /// Record a device availability transition without touching the other
    /// device_state fields (battery, link quality, last_seen).
    pub fn set_device_availability(
        &self,
        device_id: &str,
        online: bool,
        changed_at: i64,
    ) -> Result<()> {
        let state = if online { "online" } else { "offline" };
        debug!(device_id = %device_id, state = %state, "Updating device availability");

        self.conn.lock_or_recover().execute(
            "INSERT INTO device_state (device_id, availability, availability_changed_at, last_seen)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(device_id) DO UPDATE SET
                availability = excluded.availability,
                availability_changed_at = excluded.availability_changed_at",
            params![device_id, state, changed_at],
        )?;

        Ok(())
    }

    /// Get device state from database
    pub fn get_device_state(&self, device_id: &str) -> Result<Option<DeviceState>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, battery_level, link_quality, turbo_mode, last_seen
             FROM device_state
             WHERE device_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![device_id], |row| {
            Ok(DeviceState {
                device_id: row.get(0)?,
                battery_level: row.get(1)?,
                link_quality: row.get(2)?,
                turbo_mode: row.get(3)?,
                last_seen: chrono::DateTime::from_timestamp(row.get(4)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        rows.next().transpose()
    }

    /// Get all device states from database
    pub fn get_all_device_states(&self) -> Result<Vec<DeviceState>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, battery_level, link_quality, turbo_mode, last_seen
             FROM device_state
             ORDER BY last_seen DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(DeviceState {
                device_id: row.get(0)?,
                battery_level: row.get(1)?,
                link_quality: row.get(2)?,
                turbo_mode: row.get(3)?,
                last_seen: chrono::DateTime::from_timestamp(row.get(4)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        let mut states = Vec::new();
        for row in rows {
            states.push(row?);
        }

        Ok(states)
    }
}

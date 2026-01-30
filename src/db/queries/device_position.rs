use crate::models::DevicePosition;
use rusqlite::{Result, params};
use tracing::debug;

use super::super::connection::{Database, MutexExt};

impl Database {
    /// Upsert device position (insert or update)
    pub fn upsert_device_position(&self, position: &DevicePosition) -> Result<()> {
        debug!(
            device_id = %position.device_id,
            x = %position.x,
            y = %position.y,
            "Upserting device position"
        );

        self.conn.lock_or_recover().execute(
            "INSERT INTO device_positions (device_id, x, y, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(device_id) DO UPDATE SET
                x = excluded.x,
                y = excluded.y,
                updated_at = excluded.updated_at",
            params![
                position.device_id,
                position.x,
                position.y,
                position.updated_at.timestamp()
            ],
        )?;

        debug!(device_id = %position.device_id, "Device position upserted successfully");
        Ok(())
    }

    /// Get device position by device_id
    pub fn get_device_position(&self, device_id: &str) -> Result<Option<DevicePosition>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, x, y, updated_at
             FROM device_positions
             WHERE device_id = ?1",
        )?;

        let mut rows = stmt.query_map(params![device_id], |row| {
            Ok(DevicePosition {
                device_id: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
                updated_at: chrono::DateTime::from_timestamp(row.get(3)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        rows.next().transpose()
    }

    /// Get all device positions
    pub fn get_all_device_positions(&self) -> Result<Vec<DevicePosition>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT device_id, x, y, updated_at
             FROM device_positions
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(DevicePosition {
                device_id: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
                updated_at: chrono::DateTime::from_timestamp(row.get(3)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        let mut positions = Vec::new();
        for row in rows {
            positions.push(row?);
        }

        Ok(positions)
    }

    /// Delete device position
    pub fn delete_device_position(&self, device_id: &str) -> Result<bool> {
        debug!(device_id = %device_id, "Deleting device position");

        let rows_affected = self.conn.lock_or_recover().execute(
            "DELETE FROM device_positions WHERE device_id = ?1",
            params![device_id],
        )?;

        let deleted = rows_affected > 0;
        if deleted {
            debug!(device_id = %device_id, "Device position deleted successfully");
        } else {
            debug!(device_id = %device_id, "No device position found to delete");
        }

        Ok(deleted)
    }

    /// Batch upsert multiple device positions (useful for bulk updates from frontend)
    pub fn upsert_device_positions(&self, positions: &[DevicePosition]) -> Result<()> {
        if positions.is_empty() {
            return Ok(());
        }

        debug!(count = positions.len(), "Batch upserting device positions");

        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "INSERT INTO device_positions (device_id, x, y, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(device_id) DO UPDATE SET
                x = excluded.x,
                y = excluded.y,
                updated_at = excluded.updated_at",
        )?;

        for position in positions {
            stmt.execute(params![
                position.device_id,
                position.x,
                position.y,
                position.updated_at.timestamp()
            ])?;
        }

        debug!(
            count = positions.len(),
            "Batch upsert completed successfully"
        );
        Ok(())
    }
}

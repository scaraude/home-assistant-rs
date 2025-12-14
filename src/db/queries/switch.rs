use chrono::{DateTime, Utc};
use rusqlite::{params, Result};
use tracing::debug;

use super::super::connection::{Database, MutexExt};

impl Database {
    /// Persist a switch state transition to the database.
    pub fn insert_switch_state(
        &self,
        device_id: &str,
        state: bool,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let state_int = if state { 1 } else { 0 };
        debug!(
            device_id = %device_id,
            state = state_int,
            timestamp = %timestamp,
            "Recording switch state"
        );

        self.conn.lock_or_recover().execute(
            "INSERT INTO switch_state (device_id, state, timestamp)
             VALUES (?1, ?2, ?3)",
            params![device_id, state_int, timestamp.timestamp()],
        )?;

        Ok(())
    }

    /// Fetch the most recent switch state for a device, returning `None` if unknown.
    pub fn get_latest_switch_state(&self, device_id: &str) -> Result<Option<bool>> {
        debug!(device_id = %device_id, "Querying latest switch state");
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT state
             FROM switch_state
             WHERE device_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query(params![device_id])?;
        if let Some(row) = rows.next()? {
            let state: i64 = row.get(0)?;
            let as_bool = state != 0;
            debug!(device_id = %device_id, state = as_bool, "Found latest switch state");
            Ok(Some(as_bool))
        } else {
            debug!(
                device_id = %device_id,
                "No switch state found, returning None"
            );
            Ok(None)
        }
    }
}

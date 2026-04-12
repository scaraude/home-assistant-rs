use chrono::{DateTime, Utc};
use rusqlite::{Result, params};
use tracing::debug;

use crate::models::SwitchState;

use super::super::connection::{Database, MutexExt};

impl Database {
    /// Persist a switch state transition to the database.
    pub fn insert_switch_state(
        &self,
        device_id: &str,
        state: SwitchState,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let state_int = state.to_integer();
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
}

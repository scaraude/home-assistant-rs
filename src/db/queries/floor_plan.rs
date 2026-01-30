use crate::models::FloorPlan;
use rusqlite::{Result, params};
use tracing::debug;

use super::super::connection::{Database, MutexExt};

impl Database {
    /// Get the floor plan (singleton)
    pub fn get_floor_plan(&self) -> Result<Option<FloorPlan>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT svg_content, uploaded_at
             FROM floor_plan
             WHERE id = 1",
        )?;

        let mut rows = stmt.query_map([], |row| {
            Ok(FloorPlan {
                svg_content: row.get(0)?,
                uploaded_at: chrono::DateTime::from_timestamp(row.get(1)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?;

        rows.next().transpose()
    }

    /// Upsert the floor plan (insert or update the singleton)
    pub fn upsert_floor_plan(&self, floor_plan: &FloorPlan) -> Result<()> {
        debug!("Upserting floor plan");

        self.conn.lock_or_recover().execute(
            "INSERT INTO floor_plan (id, svg_content, uploaded_at)
             VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
                svg_content = excluded.svg_content,
                uploaded_at = excluded.uploaded_at",
            params![floor_plan.svg_content, floor_plan.uploaded_at.timestamp()],
        )?;

        debug!("Floor plan upserted successfully");
        Ok(())
    }

    /// Delete the floor plan
    pub fn delete_floor_plan(&self) -> Result<bool> {
        debug!("Deleting floor plan");

        let rows_affected = self
            .conn
            .lock_or_recover()
            .execute("DELETE FROM floor_plan WHERE id = 1", [])?;

        let deleted = rows_affected > 0;
        if deleted {
            debug!("Floor plan deleted successfully");
        } else {
            debug!("No floor plan found to delete");
        }

        Ok(deleted)
    }
}

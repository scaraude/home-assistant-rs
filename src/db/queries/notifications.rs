use rusqlite::{Result, params};
use tracing::debug;

use super::super::connection::{Database, MutexExt};

/// A notification waiting for delivery (kept when the network is down).
#[derive(Debug, Clone)]
pub struct PendingNotification {
    pub id: i64,
    pub message: String,
    pub created_at: i64,
    pub attempts: i64,
}

impl Database {
    /// Queue a notification for later delivery.
    pub fn enqueue_notification(&self, message: &str, created_at: i64) -> Result<()> {
        debug!(message = %message, "Queueing notification for later delivery");
        self.conn.lock_or_recover().execute(
            "INSERT INTO pending_notifications (message, created_at) VALUES (?1, ?2)",
            params![message, created_at],
        )?;
        Ok(())
    }

    /// Fetch queued notifications, oldest first.
    pub fn get_pending_notifications(&self, limit: usize) -> Result<Vec<PendingNotification>> {
        let conn = self.conn.lock_or_recover();
        let mut stmt = conn.prepare(
            "SELECT id, message, created_at, attempts
             FROM pending_notifications
             ORDER BY id ASC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(PendingNotification {
                id: row.get(0)?,
                message: row.get(1)?,
                created_at: row.get(2)?,
                attempts: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    /// Remove a notification from the queue (delivered or permanently failed).
    pub fn delete_notification(&self, id: i64) -> Result<()> {
        self.conn.lock_or_recover().execute(
            "DELETE FROM pending_notifications WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Record a failed delivery attempt.
    pub fn record_notification_attempt(&self, id: i64, attempted_at: i64) -> Result<()> {
        self.conn.lock_or_recover().execute(
            "UPDATE pending_notifications
             SET attempts = attempts + 1, last_attempt_at = ?2
             WHERE id = ?1",
            params![id, attempted_at],
        )?;
        Ok(())
    }
}

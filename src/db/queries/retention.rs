//! Historical data downsampling ("retention rollup").
//!
//! Raw readings are kept verbatim for the most recent [`RAW_RETENTION_SECS`].
//! Anything older is downsampled in place so the database (and therefore the
//! SQLite page cache on a RAM-starved Raspberry Pi Zero) stays small:
//!
//! * **energy** — a deadband + heartbeat sweep: keep a sample only when the
//!   power moved by at least [`ENERGY_DEADBAND_W`] watts from the previous
//!   sample, and always keep at least one sample per [`ENERGY_HEARTBEAT_SECS`]
//!   window so a flat signal still leaves anchor points.
//! * **temperature/humidity** — decimated to one sample per
//!   [`TEMP_BUCKET_SECS`] window (hourly).
//! * **presence** — left untouched (event-based and already tiny).
//!
//! The sweep is idempotent: re-running it over an already-downsampled window
//! deletes nothing, so it is safe to run on every startup and on a daily timer.
//! Work is done one day at a time with the connection lock released between
//! chunks so live HTTP queries can interleave.

use rusqlite::{Connection, Result, params};
use tracing::{debug, info};

use super::super::connection::{Database, MutexExt};

/// Keep raw data for the last 7 days; downsample everything older.
pub const RAW_RETENTION_SECS: i64 = 7 * 86_400;

/// Energy: minimum power delta (watts) to keep a sample. The deadband preserves
/// *every real power change* at its exact timestamp regardless of the heartbeat.
const ENERGY_DEADBAND_W: f64 = 50.0;
/// Energy: always keep at least one sample per this many seconds (1 h). This is
/// only a flat-signal anchor (and disambiguates "flat" from "offline") for data
/// older than a week — it does not affect which real changes are kept. At the
/// meter's ~4 s cadence this alone drives ~99.6 % row reduction; a shorter
/// interval mostly just stores redundant "still flat" points.
const ENERGY_HEARTBEAT_SECS: i64 = 3_600;
/// Temperature: decimate to one sample per this many seconds (1 hour).
const TEMP_BUCKET_SECS: i64 = 3_600;

/// One day per sweep chunk.
const CHUNK_SECS: i64 = 86_400;

/// Skip a day-chunk that already holds this few (or fewer) rows — it has almost
/// certainly been downsampled already, so scanning it again is wasted work.
/// A downsampled energy day is ~78 rows (24 heartbeat + real changes); a raw day
/// is ~21 600. The threshold sits well between the two (with margin for a few
/// energy devices).
const ENERGY_CHUNK_SKIP_BELOW: i64 = 300;
const TEMP_CHUNK_SKIP_BELOW: i64 = 80;

#[derive(Debug, Default, Clone, Copy)]
pub struct RetentionStats {
    pub energy_deleted: usize,
    pub temperature_deleted: usize,
    pub chunks_processed: usize,
}

impl Database {
    /// Downsample all reading tables older than [`RAW_RETENTION_SECS`] relative
    /// to `now` (a unix timestamp in seconds).
    pub fn run_retention(&self, now: i64) -> Result<RetentionStats> {
        let cutoff = now - RAW_RETENTION_SECS;
        let mut chunks = 0usize;

        let energy_deleted = self.sweep(
            "energy_readings",
            cutoff,
            ENERGY_CHUNK_SKIP_BELOW,
            &mut chunks,
            |conn, start, end| {
                conn.execute(
                    ENERGY_DELETE_SQL,
                    params![start, end, ENERGY_HEARTBEAT_SECS, ENERGY_DEADBAND_W],
                )
            },
        )?;

        let temperature_deleted = self.sweep(
            "temperature_readings",
            cutoff,
            TEMP_CHUNK_SKIP_BELOW,
            &mut chunks,
            |conn, start, end| {
                conn.execute(TEMP_DELETE_SQL, params![start, end, TEMP_BUCKET_SECS])
            },
        )?;

        let stats = RetentionStats {
            energy_deleted,
            temperature_deleted,
            chunks_processed: chunks,
        };

        info!(
            energy_deleted = stats.energy_deleted,
            temperature_deleted = stats.temperature_deleted,
            chunks = stats.chunks_processed,
            "Retention rollup completed"
        );

        Ok(stats)
    }

    /// Sweep a single table one day-chunk at a time, from its oldest row up to
    /// `cutoff`. The lock is taken and released per chunk. `delete_chunk`
    /// performs the table-specific DELETE and returns the number of rows removed.
    fn sweep(
        &self,
        table: &str,
        cutoff: i64,
        skip_below: i64,
        chunks: &mut usize,
        mut delete_chunk: impl FnMut(&Connection, i64, i64) -> Result<usize>,
    ) -> Result<usize> {
        let oldest: Option<i64> = {
            let conn = self.conn.lock_or_recover();
            let sql = format!("SELECT MIN(timestamp) FROM {table} WHERE timestamp < ?1");
            conn.query_row(&sql, params![cutoff], |row| row.get(0))?
        };

        let Some(oldest) = oldest else {
            debug!(table, "No rows older than cutoff; nothing to downsample");
            return Ok(0);
        };

        // Align chunk boundaries to whole days for stable, resumable sweeps.
        let mut start = (oldest / CHUNK_SECS) * CHUNK_SECS;
        let count_sql =
            format!("SELECT COUNT(*) FROM {table} WHERE timestamp >= ?1 AND timestamp < ?2");
        let mut deleted = 0usize;

        while start < cutoff {
            let end = (start + CHUNK_SECS).min(cutoff);
            let conn = self.conn.lock_or_recover();

            let count: i64 = conn.query_row(&count_sql, params![start, end], |row| row.get(0))?;
            if count > skip_below {
                let removed = delete_chunk(&conn, start, end)?;
                deleted += removed;
                *chunks += 1;
                debug!(table, start, end, count, removed, "Downsampled day chunk");
            }
            drop(conn); // release before the next chunk so HTTP queries can run
            start = end;
        }

        Ok(deleted)
    }
}

/// Deadband + heartbeat keep-set for energy readings within a window.
/// Params: `?1` start, `?2` end, `?3` heartbeat seconds, `?4` deadband watts.
const ENERGY_DELETE_SQL: &str = "DELETE FROM energy_readings
     WHERE timestamp >= ?1 AND timestamp < ?2
       AND id NOT IN (
         SELECT id FROM (
           SELECT id,
             row_number() OVER (
               PARTITION BY device_id, timestamp / ?3 ORDER BY timestamp
             ) AS rn,
             power - LAG(power) OVER (
               PARTITION BY device_id ORDER BY timestamp
             ) AS dp
           FROM energy_readings
           WHERE timestamp >= ?1 AND timestamp < ?2
         )
         WHERE rn = 1 OR dp IS NULL OR abs(dp) >= ?4
       )";

/// Hourly decimation keep-set for temperature readings within a window.
/// Params: `?1` start, `?2` end, `?3` bucket seconds.
const TEMP_DELETE_SQL: &str = "DELETE FROM temperature_readings
     WHERE timestamp >= ?1 AND timestamp < ?2
       AND id NOT IN (
         SELECT id FROM (
           SELECT id,
             row_number() OVER (
               PARTITION BY device_id, timestamp / ?3 ORDER BY timestamp
             ) AS rn
           FROM temperature_readings
           WHERE timestamp >= ?1 AND timestamp < ?2
         )
         WHERE rn = 1
       )";

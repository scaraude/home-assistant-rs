//! Cumulative-energy summaries for the energy dashboard.
//!
//! The meter reports `energy`/`produced_energy` as **monotonic lifetime
//! counters** (kWh since the meter was commissioned), so "how much did we use
//! this day/month?" is a *delta* of the counter, not an average of it (the
//! generic `AVG`-based aggregation in [`super::sensor`] is meaningless for a
//! counter). This module computes those deltas per time bucket.
//!
//! ## Why boundary lookups (not GROUP BY / window functions)
//!
//! Because the counter only grows, a bucket's consumption is just
//! `value(end) − value(start)`, where `value(t)` is the counter reading nearest
//! before `t`. So instead of scanning/sorting every raw sample in the range
//! (a week of 4-second samples is ~150k rows), we do one **index-backed point
//! lookup per bucket boundary**: `… WHERE device_id=? AND timestamp < t ORDER BY
//! timestamp DESC LIMIT 1`. A month is ~31 lookups per device, each O(log n).
//!
//! This matters on the Raspberry Pi Zero: the earlier `ROW_NUMBER()`/`GROUP BY`
//! version sorted the whole range in a temp B-tree that spilled to the SD card
//! and took several seconds; the boundary lookups are effectively instant.
//!
//! Consecutive boundaries share a lookup, so the deltas telescope and
//! `total_consumed` equals the sum of the buckets — histogram and hero agree.

use crate::models::{EnergyBucket, EnergySummary};
use rusqlite::{Result, ToSql, params};
use tracing::{debug, info, warn};

use super::super::connection::{Database, MutexExt};

/// Cap on the number of buckets a single summary will compute, so a pathological
/// `bucket` value (e.g. 1 second over a year) can't turn into millions of point
/// lookups. Normal callers ask for ~24–31 buckets; this only trips on abuse.
const MAX_BUCKETS: i64 = 1500;

impl Database {
    /// Compute a cumulative-energy summary for a period.
    ///
    /// `device_id = None` aggregates across every energy meter (per-device
    /// deltas summed per bucket). `peak_power` is the single highest power
    /// sample in the range (across meters when unfiltered).
    pub fn get_energy_summary(
        &self,
        device_id: Option<&str>,
        start_timestamp: i64,
        end_timestamp: i64,
        bucket_seconds: i64,
    ) -> Result<EnergySummary> {
        // Clamp the bucket up if the range would produce too many buckets.
        let span = (end_timestamp - start_timestamp).max(1);
        let mut bucket = bucket_seconds.max(1);
        if span / bucket > MAX_BUCKETS {
            bucket = span / MAX_BUCKETS + 1;
        }

        debug!(
            device_id = ?device_id,
            start = start_timestamp,
            end = end_timestamp,
            bucket_seconds = bucket,
            "Computing energy summary"
        );
        let started = std::time::Instant::now();

        let conn = self.conn.lock_or_recover();

        // --- devices in scope -------------------------------------------------
        let devices: Vec<String> = match device_id {
            Some(d) => vec![d.to_string()],
            None => {
                // Covering scan of (device_id, timestamp) index → distinct meters.
                let mut stmt =
                    conn.prepare("SELECT DISTINCT device_id FROM energy_readings")?;
                stmt.query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>>>()?
            }
        };

        // --- bucket boundaries ------------------------------------------------
        // `points[i]` is the start of bucket i; the extra final point (end+1)
        // is the upper bound of the last bucket, inclusive of a sample at `end`.
        let mut points: Vec<i64> = Vec::new();
        let mut b = start_timestamp;
        while b < end_timestamp {
            points.push(b);
            b += bucket;
        }
        points.push(end_timestamp + 1);
        let n_buckets = points.len().saturating_sub(1);

        // --- per-boundary counter value, per device --------------------------
        // Index-backed (device_id, timestamp DESC): one row per lookup.
        let mut value_stmt = conn.prepare(
            "SELECT energy, produced_energy
             FROM energy_readings
             WHERE device_id = ?1 AND timestamp < ?2
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let mut consumed = vec![0.0_f64; n_buckets];
        let mut produced = vec![0.0_f64; n_buckets];

        for device in &devices {
            // Cumulative counter value at each boundary (None before first data).
            let mut cum: Vec<Option<(f64, f64)>> = Vec::with_capacity(points.len());
            for &p in &points {
                let v = value_stmt
                    .query_row(params![device, p], |row| {
                        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))
                    })
                    .ok();
                cum.push(v);
            }
            for i in 0..n_buckets {
                if let (Some((e0, p0)), Some((e1, p1))) = (cum[i], cum[i + 1]) {
                    // Clamp ≥0 so a counter reset can't produce a negative bucket.
                    consumed[i] += (e1 - e0).max(0.0);
                    produced[i] += (p1 - p0).max(0.0);
                }
                // A `None` endpoint means no data yet → contribute nothing.
            }
        }

        // --- peak instantaneous power over the range -------------------------
        // SQLite returns the bare `timestamp` from the row holding MAX(power),
        // so this is a single streaming scan — no sort, no window, no subquery.
        let peak_filter = if device_id.is_some() {
            " AND device_id = ?3"
        } else {
            ""
        };
        let peak_sql = format!(
            "SELECT timestamp, MAX(power)
             FROM energy_readings
             WHERE timestamp >= ?1 AND timestamp <= ?2{peak_filter}"
        );
        let mut peak_params: Vec<&dyn ToSql> = vec![&start_timestamp, &end_timestamp];
        let dev = device_id.map(str::to_string);
        if let Some(ref d) = dev {
            peak_params.push(d);
        }
        let mut peak_stmt = conn.prepare(&peak_sql)?;
        let peak: Option<(i64, f64)> = peak_stmt
            .query_row(peak_params.as_slice(), |row| {
                // MAX(power) is NULL when the range is empty → no peak.
                match row.get::<_, Option<f64>>(1)? {
                    Some(power) => Ok(Some((row.get::<_, i64>(0)?, power))),
                    None => Ok(None),
                }
            })
            .unwrap_or(None);

        drop(value_stmt);
        drop(peak_stmt);

        // --- assemble ---------------------------------------------------------
        let buckets: Vec<EnergyBucket> = (0..n_buckets)
            .map(|i| EnergyBucket {
                timestamp: points[i],
                consumed: consumed[i],
                produced: produced[i],
            })
            .collect();

        let total_consumed = consumed.iter().sum();
        let total_produced = produced.iter().sum();

        let summary = EnergySummary {
            start: start_timestamp,
            end: end_timestamp,
            bucket_seconds: bucket,
            total_consumed,
            total_produced,
            peak_power: peak.map(|(_, p)| p).unwrap_or(0.0),
            peak_power_timestamp: peak.map(|(ts, _)| ts),
            buckets,
        };

        let elapsed = started.elapsed();
        info!(
            device_id = ?device_id,
            device_count = devices.len(),
            bucket_count = summary.buckets.len(),
            total_consumed = summary.total_consumed,
            duration_ms = elapsed.as_millis(),
            "Computed energy summary"
        );
        if summary.buckets.is_empty() {
            warn!(
                device_id = ?device_id,
                start = start_timestamp,
                end = end_timestamp,
                "No energy readings for summary range"
            );
        }

        Ok(summary)
    }
}

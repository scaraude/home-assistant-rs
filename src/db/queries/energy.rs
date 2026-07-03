//! Cumulative-energy summaries for the energy dashboard.
//!
//! The meter reports `energy`/`produced_energy` as **monotonic lifetime
//! counters** (kWh since the meter was commissioned), so "how much did we use
//! this day/month?" is a *delta* of the counter, not an average of it (the
//! generic `AVG`-based aggregation in [`super::sensor`] is meaningless for a
//! counter). This module computes those deltas per time bucket.
//!
//! Method, per device:
//! 1. Take the **last** counter sample in each bucket (window `ROW_NUMBER`).
//! 2. Take an **anchor** — the last counter sample strictly before the range.
//! 3. Difference consecutive bucket values (anchor → bucket 0 → bucket 1 → …),
//!    clamping negatives to zero so a counter reset can't produce a bogus
//!    negative bucket.
//!
//! Because the buckets telescope, the reported `total_consumed` is exactly the
//! sum of the per-bucket deltas — the histogram and the hero total always agree.

use std::collections::BTreeMap;

use crate::models::{EnergyBucket, EnergySummary};
use rusqlite::{Result, ToSql};
use tracing::{debug, info, warn};

use super::super::connection::{Database, MutexExt};

/// (bucket_ts, energy_counter, produced_counter) for a single device.
type BucketRow = (i64, f64, f64);

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
        let bucket = bucket_seconds.max(1);
        debug!(
            device_id = ?device_id,
            start = start_timestamp,
            end = end_timestamp,
            bucket_seconds = bucket,
            "Computing energy summary"
        );
        let started = std::time::Instant::now();

        // Owned copy so borrows in the dynamic param vectors outlive each query.
        let dev = device_id.map(str::to_string);

        let conn = self.conn.lock_or_recover();

        // --- 1. Last counter value per (device, bucket) within the range ------
        let bucket_filter = if dev.is_some() {
            " AND device_id = ?4"
        } else {
            ""
        };
        // Buckets are aligned to `start` (not the epoch) so the caller controls
        // the boundaries — the frontend passes local midnight / 1st-of-month, and
        // the bars line up with local days regardless of timezone.
        let bucket_sql = format!(
            "SELECT device_id, bucket_ts, energy, produced_energy
             FROM (
                 SELECT device_id,
                        ((timestamp - ?2) / ?1) * ?1 + ?2 AS bucket_ts,
                        energy, produced_energy,
                        ROW_NUMBER() OVER (
                            PARTITION BY device_id, ((timestamp - ?2) / ?1) * ?1 + ?2
                            ORDER BY timestamp DESC, rowid DESC
                        ) AS rn
                 FROM energy_readings
                 WHERE timestamp >= ?2 AND timestamp <= ?3{bucket_filter}
             )
             WHERE rn = 1
             ORDER BY device_id, bucket_ts"
        );
        let mut bucket_params: Vec<&dyn ToSql> =
            vec![&bucket, &start_timestamp, &end_timestamp];
        if let Some(ref d) = dev {
            bucket_params.push(d);
        }

        let mut stmt = conn.prepare(&bucket_sql)?;
        let rows = stmt
            .query_map(bucket_params.as_slice(), |row| {
                let device: String = row.get(0)?;
                Ok((device, (row.get::<_, i64>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?)))
            })?
            .collect::<Result<Vec<(String, BucketRow)>>>()?;

        // --- 2. Anchor: last counter value strictly before `start` -----------
        let anchor_filter = if dev.is_some() {
            " AND device_id = ?2"
        } else {
            ""
        };
        let anchor_sql = format!(
            "SELECT device_id, energy, produced_energy
             FROM (
                 SELECT device_id, energy, produced_energy,
                        ROW_NUMBER() OVER (
                            PARTITION BY device_id
                            ORDER BY timestamp DESC, rowid DESC
                        ) AS rn
                 FROM energy_readings
                 WHERE timestamp < ?1{anchor_filter}
             )
             WHERE rn = 1"
        );
        let mut anchor_params: Vec<&dyn ToSql> = vec![&start_timestamp];
        if let Some(ref d) = dev {
            anchor_params.push(d);
        }
        let mut stmt = conn.prepare(&anchor_sql)?;
        let anchors: std::collections::HashMap<String, (f64, f64)> = stmt
            .query_map(anchor_params.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, (row.get::<_, f64>(1)?, row.get::<_, f64>(2)?)))
            })?
            .collect::<Result<_>>()?;

        // --- 3. Peak instantaneous power over the range ----------------------
        let peak_filter = if dev.is_some() {
            " AND device_id = ?3"
        } else {
            ""
        };
        let peak_sql = format!(
            "SELECT timestamp, power
             FROM energy_readings
             WHERE timestamp >= ?1 AND timestamp <= ?2{peak_filter}
             ORDER BY power DESC, timestamp ASC
             LIMIT 1"
        );
        let mut peak_params: Vec<&dyn ToSql> = vec![&start_timestamp, &end_timestamp];
        if let Some(ref d) = dev {
            peak_params.push(d);
        }
        let mut stmt = conn.prepare(&peak_sql)?;
        let peak: Option<(i64, f64)> = stmt
            .query_row(peak_params.as_slice(), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })
            .ok();

        drop(stmt);

        // --- 4. Difference consecutive counter values, per device ------------
        // Rows are ordered by (device_id, bucket_ts); walk each device's run and
        // accumulate deltas into a shared per-bucket map so multiple meters sum.
        let mut agg: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        let mut cur_device: Option<&str> = None;
        let mut prev_energy: Option<f64> = None;
        let mut prev_produced: Option<f64> = None;

        for (device, (bucket_ts, energy, produced)) in &rows {
            if cur_device != Some(device.as_str()) {
                // New device run: seed from its anchor (None if no prior data).
                cur_device = Some(device.as_str());
                match anchors.get(device) {
                    Some((e, p)) => {
                        prev_energy = Some(*e);
                        prev_produced = Some(*p);
                    }
                    None => {
                        prev_energy = None;
                        prev_produced = None;
                    }
                }
            }

            let entry = agg.entry(*bucket_ts).or_insert((0.0, 0.0));
            if let Some(pe) = prev_energy {
                entry.0 += (energy - pe).max(0.0);
            }
            if let Some(pp) = prev_produced {
                entry.1 += (produced - pp).max(0.0);
            }
            prev_energy = Some(*energy);
            prev_produced = Some(*produced);
        }

        let buckets: Vec<EnergyBucket> = agg
            .into_iter()
            .map(|(timestamp, (consumed, produced))| EnergyBucket {
                timestamp,
                consumed,
                produced,
            })
            .collect();

        let total_consumed = buckets.iter().map(|b| b.consumed).sum();
        let total_produced = buckets.iter().map(|b| b.produced).sum();

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
            bucket_count = summary.buckets.len(),
            total_consumed = summary.total_consumed,
            total_produced = summary.total_produced,
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

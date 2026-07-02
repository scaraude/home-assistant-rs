#!/usr/bin/env bash
# One-time retroactive downsample of historical readings + VACUUM.
#
# Keeps RAW data for the last 7 days; downsamples everything older:
#   - energy:      deadband (>= DEADBAND_W watts change) + heartbeat (HEARTBEAT_SECS)
#   - temperature: decimated to one sample per TEMP_BUCKET_SECS (hourly)
#   - presence:    left untouched
#
# The keep logic MIRRORS src/db/queries/retention.rs. The RetentionService in
# the app self-heals over time and daily, but this script clears the existing
# multi-year backlog immediately and reclaims disk with VACUUM (which the
# service does not do).
#
# Run this ON THE PI (it uses sudo + sqlite3 and stops/starts the service).
# It backs the DB up first; nothing is destructive until after the backup.
set -euo pipefail

DB="${DB:-/var/lib/home-automation-rs/database/home_automation.db}"
SERVICE="${SERVICE:-home-automation-rs}"
NOW="$(date +%s)"
CUTOFF=$(( NOW - 7 * 86400 ))

DEADBAND_W=50       # keep a sample when power moves >= this many watts
HEARTBEAT_SECS=3600 # + one anchor per hour on flat signal (see retention.rs)
TEMP_BUCKET_SECS=3600

BACKUP="${DB%.db}.backup.$(date +%Y%m%d-%H%M%S).db"

echo "DB      = $DB"
echo "cutoff  = $CUTOFF (keep raw for last 7 days)"
echo "backup  = $BACKUP"
echo

echo "[1/6] Backing up database (safe point)…"
sudo sqlite3 "$DB" ".backup '$BACKUP'"
echo "      backup size: $(sudo du -h "$BACKUP" | cut -f1)"

echo "[2/6] Stopping $SERVICE (no writes during downsample)…"
sudo systemctl stop "$SERVICE"

echo "[3/6] Row counts BEFORE:"
sudo sqlite3 "$DB" \
  "SELECT 'energy', count(*) FROM energy_readings
   UNION ALL SELECT 'temperature', count(*) FROM temperature_readings;"

echo "[4/6] Downsampling data older than 7 days…"
sudo sqlite3 "$DB" <<SQL
-- Energy: keep first per heartbeat bucket OR power delta >= deadband.
DELETE FROM energy_readings
WHERE timestamp < $CUTOFF
  AND id NOT IN (
    SELECT id FROM (
      SELECT id,
        row_number() OVER (
          PARTITION BY device_id, timestamp / $HEARTBEAT_SECS ORDER BY timestamp
        ) AS rn,
        power - LAG(power) OVER (
          PARTITION BY device_id ORDER BY timestamp
        ) AS dp
      FROM energy_readings WHERE timestamp < $CUTOFF
    )
    WHERE rn = 1 OR dp IS NULL OR abs(dp) >= $DEADBAND_W
  );

-- Temperature: keep first per hourly bucket.
DELETE FROM temperature_readings
WHERE timestamp < $CUTOFF
  AND id NOT IN (
    SELECT id FROM (
      SELECT id,
        row_number() OVER (
          PARTITION BY device_id, timestamp / $TEMP_BUCKET_SECS ORDER BY timestamp
        ) AS rn
      FROM temperature_readings WHERE timestamp < $CUTOFF
    )
    WHERE rn = 1
  );
SQL

echo "[5/6] Row counts AFTER + VACUUM (reclaim space)…"
sudo sqlite3 "$DB" \
  "SELECT 'energy', count(*) FROM energy_readings
   UNION ALL SELECT 'temperature', count(*) FROM temperature_readings;"
sudo sqlite3 "$DB" "VACUUM; PRAGMA optimize;"
echo "      DB size now: $(sudo du -h "$DB" | cut -f1)"

echo "[6/6] Restarting $SERVICE…"
sudo systemctl start "$SERVICE"
sleep 2
systemctl is-active "$SERVICE"

echo
echo "Done. Backup kept at: $BACKUP"
echo "If anything looks wrong: sudo systemctl stop $SERVICE && sudo cp '$BACKUP' '$DB' && sudo systemctl start $SERVICE"

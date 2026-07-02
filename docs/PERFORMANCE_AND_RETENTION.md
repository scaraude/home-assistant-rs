# Performance & data retention mechanics

This document explains how the app keeps the UI fast and the SQLite database
small on very constrained hardware (a **Raspberry Pi Zero 2 W**: 4 cores but
only **~416 MB RAM** and a cheap SD card). Read this before touching the
floor-plan endpoint, the readings/aggregation queries, or the retention job.

## The hardware reality

- ~416 MB RAM, ~185 MB free at runtime. The DB **does not fit** in RAM, so any
  query that scans a large table hits the SD card directly (slow random reads).
- Therefore the golden rules are: **keep the DB small**, **never send large
  uncompressed payloads**, and **never return unbounded row counts**.

## 1. Floor-plan delivery (3 cooperating cache layers)

The floor plan is a single ~2.5 MB SVG stored in the `floor_plan` table and
served by `GET /api/floor-plan` as JSON (`{ svg_content, uploaded_at }`).

Three independent layers keep it cheap — they are **additive**, each covering a
case the others cannot:

| Layer | Where | Covers |
|-------|-------|--------|
| **IndexedDB** | `frontend/src/lib/memory/floorPlanMemory.ts` (store `floor_plan`) | Repeat visits on the same browser — avoids calling `fetch` at all once `loaded` is set. |
| **ETag / 304** | `src/http/routes/floor_plan.rs` + `responses.rs` (`json_response_with_etag`, `not_modified_response`) | Any `fetch` that *does* happen (first `ensureFloorPlan`, `force` refresh, or the hydrate race): a weak ETag `W/"<uploaded_at>-<len>"` lets the server answer `304` (~0 bytes). `Cache-Control: no-cache` forces revalidation. |
| **gzip** | `src/http/compression.rs`, wired in `src/http/mod.rs::handle_request` | The one unavoidable full transfer (fresh browser, cleared cache): text SVG compresses ~85–90 %. |

> `Cache-Control: max-age` is deliberately **not** used: it would be redundant
> with the IndexedDB gate and could serve stale plans.

**Most fundamental lever:** shrink the SVG source itself with SVGO
(`scripts/optimize_floor_plan.sh`) — that reduces the DB row, the IndexedDB
entry, *and* every transfer at once.

### gzip compression (general)

`compression::maybe_compress` runs on **every** response in `handle_request`
after the handler returns. It gzips the in-memory `Full<Bytes>` body when the
client sent `Accept-Encoding: gzip` and the body is ≥ 1 KB. It skips
already-encoded bodies and WebSocket `101` upgrades. This benefits all JSON
endpoints, not just the floor plan.

## 2. Readings: bounded payloads via server-side aggregation

`temperature_readings`, `presence_readings`, `energy_readings` are time series.
The energy meter reports **every ~4 s** → tens of thousands of rows per day.

`GET /api/readings` supports a `bucket` (seconds) parameter. When `bucket > 1`
the server aggregates (`AVG` for numeric, `MAX` for presence) into
`(timestamp / bucket) * bucket` buckets — see
`src/db/queries/sensor.rs::get_aggregated_readings_*`.

**The frontend must never request raw high-frequency data for a wide range.**
`frontend/src/lib/api/sensors.ts::fetchReadings` therefore **auto-buckets** to
~`TARGET_POINTS` (500) points for the requested window unless a caller passes an
explicit `bucketSeconds`. Retention (below) only downsamples data **older than 7
days**, so the recent window is still raw in the DB — the frontend bucket is
what keeps the default dashboard fast. Callers needing true raw samples pass
`bucketSeconds = 1`.

Indexes: composite `(device_id, timestamp DESC)` for per-device queries, plus
timestamp-only `idx_energy_ts` / `idx_temperature_ts` for cross-device range and
retention scans (see `src/db/schema.rs`).

## 3. Retention rollup (keeps the DB small)

Policy: **keep raw data for the last 7 days; downsample everything older, in all
cases.** Implemented in `src/db/queries/retention.rs`
(`Database::run_retention`) and driven by `src/services/retention.rs`
(`RetentionService`: first run ~60 s after boot, then every 24 h).

Per table, for data older than `RAW_RETENTION_SECS` (7 days):

- **energy** — *deadband + heartbeat*: keep a sample when power moved by
  ≥ `ENERGY_DEADBAND_W` (50 W) from the previous sample, **and** always keep at
  least one sample per `ENERGY_HEARTBEAT_SECS` (1 h) window (a flat-signal anchor
  that also distinguishes "flat" from "offline"). The deadband preserves *every
  real power change at its exact timestamp*; the heartbeat only adds redundant
  points during plateaus, so it is the dominant cost — at the meter's ~4 s
  cadence, 1 h gives **~99.6 %** reduction vs ~98.4 % at 5 min while losing no
  real events. Tune `ENERGY_HEARTBEAT_SECS` alone to trade historical flat-period
  resolution against row count; it barely affects DB size either way.
- **temperature/humidity** — decimated to one sample per `TEMP_BUCKET_SECS`
  (1 h).
- **presence** — untouched (event-based, tiny).

Key properties:
- **Idempotent**: re-running over an already-downsampled window deletes nothing
  (each surviving row is the `rn = 1` of its bucket), so running on every
  startup + daily is safe. A per-day-chunk row-count guard skips
  already-processed days.
- **Chunked by day**, releasing the DB lock between chunks so live HTTP queries
  interleave. Runs on the blocking pool (`spawn_blocking`).

### One-time backfill

The daily service self-heals over time, but to reclaim the existing multi-year
backlog immediately (and run a `VACUUM`, which the service does not do), run
`scripts/downsample_backfill.sh` **once** on the Pi. It backs up the DB, stops
the service, applies the same keep-sets in one pass, `VACUUM`s, and restarts.
The deadband keep logic there **mirrors** `retention.rs` — keep them in sync.

## 4. Operational notes (not in code)

- **Logs**: the app logs at `info` by default (every HTTP request). On the Pi,
  set `RUST_LOG=warn` in `/opt/home-automation-rs/.env`. Rotate
  `/var/log/home-automation-rs/*.log` and the `monitor.sh` logs in
  `/home/ludovic` (they grow unbounded — `home-automation-rs.log` and
  `zigbee2mqtt.log` reached multiple GB). `top_cpu_consumers.log` /
  `top_ram_consumers.log` are **not read** by the app.
- **Deploy**: cross-compiled to `aarch64` via `cross` (Docker); `make
  deploy-both` builds + transfers binary and frontend and restarts the service.

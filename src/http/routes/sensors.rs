use crate::db::Database;
use crate::http::query::QueryParams;
use crate::http::responses::*;
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;
use tracing::{debug, error, info};

pub fn serve_sensors(db: &Database) -> Response<Full<Bytes>> {
    debug!("Querying database for all sensors");

    match db.get_all_sensor_devices() {
        Ok(sensors) => {
            debug!(
                sensor_count = sensors.len(),
                "Retrieved sensors from database"
            );

            let json = match serialize_to_json(&sensors, "sensors list") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                sensor_count = sensors.len(),
                response_size = json.len(),
                "Successfully serialized sensors to JSON"
            );

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching sensors");
            internal_error_response("Database error")
        }
    }
}

pub fn serve_readings(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    debug!(query = ?query, "Parsing query parameters");

    // Parse query parameters for device_id, hours, since, start/end, and bucket
    let params = QueryParams::new(query);
    let device_id = params.get("device_id");
    let since_param = params.get_optional_i64("since");
    let start_param = params.get_optional_i64("start");
    let end_param = params.get_optional_i64("end");
    let bucket_seconds = params.get_optional_i64("bucket").unwrap_or(1).max(1);

    // Determine time range: prefer explicit start/end, otherwise use since/hours
    let (start_ts, end_ts) = if let (Some(start), Some(end)) = (start_param, end_param) {
        (start, end)
    } else {
        let since = since_param.unwrap_or_else(|| {
            let hours = params.get_i64("hours", 24);
            chrono::Utc::now().timestamp() - (hours * 3600)
        });
        (since, chrono::Utc::now().timestamp())
    };

    info!(
        device_id = ?device_id,
        start_timestamp = start_ts,
        end_timestamp = end_ts,
        bucket_seconds = bucket_seconds,
        "Querying readings (range + aggregation support enabled)"
    );

    // Use SQL-filtered queries - no more Rust-side filtering!
    let result = if bucket_seconds <= 1 {
        if let Some(sid) = device_id {
            debug!(device_id = %sid, start = start_ts, end = end_ts, "Querying raw readings for specific sensor");
            db.get_readings_for_sensor_range(sid, start_ts, end_ts)
        } else {
            debug!(start = start_ts, end = end_ts, "Querying raw readings for all sensors");
            db.get_readings_range(start_ts, end_ts)
        }
    } else if let Some(sid) = device_id {
        debug!(
            device_id = %sid,
            start = start_ts,
            end = end_ts,
            bucket_seconds = bucket_seconds,
            "Querying aggregated readings for specific sensor"
        );
        db.get_aggregated_readings_for_sensor_range(sid, start_ts, end_ts, bucket_seconds)
    } else {
        debug!(
            start = start_ts,
            end = end_ts,
            bucket_seconds = bucket_seconds,
            "Querying aggregated readings for all sensors"
        );
        db.get_aggregated_readings_range(start_ts, end_ts, bucket_seconds)
    };

    match result {
        Ok(readings) => {
            debug!(
                reading_count = readings.len(),
                "Retrieved readings from database"
            );

            // Find latest timestamp for delta tracking
            let latest_timestamp = readings
                .iter()
                .map(|r| r.timestamp().timestamp())
                .max()
                .unwrap_or(end_ts);

            let json = match serialize_to_json(&readings, "sensor readings") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                reading_count = readings.len(),
                response_size = json.len(),
                device_id = ?device_id,
                latest_ts = latest_timestamp,
                "Successfully serialized readings to JSON"
            );

            json_response_with_timestamp(json, latest_timestamp)
        }
        Err(e) => {
            error!(error = %e, device_id = ?device_id, start = start_ts, end = end_ts, "Database error while fetching readings");
            internal_error_response("Database error")
        }
    }
}

use crate::db::Database;
use crate::http::query::QueryParams;
use crate::http::responses::*;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
use tracing::{debug, error, info};

pub fn serve_sensors(db: &Database) -> Response<Full<Bytes>> {
    debug!("Querying database for all sensors");

    match db.get_all_sensors() {
        Ok(sensors) => {
            debug!(
                sensor_count = sensors.len(),
                "Retrieved sensors from database"
            );

            let json = match serialize_to_json(&sensors, "sensors list") {
                Ok(json) => json,
                Err(response) => return response,
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

    // Parse query parameters for device_id, hours, and since
    let params = QueryParams::new(query);
    let device_id = params.get("device_id");
    let since_param = params.get_optional_i64("since");

    // Determine timestamp: use 'since' if provided, otherwise calculate from 'hours'
    let since = since_param.unwrap_or_else(|| {
        let hours = params.get_i64("hours", 24);
        chrono::Utc::now().timestamp() - (hours * 3600)
    });

    info!(
        device_id = ?device_id,
        since_timestamp = since,
        "Querying readings (delta support enabled)"
    );

    // Use SQL-filtered queries - no more Rust-side filtering!
    let result = if let Some(sid) = device_id {
        debug!(device_id = %sid, since = since, "Querying readings for specific sensor");
        // Get readings for specific sensor with time filter in SQL
        db.get_readings_for_sensor_since(sid, since)
    } else {
        debug!(since = since, "Querying readings for all sensors");
        // Get all readings since timestamp
        db.get_readings_since(since)
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
                .unwrap_or(since);

            let json = match serialize_to_json(&readings, "sensor readings") {
                Ok(json) => json,
                Err(response) => return response,
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
            error!(error = %e, device_id = ?device_id, since = since, "Database error while fetching readings");
            internal_error_response("Database error")
        }
    }
}

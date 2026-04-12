use crate::db::Database;
use crate::events::SystemEvent;
use crate::events::bus::EventBus;
use crate::http::responses::*;
use crate::models::DevicePosition;
use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// GET /api/devices/positions - Get all device positions
pub fn serve_device_positions(db: &Arc<Database>) -> Response<Full<Bytes>> {
    debug!("Getting all device positions from database");

    match db.get_all_device_positions() {
        Ok(positions) => {
            debug!(
                position_count = positions.len(),
                "Retrieved device positions from database"
            );

            let json = match serialize_to_json(&positions, "device positions") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                position_count = positions.len(),
                response_size = json.len(),
                "Successfully serialized device positions to JSON"
            );

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching device positions");
            internal_error_response("Database error")
        }
    }
}

#[derive(serde::Deserialize)]
pub struct PositionUpdate {
    pub x: f64,
    pub y: f64,
}

/// PUT /api/devices/{id}/position - Update device position
pub async fn update_device_position(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
    event_bus: &EventBus,
    path: &str,
) -> Response<Full<Bytes>> {
    debug!(path = %path, "Parsing device position update request");

    // Extract device ID from path (e.g., "/api/devices/0x123abc/position")
    let device_id = path
        .strip_prefix("/api/devices/")
        .and_then(|s| s.strip_suffix("/position"))
        .unwrap_or("");

    if device_id.is_empty() {
        warn!("Missing device ID in path");
        return bad_request_response("Missing device ID");
    }

    // Verify device exists
    match db._get_device_by_id(device_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!(device_id = %device_id, "Device not found for position update");
            return not_found_response("Device not found");
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to fetch device");
            return internal_error_response("Failed to fetch device");
        }
    }

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Parse the position update payload
    let update: PositionUpdate = match serde_json::from_slice::<PositionUpdate>(&body_bytes) {
        Ok(upd) => {
            debug!(
                device_id = %device_id,
                x = upd.x,
                y = upd.y,
                "Parsed position update"
            );
            upd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse position update JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Create the position model
    let position = DevicePosition::new(device_id.to_string(), update.x, update.y);

    // Upsert position to database
    if let Err(e) = db.upsert_device_position(&position) {
        error!(
            error = %e,
            device_id = %device_id,
            "Failed to update device position in database"
        );
        return internal_error_response("Failed to update position");
    }

    info!(
        device_id = %device_id,
        x = update.x,
        y = update.y,
        "Device position updated successfully"
    );

    // Emit WebSocket event for real-time sync
    let event = SystemEvent::DevicePositionUpdated {
        device_id: device_id.to_string(),
        x: update.x,
        y: update.y,
        timestamp: Utc::now(),
    };

    if let Err(e) = event_bus.publish(event) {
        // Log but don't fail the request - position is already saved
        debug!(error = %e, "Failed to publish position update event (no subscribers)");
    }

    json_response(r#"{"status":"ok"}"#.into())
}

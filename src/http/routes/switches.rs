use crate::db::Database;
use crate::http::responses::*;
use crate::models::{SwitchCommand, SwitchCommandMessage};
use crate::mqtt::topic::ZigbeeTopic;
use crate::mqtt::MqttClient;
use crate::state::{DeviceStateStore, SwitchStateStore};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub fn serve_switches_list(
    db: &Arc<Database>,
    switch_state: &Arc<SwitchStateStore>,
    _device_state: &DeviceStateStore,
) -> Response<Full<Bytes>> {
    debug!("Getting list of available switches from database");

    // Get all devices from database and filter for commanders (switches)
    let devices = match db.get_all_devices() {
        Ok(devices) => devices,
        Err(e) => {
            error!(error = %e, "Failed to query devices from database");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Filter for commander devices and build switch list
    let switches: Vec<_> = devices
        .into_iter()
        .filter(|device| {
            device
                .capabilities
                .iter()
                .any(|cap| matches!(cap, crate::models::DeviceCapability::Commander { .. }))
        })
        .map(|device| {
            let current_state = switch_state.get_state(&device.id).unwrap_or_default();

            // Fetch device state from database instead of in-memory store
            let device_db_state = db.get_device_state(&device.id).ok().flatten();
            let link_quality = device_db_state.as_ref().and_then(|s| s.link_quality);
            let battery_level = device_db_state.as_ref().and_then(|s| s.battery_level);
            let last_seen = device_db_state.as_ref().map(|s| s.last_seen.timestamp());

            info!(
                device_id = %device.id,
                mqtt_topic = %device.mqtt_topic,
                name = %device.name,
                state = %current_state,
                link_quality = ?link_quality,
                battery_level = ?battery_level,
                "Retrieved switch state from database"
            );

            serde_json::json!({
                "id": device.id,
                "mqtt_topic": device.mqtt_topic,
                "name": device.name,
                "state": current_state.to_bool(),
                "link_quality": link_quality,
                "battery_level": battery_level,
                "last_seen": last_seen,
            })
        })
        .collect();

    let json = match serialize_to_json(&switches, "switches list") {
        Ok(json) => json,
        Err(response) => return *response,
    };

    info!(
        switch_count = switches.len(),
        response_size = json.len(),
        "Successfully serialized switches list to JSON"
    );
    json_response(json)
}

pub fn serve_device_state(db: &Arc<Database>, device_id: &str) -> Response<Full<Bytes>> {
    debug!(device_id = %device_id, "Getting device state from database");

    match db.get_device_state(device_id) {
        Ok(Some(state)) => {
            let json_data = serde_json::json!({
                "device_id": state.device_id,
                "battery_level": state.battery_level,
                "link_quality": state.link_quality,
                "turbo_mode": state.turbo_mode,
                "last_seen": state.last_seen.timestamp(),
            });

            let json = match serialize_to_json(&json_data, "device state") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                device_id = %device_id,
                battery_level = ?state.battery_level,
                link_quality = ?state.link_quality,
                turbo_mode = ?state.turbo_mode,
                "Successfully retrieved device state"
            );
            json_response(json)
        }
        Ok(None) => {
            debug!(device_id = %device_id, "No device state found");
            not_found_response("Device state not found")
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Database error while fetching device state");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_device_states(db: &Arc<Database>) -> Response<Full<Bytes>> {
    debug!("Getting all device states from database");

    match db.get_all_device_states() {
        Ok(states) => {
            let json = match serialize_to_json(&states, "device states") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                device_state_count = states.len(),
                response_size = json.len(),
                "Successfully retrieved device states"
            );
            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching device states");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub async fn execute_command(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
    mqtt: &Arc<Mutex<MqttClient>>,
) -> Response<Full<Bytes>> {
    debug!("Parsing command request body");

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Parse the command
    let command: SwitchCommand = match serde_json::from_slice::<SwitchCommand>(&body_bytes) {
        Ok(cmd) => {
            debug!(device_id = %cmd.device_id, state = %cmd.state, "Parsed command");
            cmd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse command JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Look up the MQTT topic for this device ID
    let mqtt_topic = match db.get_mqtt_topic_for_device(&command.device_id) {
        Ok(Some(topic)) => {
            debug!(
                device_id = %command.device_id,
                mqtt_topic = %topic,
                "Found MQTT topic for device"
            );
            topic
        }
        Ok(None) => {
            warn!(device_id = %command.device_id, "Device not found in database");
            return not_found_response("Device not found");
        }
        Err(e) => {
            error!(error = %e, device_id = %command.device_id, "Failed to query device from database");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Build the MQTT payload
    let payload = match serde_json::to_string(&SwitchCommandMessage {
        state: command.state,
    }) {
        Ok(json) => json,
        Err(e) => {
            error!(
                error = %e,
                device_id = %command.device_id,
                "Failed to serialize MQTT command payload"
            );
            return internal_error_response("Failed to serialize command payload");
        }
    };

    info!(
        device_id = %command.device_id,
        mqtt_topic = %mqtt_topic,
        state = %command.state,
        payload = %payload,
        "Executing switch command"
    );

    // Publish to MQTT using the actual MQTT topic
    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_command(&mqtt_topic, &payload).await {
        Ok(_) => {
            info!(
                device_id = %command.device_id,
                mqtt_topic = %mqtt_topic,
                state = %command.state,
                "Command executed successfully"
            );
            json_response(r#"{"status":"ok"}"#.into())
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %command.device_id,
                mqtt_topic = %mqtt_topic,
                "Failed to publish command to MQTT"
            );
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct PermitJoinRequest {
    value: bool,
    time: Option<u64>,
}

#[derive(serde::Serialize)]
struct PermitJoinPayload {
    value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<u64>,
}

pub async fn permit_join(
    req: Request<hyper::body::Incoming>,
    mqtt: &Arc<Mutex<MqttClient>>,
) -> Response<Full<Bytes>> {
    debug!("Parsing permit join request body");

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    let request: PermitJoinRequest = match serde_json::from_slice::<PermitJoinRequest>(&body_bytes)
    {
        Ok(req) => {
            debug!(
                value = req.value,
                time = req.time,
                "Parsed permit join request"
            );
            req
        }
        Err(e) => {
            error!(error = %e, "Failed to parse permit join JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    let payload = PermitJoinPayload {
        value: request.value,
        time: if request.value {
            Some(request.time.unwrap_or(180))
        } else {
            None
        },
    };

    let payload_json = match serde_json::to_string(&payload) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "Failed to serialize permit join payload");
            return internal_error_response("Failed to serialize payload");
        }
    };

    info!(
        value = request.value,
        time = payload.time,
        payload = %payload_json,
        "Publishing Zigbee2MQTT permit_join request"
    );

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard
        .publish_bridge_request("permit_join", &payload_json)
        .await
    {
        Ok(_) => json_response(r#"{"status":"ok"}"#.into()),
        Err(e) => {
            error!(error = %e, "Failed to publish permit_join to MQTT");
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceUpdate {
    name: String,
}

pub async fn update_device(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
    path: &str,
) -> Response<Full<Bytes>> {
    debug!("Parsing device update request");

    // Extract device ID from path (e.g., "/api/devices/0x123abc")
    let device_id = path.strip_prefix("/api/devices/").unwrap_or("");

    if device_id.is_empty() {
        warn!("Missing device ID in path");
        return bad_request_response("Missing device ID");
    }

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Parse the update payload
    let update: DeviceUpdate = match serde_json::from_slice::<DeviceUpdate>(&body_bytes) {
        Ok(upd) => {
            debug!(device_id = %device_id, name = %upd.name, "Parsed device update");
            upd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse update JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Validate name is not empty
    if update.name.trim().is_empty() {
        warn!(device_id = %device_id, "Device name cannot be empty");
        return bad_request_response("Device name cannot be empty");
    }

    info!(
        device_id = %device_id,
        new_name = %update.name,
        "Updating device name"
    );

    // Update the device name in the database
    match db.update_device_name(device_id, &update.name) {
        Ok(_) => {
            info!(
                device_id = %device_id,
                new_name = %update.name,
                "Device name updated successfully"
            );
            json_response(r#"{"status":"ok"}"#.into())
        }
        Err(e) => {
            error!(
                error = %e,
                device_id = %device_id,
                "Failed to update device name in database"
            );
            internal_error_response(e.to_string().as_str())
        }
    }
}

#[derive(serde::Deserialize)]
struct DeviceOptionUpdate {
    turbo_mode: Option<bool>,
}

pub async fn set_device_option(
    req: Request<hyper::body::Incoming>,
    mqtt: &Arc<Mutex<MqttClient>>,
    db: &Arc<Database>,
    path: &str,
) -> Response<Full<Bytes>> {
    debug!("Parsing device option update request");

    let device_path = path.strip_prefix("/api/devices/").unwrap_or("");
    let device_id = device_path.strip_suffix("/set").unwrap_or("");

    if device_id.is_empty() {
        warn!("Missing device ID in path");
        return bad_request_response("Missing device ID");
    }

    let device = match db._get_device_by_id(device_id) {
        Ok(Some(device)) => device,
        Ok(None) => {
            warn!(device_id = %device_id, "Device not found for option update");
            return not_found_response("Device not found");
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to fetch device for option update");
            return internal_error_response("Failed to fetch device");
        }
    };

    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    let update: DeviceOptionUpdate = match serde_json::from_slice::<DeviceOptionUpdate>(&body_bytes)
    {
        Ok(upd) => upd,
        Err(e) => {
            error!(error = %e, "Failed to parse device option update JSON");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    let turbo_mode = match update.turbo_mode {
        Some(value) => value,
        None => {
            warn!(device_id = %device_id, "Missing turbo_mode in update payload");
            return bad_request_response("Missing turbo_mode value");
        }
    };

    let payload = serde_json::json!({ "turbo_mode": turbo_mode });
    let payload_json = match serde_json::to_string(&payload) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "Failed to serialize turbo_mode payload");
            return internal_error_response("Failed to serialize payload");
        }
    };

    info!(
        device_id = %device_id,
        mqtt_topic = %device.mqtt_topic,
        turbo_mode,
        payload = %payload_json,
        "Publishing turbo_mode update"
    );

    let mqtt_target = ZigbeeTopic::parse(&device.mqtt_topic)
        .and_then(|topic| topic.device_id())
        .unwrap_or(device.mqtt_topic.as_str());

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_command(mqtt_target, &payload_json).await {
        Ok(_) => json_response(r#"{"status":"ok"}"#.into()),
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to publish turbo_mode update");
            internal_error_response(e.to_string().as_str())
        }
    }
}

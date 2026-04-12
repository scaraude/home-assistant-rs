use crate::db::Database;
use crate::http::responses::*;
use crate::models::{NetworkEdge, NetworkTopology};
use crate::mqtt::MqttClient;
use crate::state::DeviceStateStore;
use crate::system::storage;
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

const STORAGE_CACHE_TTL: Duration = Duration::from_secs(60);

struct StorageCache {
    last_updated: Option<Instant>,
    payload: Option<storage::StorageBreakdown>,
}

impl StorageCache {
    fn new() -> Self {
        Self {
            last_updated: None,
            payload: None,
        }
    }
}

static STORAGE_CACHE: OnceLock<Mutex<StorageCache>> = OnceLock::new();

pub fn serve_network_topology(
    db: &Database,
    device_state: &DeviceStateStore,
) -> Response<Full<Bytes>> {
    debug!("Fetching network topology");

    match db.get_all_devices() {
        Ok(devices) => {
            // Compute edges from parent_device_id relationships
            let mut edges = Vec::new();
            let mut device_state_map = HashMap::new();

            // Build a map of device states for quick link quality lookup
            for device in &devices {
                if let Some(state) = device_state.get_state(&device.id) {
                    device_state_map.insert(device.id.clone(), state);
                }
            }

            // Create edges from parent-child relationships
            for device in &devices {
                if let Some(parent_id) = &device.parent_device_id {
                    let link_quality = device_state_map
                        .get(&device.id)
                        .and_then(|state| state.link_quality);

                    edges.push(NetworkEdge {
                        source_id: parent_id.clone(),
                        target_id: device.id.clone(),
                        link_quality,
                    });
                }
            }

            let topology = NetworkTopology { devices, edges };

            let json = match serialize_to_json(&topology, "network topology") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                device_count = topology.devices.len(),
                edge_count = topology.edges.len(),
                "Served network topology"
            );
            json_response(json)
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch devices for topology");
            internal_error_response("Database error")
        }
    }
}

pub async fn refresh_network_map(mqtt: &Arc<Mutex<MqttClient>>) -> Response<Full<Bytes>> {
    debug!("Refreshing network map");

    // Publish networkmap request to zigbee2mqtt
    let topic = "zigbee2mqtt/bridge/request/networkmap";
    let payload = r#"{"type":"raw","routes":true}"#;

    let mqtt_guard = mqtt.lock().await;
    match mqtt_guard.publish_raw(topic, payload).await {
        Ok(_) => {
            info!("Successfully published networkmap request");
            json_response(r#"{"status":"ok","message":"Network map refresh requested"}"#.into())
        }
        Err(e) => {
            error!(error = %e, "Failed to publish networkmap request");
            internal_error_response("Failed to request network map")
        }
    }
}

pub async fn serve_storage_breakdown() -> Response<Full<Bytes>> {
    let cache = STORAGE_CACHE.get_or_init(|| Mutex::new(StorageCache::new()));

    {
        let guard = cache.lock().await;
        if let (Some(last_updated), Some(payload)) = (&guard.last_updated, &guard.payload)
            && last_updated.elapsed() < STORAGE_CACHE_TTL
        {
            let json = match serialize_to_json(payload, "storage breakdown") {
                Ok(json) => json,
                Err(response) => return *response,
            };
            return json_response(json);
        }
    }

    let result = tokio::task::spawn_blocking(storage::compute_storage_breakdown).await;
    let breakdown = match result {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => {
            error!(error = %err, "Failed to compute storage breakdown");
            return internal_error_response("Failed to compute storage breakdown");
        }
        Err(err) => {
            error!(error = ?err, "Storage breakdown task failed");
            return internal_error_response("Failed to compute storage breakdown");
        }
    };

    {
        let mut guard = cache.lock().await;
        guard.last_updated = Some(Instant::now());
        guard.payload = Some(breakdown.clone());
    }

    let json = match serialize_to_json(&breakdown, "storage breakdown") {
        Ok(json) => json,
        Err(response) => return *response,
    };

    json_response(json)
}

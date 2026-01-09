use crate::db::Database;
use crate::events::SystemEvent;
use crate::models::{Device, DeviceCapability, PowerSource};
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Zigbee2MQTT networkmap response structure
#[derive(Debug, Deserialize)]
pub struct NetworkMapResponse {
    pub data: NetworkMapData,
}

#[derive(Debug, Deserialize)]
pub struct NetworkMapData {
    pub routes: bool,
    #[serde(rename = "type")]
    pub map_type: Option<String>,
    pub value: NetworkMapValue,
}

#[derive(Debug, Deserialize)]
pub struct NetworkMapValue {
    pub nodes: Vec<NetworkNode>,
    pub links: Vec<NetworkLink>,
}

#[derive(Debug, Deserialize)]
pub struct NetworkNode {
    #[serde(rename = "friendlyName")]
    pub friendly_name: Option<String>,
    #[serde(rename = "ieeeAddr")]
    pub ieee_addr: String,
    #[serde(rename = "type")]
    pub node_type: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkLink {
    pub relationship: u8,
    #[serde(rename = "linkquality")]
    pub link_quality: Option<u8>,
    pub source: NetworkEndpoint,
    pub target: NetworkEndpoint,
}

#[derive(Debug, Deserialize)]
pub struct NetworkEndpoint {
    #[serde(rename = "ieeeAddr")]
    pub ieee_addr: String,
}

/// Pairing event from bridge
#[derive(Debug, Deserialize)]
pub struct PairingEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: PairingEventData,
}

#[derive(Debug, Deserialize)]
pub struct PairingEventData {
    pub friendly_name: String,
    #[serde(rename = "ieee_address")]
    pub ieee_address: String,
}

/// Handle zigbee2mqtt bridge responses (e.g., networkmap)
pub async fn handle_bridge_response(
    payload: &str,
    db: &Database,
    event_tx: &broadcast::Sender<SystemEvent>,
) {
    debug!(payload = %payload, "Received bridge response");

    // Try to parse as networkmap response
    match serde_json::from_str::<NetworkMapResponse>(payload) {
        Ok(response) => {
            info!(
                node_count = response.data.value.nodes.len(),
                "Processing networkmap response"
            );
            process_network_map(response, db, event_tx).await;
        }
        Err(e) => {
            debug!(error = %e, "Not a networkmap response, ignoring");
        }
    }
}

/// Handle zigbee2mqtt bridge events (e.g., device_joined, device_interview)
pub async fn handle_bridge_event(
    payload: &str,
    _db: &Database,
    event_tx: &broadcast::Sender<SystemEvent>,
) {
    debug!(payload = %payload, "Received bridge event");

    // Try to parse as pairing event
    match serde_json::from_str::<PairingEvent>(payload) {
        Ok(event) => {
            if event.event_type == "device_joined" || event.event_type == "device_interview" {
                info!(
                    event_type = %event.event_type,
                    ieee_address = %event.data.ieee_address,
                    friendly_name = %event.data.friendly_name,
                    "Device pairing event"
                );

                if let Err(e) = event_tx.send(SystemEvent::DevicePairing {
                    ieee_addr: event.data.ieee_address.clone(),
                    friendly_name: event.data.friendly_name.clone(),
                }) {
                    warn!(error = %e, "Failed to send DevicePairing event");
                }
            }
        }
        Err(e) => {
            debug!(error = %e, "Not a pairing event, ignoring");
        }
    }
}

/// Process network map and update device topology in database
async fn process_network_map(
    response: NetworkMapResponse,
    db: &Database,
    event_tx: &broadcast::Sender<SystemEvent>,
) {
    let mut node_types = std::collections::HashMap::new();
    for node in &response.data.value.nodes {
        node_types.insert(node.ieee_addr.clone(), node.node_type.clone());

        if node.node_type == "Coordinator" {
            ensure_coordinator_device(db, node);
        }
    }

    let mut parent_by_child = std::collections::HashMap::new();
    for link in &response.data.value.links {
        let source_ieee = link.source.ieee_addr.clone();
        let target_ieee = link.target.ieee_addr.clone();

        match link.relationship {
            // Target is child of source.
            0 => {
                assign_parent(
                    &mut parent_by_child,
                    &node_types,
                    &target_ieee,
                    &source_ieee,
                );
            }
            // Source is child of target.
            1 => {
                assign_parent(
                    &mut parent_by_child,
                    &node_types,
                    &source_ieee,
                    &target_ieee,
                );
            }
            _ => {}
        }
    }

    for node in response.data.value.nodes {
        let device = match find_device_by_ieee_or_topic(db, &node.ieee_addr) {
            Ok(Some(d)) => d,
            Ok(None) => {
                debug!(ieee_addr = %node.ieee_addr, "Device not found in database, skipping");
                continue;
            }
            Err(e) => {
                error!(error = %e, ieee_addr = %node.ieee_addr, "Failed to query device");
                continue;
            }
        };

        let is_bridge = node.node_type == "Router";
        let parent_device_id = parent_by_child
            .get(&node.ieee_addr)
            .and_then(|parent_ieee| {
                find_device_by_ieee_or_topic(db, parent_ieee)
                    .ok()
                    .flatten()
                    .map(|parent_device| parent_device.id)
            });

        if let Err(e) = db.update_device_topology(&device.id, is_bridge, parent_device_id) {
            error!(
                error = %e,
                device_id = %device.id,
                "Failed to update device topology"
            );
        } else {
            debug!(
                device_id = %device.id,
                is_bridge = %is_bridge,
                "Updated device network topology"
            );
        }
    }

    if let Err(e) = event_tx.send(SystemEvent::NetworkTopologyUpdated) {
        warn!(error = %e, "Failed to send NetworkTopologyUpdated event");
    } else {
        info!("Network topology updated successfully");
    }
}

fn find_device_by_ieee_or_topic(
    db: &Database,
    ieee_addr: &str,
) -> Result<Option<crate::models::Device>, rusqlite::Error> {
    match db.get_device_by_ieee_addr(ieee_addr) {
        Ok(Some(device)) => Ok(Some(device)),
        Ok(None) => db.get_device_by_mqtt_topic(ieee_addr),
        Err(e) => Err(e),
    }
}

fn assign_parent(
    parent_by_child: &mut std::collections::HashMap<String, String>,
    node_types: &std::collections::HashMap<String, String>,
    child_ieee: &str,
    parent_ieee: &str,
) {
    if parent_by_child.contains_key(child_ieee) {
        return;
    }

    if node_types
        .get(child_ieee)
        .is_some_and(|node_type| node_type == "Coordinator")
    {
        return;
    }

    if node_types
        .get(parent_ieee)
        .is_some_and(|node_type| node_type == "EndDevice")
    {
        return;
    }

    parent_by_child.insert(child_ieee.to_string(), parent_ieee.to_string());
}

fn ensure_coordinator_device(db: &Database, node: &NetworkNode) {
    let coordinator_name = node
        .friendly_name
        .as_ref()
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "Coordinator".to_string());

    match find_device_by_ieee_or_topic(db, &node.ieee_addr) {
        Ok(Some(device)) => {
            debug!(
                device_id = %device.id,
                ieee_addr = %node.ieee_addr,
                "Coordinator already exists in database"
            );
        }
        Ok(None) => {
            let device = Device::new(
                node.ieee_addr.clone(),
                coordinator_name,
                DeviceCapability::Coordinator,
                PowerSource::Plugged,
            );

            if let Err(e) = db.insert_device(&device) {
                error!(
                    error = %e,
                    ieee_addr = %node.ieee_addr,
                    "Failed to insert coordinator device"
                );
            } else {
                info!(
                    device_id = %device.id,
                    ieee_addr = %node.ieee_addr,
                    "Coordinator device created from network map"
                );
            }
        }
        Err(e) => {
            error!(
                error = %e,
                ieee_addr = %node.ieee_addr,
                "Failed to query coordinator device"
            );
        }
    }
}

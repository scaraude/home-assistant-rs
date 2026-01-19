use crate::models::DeviceState;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory state storage for all devices
/// Tracks ephemeral data like link quality, battery level, turbo mode, and last seen timestamp
#[derive(Clone)]
pub struct DeviceStateStore {
    states: Arc<RwLock<HashMap<String, DeviceState>>>,
}

impl DeviceStateStore {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update or create device state
    pub fn update_state<F>(&self, device_id: String, update_fn: F)
    where
        F: FnOnce(&mut DeviceState),
    {
        if let Ok(mut states) = self.states.write() {
            let state = states
                .entry(device_id.clone())
                .or_insert_with(|| DeviceState::new(device_id));
            update_fn(state);
        }
    }

    /// Get the state of a device (returns None if unknown)
    pub fn get_state(&self, device_id: &str) -> Option<DeviceState> {
        self.states.read().ok()?.get(device_id).cloned()
    }

    /// Get battery level for a device
    pub fn get_battery(&self, device_id: &str) -> Option<u8> {
        self.get_state(device_id)
            .and_then(|state| state.battery_level)
    }

    /// Get link quality for a device
    pub fn get_link_quality(&self, device_id: &str) -> Option<u8> {
        self.get_state(device_id)
            .and_then(|state| state.link_quality)
    }
}

impl Default for DeviceStateStore {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::models::SwitchState;

/// In-memory state storage for switch devices
#[derive(Clone)]
pub struct SwitchStateStore {
    states: Arc<RwLock<HashMap<String, SwitchState>>>,
}

impl SwitchStateStore {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update the state of a switch
    pub fn set_state(&self, device_id: String, state: SwitchState) {
        if let Ok(mut states) = self.states.write() {
            states.insert(device_id, state);
        }
    }

    /// Get the state of a switch (returns None if unknown)
    pub fn get_state(&self, device_id: &str) -> Option<SwitchState> {
        self.states.read().ok()?.get(device_id).copied()
    }
}

impl Default for SwitchStateStore {
    fn default() -> Self {
        Self::new()
    }
}

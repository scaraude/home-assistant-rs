use crate::db::Database;
use crate::models::SwitchMqttMessage;
use crate::mqtt::device_discovery::{MqttMessage, get_or_create_device};
use crate::state::{DeviceStateStore, SwitchStateStore};
use tracing::{debug, info};

pub async fn handle_switch_message(
    db: &Database,
    device_state: &DeviceStateStore,
    switch_state: &SwitchStateStore,
    mqtt_topic: &str,
    switch_msg: SwitchMqttMessage,
) {
    if let Some(device_id) =
        get_or_create_device(db, mqtt_topic, MqttMessage::Switch(switch_msg.clone())).await
    {
        if let Some(state_str) = &switch_msg.state {
            let state = state_str.to_uppercase() == "ON";
            info!(
                device_id = %device_id,
                state = %state,
                linkquality = ?switch_msg.linkquality,
                "Received switch state update"
            );

            // Update device state with link quality
            if let Some(lq) = switch_msg.linkquality {
                device_state.update_link_quality(device_id.clone(), lq);
            }
            device_state.mark_seen(device_id.clone());

            // Update switch state store
            switch_state.set_state(device_id.to_string(), state);
            debug!(
                device_id = %device_id,
                state = %state,
                "Updated switch state in store"
            );
        }
    }
}

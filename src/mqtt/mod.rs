// Core MQTT modules
mod client;
mod device_discovery;
mod event_loop;
mod handlers;

// Tests
#[cfg(test)]
mod tests;

// Re-export the main MQTT client and factory function
pub use client::MqttClient;

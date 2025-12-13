// State management modules
mod device;
mod switch;

// Re-export state stores
pub use device::DeviceStateStore;
pub use switch::SwitchStateStore;

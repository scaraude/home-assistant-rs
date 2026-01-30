mod automation;
mod logs;
mod positions;
mod sensors;
mod switches;
mod system;

use crate::http::responses::success_response;
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;

pub use automation::{
    create_automation_rule, delete_automation_rule, serve_automation_rule, serve_automation_rules,
    serve_execution_logs, update_automation_rule,
};
pub use logs::{serve_log_since, serve_log_view, serve_logs_list, serve_process_history};
pub use positions::{serve_device_positions, update_device_position};
pub use sensors::{serve_readings, serve_sensors};
pub use switches::{
    execute_command, permit_join, serve_device_state, serve_device_states, serve_switches_list,
    set_device_option, update_device,
};
pub use system::{refresh_network_map, serve_network_topology, serve_storage_breakdown};

pub fn health_check() -> Response<Full<Bytes>> {
    success_response()
}

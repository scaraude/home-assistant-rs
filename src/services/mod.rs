//! Service modules that react to events and coordinate state.

mod automation;
mod db_writer;
mod state_manager;
mod websocket;

pub use automation::AutomationService;
pub use db_writer::DbWriterService;
pub use state_manager::StateManagerService;
pub use websocket::WebSocketBroadcaster;

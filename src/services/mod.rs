//! Service modules that react to events and coordinate state.

mod automation;
mod db_writer;
mod log_watcher;
mod state_manager;
mod websocket;

pub use automation::AutomationService;
pub use db_writer::DbWriterService;
pub use log_watcher::LogWatcherService;
pub use state_manager::StateManagerService;
pub use websocket::WebSocketBroadcaster;

//! MCP Server for Home Automation
//! Pattern inspired by avrabe/mcp-loxone (MIT)
//!
//! Tools:
//!   - light_control(room, action)
//!   - climate_control(room, temp)
//!   - announce(text)
//!   - diagnose(section)
//!   - brain_status()

pub mod tools;
pub mod server;

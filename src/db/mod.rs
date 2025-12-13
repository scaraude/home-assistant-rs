// Core database modules
mod connection;
mod schema;

// Query modules organized by domain
mod queries;

// Tests
#[cfg(test)]
mod tests;

// Re-export the main Database struct and utilities
pub use connection::{Database, MutexExt, Transaction};

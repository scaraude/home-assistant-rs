use chrono::{DateTime, Utc};
use tracing::error;

/// Convert a Unix timestamp to a DateTime, with logging for invalid timestamps.
///
/// This helper makes data corruption visible by logging when timestamps are invalid,
/// instead of silently defaulting to epoch. This helps detect database corruption
/// or bugs in timestamp handling.
///
/// # Arguments
/// * `ts` - Unix timestamp (seconds since epoch)
/// * `context` - Description of where this timestamp came from (e.g., "temperature_reading.timestamp")
///
/// # Returns
/// Valid DateTime, or Unix epoch (1970-01-01) with error logged if timestamp is invalid
pub fn timestamp_to_datetime(ts: i64, context: &str) -> DateTime<Utc> {
    chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(|| {
        error!(
            timestamp = ts,
            context = context,
            "Invalid timestamp in database - using Unix epoch as fallback"
        );
        DateTime::UNIX_EPOCH
    })
}

/// Create an InvalidColumnType error for enum deserialization failures.
///
/// This helper eliminates duplicate error construction code and ensures consistent
/// error reporting when database string values can't be converted to enums.
///
/// # Arguments
/// * `column_index` - Zero-based column index in the query result
/// * `column_name` - Name of the column (for error messages)
///
/// # Returns
/// rusqlite::Error::InvalidColumnType with consistent formatting
pub fn invalid_column_error(column_index: usize, column_name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(
        column_index,
        column_name.to_string(),
        rusqlite::types::Type::Text,
    )
}

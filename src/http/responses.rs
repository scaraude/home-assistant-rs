// ============================================================================
// HTTP Response Helper Functions
// ============================================================================

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;
use tracing::error;

/// Create a successful JSON response (200 OK)
pub fn json_response(json: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response - this should never happen with valid headers")
}

/// Serialize payload to JSON or return an internal error response
pub fn serialize_to_json<T: Serialize>(
    data: &T,
    error_context: &str,
) -> Result<String, Box<Response<Full<Bytes>>>> {
    match serde_json::to_string(data) {
        Ok(json) => Ok(json),
        Err(e) => {
            error!(
                error = %e,
                context = error_context,
                "Failed to serialize response"
            );
            Err(Box::new(internal_error_response(
                "Failed to serialize response",
            )))
        }
    }
}

/// Serialize payload to JSON and return a standard JSON response
pub fn serialize_or_error<T: Serialize>(
    data: &T,
    error_context: &str,
) -> Response<Full<Bytes>> {
    match serialize_to_json(data, error_context) {
        Ok(json) => json_response(json),
        Err(response) => *response,
    }
}

/// Create a JSON response with custom status code
pub fn json_response_with_status(json: String, status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response - this should never happen with valid headers")
}

/// Create a JSON response with a custom latest timestamp header
pub fn json_response_with_timestamp(json: String, latest_timestamp: i64) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Latest-Timestamp", latest_timestamp.to_string())
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response with timestamp header")
}

/// Create a JSON response with total lines header (for log pagination)
pub fn json_response_with_total_lines(json: String, total_lines: usize) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Total-Lines", total_lines.to_string())
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response with total lines header")
}

/// Create an error response with JSON error message
pub fn error_response(message: &str, status: StatusCode) -> Response<Full<Bytes>> {
    let json = format!(r#"{{"error":"{}"}}"#, message);
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build error response")
}

/// Create a simple success response
pub fn success_response() -> Response<Full<Bytes>> {
    json_response(r#"{"status":"ok"}"#.to_string())
}

/// Create a 404 Not Found response
pub fn not_found_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::NOT_FOUND)
}

/// Create a 500 Internal Server Error response
pub fn internal_error_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::INTERNAL_SERVER_ERROR)
}

/// Create a 400 Bad Request response
pub fn bad_request_response(message: &str) -> Response<Full<Bytes>> {
    error_response(message, StatusCode::BAD_REQUEST)
}

/// Create a static file response with caching headers
pub fn static_file_response(contents: Vec<u8>, content_type: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(contents)))
        .expect("Failed to build static file response")
}

/// Create a 204 No Content response
pub fn no_content_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .expect("Failed to build 204 No Content response")
}

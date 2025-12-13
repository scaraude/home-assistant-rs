// ============================================================================
// HTTP Response Helper Functions
// ============================================================================

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Create a successful JSON response (200 OK)
pub fn json_response(json: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response - this should never happen with valid headers")
}

/// Create a JSON response with custom status code
pub fn json_response_with_status(json: String, status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build JSON response - this should never happen with valid headers")
}

/// Create a JSON response with ETag header
pub fn json_response_with_etag(json: String, etag: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build cached JSON response - this should never happen")
}

/// Create a JSON response with ETag and custom timestamp header
pub fn json_response_with_cache_headers(
    json: String,
    etag: String,
    latest_timestamp: i64,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .header("X-Latest-Timestamp", latest_timestamp.to_string())
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build cached JSON response with timestamp header")
}

/// Create a JSON response with ETag and total lines header (for log pagination)
pub fn json_response_with_lines_header(
    json: String,
    etag: String,
    total_lines: usize,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("ETag", etag)
        .header("Cache-Control", "private, must-revalidate")
        .header("X-Total-Lines", total_lines.to_string())
        .body(Full::new(Bytes::from(json)))
        .expect("Failed to build cached JSON response with total lines header")
}

/// Create a 304 Not Modified response with ETag
pub fn not_modified_response(etag: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header("ETag", etag)
        .body(Full::new(Bytes::new()))
        .expect("Failed to build 304 Not Modified response")
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

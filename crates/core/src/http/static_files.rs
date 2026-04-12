use crate::http::responses::{not_found_response, static_file_response};
use http_body_util::Full;
use hyper::Response;
use hyper::body::Bytes;
use tracing::{debug, error};

pub fn serve_static_file(file_path: &str, content_type: &str) -> Response<Full<Bytes>> {
    match std::fs::read(file_path) {
        Ok(contents) => {
            debug!(file_path = %file_path, size = contents.len(), "Serving static file");
            static_file_response(contents, content_type)
        }
        Err(e) => {
            error!(error = %e, file_path = %file_path, "Failed to read static file");
            not_found_response("File not found")
        }
    }
}

pub fn serve_static_asset(path: &str) -> Response<Full<Bytes>> {
    // Remove leading slash and construct file path
    let file_path = format!("static{}", path);

    // Determine content type based on extension
    let content_type = if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    };

    serve_static_file(&file_path, content_type)
}

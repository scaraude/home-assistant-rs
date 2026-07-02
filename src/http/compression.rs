//! Transparent gzip compression for HTTP responses.
//!
//! The server builds every response as an in-memory `Full<Bytes>` body, so we
//! can cheaply gzip it after the fact when the client advertises support. This
//! is the single biggest win for large payloads such as the floor-plan SVG
//! (~2.5 MB of text that compresses by ~85-90%).

use flate2::Compression;
use flate2::write::GzEncoder;
use http_body_util::{BodyExt, Full};
use hyper::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, HeaderValue, VARY};
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use std::io::Write;
use tracing::{debug, warn};

/// Do not bother compressing bodies smaller than this (gzip overhead + CPU on a
/// Raspberry Pi Zero is not worth it for tiny JSON payloads).
const MIN_COMPRESS_BYTES: usize = 1024;

/// Returns true if the `Accept-Encoding` header value advertises gzip support.
pub fn accepts_gzip(accept_encoding: Option<&str>) -> bool {
    accept_encoding
        .map(|value| value.split(',').any(|part| part.trim().starts_with("gzip")))
        .unwrap_or(false)
}

/// Extract the `Accept-Encoding` header from a request as an owned string.
pub fn accept_encoding_header<B>(req: &hyper::Request<B>) -> Option<String> {
    req.headers()
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// Compress a response body with gzip when the client supports it and the
/// payload is large enough to benefit. Already-encoded responses are left
/// untouched. Falls back to the original bytes on any encoding error.
pub async fn maybe_compress(
    accept_encoding: Option<&str>,
    response: Response<Full<Bytes>>,
) -> Response<Full<Bytes>> {
    if !accepts_gzip(accept_encoding) {
        return response;
    }

    // Never touch protocol-upgrade (e.g. WebSocket 101) responses.
    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        return response;
    }

    let (mut parts, body) = response.into_parts();

    // Never double-encode.
    if parts.headers.contains_key(CONTENT_ENCODING) {
        return Response::from_parts(parts, body);
    }

    // `Full<Bytes>` is already fully buffered, so this never actually awaits I/O.
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return Response::from_parts(parts, Full::new(Bytes::new())),
    };

    if bytes.len() < MIN_COMPRESS_BYTES {
        return Response::from_parts(parts, Full::new(bytes));
    }

    let mut encoder = GzEncoder::new(Vec::with_capacity(bytes.len() / 2), Compression::fast());
    if let Err(e) = encoder.write_all(&bytes) {
        warn!(error = %e, "gzip write failed; sending uncompressed");
        return Response::from_parts(parts, Full::new(bytes));
    }
    let compressed = match encoder.finish() {
        Ok(data) => data,
        Err(e) => {
            warn!(error = %e, "gzip finish failed; sending uncompressed");
            return Response::from_parts(parts, Full::new(bytes));
        }
    };

    debug!(
        original = bytes.len(),
        compressed = compressed.len(),
        "Compressed response with gzip"
    );

    parts
        .headers
        .insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    parts
        .headers
        .insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    // Content-Length no longer matches; hyper recomputes it from the new body.
    parts.headers.remove(CONTENT_LENGTH);

    Response::from_parts(parts, Full::new(Bytes::from(compressed)))
}

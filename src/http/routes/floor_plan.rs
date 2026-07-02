use crate::db::Database;
use crate::http::responses::*;
use crate::models::FloorPlan;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Maximum allowed SVG size: 5MB
const MAX_SVG_SIZE: usize = 5 * 1024 * 1024;

/// GET /api/floor-plan - Get the floor plan SVG
///
/// Returns a weak `ETag` derived from the upload time and SVG length so the
/// client (which also caches the SVG in IndexedDB) can revalidate with
/// `If-None-Match` and get a `304` instead of re-downloading ~2.5 MB.
pub fn serve_floor_plan(db: &Arc<Database>, if_none_match: Option<&str>) -> Response<Full<Bytes>> {
    debug!("Getting floor plan from database");

    match db.get_floor_plan() {
        Ok(Some(floor_plan)) => {
            debug!(
                svg_size = floor_plan.svg_content.len(),
                uploaded_at = %floor_plan.uploaded_at,
                "Retrieved floor plan from database"
            );

            let etag = format!(
                "W/\"{}-{}\"",
                floor_plan.uploaded_at.timestamp(),
                floor_plan.svg_content.len()
            );

            // If the client already holds this exact version, skip the payload.
            if if_none_match.is_some_and(|value| value.split(',').any(|tag| tag.trim() == etag)) {
                debug!(%etag, "Floor plan unchanged; returning 304");
                return not_modified_response(&etag);
            }

            let json = match serialize_to_json(&floor_plan, "floor plan") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            info!(
                response_size = json.len(),
                "Successfully serialized floor plan to JSON"
            );

            json_response_with_etag(json, &etag)
        }
        Ok(None) => {
            debug!("No floor plan found in database");
            not_found_response("No floor plan uploaded")
        }
        Err(e) => {
            error!(error = %e, "Database error while fetching floor plan");
            internal_error_response("Database error")
        }
    }
}

#[derive(serde::Deserialize)]
pub struct FloorPlanUpload {
    /// SVG content as a string (base64 decoded or raw SVG)
    pub svg_content: String,
}

/// POST /api/floor-plan - Upload a new floor plan SVG
pub async fn upload_floor_plan(
    req: Request<hyper::body::Incoming>,
    db: &Arc<Database>,
) -> Response<Full<Bytes>> {
    debug!("Processing floor plan upload request");

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read request body");
        }
    };

    // Check size limit
    if body_bytes.len() > MAX_SVG_SIZE {
        warn!(
            size = body_bytes.len(),
            max_size = MAX_SVG_SIZE,
            "Floor plan upload exceeds size limit"
        );
        return error_response(
            &format!(
                "SVG content too large. Maximum size is {} bytes",
                MAX_SVG_SIZE
            ),
            StatusCode::PAYLOAD_TOO_LARGE,
        );
    }

    // Parse the upload payload
    let upload: FloorPlanUpload = match serde_json::from_slice::<FloorPlanUpload>(&body_bytes) {
        Ok(upd) => {
            debug!(svg_size = upd.svg_content.len(), "Parsed floor plan upload");
            upd
        }
        Err(e) => {
            error!(error = %e, "Failed to parse floor plan upload JSON");
            return bad_request_response(&format!("Invalid JSON: {}", e));
        }
    };

    // Validate SVG content (basic check)
    if !is_valid_svg(&upload.svg_content) {
        warn!("Invalid SVG content in floor plan upload");
        return bad_request_response("Invalid SVG content. Must be a valid SVG document.");
    }

    // Create floor plan model
    let floor_plan = FloorPlan::new(upload.svg_content);

    // Upsert to database
    if let Err(e) = db.upsert_floor_plan(&floor_plan) {
        error!(error = %e, "Failed to save floor plan to database");
        return internal_error_response("Failed to save floor plan");
    }

    info!(
        svg_size = floor_plan.svg_content.len(),
        "Floor plan uploaded successfully"
    );

    json_response_with_status(r#"{"status":"ok"}"#.into(), StatusCode::CREATED)
}

/// DELETE /api/floor-plan - Delete the floor plan
pub fn delete_floor_plan(db: &Arc<Database>) -> Response<Full<Bytes>> {
    debug!("Processing floor plan delete request");

    match db.delete_floor_plan() {
        Ok(true) => {
            info!("Floor plan deleted successfully");
            no_content_response()
        }
        Ok(false) => {
            debug!("No floor plan to delete");
            not_found_response("No floor plan exists to delete")
        }
        Err(e) => {
            error!(error = %e, "Failed to delete floor plan from database");
            internal_error_response("Failed to delete floor plan")
        }
    }
}

/// Basic SVG validation
/// Checks if the content appears to be valid SVG
fn is_valid_svg(content: &str) -> bool {
    let trimmed = content.trim();

    // Check minimum length first
    if trimmed.len() < 6 {
        return false;
    }

    // Must contain svg tag
    if !trimmed.contains("<svg") {
        return false;
    }

    // Should have opening and closing svg tags (or self-closing)
    let has_closing = trimmed.contains("</svg>") || trimmed.contains("/>");

    has_closing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_svg_basic() {
        assert!(is_valid_svg("<svg></svg>"));
        assert!(is_valid_svg("<svg width=\"100\" height=\"100\"></svg>"));
        assert!(is_valid_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#
        ));
    }

    #[test]
    fn test_is_valid_svg_with_whitespace() {
        assert!(is_valid_svg("  <svg></svg>  "));
        assert!(is_valid_svg("\n<svg>\n</svg>\n"));
    }

    #[test]
    fn test_is_valid_svg_self_closing() {
        assert!(is_valid_svg("<svg/>"));
        assert!(is_valid_svg("<svg width=\"100\"/>"));
    }

    #[test]
    fn test_is_valid_svg_invalid() {
        assert!(!is_valid_svg(""));
        assert!(!is_valid_svg("not svg"));
        assert!(!is_valid_svg("<div></div>"));
        assert!(!is_valid_svg("<svg")); // no closing
    }
}

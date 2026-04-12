use crate::db::Database;
use crate::events::{SystemEvent, bus::EventBus};
use crate::http::query::ExecutionLogsQuery;
use crate::http::responses::*;
use crate::models::{
    AutomationAction, AutomationCondition, AutomationRule, CreateAutomationRuleRequest,
    DeviceCapability, SensorType, TimeWindowRequest, UpdateAutomationRuleRequest,
};
use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode};
use tracing::{error, info, warn};

pub fn serve_automation_rules(db: &Database) -> Response<Full<Bytes>> {
    match db.get_all_automation_rules() {
        Ok(rules) => {
            info!(rule_count = rules.len(), "Served automation rules list");
            serialize_or_error(&rules, "automation rules list")
        }
        Err(e) => {
            error!(error = %e, "Failed to get automation rules");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_automation_rule(db: &Database, rule_id: &str) -> Response<Full<Bytes>> {
    match db.get_automation_rule(rule_id) {
        Ok(Some(rule)) => {
            info!(rule_id = %rule_id, "Served automation rule");
            serialize_or_error(&rule, "automation rule")
        }
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found");
            not_found_response("Rule not found")
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to get automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

/// Validate that automation conditions use fields compatible with their device types
fn validate_condition_fields(
    db: &Database,
    conditions: &[AutomationCondition],
) -> Result<(), String> {
    for condition in conditions {
        // Get device from database
        let device = db
            .get_device(&condition.device_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Device '{}' not found", condition.device_id))?;

        // Convert SensorField to string for comparison
        let field_str = match condition.field {
            crate::models::SensorField::Temperature => "temperature",
            crate::models::SensorField::Humidity => "humidity",
            crate::models::SensorField::Battery => "battery",
            crate::models::SensorField::LinkQuality => "link_quality",
            crate::models::SensorField::Presence => "presence",
            crate::models::SensorField::Illumination => "illumination",
        };

        let mut valid = false;
        let mut valid_fields = Vec::new();

        for capability in &device.capabilities {
            if let DeviceCapability::Sensor { sensor_type } = capability {
                match sensor_type {
                    SensorType::TempHumidity => {
                        valid_fields.extend(["temperature", "humidity", "battery", "link_quality"]);
                        if matches!(
                            field_str,
                            "temperature" | "humidity" | "battery" | "link_quality"
                        ) {
                            valid = true;
                        }
                    }
                    SensorType::Presence => {
                        valid_fields.extend([
                            "presence",
                            "illumination",
                            "battery",
                            "link_quality",
                        ]);
                        if matches!(
                            field_str,
                            "presence" | "illumination" | "battery" | "link_quality"
                        ) {
                            valid = true;
                        }
                    }
                    SensorType::EnergyMeter => {
                        valid_fields.extend(["battery", "link_quality"]);
                        if matches!(field_str, "battery" | "link_quality") {
                            valid = true;
                        }
                    }
                }
            }
        }

        if valid_fields.is_empty() {
            return Err(format!("Device '{}' is not a sensor", condition.device_id));
        }
        valid_fields.sort_unstable();
        valid_fields.dedup();

        if !valid {
            return Err(format!(
                "Field '{}' is not valid for device '{}'. Valid fields: {}",
                field_str,
                condition.device_id,
                valid_fields.join(", ")
            ));
        }
    }

    Ok(())
}

pub async fn create_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
) -> Response<Full<Bytes>> {
    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read body");
        }
    };

    // Parse the request
    let request: CreateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule request");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Validate request
    if request.name.trim().is_empty() {
        return bad_request_response("Rule name cannot be empty");
    }

    if request.conditions.is_empty() {
        return bad_request_response("At least one condition is required");
    }

    if request.actions.is_empty() {
        return bad_request_response("At least one action is required");
    }

    // Build the automation rule
    let conditions: Vec<AutomationCondition> = request
        .conditions
        .into_iter()
        .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
        .collect();

    // Validate that condition fields are compatible with device types
    if let Err(err) = validate_condition_fields(db, &conditions) {
        warn!(error = %err, "Automation rule validation failed");
        return bad_request_response(&err);
    }

    let actions: Vec<AutomationAction> = request
        .actions
        .into_iter()
        .map(|a| AutomationAction::new(a.device_id, a.action))
        .collect();

    let mut rule = AutomationRule::new(
        request.name,
        request.description,
        request.condition_operator,
        conditions,
        actions,
    );

    if let Some(enabled) = request.enabled {
        rule.enabled = enabled;
    }

    if let Some(time_window) = request.time_window {
        apply_time_window_update(&mut rule, time_window);
    }

    // Insert into database
    match db.insert_automation_rule(&rule) {
        Ok(_) => {
            info!(rule_id = %rule.id, rule_name = %rule.name, "Created automation rule");

            let json = match serialize_to_json(&rule, "created automation rule") {
                Ok(json) => json,
                Err(response) => return *response,
            };
            json_response_with_status(json, StatusCode::CREATED)
        }
        Err(e) => {
            error!(error = %e, "Failed to insert automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub async fn update_automation_rule(
    req: Request<hyper::body::Incoming>,
    db: &Database,
    rule_id: &str,
) -> Response<Full<Bytes>> {
    // Get existing rule
    let mut rule = match db.get_automation_rule(rule_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            warn!(rule_id = %rule_id, "Automation rule not found for update");
            return not_found_response("Rule not found");
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to fetch rule for update");
            return internal_error_response(e.to_string().as_str());
        }
    };

    // Read the request body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!(error = %e, "Failed to read request body");
            return bad_request_response("Failed to read body");
        }
    };

    // Parse the update request
    let update: UpdateAutomationRuleRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!(error = %e, "Failed to parse automation rule update");
            return bad_request_response(format!("Invalid JSON: {}", e).as_str());
        }
    };

    // Apply updates
    if let Some(name) = update.name {
        if name.trim().is_empty() {
            return bad_request_response("Rule name cannot be empty");
        }
        rule.name = name;
    }

    if let Some(description) = update.description {
        rule.description = Some(description);
    }

    if let Some(enabled) = update.enabled {
        rule.enabled = enabled;
    }

    if let Some(condition_operator) = update.condition_operator {
        rule.condition_operator = condition_operator;
    }

    if let Some(conditions) = update.conditions {
        if conditions.is_empty() {
            return bad_request_response("At least one condition is required");
        }
        let new_conditions: Vec<AutomationCondition> = conditions
            .into_iter()
            .map(|c| AutomationCondition::new(c.device_id, c.field, c.operator, c.value))
            .collect();

        // Validate that condition fields are compatible with device types
        if let Err(err) = validate_condition_fields(db, &new_conditions) {
            warn!(error = %err, "Automation rule update validation failed");
            return bad_request_response(&err);
        }

        rule.conditions = new_conditions;
    }

    if let Some(actions) = update.actions {
        if actions.is_empty() {
            return bad_request_response("At least one action is required");
        }
        rule.actions = actions
            .into_iter()
            .map(|a| AutomationAction::new(a.device_id, a.action))
            .collect();
    }

    if let Some(time_window) = update.time_window {
        apply_time_window_update(&mut rule, time_window);
    }

    // Update timestamp
    rule.updated_at = chrono::Utc::now();

    // Save to database
    match db.update_automation_rule(&rule) {
        Ok(_) => {
            info!(rule_id = %rule.id, rule_name = %rule.name, "Updated automation rule");

            let json = match serialize_to_json(&rule, "updated automation rule") {
                Ok(json) => json,
                Err(response) => return *response,
            };

            json_response(json)
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to update automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

fn apply_time_window_update(rule: &mut AutomationRule, update: TimeWindowRequest) {
    if let Some(enabled) = update.enabled {
        rule.time_window.enabled = enabled;
    }

    if let Some(start_time) = update.start_time {
        rule.time_window.start_time = Some(start_time);
    }

    if let Some(end_time) = update.end_time {
        rule.time_window.end_time = Some(end_time);
    }

    if let Some(active_days) = update.active_days {
        rule.time_window.active_days = Some(active_days);
    }
}

pub fn delete_automation_rule(
    db: &Database,
    rule_id: &str,
    event_bus: &EventBus,
) -> Response<Full<Bytes>> {
    match db.delete_automation_rule(rule_id) {
        Ok(_) => {
            info!(rule_id = %rule_id, "Deleted automation rule");

            // Publish event to notify AutomationService to clean up debounce cache
            let event = SystemEvent::AutomationRuleDeleted {
                rule_id: rule_id.to_string(),
                timestamp: Utc::now(),
            };
            if let Err(e) = event_bus.publish(event) {
                warn!(
                    error = %e,
                    rule_id = %rule_id,
                    "Failed to publish AutomationRuleDeleted event"
                );
            }

            no_content_response()
        }
        Err(e) => {
            error!(error = %e, rule_id = %rule_id, "Failed to delete automation rule");
            internal_error_response(e.to_string().as_str())
        }
    }
}

pub fn serve_execution_logs(db: &Database, query: Option<&str>) -> Response<Full<Bytes>> {
    let limit = ExecutionLogsQuery::new(query).limit();

    match db.get_execution_logs(limit) {
        Ok(logs) => {
            info!(log_count = logs.len(), limit = %limit, "Served automation execution logs");
            serialize_or_error(&logs, "automation execution logs")
        }
        Err(e) => {
            error!(error = %e, "Failed to get execution logs");
            internal_error_response(e.to_string().as_str())
        }
    }
}

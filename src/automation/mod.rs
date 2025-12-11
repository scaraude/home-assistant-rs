use crate::db::Database;
use crate::device_state::DeviceStateStore;
use crate::models::{
    AutomationAction, AutomationExecutionLog, AutomationRule, SensorField, SwitchAction,
    TemperatureReading,
};
use crate::switch_state::SwitchStateStore;
use rumqttc::AsyncClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Automation engine that evaluates rules and executes actions
pub struct AutomationEngine {
    db: Arc<Database>,
    mqtt_client: AsyncClient,
    device_state: DeviceStateStore,
    switch_state: SwitchStateStore,
    /// Track which rules were last triggered to implement debouncing
    last_triggered: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
    /// Minimum time between rule triggers (in seconds)
    debounce_seconds: i64,
}

impl AutomationEngine {
    pub fn new(
        db: Arc<Database>,
        mqtt_client: AsyncClient,
        device_state: DeviceStateStore,
        switch_state: SwitchStateStore,
    ) -> Self {
        Self {
            db,
            mqtt_client,
            device_state,
            switch_state,
            last_triggered: Arc::new(RwLock::new(HashMap::new())),
            debounce_seconds: 60, // Don't trigger same rule more than once per minute
        }
    }

    /// Evaluate all automation rules against a new sensor reading
    pub async fn evaluate_reading(&self, reading: &TemperatureReading) {
        debug!(
            device_id = %reading.device_id,
            "Evaluating automation rules for new reading"
        );

        // Get all enabled rules
        let rules = match self.db.get_all_automation_rules() {
            Ok(rules) => rules.into_iter().filter(|r| r.enabled).collect::<Vec<_>>(),
            Err(e) => {
                error!(error = %e, "Failed to fetch automation rules");
                return;
            }
        };

        if rules.is_empty() {
            debug!("No enabled automation rules to evaluate");
            return;
        }

        info!(rule_count = rules.len(), "Evaluating automation rules");

        for rule in rules {
            self.evaluate_rule(&rule, reading).await;
        }
    }

    /// Evaluate a single rule against a reading
    async fn evaluate_rule(&self, rule: &AutomationRule, reading: &TemperatureReading) {
        debug!(
            rule_id = %rule.id,
            rule_name = %rule.name,
            device_id = %reading.device_id,
            "Evaluating rule"
        );

        // Check if any condition in this rule references the device from the reading
        let affects_this_device = rule
            .conditions
            .iter()
            .any(|c| c.device_id == reading.device_id);

        if !affects_this_device {
            debug!(
                rule_id = %rule.id,
                device_id = %reading.device_id,
                "Rule does not reference this device, skipping"
            );
            return;
        }

        // Check debouncing - don't trigger the same rule too frequently
        {
            let last_triggered = self.last_triggered.read().await;
            if let Some(last_time) = last_triggered.get(&rule.id) {
                let elapsed = chrono::Utc::now().signed_duration_since(*last_time);
                if elapsed.num_seconds() < self.debounce_seconds {
                    debug!(
                        rule_id = %rule.id,
                        seconds_since_last = elapsed.num_seconds(),
                        "Rule recently triggered, debouncing"
                    );
                    return;
                }
            }
        }

        // Evaluate all conditions
        let conditions_met = self.evaluate_conditions(rule).await;

        if conditions_met {
            info!(
                rule_id = %rule.id,
                rule_name = %rule.name,
                "Conditions met, executing actions"
            );

            // Execute actions
            self.execute_actions(rule).await;

            // Update debounce tracking
            let mut last_triggered = self.last_triggered.write().await;
            last_triggered.insert(rule.id.clone(), chrono::Utc::now());
        } else {
            debug!(
                rule_id = %rule.id,
                rule_name = %rule.name,
                "Conditions not met"
            );
        }
    }

    /// Evaluate all conditions for a rule
    async fn evaluate_conditions(&self, rule: &AutomationRule) -> bool {
        if rule.conditions.is_empty() {
            warn!(rule_id = %rule.id, "Rule has no conditions, treating as false");
            return false;
        }

        let mut results = Vec::new();

        for condition in &rule.conditions {
            let result = self.evaluate_condition(condition).await;
            results.push(result);

            debug!(
                rule_id = %rule.id,
                device_id = %condition.device_id,
                field = ?condition.field,
                operator = ?condition.operator,
                value = %condition.value,
                result = %result,
                "Evaluated condition"
            );
        }

        // Combine results based on logical operator
        match rule.condition_operator {
            crate::models::LogicalOperator::And => results.iter().all(|&r| r),
            crate::models::LogicalOperator::Or => results.iter().any(|&r| r),
        }
    }

    /// Evaluate a single condition
    async fn evaluate_condition(&self, condition: &crate::models::AutomationCondition) -> bool {
        // Get the latest reading for the device
        let readings = match self
            .db
            .get_readings_for_sensor_since(&condition.device_id, 0)
        {
            Ok(readings) => readings,
            Err(e) => {
                error!(
                    error = %e,
                    device_id = %condition.device_id,
                    "Failed to fetch readings for condition evaluation"
                );
                return false;
            }
        };

        let latest_reading = match readings.first() {
            Some(r) => r,
            None => {
                debug!(
                    device_id = %condition.device_id,
                    "No readings found for device"
                );
                return false;
            }
        };

        // Extract the field value
        let field_value: Option<f64> = match condition.field {
            SensorField::Temperature => Some(latest_reading.temperature.into()),
            SensorField::Humidity => latest_reading.humidity.map(|h| h as f64),
            SensorField::Battery => latest_reading.battery.map(|b| b as f64),
            SensorField::LinkQuality => latest_reading.link_quality.map(|lq| lq as f64),
        };

        let field_value = match field_value {
            Some(v) => v,
            None => {
                debug!(
                    device_id = %condition.device_id,
                    field = ?condition.field,
                    "Field value not available"
                );
                return false;
            }
        };

        // Evaluate comparison
        condition.operator.evaluate(field_value, condition.value)
    }

    /// Execute all actions for a rule
    async fn execute_actions(&self, rule: &AutomationRule) {
        let mut success = true;
        let mut error_messages = Vec::new();

        for action in &rule.actions {
            if let Err(e) = self.execute_action(action).await {
                error!(
                    error = %e,
                    rule_id = %rule.id,
                    device_id = %action.device_id,
                    "Failed to execute action"
                );
                success = false;
                error_messages.push(format!("Device {}: {}", action.device_id, e));
            }
        }

        // Record execution in database
        let log = AutomationExecutionLog::new(
            rule.id.clone(),
            rule.name.clone(),
            success,
            if error_messages.is_empty() {
                None
            } else {
                Some(error_messages.join("; "))
            },
        );

        if let Err(e) = self.db.insert_execution_log(&log) {
            error!(error = %e, "Failed to insert execution log");
        }

        // Update trigger count
        if success {
            if let Err(e) = self.db.record_rule_trigger(&rule.id) {
                error!(error = %e, "Failed to record rule trigger");
            }
        }
    }

    /// Execute a single action
    async fn execute_action(&self, action: &AutomationAction) -> Result<(), String> {
        debug!(
            device_id = %action.device_id,
            action = ?action.action,
            "Executing action"
        );

        // Get MQTT topic for the device
        let mqtt_topic = self
            .db
            .get_mqtt_topic_for_device(&action.device_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Device not found: {}", action.device_id))?;

        // Determine the desired state
        let desired_state = match action.action {
            SwitchAction::On => true,
            SwitchAction::Off => false,
            SwitchAction::Toggle => {
                // Get current state and toggle it
                if let Some(current_state) = self.switch_state.get_state(&action.device_id) {
                    !current_state
                } else {
                    return Err(format!(
                        "Current state unknown for device {}",
                        action.device_id
                    ));
                }
            }
        };

        // Publish MQTT command
        let payload = if desired_state {
            r#"{"state":"ON"}"#
        } else {
            r#"{"state":"OFF"}"#
        };

        info!(
            device_id = %action.device_id,
            mqtt_topic = %mqtt_topic,
            state = %desired_state,
            "Publishing switch command"
        );

        self.mqtt_client
            .publish(
                &format!("{}/set", mqtt_topic),
                rumqttc::QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await
            .map_err(|e| format!("MQTT publish error: {}", e))?;

        info!(
            device_id = %action.device_id,
            action = ?action.action,
            "Successfully executed action"
        );

        Ok(())
    }
}

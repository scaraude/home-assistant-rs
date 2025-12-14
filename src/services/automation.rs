use std::collections::HashMap;
use std::sync::Arc;

use crate::db::Database;
use crate::events::{SystemEvent, bus::EventBus};
use crate::models::{
    AutomationAction, AutomationCondition, AutomationRule, SensorField, SensorReading, SwitchAction,
};
use crate::mqtt::MqttClient;
use crate::state::{DeviceStateStore, SwitchStateStore};
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};

/// Service responsible for evaluating automation rules and executing actions in response
/// to incoming sensor readings from the event bus.
pub struct AutomationService {
    db: Arc<Database>,
    mqtt_client: MqttClient,
    device_state: DeviceStateStore,
    switch_state: SwitchStateStore,
    event_bus: EventBus,
    event_rx: broadcast::Receiver<SystemEvent>,
    last_triggered: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    debounce_seconds: i64,
}

impl AutomationService {
    pub fn new(
        db: Arc<Database>,
        mqtt_client: MqttClient,
        device_state: DeviceStateStore,
        switch_state: SwitchStateStore,
        event_bus: EventBus,
        event_rx: broadcast::Receiver<SystemEvent>,
    ) -> Self {
        Self {
            db,
            mqtt_client,
            device_state,
            switch_state,
            event_bus,
            event_rx,
            last_triggered: Arc::new(RwLock::new(HashMap::new())),
            debounce_seconds: 60,
        }
    }

    /// Run the automation event loop until the bus closes.
    pub async fn run(mut self) {
        info!("AutomationService started");

        loop {
            match self.event_rx.recv().await {
                Ok(event) => self.process_event(event).await,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "AutomationService lagged behind the event bus"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Event bus closed; AutomationService exiting");
                    break;
                }
            }
        }
    }

    async fn process_event(&self, event: SystemEvent) {
        if let SystemEvent::SensorReading { reading, .. } = event {
            self.handle_sensor_reading(reading).await;
        }
    }

    async fn handle_sensor_reading(&self, reading: SensorReading) {
        debug!(
            device_id = %reading.device_id(),
            "AutomationService received sensor reading"
        );

        let triggered_rules = self.evaluate_reading_internal(&reading).await;
        if triggered_rules.is_empty() {
            return;
        }

        for rule in triggered_rules {
            let trigger_timestamp = Utc::now();
            let trigger_event = SystemEvent::AutomationTriggered {
                rule_id: rule.id.clone(),
                actions: rule.actions.clone(),
                timestamp: trigger_timestamp,
            };
            self.publish_event(trigger_event);

            match self.execute_actions(&rule).await {
                Ok(_) => {
                    let event = SystemEvent::AutomationExecuted {
                        rule_id: rule.id.clone(),
                        success: true,
                        error: None,
                        timestamp: Utc::now(),
                    };
                    self.publish_event(event);
                }
                Err(err_msg) => {
                    let event = SystemEvent::AutomationExecuted {
                        rule_id: rule.id.clone(),
                        success: false,
                        error: Some(err_msg.clone()),
                        timestamp: Utc::now(),
                    };
                    self.publish_event(event);
                    error!(
                        rule_id = %rule.id,
                        rule_name = %rule.name,
                        error = %err_msg,
                        "Automation rule execution failed"
                    );
                }
            }
        }
    }

    async fn evaluate_reading_internal(&self, reading: &SensorReading) -> Vec<AutomationRule> {
        let rules = match self.db.get_all_automation_rules() {
            Ok(rules) => rules.into_iter().filter(|r| r.enabled).collect::<Vec<_>>(),
            Err(e) => {
                error!(error = %e, "Failed to fetch automation rules");
                return Vec::new();
            }
        };

        if rules.is_empty() {
            debug!("No enabled automation rules to evaluate");
            return Vec::new();
        }

        let mut triggered = Vec::new();
        for rule in rules {
            if !Self::rule_targets_device(&rule, reading.device_id()) {
                continue;
            }

            if self.should_debounce(&rule.id).await {
                continue;
            }

            if self.evaluate_conditions(&rule).await {
                info!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Automation rule conditions met"
                );
                self.mark_triggered(&rule.id).await;
                triggered.push(rule);
            } else {
                debug!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Automation rule conditions not met"
                );
            }
        }

        triggered
    }

    async fn should_debounce(&self, rule_id: &str) -> bool {
        let guard = self.last_triggered.read().await;
        if let Some(last_time) = guard.get(rule_id) {
            let elapsed = Utc::now().signed_duration_since(*last_time);
            if elapsed.num_seconds() < self.debounce_seconds {
                debug!(
                    rule_id = %rule_id,
                    seconds_since_last = elapsed.num_seconds(),
                    "Automation rule debounced"
                );
                return true;
            }
        }
        false
    }

    async fn mark_triggered(&self, rule_id: &str) {
        let mut guard = self.last_triggered.write().await;
        guard.insert(rule_id.to_string(), Utc::now());
    }

    fn rule_targets_device(rule: &AutomationRule, device_id: &str) -> bool {
        rule.conditions
            .iter()
            .any(|condition| condition.device_id == device_id)
    }

    async fn evaluate_conditions(&self, rule: &AutomationRule) -> bool {
        if rule.conditions.is_empty() {
            warn!(rule_id = %rule.id, "Automation rule has no conditions");
            return false;
        }

        let mut results = Vec::with_capacity(rule.conditions.len());
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
                "Evaluated automation condition"
            );
        }

        match rule.condition_operator {
            crate::models::LogicalOperator::And => results.iter().all(|&r| r),
            crate::models::LogicalOperator::Or => results.iter().any(|&r| r),
        }
    }

    async fn evaluate_condition(&self, condition: &AutomationCondition) -> bool {
        let field_value = match condition.field {
            SensorField::Temperature => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::TempHumidity { temperature, .. } = reading {
                        Some(temperature as f64)
                    } else {
                        None
                    }
                }),
            SensorField::Humidity => self
                .db
                .get_latest_reading_for_sensor(&condition.device_id)
                .ok()
                .flatten()
                .and_then(|reading| {
                    if let SensorReading::TempHumidity { humidity, .. } = reading {
                        Some(humidity as f64)
                    } else {
                        None
                    }
                }),
            SensorField::Battery => self
                .device_state
                .get_battery(&condition.device_id)
                .map(|b| b as f64),
            SensorField::LinkQuality => self
                .device_state
                .get_link_quality(&condition.device_id)
                .map(|lq| lq as f64),
        };

        let value = match field_value {
            Some(value) => value,
            None => {
                debug!(
                    device_id = %condition.device_id,
                    field = ?condition.field,
                    "Automation condition field value unavailable"
                );
                return false;
            }
        };

        condition.operator.evaluate(value, condition.value)
    }

    async fn execute_actions(&self, rule: &AutomationRule) -> Result<(), String> {
        let mut errors = Vec::new();

        for action in &rule.actions {
            if let Err(err) = self.execute_action(action).await {
                errors.push(format!("{}: {}", action.device_id, err));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    async fn execute_action(&self, action: &AutomationAction) -> Result<(), String> {
        debug!(
            device_id = %action.device_id,
            action = ?action.action,
            "Executing automation action"
        );

        let mqtt_topic = self
            .db
            .get_mqtt_topic_for_device(&action.device_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Device not found: {}", action.device_id))?;

        let desired_state = match action.action {
            SwitchAction::On => true,
            SwitchAction::Off => false,
            SwitchAction::Toggle => {
                if let Some(current) = self.switch_state.get_state(&action.device_id) {
                    !current
                } else {
                    return Err(format!(
                        "Current state unknown for device {}",
                        action.device_id
                    ));
                }
            }
        };

        let payload = format!(
            r#"{{"state":"{}"}}"#,
            if desired_state { "ON" } else { "OFF" }
        );

        self.mqtt_client
            .publish_command(&mqtt_topic, &payload)
            .await
            .map_err(|e| format!("MQTT publish error: {}", e))?;

        info!(
            device_id = %action.device_id,
            action = ?action.action,
            "Automation action executed successfully"
        );

        Ok(())
    }

    fn publish_event(&self, event: SystemEvent) {
        if let Err(e) = self.event_bus.publish(event.clone()) {
            error!(
                error = %e,
                event_type = event.event_type(),
                "Failed to publish automation event"
            );
        }
    }
}
